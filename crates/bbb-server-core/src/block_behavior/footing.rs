//! The classes whose whole `canSurvive` is one question about one cell, with
//! the `updateShape` bodies that are nothing but that question asked again:
//! `CandleCakeBlock`, `SeaPickleBlock`, `SnowLayerBlock`,
//! `SporeBlossomBlock`, `HangingRootsBlock`, `MangrovePropaguleBlock`,
//! `LeafLitterBlock`, `DirtPathBlock`, `FarmlandBlock`, `GrindstoneBlock`,
//! `BasePressurePlateBlock`, `FaceAttachedHorizontalDirectionalBlock`,
//! `TripWireHookBlock`, `FrogspawnBlock` and `SoulFireBlock`.
//!
//! They share a module because they share a shape: only the cell and the test
//! differ, and fifteen modules of four lines each would hide that. The two that
//! answer with a scheduled tick rather than with air -- the path and the
//! farmland -- do so for the reason [`super::farmland`] records.

use super::support::{can_support_center_at_tagged, can_support_center_tagged};
use super::BlockTags;
use super::{block_tag_matches, fluid_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride,
    VanillaNeighborChangedOverride, VANILLA_BLOCK_SNOW_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};

use crate::fluid::{above, below, with_property, FlowingFluid, HORIZONTAL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_solid, FluidFamily};

use super::{
    air_block_state_id, block_type_of, bool_property, can_support_center, can_support_center_at,
    can_support_rigid_block, can_support_rigid_block_at, can_survive_override, collision_face_full,
    collision_face_full_at, collision_face_nonempty, collision_face_nonempty_at,
    default_block_state_id, direction_property, face_sturdy, face_sturdy_at,
    get_state_for_placement_override, int_property, state_is_block, string_property,
    BlockMobSpawnRequest, BlockStateAt,
};

const WATERLOGGED: &str = "waterlogged";
const PICKLES: &str = "pickles";
const SEGMENT_AMOUNT: &str = "segment_amount";
const SUPPORTS_HANGING_MANGROVE_PROPAGULE_TAG: &str =
    "minecraft:supports_hanging_mangrove_propagule";
const SUPPORTS_MANGROVE_PROPAGULE_TAG: &str = "minecraft:supports_mangrove_propagule";
const SUPPORTS_FROGSPAWN_TAG: &str = "minecraft:supports_frogspawn";
const SOUL_FIRE_BASE_BLOCKS_TAG: &str = "minecraft:soul_fire_base_blocks";

/// `SnowLayerBlock.LAYERS` (`SnowLayerBlock.java:27`).
const LAYERS: &str = "layers";

/// The full-thickness value of that property, which is what lets one snow
/// layer stand on another (`SnowLayerBlock.java:84`).
const MAX_LAYERS: i32 = 8;

/// `MangrovePropaguleBlock.HANGING` (`MangrovePropaguleBlock.java:37`).
const HANGING: &str = "hanging";

/// `BlockStateProperties.ATTACH_FACE` (`BlockStateProperties.java:61`) and
/// `HORIZONTAL_FACING` (`:50`), the pair `getConnectedDirection` reads.
const FACE: &str = "face";
const FACING: &str = "facing";

/// `FarmlandBlock.TICK_DELAY` as `DirtPathBlock` and `FarmlandBlock` both
/// spell it at their own `scheduleTick` (`DirtPathBlock.java:55`,
/// `FarmlandBlock.java:58`).
const REVERT_TICK_DELAY: i32 = 1;

fn declares(block_state_id: i32, class: VanillaCanSurviveOverride) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override) == Some(class)
}

fn is_water_at(tags: &dyn BlockTags, block_state_id: i32) -> bool {
    tags.fluid_in_tag(
        block_state_fluid_state(block_state_id).identity(),
        "minecraft:water",
    )
}

/// The plain shape of these `updateShape` bodies: a re-check that answers with
/// air.
fn keep_or_air(block_state_id: i32, survives: Option<bool>) -> i32 {
    if survives == Some(false) {
        air_block_state_id()
    } else {
        block_state_id
    }
}

