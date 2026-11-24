//!
//! `DISTANCE` is `LeavesBlock`'s idea run over a different neighbourhood --
//! the cell below counts for nothing extra, the four sides cost one each --
//! and every update re-arms a one-tick re-measure rather than answering on the
//! spot, so a tower rewrites itself from the break outwards one tick at a
//! time. `BOTTOM` is a second derived bit that only changes the shape a player
//! stands on.
//!

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{VanillaCanSurviveOverride, VANILLA_BLOCK_SCAFFOLDING_ID};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property, FlowingFluid, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, can_replace_for_placement, can_survive_override, face_sturdy_at,
    int_property, state_is_block, BlockPlacementContext, BlockStateAt, FallingBlockFallRequest,
};

/// `BlockStateProperties.STABILITY_DISTANCE` and `BOTTOM`
/// (`ScaffoldingBlock.java:38`, `:40`).
const DISTANCE: &str = "distance";
const BOTTOM: &str = "bottom";
const WATERLOGGED: &str = "waterlogged";

/// `ScaffoldingBlock.STABILITY_MAX_DISTANCE` (`ScaffoldingBlock.java:37`).
pub const STABILITY_MAX_DISTANCE: i32 = 7;

/// `ScaffoldingBlock.TICK_DELAY` (`ScaffoldingBlock.java:28`), which the class
/// spells as a literal at both of its `scheduleTick` calls.
const TICK_DELAY: i32 = 1;

#[must_use]
pub fn is_scaffolding(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::ScaffoldingBlock)
}

fn get_distance(pos: BlockPos, block_state_at: BlockStateAt) -> Option<i32> {
    let below_pos = below(pos);
    let below_state = block_state_at(below_pos);
    let mut distance = STABILITY_MAX_DISTANCE;
    if state_is_block(below_state, VANILLA_BLOCK_SCAFFOLDING_ID) {
        distance = int_property(below_state, DISTANCE)?;
    } else if face_sturdy_at(below_state, below_pos, Direction::Up, SupportType::Full) {
        return Some(0);
    }
    for direction in HORIZONTAL {
        let side_state = block_state_at(relative(pos, direction));
        if state_is_block(side_state, VANILLA_BLOCK_SCAFFOLDING_ID) {
            distance = distance.min(int_property(side_state, DISTANCE)? + 1);
            if distance == 1 {
                break;
            }
        }
    }
    Some(distance)
}

fn get_distance_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    get_distance(pos, &mut |at| level.block_state(at))
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<i32> {
    let distance = get_distance(pos, block_state_at)?;
    let bottom = is_bottom(pos, distance, block_state_at);
    let fluid = block_state_fluid_state(block_state_at(pos));
    let waterlogged = fluid.is_source() && fluid.is_same_family(FluidFamily::Water);
    let state = with_property(
        super::default_block_state_id(block_type_id),
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )?;
    let state = with_property(state, DISTANCE, &distance.to_string())?;
    with_property(state, BOTTOM, if bottom { "true" } else { "false" })
}

///
/// `cell_at` is `(Level.isInWorldBounds, Level.getBlockState)`. Out of the
/// world still answers a state because `getDistance` reads neighbours that way;
/// the walk itself breaks on the bounds bit and skips the build-limit packet.
pub(super) fn update_placement_context(
    item_type_id: i32,
    context: BlockPlacementContext,
    cell_at: &mut dyn FnMut(BlockPos) -> (bool, i32),
) -> Option<BlockPlacementContext> {
    let replaced = cell_at(context.pos).1;
    if !is_scaffolding(replaced) {
        let at_max_distance =
            get_distance(context.pos, &mut |at| cell_at(at).1)? == STABILITY_MAX_DISTANCE;
        return if at_max_distance { None } else { Some(context) };
    }
    let direction = if context.secondary_use_active {
        if context.inside {
            opposite(context.clicked_face)
        } else {
            context.clicked_face
        }
    } else if context.clicked_face == Direction::Up {
        context.horizontal_direction
    } else {
        Direction::Up
    };
    let mut placement_pos = relative(context.pos, direction);
    let mut horizontal_distance = 0;
    while horizontal_distance < STABILITY_MAX_DISTANCE {
        let (in_world, replaced) = cell_at(placement_pos);
        if !in_world {
            break;
        }
        if !is_scaffolding(replaced) {
            if can_replace_for_placement(item_type_id, replaced, context)? {
                return context_at(item_type_id, context, placement_pos, direction, cell_at);
            }
            break;
        }
        placement_pos = relative(placement_pos, direction);
        if !matches!(direction, Direction::Down | Direction::Up) {
            horizontal_distance += 1;
        }
    }
    None
}

