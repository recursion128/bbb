//! Nether-portal target lookup over the authoritative loaded block store.

use crate::world::LevelId;
mod end;
mod gateway;

use std::collections::{BTreeMap, BTreeSet};

use crate::block_behavior::default_block_state_id;
use crate::entity::portal::PortalKind;
use crate::entity::{Entity, StoredEntityTransition};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_identity, vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_NETHER_PORTAL_ID, VANILLA_BLOCK_OBSIDIAN_ID};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_shape, dimension_type_shape, DimensionTypeShape,
};
use bbb_server_types::fluid::{air_block_state_id, block_state_fluid_state};
use bbb_server_types::geometry::{collect_block_collisions_with_moving_pistons, Aabb};
use bbb_server_types::save::{Java26BuiltinDimension, SaveError, SaveResult};

use super::{
    block_position, is_in_world_bounds, ServerWorld, WorldBorder, WorldEvent,
    SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_KNOWN_SHAPE,
};

const TO_NETHER_RADIUS: i32 = 16;
const TO_OVERWORLD_RADIUS: i32 = 128;
const RECTANGLE_LIMIT: i32 = 21;
const BORDER_EPSILON: f64 = 1.0e-5_f32 as f64;
const COLLISION_EPSILON: f64 = 1.0e-6;
const PORTAL_RELATIVES_MASK: i32 = (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PortalAxis {
    X,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PortalRectangle {
    pub(crate) min_corner: BlockPos,
    pub(crate) axis: PortalAxis,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug)]
pub(crate) struct PreparedNetherPortalSearch {
    pub(crate) target: LevelId,
    pub(crate) approximate_exit: BlockPos,
    pub(crate) source_axis: PortalAxis,
    pub(crate) found_position: Option<BlockPos>,
    pub(crate) rectangle: Option<PortalRectangle>,
    full_chunks: Vec<(i32, i32)>,
}

/// The body a Nether portal moves: `Entity.position()` and
/// `getDimensions(getPose())`; `reset_forward` is the living, boat and
/// minecart override of `getRelativePortalPosition`.
#[derive(Debug, Clone, Copy)]
pub(super) struct PortalTraveller {
    pub(super) position: Vec3d,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) reset_forward: bool,
}

impl PortalTraveller {
    fn of_entity(entity: &Entity) -> Self {
        Self {
            position: entity.position(),
            width: entity.bb_width(),
            height: entity.bb_height(),
            reset_forward: entity.living().is_some()
                || crate::entity::boat::is_boat(entity.data())
                || crate::entity::minecart::is_minecart(entity.data()),
        }
    }
}

pub(super) struct NetherPortalDestination {
    pub(super) target: LevelId,
    pub(super) transition: StoredEntityTransition,
    pub(super) ticket: super::portal_tickets::PreparedPortalTicket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PortalBlockWrite {
    pos: BlockPos,
    state: i32,
    flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PortalCreationPlan {
    target: LevelId,
    rectangle: PortalRectangle,
    writes: Vec<PortalBlockWrite>,
}

#[derive(Debug)]
pub(crate) struct PreparedNetherPortalCreation {
    pub(crate) rectangle: PortalRectangle,
    #[cfg(test)]
    pub(crate) write_trace: Vec<(BlockPos, i32, i32)>,
}

const RULE_ALLOW_ENTERING_NETHER_USING_PORTALS: &str =
    "minecraft:allow_entering_nether_using_portals";

fn invalid_search(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("nether portal search: {}", detail.into()),
    }
}

fn portal_axis(state: i32) -> Option<PortalAxis> {
    if vanilla_block_state_block_type_id(state) != Some(VANILLA_BLOCK_NETHER_PORTAL_ID) {
        return None;
    }
    match vanilla_block_state_identity(state)?
        .properties
        .get("axis")?
        .as_str()
    {
        "x" => Some(PortalAxis::X),
        "z" => Some(PortalAxis::Z),
        _ => None,
    }
}

pub(super) fn dimension_shape(
    world: &ServerWorld,
    dimension: &LevelId,
) -> SaveResult<DimensionTypeShape> {
    let element_id = world
        .cold
        .level_dimension_types
        .get(dimension)
        .ok_or_else(|| {
            invalid_search(format!(
                "{} has no dimension type",
                world.level_key(*dimension).resource_id()
            ))
        })?;
    let result = match world.cold.config.datapacks.as_deref() {
        Some(startup) => dimension_type_shape(&startup.registries, element_id),
        None => builtin_dimension_type_shape(element_id),
    };
    result.map_err(|error| invalid_search(error.to_string()))
}

// `Mth.clamp` (`Mth.java:105-107`) permits restored inverted border bounds.
fn vanilla_clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

// `WorldBorder.clampVec3ToBound` (`WorldBorder.java:84-94`).
fn clamp_to_border(border: WorldBorder, game_time: i64, position: Vec3d) -> BlockPos {
    BlockPos {
        x: vanilla_clamp(
            position.x,
            border.min_x_at(game_time),
            border.max_x_at(game_time) - BORDER_EPSILON,
        )
        .floor() as i32,
        y: position.y.floor() as i32,
        z: vanilla_clamp(
            position.z,
            border.min_z_at(game_time),
            border.max_z_at(game_time) - BORDER_EPSILON,
        )
        .floor() as i32,
    }
}

/// Chunk window covering the creation spiral (radius 16) plus the 2-block frame
/// padding around each spiral column.
fn creation_window(center: BlockPos) -> SaveResult<Vec<(i32, i32)>> {
    const CREATION_CHUNK_PADDING: i32 = 18;
    let min_x = center
        .x
        .checked_sub(CREATION_CHUNK_PADDING)
        .ok_or_else(|| invalid_search("creation window overflows min block x"))?
        >> 4;
    let max_x = center
        .x
        .checked_add(CREATION_CHUNK_PADDING)
        .ok_or_else(|| invalid_search("creation window overflows max block x"))?
        >> 4;
    let min_z = center
        .z
        .checked_sub(CREATION_CHUNK_PADDING)
        .ok_or_else(|| invalid_search("creation window overflows min block z"))?
        >> 4;
    let max_z = center
        .z
        .checked_add(CREATION_CHUNK_PADDING)
        .ok_or_else(|| invalid_search("creation window overflows max block z"))?
        >> 4;
    let mut chunks = Vec::new();
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            chunks.push((x, z));
        }
    }
    Ok(chunks)
}