fn re_arm_water(level: &mut BlockShapeCtx<'_, '_, '_>, pos: BlockPos, block_state_id: i32) {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
}

// ---------------------------------------------------------------------------
// `CandleCakeBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_candle_cake(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::CandleCakeBlock)
}

pub(super) fn candle_cake_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> bool {
    block_state_is_solid(block_state_at(below(pos)))
}

pub(super) fn candle_cake_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let survives = (direction_to_neighbour == Direction::Down)
        .then(|| candle_cake_can_survive(pos, &mut |at| level.block_state(at)));
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `SeaPickleBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_sea_pickle(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::SeaPickleBlock)
}

pub(super) fn sea_pickle_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::SeaPickleBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    let replaced_type_id = block_type_of(replaced)?;
    if replaced_type_id == block_type_id {
        let pickles = int_property(replaced, PICKLES)?;
        return with_property(
            replaced,
            PICKLES,
            &pickles.saturating_add(1).min(4).to_string(),
        );
    }
    let fluid = block_state_fluid_state(replaced);
    with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

pub(super) fn sea_pickle_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    let below_state = block_state_at(below(pos));
    Some(
        collision_face_nonempty(below_state, Direction::Up)?
            || face_sturdy(below_state, Direction::Up, SupportType::Full)?,
    )
}

pub(super) fn sea_pickle_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    if collision_face_nonempty_at(support, support_pos, Direction::Up) {
        return Some(true);
    }
    Some(face_sturdy_at(
        support,
        support_pos,
        Direction::Up,
        SupportType::Full,
    ))
}

pub(super) fn sea_pickle_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if sea_pickle_can_survive_live(level, pos) == Some(false) {
        return air_block_state_id();
    }
    re_arm_water(level, pos, block_state_id);
    block_state_id
}

// ---------------------------------------------------------------------------
// `SnowLayerBlock`
// ---------------------------------------------------------------------------

pub(super) fn snow_layer_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let below_state = block_state_at(below(pos));
    if tags.block_in_tag(below_state, "minecraft:cannot_support_snow_layer") {
        return Some(false);
    }
    if tags.block_in_tag(below_state, "minecraft:support_override_snow_layer") {
        return Some(true);
    }
    let full_snow = state_is_block(below_state, VANILLA_BLOCK_SNOW_ID)
        && int_property(below_state, LAYERS) == Some(MAX_LAYERS);
    Some(collision_face_full(below_state, Direction::Up)? || full_snow)
}

pub(super) fn snow_layer_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let tags = level.tags();
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    if tags.block_in_tag(support, "minecraft:cannot_support_snow_layer") {
        return Some(false);
    }
    if tags.block_in_tag(support, "minecraft:support_override_snow_layer") {
        return Some(true);
    }
    let full_snow = state_is_block(support, VANILLA_BLOCK_SNOW_ID)
        && int_property(support, LAYERS) == Some(MAX_LAYERS);
    Some(collision_face_full_at(support, support_pos, Direction::Up) || full_snow)
}

pub(super) fn snow_layer_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    let survives = snow_layer_can_survive_live(level, pos);
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `SporeBlossomBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_spore_blossom(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::SporeBlossomBlock)
}

pub(super) fn spore_blossom_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let supported = can_support_center_tagged(tags, block_state_at(above(pos)), Direction::Down)?;
    Some(supported && !is_water_at(tags, block_state_at(pos)))
}

pub(super) fn spore_blossom_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = above(pos);
    let supported = can_support_center_at_tagged(
        level.tags(),
        level.block_state(support_pos),
        support_pos,
        Direction::Down,
    );
    Some(supported && !is_water_at(level.tags(), level.block_state(pos)))
}

