//! `SweetBerryBushBlock`
//! (`net/minecraft/world/level/block/SweetBerryBushBlock.java`).
//!
//! A `VegetationBlock` that adds a `randomTick` and replaces no survival body,
//! so its two other halves come from [`super::vegetation`].

use super::LevelCtx;
use bbb_protocol::block_types::VanillaRandomTickOverride;
use bbb_protocol::packets::BlockPos;

use crate::fluid::above;
use crate::level_event::VanillaGameEvent;

use super::{
    block_type_of, int_property, random_tick_override, with_property, SET_BLOCK_UPDATE_CLIENTS,
};

/// `SweetBerryBushBlock.MAX_AGE` (`SweetBerryBushBlock.java:37`).
pub const MAX_AGE: i32 = 3;

/// `SweetBerryBushBlock.AGE` (`SweetBerryBushBlock.java:38`).
const AGE: &str = "age";

/// `random_tick_override == SweetBerryBushBlock` is the generated statement
/// that this block reaches that class's bodies; one registration.
#[must_use]
pub fn is_sweet_berry_bush(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(random_tick_override)
        == Some(VanillaRandomTickOverride::SweetBerryBushBlock)
}

#[must_use]
pub fn age(block_state_id: i32) -> Option<i32> {
    int_property(block_state_id, AGE)
}

#[must_use]
pub fn with_age(block_state_id: i32, age: i32) -> Option<i32> {
    with_property(block_state_id, AGE, &age.to_string())
}

/// The one-in-five growth draw (`SweetBerryBushBlock.java:74`).
const GROWTH_CHANCE: i32 = 5;

/// The raw brightness the cell *above* the bush must reach before it grows
/// (`SweetBerryBushBlock.java:74`).
const GROWTH_LIGHT_THRESHOLD: i32 = 9;

///
/// The draw comes before the light read, so a bush in the dark still advances
/// the level's stream one in five ticks.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };
    if age >= MAX_AGE {
        return;
    }
    if level.level_random().next_int(GROWTH_CHANCE) != 0 {
        return;
    }
    if level.raw_brightness(above(pos), 0) < GROWTH_LIGHT_THRESHOLD {
        return;
    }
    let Some(new_state) = with_property(block_state_id, AGE, &(age + 1).to_string()) else {
        return;
    };
    level.set_block(
        pos,
        new_state,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(new_state));
}

pub fn inside_applies(living: bool, entity_type: i32) -> bool {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_FOX_ID};
    living
        && !matches!(
            entity_type,
            VANILLA_ENTITY_TYPE_FOX_ID | VANILLA_ENTITY_TYPE_BEE_ID
        )
}

pub fn stuck_multiplier() -> bbb_protocol::packets::Vec3d {
    bbb_protocol::packets::Vec3d {
        x: f64::from(0.8_f32),
        y: 0.75,
        z: f64::from(0.8_f32),
    }
}

pub fn movement_hurts(movement: bbb_protocol::packets::Vec3d) -> bool {
    let threshold = f64::from(0.003_f32);
    movement.x * movement.x + movement.z * movement.z > 0.0
        && (movement.x.abs() >= threshold || movement.z.abs() >= threshold)
}