/// One-chunk margin around an exit portal rectangle so collision-free placement
/// never misses an adjacent chunk on chunk-boundary portals.
fn exit_margin_chunks(exit: PortalRectangle) -> SaveResult<Vec<(i32, i32)>> {
    let max_corner = offset(exit.min_corner, exit.axis, (exit.width - 1).max(0), 0);
    let min_x = (exit.min_corner.x.min(max_corner.x) >> 4)
        .checked_sub(1)
        .ok_or_else(|| invalid_search("exit margin overflows min chunk x"))?;
    let max_x = (exit.min_corner.x.max(max_corner.x) >> 4)
        .checked_add(1)
        .ok_or_else(|| invalid_search("exit margin overflows max chunk x"))?;
    let min_z = (exit.min_corner.z.min(max_corner.z) >> 4)
        .checked_sub(1)
        .ok_or_else(|| invalid_search("exit margin overflows min chunk z"))?;
    let max_z = (exit.min_corner.z.max(max_corner.z) >> 4)
        .checked_add(1)
        .ok_or_else(|| invalid_search("exit margin overflows max chunk z"))?;
    let mut chunks = Vec::new();
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            chunks.push((x, z));
        }
    }
    Ok(chunks)
}

fn offset(pos: BlockPos, axis: PortalAxis, horizontal: i32, vertical: i32) -> BlockPos {
    match axis {
        PortalAxis::X => BlockPos {
            x: pos.x + horizontal,
            y: pos.y + vertical,
            z: pos.z,
        },
        PortalAxis::Z => BlockPos {
            x: pos.x,
            y: pos.y + vertical,
            z: pos.z + horizontal,
        },
    }
}

fn get_limit(
    state_at: &impl Fn(BlockPos) -> Option<i32>,
    center: BlockPos,
    state: i32,
    axis: PortalAxis,
    horizontal_step: i32,
    vertical_step: i32,
    limit: i32,
) -> i32 {
    let mut distance = 0;
    while distance < limit {
        let next = offset(
            center,
            axis,
            horizontal_step * (distance + 1),
            vertical_step * (distance + 1),
        );
        if state_at(next) != Some(state) {
            break;
        }
        distance += 1;
    }
    distance
}

fn max_rectangle_location(columns: &[i32]) -> ((usize, usize), i32) {
    let mut max_start = 0;
    let mut max_end = 0;
    let mut max_height = 0;
    let mut stack = vec![0_usize];
    for column in 1..=columns.len() {
        let height = if column == columns.len() {
            0
        } else {
            columns[column]
        };
        while let Some(&top) = stack.last() {
            let stack_height = columns[top];
            if height >= stack_height {
                stack.push(column);
                break;
            }
            stack.pop();
            let start = stack.last().map_or(0, |top| top + 1);
            if stack_height * (column - start) as i32 > max_height * (max_end - max_start) as i32 {
                max_end = column;
                max_start = start;
                max_height = stack_height;
            }
        }
        if stack.is_empty() {
            stack.push(column);
        }
    }
    ((max_start, max_end - 1), max_height)
}

// `BlockUtil.getLargestRectangleAround` (`BlockUtil.java:15-75`).
fn largest_rectangle(
    state_at: &impl Fn(BlockPos) -> Option<i32>,
    center: BlockPos,
    state: i32,
    axis: PortalAxis,
) -> PortalRectangle {
    let negative_width = get_limit(state_at, center, state, axis, -1, 0, RECTANGLE_LIMIT);
    let positive_width = get_limit(state_at, center, state, axis, 1, 0, RECTANGLE_LIMIT);
    let center_width = negative_width as usize;
    let mut bounds = vec![(0_i32, 0_i32); (negative_width + 1 + positive_width) as usize];
    bounds[center_width] = (
        get_limit(state_at, center, state, axis, 0, -1, RECTANGLE_LIMIT),
        get_limit(state_at, center, state, axis, 0, 1, RECTANGLE_LIMIT),
    );
    let center_height = bounds[center_width].0;

    for distance in 1..=negative_width {
        let previous = bounds[center_width - distance as usize + 1];
        let at = offset(center, axis, -distance, 0);
        bounds[center_width - distance as usize] = (
            get_limit(state_at, at, state, axis, 0, -1, previous.0),
            get_limit(state_at, at, state, axis, 0, 1, previous.1),
        );
    }
    for distance in 1..=positive_width {
        let previous = bounds[center_width + distance as usize - 1];
        let at = offset(center, axis, distance, 0);
        bounds[center_width + distance as usize] = (
            get_limit(state_at, at, state, axis, 0, -1, previous.0),
            get_limit(state_at, at, state, axis, 0, 1, previous.1),
        );
    }

    let mut min_width = 0;
    let mut min_height = 0;
    let mut width = 0;
    let mut height = 0;
    let mut columns = vec![0_i32; bounds.len()];
    for height_index in (0..=center_height).rev() {
        for (index, &(negative, positive)) in bounds.iter().enumerate() {
            let lower = center_height - negative;
            let upper = center_height + positive;
            columns[index] = if (lower..=upper).contains(&height_index) {
                upper + 1 - height_index
            } else {
                0
            };
        }
        let ((start, end), candidate_height) = max_rectangle_location(&columns);
        let candidate_width = 1 + end - start;
        if candidate_width as i32 * candidate_height > width * height {
            min_width = start as i32;
            min_height = height_index;
            width = candidate_width as i32;
            height = candidate_height;
        }
    }
    PortalRectangle {
        min_corner: offset(
            center,
            axis,
            min_width - negative_width,
            min_height - center_height,
        ),
        axis,
        width,
        height,
    }
}

fn portal_poi_candidates(
    chunks: &mut crate::chunk::ChunkStore,
    border: WorldBorder,
    game_time: i64,
    center: BlockPos,
    radius: i32,
) -> Vec<BlockPos> {
    // `PortalForcer.findClosestPortalPosition` (`PortalForcer.java:41-50`):
    // query `minecraft:nether_portal` POIs in the square without generating
    // terrain, filter to world border, then order by distance squared then Y.
    let mut candidates = chunks
        .poi_mut()
        .records_in_square(
            [center.x, center.y, center.z],
            radius,
            crate::chunk::poi::PoiOccupancy::Any,
            |record| record.type_id == "minecraft:nether_portal",
        )
        .into_iter()
        .map(|record| BlockPos {
            x: record.pos[0],
            y: record.pos[1],
            z: record.pos[2],
        })
        .filter(|&pos| border.contains_block_at(pos, game_time))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        block_distance_squared(*left, center)
            .total_cmp(&block_distance_squared(*right, center))
            .then(left.y.cmp(&right.y))
    });
    candidates
}

