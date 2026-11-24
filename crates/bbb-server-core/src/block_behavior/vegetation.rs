//! `VegetationBlock` (`net/minecraft/world/level/block/VegetationBlock.java`)
//! and the subclass `mayPlaceOn` bodies this slice can state exactly.
//!
//!
//! * a tag the class itself names — the base body plus the five subclass
//!   bodies that are also one `BlockTags` constant;
//! * a tag the *registration* names, for the four classes whose body is
//!   `state.is(this.supportBlocks)`. That `TagKey<Block>` is a constructor
//!   argument in `Blocks.java`, so it comes from the generated
//!   `may_place_on_support_tag` fact rather than from the class;
//! * `BlockState.isFaceSturdy`, for `CactusFlowerBlock` and `SeagrassBlock`,
//!   answered from the generated per-state support facts;
//! * `LilyPadBlock`'s pair of fluid questions.
//!
//!
//! # What is not here
//!

use super::{block_tag_matches, fluid_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::{BlockGetter, BlockTags};

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride, VanillaMayPlaceOnOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

use crate::fluid::{above, with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::FluidFamily;

use super::{
    below, block_state_fluid_state, block_type_of, bool_property, can_survive_override,
    default_block_state_id, face_sturdy_at, get_state_for_placement_override,
    may_place_on_override, may_place_on_support_tag, string_property, BlockStateAt,
};

/// `SmallDripleafBlock.FACING` (`SmallDripleafBlock.java:32`) and the `HALF`
/// it inherits from `DoublePlantBlock` (`DoublePlantBlock.java:28`).
const FACING: &str = "facing";
const HALF: &str = "half";

const SUPPORTS_VEGETATION_TAG: &str = "minecraft:supports_vegetation";
const SUPPORTS_SMALL_DRIPLEAF_TAG: &str = "minecraft:supports_small_dripleaf";
const SUPPORTS_AZALEA_TAG: &str = "minecraft:supports_azalea";
const SUPPORTS_DRY_VEGETATION_TAG: &str = "minecraft:supports_dry_vegetation";
const SUPPORTS_NETHER_SPROUTS_TAG: &str = "minecraft:supports_nether_sprouts";
const SUPPORTS_NETHER_WART_TAG: &str = "minecraft:supports_nether_wart";
const SUPPORTS_WITHER_ROSE_TAG: &str = "minecraft:supports_wither_rose";
const SUPPORT_OVERRIDE_CACTUS_FLOWER_TAG: &str = "minecraft:support_override_cactus_flower";
const CANNOT_SUPPORT_SEAGRASS_TAG: &str = "minecraft:cannot_support_seagrass";
const SUPPORTS_LILY_PAD_TAG: &str = "minecraft:supports_lily_pad";

#[must_use]
pub(super) fn is_seagrass(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::SeagrassBlock)
}

pub(super) fn seagrass_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::SeagrassBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    if !fluid.is_same_family(FluidFamily::Water) || !fluid.is_full() {
        return None;
    }
    let state = default_block_state_id(block_type_id);
    is_seagrass(state).then_some(state)
}

/// The `mayPlaceOn` bodies this slice transcribes.
///
/// One variant per distinct vanilla body, not one per block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VegetationSupport {
    Vegetation,
    Azalea,
    DryVegetation,
    NetherSprouts,
    NetherWart,
    WitherRose,
    RegistrationTag(&'static str),
    CactusFlower,
    Seagrass,
    Crops,
    LilyPad,
}

/// The `mayPlaceOn` body a block's class declares, off the generated
/// `may_place_on_override` class tag; for the four classes that read a
/// constructor argument, `may_place_on_support_tag` names that argument's tag.
///
/// Read without asking which `canSurvive` reaches it, because a subclass that
/// replaces `canSurvive` still calls its own `mayPlaceOn` from inside the
/// replacement — [`super::double_plant`] is the family that does.
///
/// The `match` is written out variant by variant rather than with a wildcard
/// so that a 26.1 class this module has never seen is a compile error instead
/// of a silent fall-through onto the base tag.
#[must_use]
pub fn may_place_on_body(block_type_id: i32) -> Option<VegetationSupport> {
    match may_place_on_override(block_type_id)? {
        VanillaMayPlaceOnOverride::VegetationBlock => Some(VegetationSupport::Vegetation),
        VanillaMayPlaceOnOverride::AzaleaBlock => Some(VegetationSupport::Azalea),
        VanillaMayPlaceOnOverride::DryVegetationBlock => Some(VegetationSupport::DryVegetation),
        VanillaMayPlaceOnOverride::NetherSproutsBlock => Some(VegetationSupport::NetherSprouts),
        VanillaMayPlaceOnOverride::NetherWartBlock => Some(VegetationSupport::NetherWart),
        VanillaMayPlaceOnOverride::WitherRoseBlock => Some(VegetationSupport::WitherRose),
        VanillaMayPlaceOnOverride::CactusFlowerBlock => Some(VegetationSupport::CactusFlower),
        VanillaMayPlaceOnOverride::SeagrassBlock => Some(VegetationSupport::Seagrass),
        VanillaMayPlaceOnOverride::LilyPadBlock => Some(VegetationSupport::LilyPad),

        // The four bodies that read `this.supportBlocks`. The tag is the
        // registration's, so a class whose registration supplied none — which
        // 26.1 has none of — is refused rather than guessed at.
        VanillaMayPlaceOnOverride::AttachedStemBlock
        | VanillaMayPlaceOnOverride::StemBlock
        | VanillaMayPlaceOnOverride::NetherRootsBlock
        | VanillaMayPlaceOnOverride::NetherFungusBlock => {
            may_place_on_support_tag(block_type_id).map(VegetationSupport::RegistrationTag)
        }

        VanillaMayPlaceOnOverride::CropBlock | VanillaMayPlaceOnOverride::PitcherCropBlock => {
            Some(VegetationSupport::Crops)
        }
        VanillaMayPlaceOnOverride::TallSeagrassBlock => Some(VegetationSupport::Seagrass),

        // The four bodies this module does not state. Each class replaces
        // `canSurvive` too and answers the pair in its own owner module, so a
        // `None` here is a routing statement rather than a gap.
        VanillaMayPlaceOnOverride::MangrovePropaguleBlock
        | VanillaMayPlaceOnOverride::MushroomBlock
        | VanillaMayPlaceOnOverride::SeaPickleBlock
        | VanillaMayPlaceOnOverride::SmallDripleafBlock => None,
    }
}

