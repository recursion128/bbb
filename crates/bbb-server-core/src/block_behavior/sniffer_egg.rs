//! `SnifferEggBlock`: boosted placement and scheduled hatching.

use super::LevelCtx;
use crate::fluid::with_property;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::VANILLA_BLOCK_SNIFFER_EGG_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{below, block_type_of, int_property, BlockMobSpawnRequest, SET_BLOCK_UPDATE_CLIENTS};

pub(super) const LEVEL_EVENT_SNIFFER_EGG_PLACED: i32 = 3009;
const RANDOM_HATCH_OFFSET_TICKS: i32 = 300;
const REGULAR_PROGRESSION_TICKS: i32 = 8_000;
const BOOSTED_PROGRESSION_TICKS: i32 = 4_000;
const HATCH_BOOST_TAG: &str = "minecraft:sniffer_egg_hatch_boost";
const HATCH: &str = "hatch";
const MAX_HATCH_LEVEL: i32 = 2;
const CRACK_SOUND: &str = "minecraft:block.sniffer_egg.crack";
const HATCH_SOUND: &str = "minecraft:block.sniffer_egg.hatch";

pub(super) fn is_sniffer_egg(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_SNIFFER_EGG_ID)
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if !is_sniffer_egg(block_state_id) {
        return;
    }
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    let boosted = level.block_state_in_tag(level.block_state(below(pos)), HATCH_BOOST_TAG);

    if boosted {
        level.level_event(LEVEL_EVENT_SNIFFER_EGG_PLACED, pos, 0);
    }
    level.block_game_event(VanillaGameEvent::BLOCK_PLACE, pos, Some(block_state_id));
    let delay = if boosted {
        BOOSTED_PROGRESSION_TICKS
    } else {
        REGULAR_PROGRESSION_TICKS
    } + level.level_random().next_int(RANDOM_HATCH_OFFSET_TICKS);
    level.schedule_block_tick(pos, tick_type, delay, TickPriority::Normal);
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(hatch) = int_property(block_state_id, HATCH) else {
        return;
    };
    if !(0..=MAX_HATCH_LEVEL).contains(&hatch) {
        return;
    }

    let pitch = 0.9 + level.level_random().next_float() * 0.2;
    if hatch < MAX_HATCH_LEVEL {
        let Some(next) = with_property(block_state_id, HATCH, &(hatch + 1).to_string()) else {
            return;
        };
        level.play_block_sound_with_volume_pitch(pos, CRACK_SOUND, 0.7, pitch);
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        return;
    }

    level.play_block_sound_with_volume_pitch(pos, HATCH_SOUND, 0.7, pitch);
    level.destroy_block(pos, false);
    let y_rot = wrap_degrees(level.level_random().next_float() * 360.0);
    let _ = level.request_block_mob_spawn(BlockMobSpawnRequest::HatchedSniffer {
        position: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        y_rot,
    });
}

fn wrap_degrees(angle: f32) -> f32 {
    let mut wrapped = angle % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
