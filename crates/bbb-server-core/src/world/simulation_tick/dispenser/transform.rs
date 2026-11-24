//! The behaviours that change the block in front in place: bone meal,
//! glowstone into a respawn anchor and honeycomb wax.

use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::block_behavior::bonemeal::{grow_crop, grow_water_plant};
use crate::block_behavior::respawn_anchor::{
    charges, is_respawn_anchor, with_charges, MAX_CHARGES,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::item_use_on::block_transform::waxed_state;
use crate::world::simulation_level::LevelCtx;
use crate::world::ServerWorld;

/// `LevelEvent.PARTICLES_AND_SOUND_PLANT_GROWTH`.
const PLANT_GROWTH: i32 = 1505;
/// `LevelEvent.PARTICLES_AND_SOUND_WAX_ON`.
const WAX_ON: i32 = 3003;

/// `BoneMealItem.growCrop` or `growWaterPlant` (`DispenseItemBehavior.java:217-232`).
pub(super) fn bone_meal(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let grown = world.with_level(source.level, |level| {
        let grown = grow_crop(level, target) || grow_water_plant(level, target, None);
        if grown {
            LevelCtx::level_event(level, PLANT_GROWTH, target, 15);
        }
        grown
    });
    if grown {
        stack.shrink(1);
    }
    Ok(Dispensed::optional(stack, grown))
}

/// `RespawnAnchorBlock.charge` for a respawn anchor that is not full.
pub(super) fn glowstone(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let state = world.with_level(source.level, |level| level.block_state(target));
    if !is_respawn_anchor(state) {
        return world.default_dispense(source, stack);
    }
    let Some(charged) = charges(state)
        .filter(|&charge| charge != MAX_CHARGES)
        .and_then(|charge| with_charges(state, charge + 1))
    else {
        return Ok(Dispensed::optional(stack, false));
    };
    world.with_level(source.level, |level| {
        level.set_block(target, charged, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
        level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, target, None, Some(charged));
        level.play_block_sound(target, "minecraft:block.respawn_anchor.charge");
    });
    stack.shrink(1);
    Ok(Dispensed::optional(stack, true))
}

/// `HoneycombItem.getWaxed` on the block in front.
pub(super) fn honeycomb(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let waxed = world.with_level(source.level, |level| {
        let waxed = waxed_state(level.block_state(target))?;
        level.set_block(target, waxed, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
        LevelCtx::level_event(level, WAX_ON, target, 0);
        Some(())
    });
    if waxed.is_none() {
        return world.default_dispense(source, stack);
    }
    stack.shrink(1);
    Ok(Dispensed::optional(stack, true))
}
