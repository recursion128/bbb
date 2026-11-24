//! Fire placement, survival, scheduling, burnout and lava ignition.

use super::BlockTags;
use super::{block_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use std::collections::HashSet;

use super::{
    block_type_of, default_block_state_id, int_property, offset, state_is_block, tick_override,
    BlockMobSpawnRequest, BlockStateAt, DIRECTION_VALUES,
};
use crate::fluid::{above, below, with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_state_types::{
    vanilla_block_state_ignited_by_lava, vanilla_block_state_property,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaTickOverride, VANILLA_BLOCK_FIRE_ID, VANILLA_BLOCK_NETHER_PORTAL_ID,
    VANILLA_BLOCK_OBSIDIAN_ID, VANILLA_BLOCK_SOUL_FIRE_ID, VANILLA_BLOCK_TNT_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::fluid::{air_block_state_id, block_state_is_air};

const AGE: &str = "age";
const WATERLOGGED: &str = "waterlogged";
const MAX_AGE: i32 = 15;
const FIRE_UPDATE_FLAGS: i32 = 260;
const PORTAL_UPDATE_FLAGS: i32 = 18;
const MAX_PORTAL_SIZE: i32 = 21;
const SOUL_FIRE_BASE_BLOCKS_TAG: &str = "minecraft:soul_fire_base_blocks";

#[derive(Clone, Copy)]
enum PortalAxis {
    X,
    Z,
}

impl PortalAxis {
    fn right(self) -> Direction {
        match self {
            Self::X => Direction::West,
            Self::Z => Direction::South,
        }
    }

    fn property(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Z => "z",
        }
    }
}

#[derive(Clone, Copy)]
struct PortalShape {
    axis: PortalAxis,
    bottom_left: BlockPos,
    right: Direction,
    width: i32,
    height: i32,
    portal_blocks: i32,
}

struct PortalCell {
    empty: bool,
    frame: bool,
    portal: bool,
}

pub(crate) struct FireFacts<'a> {
    pub(crate) infiniburn_blocks: &'a HashSet<i32>,
    pub(crate) players: &'a [crate::entity::NearbyPlayer],
    pub(crate) fire_spread_radius: i32,
    pub(crate) difficulty_id: i32,
    pub(crate) spawning_monsters: bool,
    pub(crate) raining: bool,
}

fn can_spread_fire_around(facts: &FireFacts<'_>, pos: BlockPos) -> bool {
    spread_radius_allows(facts.players, facts.fire_spread_radius, pos)
}

/// `ServerLevel.canSpreadFireAround` (`ServerLevel.java:1809-1812`).
pub(crate) fn spread_radius_allows(
    players: &[crate::entity::NearbyPlayer],
    fire_spread_radius: i32,
    pos: BlockPos,
) -> bool {
    if fire_spread_radius == -1 {
        return true;
    }
    let radius = f64::from(fire_spread_radius);
    let radius_squared = radius * radius;
    players
        .iter()
        .filter(|player| !player.spectator)
        .any(|player| {
            let dx = player.position.x - f64::from(pos.x);
            let dy = player.position.y - f64::from(pos.y);
            let dz = player.position.z - f64::from(pos.z);
            dx * dx + dy * dy + dz * dz < radius_squared
        })
}

fn any_player_close_enough_for_spawning(facts: &FireFacts<'_>, pos: BlockPos) -> bool {
    let chunk_x = f64::from((pos.x >> 4) * 16 + 8);
    let chunk_z = f64::from((pos.z >> 4) * 16 + 8);
    facts
        .players
        .iter()
        .filter(|player| !player.spectator)
        .any(|player| {
            let dx = player.position.x - chunk_x;
            let dz = player.position.z - chunk_z;
            dx * dx + dz * dz < 16384.0
        })
}

#[must_use]
pub fn is_fire(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(tick_override) == Some(VanillaTickOverride::FireBlock)
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => below(pos),
        Direction::Up => above(pos),
        Direction::North => offset(pos, 0, 0, -1),
        Direction::South => offset(pos, 0, 0, 1),
        Direction::West => offset(pos, -1, 0, 0),
        Direction::East => offset(pos, 1, 0, 0),
    }
}

