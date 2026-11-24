//! `LeavesBlock` (`net/minecraft/world/level/block/LeavesBlock.java`).
//!
//! The scheduled-tick template for this slice. Leaves carry a `DISTANCE`
//! property that measures how far they are from the nearest log; a shape
//! update from any side arms a one-tick block tick, that tick recomputes the
//! distance from the six neighbours, and the write it performs arms the next
//! ring outward. A leaf that reaches `DECAY_DISTANCE` becomes randomly
//! ticking, and its random tick drops it.
//!
//! # Vanilla 26.1 has no `neighborChanged` here
//!
//!
//! # And no particle or level event
//!

use super::{block_tag_matches, BlockShapeCtx, BlockTags, LevelCtx};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::block_queries::{distance, is_leaves, optional_distance_at};

use crate::fluid::{FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, with_property, BlockStateAt,
    DIRECTION_VALUES,
};

/// `LeavesBlock.DECAY_DISTANCE` (`LeavesBlock.java:30`): both the maximum of
/// `BlockStateProperties.DISTANCE` (`BlockStateProperties.java:103`,
/// `IntegerProperty.create("distance", 1, 7)`) and the value at which a leaf
/// starts decaying.
pub const DECAY_DISTANCE: i32 = 7;

/// `LeavesBlock.TICK_DELAY` (`LeavesBlock.java:35`). `updateShape` spells the
/// same 1 out as a literal (`:105`).
pub const TICK_DELAY: i32 = 1;

const DISTANCE: &str = "distance";
const PERSISTENT: &str = "persistent";
const WATERLOGGED: &str = "waterlogged";
const PREVENTS_NEARBY_LEAF_DECAY_TAG: &str = "minecraft:prevents_nearby_leaf_decay";

/// `LeavesBlock.PERSISTENT`'s value.
#[must_use]
pub fn persistent(block_state_id: i32) -> Option<bool> {
    bool_property(block_state_id, PERSISTENT)
}

#[must_use]
pub fn distance_at(block_state_id: i32) -> i32 {
    optional_distance_at(block_state_id).unwrap_or(DECAY_DISTANCE)
}

fn distance_at_live(
    block_state_id: i32,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<i32> {
    if block_tag_matches(block_state_id, PREVENTS_NEARBY_LEAF_DECAY_TAG, membership) {
        Some(0)
    } else {
        Some(distance(block_state_id).unwrap_or(DECAY_DISTANCE))
    }
}

///
/// The minimum over the six neighbours' [`distance_at`] plus one, clamped at
/// [`DECAY_DISTANCE`] by starting there. The loop breaks as soon as it reaches
/// 1 (`:118-120`), which is why `Direction.values()` order and not one of the
/// neighbour-update orders is the order to walk.
#[must_use]
pub(crate) fn update_distance(level: &LevelCtx<'_, '_>, block_state_id: i32, pos: BlockPos) -> i32 {
    if !is_leaves(block_state_id) {
        return block_state_id;
    }
    let mut new_distance = DECAY_DISTANCE;
    let mut unavailable = false;
    for direction in DIRECTION_VALUES {
        let neighbour = relative(pos, direction);
        let Some(neighbour_distance) =
            distance_at_live(level.block_state(neighbour), &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            })
        else {
            unavailable = true;
            continue;
        };
        new_distance = new_distance.min(neighbour_distance + 1);
        if new_distance == 1 {
            return with_property(block_state_id, DISTANCE, "1").unwrap_or(block_state_id);
        }
    }
    if unavailable {
        return block_state_id;
    }
    with_property(block_state_id, DISTANCE, &new_distance.to_string()).unwrap_or(block_state_id)
}

fn placement_distance_at(tags: &dyn BlockTags, block_state_id: i32) -> Option<i32> {
    block_type_of(block_state_id)?;
    if tags.block_in_tag(block_state_id, PREVENTS_NEARBY_LEAF_DECAY_TAG) {
        return Some(0);
    }
    if is_leaves(block_state_id) {
        distance(block_state_id)
    } else {
        Some(DECAY_DISTANCE)
    }
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);

    let state = default_block_state_id(block_type_id);
    if !is_leaves(state) {
        return None;
    }
    let state = with_property(state, PERSISTENT, "true")?;
    let state = with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )?;

    let mut new_distance = DECAY_DISTANCE;
    for direction in DIRECTION_VALUES {
        let neighbour = block_state_at(relative(pos, direction));
        new_distance = new_distance.min(placement_distance_at(tags, neighbour)? + 1);
        if new_distance == 1 {
            break;
        }
    }
    with_property(state, DISTANCE, &new_distance.to_string())
}

#[must_use]
pub fn decaying(block_state_id: i32) -> bool {
    persistent(block_state_id) == Some(false) && distance(block_state_id) == Some(DECAY_DISTANCE)
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if decaying(block_state_id) {
        level.drop_resources(pos, block_state_id);
        level.remove_block(pos);
    }
}

///
/// The write uses `Block.UPDATE_ALL`, so a distance that actually changed
/// pushes a shape update into the next ring of leaves, and that is what
/// re-arms their ticks.
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let updated = update_distance(level, block_state_id, pos);
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

///
/// Two independent effects and no state change: a waterlogged leaf re-arms its
/// water, and the leaf arms its own distance tick unless the neighbour already
/// justifies the distance it has. The `distanceFromNeighbor != 1` half of the
/// guard (`:104`) means only an adjacent *log* can ever suppress the tick;
/// every other neighbour schedules one.
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    neighbour_state: i32,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    let Some(distance_from_neighbour) = distance_at_live(neighbour_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    })
    .map(|distance| distance + 1) else {
        return block_state_id;
    };

    if distance_from_neighbour != 1 || distance(block_state_id) != Some(distance_from_neighbour) {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
    }

    block_state_id
}
