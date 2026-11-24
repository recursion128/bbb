//! Flowing fluid spread, block conversion and live shape caches.

use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::simulation_level::LevelCtx;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_static_collision_shape, VanillaBlockCollisionBox,
    VanillaBlockCollisionShape,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaLiquidBlockContainerOverride, VANILLA_BLOCK_BASALT_ID,
    VANILLA_BLOCK_BLUE_ICE_ID, VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_COBBLESTONE_ID,
    VANILLA_BLOCK_END_GATEWAY_ID, VANILLA_BLOCK_END_PORTAL_ID, VANILLA_BLOCK_LADDER_ID,
    VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_NETHER_PORTAL_ID, VANILLA_BLOCK_OBSIDIAN_ID,
    VANILLA_BLOCK_SOUL_SOIL_ID, VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_STRUCTURE_VOID_ID,
    VANILLA_BLOCK_SUGAR_CANE_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

use crate::level_event::LEVEL_EVENT_LAVA_FIZZ;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{
    air_block_state_id, block_state_blocks_motion, block_state_fluid_state, block_state_is_air,
    block_state_is_solid, FluidFamily, FluidState, AMOUNT_FULL,
};

#[cfg(test)]
mod tests;

mod bucket;
pub(crate) use bucket::{empty_bucket, BucketEmptied, BucketUser};
mod shapes;
pub use shapes::SharedFluidShapeCache;

/// `Block.UPDATE_ALL` (`net/minecraft/world/level/block/Block.java:104`) =
/// `UPDATE_NEIGHBORS | UPDATE_CLIENTS` (`:91`, `:92`). Every `setBlock` the
/// fluid mechanism performs uses this flag set
/// (`FlowingFluid.java:276`, `:450`, `:454`; `LavaFluid.java:209`;
/// `SimpleWaterloggedBlock.java:28`), and `setBlockAndUpdate` is the same
/// thing spelled shorter (`LiquidBlock.java:224`, `:230`).
pub const SET_BLOCK_UPDATE_ALL: i32 = 3;

/// `LavaFluid.MIN_LEVEL_CUTOFF` (`LavaFluid.java:34`): the height a lava state
/// must reach before water is allowed to replace it
/// (`LavaFluid.java:170-171`).
pub const LAVA_MIN_LEVEL_CUTOFF: f32 = 0.444_444_45;

/// `Direction.Plane.HORIZONTAL` in its declared order (`Direction.java:577`).
pub const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// Horizontal spread order (`Direction.NORTH`, `SOUTH`, `WEST`, `EAST`),
/// matching Vanilla's `EnumMap<Direction, FluidState>` iteration order
/// (`Direction.java:33-38`, `FlowingFluid.java`).
const HORIZONTAL_SPREAD_ORDER: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

// ---------------------------------------------------------------------------
// Fluid identity and state
// ---------------------------------------------------------------------------

fn can_be_replaced_with(
    state: FluidState,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    other: Option<FluidFamily>,
    direction: Direction,
) -> bool {
    match state.family() {
        None => true,
        Some(FluidFamily::Water) => {
            direction == Direction::Down && other != Some(FluidFamily::Water)
        }
        Some(FluidFamily::Lava) => {
            fluid_height(state, level, pos) >= LAVA_MIN_LEVEL_CUTOFF
                && other == Some(FluidFamily::Water)
        }
    }
}

/// `FlowingFluid.getHeight` (`FlowingFluid.java`) with `EmptyFluid`'s
/// constant zero (`EmptyFluid.java`).
pub(crate) fn fluid_height(state: FluidState, level: &LevelCtx<'_, '_>, pos: BlockPos) -> f32 {
    let Some(sample) = state.to_facts_sample() else {
        return 0.0;
    };
    let same_above = level.fluid_state(above(pos)).family() == state.family();
    bbb_facts::fluid::fluid_height_f32(Some(sample), same_above)
}

// FlowingFluid.java; EmptyFluid.java.
pub fn checked_fluid_shape(
    state: FluidState,
    pos: BlockPos,
    block_state_at: impl FnMut(BlockPos) -> Option<i32>,
) -> Option<Option<bbb_server_types::geometry::Aabb>> {
    checked_fluid_shape_with_registry(
        state,
        pos,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
        block_state_at,
    )
}

pub fn checked_fluid_shape_with_registry(
    state: FluidState,
    pos: BlockPos,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    mut block_state_at: impl FnMut(BlockPos) -> Option<i32>,
) -> Option<Option<bbb_server_types::geometry::Aabb>> {
    let sample = state.to_facts_sample();
    let height = bbb_facts::fluid::checked_fluid_height_f32(sample, {
        if sample.is_none() {
            None
        } else {
            let above_block = block_state_at(above(pos))?;
            if vanilla_block_state_block_type_id(above_block).is_none() {
                content_registry.block_by_state_id(above_block)?;
            }
            Some(
                block_state_fluid_state(above_block)
                    .family()
                    .map(bbb_server_types::fluid::FluidFamily::to_facts_kind),
            )
        }
    })?;
    if height <= 0.0 {
        return Some(None);
    }
    Some(Some(bbb_server_types::geometry::Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: f64::from(height),
        max_z: 1.0,
    }))
}

// ---------------------------------------------------------------------------
// Position helpers (BlockPos in this repo is a bare coordinate triple)
// ---------------------------------------------------------------------------

pub fn above(pos: BlockPos) -> BlockPos {
    relative(pos, Direction::Up)
}

pub fn below(pos: BlockPos) -> BlockPos {
    relative(pos, Direction::Down)
}

// ---------------------------------------------------------------------------
// Block-state facts derived from the generated Vanilla 26.1 registry
// ---------------------------------------------------------------------------

/// `state.getBlock() instanceof LiquidBlockContainer`, from the generated
/// class fact `BlockTypeCatalogEntry::liquid_block_container_override`.
pub(crate) fn liquid_block_container(
    block_state_id: i32,
) -> Option<VanillaLiquidBlockContainerOverride> {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.liquid_block_container_override)
}

/// `FlowingFluid.canHoldAnyFluid` (`FlowingFluid.java`).
///
/// The two class/tag tests vanilla writes as `instanceof DoorBlock` and
/// `state.is(BlockTags.SIGNS)` are answered off the generated 26.1 tag
/// snapshot: `#minecraft:doors` holds exactly the `DoorBlock` instances (the
/// twelve wooden doors, the eight copper doors and the iron door) and
/// `#minecraft:signs` is the tag vanilla itself queries.
fn can_hold_any_fluid(tags: &RegistryTagSnapshot, block_state_id: i32) -> bool {
    if liquid_block_container(block_state_id).is_some() {
        return true;
    }
    if block_state_blocks_motion(block_state_id) {
        return false;
    }
    let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
        return false;
    };
    !tags.contains_block("minecraft:doors", block_type_id)
        && !tags.contains_block("minecraft:signs", block_type_id)
        && block_type_id != VANILLA_BLOCK_LADDER_ID
        && block_type_id != VANILLA_BLOCK_SUGAR_CANE_ID
        && block_type_id != VANILLA_BLOCK_BUBBLE_COLUMN_ID
        && block_type_id != VANILLA_BLOCK_NETHER_PORTAL_ID
        && block_type_id != VANILLA_BLOCK_END_PORTAL_ID
        && block_type_id != VANILLA_BLOCK_END_GATEWAY_ID
        && block_type_id != VANILLA_BLOCK_STRUCTURE_VOID_ID
}

