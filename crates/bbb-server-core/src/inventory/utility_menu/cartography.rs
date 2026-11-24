//! `CartographyTableMenu`: map copy, zoom-out and lock. The new map itself is
//! made when the crafted stack's `map_post_processing` is run.

use crate::world::maps::ServerMapStore;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_GLASS_PANE_ID, VANILLA_ITEM_MAP_ID, VANILLA_ITEM_PAPER_ID,
};
use bbb_protocol::packets::{DataComponentPatchSummary, MapPostProcessingSummary};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::rules::has_component;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::{PlayerInventoryMenu, UtilityMenuEvent};

const MAP: usize = 0;
const ADDITIONAL: usize = 1;
const RESULT: usize = 2;
/// `MapItemSavedData.MAX_SCALE`.
const MAX_SCALE: i8 = 4;

/// The two input slots' `mayPlace` (`CartographyTableMenu.java:49-60`).
pub(super) fn may_place(slot: i16, item: &RuntimeItemStack) -> Result<bool> {
    if slot == MAP as i16 {
        return has_component(item.summary(), VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    }
    Ok(matches!(
        item.item_id(),
        Some(VANILLA_ITEM_PAPER_ID | VANILLA_ITEM_MAP_ID | VANILLA_ITEM_GLASS_PANE_ID)
    ))
}

impl PlayerInventoryMenu {
    /// `CartographyTableMenu.slotsChanged` (`CartographyTableMenu.java:91-102`).
    pub(super) fn cartography_slots_changed(&mut self) -> Result<()> {
        let map = &self.container[MAP];
        let additional = &self.container[ADDITIONAL];
        if self.container[RESULT].is_empty() || !map.is_empty() && !additional.is_empty() {
            if !map.is_empty() && !additional.is_empty() {
                self.cartography_setup_result()?;
            }
        } else {
            self.container[RESULT] = RuntimeItemStack::empty();
        }
        Ok(())
    }

    /// `CartographyTableMenu.setupResultSlot` (`CartographyTableMenu.java:104-134`).
    fn cartography_setup_result(&mut self) -> Result<()> {
        let map = self.container[MAP].clone();
        let additional = self.container[ADDITIONAL].item_id();
        let Some(facts) = self.utility().facts.clone() else {
            return Ok(());
        };
        let Some(data) = map
            .map_id()
            .and_then(|id| self.utility().player.maps.get(&id).copied())
        else {
            return Ok(());
        };
        let operation = match additional {
            Some(VANILLA_ITEM_PAPER_ID) if !data.locked && data.scale < MAX_SCALE => {
                Some(MapPostProcessingSummary::Scale)
            }
            Some(VANILLA_ITEM_GLASS_PANE_ID) if !data.locked => {
                Some(MapPostProcessingSummary::Lock)
            }
            Some(VANILLA_ITEM_MAP_ID) => None,
            _ => {
                self.container[RESULT] = RuntimeItemStack::empty();
                return Ok(());
            }
        };
        let result = match operation {
            Some(operation) => map.with_count(1).with_component_patch(
                &DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID],
                    map_post_processing: Some(operation),
                    ..Default::default()
                },
                &facts.resolver,
            )?,
            None => map.with_count(2),
        };
        if result != self.container[RESULT] {
            self.container[RESULT] = result;
        }
        Ok(())
    }

    /// The result slot's `onTake` (`CartographyTableMenu.java:68-80`).
    pub(super) fn cartography_on_take(&mut self) -> Result<()> {
        for slot in [MAP, ADDITIONAL] {
            self.container[slot].shrink(1);
            self.cartography_slots_changed()?;
        }
        self.utility_mut()
            .events
            .push(UtilityMenuEvent::CartographyUsed);
        self.cartography_slots_changed()
    }

    /// `CartographyTableMenu.quickMoveStack` (`CartographyTableMenu.java:142-189`).
    pub(super) fn cartography_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
        dropped: &mut Vec<crate::inventory::DroppedMenuStack>,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        if slot == RESULT as i16
            && stack
                .summary()
                .component_patch
                .map_post_processing
                .is_some()
        {
            if !(3..39).any(|inv_slot| self.slot_item(inv_slot).is_empty()) {
                return Ok(RuntimeItemStack::empty());
            }
            stack = Self::post_process_crafting_result(maps.as_deref_mut(), resolver, stack)?;
        }
        let moved = if slot == RESULT as i16 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 39, true)?
        } else if slot != 0 && slot != 1 {
            if has_component(stack.summary(), VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)? {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 1, false)?
            } else if !may_place(ADDITIONAL as i16, &stack)? {
                if (3..30).contains(&slot) {
                    self.move_item_stack_to(
                        resolver,
                        maps.as_deref_mut(),
                        &mut stack,
                        30,
                        39,
                        false,
                    )?
                } else if (30..39).contains(&slot) {
                    self.move_item_stack_to(
                        resolver,
                        maps.as_deref_mut(),
                        &mut stack,
                        3,
                        30,
                        false,
                    )?
                } else {
                    true
                }
            } else {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 1, 2, false)?
            }
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 39, false)?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        let remainder = stack.clone();
        let moved = self.finish_utility_move(resolver, maps, slot, &clicked, stack)?;
        if slot == RESULT as i16 && !moved.is_empty() && !remainder.is_empty() {
            dropped.push(crate::inventory::DroppedMenuStack {
                stack: remainder,
                thrown_from_hand: false,
            });
        }
        Ok(moved)
    }
}
