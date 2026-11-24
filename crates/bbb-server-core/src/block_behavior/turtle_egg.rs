//! `TurtleEggBlock`: sand placement and the complete hatch lifecycle.

use super::LevelCtx;
use crate::fluid::with_property;
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VanillaRandomTickOverride};
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::{
    below, block_type_of, default_block_state_id, get_state_for_placement_override, int_property,
    random_tick_override, BlockMobSpawnRequest, BlockStateAt, SET_BLOCK_UPDATE_CLIENTS,
};

pub(super) const LEVEL_EVENT_TURTLE_EGG_PLACED: i32 = 2012;
const EGGS: &str = "eggs";
const HATCH: &str = "hatch";
const MAX_HATCH_LEVEL: i32 = 2;
const TURTLE_EGG_CRACK_SOUND: &str = "minecraft:entity.turtle.egg_crack";
const TURTLE_EGG_HATCH_SOUND: &str = "minecraft:entity.turtle.egg_hatch";

const SAND_TAG: &str = "minecraft:sand";

pub(crate) fn decrease_eggs(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> Option<()> {
    if block_type_of(state)? != bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID {
        return None;
    }
    let eggs = int_property(state, EGGS)?;
    let decreased = if eggs > 1 {
        Some(with_property(state, EGGS, &(eggs - 1).to_string())?)
    } else {
        None
    };
    let pitch = 0.9_f32 + level.level_random().next_float() * 0.2_f32;
    level.play_block_sound_with_volume_pitch(pos, "minecraft:entity.turtle.egg_break", 0.7, pitch);
    if let Some(decreased) = decreased {
        level.set_block(
            pos,
            decreased,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.block_game_event(VanillaGameEvent::BLOCK_DESTROY, pos, Some(state));
        level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, state);
    } else {
        level.destroy_block(pos, false);
    }
    Some(())
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::TurtleEggBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    if block_type_of(replaced)? != block_type_id {
        return Some(default_block_state_id(block_type_id));
    }
    let eggs = int_property(replaced, EGGS)?;
    with_property(replaced, EGGS, &eggs.saturating_add(1).min(4).to_string())
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, _block_state_id: i32) {
    if level.block_state_in_tag(level.block_state(below(pos)), SAND_TAG) {
        level.level_event(LEVEL_EVENT_TURTLE_EGG_PLACED, pos, 15);
    }
}

fn random_tick_requests(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<Vec<BlockMobSpawnRequest>> {
    if block_type_of(block_state_id).and_then(random_tick_override)
        != Some(VanillaRandomTickOverride::TurtleEggBlock)
    {
        return None;
    }
    let hatch = int_property(block_state_id, HATCH)?;
    let eggs = int_property(block_state_id, EGGS)?;
    if !(0..=MAX_HATCH_LEVEL).contains(&hatch) || !(1..=4).contains(&eggs) {
        return None;
    }

    let floor = level.block_state(below(pos));
    block_type_of(floor)?;
    let on_sand = level.block_state_in_tag(floor, SAND_TAG);
    // TurtleEggBlock.java:136-139.
    let chance = level.turtle_egg_hatch_chance(pos);
    if chance <= 0.0 || level.level_random().next_float() >= chance || !on_sand {
        return Some(Vec::new());
    }

    let pitch = 0.9 + level.level_random().next_float() * 0.2;
    if hatch < MAX_HATCH_LEVEL {
        let cracked = with_property(block_state_id, HATCH, &(hatch + 1).to_string())?;
        level.play_block_sound_with_volume_pitch(pos, TURTLE_EGG_CRACK_SOUND, 0.7, pitch);
        level.set_block(
            pos,
            cracked,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(block_state_id));
        return Some(Vec::new());
    }

    level.play_block_sound_with_volume_pitch(pos, TURTLE_EGG_HATCH_SOUND, 0.7, pitch);
    level.remove_block(pos);
    level.block_game_event(VanillaGameEvent::BLOCK_DESTROY, pos, Some(block_state_id));
    let mut requests = Vec::with_capacity(eggs as usize);
    for index in 0..eggs {
        level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, block_state_id);
        requests.push(BlockMobSpawnRequest::HatchedTurtle {
            position: Vec3d {
                x: f64::from(pos.x) + 0.3 + f64::from(index) * 0.2,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.3,
            },
            home_pos: pos,
        });
    }
    Some(requests)
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(requests) = random_tick_requests(level, pos, block_state_id) else {
        return;
    };
    for request in requests {
        let _ = level.request_block_mob_spawn(request);
    }
}
