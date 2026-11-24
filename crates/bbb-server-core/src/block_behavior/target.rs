//! `TargetBlock`: the output a projectile hit sets and its scheduled reset.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::packets::BlockPos;

use bbb_protocol::packets::{Direction, Vec3d};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};

use super::{block_type_of, int_property};

const POWER: &str = "power";
const UPDATE_CLIENTS_AND_KNOWN_SHAPE: i32 = 18;

#[must_use]
pub fn signal(block_state_id: i32) -> i32 {
    int_property(block_state_id, POWER).unwrap_or(0)
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(power) = int_property(block_state_id, POWER).filter(|power| (0..=15).contains(power))
    else {
        return;
    };
    if power == 0 {
        return;
    }
    let Some(unpowered) = with_property(block_state_id, POWER, "0") else {
        return;
    };
    level.set_block(
        pos,
        unpowered,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    if block_type_of(block_state_id) == block_type_of(old_state)
        || int_property(block_state_id, POWER).is_none_or(|power| power <= 0)
    {
        return;
    }
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    if level.has_scheduled_tick(pos, tick_type) {
        return;
    }
    if let Some(unpowered) = with_property(block_state_id, POWER, "0") {
        level.set_block(
            pos,
            unpowered,
            UPDATE_CLIENTS_AND_KNOWN_SHAPE,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// `TargetBlock.updateRedstoneOutput` (`TargetBlock.java:51-59`): the
/// strength the hit is worth, written unless a reset is already pending.
pub(super) fn on_projectile_hit(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    location: Vec3d,
    direction: Direction,
    arrow: bool,
) -> i32 {
    let strength = redstone_strength(location, direction);
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return strength;
    };
    if !level.has_scheduled_tick(pos, tick_type) {
        if let Some(powered) = with_property(block_state_id, POWER, &strength.to_string()) {
            level.set_block(
                pos,
                powered,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        let duration = if arrow { 20 } else { 8 };
        level.schedule_block_tick(pos, tick_type, duration, TickPriority::Normal);
    }
    strength
}

/// `TargetBlock.getRedstoneStrength` (`TargetBlock.java:61-77`).
fn redstone_strength(location: Vec3d, direction: Direction) -> i32 {
    let distance_from_centre = |value: f64| (value - value.floor() - 0.5).abs();
    let (x, y, z) = (
        distance_from_centre(location.x),
        distance_from_centre(location.y),
        distance_from_centre(location.z),
    );
    let distance = match direction {
        Direction::Up | Direction::Down => x.max(z),
        Direction::North | Direction::South => x.max(y),
        Direction::East | Direction::West => y.max(z),
    };
    1.max((15.0 * ((0.5 - distance) / 0.5).clamp(0.0, 1.0)).ceil() as i32)
}