/// The `mayPlaceOn` body reached by a state that inherits
/// `VegetationBlock.canSurvive` rather than replacing it, or `None` for one
/// that replaces it — which the generated `can_survive_override` class tag is
/// exactly the statement of.
#[must_use]
pub fn vegetation_support(block_state_id: i32) -> Option<VegetationSupport> {
    let block_type_id = block_type_of(block_state_id)?;
    if can_survive_override(block_type_id) != Some(VanillaCanSurviveOverride::VegetationBlock) {
        return None;
    }
    may_place_on_body(block_type_id)
}

/// Whether this slice owns a stored state's survival.
#[must_use]
pub fn is_vegetation(block_state_id: i32) -> bool {
    vegetation_support(block_state_id).is_some()
}

#[must_use]
pub fn is_small_dripleaf(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::SmallDripleafBlock)
}

#[must_use]
pub fn small_dripleaf_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    if !is_small_dripleaf(block_state_id) {
        return None;
    }
    match string_property(block_state_id, "half")?.as_str() {
        "upper" => {
            let below_state = block_state_at(below(pos));
            Some(
                is_small_dripleaf(below_state)
                    && string_property(below_state, "half").as_deref() == Some("lower"),
            )
        }
        "lower" => {
            let below_state = block_state_at(below(pos));
            if tags.block_in_tag(below_state, SUPPORTS_SMALL_DRIPLEAF_TAG) {
                return Some(true);
            }
            if block_state_fluid_state(block_state_at(pos)).identity()
                != VanillaFluidIdentity::Water
            {
                return Some(false);
            }
            Some(tags.block_in_tag(below_state, SUPPORTS_VEGETATION_TAG))
        }
        _ => None,
    }
}

///
///
/// `block_state_at` is the `LevelReader` half. Every body but
/// `LilyPadBlock`'s reads exactly the one cell below, which is all
/// `VegetationBlock.canSurvive` itself touches.
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    may_place_on(
        vegetation_support(block_state_id)?,
        pos,
        tags,
        block_state_at,
    )
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    Some(may_place_on_at(
        level,
        vegetation_support(block_state_id)?,
        pos,
        membership,
    ))
}

fn may_place_on_with(
    support: VegetationSupport,
    support_pos: BlockPos,
    support_state: i32,
    own_state: impl FnOnce() -> i32,
    block_in_tag: impl Fn(&str) -> bool,
    fluid_in_tag: impl Fn(VanillaFluidIdentity, &str) -> bool,
) -> bool {
    match support {
        VegetationSupport::Vegetation => block_in_tag(SUPPORTS_VEGETATION_TAG),
        VegetationSupport::Azalea => block_in_tag(SUPPORTS_AZALEA_TAG),
        VegetationSupport::DryVegetation => block_in_tag(SUPPORTS_DRY_VEGETATION_TAG),
        VegetationSupport::NetherSprouts => block_in_tag(SUPPORTS_NETHER_SPROUTS_TAG),
        VegetationSupport::NetherWart => block_in_tag(SUPPORTS_NETHER_WART_TAG),
        VegetationSupport::WitherRose => block_in_tag(SUPPORTS_WITHER_ROSE_TAG),
        VegetationSupport::RegistrationTag(tag_id) => block_in_tag(tag_id),
        VegetationSupport::CactusFlower => {
            block_in_tag(SUPPORT_OVERRIDE_CACTUS_FLOWER_TAG)
                || face_sturdy_at(
                    support_state,
                    support_pos,
                    Direction::Up,
                    SupportType::Center,
                )
        }
        VegetationSupport::Seagrass => {
            face_sturdy_at(support_state, support_pos, Direction::Up, SupportType::Full)
                && !block_in_tag(CANNOT_SUPPORT_SEAGRASS_TAG)
        }
        VegetationSupport::Crops => block_in_tag(super::crop::SUPPORTS_CROPS_TAG),
        VegetationSupport::LilyPad => {
            let below_fluid = block_state_fluid_state(support_state);
            block_state_fluid_state(own_state()).is_empty()
                && (fluid_in_tag(below_fluid.identity(), SUPPORTS_LILY_PAD_TAG)
                    || block_in_tag(SUPPORTS_LILY_PAD_TAG))
        }
    }
}

