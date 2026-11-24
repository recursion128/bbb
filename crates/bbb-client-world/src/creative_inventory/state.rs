use bbb_protocol::packets::{
    ContainerSetSlot as ProtocolContainerSetSlot, SetCursorItem as ProtocolSetCursorItem,
};
use serde::{Deserialize, Serialize};

use crate::{CreativeInventorySnapshot, PublicationShared, WorldStore};

use super::{materialize::materialize_creative_inventory, CreativeInventoryBuildError};

const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const PLAYER_HOTBAR_SIZE: usize = 9;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreativeInventoryState {
    #[serde(skip)]
    snapshot: Option<PublicationShared<CreativeInventorySnapshot>>,
    #[serde(skip)]
    last_build_error: Option<String>,
    #[serde(default)]
    hotbar_pop_times: [u8; PLAYER_HOTBAR_SIZE],
    #[serde(default)]
    generation: u64,
}

impl Default for CreativeInventoryState {
    fn default() -> Self {
        Self {
            snapshot: None,
            last_build_error: None,
            hotbar_pop_times: [0; PLAYER_HOTBAR_SIZE],
            generation: 0,
        }
    }
}

impl WorldStore {
    pub fn rebuild_creative_inventory(
        &mut self,
        display_operator_tab: bool,
    ) -> Result<bool, CreativeInventoryBuildError> {
        let has_permissions = display_operator_tab && self.local_player_can_use_gamemaster_blocks();
        let next_generation = self.creative_inventory.generation.saturating_add(1);
        let snapshot = match materialize_creative_inventory(self, next_generation, has_permissions)
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.creative_inventory.last_build_error = Some(error.to_string());
                return Err(error);
            }
        };
        let changed = self
            .creative_inventory
            .snapshot
            .as_ref()
            .is_none_or(|current| {
                current.revision != snapshot.revision || current.tabs != snapshot.tabs
            });
        if changed {
            self.creative_inventory.generation = next_generation;
            self.creative_inventory.snapshot = Some(snapshot.into());
        }
        self.creative_inventory.last_build_error = None;
        Ok(changed)
    }

    fn ensure_creative_inventory_for_open(
        &mut self,
        display_operator_tab: bool,
    ) -> Result<bool, CreativeInventoryBuildError> {
        let has_permissions = display_operator_tab && self.local_player_can_use_gamemaster_blocks();
        if let Some(current) = self.creative_inventory.snapshot.as_ref() {
            if self.creative_inventory.last_build_error.is_none()
                && current.revision.registry == self.registry_access_revision()
                && current.revision.has_permissions == has_permissions
                && current.revision.enabled_features == self.enabled_feature_list()
            {
                return Ok(false);
            }
        }
        self.rebuild_creative_inventory(display_operator_tab)
    }

    pub fn creative_inventory_snapshot(&self) -> Option<&CreativeInventorySnapshot> {
        self.creative_inventory.snapshot.as_deref()
    }

    pub fn creative_inventory_snapshot_shared(
        &self,
    ) -> Option<std::sync::Arc<CreativeInventorySnapshot>> {
        self.creative_inventory
            .snapshot
            .as_ref()
            .map(PublicationShared::clone_arc)
    }

    pub fn creative_inventory_build_error(&self) -> Option<&str> {
        self.creative_inventory.last_build_error.as_deref()
    }

    pub fn open_local_inventory_with_operator_tab(
        &mut self,
        display_operator_tab: bool,
    ) -> Result<bool, CreativeInventoryBuildError> {
        if self.inventory.open_container.is_some() {
            return Ok(false);
        }
        let infinite_materials = self
            .local_player
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.instabuild);
        if infinite_materials {
            self.ensure_creative_inventory_for_open(display_operator_tab)?;
        }
        let opened = self.inventory.open_local_inventory(&mut self.client_ui);
        self.push_screen_request(if infinite_materials {
            crate::ScreenRequest::CreativeInventory(display_operator_tab)
        } else {
            crate::ScreenRequest::Container
        });
        Ok(opened)
    }

    pub(crate) fn reset_creative_inventory_for_new_level(&mut self) {
        self.creative_inventory.hotbar_pop_times = [0; PLAYER_HOTBAR_SIZE];
    }

    pub fn hotbar_pop_times(&self) -> [u8; PLAYER_HOTBAR_SIZE] {
        self.creative_inventory.hotbar_pop_times
    }

    pub fn advance_hotbar_pop_times(&mut self, ticks: u32) {
        let ticks = u8::try_from(ticks).unwrap_or(u8::MAX);
        for pop_time in &mut self.creative_inventory.hotbar_pop_times {
            *pop_time = pop_time.saturating_sub(ticks);
        }
    }

    pub(crate) fn apply_creative_aware_set_cursor_item(
        &mut self,
        packet: ProtocolSetCursorItem,
        creative_screen: bool,
    ) {
        self.tutorial_on_get_item(&packet.item);
        if creative_screen {
            self.counters.cursor_item_updates_received += 1;
            return;
        }
        self.inventory
            .apply_set_cursor_item(&mut self.counters, packet);
    }

    pub fn apply_container_set_slot(&mut self, packet: ProtocolContainerSetSlot) -> bool {
        self.tutorial_on_get_item(&packet.item);
        let hotbar_pop = (packet.container_id == INVENTORY_MENU_CONTAINER_ID)
            .then_some(packet.slot)
            .and_then(|slot| slot.checked_sub(INVENTORY_MENU_HOTBAR_START))
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < PLAYER_HOTBAR_SIZE)
            .filter(|_| {
                packet.item.item_id.is_some()
                    && packet.item.count > 0
                    && self
                        .inventory
                        .inventory_menu_slot_item(packet.slot)
                        .is_none_or(|old| old.item_id.is_none() || old.count < packet.item.count)
            });
        if !self
            .inventory
            .apply_container_set_slot(&mut self.counters, packet)
        {
            return false;
        }
        if let Some(index) = hotbar_pop {
            self.creative_inventory.hotbar_pop_times[index] = 5;
        }
        true
    }
}