fn property(direction: Direction) -> Option<&'static str> {
    match direction {
        Direction::Down => None,
        Direction::Up => Some("up"),
        Direction::North => Some("north"),
        Direction::South => Some("south"),
        Direction::West => Some("west"),
        Direction::East => Some("east"),
    }
}

fn fire_odds(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<(i32, i32)> {
    let waterlogged = match vanilla_block_state_property(block_state_id, WATERLOGGED) {
        None | Some("false") => false,
        Some("true") => true,
        Some(_) => return None,
    };
    let Some(block_id) = block_type_of(block_state_id) else {
        content_registry.block_by_state_id(block_state_id)?;
        return Some((0, 0));
    };
    let block = vanilla_block_type(block_id)?;
    Some(if waterlogged {
        (0, 0)
    } else {
        (block.fire_ignite_odds, block.fire_burn_odds)
    })
}

fn can_burn(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<bool> {
    Some(fire_odds(block_state_id, content_registry)?.0 > 0)
}

fn block_type_or_custom(
    state: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<i32> {
    bbb_server_types::block_queries::block_type_of_with_registry(state, content_registry)
}

/// `FireBlock.getStateForPlacement` (`FireBlock.java`).
fn fire_state_for_placement(
    pos: BlockPos,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let below_pos = below(pos);
    let below_state = block_state_at(below_pos);
    block_type_or_custom(below_state, content_registry)?;
    if can_burn(below_state, content_registry)?
        || bbb_server_types::block_queries::face_sturdy_at_with_registry(
            below_state,
            below_pos,
            Direction::Up,
            SupportType::Full,
            content_registry,
        )
    {
        return Some(default_block_state_id(VANILLA_BLOCK_FIRE_ID));
    }

    let mut result = default_block_state_id(VANILLA_BLOCK_FIRE_ID);
    for direction in DIRECTION_VALUES {
        let Some(property) = property(direction) else {
            continue;
        };
        let neighbour = block_state_at(relative(pos, direction));
        block_type_or_custom(neighbour, content_registry)?;
        result = with_property(
            result,
            property,
            if can_burn(neighbour, content_registry)? {
                "true"
            } else {
                "false"
            },
        )?;
    }
    Some(result)
}

/// `BaseFireBlock.getState` (`BaseFireBlock.java`).
pub fn state_for_placement(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let content_registry = tags.content_registry();
    let below_state = block_state_at(below(pos));
    block_type_or_custom(below_state, content_registry)?;
    if tags.block_in_tag(below_state, "minecraft:soul_fire_base_blocks") {
        Some(default_block_state_id(VANILLA_BLOCK_SOUL_FIRE_ID))
    } else {
        fire_state_for_placement(pos, content_registry, block_state_at)
    }
}

fn valid_fire_location(
    pos: BlockPos,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    for direction in DIRECTION_VALUES {
        let state = block_state_at(relative(pos, direction));
        block_type_or_custom(state, content_registry)?;
        if can_burn(state, content_registry)? {
            return Some(true);
        }
    }
    Some(false)
}

fn valid_fire_location_at<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    valid_fire_location(pos, level.content_registry(), &mut |at| {
        level.block_state(at)
    })
}

fn fire_state_for_placement_at<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<i32> {
    fire_state_for_placement(pos, level.content_registry(), &mut |at| {
        level.block_state(at)
    })
}

fn state_for_placement_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<i32> {
    let content_registry = level.content_registry();
    let support = level.block_state(below(pos));
    block_type_or_custom(support, content_registry)?;
    let in_soul_tag = if block_type_of(support).is_some() {
        block_tag_matches(support, SOUL_FIRE_BASE_BLOCKS_TAG, membership)
    } else {
        content_registry
            .block_by_state_id(support)
            .is_some_and(|(block, _)| {
                membership(
                    "minecraft:block",
                    SOUL_FIRE_BASE_BLOCKS_TAG,
                    block.resource_id(),
                )
            })
    };
    if in_soul_tag {
        Some(default_block_state_id(VANILLA_BLOCK_SOUL_FIRE_ID))
    } else {
        fire_state_for_placement_at(level, pos)
    }
}

fn can_survive_at<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    can_survive_with_registry(0, pos, level.content_registry(), &mut |at| {
        level.block_state(at)
    })
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    can_survive_at(level, pos)
}

/// `FireBlock.canSurvive` (`FireBlock.java`).
pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    can_survive_with_registry(
        block_state_id,
        pos,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
        block_state_at,
    )
}

