//! `MushroomBlock` (`net/minecraft/world/level/block/MushroomBlock.java`).

use super::{block_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use crate::fluid::below;
use bbb_protocol::block_types::VanillaRandomTickOverride;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::block_state_is_air;

use super::support::solid_render;
use super::{
    air_block_state_id, block_type_of, offset, random_tick_override, SET_BLOCK_UPDATE_CLIENTS,
};

const LIGHT_LIMIT: i32 = 13;
const DENSITY_LIMIT: i32 = 5;
const OVERRIDES_LIGHT_REQUIREMENT_TAG: &str = "minecraft:overrides_mushroom_light_requirement";

/// `BlockTags.OVERRIDES_MUSHROOM_LIGHT_REQUIREMENT` as the built-in members,
/// for the shape seam that has no live registry tags.
#[must_use]
pub(super) fn is_mushroom(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(random_tick_override)
        == Some(VanillaRandomTickOverride::MushroomBlock)
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id).filter(|_| is_mushroom(block_state_id))
    else {
        return;
    };
    if level.level_random().next_int(25) != 0 {
        return;
    }

    let mut remaining = DENSITY_LIMIT;
    for dz in -4..=4 {
        for dy in -1..=1 {
            for dx in -4..=4 {
                if block_type_of(level.block_state(offset(pos, dx, dy, dz))) == Some(block_type_id)
                {
                    remaining -= 1;
                    if remaining <= 0 {
                        return;
                    }
                }
            }
        }
    }

    let mut base = pos;
    let mut candidate = random_offset(level, base);
    for _ in 0..4 {
        if block_state_is_air(level.block_state(candidate))
            && can_survive(level, block_state_id, candidate, &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            }) == Some(true)
        {
            base = candidate;
        }
        candidate = random_offset(level, base);
    }
    if block_state_is_air(level.block_state(candidate))
        && can_survive(level, block_state_id, candidate, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) == Some(true)
    {
        level.set_block(
            candidate,
            block_state_id,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

fn random_offset(level: &mut LevelCtx<'_, '_>, pos: BlockPos) -> BlockPos {
    let dx = level.level_random().next_int(3) - 1;
    let dy = level.level_random().next_int(2) - level.level_random().next_int(2);
    let dz = level.level_random().next_int(3) - 1;
    offset(pos, dx, dy, dz)
}

#[must_use]
pub(super) fn can_survive<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if !is_mushroom(block_state_id) {
        return None;
    }
    let below_state = level.block_state(crate::fluid::below(pos));
    survives_on(
        below_state,
        block_tag_matches(below_state, OVERRIDES_LIGHT_REQUIREMENT_TAG, membership),
        || Some(level.raw_brightness(pos, 0).unwrap_or(0)),
    )
}

fn survives_on(
    support: i32,
    overrides_light_requirement: bool,
    brightness: impl FnOnce() -> Option<i32>,
) -> Option<bool> {
    if overrides_light_requirement {
        return Some(true);
    }
    Some(brightness()? < LIGHT_LIMIT && solid_render(support)?)
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    match can_survive(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) {
        Some(false) => air_block_state_id(),
        Some(true) | None => block_state_id,
    }
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let below_state = level.block_state(below(pos));
    let tags = level.tags();
    survives_on(
        below_state,
        tags.block_in_tag(below_state, OVERRIDES_LIGHT_REQUIREMENT_TAG),
        || level.raw_brightness(pos, 0),
    )
}
