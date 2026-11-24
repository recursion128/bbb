//! Hydration scheduling for `DriedGhastBlock`.

use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, FlowingFluid};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VanillaTickOverride};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{facing_y_rot, opposite};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, get_state_for_placement_override, int_property, tick_override,
    BlockMobSpawnRequest, BlockStateAt, SET_BLOCK_UPDATE_CLIENTS,
};

const FACING: &str = "facing";
const HYDRATION_LEVEL: &str = "hydration";
const WATERLOGGED: &str = "waterlogged";
const MAX_HYDRATION_LEVEL: i32 = 3;
const HYDRATION_TICK_DELAY: i32 = 5_000;

#[must_use]
pub fn is_dried_ghast(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(tick_override)
        == Some(VanillaTickOverride::DriedGhastBlock)
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let sound = if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        "minecraft:block.dried_ghast.place_in_water"
    } else {
        "minecraft:block.dried_ghast.place"
    };
    level.play_block_sound(pos, sound);
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::DriedGhastBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    let state = with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )?;
    if !matches!(
        horizontal_direction,
        Direction::North | Direction::South | Direction::West | Direction::East
    ) {
        return None;
    }
    with_property(
        state,
        FACING,
        direction_serialized_name(opposite(horizontal_direction)),
    )
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

fn write_hydration(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    hydration: i32,
) {
    let Some(next) = with_property(block_state_id, HYDRATION_LEVEL, &hydration.to_string()) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(block_state_id));
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(hydration) = int_property(block_state_id, HYDRATION_LEVEL) else {
        return;
    };
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        if hydration < MAX_HYDRATION_LEVEL {
            level.play_block_sound(pos, "minecraft:block.dried_ghast.transition");
            write_hydration(level, pos, block_state_id, hydration + 1);
        } else if hydration == MAX_HYDRATION_LEVEL {
            let Some(facing) = direction_property(block_state_id, FACING) else {
                return;
            };
            let Some(y_rot) = facing_y_rot(facing) else {
                return;
            };
            level.remove_block(pos);
            let _ = level.request_block_mob_spawn(BlockMobSpawnRequest::HydratedHappyGhast {
                position: Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y),
                    z: f64::from(pos.z) + 0.5,
                },
                y_rot,
            });
        }
    } else if hydration > 0 {
        write_hydration(level, pos, block_state_id, hydration - 1);
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let needs_tick = bool_property(block_state_id, WATERLOGGED) == Some(true)
        || int_property(block_state_id, HYDRATION_LEVEL).is_some_and(|level| level > 0);
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    if needs_tick && !level.has_scheduled_tick(pos, tick_type) {
        level.schedule_block_tick(pos, tick_type, HYDRATION_TICK_DELAY, TickPriority::Normal);
    }
}