pub(super) fn spore_blossom_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let survives = (direction_to_neighbour == Direction::Up)
        .then(|| spore_blossom_can_survive_live(level, pos))
        .flatten();
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `HangingRootsBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_hanging_roots(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::HangingRootsBlock)
}

pub(super) fn hanging_roots_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::HangingRootsBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    let state = default_block_state_id(block_type_id);
    if !is_hanging_roots(state) {
        return None;
    }
    with_property(
        state,
        WATERLOGGED,
        if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

pub(super) fn hanging_roots_can_survive(
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    face_sturdy(
        block_state_at(above(pos)),
        Direction::Down,
        SupportType::Full,
    )
}

pub(super) fn hanging_roots_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = above(pos);
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Down,
        SupportType::Full,
    ))
}

pub(super) fn hanging_roots_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Up
        && hanging_roots_can_survive_live(level, pos) == Some(false)
    {
        return air_block_state_id();
    }
    re_arm_water(level, pos, block_state_id);
    block_state_id
}

// ---------------------------------------------------------------------------
// `MangrovePropaguleBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_mangrove_propagule(block_state_id: i32) -> bool {
    declares(
        block_state_id,
        VanillaCanSurviveOverride::MangrovePropaguleBlock,
    )
}

pub(super) fn mangrove_propagule_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    let (support, tag) = if bool_property(block_state_id, HANGING) == Some(true) {
        (above(pos), "minecraft:supports_hanging_mangrove_propagule")
    } else {
        (below(pos), "minecraft:supports_mangrove_propagule")
    };
    tags.block_in_tag(block_state_at(support), tag)
}

pub(super) fn mangrove_propagule_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    let (support_pos, tag_id) = if bool_property(block_state_id, HANGING) == Some(true) {
        (above(pos), SUPPORTS_HANGING_MANGROVE_PROPAGULE_TAG)
    } else {
        (below(pos), SUPPORTS_MANGROVE_PROPAGULE_TAG)
    };
    Some(block_tag_matches(
        level.block_state(support_pos),
        tag_id,
        membership,
    ))
}

pub(super) fn mangrove_propagule_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    re_arm_water(level, pos, block_state_id);
    let survives = if direction_to_neighbour == Direction::Up {
        let Some(survives) =
            mangrove_propagule_can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            })
        else {
            return block_state_id;
        };
        survives
    } else {
        true
    };
    if !survives {
        air_block_state_id()
    } else {
        block_state_id
    }
}

// ---------------------------------------------------------------------------
// `LeafLitterBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_leaf_litter(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::LeafLitterBlock)
}

pub(super) fn leaf_litter_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::LeafLitterBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    if block_type_of(replaced)? == block_type_id {
        let amount = int_property(replaced, SEGMENT_AMOUNT)?;
        return with_property(
            replaced,
            SEGMENT_AMOUNT,
            &amount.saturating_add(1).min(4).to_string(),
        );
    }
    if !HORIZONTAL.contains(&horizontal_direction) {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(opposite(horizontal_direction)),
    )
}

pub(super) fn leaf_litter_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    face_sturdy(block_state_at(below(pos)), Direction::Up, SupportType::Full)
}

pub(super) fn leaf_litter_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Up,
        SupportType::Full,
    ))
}

pub(super) fn leaf_litter_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    let survives = leaf_litter_can_survive_live(level, pos);
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `DirtPathBlock` and `FarmlandBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_dirt_path(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::DirtPathBlock)
}

pub(super) fn is_fence_gate(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.neighbor_changed_override)
        == Some(VanillaNeighborChangedOverride::FenceGateBlock)
}

pub(super) fn dirt_path_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    let above_state = block_state_at(above(pos));
    block_type_of(above_state)?;
    Some(!block_state_is_solid(above_state) || is_fence_gate(above_state))
}

pub(super) fn dirt_path_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Up
        && dirt_path_can_survive(pos, &mut |at| level.block_state(at)) == Some(false)
    {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, REVERT_TICK_DELAY, TickPriority::Normal);
        }
    }
    block_state_id
}

pub(super) fn dirt_path_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    super::farmland::turn_to_dirt(level, pos);
}

// ---------------------------------------------------------------------------
// `GrindstoneBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_grindstone(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::GrindstoneBlock)
}

pub(super) fn grindstone_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> bool {
    let _ = (pos, block_state_at);
    true
}

// ---------------------------------------------------------------------------
// `BasePressurePlateBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_pressure_plate(block_state_id: i32) -> bool {
    declares(
        block_state_id,
        VanillaCanSurviveOverride::BasePressurePlateBlock,
    )
}

