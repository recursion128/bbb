//! `DaylightDetectorBlock` stored signal, invert, and `updateSignalStrength`.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseWithoutItemOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::mth_cos;

use super::{
    block_type_of, bool_property, cycle_property, int_property, BlockInteractionPlayer, LightLayer,
    SET_BLOCK_UPDATE_CLIENTS,
};

const POWER: &str = "power";
const INVERTED: &str = "inverted";

/// `Mth.DEG_TO_RAD` (`Mth.java:23`).
const DEG_TO_RAD: f32 = (std::f64::consts::PI / 180.0) as f32;
/// `(float) Math.PI` (`DaylightDetectorBlock.java:66`).
const PI: f32 = std::f64::consts::PI as f32;
/// `(float) (Math.PI * 2)` (`DaylightDetectorBlock.java:66`).
const TWO_PI: f32 = (std::f64::consts::PI * 2.0) as f32;

#[must_use]
pub fn signal(block_state_id: i32) -> i32 {
    int_property(block_state_id, POWER).unwrap_or(0)
}

#[must_use]
pub fn is_daylight_detector(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::DaylightDetectorBlock)
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    if !player.may_build {
        return false;
    }
    let Some(inverted) = cycle_property(block_state_id, INVERTED) else {
        return false;
    };
    // `:86-89`.
    level.set_block(
        pos,
        inverted,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        player.source_entity_id(),
        Some(inverted),
    );
    update_signal_strength(level, pos, inverted);
    true
}

pub(crate) fn update_signal_strength(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(next) = with_updated_power(block_state_id, level, pos) else {
        return;
    };
    if next != block_state_id {
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

fn with_updated_power(block_state_id: i32, level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let inverted = bool_property(block_state_id, INVERTED)?;
    let target = signal_strength(
        inverted,
        level.brightness(LightLayer::Sky, pos) - level.sky_darken(),
        level.sun_angle_degrees(pos)?,
    );
    if signal(block_state_id) == target {
        return Some(block_state_id);
    }
    with_property(block_state_id, POWER, &target.to_string())
}

/// `DaylightDetectorBlock.updateSignalStrength` (`:60-71`).
#[must_use]
pub(super) fn signal_strength(inverted: bool, effective_sky: i32, sun_angle_degrees: f32) -> i32 {
    let mut target = effective_sky;
    let mut sun_angle = sun_angle_degrees * DEG_TO_RAD;
    if inverted {
        target = 15 - target;
    } else if target > 0 {
        // `:65-68`.
        let offset = if sun_angle < PI { 0.0 } else { TWO_PI };
        sun_angle += (offset - sun_angle) * 0.2;
        target = java_math_round(target as f32 * mth_cos(f64::from(sun_angle)));
    }
    target.clamp(0, 15)
}

/// Java `Math.round(float)`: `(int) Math.floor(a + 0.5f)`.
fn java_math_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}