fn rectangle_chunks(candidates: &[(BlockPos, i32)]) -> SaveResult<Vec<(i32, i32)>> {
    let mut chunks = BTreeSet::new();
    for &(position, state) in candidates {
        let axis = portal_axis(state).expect("portal candidate has an axis");
        for horizontal in -RECTANGLE_LIMIT..=RECTANGLE_LIMIT {
            let at = match axis {
                PortalAxis::X => position
                    .x
                    .checked_add(horizontal)
                    .map(|x| (x >> 4, position.z >> 4)),
                PortalAxis::Z => position
                    .z
                    .checked_add(horizontal)
                    .map(|z| (position.x >> 4, z >> 4)),
            }
            .ok_or_else(|| invalid_search("portal rectangle window overflows block position"))?;
            chunks.insert(at);
        }
    }
    Ok(chunks.into_iter().collect())
}

fn found_portal_rectangle(
    chunks: &crate::chunk::ChunkStore,
    found: Option<(BlockPos, i32)>,
) -> (Option<BlockPos>, Option<PortalRectangle>) {
    let Some((position, state)) = found else {
        return (None, None);
    };
    let state_at = |pos: BlockPos| chunks.get_block_state(pos.x, pos.y, pos.z);
    let axis = portal_axis(state).expect("portal candidate has an axis");
    (
        Some(position),
        Some(largest_rectangle(&state_at, position, state, axis)),
    )
}

fn axis_coordinate(position: Vec3d, axis: PortalAxis) -> f64 {
    match axis {
        PortalAxis::X => position.x,
        PortalAxis::Z => position.z,
    }
}

// `PortalShape.getRelativePosition` (`PortalShape.java:198-222`).
fn relative_portal_position(traveller: PortalTraveller, portal: PortalRectangle) -> Vec3d {
    let width = f64::from(portal.width) - f64::from(traveller.width);
    let height = f64::from(portal.height) - f64::from(traveller.height);
    let min = portal.min_corner;
    let relative_right = if width > 0.0 {
        let bottom_start = match portal.axis {
            PortalAxis::X => f64::from(min.x),
            PortalAxis::Z => f64::from(min.z),
        } + f64::from(traveller.width / 2.0);
        vanilla_clamp(
            (axis_coordinate(traveller.position, portal.axis) - bottom_start) / width,
            0.0,
            1.0,
        )
    } else {
        0.5
    };
    let relative_up = if height > 0.0 {
        vanilla_clamp((traveller.position.y - f64::from(min.y)) / height, 0.0, 1.0)
    } else {
        0.0
    };
    let relative_forward = if traveller.reset_forward {
        0.0
    } else {
        match portal.axis {
            PortalAxis::X => traveller.position.z - (f64::from(min.z) + 0.5),
            PortalAxis::Z => traveller.position.x - (f64::from(min.x) + 0.5),
        }
    };
    Vec3d {
        x: relative_right,
        y: relative_up,
        z: relative_forward,
    }
}

pub(super) fn shape_within_border(shape: Aabb, border: WorldBorder, game_time: i64) -> bool {
    let min_x = border.min_x_at(game_time);
    let max_x = border.max_x_at(game_time);
    let min_z = border.min_z_at(game_time);
    let max_z = border.max_z_at(game_time);
    shape.min_x >= min_x
        && shape.min_x < max_x
        && shape.max_x - BORDER_EPSILON >= min_x
        && shape.max_x - BORDER_EPSILON < max_x
        && shape.min_z >= min_z
        && shape.min_z < max_z
        && shape.max_z - BORDER_EPSILON >= min_z
        && shape.max_z - BORDER_EPSILON < max_z
}

fn add_partition_coordinate(coordinates: &mut Vec<f64>, value: f64, min: f64, max: f64) {
    if value > min && value < max {
        coordinates.push(value);
    }
}

fn partition_coordinates(
    min: f64,
    max: f64,
    obstacles: &[Aabb],
    bounds: impl Fn(Aabb) -> (f64, f64),
) -> Vec<f64> {
    let mut coordinates = vec![min, max];
    for obstacle in obstacles {
        let (obstacle_min, obstacle_max) = bounds(*obstacle);
        add_partition_coordinate(&mut coordinates, obstacle_min, min, max);
        add_partition_coordinate(&mut coordinates, obstacle_max, min, max);
    }
    coordinates.sort_by(f64::total_cmp);
    coordinates.dedup_by(|left, right| left.to_bits() == right.to_bits());
    coordinates
}

fn point_in_box(point: Vec3d, box_: Aabb) -> bool {
    point.x > box_.min_x
        && point.x < box_.max_x
        && point.y > box_.min_y
        && point.y < box_.max_y
        && point.z > box_.min_z
        && point.z < box_.max_z
}

fn clamp_point(point: Vec3d, box_: Aabb) -> Vec3d {
    Vec3d {
        x: point.x.clamp(box_.min_x, box_.max_x),
        y: point.y.clamp(box_.min_y, box_.max_y),
        z: point.z.clamp(box_.min_z, box_.max_z),
    }
}

fn distance_squared(left: Vec3d, right: Vec3d) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    dx * dx + dy * dy + dz * dz
}

fn free_cell_index(x: usize, y: usize, z: usize, x_size: usize, z_size: usize) -> usize {
    (y * x_size + x) * z_size + z
}

fn z_strip_is_free(
    free_cells: &[bool],
    x: usize,
    y: usize,
    start_z: usize,
    end_z: usize,
    x_size: usize,
    z_size: usize,
) -> bool {
    x < x_size && (start_z..end_z).all(|z| free_cells[free_cell_index(x, y, z, x_size, z_size)])
}

fn clear_z_strip(
    free_cells: &mut [bool],
    x: usize,
    y: usize,
    start_z: usize,
    end_z: usize,
    x_size: usize,
    z_size: usize,
) {
    for z in start_z..end_z {
        free_cells[free_cell_index(x, y, z, x_size, z_size)] = false;
    }
}