pub(super) fn pressure_plate_can_survive(
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let below_state = block_state_at(below(pos));
    Some(can_support_rigid_block(below_state)? || can_support_center(below_state, Direction::Up)?)
}

pub(super) fn pressure_plate_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    if can_support_rigid_block_at(support, support_pos) {
        return Some(true);
    }
    Some(can_support_center_at(support, support_pos, Direction::Up))
}

pub(super) fn pressure_plate_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let survives = (direction_to_neighbour == Direction::Down)
        .then(|| pressure_plate_can_survive_live(level, pos))
        .flatten();
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `FaceAttachedHorizontalDirectionalBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_face_attached(block_state_id: i32) -> bool {
    declares(
        block_state_id,
        VanillaCanSurviveOverride::FaceAttachedHorizontalDirectionalBlock,
    )
}

pub(super) fn connected_direction(block_state_id: i32) -> Option<Direction> {
    match string_property(block_state_id, FACE)?.as_str() {
        "ceiling" => Some(Direction::Down),
        "floor" => Some(Direction::Up),
        _ => direction_property(block_state_id, FACING),
    }
}

pub(super) fn face_attached_update_neighbours(
    level: &mut LevelCtx<'_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) {
    let Some(front) = connected_direction(block_state_id).map(opposite) else {
        return;
    };
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbors_at(pos, block_type_id);
    level.update_neighbors_at(relative(pos, front), block_type_id);
}

pub(super) fn face_attached_affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    moved_by_piston: bool,
) {
    if !moved_by_piston && bool_property(block_state_id, "powered") == Some(true) {
        face_attached_update_neighbours(level, block_state_id, pos);
    }
}

pub(crate) fn face_attached_signal(block_state_id: i32) -> i32 {
    if bool_property(block_state_id, "powered") == Some(true) {
        15
    } else {
        0
    }
}

pub(crate) fn face_attached_direct_signal(block_state_id: i32, direction: Direction) -> i32 {
    if bool_property(block_state_id, "powered") == Some(true)
        && connected_direction(block_state_id) == Some(direction)
    {
        15
    } else {
        0
    }
}

pub(super) fn face_attached_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let direction = opposite(connected_direction(block_state_id)?);
    let at = relative(pos, direction);
    face_sturdy(block_state_at(at), opposite(direction), SupportType::Full)
}

pub(super) fn face_attached_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let direction = connected_direction(block_state_id).map(opposite)?;
    let support_pos = relative(pos, direction);
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        opposite(direction),
        SupportType::Full,
    ))
}

pub(super) fn face_attached_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let attached_to = connected_direction(block_state_id).map(opposite);
    let survives = (attached_to == Some(direction_to_neighbour))
        .then(|| face_attached_can_survive_live(level, block_state_id, pos))
        .flatten();
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `TripWireHookBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_tripwire_hook(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::TripWireHookBlock)
}

pub(super) fn tripwire_hook_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    if !HORIZONTAL.contains(&facing) {
        return Some(false);
    }
    let at = relative(pos, opposite(facing));
    face_sturdy(block_state_at(at), facing, SupportType::Full)
}

pub(super) fn tripwire_hook_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    if !HORIZONTAL.contains(&facing) {
        return Some(false);
    }
    let support_pos = relative(pos, opposite(facing));
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        facing,
        SupportType::Full,
    ))
}

pub(super) fn tripwire_hook_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let facing = direction_property(block_state_id, FACING);
    let survives = (facing == Some(opposite(direction_to_neighbour)))
        .then(|| tripwire_hook_can_survive_live(level, block_state_id, pos))
        .flatten();
    keep_or_air(block_state_id, survives)
}

// ---------------------------------------------------------------------------
// `FrogspawnBlock`
// ---------------------------------------------------------------------------

/// `FrogspawnBlock.minHatchTickDelay` and `maxHatchTickDelay`
/// (`FrogspawnBlock.java:37-38`), whose defaults are the two constants above
/// them (`:34-35`) and which nothing in 26.1 ever writes.
pub const MIN_HATCH_TICK_DELAY: i32 = 3600;
pub const MAX_HATCH_TICK_DELAY: i32 = 12000;

