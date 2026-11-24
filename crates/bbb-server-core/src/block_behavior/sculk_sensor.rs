//! Scheduled phase transitions and block-only updates for both sculk sensors.

use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::fluid::FluidFamily;

use super::support::str_property;
use super::{block_type_of, bool_property, direction_property, int_property, offset};

const PHASE: &str = "sculk_sensor_phase";
const POWER: &str = "power";
const WATERLOGGED: &str = "waterlogged";
const FACING: &str = "facing";
const ACTIVE: &str = "active";
const COOLDOWN: &str = "cooldown";
const INACTIVE: &str = "inactive";
const COOLDOWN_TICKS: i32 = 10;
/// `SoundEvents.SCULK_CLICKING` (`SculkSensorBlock.java:224`).
pub const CLICKING_SOUND: &str = "minecraft:block.sculk_sensor.clicking";
const CLICKING_STOP_SOUND: &str = "minecraft:block.sculk_sensor.clicking_stop";
const UPDATE_CLIENTS_AND_KNOWN_SHAPE: i32 = 18;
/// `CalibratedSculkSensorBlock`'s own `getActiveTicks`
/// (`CalibratedSculkSensorBlock.java:78-80`), against the base's 30.
const CALIBRATED_ACTIVE_TICKS: i32 = 10;
/// `SculkSensorBlock.getActiveTicks` (`SculkSensorBlock.java:201-203`).
const ACTIVE_TICKS: i32 = 30;

#[must_use]
pub fn is_active(block_state_id: i32) -> bool {
    str_property(block_state_id, PHASE) == Some(ACTIVE)
}

#[must_use]
pub fn can_activate(block_state_id: i32) -> bool {
    str_property(block_state_id, PHASE) == Some(INACTIVE)
}

#[must_use]
pub fn active_state(block_state_id: i32, power: i32) -> Option<i32> {
    let power = power.to_string();
    with_property(block_state_id, PHASE, ACTIVE)
        .and_then(|state| with_property(state, POWER, &power))
}

#[must_use]
pub fn active_ticks(block_state_id: i32) -> i32 {
    if block_type_of(block_state_id) == Some(VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID) {
        CALIBRATED_ACTIVE_TICKS
    } else {
        ACTIVE_TICKS
    }
}

#[must_use]
pub fn is_waterlogged(block_state_id: i32) -> bool {
    bool_property(block_state_id, WATERLOGGED) == Some(true)
}

#[must_use]
pub fn update_neighbour_positions(pos: BlockPos) -> [BlockPos; 2] {
    [pos, offset(pos, 0, -1, 0)]
}

#[must_use]
pub fn signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction_property(block_state_id, FACING) == Some(direction) {
        0
    } else {
        int_property(block_state_id, POWER).unwrap_or(0)
    }
}

#[must_use]
pub fn direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction == Direction::Up {
        signal(block_state_id, direction)
    } else {
        0
    }
}

fn update_neighbours(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    for pos in update_neighbour_positions(pos) {
        level.update_neighbors_at(pos, block_type_id);
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    match str_property(block_state_id, PHASE) {
        Some(ACTIVE) => {
            let Some(cooldown) = with_property(block_state_id, PHASE, COOLDOWN)
                .and_then(|state| with_property(state, POWER, "0"))
            else {
                return;
            };
            let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
                return;
            };
            level.set_block(
                pos,
                cooldown,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            level.schedule_block_tick(pos, tick_type, COOLDOWN_TICKS, TickPriority::Normal);
            update_neighbours(level, pos, block_state_id);
        }
        Some(COOLDOWN) => {
            let Some(inactive) = with_property(block_state_id, PHASE, INACTIVE) else {
                return;
            };
            level.set_block(
                pos,
                inactive,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            if bool_property(block_state_id, WATERLOGGED) == Some(false) {
                let pitch = level.level_random().next_float() * 0.2 + 0.8;
                level.play_block_sound_with_volume_pitch(pos, CLICKING_STOP_SOUND, 1.0, pitch);
            }
        }
        Some(INACTIVE) | None => {}
        Some(_) => {}
    }
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

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if is_active(block_state_id) {
        update_neighbours(level, pos, block_state_id);
    }
}
