//!
//! Two blocks and one mechanism: a dry sponge sweeps the water out of the
//! sixty-five cells `BlockPos.breadthFirstTraversal` reaches around it and
//! turns wet if it took any, and a wet sponge dries again the moment it lands
//! somewhere water evaporates. What the sweep does to a cell depends on how
//! that cell holds its water -- a source and a waterlogged block go through
//! [`super::bucket_pickup`], flowing water is overwritten, and kelp and
//! seagrass drop their items first.

use super::LevelCtx;

use bbb_protocol::block_types::{
    vanilla_block_type, VanillaOnPlaceOverride, VANILLA_BLOCK_KELP_ID, VANILLA_BLOCK_KELP_PLANT_ID,
    VANILLA_BLOCK_SEAGRASS_ID, VANILLA_BLOCK_SPONGE_ID, VANILLA_BLOCK_TALL_SEAGRASS_ID,
    VANILLA_BLOCK_WET_SPONGE_ID,
};
use bbb_protocol::packets::BlockPos;

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_server_types::directions::relative;

use super::{
    air_block_state_id, block_type_of, bucket_pickup, default_block_state_id, state_is_block,
    DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

/// `SpongeBlock.MAX_DEPTH` (`SpongeBlock.java:19`).
pub const MAX_DEPTH: i32 = 6;

/// `SpongeBlock.MAX_COUNT` (`SpongeBlock.java:20`), which the traversal is
/// handed as `65` rather than as this constant (`:55`).
pub const MAX_COUNT: i32 = 64;

/// `SoundEvents.SPONGE_ABSORB` (`SpongeBlock.java:50`).
const ABSORB_SOUND: &str = "minecraft:block.sponge.absorb";

/// `SoundEvents.WET_SPONGE_DRIES` (`WetSpongeBlock.java:32`).
const DRIES_SOUND: &str = "minecraft:block.wet_sponge.dries";

/// `LevelEvent.PARTICLES_WATER_EVAPORATING` (`LevelEvent.java:60`), raised
/// only here.
const LEVEL_EVENT_PARTICLES_WATER_EVAPORATING: i32 = 2009;
const WATER_TAG: &str = "minecraft:water";

fn declares_on_place(block_state_id: i32, class: VanillaOnPlaceOverride) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.on_place_override)
        == Some(class)
}

#[must_use]
pub fn is_sponge(block_state_id: i32) -> bool {
    declares_on_place(block_state_id, VanillaOnPlaceOverride::SpongeBlock)
}

#[must_use]
pub fn is_wet_sponge(block_state_id: i32) -> bool {
    declares_on_place(block_state_id, VanillaOnPlaceOverride::WetSpongeBlock)
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(old_state) != block_type_of(block_state_id) {
        try_absorb_water(level, pos);
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, _block_state_id: i32) {
    try_absorb_water(level, pos);
}

pub(super) fn wet_sponge_on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    _block_state_id: i32,
) {
    if !level.water_evaporates(pos) {
        return;
    }
    level.set_block(
        pos,
        default_block_state_id(VANILLA_BLOCK_SPONGE_ID),
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.level_event(LEVEL_EVENT_PARTICLES_WATER_EVAPORATING, pos, 0);
    level.play_block_sound(pos, DRIES_SOUND);
}

fn try_absorb_water(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if !remove_water_breadth_first_search(level, pos) {
        return;
    }
    // `:49`: flag 2 is `UPDATE_CLIENTS`.
    level.set_block(
        pos,
        default_block_state_id(VANILLA_BLOCK_WET_SPONGE_ID),
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.play_block_sound(pos, ABSORB_SOUND);
}

fn remove_water_breadth_first_search(level: &mut LevelCtx<'_, '_>, start_pos: BlockPos) -> bool {
    let reached = breadth_first_traversal(start_pos, MAX_DEPTH, MAX_COUNT + 1, &mut |pos| {
        if pos == start_pos {
            return true;
        }
        let state = level.block_state(pos);
        if !(level.fluid_identity_in_tag(level.fluid_state(pos).identity(), WATER_TAG)) {
            return false;
        }
        if bucket_pickup::pickup_block(level, pos, state, false).is_some() {
            return true;
        }
        if bucket_pickup::is_liquid_block(state) {
            level.set_block(
                pos,
                air_block_state_id(),
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        } else {
            if !drops_before_it_goes(state) {
                return false;
            }
            // `:77-78`: none of the four carries a block entity, so the
            // four-argument `dropResources` is the plain one.
            level.drop_resources(pos, state);
            level.set_block(
                pos,
                air_block_state_id(),
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        true
    });
    reached > 1
}

fn drops_before_it_goes(block_state_id: i32) -> bool {
    [
        VANILLA_BLOCK_KELP_ID,
        VANILLA_BLOCK_KELP_PLANT_ID,
        VANILLA_BLOCK_SEAGRASS_ID,
        VANILLA_BLOCK_TALL_SEAGRASS_ID,
    ]
    .iter()
    .any(|id| state_is_block(block_state_id, *id))
}

fn breadth_first_traversal(
    start_pos: BlockPos,
    max_depth: i32,
    max_count: i32,
    node: &mut dyn FnMut(BlockPos) -> bool,
) -> i32 {
    debug_assert!(max_depth <= MAX_DEPTH);
    const SIDE: usize = (MAX_DEPTH as usize) * 2 + 1; // 13
    const WORDS: usize = (SIDE * SIDE * SIDE).div_ceil(64); // 35
    let mut queue = std::collections::VecDeque::with_capacity(64);
    queue.push_back((start_pos, 0));
    let mut visited = [0_u64; WORDS];
    let mut count = 0;
    while let Some((current_pos, depth)) = queue.pop_front() {
        let dx = current_pos.x - start_pos.x + MAX_DEPTH;
        let dy = current_pos.y - start_pos.y + MAX_DEPTH;
        let dz = current_pos.z - start_pos.z + MAX_DEPTH;
        let bit = (dx as usize) * SIDE * SIDE + (dy as usize) * SIDE + (dz as usize);
        let mask = 1_u64 << (bit & 63);
        if (visited[bit >> 6] & mask) != 0 {
            continue;
        }
        visited[bit >> 6] |= mask;
        if !node(current_pos) {
            continue;
        }
        count += 1;
        if count >= max_count {
            return count;
        }
        if depth < max_depth {
            for direction in DIRECTION_VALUES {
                queue.push_back((relative(current_pos, direction), depth + 1));
            }
        }
    }
    count
}