fn can_survive_with_registry(
    _block_state_id: i32,
    pos: BlockPos,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let below_pos = below(pos);
    let below_state = block_state_at(below_pos);
    block_type_or_custom(below_state, content_registry)?;
    if bbb_server_types::block_queries::face_sturdy_at_with_registry(
        below_state,
        below_pos,
        Direction::Up,
        SupportType::Full,
        content_registry,
    ) {
        Some(true)
    } else {
        valid_fire_location(pos, content_registry, block_state_at)
    }
}

fn state_with_age<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    age: i32,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<i32> {
    let state = state_for_placement_at(level, pos, membership)?;
    if block_type_of(state) == Some(VANILLA_BLOCK_FIRE_ID) {
        with_property(state, AGE, &age.to_string())
    } else {
        Some(state)
    }
}

/// `FireBlock.updateShape` (`FireBlock.java`).
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    match can_survive_at(level, pos) {
        Some(false) => air_block_state_id(),
        Some(true) => int_property(block_state_id, AGE)
            .and_then(|age| {
                state_with_age(level, pos, age, &|registry, tag, id| {
                    level.registry_tag_membership(registry, tag, id)
                })
            })
            .unwrap_or(block_state_id),
        None => block_state_id,
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn relative_by(pos: BlockPos, direction: Direction, distance: i32) -> BlockPos {
    match direction {
        Direction::Down => offset(pos, 0, -distance, 0),
        Direction::Up => offset(pos, 0, distance, 0),
        Direction::North => offset(pos, 0, 0, -distance),
        Direction::South => offset(pos, 0, 0, distance),
        Direction::West => offset(pos, -distance, 0, 0),
        Direction::East => offset(pos, distance, 0, 0),
    }
}

fn portal_cell(state: i32, is_fire: bool) -> PortalCell {
    let block = block_type_of(state);
    let portal = block == Some(VANILLA_BLOCK_NETHER_PORTAL_ID);
    PortalCell {
        empty: block_state_is_air(state) || portal || is_fire,
        frame: block == Some(VANILLA_BLOCK_OBSIDIAN_ID),
        portal,
    }
}

fn portal_frame(cell_at: &dyn Fn(BlockPos) -> PortalCell, pos: BlockPos) -> bool {
    cell_at(pos).frame
}

fn distance_until_edge(
    cell_at: &dyn Fn(BlockPos) -> PortalCell,
    pos: BlockPos,
    direction: Direction,
) -> i32 {
    for width in 0..=MAX_PORTAL_SIZE {
        let at = relative_by(pos, direction, width);
        let cell = cell_at(at);
        if !cell.empty {
            return if cell.frame { width } else { 0 };
        }
        if !portal_frame(cell_at, below(at)) {
            return 0;
        }
    }
    0
}

fn bottom_left(
    cell_at: &dyn Fn(BlockPos) -> PortalCell,
    min_y: i32,
    pos: BlockPos,
    right: Direction,
) -> Option<BlockPos> {
    let min_y = min_y.max(pos.y.saturating_sub(MAX_PORTAL_SIZE));
    let mut bottom = pos;
    while bottom.y > min_y {
        if !cell_at(below(bottom)).empty {
            break;
        }
        bottom = below(bottom);
    }
    let edge = distance_until_edge(cell_at, bottom, opposite(right)) - 1;
    (edge >= 0).then(|| relative_by(bottom, opposite(right), edge))
}

fn distance_until_top(
    cell_at: &dyn Fn(BlockPos) -> PortalCell,
    bottom_left: BlockPos,
    right: Direction,
    width: i32,
) -> (i32, i32) {
    let mut portal_blocks = 0;
    for height in 0..MAX_PORTAL_SIZE {
        let row = relative_by(bottom_left, Direction::Up, height);
        if !portal_frame(cell_at, relative_by(row, right, -1))
            || !portal_frame(cell_at, relative_by(row, right, width))
        {
            return (height, portal_blocks);
        }
        for column in 0..width {
            let cell = cell_at(relative_by(row, right, column));
            if !cell.empty {
                return (height, portal_blocks);
            }
            if cell.portal {
                portal_blocks += 1;
            }
        }
    }
    (MAX_PORTAL_SIZE, portal_blocks)
}

fn top_frame(
    cell_at: &dyn Fn(BlockPos) -> PortalCell,
    bottom_left: BlockPos,
    right: Direction,
    width: i32,
    height: i32,
) -> bool {
    let row = relative_by(bottom_left, Direction::Up, height);
    for column in 0..width {
        if !portal_frame(cell_at, relative_by(row, right, column)) {
            return false;
        }
    }
    true
}

fn portal_shape(
    cell_at: &dyn Fn(BlockPos) -> PortalCell,
    min_y: i32,
    pos: BlockPos,
    axis: PortalAxis,
) -> Option<PortalShape> {
    let right = axis.right();
    let bottom_left = bottom_left(cell_at, min_y, pos, right)?;
    let width = distance_until_edge(cell_at, bottom_left, right);
    if !(2..=MAX_PORTAL_SIZE).contains(&width) {
        return None;
    }
    let (height, portal_blocks) = distance_until_top(cell_at, bottom_left, right, width);
    if !(3..=MAX_PORTAL_SIZE).contains(&height)
        || !top_frame(cell_at, bottom_left, right, width, height)
    {
        return None;
    }
    Some(PortalShape {
        axis,
        bottom_left,
        right,
        width,
        height,
        portal_blocks,
    })
}

/// `PortalShape.findEmptyPortalShape` and `createPortalBlocks`
/// (`PortalShape.java`).
fn find_empty_portal(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<PortalShape> {
    let cell_at = |at| {
        let state = level.block_state(at);
        portal_cell(state, level.block_state_in_tag(state, "minecraft:fire"))
    };
    [PortalAxis::X, PortalAxis::Z].into_iter().find_map(|axis| {
        portal_shape(&cell_at, level.get_min_y(), pos, axis)
            .filter(|shape| shape.portal_blocks == 0)
    })
}

/// `NetherPortalBlock.updateShape` (`NetherPortalBlock.java:91-107`).
pub(super) fn portal_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    state: i32,
    pos: BlockPos,
    direction: Direction,
    neighbour_state: i32,
) -> i32 {
    let axis = match vanilla_block_state_property(state, "axis") {
        Some("x") => PortalAxis::X,
        Some("z") => PortalAxis::Z,
        _ => return state,
    };
    let wrong_axis = match direction {
        Direction::West | Direction::East => matches!(axis, PortalAxis::Z),
        Direction::North | Direction::South => matches!(axis, PortalAxis::X),
        Direction::Up | Direction::Down => false,
    };
    if wrong_axis || state_is_block(neighbour_state, VANILLA_BLOCK_NETHER_PORTAL_ID) {
        return state;
    }
    let cell_at = |at| {
        let state = level.block_state(at);
        portal_cell(state, level.block_state_in_tag(state, "minecraft:fire"))
    };
    if portal_shape(&cell_at, level.get_min_y(), pos, axis)
        .is_some_and(|shape| shape.portal_blocks == shape.width * shape.height)
    {
        state
    } else {
        air_block_state_id()
    }
}

fn portal_positions(shape: PortalShape) -> Vec<BlockPos> {
    let end = relative_by(
        relative_by(shape.bottom_left, Direction::Up, shape.height - 1),
        shape.right,
        shape.width - 1,
    );
    let (min_x, max_x) = (
        shape.bottom_left.x.min(end.x),
        shape.bottom_left.x.max(end.x),
    );
    let (min_y, max_y) = (
        shape.bottom_left.y.min(end.y),
        shape.bottom_left.y.max(end.y),
    );
    let (min_z, max_z) = (
        shape.bottom_left.z.min(end.z),
        shape.bottom_left.z.max(end.z),
    );
    let mut positions = Vec::with_capacity((shape.width * shape.height) as usize);
    // `BlockPos.betweenClosed` is x-fastest (`BlockPos.java`).
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                positions.push(BlockPos { x, y, z });
            }
        }
    }
    positions
}

