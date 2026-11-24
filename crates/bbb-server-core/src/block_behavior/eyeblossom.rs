//! `EyeblossomBlock` (`net/minecraft/world/level/block/EyeblossomBlock.java`).

use super::LevelCtx;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID, VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID,
};
use bbb_protocol::packets::BlockPos;

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{block_type_of, default_block_state_id, offset, VanillaGameEvent};

const OPEN_LONG_SOUND: &str = "minecraft:block.eyeblossom.open_long";
const CLOSE_LONG_SOUND: &str = "minecraft:block.eyeblossom.close_long";
const OPEN_SHORT_SOUND: &str = "minecraft:block.eyeblossom.open";
const CLOSE_SHORT_SOUND: &str = "minecraft:block.eyeblossom.close";
const OPEN_PARTICLE_COLOR: i32 = 16_545_810;
const CLOSE_PARTICLE_COLOR: i32 = 6_250_335;

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if let Some(sound) = try_changing_state(level, pos, block_state_id, true) {
        level.play_block_sound(pos, sound);
    }
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if let Some(sound) = try_changing_state(level, pos, block_state_id, false) {
        level.play_block_sound(pos, sound);
    }
}

fn try_changing_state(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    long_sound: bool,
) -> Option<&'static str> {
    let currently_open = match block_type_of(block_state_id)? {
        VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID => true,
        VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID => false,
        _ => return None,
    };
    let block = BlockTickType::of_block_state(block_state_id)?;
    let should_be_open = level.eyeblossom_open(pos)?.to_boolean(currently_open);
    if should_be_open == currently_open {
        return None;
    }

    let next_block_type = if currently_open {
        VANILLA_BLOCK_CLOSED_EYEBLOSSOM_ID
    } else {
        VANILLA_BLOCK_OPEN_EYEBLOSSOM_ID
    };
    let next_state = default_block_state_id(next_block_type);
    level.set_block(
        pos,
        next_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(block_state_id));
    spawn_transform_particle(level, pos, !currently_open);

    for dz in -3..=3 {
        for dy in -2..=2 {
            for dx in -3..=3 {
                let nearby = offset(pos, dx, dy, dz);
                if level.block_state(nearby) != block_state_id {
                    continue;
                }
                let distance = f64::from(dx * dx + dy * dy + dz * dz).sqrt();
                let min_delay = (distance * 5.0) as i32;
                let max_delay = (distance * 10.0) as i32;
                let delay = min_delay + level.level_random().next_int(max_delay - min_delay + 1);
                level.schedule_block_tick(nearby, block, delay, TickPriority::Normal);
            }
        }
    }

    Some(match (!currently_open, long_sound) {
        (true, true) => OPEN_LONG_SOUND,
        (false, true) => CLOSE_LONG_SOUND,
        (true, false) => OPEN_SHORT_SOUND,
        (false, false) => CLOSE_SHORT_SOUND,
    })
}

fn spawn_transform_particle(level: &mut LevelCtx<'_, '_>, pos: BlockPos, now_open: bool) {
    let lifetime = 0.5 + level.level_random().next_double();
    let velocity_x = level.level_random().next_double() - 0.5;
    let velocity_y = level.level_random().next_double() + 1.0;
    let velocity_z = level.level_random().next_double() - 0.5;
    let start_x = f64::from(pos.x) + 0.5;
    let start_y = f64::from(pos.y) + 0.5;
    let start_z = f64::from(pos.z) + 0.5;
    level.send_trail_particle(
        [start_x, start_y, start_z],
        [
            start_x + velocity_x * lifetime,
            start_y + velocity_y * lifetime,
            start_z + velocity_z * lifetime,
        ],
        if now_open {
            OPEN_PARTICLE_COLOR
        } else {
            CLOSE_PARTICLE_COLOR
        },
        (20.0 * lifetime) as i32,
    );
}