#[must_use]
pub fn is_frogspawn(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::FrogspawnBlock)
}

pub(super) fn frogspawn_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    let below_state = block_state_at(below(pos));
    let held = tags.fluid_in_tag(
        block_state_fluid_state(below_state).identity(),
        "minecraft:supports_frogspawn",
    ) || tags.block_in_tag(below_state, "minecraft:supports_frogspawn");
    held && block_state_fluid_state(block_state_at(pos)).is_empty()
}

pub(super) fn frogspawn_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if !block_state_fluid_state(level.block_state(pos)).is_empty() {
        return air_block_state_id();
    }
    let below_state = level.block_state(below(pos));
    let survives = frogspawn_supported_at(below_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    });
    keep_or_air(block_state_id, Some(survives))
}

fn frogspawn_supported_at(below_state: i32, membership: &dyn Fn(&str, &str, &str) -> bool) -> bool {
    fluid_tag_matches(
        block_state_fluid_state(below_state).identity(),
        SUPPORTS_FROGSPAWN_TAG,
        membership,
    ) || block_tag_matches(below_state, SUPPORTS_FROGSPAWN_TAG, membership)
}

pub(super) fn frogspawn_on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let delay = MIN_HATCH_TICK_DELAY
        + level
            .level_random()
            .next_int(MAX_HATCH_TICK_DELAY - MIN_HATCH_TICK_DELAY);
    if let Some(block) = BlockTickType::of_block_state(block_state_id) {
        level.schedule_block_tick(pos, block, delay, TickPriority::Normal);
    }
}

fn frogspawn_tick_requests(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<Vec<BlockMobSpawnRequest>> {
    if !is_frogspawn(block_state_id) {
        return None;
    }
    let below_state = level.block_state(below(pos));
    block_type_of(below_state)?;
    let current_state = level.block_state(pos);
    block_type_of(current_state)?;
    if !block_state_fluid_state(current_state).is_empty() {
        level.destroy_block(pos, false);
        return Some(Vec::new());
    }
    let supported = frogspawn_supported_at(below_state, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    });
    if !supported {
        level.destroy_block(pos, false);
        return Some(Vec::new());
    }

    level.destroy_block(pos, false);
    level.play_block_sound_with_volume_pitch(pos, "minecraft:block.frogspawn.hatch", 1.0, 1.0);
    let amount = 2 + level.level_random().next_int(4);
    let mut requests = Vec::with_capacity(amount as usize);
    for _ in 0..amount {
        let x_offset = level
            .level_random()
            .next_double()
            .clamp(f64::from(0.2_f32), 0.799_999_997_019_767_8);
        let z_offset = level
            .level_random()
            .next_double()
            .clamp(f64::from(0.2_f32), 0.799_999_997_019_767_8);
        let y_rot = (1 + level.level_random().next_int(360)) as f32;
        requests.push(BlockMobSpawnRequest::FrogspawnTadpole {
            position: Vec3d {
                x: f64::from(pos.x) + x_offset,
                y: f64::from(pos.y) - 0.5,
                z: f64::from(pos.z) + z_offset,
            },
            y_rot,
        });
    }
    Some(requests)
}

pub(super) fn frogspawn_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(requests) = frogspawn_tick_requests(level, pos, block_state_id) else {
        return;
    };
    for request in requests {
        let _ = level.request_block_mob_spawn(request);
    }
}

// ---------------------------------------------------------------------------
// `SoulFireBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_soul_fire(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::SoulFireBlock)
}

pub(super) fn soul_fire_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    tags.block_in_tag(
        block_state_at(below(pos)),
        "minecraft:soul_fire_base_blocks",
    )
}

pub(super) fn soul_fire_can_survive_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    Some(level.block_state_in_tag(level.block_state(below(pos)), SOUL_FIRE_BASE_BLOCKS_TAG))
}

pub(super) fn soul_fire_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if level.block_state_in_tag(level.block_state(below(pos)), SOUL_FIRE_BASE_BLOCKS_TAG) {
        block_type_of(block_state_id).map_or(block_state_id, default_block_state_id)
    } else {
        air_block_state_id()
    }
}
