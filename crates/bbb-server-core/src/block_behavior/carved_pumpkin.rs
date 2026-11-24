use super::LevelCtx;
use crate::level_event::LEVEL_EVENT_PARTICLES_DESTROY_BLOCK;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaOnPlaceOverride, VANILLA_BLOCK_AIR_ID,
    VANILLA_BLOCK_COPPER_BLOCK_ID, VANILLA_BLOCK_COPPER_CHEST_ID,
    VANILLA_BLOCK_EXPOSED_COPPER_CHEST_ID, VANILLA_BLOCK_EXPOSED_COPPER_ID,
    VANILLA_BLOCK_IRON_BLOCK_ID, VANILLA_BLOCK_OXIDIZED_COPPER_CHEST_ID,
    VANILLA_BLOCK_OXIDIZED_COPPER_ID, VANILLA_BLOCK_SNOW_BLOCK_ID,
    VANILLA_BLOCK_WAXED_COPPER_BLOCK_ID, VANILLA_BLOCK_WAXED_EXPOSED_COPPER_ID,
    VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_ID, VANILLA_BLOCK_WAXED_WEATHERED_COPPER_ID,
    VANILLA_BLOCK_WEATHERED_COPPER_CHEST_ID, VANILLA_BLOCK_WEATHERED_COPPER_ID,
    VANILLA_BLOCK_WITHER_SKELETON_SKULL_ID, VANILLA_BLOCK_WITHER_SKELETON_WALL_SKULL_ID,
};
use bbb_protocol::item_types::VANILLA_AXE_WAX_OFF;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{clock_wise, counter_clock_wise, opposite, relative};
use bbb_server_types::fluid::{air_block_state_id, block_state_is_air};

use super::{
    block_type_of, default_block_state_id, direction_property, direction_serialized_name, offset,
    string_property, weathering, with_property, CopperWeatherState, GolemKind, GolemSpawnRequest,
    DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

const FACING: &str = "facing";
const CHEST_TYPE: &str = "type";
const CHEST_TYPE_SINGLE: &str = "single";
const CHEST_TYPE_LEFT: &str = "left";
const CHEST_TYPE_RIGHT: &str = "right";
const COPPER_TAG: &str = "minecraft:copper";
const COPPER_CHESTS_TAG: &str = "minecraft:copper_chests";
const WITHER_SUMMON_BASE_BLOCKS_TAG: &str = "minecraft:wither_summon_base_blocks";

#[derive(Clone, Copy)]
struct PatternCell {
    pos: BlockPos,
    state: i32,
}

struct PatternMatch {
    forwards: Direction,
    width: i32,
    height: i32,
    cells: Vec<PatternCell>,
}

impl PatternMatch {
    fn get(&self, right: i32, down: i32) -> PatternCell {
        self.cells[(down * self.width + right) as usize]
    }
}

#[derive(Clone, Copy)]
enum CellPred {
    Pumpkin,
    Snow,
    Iron,
    Copper,
    WitherSkull,
    WitherBase,
    Air,
    Any,
}

#[must_use]
pub fn is_carved_pumpkin(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.on_place_override)
        == Some(VanillaOnPlaceOverride::CarvedPumpkinBlock)
}

fn is_pumpkin_head(block_state_id: i32) -> bool {
    is_carved_pumpkin(block_state_id)
}

fn cell_matches(level: &LevelCtx<'_, '_>, pred: CellPred, state: i32) -> bool {
    match pred {
        CellPred::Pumpkin => is_pumpkin_head(state),
        CellPred::Snow => block_type_of(state) == Some(VANILLA_BLOCK_SNOW_BLOCK_ID),
        CellPred::Iron => block_type_of(state) == Some(VANILLA_BLOCK_IRON_BLOCK_ID),
        CellPred::Copper => level.block_state_in_tag(state, COPPER_TAG),
        CellPred::WitherSkull => matches!(
            block_type_of(state),
            Some(
                VANILLA_BLOCK_WITHER_SKELETON_SKULL_ID
                    | VANILLA_BLOCK_WITHER_SKELETON_WALL_SKULL_ID
            )
        ),
        CellPred::WitherBase => level.block_state_in_tag(state, WITHER_SUMMON_BASE_BLOCKS_TAG),
        CellPred::Air => block_state_is_air(state),
        CellPred::Any => true,
    }
}

fn step(direction: Direction) -> (i32, i32, i32) {
    match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    }
}

fn cross(a: (i32, i32, i32), b: (i32, i32, i32)) -> (i32, i32, i32) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

fn translate_and_rotate(
    origin: BlockPos,
    forwards: Direction,
    up: Direction,
    right: i32,
    down: i32,
    forwards_off: i32,
) -> BlockPos {
    let fv = step(forwards);
    let uv = step(up);
    let rv = cross(fv, uv);
    offset(
        origin,
        uv.0 * -down + rv.0 * right + fv.0 * forwards_off,
        uv.1 * -down + rv.1 * right + fv.1 * forwards_off,
        uv.2 * -down + rv.2 * right + fv.2 * forwards_off,
    )
}