fn create_portal(level: &mut LevelCtx<'_, '_>, shape: PortalShape) {
    let portal = default_block_state_id(VANILLA_BLOCK_NETHER_PORTAL_ID);
    let portal_state = with_property(portal, "axis", shape.axis.property()).unwrap_or(portal);
    for pos in portal_positions(shape) {
        level.set_block(
            pos,
            portal_state,
            PORTAL_UPDATE_FLAGS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// The fire an igniter item writes at `pos`: `BaseFireBlock.canBePlacedAt`
/// (`BaseFireBlock.java:188-216`) over `getState` (`:45-49`). The portal
/// test tries both axes rather than the click's preferred one first.
pub(crate) fn igniter_fire_state(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let (state, survives) = air_fire_state(level, pos)?;
    let portal = || {
        level.in_nether_portal_dimension()
            && DIRECTION_VALUES.into_iter().any(|direction| {
                state_is_block(
                    level.block_state(relative(pos, direction)),
                    VANILLA_BLOCK_OBSIDIAN_ID,
                )
            })
            && find_empty_portal(level, pos).is_some()
    };
    (survives || portal()).then_some(state)
}

/// The fire `LightningBolt.spawnFire` writes at `pos`: air, and a
/// `BaseFireBlock.getState` that survives there (`LightningBolt.java:171-183`).
pub(crate) fn surviving_fire_state(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let (state, survives) = air_fire_state(level, pos)?;
    survives.then_some(state)
}

fn air_fire_state(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<(i32, bool)> {
    if !block_state_is_air(level.block_state(pos)) {
        return None;
    }
    let state = state_for_placement_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    })?;
    let survives = if super::footing::is_soul_fire(state) {
        super::footing::soul_fire_can_survive_at(level, pos)
    } else {
        can_survive_at(level, pos)
    };
    Some((state, survives == Some(true)))
}

/// `BaseFireBlock.onPlace` (`BaseFireBlock.java`).
fn base_on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32, old_state: i32) {
    if block_type_of(block_state_id) == block_type_of(old_state) {
        return;
    }
    if level.in_nether_portal_dimension() {
        if let Some(shape) = find_empty_portal(level, pos) {
            create_portal(level, shape);
            return;
        }
    }
    let survives = if super::footing::is_soul_fire(block_state_id) {
        super::footing::soul_fire_can_survive_at(level, pos)
    } else {
        can_survive_at(level, pos)
    };
    if survives == Some(false) {
        level.remove_block(pos);
    }
}

fn schedule_next(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> Option<()> {
    let tick_type = BlockTickType::of_block_state(block_state_id)?;
    // `FireBlock.getFireTickDelay` (`FireBlock.java`).
    let delay = 30 + level.level_random().next_int(10);
    level.schedule_block_tick(pos, tick_type, delay, TickPriority::Normal);
    Some(())
}

/// `FireBlock.onPlace` (`FireBlock.java`).
pub(super) fn on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    base_on_place(level, pos, block_state_id, old_state);
    let _ = schedule_next(level, pos, block_state_id);
}

pub(super) fn soul_fire_on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    old_state: i32,
) {
    base_on_place(level, pos, block_state_id, old_state);
}

