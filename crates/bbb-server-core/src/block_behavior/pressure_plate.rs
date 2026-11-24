//! `BasePressurePlateBlock` stored signal and removal fan-out.
//!
//! Both entry points into `checkPressed` live here: the first activation an
//! entity walking into the cell makes, and the scheduled recheck that releases
//! the plate again.

use super::LevelCtx;
use crate::fluid::{below, with_property};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_BAMBOO_PRESSURE_PLATE_ID, VANILLA_BLOCK_CHERRY_PRESSURE_PLATE_ID,
    VANILLA_BLOCK_CRIMSON_PRESSURE_PLATE_ID, VANILLA_BLOCK_HEAVY_WEIGHTED_PRESSURE_PLATE_ID,
    VANILLA_BLOCK_LIGHT_WEIGHTED_PRESSURE_PLATE_ID,
    VANILLA_BLOCK_POLISHED_BLACKSTONE_PRESSURE_PLATE_ID, VANILLA_BLOCK_STONE_PRESSURE_PLATE_ID,
    VANILLA_BLOCK_WARPED_PRESSURE_PLATE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::geometry::Aabb;

use super::level::{BlockEntityQuery, InsideEntity};
use super::{block_type_of, bool_property, int_property, SET_BLOCK_UPDATE_CLIENTS};

const POWER: &str = "power";
const POWERED: &str = "powered";

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let old_signal = signal(block_state_id);
    if old_signal <= 0 {
        return;
    }
    let Some(new_signal) = sensed_signal(level, pos, block_state_id) else {
        return;
    };
    check_pressed(level, pos, block_state_id, old_signal, new_signal, None);
}

///
/// `false` is a fact this level could not answer, which the entity-inside
/// caller reports rather than reading as an empty touch box.
pub(super) fn entity_inside(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    inside: InsideEntity,
) -> bool {
    let old_signal = signal(block_state_id);
    if old_signal != 0 {
        return true;
    }
    let new_signal = match visiting_signal(pos, block_state_id, inside) {
        Some(new_signal) => new_signal,
        None => match sensed_signal(level, pos, block_state_id) {
            Some(new_signal) => new_signal,
            None => return false,
        },
    };
    check_pressed(
        level,
        pos,
        block_state_id,
        old_signal,
        new_signal,
        inside.source,
    );
    true
}

fn visiting_signal(pos: BlockPos, block_state_id: i32, inside: InsideEntity) -> Option<i32> {
    let block_type_id = block_type_of(block_state_id)?;
    let query = binary_plate_query(block_type_id)?;
    let counts = (query != BlockEntityQuery::BlockTriggerLiving || inside.living)
        && !inside.ignores_block_triggers
        && inside.bounds.intersects(touch_aabb(pos));
    counts.then_some(15)
}

fn sensed_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> Option<i32> {
    let block_type_id = block_type_of(block_state_id)?;
    let max_weight = match block_type_id {
        VANILLA_BLOCK_LIGHT_WEIGHTED_PRESSURE_PLATE_ID => Some(15_usize),
        VANILLA_BLOCK_HEAVY_WEIGHTED_PRESSURE_PLATE_ID => Some(150_usize),
        _ => None,
    };
    let query = binary_plate_query(block_type_id).unwrap_or(BlockEntityQuery::BlockTriggerAll);
    let count = level.entity_count_in_box(touch_aabb(pos), query);
    Some(match max_weight {
        Some(max_weight) if count > 0 => {
            ((count.min(max_weight) as f32 / max_weight as f32) * 15.0).ceil() as i32
        }
        Some(_) => 0,
        None if count > 0 => 15,
        None => 0,
    })
}

fn binary_plate_query(block_type_id: i32) -> Option<BlockEntityQuery> {
    match block_type_id {
        VANILLA_BLOCK_STONE_PRESSURE_PLATE_ID
        | VANILLA_BLOCK_POLISHED_BLACKSTONE_PRESSURE_PLATE_ID => {
            Some(BlockEntityQuery::BlockTriggerLiving)
        }
        VANILLA_BLOCK_LIGHT_WEIGHTED_PRESSURE_PLATE_ID
        | VANILLA_BLOCK_HEAVY_WEIGHTED_PRESSURE_PLATE_ID => None,
        _ => Some(BlockEntityQuery::BlockTriggerAll),
    }
}