fn between_closed(min: BlockPos, max: BlockPos) -> impl Iterator<Item = BlockPos> {
    let width = max.x.wrapping_sub(min.x).wrapping_add(1);
    let height = max.y.wrapping_sub(min.y).wrapping_add(1);
    let depth = max.z.wrapping_sub(min.z).wrapping_add(1);
    let end = width.wrapping_mul(height).wrapping_mul(depth);
    (0..end).map(move |index| {
        let x = index % width;
        let slice = index / width;
        let y = slice % height;
        let z = slice / height;
        offset(min, x, y, z)
    })
}

fn read_cell(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    if !level.has_chunk_at(pos) {
        return None;
    }
    Some(level.block_state(pos))
}

fn matches_at(
    level: &LevelCtx<'_, '_>,
    origin: BlockPos,
    forwards: Direction,
    up: Direction,
    width: i32,
    height: i32,
    pred: impl Fn(i32, i32) -> CellPred,
) -> Option<PatternMatch> {
    let mut cells = Vec::with_capacity((width * height) as usize);
    for down in 0..height {
        for right in 0..width {
            let pos = translate_and_rotate(origin, forwards, up, right, down, 0);
            let state = read_cell(level, pos)?;
            if !cell_matches(level, pred(right, down), state) {
                return None;
            }
            cells.push(PatternCell { pos, state });
        }
    }
    Some(PatternMatch {
        forwards,
        width,
        height,
        cells,
    })
}