/// The generic table schedules the fire; the world applies [`tick_with_facts`].
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let _ = schedule_next(level, pos, block_state_id);
}

/// `FireBlock.isNearRain` (`FireBlock.java`).
///
/// `Level.isRainingAt` needs the heightmap and the biome at each of the five
/// positions (`Level.java` over `Level.java`), which is a read
/// of the level rather than of the fire environment: an environment alive
/// beside a `&mut` level cannot hold the block store to answer it.
fn near_rain(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    [
        pos,
        relative(pos, Direction::West),
        relative(pos, Direction::East),
        relative(pos, Direction::North),
        relative(pos, Direction::South),
    ]
    .into_iter()
    .any(|at| level.is_raining_at(at))
}

fn check_burn_out(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    chance: i32,
    age: i32,
) -> Option<()> {
    let content_registry = level.content_registry();
    let odds_state = level.block_state(pos);
    block_type_or_custom(odds_state, content_registry)?;
    let burn_odds = fire_odds(odds_state, content_registry)?.1;
    if level.level_random().next_int(chance) >= burn_odds {
        return Some(());
    }

    let old_state = level.block_state(pos);
    let old_type = block_type_or_custom(old_state, level.content_registry())?;
    let next_int = level.level_random().next_int(age + 10);
    if next_int < 5 && !level.is_raining_at(pos) {
        let new_age = (age + level.level_random().next_int(5) / 4).min(MAX_AGE);
        let state = state_with_age(level, pos, new_age, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        })?;
        level.set_block(
            pos,
            state,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    } else {
        level.remove_block(pos);
    }

    if old_type == VANILLA_BLOCK_TNT_ID {
        let primed = level.prime_tnt_from_fire(pos)?;
        if primed {
            level.play_block_sound(pos, "minecraft:entity.tnt.primed");
            level.block_game_event(crate::level_event::VanillaGameEvent::PRIME_FUSE, pos, None);
        }
    }
    Some(())
}