fn check_pressed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_signal: i32,
    new_signal: i32,
    source: Option<i32>,
) {
    let was_pressed = old_signal > 0;
    let is_pressed = new_signal > 0;
    if old_signal != new_signal {
        let next = if int_property(block_state_id, POWER).is_some() {
            with_property(block_state_id, POWER, &new_signal.to_string())
        } else {
            with_property(
                block_state_id,
                POWERED,
                if is_pressed { "true" } else { "false" },
            )
        };
        let Some(next) = next else {
            return;
        };
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        update_neighbours(level, pos, block_state_id);
    }
    if !is_pressed && was_pressed {
        level.play_block_sound(pos, pressure_plate_sound(block_state_id, false));
        publish_event(level, VanillaGameEvent::BLOCK_DEACTIVATE, pos, source);
    } else if is_pressed && !was_pressed {
        level.play_block_sound(pos, pressure_plate_sound(block_state_id, true));
        publish_event(level, VanillaGameEvent::BLOCK_ACTIVATE, pos, source);
    }
    if is_pressed {
        let Some(block) = BlockTickType::of_block_state(block_state_id) else {
            return;
        };
        let delay = if int_property(block_state_id, POWER).is_some() {
            10
        } else {
            20
        };
        level.schedule_block_tick(pos, block, delay, TickPriority::Normal);
    }
}

fn publish_event(
    level: &mut LevelCtx<'_, '_>,
    event: VanillaGameEvent,
    pos: BlockPos,
    source: Option<i32>,
) {
    match source {
        Some(id) => level.game_event_from_entity(event, pos, Some(id), None),
        None => level.block_game_event(event, pos, None),
    }
}

fn update_neighbours(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(pos, block_type_id);
    level.update_neighbors_at(below(pos), block_type_id);
}

fn touch_aabb(pos: BlockPos) -> Aabb {
    Aabb {
        min_x: f64::from(pos.x) + 1.0 / 16.0,
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z) + 1.0 / 16.0,
        max_x: f64::from(pos.x) + 15.0 / 16.0,
        max_y: f64::from(pos.y) + 4.0 / 16.0,
        max_z: f64::from(pos.z) + 15.0 / 16.0,
    }
}

fn pressure_plate_sound(block_state_id: i32, pressed: bool) -> &'static str {
    let family = match block_type_of(block_state_id) {
        Some(
            VANILLA_BLOCK_LIGHT_WEIGHTED_PRESSURE_PLATE_ID
            | VANILLA_BLOCK_HEAVY_WEIGHTED_PRESSURE_PLATE_ID,
        ) => "metal",
        Some(
            VANILLA_BLOCK_STONE_PRESSURE_PLATE_ID
            | VANILLA_BLOCK_POLISHED_BLACKSTONE_PRESSURE_PLATE_ID,
        ) => "stone",
        Some(VANILLA_BLOCK_CHERRY_PRESSURE_PLATE_ID) => "cherry_wood",
        Some(VANILLA_BLOCK_CRIMSON_PRESSURE_PLATE_ID | VANILLA_BLOCK_WARPED_PRESSURE_PLATE_ID) => {
            "nether_wood"
        }
        Some(VANILLA_BLOCK_BAMBOO_PRESSURE_PLATE_ID) => "bamboo_wood",
        _ => "wooden",
    };
    match (family, pressed) {
        ("metal", true) => "minecraft:block.metal_pressure_plate.click_on",
        ("metal", false) => "minecraft:block.metal_pressure_plate.click_off",
        ("stone", true) => "minecraft:block.stone_pressure_plate.click_on",
        ("stone", false) => "minecraft:block.stone_pressure_plate.click_off",
        ("cherry_wood", true) => "minecraft:block.cherry_wood_pressure_plate.click_on",
        ("cherry_wood", false) => "minecraft:block.cherry_wood_pressure_plate.click_off",
        ("nether_wood", true) => "minecraft:block.nether_wood_pressure_plate.click_on",
        ("nether_wood", false) => "minecraft:block.nether_wood_pressure_plate.click_off",
        ("bamboo_wood", true) => "minecraft:block.bamboo_wood_pressure_plate.click_on",
        ("bamboo_wood", false) => "minecraft:block.bamboo_wood_pressure_plate.click_off",
        ("wooden", true) => "minecraft:block.wooden_pressure_plate.click_on",
        ("wooden", false) => "minecraft:block.wooden_pressure_plate.click_off",
        _ => unreachable!(),
    }
}

#[must_use]
pub fn signal(block_state_id: i32) -> i32 {
    int_property(block_state_id, POWER).unwrap_or_else(|| {
        if bool_property(block_state_id, POWERED).unwrap_or(false) {
            15
        } else {
            0
        }
    })
}

#[must_use]
pub fn direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if direction == Direction::Up {
        signal(block_state_id)
    } else {
        0
    }
}

pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if moved_by_piston || signal(block_state_id) == 0 {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(pos, block_type_id);
    level.update_neighbors_at(below(pos), block_type_id);
}