pub(super) fn closest_free_center(
    allowed: Aabb,
    preferred: Vec3d,
    obstacles: &[Aabb],
) -> Option<Vec3d> {
    let xs = partition_coordinates(allowed.min_x, allowed.max_x, obstacles, |box_| {
        (box_.min_x, box_.max_x)
    });
    let ys = partition_coordinates(allowed.min_y, allowed.max_y, obstacles, |box_| {
        (box_.min_y, box_.max_y)
    });
    let zs = partition_coordinates(allowed.min_z, allowed.max_z, obstacles, |box_| {
        (box_.min_z, box_.max_z)
    });
    let x_size = xs.len() - 1;
    let y_size = ys.len() - 1;
    let z_size = zs.len() - 1;
    let mut free_cells = vec![false; x_size * y_size * z_size];
    for y in 0..y_size {
        for x in 0..x_size {
            for z in 0..z_size {
                let center = Vec3d {
                    x: (xs[x] + xs[x + 1]) / 2.0,
                    y: (ys[y] + ys[y + 1]) / 2.0,
                    z: (zs[z] + zs[z + 1]) / 2.0,
                };
                free_cells[free_cell_index(x, y, z, x_size, z_size)] = !obstacles
                    .iter()
                    .any(|obstacle| point_in_box(center, *obstacle));
            }
        }
    }
    let mut closest: Option<(f64, Vec3d)> = None;
    // `BitSetDiscreteVoxelShape.forAllBoxes` walks Y, X, then Z and merges
    // adjacent strips along Z, X, then Y
    // (`BitSetDiscreteVoxelShape.java:160-199`).
    for y in 0..y_size {
        for x in 0..x_size {
            let mut start_z = None;
            for z in 0..=z_size {
                let is_free = z < z_size && free_cells[free_cell_index(x, y, z, x_size, z_size)];
                if is_free {
                    start_z.get_or_insert(z);
                } else if let Some(first_z) = start_z.take() {
                    let mut end_x = x;
                    let mut end_y = y;
                    clear_z_strip(&mut free_cells, x, y, first_z, z, x_size, z_size);
                    while z_strip_is_free(&free_cells, end_x + 1, y, first_z, z, x_size, z_size) {
                        end_x += 1;
                        clear_z_strip(&mut free_cells, end_x, y, first_z, z, x_size, z_size);
                    }
                    while end_y + 1 < y_size
                        && (x..=end_x).all(|candidate_x| {
                            z_strip_is_free(
                                &free_cells,
                                candidate_x,
                                end_y + 1,
                                first_z,
                                z,
                                x_size,
                                z_size,
                            )
                        })
                    {
                        end_y += 1;
                        for candidate_x in x..=end_x {
                            clear_z_strip(
                                &mut free_cells,
                                candidate_x,
                                end_y,
                                first_z,
                                z,
                                x_size,
                                z_size,
                            );
                        }
                    }
                    let box_ = Aabb {
                        min_x: xs[x],
                        min_y: ys[y],
                        min_z: zs[first_z],
                        max_x: xs[end_x + 1],
                        max_y: ys[end_y + 1],
                        max_z: zs[z],
                    };
                    let candidate = clamp_point(preferred, box_);
                    let distance = distance_squared(preferred, candidate);
                    if closest.is_none_or(|(best, _)| distance < best) {
                        closest = Some((distance, candidate));
                    }
                }
            }
        }
    }
    closest.map(|(_, point)| point)
}

fn portal_collision_boxes(chunks: &crate::chunk::ChunkStore, pos: BlockPos) -> Vec<Aabb> {
    if !chunks.is_inside_build_height(pos.y) {
        return Vec::new();
    }
    let Some(state) = chunks.get_block_state(pos.x, pos.y, pos.z) else {
        return vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })];
    };
    super::movement_collision::static_collision_shape_with_live_piston(chunks, pos, state)
}

// `PortalShape.findCollisionFreePosition` (`PortalShape.java:225-239`) over
// `CollisionGetter.findFreePosition` (`CollisionGetter.java:155-170`).
fn collision_free_position(
    chunks: &crate::chunk::ChunkStore,
    border: WorldBorder,
    game_time: i64,
    bottom_center: Vec3d,
    width: f32,
    height: f32,
) -> SaveResult<Vec3d> {
    if width > 4.0 || height > 4.0 {
        return Ok(bottom_center);
    }
    let half_height = f64::from(height) / 2.0;
    let center = Vec3d {
        y: bottom_center.y + half_height,
        ..bottom_center
    };
    let allowed = Aabb::of_size(center, f64::from(width), 0.0, f64::from(width))
        .expand_towards(Vec3d {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        })
        .inflate(COLLISION_EPSILON, COLLISION_EPSILON, COLLISION_EPSILON);
    let search = allowed.inflate(f64::from(width), f64::from(height), f64::from(width));
    let missing_chunk = std::cell::Cell::new(None);
    let unknown_identity = std::cell::Cell::new(false);
    let collisions = collect_block_collisions_with_moving_pistons(
        search,
        |pos| {
            if chunks.column(pos.x >> 4, pos.z >> 4).is_none()
                || !chunks.is_ticking_full(pos.x >> 4, pos.z >> 4)
            {
                if missing_chunk.get().is_none() {
                    missing_chunk.set(Some((pos.x >> 4, pos.z >> 4)));
                }
                return false;
            }
            super::movement_collision::moving_piston_at(chunks, pos).unwrap_or_else(|| {
                unknown_identity.set(true);
                false
            })
        },
        |pos| {
            if chunks.column(pos.x >> 4, pos.z >> 4).is_none()
                || !chunks.is_ticking_full(pos.x >> 4, pos.z >> 4)
            {
                if missing_chunk.get().is_none() {
                    missing_chunk.set(Some((pos.x >> 4, pos.z >> 4)));
                }
                return vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })];
            }
            portal_collision_boxes(chunks, pos)
        },
    );
    if let Some((chunk_x, chunk_z)) = missing_chunk.get() {
        return Err(invalid_search(format!(
            "portal collision chunk ({chunk_x},{chunk_z}) is not FULL-loaded"
        )));
    }
    if unknown_identity.get() {
        return Err(invalid_search(
            "portal collision moving-piston identity is unknown",
        ));
    }
    let positions = collisions
        .into_iter()
        .map(|(pos, _)| (pos.x, pos.y, pos.z))
        .collect::<BTreeSet<_>>();
    let mut obstacles = Vec::new();
    for (x, y, z) in positions {
        let pos = BlockPos { x, y, z };
        let offset = Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        };
        let boxes = portal_collision_boxes(chunks, pos)
            .into_iter()
            .map(|box_| box_.moved(offset))
            .collect::<Vec<_>>();
        let Some(first) = boxes.first().copied() else {
            continue;
        };
        let bounds = boxes.iter().skip(1).fold(first, |bounds, box_| Aabb {
            min_x: bounds.min_x.min(box_.min_x),
            min_y: bounds.min_y.min(box_.min_y),
            min_z: bounds.min_z.min(box_.min_z),
            max_x: bounds.max_x.max(box_.max_x),
            max_y: bounds.max_y.max(box_.max_y),
            max_z: bounds.max_z.max(box_.max_z),
        });
        if !shape_within_border(bounds, border, game_time) {
            continue;
        }
        obstacles.extend(boxes.into_iter().map(|shape| {
            shape.inflate(
                f64::from(width) / 2.0,
                f64::from(height) / 2.0,
                f64::from(width) / 2.0,
            )
        }));
    }
    let free_center = closest_free_center(allowed, center, &obstacles).unwrap_or(center);
    Ok(Vec3d {
        y: free_center.y - half_height,
        ..free_center
    })
}

