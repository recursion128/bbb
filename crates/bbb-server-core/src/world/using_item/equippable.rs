//! Held equipment use and displacement into the player's inventory.

use bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use crate::entity::living::valid_equipment::{can_be_equipped_by, equippable};
use crate::entity::EntityRegistryTagView;
use crate::world::item_use::{ItemUseContext, ItemUseResult};
use crate::world::{ServerWorld, WorldEvent};

use super::{runtime_item_in_hand, HeldItem};

impl ServerWorld {
    /// `Equippable.swapWithEquipmentSlot` (`Equippable.java:127-157`).
    pub(super) fn swap_player_equipment(
        &mut self,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(ItemUseResult, Option<HeldItem>), String> {
        let id = context.connection_id;
        let resolver = self.item_stack_resolver();
        let player = self.levels.player_at(id);
        let dimension = player.dimension;
        let held = runtime_item_in_hand(player, context.hand).clone();
        let tags = self.player_registry_tags(id).map_or(
            EntityRegistryTagView::Builtin26_1,
            EntityRegistryTagView::Frozen,
        );
        let Some(Some(equippable)) = equippable(&held) else {
            return Ok((ItemUseResult::Pass, None));
        };
        let Some(slot) = vanilla_equipment_slot_by_id(equippable.slot_id) else {
            return Ok((ItemUseResult::Pass, None));
        };
        let Some((_, inventory_slot)) = player
            .inventory
            .equipment_slots()
            .into_iter()
            .find(|(candidate, _)| *candidate == slot.slot)
        else {
            return Ok((ItemUseResult::Pass, None));
        };
        if can_be_equipped_by(player.body.entity_type_id(), equippable, &tags) != Some(true) {
            return Ok((ItemUseResult::Pass, None));
        }
        let old = player.inventory.slot(inventory_slot).unwrap().clone();
        let creative = player.abilities.instabuild;
        let entities = &self.level_by_id(&dimension).entities;
        if old
            .same_item_same_effective_components(&held, &resolver)
            .map_err(|error| error.to_string())?
            || (!creative
                && crate::entity::armor_stand::stack_enchantment_ids(&old, entities)
                    .iter()
                    .any(|id| entities.prevents_armor_change(id)))
        {
            return Ok((ItemUseResult::Fail, None));
        }
        let equipped = if held.count() <= 1 {
            held.clone()
        } else {
            held.with_count(1)
        };
        let in_hand = if held.count() <= 1 {
            if !old.is_empty() {
                old.clone()
            } else if creative {
                held.clone()
            } else {
                RuntimeItemStack::empty()
            }
        } else if creative {
            held.clone()
        } else {
            held.with_count(held.count() - 1)
        };
        let player = &mut self.levels[id];
        player
            .inventory
            .set_inventory_slot(inventory_slot, equipped.clone());
        player.award_stat_once(
            &mut self.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, held.item_id().unwrap()),
        );
        self.player_equip_item(id, slot.slot, &old, &equipped, events);
        if held.count() > 1 && !old.is_empty() {
            let mut displaced = old;
            self.levels[id]
                .inventory
                .add_anywhere(&resolver, &mut displaced, creative)
                .map_err(|error| error.to_string())?;
            if !displaced.is_empty() {
                self.drop_player_stack(id, &displaced, false);
            }
        }
        Ok((ItemUseResult::Success, Some(HeldItem::Replaced(in_hand))))
    }
}

#[cfg(test)]
mod tests;