pub(super) fn may_place_on_at<L: BlockGetter + ?Sized>(
    level: &L,
    support: VegetationSupport,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    let support_pos = below(pos);
    let support_state = level.block_state(support_pos);
    may_place_on_with(
        support,
        support_pos,
        support_state,
        || level.block_state(pos),
        |tag_id| block_tag_matches(support_state, tag_id, membership),
        |identity, tag_id| fluid_tag_matches(identity, tag_id, membership),
    )
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    can_survive(block_state_id, pos, level.tags(), &mut |at| {
        level.block_state(at)
    })
}

///
/// Takes the plant's own position rather than the cell below, because
/// `LilyPadBlock`'s body reads both.
#[must_use]
pub fn may_place_on(
    support: VegetationSupport,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let support_pos = below(pos);
    let support_state = block_state_at(support_pos);
    Some(may_place_on_with(
        support,
        support_pos,
        support_state,
        || block_state_at(pos),
        |tag_id| tags.block_in_tag(support_state, tag_id),
        |identity, tag_id| tags.fluid_in_tag(identity, tag_id),
    ))
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    // The dispatch only routes states [`vegetation_support`] claims, so a
    // `None` here is `super::face_sturdy`'s dynamic-shape gap; keeping the
    // state is the same fail-closed direction the rest of the slice takes.
    if can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) != Some(false)
    {
        block_state_id
    } else {
        super::air_block_state_id()
    }
}

pub(super) fn seagrass_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if !is_seagrass(block_state_id) {
        return block_state_id;
    }
    let result = update_shape(level, block_state_id, pos);
    if result != super::air_block_state_id() {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    result
}

pub(super) fn small_dripleaf_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if !is_small_dripleaf(block_state_id) {
        return block_state_id;
    }
    if bool_property(block_state_id, "waterlogged") == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    super::double_plant::update_shape(
        block_state_id,
        direction_to_neighbour,
        neighbour_state,
        &mut || {
            small_dripleaf_can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
                level.registry_tag_membership(registry, tag, id)
            })
        },
    )
}

pub(super) fn small_dripleaf_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if !is_small_dripleaf(block_state_id) {
        return None;
    }
    match string_property(block_state_id, "half")?.as_str() {
        "upper" => {
            let below_state = level.block_state(below(pos));
            Some(
                is_small_dripleaf(below_state)
                    && string_property(below_state, "half").as_deref() == Some("lower"),
            )
        }
        "lower" => {
            let below_state = level.block_state(below(pos));
            if block_tag_matches(below_state, SUPPORTS_SMALL_DRIPLEAF_TAG, membership) {
                return Some(true);
            }
            if block_state_fluid_state(level.block_state(pos)).identity()
                != VanillaFluidIdentity::Water
            {
                return Some(false);
            }
            Some(block_tag_matches(
                below_state,
                SUPPORTS_VEGETATION_TAG,
                membership,
            ))
        }
        _ => None,
    }
}

///
/// The facing is the *opposite* of the player's, so a placed dripleaf leans
/// back towards them (`:60`).
#[must_use]
pub fn small_dripleaf_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let state = with_property(
        default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(opposite(horizontal_direction)),
    )?;
    super::double_plant::copy_waterlogged_from(tags, block_state_at(pos), state)
}

pub(super) fn small_dripleaf_set_placed_by(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let above_pos = above(pos);
    if let Some(upper) = small_dripleaf_upper_state_after_placement(
        level.tags(),
        block_state_id,
        level.block_state(above_pos),
    ) {
        level.set_block(
            above_pos,
            upper,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// The upper half takes the lower one's facing, but its own cell's water:
/// `copyWaterloggedFrom` is called with `abovePos` (`:68-70`), so a dripleaf
/// placed with its head out of the water is waterlogged only below.
#[must_use]
pub fn small_dripleaf_upper_state_after_placement(
    tags: &dyn BlockTags,
    lower_state: i32,
    above_state: i32,
) -> Option<i32> {
    if !is_small_dripleaf(lower_state) || string_property(lower_state, HALF)? != "lower" {
        return None;
    }
    let state = with_property(
        default_block_state_id(block_type_of(lower_state)?),
        HALF,
        "upper",
    )?;
    let state = with_property(state, FACING, &string_property(lower_state, FACING)?)?;
    super::double_plant::copy_waterlogged_from(tags, above_state, state)
}
