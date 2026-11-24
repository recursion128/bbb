//! `FarmlandBlock` (`net/minecraft/world/level/block/FarmlandBlock.java`).
//!

use super::{block_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};

use bbb_protocol::packets::{BlockPos, Direction};

use bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID;

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{
    above, block_state_is_solid, default_block_state_id, int_property, offset, with_property,
    VanillaGameEvent, SET_BLOCK_UPDATE_CLIENTS,
};

/// `FarmlandBlock.MAX_MOISTURE` (`FarmlandBlock.java:34`), which is also the
/// maximum of `BlockStateProperties.MOISTURE`
/// (`BlockStateProperties.java:117`, `IntegerProperty.create("moisture", 0,
/// 7)`).
pub const MAX_MOISTURE: i32 = 7;

/// The delay `updateShape` arms its own tick with (`FarmlandBlock.java:58`).
pub const TICK_DELAY: i32 = 1;

/// The half-width of the box `isNearWater` scans (`FarmlandBlock.java:132`,
/// `pos.offset(-4, 0, -4)` to `pos.offset(4, 1, 4)`). The box is 9×2×9: it
/// reaches four blocks out horizontally, the farmland's own layer and the one
/// directly above it, and no lower.
pub const WATER_SCAN_RADIUS: i32 = 4;

pub(super) const MOISTURE: &str = "moisture";
const MAINTAINS_FARMLAND_TAG: &str = "minecraft:maintains_farmland";
const WATER_TAG: &str = "minecraft:water";

/// `FarmlandBlock.MOISTURE`'s value.
#[must_use]
pub fn moisture(block_state_id: i32) -> Option<i32> {
    int_property(block_state_id, MOISTURE)
}

#[must_use]
pub fn should_maintain_farmland<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    block_tag_matches(
        level.block_state(above(pos)),
        MAINTAINS_FARMLAND_TAG,
        membership,
    )
}

#[must_use]
pub(crate) fn is_near_water(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    for dx in -WATER_SCAN_RADIUS..=WATER_SCAN_RADIUS {
        for dy in 0..=1 {
            for dz in -WATER_SCAN_RADIUS..=WATER_SCAN_RADIUS {
                let fluid = level.fluid_state(offset(pos, dx, dy, dz));
                if !fluid.is_empty() && level.fluid_identity_in_tag(fluid.identity(), WATER_TAG) {
                    return true;
                }
            }
        }
    }
    false
}

#[must_use]
pub fn can_survive<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    Some(
        !block_state_is_solid(level.block_state(above(pos)))
            || should_maintain_farmland(level, pos, membership),
    )
}

pub(super) fn can_survive_at_state(
    pos: BlockPos,
    tags: &dyn super::BlockTags,
    block_state_at: super::BlockStateAt,
) -> Option<bool> {
    let above_state = block_state_at(above(pos));
    super::block_type_of(above_state)?;
    Some(
        !block_state_is_solid(above_state)
            || tags.block_in_tag(above_state, MAINTAINS_FARMLAND_TAG),
    )
}

/// The sourceless `turnToDirt` of a block tick (`FarmlandBlock.java:121-125`):
/// the entity lift waits for the tick's drain, which holds the events.
pub(crate) fn turn_to_dirt(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let old_state = level.block_state(pos);
    let dirt = default_block_state_id(VANILLA_BLOCK_DIRT_ID);
    bbb_server_worldgen::WorldGenLevel::push_entities_up(level, old_state, dirt, pos);
    turn_to_dirt_with_source(level, pos, None);
}

pub(crate) fn turn_to_dirt_with_source(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    source_entity_id: Option<i32>,
) {
    let dirt = default_block_state_id(VANILLA_BLOCK_DIRT_ID);
    level.set_block(
        pos,
        dirt,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        source_entity_id,
        Some(dirt),
    );
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let _ = block_state_id;
    if can_survive(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) == Some(false)
    {
        turn_to_dirt(level, pos);
    }
}

///
/// Wet farmland jumps straight to [`MAX_MOISTURE`]; dry farmland loses one
/// step per tick and only turns over once it has run out **and** has no crop
/// on it. The moisture writes use flag 2 (`:99`, `:104`), the turn-over uses
/// `setBlockAndUpdate` through [`turn_to_dirt`].
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(moisture) = moisture(block_state_id) else {
        return;
    };
    if !(level.is_raining_at(above(pos)) || is_near_water(level, pos)) {
        if moisture > 0 {
            if let Some(drier) =
                with_property(block_state_id, MOISTURE, &(moisture - 1).to_string())
            {
                level.set_block(
                    pos,
                    drier,
                    SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        } else if !should_maintain_farmland(level, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) {
            turn_to_dirt(level, pos);
        }
    } else if moisture < MAX_MOISTURE {
        if let Some(wet) = with_property(block_state_id, MOISTURE, &MAX_MOISTURE.to_string()) {
            level.set_block(
                pos,
                wet,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

///
/// Returns the state unchanged in every case; the only effect is the tick it
/// arms when something lands on top that farmland cannot survive.
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Up
        && can_survive(level, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) == Some(false)
    {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
        }
    }
    block_state_id
}