fn direction_step(direction: Direction) -> (f64, f64, f64) {
    match direction {
        Direction::Down => (0.0, -1.0, 0.0),
        Direction::Up => (0.0, 1.0, 0.0),
        Direction::North => (0.0, 0.0, -1.0),
        Direction::South => (0.0, 0.0, 1.0),
        Direction::West => (-1.0, 0.0, 0.0),
        Direction::East => (1.0, 0.0, 0.0),
    }
}

fn context_at(
    item_type_id: i32,
    context: BlockPlacementContext,
    pos: BlockPos,
    direction: Direction,
    cell_at: &mut dyn FnMut(BlockPos) -> (bool, i32),
) -> Option<BlockPlacementContext> {
    let (step_x, step_y, step_z) = direction_step(direction);
    let hit = BlockPlacementContext {
        pos,
        clicked_face: direction,
        click_x: 0.5 + step_x * 0.5,
        click_y: 0.5 + step_y * 0.5,
        click_z: 0.5 + step_z * 0.5,
        replacing_clicked: true,
        horizontal_direction: context.horizontal_direction,
        secondary_use_active: context.secondary_use_active,
        inside: false,
    };
    if can_replace_for_placement(item_type_id, cell_at(pos).1, hit)? {
        Some(hit)
    } else {
        Some(BlockPlacementContext {
            pos: relative(pos, direction),
            click_x: hit.click_x - step_x,
            click_y: hit.click_y - step_y,
            click_z: hit.click_z - step_z,
            replacing_clicked: false,
            ..hit
        })
    }
}

fn is_bottom(pos: BlockPos, distance: i32, block_state_at: BlockStateAt) -> bool {
    distance > 0 && !state_is_block(block_state_at(below(pos)), VANILLA_BLOCK_SCAFFOLDING_ID)
}

pub(super) fn can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    Some(get_distance(pos, block_state_at)? < STABILITY_MAX_DISTANCE)
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    let mut distance = STABILITY_MAX_DISTANCE;
    if state_is_block(support, VANILLA_BLOCK_SCAFFOLDING_ID) {
        let value = int_property(support, DISTANCE)?;
        distance = value;
    } else if face_sturdy_at(support, support_pos, Direction::Up, SupportType::Full) {
        return Some(true);
    }
    for direction in HORIZONTAL {
        let side = level.block_state(relative(pos, direction));
        if state_is_block(side, VANILLA_BLOCK_SCAFFOLDING_ID) {
            let side_distance = int_property(side, DISTANCE)?;
            distance = distance.min(side_distance + 1);
            if distance == 1 {
                break;
            }
        }
    }
    Some(distance < STABILITY_MAX_DISTANCE)
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    schedule_remeasure(&mut BlockShapeCtx::Live(level), pos, block_state_id);
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
    schedule_remeasure(level, pos, block_state_id);
    block_state_id
}

fn schedule_remeasure(level: &mut BlockShapeCtx<'_, '_, '_>, pos: BlockPos, block_state_id: i32) {
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
    }
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(distance) = get_distance_at(level, pos) else {
        return;
    };
    // `:119`: both derived values are measured before the branch, so the
    // state the falling arm hands the entity already carries them.
    let bottom = is_bottom(pos, distance, &mut |at| level.block_state(at));
    let Some(new_state) = measured(block_state_id, distance, bottom) else {
        return;
    };
    if distance == STABILITY_MAX_DISTANCE {
        // `:121-125`: a tower already adrift falls, and one only now cut at
        // the bottom breaks.
        if int_property(block_state_id, DISTANCE) == Some(STABILITY_MAX_DISTANCE) {
            let _ = level.request_falling_block_fall(FallingBlockFallRequest {
                pos,
                block_state_id: new_state,
                disable_drop: false,
                hurt_entities: None,
            });
        } else {
            level.destroy_block(pos, true);
        }
        return;
    }
    if new_state != block_state_id {
        level.set_block(
            pos,
            new_state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

fn measured(block_state_id: i32, distance: i32, bottom: bool) -> Option<i32> {
    let state = with_property(block_state_id, DISTANCE, &distance.to_string())?;
    with_property(state, BOTTOM, if bottom { "true" } else { "false" })
}
