//! Respawn anchor charge and empty-hand respawn/explosion operations.

use super::BlockUseContext;
use crate::block_behavior::respawn_anchor as respawn_anchor_block;
use crate::block_behavior::BlockUseResult;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::packets::InteractionHand;

/// `SoundEvents.RESPAWN_ANCHOR_CHARGE` (`RespawnAnchorBlock.java:185`).
const CHARGE_SOUND: &str = "minecraft:block.respawn_anchor.charge";

/// The `item_on:` column: [`ServerWorld::use_respawn_anchor_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_respawn_anchor_item_on(context, events)
}

impl ServerWorld {
    /// `RespawnAnchorBlock.useItemOn` (`RespawnAnchorBlock.java:78-96`).
    pub(super) fn use_respawn_anchor_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let can_charge = respawn_anchor_block::can_be_charged(context.block_state_id);
        if respawn_anchor_block::is_respawn_fuel(context.selected_item.item_id()) && can_charge {
            self.charge_respawn_anchor(context, events);
            return BlockUseResult::Consumed;
        }
        let offhand_is_fuel = self
            .levels
            .player(context.connection_id)
            .is_some_and(|player| {
                respawn_anchor_block::is_respawn_fuel(player.inventory.off_hand_stack().item_id())
            });
        if context.update.hand == InteractionHand::MainHand && offhand_is_fuel && can_charge {
            return BlockUseResult::Pass;
        }
        BlockUseResult::TryWithEmptyHand
    }

    /// `RespawnAnchorBlock.charge` (`RespawnAnchorBlock.java:181-186`) then
    /// `ItemStack.consume` (`:89`).
    fn charge_respawn_anchor(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) {
        let charge = respawn_anchor_block::charges(context.block_state_id)
            .expect("a respawn anchor carries charges");
        let new_state = respawn_anchor_block::with_charges(context.block_state_id, charge + 1)
            .expect("charges 0..=4 is a legal state");
        let (instabuild, source_entity_id) = {
            let player = self
                .levels
                .player(context.connection_id)
                .expect("connection presence was checked");
            (player.abilities.instabuild, player.entity_id())
        };
        self.set_block(
            context.dimension,
            context.update.hit.pos,
            new_state,
            SET_BLOCK_UPDATE_ALL,
        );
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                context.update.hit.pos,
                Some(source_entity_id),
                Some(new_state),
            );
        self.play_block_sound(
            context.dimension,
            context.update.hit.pos,
            CHARGE_SOUND,
            events,
        );
        if let Some(player) = self.levels.player_mut(context.connection_id) {
            player
                .inventory
                .consume_in_hand(context.update.hand, 1, instabuild);
        }
    }
}

// RespawnAnchorBlock.java:99-129; water protection is sampled after removal.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if respawn_anchor_block::charges(context.block_state_id) == Some(0) {
        return false;
    }
    let pos = context.update.hit.pos;
    let dimension = context.dimension;
    let key = world.level_key(dimension).clone();
    let Some(works) = world.anchor_works_at(&world.require_level_id(&key), pos) else {
        return true;
    };
    if !works {
        world.set_block(
            dimension,
            pos,
            bbb_server_types::fluid::air_block_state_id(),
            SET_BLOCK_UPDATE_ALL,
        );
        let chunks = &world.level_by_id(&dimension).chunks;
        let fluid = |pos| {
            bbb_server_types::fluid::block_state_fluid_state(crate::world::stored_block_state(
                chunks, pos,
            ))
        };
        let water = |pos| fluid(pos).family() == Some(bbb_server_types::fluid::FluidFamily::Water);
        let neighbor_water = [
            bbb_protocol::packets::Direction::North,
            bbb_protocol::packets::Direction::South,
            bbb_protocol::packets::Direction::West,
            bbb_protocol::packets::Direction::East,
        ]
        .into_iter()
        .any(|direction| {
            let neighbor = bbb_server_types::directions::relative(pos, direction);
            let neighbor_fluid = fluid(neighbor);
            water(neighbor)
                && (neighbor_fluid.is_source()
                    || neighbor_fluid.amount() >= 2
                        && !water(bbb_protocol::packets::BlockPos {
                            y: neighbor.y - 1,
                            ..neighbor
                        }))
        });
        let shield = neighbor_water
            || water(bbb_protocol::packets::BlockPos {
                y: pos.y + 1,
                ..pos
            });
        world.explode_bad_respawn_point(dimension, pos, shield, events);
        world.mark_use_item_on_server_swing();
    } else {
        let player = world.levels.player_at(context.connection_id);
        let changed = player
            .cold
            .respawn
            .as_ref()
            .is_none_or(|old| old.dimension != key || old.pos != pos);
        if changed {
            world.set_sleep_respawn(
                context.connection_id,
                crate::world::PlayerRespawnConfig {
                    dimension: key,
                    pos,
                    yaw: 0.0,
                    pitch: 0.0,
                    forced: false,
                },
                events,
            );
            world.play_block_sound(
                dimension,
                pos,
                "minecraft:block.respawn_anchor.set_spawn",
                events,
            );
            world.mark_use_item_on_server_swing();
        }
    }
    true
}