// `NetherPortalBlock.getDimensionTransitionFromExit` and
// `createDimensionTransition` (`NetherPortalBlock.java:182-233`).
fn nether_portal_transition(
    source_chunks: &crate::chunk::ChunkStore,
    target_chunks: &crate::chunk::ChunkStore,
    target_border: WorldBorder,
    game_time: i64,
    traveller: PortalTraveller,
    portal_entry: BlockPos,
    exit: PortalRectangle,
) -> SaveResult<StoredEntityTransition> {
    let position = traveller.position;
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !position.z.is_finite()
        || !traveller.width.is_finite()
        || !traveller.height.is_finite()
    {
        return Err(invalid_search("portal traveller geometry is not finite"));
    }
    let source_state = source_chunks
        .get_block_state(portal_entry.x, portal_entry.y, portal_entry.z)
        .ok_or_else(|| invalid_search("source portal entry is outside a loaded world cell"))?;
    let (source_axis, relative) = if let Some(axis) = portal_axis(source_state) {
        for horizontal in -RECTANGLE_LIMIT..=RECTANGLE_LIMIT {
            let pos = offset(portal_entry, axis, horizontal, 0);
            if source_chunks.column(pos.x >> 4, pos.z >> 4).is_none()
                || !source_chunks.is_ticking_full(pos.x >> 4, pos.z >> 4)
            {
                return Err(invalid_search(format!(
                    "source portal rectangle chunk ({},{}) is not FULL-loaded",
                    pos.x >> 4,
                    pos.z >> 4
                )));
            }
        }
        let source = largest_rectangle(
            &|pos| source_chunks.get_block_state(pos.x, pos.y, pos.z),
            portal_entry,
            source_state,
            axis,
        );
        (axis, relative_portal_position(traveller, source))
    } else {
        (
            PortalAxis::X,
            Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            },
        )
    };
    let target_axis = target_chunks
        .get_block_state(exit.min_corner.x, exit.min_corner.y, exit.min_corner.z)
        .and_then(portal_axis)
        .unwrap_or(PortalAxis::X);
    let width = f64::from(exit.width);
    let height = f64::from(exit.height);
    let dimensions_width = f64::from(traveller.width);
    let dimensions_height = f64::from(traveller.height);
    let offset_right = dimensions_width / 2.0 + (width - dimensions_width) * relative.x;
    let offset_up = (height - dimensions_height) * relative.y;
    let offset_forward = 0.5 + relative.z;
    let x_aligned = target_axis == PortalAxis::X;
    let bottom_center = Vec3d {
        x: f64::from(exit.min_corner.x)
            + if x_aligned {
                offset_right
            } else {
                offset_forward
            },
        y: f64::from(exit.min_corner.y) + offset_up,
        z: f64::from(exit.min_corner.z)
            + if x_aligned {
                offset_forward
            } else {
                offset_right
            },
    };
    let position = collision_free_position(
        target_chunks,
        target_border,
        game_time,
        bottom_center,
        traveller.width,
        traveller.height,
    )?;
    Ok(StoredEntityTransition {
        position,
        delta_movement: Vec3d::default(),
        y_rot: if source_axis == target_axis {
            0.0
        } else {
            90.0
        },
        x_rot: 0.0,
        relatives_mask: PORTAL_RELATIVES_MASK,
    })
}

fn source_portal_axis(world: &ServerWorld, source: &LevelId, entry: BlockPos) -> PortalAxis {
    world
        .level_by_id(source)
        .chunks
        .get_block_state(entry.x, entry.y, entry.z)
        .and_then(portal_axis)
        .unwrap_or(PortalAxis::X)
}

// `BlockPos.spiralAround(origin, 16, EAST, SOUTH)` (`BlockPos.java:429-463`).
fn creation_spiral(origin: BlockPos) -> Vec<BlockPos> {
    let radius = 16;
    let directions = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let mut cursor = BlockPos {
        z: origin.z + 1,
        ..origin
    };
    let legs = 4 * radius;
    let mut leg = -1;
    let mut leg_size = 0;
    let mut leg_index = 0;
    let mut positions = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);
    loop {
        let direction = directions[((leg + 4) % 4) as usize];
        cursor.x += direction.0;
        cursor.z += direction.1;
        if leg_index >= leg_size {
            if leg >= legs {
                break;
            }
            leg += 1;
            leg_index = 0;
            leg_size = leg / 2 + 1;
        }
        leg_index += 1;
        positions.push(cursor);
    }
    positions
}

fn portal_frame_pos(
    origin: BlockPos,
    axis: PortalAxis,
    width: i32,
    height: i32,
    perpendicular: i32,
) -> BlockPos {
    match axis {
        PortalAxis::X => BlockPos {
            x: origin.x + width,
            y: origin.y + height,
            z: origin.z + perpendicular,
        },
        PortalAxis::Z => BlockPos {
            x: origin.x - perpendicular,
            y: origin.y + height,
            z: origin.z + width,
        },
    }
}