fn ignite_odds_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let content_registry = level.content_registry();
    let state = level.block_state(pos);
    block_type_or_custom(state, content_registry)?;
    if !block_state_is_air(state) {
        return Some(0);
    }
    let mut odds = 0;
    for direction in DIRECTION_VALUES {
        let neighbour = level.block_state(relative(pos, direction));
        block_type_or_custom(neighbour, content_registry)?;
        odds = odds.max(fire_odds(neighbour, content_registry)?.0);
    }
    Some(odds)
}

/// `FireBlock.tick` (`FireBlock.java`).
pub(crate) fn tick_with_facts(
    level: &mut LevelCtx<'_, '_>,
    environment: &FireFacts<'_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<()> {
    schedule_next(level, pos, block_state_id)?;
    if !can_spread_fire_around(environment, pos) {
        return Some(());
    }
    if !can_survive_at(level, pos)? {
        level.remove_block(pos);
    }

    let below_state = level.block_state(below(pos));
    let below_type = block_type_or_custom(below_state, level.content_registry())?;
    let infini_burn = environment.infiniburn_blocks.contains(&below_type);
    let age = int_property(block_state_id, AGE)?;
    if !infini_burn
        && environment.raining
        && near_rain(level, pos)
        && level.level_random().next_float() < 0.2 + age as f32 * 0.03
    {
        level.remove_block(pos);
        return Some(());
    }

    let new_age = (age + level.level_random().next_int(3) / 2).min(MAX_AGE);
    let mut state = block_state_id;
    if age != new_age {
        state = with_property(state, AGE, &new_age.to_string())?;
        level.set_block(
            pos,
            state,
            FIRE_UPDATE_FLAGS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }

    if !infini_burn {
        if !valid_fire_location_at(level, pos)? {
            let below_pos = below(pos);
            let below_state = level.block_state(below_pos);
            block_type_or_custom(below_state, level.content_registry())?;
            if !bbb_server_types::block_queries::face_sturdy_at_with_registry(
                below_state,
                below_pos,
                Direction::Up,
                SupportType::Full,
                level.content_registry(),
            ) || age > 3
            {
                level.remove_block(pos);
            }
            return Some(());
        }
        if age == MAX_AGE
            && level.level_random().next_int(4) == 0
            && !can_burn(level.block_state(below(pos)), level.content_registry())?
        {
            level.remove_block(pos);
            return Some(());
        }
    }

    let increased = level.increased_fire_burnout(pos)?;
    let extra = if increased { -50 } else { 0 };
    for (direction, chance) in [
        (Direction::East, 300),
        (Direction::West, 300),
        (Direction::Down, 250),
        (Direction::Up, 250),
        (Direction::North, 300),
        (Direction::South, 300),
    ] {
        check_burn_out(level, relative(pos, direction), chance + extra, age)?;
    }

    let difficulty = environment.difficulty_id;
    for xx in -1..=1 {
        for zz in -1..=1 {
            for yy in -1..=4 {
                if xx == 0 && yy == 0 && zz == 0 {
                    continue;
                }
                let mut rate = 100;
                if yy > 1 {
                    rate += (yy - 1) * 100;
                }
                let test_pos = offset(pos, xx, yy, zz);
                let ignite_odds = ignite_odds_at(level, test_pos)?;
                if ignite_odds <= 0 {
                    continue;
                }
                let mut odds = (ignite_odds + 40 + difficulty * 7) / (age + 30);
                if increased {
                    odds /= 2;
                }
                if odds > 0 && level.level_random().next_int(rate) <= odds {
                    let raining = environment.raining;
                    if !raining || !near_rain(level, test_pos) {
                        let spread_age = (age + level.level_random().next_int(5) / 4).min(MAX_AGE);
                        let spread_state =
                            state_with_age(level, test_pos, spread_age, &|registry, tag, id| {
                                level.registry_tag_membership(registry, tag, id)
                            })?;
                        level.set_block(
                            test_pos,
                            spread_state,
                            SET_BLOCK_UPDATE_ALL,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        );
                    }
                }
            }
        }
    }
    Some(())
}

/// `LavaFluid.isFlammable` (`LavaFluid.java`). The question is
/// `ignitedByLava`, not the `FireBlock.bootStrap` odds [`can_burn`] reads: a
/// chest answers yes with no odds at all, and a coal block answers no with
/// odds of five.
fn lava_flammable(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    let inside_build_height = pos.y >= level.get_min_y() && pos.y <= level.get_max_y();
    if inside_build_height && !level.has_chunk_at(pos) {
        return Some(false);
    }
    vanilla_block_state_ignited_by_lava(level.block_state(pos))
}

fn has_lava_flammable_neighbour(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    for direction in DIRECTION_VALUES {
        if lava_flammable(level, relative(pos, direction))? {
            return Some(true);
        }
    }
    Some(false)
}

/// `LavaFluid.randomTick` (`LavaFluid.java`).
pub(crate) fn lava_random_tick(
    level: &mut LevelCtx<'_, '_>,
    environment: &FireFacts<'_>,
    pos: BlockPos,
) -> Option<()> {
    if !can_spread_fire_around(environment, pos) {
        return Some(());
    }
    let passes = level.level_random().next_int(3);
    if passes > 0 {
        let mut test_pos = pos;
        for _ in 0..passes {
            let dx = level.level_random().next_int(3) - 1;
            let dz = level.level_random().next_int(3) - 1;
            test_pos = offset(test_pos, dx, 1, dz);
            if !level.has_chunk_at(test_pos) {
                return Some(());
            }
            let state = level.block_state(test_pos);
            block_type_or_custom(state, level.content_registry())?;
            if block_state_is_air(state) {
                if has_lava_flammable_neighbour(level, test_pos)? {
                    let fire = state_for_placement_at(level, test_pos, &|registry, tag, id| {
                        level.registry_tag_membership(registry, tag, id)
                    })?;
                    level.set_block(
                        test_pos,
                        fire,
                        SET_BLOCK_UPDATE_ALL,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                    return Some(());
                }
            } else if bbb_server_types::fluid::block_state_blocks_motion_with_registry(
                state,
                level.content_registry(),
            ) {
                return Some(());
            }
        }
    } else {
        for _ in 0..3 {
            let dx = level.level_random().next_int(3) - 1;
            let dz = level.level_random().next_int(3) - 1;
            let test_pos = offset(pos, dx, 0, dz);
            if !level.has_chunk_at(test_pos) {
                return Some(());
            }
            let above_pos = above(test_pos);
            let above_state = level.block_state(above_pos);
            block_type_or_custom(above_state, level.content_registry())?;
            if block_state_is_air(above_state) && lava_flammable(level, test_pos)? {
                // The write lands above, but the state is derived at `testPos`
                // itself (`LavaFluid.java`).
                let fire = state_for_placement_at(level, test_pos, &|registry, tag, id| {
                    level.registry_tag_membership(registry, tag, id)
                })?;
                level.set_block(
                    above_pos,
                    fire,
                    SET_BLOCK_UPDATE_ALL,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        }
    }
    Some(())
}

/// `Mth.wrapDegrees(float)` (`Mth.java`).
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

/// `NetherPortalBlock.randomTick` (`NetherPortalBlock.java`).
pub(crate) fn portal_random_tick(
    level: &mut LevelCtx<'_, '_>,
    environment: &FireFacts<'_>,
    pos: BlockPos,
) -> Option<()> {
    if !environment.spawning_monsters || !level.nether_portal_spawns_piglins(pos)? {
        return Some(());
    }
    let difficulty = environment.difficulty_id;
    if level.level_random().next_int(2000) >= difficulty
        || !any_player_close_enough_for_spawning(environment, pos)
    {
        return Some(());
    }

    let mut floor = pos;
    let mut floor_state = level.block_state(floor);
    while block_type_or_custom(floor_state, level.content_registry())?
        == VANILLA_BLOCK_NETHER_PORTAL_ID
    {
        floor = below(floor);
        floor_state = level.block_state(floor);
    }
    if !level.is_valid_spawn(floor_state, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID)? {
        return Some(());
    }

    let spawn = above(floor);
    let y_rot = wrap_degrees(level.level_random().next_float() * 360.0);
    let _ = level.request_block_mob_spawn(BlockMobSpawnRequest::PortalZombifiedPiglin {
        position: Vec3d {
            x: f64::from(spawn.x) + 0.5,
            y: f64::from(spawn.y),
            z: f64::from(spawn.z) + 0.5,
        },
        y_rot,
    });
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
        vanilla_block_state_identity,
    };
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use std::collections::BTreeMap;

    fn default_state(name: &str) -> i32 {
        vanilla_block_state_id_by_name_and_properties(name, &BTreeMap::new()).unwrap()
    }

    fn catalog_default_state(name: &str) -> i32 {
        vanilla_block_type_for_resource_id(name)
            .and_then(|block| vanilla_block_default_state_id(block.id))
            .unwrap()
    }

    fn fire_at(pos: BlockPos, background: i32, cells: &[(BlockPos, i32)]) -> Option<i32> {
        state_for_placement(
            pos,
            crate::registry_tags::RegistryTagSnapshot::builtin(),
            &mut |probe| {
                cells
                    .iter()
                    .find(|(at, _)| *at == probe)
                    .map_or(background, |(_, state)| *state)
            },
        )
    }

    /// `BaseFireBlock.getState` as end spikes place it: soul fire on soul
    /// sand, plain fire on a sturdy floor, and a side-attached fire facing the
    /// flammable neighbour.
    #[test]
    fn placed_fire_state_follows_its_support() {
        let pos = BlockPos { x: 4, y: 69, z: -8 };
        let air = default_state("minecraft:air");
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };

        let soul = fire_at(pos, air, &[(below, default_state("minecraft:soul_sand"))]);
        assert_eq!(soul, Some(default_state("minecraft:soul_fire")));

        let sturdy = fire_at(pos, air, &[(below, default_state("minecraft:stone"))]);
        assert_eq!(sturdy, Some(catalog_default_state("minecraft:fire")));

        let east = BlockPos {
            x: pos.x + 1,
            ..pos
        };
        let fire = fire_at(pos, air, &[(east, default_state("minecraft:oak_planks"))]).unwrap();
        let identity = vanilla_block_state_identity(fire).unwrap();
        assert_eq!(
            identity.properties.get("east").map(String::as_str),
            Some("true")
        );
        for direction in ["up", "north", "south", "west"] {
            assert_eq!(
                identity.properties.get(direction).map(String::as_str),
                Some("false"),
                "{direction}"
            );
        }

        assert_eq!(fire_at(pos, i32::MAX, &[]), None);
    }
}