/// `FlowingFluid.canHoldSpecificFluid` (`FlowingFluid.java`), i.e.
/// `LiquidBlockContainer.canPlaceLiquid(null, ...)` for the container blocks
/// and an unconditional yes for everything else.
///
/// The three container behaviours, all with vanilla's `user == null`:
///
/// * `SimpleWaterloggedBlock.canPlaceLiquid` (`SimpleWaterloggedBlock.java`)
///   accepts `Fluids.WATER` — the *source* singleton, so a flowing state is
///   refused.
/// * `SlabBlock` (`SlabBlock.java`) adds "not a double slab".
/// * `BarrierBlock` (`BarrierBlock.java`) needs a creative player, and
///   the fluid always passes `null`, so it is always no.
/// * kelp, kelp plant, seagrass and tall seagrass return a flat false
///   (`KelpBlock.java`, `KelpPlantBlock.java`,
///   `SeagrassBlock.java`, `TallSeagrassBlock.java`).
fn can_hold_specific_fluid(block_state_id: i32, fluid: VanillaFluidIdentity) -> bool {
    match liquid_block_container(block_state_id) {
        None => true,
        Some(VanillaLiquidBlockContainerOverride::SimpleWaterloggedBlock) => {
            fluid == VanillaFluidIdentity::Water
        }
        Some(VanillaLiquidBlockContainerOverride::SlabBlock) => {
            fluid == VanillaFluidIdentity::Water && !is_double_slab(block_state_id)
        }
        Some(
            VanillaLiquidBlockContainerOverride::BarrierBlock
            | VanillaLiquidBlockContainerOverride::KelpBlock
            | VanillaLiquidBlockContainerOverride::KelpPlantBlock
            | VanillaLiquidBlockContainerOverride::SeagrassBlock
            | VanillaLiquidBlockContainerOverride::TallSeagrassBlock,
        ) => false,
    }
}

fn is_double_slab(block_state_id: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "type")
        == Some("double")
}

/// `FlowingFluid.canHoldFluid` (`FlowingFluid.java`).
fn can_hold_fluid(
    tags: &RegistryTagSnapshot,
    block_state_id: i32,
    fluid: VanillaFluidIdentity,
) -> bool {
    can_hold_any_fluid(tags, block_state_id) && can_hold_specific_fluid(block_state_id, fluid)
}

