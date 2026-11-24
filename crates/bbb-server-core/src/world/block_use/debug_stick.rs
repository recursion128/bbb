//! `DebugStickItem` (`DebugStickItem.java:29-104`): right click cycles the
//! selected property's value, left click selects the next property.

use crate::connection::ConnectionId;
use crate::inventory::has_component;
use bbb_protocol::block_state_types::{
    vanilla_block_property_values, vanilla_block_state_block_type_id,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID;
use bbb_protocol::packets::{
    BlockPos, DebugStickPropertySummary, InteractionHand, ItemStackSummary, SystemChat, UseItemOn,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{ServerWorld, WorldEvent};
use crate::world::using_item::runtime_item_in_hand;

/// `Block.UPDATE_CLIENTS | Block.UPDATE_KNOWN_SHAPE` (`DebugStickItem.java:78`).
const UPDATE_CLIENTS_AND_KNOWN_SHAPE: i32 = 18;

/// `DebugStickItem.useOn` (`DebugStickItem.java:38-49`). `Some` ends the item
/// leg; its value distinguishes consuming success from refused `FAIL`.
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    update: UseItemOn,
    stack: &ItemStackSummary,
    events: &mut Vec<WorldEvent>,
) -> Option<bool> {
    if stack.item_id != Some(VANILLA_ITEM_DEBUG_STICK_ID) {
        return None;
    }
    let pos = update.hit.pos;
    let consumed = world.debug_stick_interaction(connection_id, update.hand, pos, true, events);
    if consumed {
        // `ItemStack.useOn` (`ItemStack.java:367-369`) and
        // `ServerPlayerGameMode.useItemOn` (`:397-399`) on `SUCCESS`.
        world.levels[connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_DEBUG_STICK_ID),
        );
        world.trigger_item_used_on_location(
            connection_id,
            "minecraft:item_used_on_block",
            pos,
            stack,
            events,
        );
    }
    Some(consumed)
}

impl ServerWorld {
    /// `DebugStickItem.handleInteraction` (`DebugStickItem.java:51-87`).
    pub(in crate::world) fn debug_stick_interaction(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        pos: BlockPos,
        cycle: bool,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if !self.can_use_game_master_blocks(connection_id) {
            return false;
        }
        let dimension = self.player_level_id(connection_id);
        let state = self.block_state_or_air(dimension, pos);
        let (Some(identity), Some(block_registry_id)) = (
            vanilla_block_state_identity(state),
            vanilla_block_state_block_type_id(state),
        ) else {
            return false;
        };
        // `StateDefinition.getProperties` is an `ImmutableSortedMap` by name.
        let properties: Vec<&str> = identity.properties.keys().map(String::as_str).collect();
        if properties.is_empty() {
            self.debug_stick_message(
                connection_id,
                "item.minecraft.debug_stick.empty",
                vec![identity.name.clone()],
                events,
            );
            return false;
        }
        let player = self.levels.player_at(connection_id);
        let backward = player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN);
        let held = runtime_item_in_hand(player, hand).clone();
        let stack = held.summary();
        if !has_component(stack, VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID).unwrap_or(false)
        {
            return false;
        }
        let patched = stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID);
        let mut selections = if patched {
            stack.component_patch.debug_stick_properties.clone()
        } else {
            Vec::new()
        };
        let selected = selections
            .iter()
            .find(|entry| entry.block_registry_id == block_registry_id)
            .and_then(|entry| {
                properties
                    .iter()
                    .copied()
                    .find(|property| *property == entry.property)
            });
        if cycle {
            let property = selected.unwrap_or(properties[0]);
            let Some(values) = vanilla_block_property_values(&identity.name, property) else {
                return false;
            };
            let value = relative(
                values,
                Some(identity.properties[property].as_str()),
                backward,
            );
            let mut cycled = identity.properties.clone();
            cycled.insert(property.to_string(), value.to_string());
            let Some(new_state) =
                vanilla_block_state_id_by_name_and_properties(&identity.name, &cycled)
            else {
                return false;
            };
            self.set_block(dimension, pos, new_state, UPDATE_CLIENTS_AND_KNOWN_SHAPE);
            self.debug_stick_message(
                connection_id,
                "item.minecraft.debug_stick.update",
                vec![property.to_string(), value.to_string()],
                events,
            );
        } else {
            let property = relative(&properties, selected, backward);
            // `Util.copyAndPut`'s `buildKeepingLast` keeps the block's first slot.
            match selections
                .iter_mut()
                .find(|entry| entry.block_registry_id == block_registry_id)
            {
                Some(entry) => entry.property = property.to_string(),
                None => selections.push(DebugStickPropertySummary {
                    block_registry_id,
                    property: property.to_string(),
                }),
            }
            let mut edited = stack.clone();
            let patch = &mut edited.component_patch;
            patch
                .removed_type_ids
                .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID);
            if !patched {
                patch
                    .added_type_ids
                    .push(VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID);
            }
            patch.debug_stick_properties = selections;
            let edited = RuntimeItemStack::from_summary(edited);
            *self.levels[connection_id].inventory.item_in_hand_mut(hand) = edited;
            self.debug_stick_message(
                connection_id,
                "item.minecraft.debug_stick.select",
                vec![property.to_string(), identity.properties[property].clone()],
                events,
            );
        }
        true
    }

    /// `DebugStickItem.message` (`DebugStickItem.java:97-99`):
    /// `sendOverlayMessage`.
    fn debug_stick_message(
        &self,
        connection_id: ConnectionId,
        key: &str,
        arguments: Vec<String>,
        events: &mut Vec<WorldEvent>,
    ) {
        let message = bbb_protocol::component::RichTextComponent::try_from_component_nbt(
            crate::commands::translatable(
                key,
                arguments.into_iter().map(NbtSummaryValue::string).collect(),
            ),
        )
        .expect("a translatable with string arguments is a valid component");
        self.send_system_message(
            connection_id,
            SystemChat::from_component(message, true),
            events,
        );
    }
}

/// `Util.findNextInIterable` / `findPreviousInIterable` (`Util.java:530-567`).
fn relative<'a>(values: &[&'a str], current: Option<&str>, backward: bool) -> &'a str {
    if backward {
        let mut last: Option<&'a str> = None;
        for (index, &value) in values.iter().enumerate() {
            if Some(value) == current {
                if last.is_none() {
                    last = Some(if index + 1 < values.len() {
                        values[values.len() - 1]
                    } else {
                        value
                    });
                }
                break;
            }
            last = Some(value);
        }
        last.expect("a property has at least one value")
    } else {
        match current.and_then(|current| values.iter().position(|value| *value == current)) {
            Some(index) if index + 1 < values.len() => values[index + 1],
            _ => values[0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::relative;

    /// Sneaking walks `findPreviousInIterable`, which wraps from the first
    /// value and from no selection alike to the last.
    #[test]
    fn relative_wraps_both_ways() {
        let values = ["north", "south", "west", "east"];
        assert_eq!(relative(&values, Some("east"), false), "north");
        assert_eq!(relative(&values, Some("north"), true), "east");
        assert_eq!(relative(&values, Some("west"), true), "south");
        assert_eq!(relative(&values, None, true), "east");
    }
}