fn creation_state(chunks: &crate::chunk::ChunkStore, pos: BlockPos) -> SaveResult<Option<i32>> {
    if chunks.column(pos.x >> 4, pos.z >> 4).is_none() {
        return Err(invalid_search(format!(
            "portal creation chunk ({},{}) is not FULL-loaded",
            pos.x >> 4,
            pos.z >> 4
        )));
    }
    Ok(chunks.get_block_state(pos.x, pos.y, pos.z))
}

fn can_portal_replace_block(chunks: &crate::chunk::ChunkStore, pos: BlockPos) -> SaveResult<bool> {
    let Some(state) = creation_state(chunks, pos)? else {
        return Ok(false);
    };
    let Some(facts) = vanilla_block_state_support_facts(state) else {
        if chunks.content_registry().block_by_state_id(state).is_some() {
            return Ok(false);
        }
        return Err(invalid_search(format!(
            "block state {state} has no support facts"
        )));
    };
    Ok(facts.replaceable() && block_state_fluid_state(state).is_empty())
}

fn can_host_frame(
    chunks: &crate::chunk::ChunkStore,
    origin: BlockPos,
    axis: PortalAxis,
    perpendicular: i32,
) -> SaveResult<bool> {
    for width in -1..3 {
        for height in -1..4 {
            let pos = portal_frame_pos(origin, axis, width, height, perpendicular);
            let Some(state) = creation_state(chunks, pos)? else {
                return Ok(false);
            };
            if height < 0 {
                if !bbb_server_types::fluid::block_state_is_solid_with_registry(
                    state,
                    chunks.content_registry(),
                ) {
                    return Ok(false);
                }
            } else if !can_portal_replace_block(chunks, pos)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn block_distance_squared(from: BlockPos, to: BlockPos) -> f64 {
    let dx = f64::from(from.x) - f64::from(to.x);
    let dy = f64::from(from.y) - f64::from(to.y);
    let dz = f64::from(from.z) - f64::from(to.z);
    dx * dx + dy * dy + dz * dz
}

fn push_portal_frame_writes(
    writes: &mut Vec<PortalBlockWrite>,
    origin: BlockPos,
    axis: PortalAxis,
    obsidian: i32,
    portal: i32,
) {
    for width in -1..3 {
        for height in -1..4 {
            if width == -1 || width == 2 || height == -1 || height == 3 {
                writes.push(PortalBlockWrite {
                    pos: portal_frame_pos(origin, axis, width, height, 0),
                    state: obsidian,
                    flags: SET_BLOCK_UPDATE_ALL,
                });
            }
        }
    }
    for width in 0..2 {
        for height in 0..3 {
            writes.push(PortalBlockWrite {
                pos: portal_frame_pos(origin, axis, width, height, 0),
                state: portal,
                flags: SET_BLOCK_UPDATE_CLIENTS | SET_BLOCK_UPDATE_KNOWN_SHAPE,
            });
        }
    }
}

fn portal_state_for_axis(axis: PortalAxis) -> SaveResult<i32> {
    let axis = match axis {
        PortalAxis::X => "x",
        PortalAxis::Z => "z",
    };
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:nether_portal",
        &BTreeMap::from([("axis".to_string(), axis.to_string())]),
    )
    .ok_or_else(|| invalid_search(format!("built-in nether portal axis {axis} is absent")))
}

fn validate_creation_writes(
    chunks: &crate::chunk::ChunkStore,
    writes: &[PortalBlockWrite],
) -> SaveResult<()> {
    for write in writes {
        if !is_in_world_bounds(chunks, write.pos)
            || !chunks.is_ticking_full(write.pos.x >> 4, write.pos.z >> 4)
        {
            return Err(invalid_search(format!(
                "portal write at ({},{},{}) is outside a FULL-loaded world cell",
                write.pos.x, write.pos.y, write.pos.z
            )));
        }
    }
    Ok(())
}

// `PortalForcer.createPortal` (`PortalForcer.java:52-182`).
fn prepare_creation(
    world: &ServerWorld,
    target: &LevelId,
    chunks: &crate::chunk::ChunkStore,
    origin: BlockPos,
    axis: PortalAxis,
) -> SaveResult<Option<PortalCreationPlan>> {
    let shape = dimension_shape(world, target)?;
    let min_y = shape.min_y;
    let max_placeable_y = min_y
        .checked_add(shape.height.min(shape.logical_height))
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| invalid_search("portal creation height overflows"))?;
    let level = world.level_by_id(target);
    let border = level.world_border;
    let game_time = world.game_time();
    let mut closest_full: Option<(f64, BlockPos)> = None;
    let mut closest_partial: Option<(f64, BlockPos)> = None;

    for column in creation_spiral(origin) {
        let height = chunks
            .final_heightmap_first_available(
                column.x >> 4,
                column.z >> 4,
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                column.x & 15,
                column.z & 15,
            )
            .ok_or_else(|| {
                invalid_search("FULL portal-search chunk has no motion-blocking heightmap")
            })?
            .min(max_placeable_y);
        if !border.contains_block_at(column, game_time)
            || !border.contains_block_at(offset(column, axis, 1, 0), game_time)
        {
            continue;
        }
        let mut y = height;
        while y >= min_y {
            let mut at = BlockPos { y, ..column };
            if can_portal_replace_block(chunks, at)? {
                let first_empty_y = y;
                while y > min_y {
                    let below = BlockPos { y: y - 1, ..at };
                    if !can_portal_replace_block(chunks, below)? {
                        break;
                    }
                    y -= 1;
                    at.y = y;
                }
                if y + 4 <= max_placeable_y
                    && (first_empty_y - y <= 0 || first_empty_y - y >= 3)
                    && can_host_frame(chunks, at, axis, 0)?
                {
                    let distance = block_distance_squared(origin, at);
                    if can_host_frame(chunks, at, axis, -1)?
                        && can_host_frame(chunks, at, axis, 1)?
                        && closest_full.is_none_or(|(best, _)| best > distance)
                    {
                        closest_full = Some((distance, at));
                    }
                    if closest_full.is_none()
                        && closest_partial.is_none_or(|(best, _)| best > distance)
                    {
                        closest_partial = Some((distance, at));
                    }
                }
            }
            if y == i32::MIN {
                break;
            }
            y -= 1;
        }
    }

    let mut writes = Vec::new();
    let base = if let Some((_, full)) = closest_full.or(closest_partial) {
        full
    } else {
        let min_start_y = min_y.saturating_add(1).max(70);
        let max_start_y = max_placeable_y.saturating_sub(9);
        if max_start_y < min_start_y {
            return Ok(None);
        }
        let unclamped = offset(
            BlockPos {
                y: origin.y.clamp(min_start_y, max_start_y),
                ..origin
            },
            axis,
            -1,
            0,
        );
        let base = clamp_to_border(
            border,
            game_time,
            Vec3d {
                x: f64::from(unclamped.x),
                y: f64::from(unclamped.y),
                z: f64::from(unclamped.z),
            },
        );
        let obsidian = default_block_state_id(VANILLA_BLOCK_OBSIDIAN_ID);
        let air = air_block_state_id();
        for perpendicular in -1..2 {
            for width in 0..2 {
                for height in -1..3 {
                    writes.push(PortalBlockWrite {
                        pos: portal_frame_pos(base, axis, width, height, perpendicular),
                        state: if height < 0 { obsidian } else { air },
                        flags: SET_BLOCK_UPDATE_ALL,
                    });
                }
            }
        }
        base
    };

    let obsidian = default_block_state_id(VANILLA_BLOCK_OBSIDIAN_ID);
    let portal = portal_state_for_axis(axis)?;
    push_portal_frame_writes(&mut writes, base, axis, obsidian, portal);
    validate_creation_writes(chunks, &writes)?;
    Ok(Some(PortalCreationPlan {
        target: *target,
        rectangle: PortalRectangle {
            min_corner: base,
            axis,
            width: 2,
            height: 3,
        },
        writes,
    }))
}

impl ServerWorld {
    /// `NetherPortalBlock.getPortalDestination` and
    /// `PortalForcer.findClosestPortalPosition`
    /// (`NetherPortalBlock.java:130-180`, `PortalForcer.java:41-50`).
    pub(crate) fn prepare_nether_portal_search(
        &mut self,
        source: &LevelId,
        entity_position: Vec3d,
        portal_entry: BlockPos,
    ) -> SaveResult<PreparedNetherPortalSearch> {
        if !self.has_level_id(source) {
            return Err(invalid_search(format!(
                "source level {} is not served",
                self.level_key(*source).resource_id()
            )));
        }
        let target = if source.builtin() == Some(Java26BuiltinDimension::Nether) {
            LevelId::from(Java26BuiltinDimension::Overworld)
        } else {
            LevelId::from(Java26BuiltinDimension::Nether)
        };
        if !self.has_level_id(&target) {
            return Err(invalid_search(format!(
                "target level {} is not served",
                self.level_key(target).resource_id()
            )));
        }
        let source_shape = dimension_shape(self, source)?;
        let target_shape = dimension_shape(self, &target)?;
        let scale = source_shape.coordinate_scale / target_shape.coordinate_scale;
        if !scale.is_finite() {
            return Err(invalid_search(
                "dimension teleportation scale is not finite",
            ));
        }
        let target_border = self.level_by_id(&target).world_border;
        let approximate_exit = clamp_to_border(
            target_border,
            self.game_time(),
            Vec3d {
                x: entity_position.x * scale,
                y: entity_position.y,
                z: entity_position.z * scale,
            },
        );
        let to_nether = target.builtin() == Some(Java26BuiltinDimension::Nether);
        let radius = if to_nether {
            TO_NETHER_RADIUS
        } else {
            TO_OVERWORLD_RADIUS
        };
        let game_time = self.game_time();
        let poi_candidates = portal_poi_candidates(
            &mut self.level_mut_by_id(&target).chunks,
            target_border,
            game_time,
            approximate_exit,
            radius,
        );
        let mut loaded_chunks = BTreeSet::new();
        let mut candidates = None;
        for pos in poi_candidates {
            let chunk = (pos.x >> 4, pos.z >> 4);
            if loaded_chunks.insert(chunk) {
                self.generate_original_mob_columns(&target, &[chunk])?;
            }
            if let Some(state) = self
                .level_by_id(&target)
                .chunks
                .get_block_state(pos.x, pos.y, pos.z)
            {
                if portal_axis(state).is_some() {
                    candidates = Some((pos, state));
                    break;
                }
            }
        }
        let rectangle_window = rectangle_chunks(candidates.as_slice())?;
        if !rectangle_window.is_empty() {
            self.generate_original_mob_columns(&target, &rectangle_window)?;
        }
        let (found_position, rectangle) =
            found_portal_rectangle(&self.level_by_id(&target).chunks, candidates);
        let full_chunks = loaded_chunks
            .into_iter()
            .chain(rectangle_window)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(PreparedNetherPortalSearch {
            target,
            approximate_exit,
            source_axis: source_portal_axis(self, source, portal_entry),
            found_position,
            rectangle,
            full_chunks,
        })
    }

    fn stored_portal_traveller(
        &self,
        source: &LevelId,
        entity_id: i32,
    ) -> SaveResult<PortalTraveller> {
        let entity = self
            .level_by_id(source)
            .entities
            .get(entity_id)
            .ok_or_else(|| invalid_search(format!("stored entity {entity_id} is absent")))?;
        Ok(PortalTraveller::of_entity(entity))
    }

    fn nether_portal_transition_in_chunks(
        &self,
        source: &LevelId,
        traveller: PortalTraveller,
        portal_entry: BlockPos,
        target_chunks: &crate::chunk::ChunkStore,
        target_border: WorldBorder,
        exit: PortalRectangle,
    ) -> SaveResult<StoredEntityTransition> {
        nether_portal_transition(
            &self.level_by_id(source).chunks,
            target_chunks,
            target_border,
            self.game_time(),
            traveller,
            portal_entry,
            exit,
        )
    }

    fn prepare_nether_portal_creation(
        &mut self,
        search: &mut PreparedNetherPortalSearch,
    ) -> SaveResult<Option<PreparedNetherPortalCreation>> {
        if search.rectangle.is_some() {
            return Ok(None);
        }
        if !self.has_level_id(&search.target) {
            return Err(invalid_search(format!(
                "target level {} is not served",
                self.level_key(search.target).resource_id()
            )));
        }
        let mut window = creation_window(search.approximate_exit)?;
        let target_border = self.level_by_id(&search.target).world_border;
        let unclamped = offset(search.approximate_exit, search.source_axis, -1, 0);
        let platform_base = clamp_to_border(
            target_border,
            self.game_time(),
            Vec3d {
                x: f64::from(unclamped.x),
                y: f64::from(unclamped.y),
                z: f64::from(unclamped.z),
            },
        );
        for dx in -2..=2 {
            for dz in -2..=2 {
                let chunk = ((platform_base.x + dx) >> 4, (platform_base.z + dz) >> 4);
                if !window.contains(&chunk) {
                    window.push(chunk);
                }
            }
        }
        self.generate_original_mob_columns(&search.target, &window)?;
        let full_chunks = search
            .full_chunks
            .iter()
            .copied()
            .chain(window)
            .collect::<BTreeSet<_>>();
        search.full_chunks = full_chunks.into_iter().collect();
        let Some(plan) = prepare_creation(
            self,
            &search.target,
            &self.level_by_id(&search.target).chunks,
            search.approximate_exit,
            search.source_axis,
        )?
        else {
            return Ok(None);
        };

        self.with_level(plan.target, |level| {
            for write in &plan.writes {
                let _ = level.set_block(write.pos, write.state, write.flags, super::UPDATE_LIMIT);
            }
        });
        self.level_mut_by_id(&plan.target)
            .chunks
            .flush_live_relights()?;
        #[cfg(test)]
        let write_trace = plan
            .writes
            .iter()
            .map(|write| (write.pos, write.state, write.flags))
            .collect();
        Ok(Some(PreparedNetherPortalCreation {
            rectangle: plan.rectangle,
            #[cfg(test)]
            write_trace,
        }))
    }

    /// `NetherPortalBlock.getPortalDestination` for any traveller: finds or
    /// builds the exit, then answers the transition and the prepared portal
    /// ticket. `None` is vanilla's `null` destination or a refused Nether entry.
    pub(super) fn resolve_nether_portal_destination(
        &mut self,
        source: &LevelId,
        traveller: PortalTraveller,
        portal_entry: BlockPos,
    ) -> SaveResult<Option<NetherPortalDestination>> {
        let mut search =
            self.prepare_nether_portal_search(source, traveller.position, portal_entry)?;
        let creation = self.prepare_nether_portal_creation(&mut search)?;
        let Some(exit) = search
            .rectangle
            .or_else(|| creation.as_ref().map(|prepared| prepared.rectangle))
        else {
            return Ok(None);
        };
        // `ServerLevel.isAllowedToEnterPortal` (`ServerLevel.java:1871-1873`)
        // is asked only after `getPortalDestination` built any missing exit
        // (`Entity.java:2548-2551`), so a refused traveller still leaves the
        // portal behind, without the ticket its post-teleport would place.
        if !self.portal_destination_allowed(&search.target) {
            return Ok(None);
        }
        let exit_margin = exit_margin_chunks(exit)?;
        self.generate_original_mob_columns(&search.target, &exit_margin)?;
        let full_chunks = search
            .full_chunks
            .iter()
            .copied()
            .chain(exit_margin)
            .collect::<BTreeSet<_>>();
        search.full_chunks = full_chunks.into_iter().collect();
        let target_level = self.level_by_id(&search.target);
        let transition = self.nether_portal_transition_in_chunks(
            source,
            traveller,
            portal_entry,
            &target_level.chunks,
            target_level.world_border,
            exit,
        )?;
        let ticket_position = search
            .found_position
            .unwrap_or_else(|| block_position(transition.position));
        let ticket = self.prepare_portal_ticket_by_key(
            &search.target,
            ticket_position,
            &search.full_chunks,
        )?;
        Ok(Some(NetherPortalDestination {
            target: search.target,
            transition,
            ticket,
        }))
    }

    fn transfer_stored_entity_through_nether_portal(
        &mut self,
        source: &LevelId,
        entity_id: i32,
        portal_entry: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> SaveResult<bool> {
        let traveller = self.stored_portal_traveller(source, entity_id)?;
        let Some(destination) =
            self.resolve_nether_portal_destination(source, traveller, portal_entry)?
        else {
            return Ok(false);
        };
        let ticket = self.publish_portal_destination(destination.ticket);
        if self
            .teleport_entity_subtree(
                *source,
                destination.target,
                entity_id,
                destination.transition,
                true,
                events,
            )
            .map_err(|error| invalid_search(format!("portal transfer failed: {error}")))?
            .is_none()
        {
            return Ok(false);
        }
        self.commit_portal_ticket_state(ticket);
        Ok(true)
    }

    // ServerLevel.java:1871-1873, including same-level Nether gateways.
    pub(super) fn portal_destination_allowed(&self, target: &LevelId) -> bool {
        target.builtin() != Some(Java26BuiltinDimension::Nether)
            || self.game_rule_bool(RULE_ALLOW_ENTERING_NETHER_USING_PORTALS, true)
    }

    pub(super) fn apply_stored_nether_portal_request(
        &mut self,
        source: &LevelId,
        entity_id: i32,
        request: crate::entity::portal::PortalTransitionRequest,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        match request.kind {
            PortalKind::Nether => self
                .transfer_stored_entity_through_nether_portal(
                    source,
                    entity_id,
                    request.entry,
                    events,
                )
                .unwrap_or(false),
            PortalKind::End => self
                .transfer_stored_entity_through_end_portal(source, entity_id, events)
                .unwrap_or(false),
            PortalKind::EndGateway => self
                .transfer_stored_entity_through_gateway(source, entity_id, request.entry, events)
                .unwrap_or(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn prepare_nether_portal_creation_for_test(
        &mut self,
        search: &mut PreparedNetherPortalSearch,
    ) -> SaveResult<Option<PreparedNetherPortalCreation>> {
        self.prepare_nether_portal_creation(search)
    }

    #[cfg(test)]
    pub(crate) fn stored_nether_portal_transition_for_test(
        &self,
        source: &LevelId,
        entity_id: i32,
        portal_entry: BlockPos,
        target: &LevelId,
        exit: PortalRectangle,
    ) -> SaveResult<StoredEntityTransition> {
        let target_level = self.level_by_id(target);
        self.nether_portal_transition_in_chunks(
            source,
            self.stored_portal_traveller(source, entity_id)?,
            portal_entry,
            &target_level.chunks,
            target_level.world_border,
            exit,
        )
    }
}