/// `LiquidBlockContainer.placeLiquid` for the container blocks
/// (`SimpleWaterloggedBlock.java`, gated by `SlabBlock.java`).
///
/// Returns vanilla's boolean; the fluid ignores it
/// (`FlowingFluid.java`), but the block-side callers do not.
fn place_liquid(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    target: FluidState,
) -> bool {
    match liquid_block_container(block_state_id) {
        // `BarrierBlock` only gates `canPlaceLiquid`; its `placeLiquid` is the
        // waterlogged default.
        Some(
            VanillaLiquidBlockContainerOverride::SimpleWaterloggedBlock
            | VanillaLiquidBlockContainerOverride::BarrierBlock,
        ) => {}
        Some(VanillaLiquidBlockContainerOverride::SlabBlock) if !is_double_slab(block_state_id) => {
        }
        _ => return false,
    }
    if target.identity() != VanillaFluidIdentity::Water {
        return false;
    }
    let Some(waterlogged) = with_property(block_state_id, "waterlogged", "true") else {
        return false;
    };
    if waterlogged == block_state_id {
        // Already `waterlogged=true`, which is vanilla's first guard.
        return false;
    }
    use crate::block_behavior::{campfire, candle, dried_ghast};
    let delay = FlowingFluid::of(FluidFamily::Water).tick_delay(level.fluid_fast_lava());
    if candle::is_candle(block_state_id) && candle::is_lit(block_state_id) {
        // CandleBlock.java:150-164.
        candle::extinguish(level, pos, waterlogged, None);
    } else if campfire::is_campfire(block_state_id) && campfire::is_lit(block_state_id) {
        // CampfireBlock.java:203-220.
        level.play_block_sound(pos, "minecraft:entity.generic.extinguish_fire");
        campfire::dowse(level, pos, None);
        let unlit = with_property(waterlogged, "lit", "false").unwrap_or(waterlogged);
        level.set_block(pos, unlit, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
    } else {
        level.set_block(pos, waterlogged, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
    }
    // `SimpleWaterloggedBlock.java`: the placed source arms its own tick,
    // with the *fluid's* delay, not the block's.
    level.schedule_fluid_tick(pos, target.tick_type(), delay, TickPriority::Normal);
    if dried_ghast::is_dried_ghast(block_state_id) {
        // DriedGhastBlock.java:181-193.
        level.play_block_sound(pos, "minecraft:block.dried_ghast.place_in_water");
    }
    true
}

/// `BlockState.setValue(property, value)` over the generated registry.
pub fn with_property(block_state_id: i32, property: &str, value: &str) -> Option<i32> {
    bbb_protocol::block_state_types::vanilla_block_state_with_property(
        block_state_id,
        property,
        value,
    )
}

// ---------------------------------------------------------------------------
// canPassThroughWall and the voxel-face merge behind it
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

/// `Direction.getAxis` / `Direction.getAxisDirection`
/// (`Direction.java`, third and fifth constructor arguments).
const fn direction_axis(direction: Direction) -> (Axis, bool) {
    match direction {
        Direction::Down => (Axis::Y, false),
        Direction::Up => (Axis::Y, true),
        Direction::North => (Axis::Z, false),
        Direction::South => (Axis::Z, true),
        Direction::West => (Axis::X, false),
        Direction::East => (Axis::X, true),
    }
}

enum FluidCollisionShape {
    Static(VanillaBlockCollisionShape),
    Dynamic(Vec<bbb_server_types::geometry::Aabb>),
}

/// `BlockState.getCollisionShape(level, pos)` (`CollisionContext.empty()`,
/// `FlowingFluid.java:207-212`).
#[cfg(test)]
fn collision_shape(
    level: Option<&LevelCtx<'_, '_>>,
    pos: BlockPos,
    block_state_id: i32,
) -> FluidCollisionShape {
    if let Some(shape) = vanilla_block_state_static_collision_shape(block_state_id) {
        return FluidCollisionShape::Static(shape);
    }
    FluidCollisionShape::Dynamic(dynamic_collision_boxes(level, pos, block_state_id))
}

fn dynamic_collision_boxes(
    level: Option<&LevelCtx<'_, '_>>,
    pos: BlockPos,
    block_state_id: i32,
) -> Vec<bbb_server_types::geometry::Aabb> {
    // MovingPistonBlock.java:106-113: collision comes from PistonMovingBlockEntity when present.
    if vanilla_block_state_block_type_id(block_state_id)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
    {
        let Some(chunks) = level.map(LevelCtx::chunks) else {
            return Vec::new();
        };
        return chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::piston_snapshot)
            .and_then(|snapshot| {
                let boxes = snapshot.collision_boxes_with_resolver(None, |moved_state| {
                    empty_context_boxes_with_chunks(Some(chunks), pos, moved_state)
                })?;
                Some(
                    bbb_server_types::geometry::BoxShape::from_boxes(&boxes)?
                        .optimized()?
                        .boxes(),
                )
            })
            .unwrap_or_default();
    }
    empty_context_boxes_with_chunks(level.map(LevelCtx::chunks), pos, block_state_id)
        .unwrap_or_default()
}

fn empty_context_boxes_with_chunks(
    chunks: Option<&crate::chunk::ChunkStore>,
    pos: BlockPos,
    state: i32,
) -> Option<Vec<bbb_server_types::geometry::Aabb>> {
    // ShulkerBoxBlock.java:152-156.
    if bbb_server_types::block_queries::is_shulker_box(state) {
        if let Some(shulker) = chunks
            .and_then(|chunks| chunks.block_entities().get(pos))
            .and_then(crate::block_entity::BlockEntity::shulker_box)
        {
            let bounds = shulker.collision_box(crate::block_behavior::shulker_box::facing(state));
            return Some(bbb_server_types::geometry::BoxShape::from_box(bounds)?.boxes());
        }
    }
    bbb_server_types::geometry::empty_context_block_collision_boxes(state, pos)
}

fn shape_is_empty(shape: &FluidCollisionShape) -> bool {
    match shape {
        FluidCollisionShape::Static(shape) => shape.boxes().len() == 0,
        FluidCollisionShape::Dynamic(boxes) => boxes.is_empty(),
    }
}

/// Whether the shape is the interned unit cube `Shapes.block()` (`FlowingFluid.java:208, 213`).
fn shape_is_full_cube(shape: &FluidCollisionShape) -> bool {
    match shape {
        FluidCollisionShape::Static(shape) => {
            let mut boxes = shape.boxes();
            if boxes.len() != 1 {
                return false;
            }
            let Some(unit) = boxes.next() else {
                return false;
            };
            unit.min_x <= 0.0
                && unit.min_y <= 0.0
                && unit.min_z <= 0.0
                && unit.max_x >= 1.0
                && unit.max_y >= 1.0
                && unit.max_z >= 1.0
        }
        FluidCollisionShape::Dynamic(boxes) => boxes.len() == 1 && boxes[0].is_unit_cube(),
    }
}

fn aabb_to_collision_box(unit: bbb_server_types::geometry::Aabb) -> VanillaBlockCollisionBox {
    VanillaBlockCollisionBox {
        min_x: unit.min_x,
        min_y: unit.min_y,
        min_z: unit.min_z,
        max_x: unit.max_x,
        max_y: unit.max_y,
        max_z: unit.max_z,
    }
}

fn box_min(unit: VanillaBlockCollisionBox, axis: Axis) -> f64 {
    match axis {
        Axis::X => unit.min_x,
        Axis::Y => unit.min_y,
        Axis::Z => unit.min_z,
    }
}

fn box_max(unit: VanillaBlockCollisionBox, axis: Axis) -> f64 {
    match axis {
        Axis::X => unit.max_x,
        Axis::Y => unit.max_y,
        Axis::Z => unit.max_z,
    }
}

/// A rectangle in the plane perpendicular to an axis, clipped to the unit
/// square: `(min_u, min_v, max_u, max_v)`.
type FaceRect = (f64, f64, f64, f64);

/// `DoubleMath.fuzzyEquals(value, target, 1.0E-7)` as `Shapes.mergedFaceOccludes`
/// uses it (`Shapes.java`, `:272`).
fn fuzzy_equals(value: f64, target: f64) -> bool {
    (value - target).abs() <= 1.0e-7
}

fn box_face_rect(unit: VanillaBlockCollisionBox, axis: Axis) -> FaceRect {
    let (min_u, max_u, min_v, max_v) = match axis {
        Axis::X => (unit.min_y, unit.max_y, unit.min_z, unit.max_z),
        Axis::Y => (unit.min_x, unit.max_x, unit.min_z, unit.max_z),
        Axis::Z => (unit.min_x, unit.max_x, unit.min_y, unit.max_y),
    };
    (
        min_u.max(0.0),
        min_v.max(0.0),
        max_u.min(1.0),
        max_v.min(1.0),
    )
}

/// The 2-D footprint of a shape's outermost slice along `axis`.
///
/// `Shapes.mergedFaceOccludes` empties the shape unless it reaches the
/// boundary (`Shapes.java`), then slices it at the last (or first)
/// discrete index (`:277`). A discrete slice's breakpoints are exactly the
/// boxes' own coordinates, so the outermost slice is covered by exactly the
/// boxes that touch the boundary: a box with `max(axis) == 1` spans the whole
/// last slice, and one with `max(axis) < 1` cannot reach it at all.
fn push_face_rects_iter<I>(
    make_boxes: impl Fn() -> I,
    axis: Axis,
    at_max: bool,
    out: &mut Vec<FaceRect>,
) where
    I: Iterator<Item = VanillaBlockCollisionBox>,
{
    let mut boundary_reached = if at_max {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut any = false;
    for unit in make_boxes() {
        any = true;
        boundary_reached = if at_max {
            boundary_reached.max(box_max(unit, axis))
        } else {
            boundary_reached.min(box_min(unit, axis))
        };
    }
    if !any || !fuzzy_equals(boundary_reached, if at_max { 1.0 } else { 0.0 }) {
        return;
    }
    for unit in make_boxes() {
        let on_boundary = if at_max {
            fuzzy_equals(box_max(unit, axis), 1.0)
        } else {
            fuzzy_equals(box_min(unit, axis), 0.0)
        };
        if on_boundary {
            out.push(box_face_rect(unit, axis));
        }
    }
}

fn push_face_rects(shape: &FluidCollisionShape, axis: Axis, at_max: bool, out: &mut Vec<FaceRect>) {
    match shape {
        FluidCollisionShape::Static(shape) => {
            push_face_rects_iter(|| shape.boxes(), axis, at_max, out)
        }
        FluidCollisionShape::Dynamic(boxes) => push_face_rects_iter(
            || boxes.iter().copied().map(aabb_to_collision_box),
            axis,
            at_max,
            out,
        ),
    }
}

#[cfg(test)]
fn face_rects(shape: &FluidCollisionShape, axis: Axis, at_max: bool) -> Vec<FaceRect> {
    let mut rects = Vec::new();
    push_face_rects(shape, axis, at_max, &mut rects);
    rects
}

/// Whether the rectangles cover the whole unit square.
///
/// This is `!Shapes.joinIsNotEmpty(block(), union, ONLY_FIRST)`
/// (`Shapes.java`) specialised to one slice: is there any part of the
/// unit face the union misses? Coordinate compression makes it exact.
fn covers_unit_square(rects: &[FaceRect]) -> bool {
    if rects.is_empty() {
        return false;
    }
    // Fast path: a single rectangle covers the whole unit square.
    if rects.iter().any(|&(min_u, min_v, max_u, max_v)| {
        min_u <= 1.0e-7 && min_v <= 1.0e-7 && max_u >= 1.0 - 1.0e-7 && max_v >= 1.0 - 1.0e-7
    }) {
        return true;
    }
    if rects.len() <= 15 {
        let mut us = [0.0_f64; 32];
        let mut vs = [0.0_f64; 32];
        us[0] = 0.0;
        us[1] = 1.0;
        vs[0] = 0.0;
        vs[1] = 1.0;
        let mut len = 2;
        for &(min_u, min_v, max_u, max_v) in rects {
            us[len] = min_u;
            us[len + 1] = max_u;
            vs[len] = min_v;
            vs[len + 1] = max_v;
            len += 2;
        }
        let u_len = sort_dedup_slice(&mut us[..len]);
        let v_len = sort_dedup_slice(&mut vs[..len]);
        return covers_unit_square_grid(rects, &us[..u_len], &vs[..v_len]);
    }
    let mut us = Vec::with_capacity(2 + rects.len() * 2);
    let mut vs = Vec::with_capacity(2 + rects.len() * 2);
    us.extend_from_slice(&[0.0, 1.0]);
    vs.extend_from_slice(&[0.0, 1.0]);
    for &(min_u, min_v, max_u, max_v) in rects {
        us.push(min_u);
        us.push(max_u);
        vs.push(min_v);
        vs.push(max_v);
    }
    let u_len = sort_dedup_slice(&mut us);
    let v_len = sort_dedup_slice(&mut vs);
    covers_unit_square_grid(rects, &us[..u_len], &vs[..v_len])
}

fn covers_unit_square_grid(rects: &[FaceRect], us: &[f64], vs: &[f64]) -> bool {
    for pair_u in us.windows(2) {
        if pair_u[1] <= pair_u[0] || pair_u[0] >= 1.0 || pair_u[1] <= 0.0 {
            continue;
        }
        let mid_u = f64::midpoint(pair_u[0], pair_u[1]);
        for pair_v in vs.windows(2) {
            if pair_v[1] <= pair_v[0] || pair_v[0] >= 1.0 || pair_v[1] <= 0.0 {
                continue;
            }
            let mid_v = f64::midpoint(pair_v[0], pair_v[1]);
            let covered = rects.iter().any(|&(min_u, min_v, max_u, max_v)| {
                min_u < mid_u && mid_u < max_u && min_v < mid_v && mid_v < max_v
            });
            if !covered {
                return false;
            }
        }
    }
    true
}

fn sort_dedup_slice(values: &mut [f64]) -> usize {
    values.sort_by(|a, b| a.partial_cmp(b).expect("collision bounds are never NaN"));
    let mut write = 0;
    for read in 0..values.len() {
        if write == 0 || !fuzzy_equals(values[read], values[write - 1]) {
            values[write] = values[read];
            write += 1;
        }
    }
    write
}

/// `Shapes.mergedFaceOccludes(shape, occluder, direction)`
/// (`Shapes.java`).
fn merged_face_occludes(
    source_shape: &FluidCollisionShape,
    target_shape: &FluidCollisionShape,
    direction: Direction,
) -> bool {
    let (axis, positive) = direction_axis(direction);
    let (first, second) = if positive {
        (source_shape, target_shape)
    } else {
        (target_shape, source_shape)
    };
    let mut rects = Vec::with_capacity(8);
    push_face_rects(first, axis, true, &mut rects);
    push_face_rects(second, axis, false, &mut rects);
    covers_unit_square(&rects)
}

/// Thread-local direct-mapped static `(source_state, target_state, direction)`
/// wall-occlusion cache (`FlowingFluid.OCCLUSION_CACHE`, `FlowingFluid.java:38-46`).
/// Only pairs where both block states have a static collision shape are stored.
const WALL_CACHE_LEN: usize = 512;

thread_local! {
    static WALL_OCCLUSION_CACHE: RefCell<[(u64, bool); WALL_CACHE_LEN]> =
        const { RefCell::new([(0, false); WALL_CACHE_LEN]) };
}

fn static_wall_cache_key(
    source_state: i32,
    target_state: i32,
    direction: Direction,
) -> Option<u64> {
    if (0..(1 << 24)).contains(&source_state) && (0..(1 << 24)).contains(&target_state) {
        Some(
            1_u64
                + ((source_state as u64)
                    | ((target_state as u64) << 24)
                    | (u64::from(direction.id()) << 48)),
        )
    } else {
        None
    }
}

fn static_wall_cache_slot(key: u64) -> usize {
    (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 55) as usize
}

/// `FlowingFluid.canPassThroughWall` (`FlowingFluid.java:207-248`).
///
/// Dynamic-shape blocks (`hasDynamicShape()`, `FlowingFluid.java:222`) resolve
/// per `(level, pos)` rather than from the static cache.
fn can_pass_through_wall_at(
    level: Option<&LevelCtx<'_, '_>>,
    direction: Direction,
    source_pos: BlockPos,
    source_state: i32,
    target_pos: BlockPos,
    target_state: i32,
) -> bool {
    let cache_key = static_wall_cache_key(source_state, target_state, direction);
    if let Some(key) = cache_key {
        let slot = static_wall_cache_slot(key);
        let cached = WALL_OCCLUSION_CACHE.with(|cache| {
            let entry = cache.borrow()[slot];
            (entry.0 == key).then_some(entry.1)
        });
        if let Some(passable) = cached {
            return passable;
        }
    }
    let static_target = vanilla_block_state_static_collision_shape(target_state);
    let target_shape = match static_target {
        Some(shape) => FluidCollisionShape::Static(shape),
        None => {
            FluidCollisionShape::Dynamic(dynamic_collision_boxes(level, target_pos, target_state))
        }
    };
    let static_source = vanilla_block_state_static_collision_shape(source_state);
    let passable = if shape_is_full_cube(&target_shape) {
        false
    } else {
        let source_shape = match static_source {
            Some(shape) => FluidCollisionShape::Static(shape),
            None => FluidCollisionShape::Dynamic(dynamic_collision_boxes(
                level,
                source_pos,
                source_state,
            )),
        };
        if shape_is_full_cube(&source_shape) {
            false
        } else if shape_is_empty(&source_shape) && shape_is_empty(&target_shape) {
            true
        } else {
            !merged_face_occludes(&source_shape, &target_shape, direction)
        }
    };
    if let (Some(key), Some(_), Some(_)) = (cache_key, static_source, static_target) {
        let slot = static_wall_cache_slot(key);
        WALL_OCCLUSION_CACHE.with(|cache| {
            cache.borrow_mut()[slot] = (key, passable);
        });
    }
    passable
}

#[cfg(test)]
fn can_pass_through_wall(direction: Direction, source_state: i32, target_state: i32) -> bool {
    let origin = BlockPos { x: 0, y: 0, z: 0 };
    can_pass_through_wall_at(
        None,
        direction,
        origin,
        source_state,
        relative(origin, direction),
        target_state,
    )
}

// ---------------------------------------------------------------------------
// SpreadContext
// ---------------------------------------------------------------------------

// `slope_find_distance <= 4`, so `get_spread` + `get_slope_distance` reach at
// most 5 blocks horizontally from `origin` (`[-5..=5]` -> 11x11 = 121 cells).
const SPREAD_GRID_RADIUS: i32 = 5;
const SPREAD_GRID_SIDE: usize = (SPREAD_GRID_RADIUS as usize) * 2 + 1;
const SPREAD_GRID_LEN: usize = SPREAD_GRID_SIDE * SPREAD_GRID_SIDE;

// Slope searches stay at the origin's height, so horizontal offsets identify cached positions.
struct SpreadContext {
    origin: BlockPos,
    states: [i32; SPREAD_GRID_LEN],
    holes: [u8; SPREAD_GRID_LEN],
}

impl SpreadContext {
    fn new(origin: BlockPos) -> Self {
        Self {
            origin,
            states: [-1; SPREAD_GRID_LEN],
            holes: [0; SPREAD_GRID_LEN],
        }
    }

    fn slot(&self, pos: BlockPos) -> Option<usize> {
        let dx = pos.x - self.origin.x + SPREAD_GRID_RADIUS;
        let dz = pos.z - self.origin.z + SPREAD_GRID_RADIUS;
        if (0..SPREAD_GRID_SIDE as i32).contains(&dx) && (0..SPREAD_GRID_SIDE as i32).contains(&dz)
        {
            Some(dx as usize * SPREAD_GRID_SIDE + dz as usize)
        } else {
            None
        }
    }

    /// `SpreadContext.getBlockState` (`FlowingFluid.java`).
    fn block_state(&mut self, level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
        let Some(slot) = self.slot(pos) else {
            return level.fluid_block_state(pos);
        };
        let cached = self.states[slot];
        if cached >= 0 {
            return cached;
        }
        let state = level.fluid_block_state(pos);
        if state >= 0 {
            self.states[slot] = state;
        }
        state
    }

    /// `SpreadContext.isHole` (`FlowingFluid.java`). Note the state at
    /// `pos` comes from the cache but the one below it does not.
    fn is_hole(&mut self, fluid: FlowingFluid, level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
        let slot = self.slot(pos);
        if let Some(slot) = slot {
            match self.holes[slot] {
                1 => return false,
                2 => return true,
                _ => {}
            }
        }
        let state = self.block_state(level, pos);
        let bottom = below(pos);
        let bottom_state = level.fluid_block_state(bottom);
        let hole = fluid.is_water_hole(level, pos, state, bottom, bottom_state);
        if let Some(slot) = slot {
            self.holes[slot] = if hole { 2 } else { 1 };
        }
        hole
    }
}

// ---------------------------------------------------------------------------
// FlowingFluid
// ---------------------------------------------------------------------------

/// One family's `FlowingFluid` behaviour (`FlowingFluid.java`).
///
/// Vanilla has four singletons; the source/flowing split only changes
/// `getAmount` and `isSource`, which live on [`FluidState`] here, so one value
/// per family is enough to carry the tick machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowingFluid {
    family: FluidFamily,
}

impl FlowingFluid {
    pub const WATER: Self = Self {
        family: FluidFamily::Water,
    };
    pub const LAVA: Self = Self {
        family: FluidFamily::Lava,
    };

    /// `WaterFluid.getTickDelay` (`WaterFluid.java:112-114`): a flat 5 that
    /// reads nothing off the level, which is why callers that hold something
    /// other than the live level — `LeavesBlock.updateShape`
    /// (`LeavesBlock.java:100`) is the one in this workspace — can name it
    /// without one.
    pub const WATER_TICK_DELAY: i32 = 5;

    pub const fn of(family: FluidFamily) -> Self {
        Self { family }
    }

    pub const fn family(self) -> FluidFamily {
        self.family
    }

    /// `FlowingFluid.getFlowing(amount, falling)` (`FlowingFluid.java`).
    fn get_flowing(self, amount: i32, falling: bool) -> FluidState {
        FluidState::flowing(self.family, amount, falling)
    }

    /// `FlowingFluid.getSource(falling)` (`FlowingFluid.java`).
    const fn get_source(self, falling: bool) -> FluidState {
        FluidState::source(self.family, falling)
    }

    /// `FlowingFluid.getDropOff` (`WaterFluid.java` is a flat 1;
    /// `LavaFluid.java` is 1 in a fast-lava dimension and 2
    /// otherwise).
    pub fn drop_off(self, fast_lava: bool) -> i32 {
        match self.family {
            FluidFamily::Water => 1,
            FluidFamily::Lava => {
                if fast_lava {
                    1
                } else {
                    2
                }
            }
        }
    }

    /// `FlowingFluid.getSlopeFindDistance` (`WaterFluid.java` is a flat
    /// 4; `LavaFluid.java` is 4 in a fast-lava dimension and 2
    /// otherwise).
    ///
    /// Older versions spelled this as a `SLOPE_FIND_DISTANCE` constant; 26.1
    /// has no such field, only these two overrides.
    pub fn slope_find_distance(self, fast_lava: bool) -> i32 {
        match self.family {
            FluidFamily::Water => 4,
            FluidFamily::Lava => {
                if fast_lava {
                    4
                } else {
                    2
                }
            }
        }
    }

    /// `Fluid.getTickDelay` (`WaterFluid.java` is a flat 5;
    /// `LavaFluid.java` is 10 in a fast-lava dimension and 30
    /// otherwise).
    pub fn tick_delay(self, fast_lava: bool) -> i32 {
        match self.family {
            FluidFamily::Water => Self::WATER_TICK_DELAY,
            FluidFamily::Lava => {
                if fast_lava {
                    10
                } else {
                    30
                }
            }
        }
    }

    /// `FlowingFluid.getSpreadDelay` (`FlowingFluid.java`) plus lava's
    /// override (`LavaFluid.java`): lava that is *rising* in place
    /// takes four times as long three quarters of the time.
    fn spread_delay(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        old_state: FluidState,
        new_state: FluidState,
    ) -> i32 {
        let mut result = self.tick_delay(level.fluid_fast_lava());
        if self.family != FluidFamily::Lava {
            return result;
        }
        if !old_state.is_empty()
            && !new_state.is_empty()
            && !old_state.falling()
            && !new_state.falling()
            && fluid_height(new_state, level, pos) > fluid_height(old_state, level, pos)
            && level.level_random().next_int(4) != 0
        {
            result *= 4;
        }
        result
    }

    /// `FlowingFluid.isSourceBlockOfThisType` (`FlowingFluid.java`).
    fn is_source_block_of_this_type(self, state: FluidState) -> bool {
        state.is_same_family(self.family) && state.is_source()
    }

    /// `FlowingFluid.tick` (`FlowingFluid.java`).
    ///
    /// A source never recomputes itself; it only spreads. A flowing state
    /// first re-derives what it *should* be from its surroundings, writes that
    /// back (turning to air when the answer is empty), re-arms its own tick
    /// when the state actually changed, and only then spreads.
    pub(crate) fn tick(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
        fluid_state: FluidState,
    ) {
        let mut block_state_id = block_state_id;
        let mut fluid_state = fluid_state;
        if !fluid_state.is_source() {
            let current = level.fluid_block_state(pos);
            let new_fluid_state = self.get_new_liquid(level, pos, current);
            let tick_delay = self.spread_delay(level, pos, fluid_state, new_fluid_state);
            if new_fluid_state.is_empty() {
                fluid_state = new_fluid_state;
                block_state_id = air_block_state_id();
                level.set_block(pos, block_state_id, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
            } else if new_fluid_state != fluid_state {
                fluid_state = new_fluid_state;
                block_state_id = fluid_state.create_legacy_block();
                level.set_block(pos, block_state_id, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                level.schedule_fluid_tick(
                    pos,
                    fluid_state.tick_type(),
                    tick_delay,
                    TickPriority::Normal,
                );
            }
        }
        self.spread(level, pos, block_state_id, fluid_state);
    }

    /// `FlowingFluid.spread` (`FlowingFluid.java`).
    ///
    /// Down first. If the fluid got down, it stops there — unless at least
    /// three of its horizontal neighbours are sources, in which case it also
    /// spreads sideways in the same tick. If it could not get down, it spreads
    /// sideways only when it is a source or when the block below is not a
    /// hole it could still drain into.
    fn spread(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
        fluid_state: FluidState,
    ) {
        if fluid_state.is_empty() {
            return;
        }
        let below_pos = below(pos);
        let below_state = level.fluid_block_state(below_pos);
        let below_fluid = block_state_fluid_state(below_state);
        if self.can_maybe_pass_through(
            level,
            pos,
            block_state_id,
            Direction::Down,
            below_pos,
            below_state,
            below_fluid,
        ) {
            let new_below_fluid = self.get_new_liquid(level, below_pos, below_state);
            let new_below_type = new_below_fluid.identity();
            if can_be_replaced_with(
                below_fluid,
                level,
                below_pos,
                FluidFamily::of_identity(new_below_type),
                Direction::Down,
            ) && can_hold_specific_fluid(below_state, new_below_type)
            {
                self.spread_to(
                    level,
                    below_pos,
                    below_state,
                    Direction::Down,
                    new_below_fluid,
                );
                if self.source_neighbor_count(level, pos) >= 3 {
                    self.spread_to_sides(level, pos, fluid_state, block_state_id);
                }
                return;
            }
        }
        if fluid_state.is_source()
            || !self.is_water_hole(level, pos, block_state_id, below_pos, below_state)
        {
            self.spread_to_sides(level, pos, fluid_state, block_state_id);
        }
    }

    /// `FlowingFluid.spreadToSides` (`FlowingFluid.java`).
    ///
    /// The local `neighbor` is only ever compared against zero: vanilla
    /// computes it, then throws the value away and takes the actual new
    /// neighbour states from `getSpread`/`getNewLiquid`. A falling column
    /// always passes the gate, because its `neighbor` is pinned at 7.
    fn spread_to_sides(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        fluid_state: FluidState,
        block_state_id: i32,
    ) {
        let mut neighbor = fluid_state.amount() - self.drop_off(level.fluid_fast_lava());
        if fluid_state.falling() {
            neighbor = 7;
        }
        if neighbor <= 0 {
            return;
        }
        for (direction, new_neighbor_fluid) in self
            .get_spread(level, pos, block_state_id)
            .into_iter()
            .flatten()
        {
            let neighbor_pos = relative(pos, direction);
            let neighbor_state = level.fluid_block_state(neighbor_pos);
            self.spread_to(
                level,
                neighbor_pos,
                neighbor_state,
                direction,
                new_neighbor_fluid,
            );
        }
    }

    /// `FlowingFluid.getNewLiquid` (`FlowingFluid.java`).
    ///
    /// Three answers, in order: a new source when two neighbouring sources
    /// meet over something that holds them and the game rule allows it; a
    /// full falling state when the same fluid sits directly above; otherwise
    /// the highest reachable neighbour minus the drop-off, or empty.
    fn get_new_liquid(
        self,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
    ) -> FluidState {
        let mut highest_neighbor = 0;
        let mut neighbour_sources = 0;
        for direction in HORIZONTAL {
            let relative_pos = relative(pos, direction);
            let neighbor_state = level.fluid_block_state(relative_pos);
            let neighbor_fluid = block_state_fluid_state(neighbor_state);
            if neighbor_fluid.is_same_family(self.family)
                && can_pass_through_wall_at(
                    Some(level),
                    direction,
                    pos,
                    block_state_id,
                    relative_pos,
                    neighbor_state,
                )
            {
                if neighbor_fluid.is_source() {
                    neighbour_sources += 1;
                }
                highest_neighbor = highest_neighbor.max(neighbor_fluid.amount());
            }
        }
        if neighbour_sources >= 2 && level.fluid_can_convert_to_source(self.family) {
            let below_state = level.fluid_block_state(below(pos));
            let below_fluid = block_state_fluid_state(below_state);
            if block_state_is_solid(below_state) || self.is_source_block_of_this_type(below_fluid) {
                return self.get_source(false);
            }
        }
        let above_pos = above(pos);
        let above_state = level.fluid_block_state(above_pos);
        let above_fluid = block_state_fluid_state(above_state);
        if !above_fluid.is_empty()
            && above_fluid.is_same_family(self.family)
            && can_pass_through_wall_at(
                Some(level),
                Direction::Up,
                pos,
                block_state_id,
                above_pos,
                above_state,
            )
        {
            return self.get_flowing(AMOUNT_FULL, true);
        }
        let amount = highest_neighbor - self.drop_off(level.fluid_fast_lava());
        if amount <= 0 {
            FluidState::Empty
        } else {
            self.get_flowing(amount, false)
        }
    }

    /// `FlowingFluid.spreadTo` (`FlowingFluid.java`) with lava's
    /// override in front (`LavaFluid.java`).
    ///
    /// Order matters: the block that is about to be flooded gets
    /// `beforeDestroyingBlock` *before* the `setBlock` that erases it, so
    /// water still drops the block's items and lava still fizzes at the old
    /// state. A `LiquidBlockContainer` skips both — it is not destroyed, it is
    /// filled.
    fn spread_to(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
        direction: Direction,
        target: FluidState,
    ) {
        // `LavaFluid.spreadTo` (`LavaFluid.java`): lava flowing down
        // onto water turns to stone instead of spreading. Note it re-reads the
        // fluid at the target from the level and that it only replaces the
        // block when the target really is a `LiquidBlock` -- a waterlogged
        // block just fizzes and keeps the lava out.
        if self.family == FluidFamily::Lava && direction == Direction::Down {
            let target_fluid = level.fluid_state(pos);
            if target_fluid.is_same_family(FluidFamily::Water) {
                if is_liquid_block(block_state_id) {
                    level.set_block(
                        pos,
                        stone_block_state_id(),
                        SET_BLOCK_UPDATE_ALL,
                        UPDATE_LIMIT,
                    );
                }
                fizz(level, pos);
                return;
            }
        }
        if liquid_block_container(block_state_id).is_some() {
            place_liquid(level, pos, block_state_id, target);
            return;
        }
        if !block_state_is_air(block_state_id) {
            self.before_destroying_block(level, pos, block_state_id);
        }
        level.set_block(
            pos,
            target.create_legacy_block(),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        );
    }

    /// `FlowingFluid.beforeDestroyingBlock` (`WaterFluid.java` drops the
    /// block's items; `LavaFluid.java` fizzes).
    fn before_destroying_block(
        self,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
    ) {
        match self.family {
            FluidFamily::Water => level.fluid_drop_resources(pos, block_state_id),
            FluidFamily::Lava => fizz(level, pos),
        }
    }

    /// `FlowingFluid.sourceNeighborCount` (`FlowingFluid.java`).
    fn source_neighbor_count(self, level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
        HORIZONTAL
            .into_iter()
            .filter(|direction| {
                self.is_source_block_of_this_type(level.fluid_state(relative(pos, *direction)))
            })
            .count() as i32
    }

    /// `FlowingFluid.isWaterHole` (`FlowingFluid.java:310-318`).
    ///
    /// Note the second branch asks for `getFlowing()`, the *flowing*
    /// singleton, so a waterloggable block below is not a hole: its
    /// `canPlaceLiquid` only accepts the source
    /// (`SimpleWaterloggedBlock.java`).
    fn is_water_hole(
        self,
        level: &LevelCtx<'_, '_>,
        top_pos: BlockPos,
        top_state: i32,
        bottom_pos: BlockPos,
        bottom_state: i32,
    ) -> bool {
        if !can_pass_through_wall_at(
            Some(level),
            Direction::Down,
            top_pos,
            top_state,
            bottom_pos,
            bottom_state,
        ) {
            return false;
        }
        if block_state_fluid_state(bottom_state).is_same_family(self.family) {
            return true;
        }
        can_hold_fluid(level.tags(), bottom_state, self.family.flowing_identity())
    }

    /// `FlowingFluid.canPassThrough` (`FlowingFluid.java:320-332`).
    #[allow(clippy::too_many_arguments)]
    fn can_pass_through(
        self,
        level: &LevelCtx<'_, '_>,
        fluid: VanillaFluidIdentity,
        source_pos: BlockPos,
        source_state: i32,
        direction: Direction,
        test_pos: BlockPos,
        test_state: i32,
        test_fluid: FluidState,
    ) -> bool {
        self.can_maybe_pass_through(
            level,
            source_pos,
            source_state,
            direction,
            test_pos,
            test_state,
            test_fluid,
        ) && can_hold_specific_fluid(test_state, fluid)
    }

    /// `FlowingFluid.canMaybePassThrough` (`FlowingFluid.java:334-346`).
    #[allow(clippy::too_many_arguments)]
    fn can_maybe_pass_through(
        self,
        level: &LevelCtx<'_, '_>,
        source_pos: BlockPos,
        source_state: i32,
        direction: Direction,
        test_pos: BlockPos,
        test_state: i32,
        test_fluid: FluidState,
    ) -> bool {
        !self.is_source_block_of_this_type(test_fluid)
            && can_hold_any_fluid(level.tags(), test_state)
            && can_pass_through_wall_at(
                Some(level),
                direction,
                source_pos,
                source_state,
                test_pos,
                test_state,
            )
    }

    /// `FlowingFluid.getSpread` (`FlowingFluid.java`).
    ///
    /// Every horizontal direction the fluid could enter is scored by how far
    /// it is from a hole it could fall into; only the closest ones survive.
    fn get_spread(
        self,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
        block_state_id: i32,
    ) -> [Option<(Direction, FluidState)>; 4] {
        let mut lowest = 1000;
        let mut result: [Option<FluidState>; 6] = [None; 6];
        let mut context: Option<SpreadContext> = None;
        for direction in HORIZONTAL {
            let test_pos = relative(pos, direction);
            let test_state = level.fluid_block_state(test_pos);
            let test_fluid = block_state_fluid_state(test_state);
            if !self.can_maybe_pass_through(
                level,
                pos,
                block_state_id,
                direction,
                test_pos,
                test_state,
                test_fluid,
            ) {
                continue;
            }
            let new_fluid = self.get_new_liquid(level, test_pos, test_state);
            if !can_hold_specific_fluid(test_state, new_fluid.identity()) {
                continue;
            }
            let context = context.get_or_insert_with(|| SpreadContext::new(pos));
            let distance = if context.is_hole(self, level, test_pos) {
                0
            } else {
                self.get_slope_distance(
                    level,
                    test_pos,
                    1,
                    opposite(direction),
                    test_state,
                    context,
                )
            };
            if distance < lowest {
                result = [None; 6];
            }
            if distance <= lowest {
                if can_be_replaced_with(
                    test_fluid,
                    level,
                    test_pos,
                    FluidFamily::of_identity(new_fluid.identity()),
                    direction,
                ) {
                    result[usize::from(direction.id())] = Some(new_fluid);
                }
                lowest = distance;
            }
        }
        HORIZONTAL_SPREAD_ORDER
            .map(|direction| result[usize::from(direction.id())].map(|fluid| (direction, fluid)))
    }

    /// `FlowingFluid.getSlopeDistance` (`FlowingFluid.java`).
    ///
    /// A breadth-limited flood fill that never turns back the way it came and
    /// stops as soon as it stands over a hole, returning how many steps that
    /// took. `1000` means "no hole within `getSlopeFindDistance` steps".
    fn get_slope_distance(
        self,
        level: &LevelCtx<'_, '_>,
        pos: BlockPos,
        pass: i32,
        from: Direction,
        block_state_id: i32,
        context: &mut SpreadContext,
    ) -> i32 {
        let mut lowest = 1000;
        for direction in HORIZONTAL {
            if direction == from {
                continue;
            }
            let test_pos = relative(pos, direction);
            let test_state = context.block_state(level, test_pos);
            let test_fluid = block_state_fluid_state(test_state);
            if !self.can_pass_through(
                level,
                self.family.flowing_identity(),
                pos,
                block_state_id,
                direction,
                test_pos,
                test_state,
                test_fluid,
            ) {
                continue;
            }
            if context.is_hole(self, level, test_pos) {
                return pass;
            }
            if pass < self.slope_find_distance(level.fluid_fast_lava()) {
                let found = self.get_slope_distance(
                    level,
                    test_pos,
                    pass + 1,
                    opposite(direction),
                    test_state,
                    context,
                );
                if found < lowest {
                    lowest = found;
                }
            }
        }
        lowest
    }
}

/// `LavaFluid.fizz` / `LiquidBlock.fizz` (`LavaFluid.java`,
/// `LiquidBlock.java`).
fn fizz(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    level.fluid_level_event(LEVEL_EVENT_LAVA_FIZZ, pos, 0);
}

fn stone_block_state_id() -> i32 {
    static ID: OnceLock<i32> = OnceLock::new();
    *ID.get_or_init(|| named_default_block_state_id(VANILLA_BLOCK_STONE_ID))
}

/// `Block.defaultBlockState()` for a property-less built-in block.
fn named_default_block_state_id(block_type_id: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type_id)
        .expect("generated block catalog covers every built-in default state")
}

/// `state.getBlock() instanceof LiquidBlock` — true for exactly the water and
/// lava blocks (`Fluids.java` registers no third `LiquidBlock`).
fn is_liquid_block(block_state_id: i32) -> bool {
    liquid_block_family(block_state_id).is_some()
}

/// The family of a `LiquidBlock` state, or `None` for anything else.
pub fn liquid_block_family(block_state_id: i32) -> Option<FluidFamily> {
    match vanilla_block_state_block_type_id(block_state_id)? {
        VANILLA_BLOCK_WATER_ID => Some(FluidFamily::Water),
        VANILLA_BLOCK_LAVA_ID => Some(FluidFamily::Lava),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ServerLevel / LiquidBlock forwarding
// ---------------------------------------------------------------------------

/// `ServerLevel.tickFluid` (`ServerLevel.java`): the entry point the
/// scheduled-tick driver calls.
///
/// The `is(type)` guard compares the *singleton*, not the family, so a tick
/// scheduled as `flowing_water` is dropped if the position has meanwhile
/// become a source (and vice versa).
pub(crate) fn tick_fluid(level: &mut LevelCtx<'_, '_>, pos: BlockPos, fluid: FluidTickType) {
    let block_state_id = level.fluid_block_state(pos);
    let fluid_state = block_state_fluid_state(block_state_id);
    if fluid_state.identity() != fluid.identity() {
        return;
    }
    let Some(family) = fluid_state.family() else {
        // `Fluid.tick` is an empty default (`Fluid.java`) and
        // `EmptyFluid` does not override it.
        return;
    };
    FlowingFluid::of(family).tick(level, pos, block_state_id, fluid_state);
}

/// `LiquidBlock.POSSIBLE_FLOW_DIRECTIONS` (`LiquidBlock.java:57-59`).
const POSSIBLE_FLOW_DIRECTIONS: [Direction; 5] = [
    Direction::Down,
    Direction::South,
    Direction::North,
    Direction::East,
    Direction::West,
];

/// `LiquidBlock.shouldSpreadLiquid` (`LiquidBlock.java`).
///
/// Lava that touches water turns to obsidian (when it is a source) or
/// cobblestone (when it is flowing), and lava over soul soil next to blue ice
/// turns to basalt. Returns whether the caller should still arm the fluid's
/// tick.
///
/// The direction walk has a quirk worth naming: it iterates
/// [`POSSIBLE_FLOW_DIRECTIONS`] but probes `pos.relative(direction.getOpposite())`
/// (`LiquidBlock.java`), so the positions actually examined are UP, NORTH,
/// SOUTH, WEST and EAST — the block *below* is never checked.
pub(crate) fn should_spread_liquid(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> bool {
    if liquid_block_family(block_state_id) != Some(FluidFamily::Lava) {
        return true;
    }
    let is_over_soul_soil = vanilla_block_state_block_type_id(level.fluid_block_state(below(pos)))
        == Some(VANILLA_BLOCK_SOUL_SOIL_ID);
    for direction in POSSIBLE_FLOW_DIRECTIONS {
        let neighbour_pos = relative(pos, opposite(direction));
        if level
            .fluid_state(neighbour_pos)
            .is_same_family(FluidFamily::Water)
        {
            let convert_to = if level.fluid_state(pos).is_source() {
                VANILLA_BLOCK_OBSIDIAN_ID
            } else {
                VANILLA_BLOCK_COBBLESTONE_ID
            };
            level.set_block(
                pos,
                named_default_block_state_id(convert_to),
                SET_BLOCK_UPDATE_ALL,
                UPDATE_LIMIT,
            );
            fizz(level, pos);
            return false;
        }
        if is_over_soul_soil
            && vanilla_block_state_block_type_id(level.fluid_block_state(neighbour_pos))
                == Some(VANILLA_BLOCK_BLUE_ICE_ID)
        {
            level.set_block(
                pos,
                basalt_default_block_state_id(),
                SET_BLOCK_UPDATE_ALL,
                UPDATE_LIMIT,
            );
            fizz(level, pos);
            return false;
        }
    }
    true
}

/// Basalt is a `RotatedPillarBlock`, so it has an `axis` property and no
/// single property-less state; `Blocks.BASALT.defaultBlockState()` is the
/// `axis=y` one (`RotatedPillarBlock` registers `axis` defaulting to `Y`).
fn basalt_default_block_state_id() -> i32 {
    static ID: OnceLock<i32> = OnceLock::new();
    *ID.get_or_init(|| {
        let name = vanilla_block_type(VANILLA_BLOCK_BASALT_ID)
            .expect("generated block catalog holds minecraft:basalt")
            .resource_id;
        let mut properties = BTreeMap::new();
        properties.insert("axis".to_string(), "y".to_string());
        vanilla_block_state_id_by_name_and_properties(name, &properties)
            .expect("minecraft:basalt[axis=y] is a built-in state")
    })
}

/// `LiquidBlock.onPlace` (`LiquidBlock.java`), fluid half only.
///
/// This is what arms the very first tick after a bucket, a world edit or a
/// generator writes a liquid block.
pub(crate) fn liquid_block_on_place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(family) = liquid_block_family(block_state_id) else {
        return;
    };
    if should_spread_liquid(level, pos, block_state_id) {
        let delay = FlowingFluid::of(family).tick_delay(level.fluid_fast_lava());
        level.schedule_fluid_tick(
            pos,
            block_state_fluid_state(block_state_id).tick_type(),
            delay,
            TickPriority::Normal,
        );
    }
}

/// `LiquidBlock.neighborChanged` (`LiquidBlock.java`), fluid half
/// only. Same body as [`liquid_block_on_place`] in 26.1.
pub(crate) fn liquid_block_neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    liquid_block_on_place(level, pos, block_state_id);
}

/// `LiquidBlock.updateShape` (`LiquidBlock.java`), fluid half only.
///
/// Unlike the other two this does not run `shouldSpreadLiquid`; it arms the
/// tick whenever either side of the shape update is a source.
pub(crate) fn liquid_block_update_shape(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    neighbour_state_id: i32,
) {
    let Some(family) = liquid_block_family(block_state_id) else {
        return;
    };
    let fluid_state = block_state_fluid_state(block_state_id);
    if fluid_state.is_source() || block_state_fluid_state(neighbour_state_id).is_source() {
        let delay = FlowingFluid::of(family).tick_delay(level.fluid_fast_lava());
        level.schedule_fluid_tick(pos, fluid_state.tick_type(), delay, TickPriority::Normal);
    }
}

#[cfg(test)]
mod dynamic_shape_tests {
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    };
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

    use super::*;

    fn default_state(name: &str) -> i32 {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    }

    #[test]
    fn dynamic_shapes_occlude_scaffolding_top_and_offset_bamboo_and_dripstone_by_position() {
        let air = default_state("minecraft:air");
        let water = default_state("minecraft:water");
        let scaffolding = default_state("minecraft:scaffolding");
        let powder_snow = default_state("minecraft:powder_snow");
        let moving_piston = default_state("minecraft:moving_piston");
        let shulker_box = default_state("minecraft:shulker_box");

        let above = BlockPos { x: 4, y: 65, z: 4 };
        let cell = BlockPos { x: 4, y: 64, z: 4 };
        let below = BlockPos { x: 4, y: 63, z: 4 };

        // Scaffolding (`SHAPE_STABLE` in `CollisionContext.empty()`) seals its top face (`max_y == 1`)
        // against downward flow, while its open bottom (`min_y == 0` corner posts) stays passable.
        assert!(!can_pass_through_wall_at(
            None,
            Direction::Down,
            above,
            water,
            cell,
            scaffolding,
        ));
        assert!(can_pass_through_wall_at(
            None,
            Direction::Down,
            cell,
            scaffolding,
            below,
            air,
        ));

        // Powder snow and BE-less moving piston have empty CollisionContext.empty() shapes;
        // closed shulker box has a full unit-cube collision shape.
        assert!(can_pass_through_wall_at(
            None,
            Direction::Down,
            above,
            water,
            cell,
            powder_snow,
        ));
        assert!(can_pass_through_wall_at(
            None,
            Direction::Down,
            above,
            water,
            cell,
            moving_piston,
        ));
        assert!(!can_pass_through_wall_at(
            None,
            Direction::Down,
            above,
            water,
            cell,
            shulker_box,
        ));

        // Bamboo and pointed dripstone resolve non-empty position-offset boxes rather than `Shapes.empty()`.
        let bamboo = default_state("minecraft:bamboo");
        let dripstone = vanilla_block_state_id_by_name_and_properties(
            "minecraft:pointed_dripstone",
            &[
                ("thickness".to_string(), "base".to_string()),
                ("vertical_direction".to_string(), "up".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let pos_a = BlockPos { x: 0, y: 64, z: 0 };
        let pos_b = BlockPos { x: 1, y: 64, z: 2 };
        let bamboo_a = face_rects(&collision_shape(None, pos_a, bamboo), Axis::Y, true);
        let bamboo_b = face_rects(&collision_shape(None, pos_b, bamboo), Axis::Y, true);
        assert_eq!(bamboo_a.len(), 1);
        assert_eq!(bamboo_b.len(), 1);
        assert_ne!(bamboo_a, bamboo_b);

        let drip_a = face_rects(&collision_shape(None, pos_a, dripstone), Axis::Y, true);
        let drip_b = face_rects(&collision_shape(None, pos_b, dripstone), Axis::Y, true);
        assert_eq!(drip_a.len(), 1);
        assert_eq!(drip_b.len(), 1);
        assert_ne!(drip_a, drip_b);
    }

    #[test]
    fn flowing_water_spreads_sideways_across_waterlogged_scaffolding_top_face() {
        let mut native = crate::entity::tests::native_level::NativeLevel::new();
        let stone = default_state("minecraft:stone");
        let air = default_state("minecraft:air");
        let water_flowing_1 = vanilla_block_state_id_by_name_and_properties(
            "minecraft:water",
            &[("level".to_string(), "1".to_string())]
                .into_iter()
                .collect(),
        )
        .unwrap();
        let water_source = default_state("minecraft:water");
        let waterlogged_scaffolding = with_property(
            default_state("minecraft:scaffolding"),
            "waterlogged",
            "true",
        )
        .unwrap();

        let base = BlockPos { x: 8, y: 70, z: 8 };
        let set = |native: &mut crate::entity::tests::native_level::NativeLevel, pos, state| {
            native.world.set_block(
                crate::entity::tests::native_level::DIMENSION,
                pos,
                state,
                SET_BLOCK_UPDATE_ALL,
            );
        };
        for x in 1..=14 {
            for z in 1..=14 {
                set(&mut native, BlockPos { x, y: 69, z }, stone);
                for y in 70..=72 {
                    set(&mut native, BlockPos { x, y, z }, air);
                }
            }
        }
        // Waterlogged scaffolding at `(8, 69, 8)`, fed from a source at `(7, 70, 8)` into `(8, 70, 8)`.
        set(
            &mut native,
            BlockPos { x: 8, y: 69, z: 8 },
            waterlogged_scaffolding,
        );
        set(&mut native, BlockPos { x: 7, y: 70, z: 8 }, water_source);
        set(&mut native, base, water_flowing_1);

        let fluid_state = block_state_fluid_state(water_flowing_1);
        native.with_level(|level| {
            FlowingFluid::of(FluidFamily::Water).tick(level, base, water_flowing_1, fluid_state);
        });

        let east_after =
            native.with_level(|level| level.block_state(BlockPos { x: 9, y: 70, z: 8 }));
        assert_eq!(
            block_state_fluid_state(east_after).family(),
            Some(FluidFamily::Water),
            "scaffolding's solid top face prevents `isWaterHole` from trapping flowing water above it"
        );
    }
}
