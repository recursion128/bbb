//! The carved-pumpkin and wither-skeleton-skull behaviours: place the head
//! on a finished golem or wither base, else wear it.

use bbb_protocol::block_types::{
    VANILLA_BLOCK_CARVED_PUMPKIN_ID, VANILLA_BLOCK_WITHER_SKELETON_SKULL_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::Direction;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::equipment::dispense_equipment;
use super::{BlockSource, Dispensed};
use crate::block_behavior::carved_pumpkin::{can_spawn_golem, has_wither_base, wither_check_spawn};
use crate::block_behavior::default_block_state_id;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::ServerWorld;

pub(super) fn wither_skull(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let peaceful = world.admin.settings().difficulty == Difficulty::Peaceful;
    let skull = with_property(
        default_block_state_id(VANILLA_BLOCK_WITHER_SKELETON_SKULL_ID),
        "rotation",
        &rotation_segment(source.facing).to_string(),
    );
    // `WitherSkullBlock.canSpawnMob`.
    let placed = world.with_level(source.level, |level| {
        let skull = skull?;
        let can_spawn = block_state_is_air(level.block_state(target))
            && target.y >= level.get_min_y() + 2
            && !peaceful
            && has_wither_base(level, target);
        if !can_spawn {
            return None;
        }
        level.set_block(target, skull, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
        level.game_event_from_entity(VanillaGameEvent::BLOCK_PLACE, target, None, None);
        wither_check_spawn(level, target, skull, peaceful);
        Some(())
    });
    let success = if placed.is_some() {
        stack.shrink(1);
        true
    } else {
        dispense_equipment(world, source, &mut stack)
    };
    Ok(Dispensed::optional(stack, success))
}

pub(super) fn carved_pumpkin(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let placed = world.with_level(source.level, |level| {
        if !block_state_is_air(level.block_state(target)) || !can_spawn_golem(level, target) {
            return false;
        }
        level.set_block(
            target,
            default_block_state_id(VANILLA_BLOCK_CARVED_PUMPKIN_ID),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        );
        level.game_event_from_entity(VanillaGameEvent::BLOCK_PLACE, target, None, None);
        true
    });
    let success = if placed {
        stack.shrink(1);
        true
    } else {
        dispense_equipment(world, source, &mut stack)
    };
    Ok(Dispensed::optional(stack, success))
}

/// `RotationSegment.convertToSegment(Direction)`: vertical faces read as 0.
fn rotation_segment(facing: Direction) -> i32 {
    match facing {
        Direction::South | Direction::Up | Direction::Down => 0,
        Direction::West => 4,
        Direction::North => 8,
        Direction::East => 12,
    }
}
