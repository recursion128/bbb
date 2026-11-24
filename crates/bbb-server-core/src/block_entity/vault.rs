//! Vault key admission, persisted reward queue, and server state transitions.

mod codec;
mod data;
#[cfg(test)]
mod tests;

use super::{BlockEntityItemLoad, BlockEntityItemSave};
use crate::world::simulation_tick::vault::VaultLevel;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
};
use bbb_protocol::block_types::VANILLA_BLOCK_VAULT_ID;
use bbb_protocol::NbtSummaryEntry;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;
pub use data::{VaultConfig, VaultServerData, VaultSharedData};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultState {
    Inactive,
    Active,
    Unlocking,
    Ejecting,
}

impl VaultState {
    pub fn from_block_state(state: i32) -> Option<Self> {
        if vanilla_block_state_block_type_id(state)? != VANILLA_BLOCK_VAULT_ID {
            return None;
        }
        match vanilla_block_state_property(state, "vault_state")? {
            "inactive" => Some(Self::Inactive),
            "active" => Some(Self::Active),
            "unlocking" => Some(Self::Unlocking),
            "ejecting" => Some(Self::Ejecting),
            _ => None,
        }
    }
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Active => "active",
            Self::Unlocking => "unlocking",
            Self::Ejecting => "ejecting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultSound {
    Insert,
    InsertFail,
    RejectRewardedPlayer,
    OpenShutter,
    CloseShutter,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultKeyDecision {
    Skip,
    Fail(VaultSound),
    DrawReward,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultTickResult {
    pub changed: bool,
    pub shared_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VaultBlockEntity {
    config: VaultConfig,
    shared_data: VaultSharedData,
    server_data: VaultServerData,
}

impl VaultBlockEntity {
    pub fn config(&self) -> &VaultConfig {
        &self.config
    }
    pub fn shared_data(&self) -> &VaultSharedData {
        &self.shared_data
    }
    pub fn server_data(&self) -> &VaultServerData {
        &self.server_data
    }

    // VaultBlockEntity.java: item validity precedes reward history.
    // VaultBlockEntity.java: failed insertions share one transient clock.
    pub fn key_decision(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        state: VaultState,
        now: i64,
        uuid: Uuid,
        inserted: &RuntimeItemStack,
    ) -> Result<VaultKeyDecision> {
        if state == VaultState::Inactive {
            return Ok(VaultKeyDecision::Skip);
        }
        let key = self.config.key_item();
        if self.config.key_is_empty() {
            return Ok(VaultKeyDecision::Skip);
        }
        let failure = if !inserted.same_item_same_effective_components(key, resolver)?
            || inserted.count() < key.count()
        {
            Some(VaultSound::InsertFail)
        } else if self.server_data.rewarded_players.contains(&uuid) {
            Some(VaultSound::RejectRewardedPlayer)
        } else {
            None
        };
        if let Some(sound) = failure {
            if now >= self.server_data.last_insert_fail_timestamp.wrapping_add(15) {
                self.server_data.last_insert_fail_timestamp = now;
                return Ok(VaultKeyDecision::Fail(sound));
            }
            return Ok(VaultKeyDecision::Skip);
        }
        Ok(VaultKeyDecision::DrawReward)
    }

    // VaultBlockEntity.java: the world consumes the key before unlock.
    // VaultBlockEntity.java; VaultServerData.java.
    pub(crate) fn unlock(
        &mut self,
        uuid: Uuid,
        items: Vec<RuntimeItemStack>,
        time: i64,
        state: VaultState,
        context: &mut VaultLevel<'_>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        self.server_data.total_ejections_needed = items.len() as i32;
        self.server_data.items_to_eject = items;
        self.server_data.dirty = true;
        self.shared_data.set_display(self.server_data.next_item());
        self.server_data.pause_until(time.wrapping_add(14));
        self.transition(state, VaultState::Unlocking, context)?;
        if !self.server_data.rewarded_players.contains(&uuid) {
            self.server_data.rewarded_players.push(uuid);
        }
        if self.server_data.rewarded_players.len() > 128 {
            self.server_data.rewarded_players.remove(0);
        }
        self.server_data.dirty = true;
        self.refresh_players(self.config.deactivation_range(), context)?;
        Ok(())
    }

    pub(crate) const fn needs_tick(&self, time: i64) -> bool {
        time >= self.server_data.state_updating_resumes_at
    }

    pub(crate) const fn has_pending_changes(&self) -> bool {
        self.server_data.dirty || self.shared_data.dirty
    }

    // VaultState.java: connected checks and ejections pause for twenty ticks.
    pub(crate) fn tick(
        &mut self,
        state: VaultState,
        time: i64,
        context: &mut VaultLevel<'_>,
    ) -> Result<()> {
        if time >= self.server_data.state_updating_resumes_at {
            let next = match state {
                VaultState::Inactive | VaultState::Active => {
                    let range = if state == VaultState::Inactive {
                        self.config.activation_range()
                    } else {
                        self.config.deactivation_range()
                    };
                    self.refresh_players(range, context)?;
                    self.server_data.pause_until(time.wrapping_add(20));
                    self.connected_state()
                }
                VaultState::Unlocking => {
                    self.server_data.pause_until(time.wrapping_add(20));
                    VaultState::Ejecting
                }
                VaultState::Ejecting => {
                    if let Some(item) = self.server_data.items_to_eject.last().cloned() {
                        let progress = self.server_data.ejection_progress();
                        // A refused physical output must not consume its persistent queue entry.
                        context.eject_item(item, progress)?;
                        self.server_data.items_to_eject.pop();
                        self.server_data.dirty = true;
                        self.shared_data.set_display(self.server_data.next_item());
                        self.server_data.pause_until(time.wrapping_add(20));
                        VaultState::Ejecting
                    } else {
                        self.server_data.total_ejections_needed = 0;
                        self.server_data.dirty = true;
                        self.refresh_players(self.config.deactivation_range(), context)?;
                        self.server_data.pause_until(time.wrapping_add(20));
                        self.connected_state()
                    }
                }
            };
            if state != next {
                self.transition(state, next, context)?;
            }
        }
        Ok(())
    }

    pub(crate) fn set_display(&mut self, item: RuntimeItemStack) {
        self.shared_data.set_display(item);
    }

    pub(crate) fn take_changes(&mut self) -> VaultTickResult {
        let result = VaultTickResult {
            changed: self.server_data.dirty || self.shared_data.dirty,
            shared_changed: self.shared_data.dirty,
        };
        self.server_data.dirty = false;
        self.shared_data.dirty = false;
        result
    }

    fn connected_state(&self) -> VaultState {
        if self.shared_data.connected_players.is_empty() {
            VaultState::Inactive
        } else {
            VaultState::Active
        }
    }
    // VaultSharedData.java: keep the old set if equal; HashSet order is not a target.
    fn refresh_players(&mut self, range: f64, context: &mut VaultLevel<'_>) -> Result<()> {
        let mut players = Vec::new();
        for uuid in context.connected_players(range)? {
            if !self.server_data.rewarded_players.contains(&uuid) && !players.contains(&uuid) {
                players.push(uuid);
            }
        }
        let old = &self.shared_data.connected_players;
        if old.len() != players.len() || old.iter().any(|uuid| !players.contains(uuid)) {
            self.shared_data.connected_players = players;
            self.shared_data.dirty = true;
        }
        Ok(())
    }
    // VaultBlockEntity.java; VaultState.java: setBlock precedes hooks.
    fn transition(
        &mut self,
        from: VaultState,
        to: VaultState,
        context: &mut VaultLevel<'_>,
    ) -> Result<()> {
        context.set_state(from, to)?;
        if from == VaultState::Ejecting {
            context.sound(VaultSound::CloseShutter);
        }
        match to {
            VaultState::Inactive => {
                self.shared_data.set_display(RuntimeItemStack::empty());
                context.level_event(3016);
            }
            VaultState::Active => {
                context.level_event(3015);
            }
            VaultState::Unlocking => context.sound(VaultSound::Insert),
            VaultState::Ejecting => context.sound(VaultSound::OpenShutter),
        }
        Ok(())
    }

    pub(super) fn save_items(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        Ok(vec![super::entry(
            "shared_data",
            self.shared_data.to_nbt(save)?,
        )])
    }
    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> Result<Vec<NbtSummaryEntry>> {
        Ok(vec![
            super::entry("config", self.config.to_nbt(save)?),
            super::entry("shared_data", self.shared_data.to_nbt(save)?),
            super::entry("server_data", self.server_data.to_nbt(save)?),
        ])
    }
    // VaultBlockEntity.java: config resets; missing shared/server records keep live data.
    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> Result<()> {
        if let Some(value) = super::unique_field(record, "server_data") {
            if let Some(data) = VaultServerData::from_nbt(value, load)? {
                self.server_data.set(data);
            }
        }
        self.config = match super::unique_field(record, "config") {
            Some(value) => VaultConfig::from_nbt(value, load)?.unwrap_or_default(),
            None => VaultConfig::default(),
        };
        if let Some(value) = super::unique_field(record, "shared_data") {
            if let Some(data) = VaultSharedData::from_nbt(value, load)? {
                self.shared_data.set(data);
            }
        }
        Ok(())
    }
}
