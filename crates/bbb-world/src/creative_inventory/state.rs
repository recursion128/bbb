use bbb_protocol::packets::{
    ContainerSetSlot as ProtocolContainerSetSlot, SetCursorItem as ProtocolSetCursorItem,
};
use serde::{Deserialize, Serialize};

use crate::{ContainerSlot, CreativeInventorySnapshot, PublicationShared, WorldStore};

use super::{materialize::materialize_creative_inventory, CreativeInventoryBuildError};

const INVENTORY_MENU_CONTAINER_ID: i32 = 0;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX: i32 = -1;
const PLAYER_HOTBAR_SIZE: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeInventoryScreenState {
    pub client_level_generation: u64,
    pub catalog_generation: u64,
    pub display_operator_tab: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreativeInventoryState {
    #[serde(skip)]
    snapshot: Option<PublicationShared<CreativeInventorySnapshot>>,
    #[serde(skip)]
    last_build_error: Option<String>,
    #[serde(default)]
    hotbar_pop_times: [u8; PLAYER_HOTBAR_SIZE],
    #[serde(default)]
    remote_inventory_menu_slots: Vec<ContainerSlot>,
    #[serde(default)]
    generation: u64,
}

impl Default for CreativeInventoryState {
    fn default() -> Self {
        Self {
            snapshot: None,
            last_build_error: None,
            hotbar_pop_times: [0; PLAYER_HOTBAR_SIZE],
            remote_inventory_menu_slots: Vec::new(),
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
        if let Some(screen) = self.client_ui.current_creative_inventory.as_mut() {
            screen.catalog_generation = self.creative_inventory.generation;
            screen.display_operator_tab = display_operator_tab;
        }
        Ok(changed)
    }

    pub fn creative_inventory_snapshot(&self) -> Option<&CreativeInventorySnapshot> {
        self.creative_inventory.snapshot.as_deref()
    }

    pub fn creative_inventory_build_error(&self) -> Option<&str> {
        self.creative_inventory.last_build_error.as_deref()
    }

    pub fn creative_inventory_screen(&self) -> Option<&CreativeInventoryScreenState> {
        self.client_ui.current_creative_inventory.as_ref()
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
            self.rebuild_creative_inventory(display_operator_tab)?;
        }
        let opened = self.inventory.open_local_inventory(&mut self.client_ui);
        if infinite_materials {
            self.creative_inventory.remote_inventory_menu_slots =
                self.inventory.inventory_menu.slots.clone();
            self.client_ui.current_creative_inventory = Some(CreativeInventoryScreenState {
                client_level_generation: self.client_level_generation,
                catalog_generation: self.creative_inventory.generation,
                display_operator_tab,
            });
        }
        Ok(opened)
    }

    pub(crate) fn refresh_open_creative_inventory(&mut self) {
        let Some(display_operator_tab) = self
            .client_ui
            .current_creative_inventory
            .as_ref()
            .map(|screen| screen.display_operator_tab)
        else {
            return;
        };
        if let Err(error) = self.rebuild_creative_inventory(display_operator_tab) {
            self.record_apply_error("creative_inventory_rebuild", error);
        }
    }

    pub(crate) fn reconcile_creative_inventory_abilities(&mut self) {
        let infinite_materials = self
            .local_player
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.instabuild);
        if !infinite_materials && self.client_ui.current_creative_inventory.is_some() {
            self.client_ui.replace_screen();
            self.inventory.open_local_inventory(&mut self.client_ui);
        }
    }

    pub(crate) fn reset_creative_inventory_for_new_level(&mut self) {
        self.client_ui.current_creative_inventory = None;
        self.creative_inventory.hotbar_pop_times = [0; PLAYER_HOTBAR_SIZE];
        self.creative_inventory.remote_inventory_menu_slots.clear();
    }

    pub fn creative_hotbar_pop_times(&self) -> [u8; PLAYER_HOTBAR_SIZE] {
        self.creative_inventory.hotbar_pop_times
    }

    pub fn advance_creative_hotbar_pop_times(&mut self, ticks: u32) {
        let ticks = u8::try_from(ticks).unwrap_or(u8::MAX);
        for pop_time in &mut self.creative_inventory.hotbar_pop_times {
            *pop_time = pop_time.saturating_sub(ticks);
        }
    }

    pub fn creative_remote_inventory_menu_slots(&self) -> &[ContainerSlot] {
        &self.creative_inventory.remote_inventory_menu_slots
    }

    pub(crate) fn apply_creative_aware_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        if self.client_ui.current_creative_inventory.is_some() {
            self.counters.cursor_item_updates_received += 1;
            return;
        }
        self.inventory
            .apply_set_cursor_item(&mut self.counters, packet);
    }

    pub(crate) fn apply_creative_aware_container_set_slot(
        &mut self,
        packet: ProtocolContainerSetSlot,
    ) -> bool {
        let inventory_menu_slot =
            (packet.container_id == INVENTORY_MENU_CONTAINER_ID).then_some(packet.slot);
        let old_item = inventory_menu_slot.and_then(|slot| {
            self.inventory
                .inventory_menu
                .slots
                .iter()
                .find(|existing| existing.slot == slot)
                .map(|existing| existing.item.clone())
        });
        let remote_item = packet.item.clone();
        let applied = self
            .inventory
            .apply_container_set_slot(&mut self.counters, packet);
        if !applied {
            return false;
        }
        let Some(menu_slot) = inventory_menu_slot else {
            return true;
        };

        if self.client_ui.current_creative_inventory.is_some() {
            set_container_slot(
                &mut self.creative_inventory.remote_inventory_menu_slots,
                ContainerSlot {
                    slot: menu_slot,
                    item: remote_item.clone(),
                    local_selected_bundle_item_index: NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX,
                },
            );
        }
        if let Some(hotbar_index) = menu_slot
            .checked_sub(INVENTORY_MENU_HOTBAR_START)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < PLAYER_HOTBAR_SIZE)
        {
            let old_count = old_item.as_ref().map_or(0, |item| item.count);
            if remote_item.item_id.is_some()
                && remote_item.count > 0
                && (old_item.as_ref().is_none_or(|item| item.item_id.is_none())
                    || old_count < remote_item.count)
            {
                self.creative_inventory.hotbar_pop_times[hotbar_index] = 5;
            }
        }
        true
    }
}

fn set_container_slot(slots: &mut Vec<ContainerSlot>, row: ContainerSlot) {
    if let Some(existing) = slots.iter_mut().find(|existing| existing.slot == row.slot) {
        *existing = row;
    } else {
        slots.push(row);
        slots.sort_by_key(|slot| slot.slot);
    }
}