fn find_pattern(
    level: &LevelCtx<'_, '_>,
    origin: BlockPos,
    width: i32,
    height: i32,
    pred: impl Fn(i32, i32) -> CellPred + Copy,
) -> Option<PatternMatch> {
    let dist = width.max(height).max(1);
    let max = offset(origin, dist - 1, dist - 1, dist - 1);
    for test_pos in between_closed(origin, max) {
        for forwards in DIRECTION_VALUES {
            for up in DIRECTION_VALUES {
                if up == forwards || up == opposite(forwards) {
                    continue;
                }
                if let Some(found) = matches_at(level, test_pos, forwards, up, width, height, pred)
                {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn snow_full(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<PatternMatch> {
    find_pattern(level, pos, 1, 3, |right, down| match (right, down) {
        (0, 0) => CellPred::Pumpkin,
        (0, 1) | (0, 2) => CellPred::Snow,
        _ => CellPred::Air,
    })
}

fn iron_full(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<PatternMatch> {
    find_pattern(level, pos, 3, 3, |right, down| match (right, down) {
        (1, 0) => CellPred::Pumpkin,
        (0, 1) | (1, 1) | (2, 1) | (1, 2) => CellPred::Iron,
        _ => CellPred::Air,
    })
}

fn copper_full(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<PatternMatch> {
    find_pattern(level, pos, 1, 2, |right, down| match (right, down) {
        (0, 0) => CellPred::Pumpkin,
        (0, 1) => CellPred::Copper,
        _ => CellPred::Air,
    })
}

fn wither_full(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<PatternMatch> {
    find_pattern(level, pos, 3, 3, |right, down| match (right, down) {
        (_, 0) => CellPred::WitherSkull,
        (_, 1) | (1, 2) => CellPred::WitherBase,
        _ => CellPred::Air,
    })
}

/// `CarvedPumpkinBlock.canSpawnGolem`: a snow, iron or copper golem base
/// under `pos`.
pub(crate) fn can_spawn_golem(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let snow = find_pattern(level, pos, 1, 3, |_, down| match down {
        0 => CellPred::Any,
        _ => CellPred::Snow,
    });
    let iron = || {
        find_pattern(level, pos, 3, 3, |right, down| match (right, down) {
            (1, 0) => CellPred::Any,
            (0, 1) | (1, 1) | (2, 1) | (1, 2) => CellPred::Iron,
            _ => CellPred::Air,
        })
    };
    let copper = || {
        find_pattern(level, pos, 1, 2, |_, down| match down {
            0 => CellPred::Any,
            _ => CellPred::Copper,
        })
    };
    snow.is_some() || iron().is_some() || copper().is_some()
}

/// `WitherSkullBlock.getOrCreateWitherBase` under `pos`.
pub(crate) fn has_wither_base(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    find_pattern(level, pos, 3, 3, |right, down| match (right, down) {
        (_, 0) => CellPred::Any,
        (_, 1) | (1, 2) => CellPred::WitherBase,
        _ => CellPred::Air,
    })
    .is_some()
}

fn clear_pattern_blocks(level: &mut LevelCtx<'_, '_>, pattern: &PatternMatch) {
    let air = air_block_state_id();
    for right in 0..pattern.width {
        for down in 0..pattern.height {
            let cell = pattern.get(right, down);
            level.set_block(
                cell.pos,
                air,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, cell.pos, cell.state);
        }
    }
}

fn update_pattern_blocks(level: &mut LevelCtx<'_, '_>, pattern: &PatternMatch) {
    for right in 0..pattern.width {
        for down in 0..pattern.height {
            let cell = pattern.get(right, down);
            level.update_neighbors_at(cell.pos, VANILLA_BLOCK_AIR_ID);
        }
    }
}

fn spawn_golem_in_world(
    level: &mut LevelCtx<'_, '_>,
    pattern: &PatternMatch,
    request: GolemSpawnRequest,
) -> bool {
    if level.request_golem_spawn(request) != Some(true) {
        return false;
    }
    clear_pattern_blocks(level, pattern);
    update_pattern_blocks(level, pattern);
    true
}

fn weather_state_from_copper(state: i32) -> CopperWeatherState {
    if let Some(age) = weathering::age(state) {
        return CopperWeatherState::from_age(age).unwrap_or(CopperWeatherState::Unaffected);
    }
    let Some(block_type_id) = block_type_of(state) else {
        return CopperWeatherState::Unaffected;
    };
    let unwaxed = VANILLA_AXE_WAX_OFF
        .iter()
        .find(|transform| transform.source_block_type_id == block_type_id)
        .map(|transform| transform.target_block_type_id);
    if let Some(unwaxed) = unwaxed {
        if let Some(age) = vanilla_block_type(unwaxed).and_then(|entry| entry.change_over_time_age)
        {
            return CopperWeatherState::from_age(age).unwrap_or(CopperWeatherState::Unaffected);
        }
    }
    CopperWeatherState::Unaffected
}

fn chest_block_from_copper(copper_block_type_id: i32) -> i32 {
    match copper_block_type_id {
        VANILLA_BLOCK_COPPER_BLOCK_ID | VANILLA_BLOCK_WAXED_COPPER_BLOCK_ID => {
            VANILLA_BLOCK_COPPER_CHEST_ID
        }
        VANILLA_BLOCK_EXPOSED_COPPER_ID | VANILLA_BLOCK_WAXED_EXPOSED_COPPER_ID => {
            VANILLA_BLOCK_EXPOSED_COPPER_CHEST_ID
        }
        VANILLA_BLOCK_WEATHERED_COPPER_ID | VANILLA_BLOCK_WAXED_WEATHERED_COPPER_ID => {
            VANILLA_BLOCK_WEATHERED_COPPER_CHEST_ID
        }
        VANILLA_BLOCK_OXIDIZED_COPPER_ID | VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_ID => {
            VANILLA_BLOCK_OXIDIZED_COPPER_CHEST_ID
        }
        _ => VANILLA_BLOCK_COPPER_CHEST_ID,
    }
}

fn candidate_partner_facing(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    neighbour_direction: Direction,
) -> Option<Option<Direction>> {
    let neighbour = level.block_state(relative(pos, neighbour_direction));
    if !level.block_state_in_tag(neighbour, COPPER_CHESTS_TAG) {
        return Some(None);
    }
    if string_property(neighbour, CHEST_TYPE).as_deref() != Some(CHEST_TYPE_SINGLE) {
        return Some(None);
    }
    Some(direction_property(neighbour, FACING))
}

fn get_chest_type(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    facing: Direction,
) -> Option<&'static str> {
    if candidate_partner_facing(level, pos, clock_wise(facing))? == Some(facing) {
        Some(CHEST_TYPE_LEFT)
    } else if candidate_partner_facing(level, pos, counter_clock_wise(facing))? == Some(facing) {
        Some(CHEST_TYPE_RIGHT)
    } else {
        Some(CHEST_TYPE_SINGLE)
    }
}

fn copper_chest_age_and_wax(block_type_id: i32) -> Option<(i32, bool)> {
    if let Some(transform) = VANILLA_AXE_WAX_OFF
        .iter()
        .find(|transform| transform.source_block_type_id == block_type_id)
    {
        let age = vanilla_block_type(transform.target_block_type_id)?.change_over_time_age?;
        return Some((age, true));
    }
    let age = vanilla_block_type(block_type_id)?.change_over_time_age?;
    Some((age, false))
}

fn least_oxidized_connected(level: &LevelCtx<'_, '_>, state: i32, pos: BlockPos) -> Option<i32> {
    let ty = string_property(state, CHEST_TYPE)?;
    if ty == CHEST_TYPE_SINGLE {
        return Some(state);
    }
    let facing = direction_property(state, FACING)?;
    let connected_dir = if ty == CHEST_TYPE_LEFT {
        clock_wise(facing)
    } else {
        counter_clock_wise(facing)
    };
    let connected = level.block_state(relative(pos, connected_dir));
    if !level.block_state_in_tag(connected, COPPER_CHESTS_TAG) {
        return Some(state);
    }
    let block_type_id = block_type_of(state)?;
    let connected_block_type_id = block_type_of(connected)?;
    let (own_age, waxed) = copper_chest_age_and_wax(block_type_id)?;
    let (connected_age, connected_waxed) = copper_chest_age_and_wax(connected_block_type_id)?;
    let wax_mismatch = waxed != connected_waxed;
    let updated = if wax_mismatch {
        let unwaxed = VANILLA_AXE_WAX_OFF
            .iter()
            .find(|transform| transform.source_block_type_id == block_type_id)
            .map(|transform| transform.target_block_type_id)
            .unwrap_or(block_type_id);
        weathering::with_properties_of(unwaxed, state)?
    } else {
        state
    };
    let connected_predicted = if wax_mismatch {
        let unwaxed = VANILLA_AXE_WAX_OFF
            .iter()
            .find(|transform| transform.source_block_type_id == connected_block_type_id)
            .map(|transform| transform.target_block_type_id)
            .unwrap_or(connected_block_type_id);
        weathering::with_properties_of(unwaxed, connected)?
    } else {
        connected
    };
    let least = if own_age <= connected_age {
        updated
    } else {
        connected_predicted
    };
    weathering::with_properties_of(block_type_of(least)?, updated)
}

fn replace_copper_block_with_chest(
    level: &mut LevelCtx<'_, '_>,
    copper: PatternCell,
    pumpkin: PatternCell,
) {
    let Some(copper_block) = block_type_of(copper.state) else {
        return;
    };
    let Some(facing) = direction_property(pumpkin.state, FACING) else {
        return;
    };
    if matches!(facing, Direction::Down | Direction::Up) {
        return;
    }
    let chest_block = chest_block_from_copper(copper_block);
    let Some(chest_kind) = get_chest_type(level, copper.pos, facing) else {
        return;
    };
    let Some(state) = with_property(
        default_block_state_id(chest_block),
        FACING,
        direction_serialized_name(facing),
    ) else {
        return;
    };
    let Some(state) = with_property(state, CHEST_TYPE, chest_kind) else {
        return;
    };
    let Some(state) = least_oxidized_connected(level, state, copper.pos) else {
        return;
    };
    level.set_block(
        copper.pos,
        state,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

fn try_spawn_golem(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if let Some(pattern) = snow_full(level, pos) {
        let request = GolemSpawnRequest {
            kind: GolemKind::Snow,
            pos: pattern.get(0, 2).pos,
            player_created: false,
            copper_weather: None,
            y_rot: 0.0,
        };
        if spawn_golem_in_world(level, &pattern, request) {
            return;
        }
    }

    if let Some(pattern) = iron_full(level, pos) {
        let request = GolemSpawnRequest {
            kind: GolemKind::Iron,
            pos: pattern.get(1, 2).pos,
            player_created: true,
            copper_weather: None,
            y_rot: 0.0,
        };
        if spawn_golem_in_world(level, &pattern, request) {
            return;
        }
    }

    if let Some(pattern) = copper_full(level, pos) {
        let copper = pattern.get(0, 1);
        let pumpkin = pattern.get(0, 0);
        let request = GolemSpawnRequest {
            kind: GolemKind::Copper,
            pos: pumpkin.pos,
            player_created: false,
            copper_weather: Some(weather_state_from_copper(copper.state)),
            y_rot: 0.0,
        };
        if spawn_golem_in_world(level, &pattern, request) {
            replace_copper_block_with_chest(level, copper, pumpkin);
        }
    }
}

pub(crate) fn wither_check_spawn(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    peaceful: bool,
) {
    // `:45`: the placed skull's own block entity.
    if !level.block_entity_present(pos) {
        return;
    }
    if !matches!(
        block_type_of(state),
        Some(VANILLA_BLOCK_WITHER_SKELETON_SKULL_ID | VANILLA_BLOCK_WITHER_SKELETON_WALL_SKULL_ID)
    ) || pos.y < level.get_min_y()
        || peaceful
    {
        return;
    }
    let Some(pattern) = wither_full(level, pos) else {
        return;
    };
    let y_rot = if matches!(pattern.forwards, Direction::East | Direction::West) {
        0.0
    } else {
        90.0
    };
    let request = GolemSpawnRequest {
        kind: GolemKind::Wither,
        pos: pattern.get(1, 2).pos,
        player_created: false,
        copper_weather: None,
        y_rot,
    };
    spawn_golem_in_world(level, &pattern, request);
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, old_state: i32) {
    if block_type_of(old_state) == block_type_of(state) {
        return;
    }
    try_spawn_golem(level, pos);
}
