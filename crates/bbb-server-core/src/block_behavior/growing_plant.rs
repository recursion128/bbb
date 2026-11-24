//! `GrowingPlantBlock` (`net/minecraft/world/level/block/GrowingPlantBlock.java`)
//! and its two halves, `GrowingPlantHeadBlock` and `GrowingPlantBodyBlock`.
//!
//!
//! # The four plants differ only in their constructor arguments
//!
//! `GrowingPlantBlock` stores a growth direction and a fluid-tick flag
//! (`:18-19`) and `GrowingPlantHeadBlock` a growth probability (`:22`); every
//! per-plant difference in these bodies is one of those three, plus the two
//! abstract methods `canGrowInto` and `getBodyBlock` and the two optional
//! `canAttachTo`/`getGrowIntoState` overrides. [`GrowingPlantKind`] carries
//! them, transcribed one registration at a time, exactly as
//! [`super::crop::CropKind`] carries `CropBlock`'s subclass overrides.
//!
//!
//! # What is not here
//!

use super::BlockTags;
use super::{block_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaCanSurviveOverride, VANILLA_BLOCK_CAVE_VINES_ID,
    VANILLA_BLOCK_CAVE_VINES_PLANT_ID, VANILLA_BLOCK_KELP_ID, VANILLA_BLOCK_KELP_PLANT_ID,
    VANILLA_BLOCK_TWISTING_VINES_ID, VANILLA_BLOCK_TWISTING_VINES_PLANT_ID, VANILLA_BLOCK_WATER_ID,
    VANILLA_BLOCK_WEEPING_VINES_ID, VANILLA_BLOCK_WEEPING_VINES_PLANT_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_is_air, FluidFamily};

use super::{
    block_type_of, bool_property, can_survive_override, cycle_property, default_block_state_id,
    face_sturdy, face_sturdy_at, int_property, state_is_block, BlockStateAt,
};

/// The property `GrowingPlantHeadBlock.AGE` names
/// (`GrowingPlantHeadBlock.java:20`, `BlockStateProperties.AGE_25` =
/// `IntegerProperty.create("age", 0, 25)`,
/// `BlockStateProperties.java:98`).
const AGE: &str = "age";

/// The property `CaveVines.BERRIES` names (`CaveVines.java:21`,
/// `BlockStateProperties.BERRIES` = `BooleanProperty.create("berries")`,
/// `BlockStateProperties.java:11`).
const BERRIES: &str = "berries";
const CANNOT_SUPPORT_KELP_TAG: &str = "minecraft:cannot_support_kelp";

/// `GrowingPlantHeadBlock.MAX_AGE` (`GrowingPlantHeadBlock.java:21`).
pub const MAX_AGE: i32 = 25;

/// The bound `GrowingPlantHeadBlock.getStateForPlacement` draws against
/// (`GrowingPlantHeadBlock.java:41`), which is [`MAX_AGE`] spelled as a
/// literal there — so a converted head is never at max age.
pub const PLACEMENT_AGE_BOUND: i32 = 25;

/// `CaveVinesBlock.CHANCE_OF_BERRIES_ON_GROWTH` (`CaveVinesBlock.java:21`).
pub const CHANCE_OF_BERRIES_ON_GROWTH: f32 = 0.11;

/// The tick delay both halves' `updateShape` arms
/// (`GrowingPlantHeadBlock.java:88`, `GrowingPlantBodyBlock.java:47`).
pub const TICK_DELAY: i32 = 1;

/// The four `GrowingPlantBlock` registrations 26.1 carries, as the pair of
/// blocks each one owns.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowingPlantKind {
    Kelp,
    WeepingVines,
    TwistingVines,
    CaveVines,
}

impl GrowingPlantKind {
    /// `GrowingPlantBlock.growthDirection` (`GrowingPlantBlock.java:18`), as
    /// the four constructors set it (`KelpBlock.java:32`,
    /// `WeepingVinesBlock.java:20`, `TwistingVinesBlock.java:20`,
    /// `CaveVinesBlock.java:29`).
    #[must_use]
    pub const fn growth_direction(self) -> Direction {
        match self {
            Self::Kelp | Self::TwistingVines => Direction::Up,
            Self::WeepingVines | Self::CaveVines => Direction::Down,
        }
    }

    /// `GrowingPlantHeadBlock.growPerTickProbability`
    /// (`GrowingPlantHeadBlock.java:22`): `KelpBlock.GROW_PER_TICK_PROBABILITY`
    /// (`KelpBlock.java:23`) for kelp and `NetherVines.GROW_PER_TICK_PROBABILITY`
    /// (`NetherVines.java:8`) for the other three, which spell the 0.1 out.
    #[must_use]
    pub const fn grow_per_tick_probability(self) -> f64 {
        match self {
            Self::Kelp => 0.14,
            Self::WeepingVines | Self::TwistingVines | Self::CaveVines => 0.1,
        }
    }

    /// `GrowingPlantBlock.scheduleFluidTicks` (`GrowingPlantBlock.java:19`).
    /// Only kelp stands in water (`KelpBlock.java:32`).
    #[must_use]
    pub const fn schedules_fluid_ticks(self) -> bool {
        matches!(self, Self::Kelp)
    }

    /// `GrowingPlantBlock.getHeadBlock` (`GrowingPlantHeadBlock.java:142-144`)
    /// as a `BuiltInRegistries.BLOCK` id.
    #[must_use]
    pub const fn head_block_type_id(self) -> i32 {
        match self {
            Self::Kelp => VANILLA_BLOCK_KELP_ID,
            Self::WeepingVines => VANILLA_BLOCK_WEEPING_VINES_ID,
            Self::TwistingVines => VANILLA_BLOCK_TWISTING_VINES_ID,
            Self::CaveVines => VANILLA_BLOCK_CAVE_VINES_ID,
        }
    }

    /// `GrowingPlantHeadBlock.getBodyBlock` (`KelpBlock.java:41-43`,
    /// `WeepingVinesBlock.java:29-31`, `TwistingVinesBlock.java:29-31`,
    /// `CaveVinesBlock.java:44-46`) as a `BuiltInRegistries.BLOCK` id.
    #[must_use]
    pub const fn body_block_type_id(self) -> i32 {
        match self {
            Self::Kelp => VANILLA_BLOCK_KELP_PLANT_ID,
            Self::WeepingVines => VANILLA_BLOCK_WEEPING_VINES_PLANT_ID,
            Self::TwistingVines => VANILLA_BLOCK_TWISTING_VINES_PLANT_ID,
            Self::CaveVines => VANILLA_BLOCK_CAVE_VINES_PLANT_ID,
        }
    }

    /// Whether both halves carry `CaveVines.BERRIES`, i.e. whether the pair of
    /// conversion hooks and `getGrowIntoState` have an override to run
    /// (`CaveVinesBlock.java:49-56`, `CaveVinesPlantBlock.java:38-40`).
    #[must_use]
    pub const fn carries_berries(self) -> bool {
        matches!(self, Self::CaveVines)
    }

    #[must_use]
    pub fn can_grow_into(self, block_state_id: i32) -> bool {
        match self {
            Self::Kelp => state_is_block(block_state_id, VANILLA_BLOCK_WATER_ID),
            Self::WeepingVines | Self::TwistingVines | Self::CaveVines => {
                block_state_is_air(block_state_id)
            }
        }
    }

    #[must_use]
    pub fn can_attach_to(self, tags: &dyn BlockTags, block_state_id: i32) -> bool {
        match self {
            Self::Kelp => !tags.block_in_tag(block_state_id, "minecraft:cannot_support_kelp"),
            Self::WeepingVines | Self::TwistingVines | Self::CaveVines => true,
        }
    }
}

/// The plant a stored state belongs to, or `None` for anything outside the
/// family.
///
/// The family test is the generated `can_survive_override` class tag, which
/// names `GrowingPlantBlock` for exactly these eight blocks; the split into
/// four plants is then eight `bbb_protocol::block_types` id constants, because
/// a constructor argument is not something any generated fact carries.
#[must_use]
pub fn growing_plant_kind(block_state_id: i32) -> Option<GrowingPlantKind> {
    let block_type_id = block_type_of(block_state_id)?;
    if can_survive_override(block_type_id) != Some(VanillaCanSurviveOverride::GrowingPlantBlock) {
        return None;
    }
    match block_type_id {
        VANILLA_BLOCK_KELP_ID | VANILLA_BLOCK_KELP_PLANT_ID => Some(GrowingPlantKind::Kelp),
        VANILLA_BLOCK_WEEPING_VINES_ID | VANILLA_BLOCK_WEEPING_VINES_PLANT_ID => {
            Some(GrowingPlantKind::WeepingVines)
        }
        VANILLA_BLOCK_TWISTING_VINES_ID | VANILLA_BLOCK_TWISTING_VINES_PLANT_ID => {
            Some(GrowingPlantKind::TwistingVines)
        }
        VANILLA_BLOCK_CAVE_VINES_ID | VANILLA_BLOCK_CAVE_VINES_PLANT_ID => {
            Some(GrowingPlantKind::CaveVines)
        }
        _ => None,
    }
}

/// Whether a stored state is one of the four `GrowingPlantHeadBlock` tips.
///
/// `growing_plant_head` is the generated `instanceof GrowingPlantHeadBlock`
/// bit; the family test beside it keeps a future class that gains the bit
/// without gaining `GrowingPlantBlock.canSurvive` from landing here silently.
#[must_use]
pub fn is_head(block_state_id: i32) -> bool {
    growing_plant_kind(block_state_id).is_some()
        && block_type_of(block_state_id)
            .and_then(vanilla_block_type)
            .is_some_and(|entry| entry.growing_plant_head)
}

/// Whether a stored state is one of the four `GrowingPlantBodyBlock` stems.
#[must_use]
pub fn is_body(block_state_id: i32) -> bool {
    growing_plant_kind(block_state_id).is_some() && !is_head(block_state_id)
}

#[must_use]
pub fn has_berries(block_state_id: i32) -> bool {
    bool_property(block_state_id, BERRIES) == Some(true)
}

#[must_use]
pub fn with_berries(block_state_id: i32, berries: bool) -> Option<i32> {
    with_property(block_state_id, BERRIES, bool_str(berries))
}

///
/// `None` is the [`super::face_sturdy`] refusal and nothing else: the two
/// early answers are exact, so only a dynamic-shape block behind the plant
/// leaves the question open.
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let kind = growing_plant_kind(block_state_id)?;
    let growth_direction = kind.growth_direction();
    let attached_to_pos = relative(pos, opposite(growth_direction));
    let attached_to_state = block_state_at(attached_to_pos);
    survives_on(
        kind,
        attached_to_state,
        !kind.can_attach_to(tags, attached_to_state),
        || face_sturdy(attached_to_state, growth_direction, SupportType::Full),
    )
}

pub(super) fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    let kind = growing_plant_kind(block_state_id)?;
    let growth_direction = kind.growth_direction();
    let support_pos = relative(pos, opposite(growth_direction));
    let support = level.block_state(support_pos);
    let cannot_support = matches!(kind, GrowingPlantKind::Kelp)
        && block_tag_matches(support, CANNOT_SUPPORT_KELP_TAG, membership);
    survives_on(kind, support, cannot_support, || {
        Some(face_sturdy_at(
            support,
            support_pos,
            growth_direction,
            SupportType::Full,
        ))
    })
}

fn survives_on(
    kind: GrowingPlantKind,
    support: i32,
    cannot_support: bool,
    face_support: impl FnOnce() -> Option<bool>,
) -> Option<bool> {
    if cannot_support {
        return Some(false);
    }
    if state_is_block(support, kind.head_block_type_id())
        || state_is_block(support, kind.body_block_type_id())
    {
        return Some(true);
    }
    face_support()
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let kind = growing_plant_kind(block_state_id)?;
    let growth_direction = kind.growth_direction();
    let support_pos = relative(pos, opposite(growth_direction));
    let support = level.block_state(support_pos);
    survives_on(
        kind,
        support,
        !kind.can_attach_to(level.tags(), support),
        || {
            Some(face_sturdy_at(
                support,
                support_pos,
                growth_direction,
                SupportType::Full,
            ))
        },
    )
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) == Some(false)
    {
        level.destroy_block(pos, true);
    }
}

///
/// The `&&` short-circuits, so a head already at [`MAX_AGE`] draws nothing —
/// which is also what `isRandomlyTicking` (`:44-47`) already promised the
/// caller, so the two agree and the state is never sampled anyway.
pub(super) fn head_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(kind) = growing_plant_kind(block_state_id) else {
        return;
    };
    if int_property(block_state_id, AGE).is_none_or(|age| age >= MAX_AGE) {
        return;
    }
    if level.level_random().next_double() >= kind.grow_per_tick_probability() {
        return;
    }
    let growth_pos = relative(pos, kind.growth_direction());
    if !kind.can_grow_into(level.block_state(growth_pos)) {
        return;
    }
    let Some(grown) = grow_into_state(level, kind, block_state_id) else {
        return;
    };
    level.set_block(
        growth_pos,
        grown,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

///
/// `state.cycle(AGE)` wraps at [`MAX_AGE`], which is why the caller's age gate
/// comes first.
fn grow_into_state(
    level: &mut LevelCtx<'_, '_>,
    kind: GrowingPlantKind,
    block_state_id: i32,
) -> Option<i32> {
    let cycled = cycle_property(block_state_id, AGE);
    if !kind.carries_berries() {
        return cycled;
    }
    let berries = level.level_random().next_float() < CHANCE_OF_BERRIES_ON_GROWTH;
    with_property(cycled?, BERRIES, bool_str(berries))
}

fn body_after_converted_from_head(kind: GrowingPlantKind, head_state: i32) -> i32 {
    let body = default_block_state_id(kind.body_block_type_id());
    if !kind.carries_berries() {
        return body;
    }
    bool_property(head_state, BERRIES)
        .and_then(|berries| with_property(body, BERRIES, bool_str(berries)))
        .unwrap_or(body)
}

fn head_after_converted_from_body(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    kind: GrowingPlantKind,
    body_state: i32,
) -> i32 {
    let age = level.level_random().next_int(PLACEMENT_AGE_BOUND);
    let head = default_block_state_id(kind.head_block_type_id());
    let placed = with_property(head, AGE, &age.to_string()).unwrap_or(head);
    if !kind.carries_berries() {
        return placed;
    }
    bool_property(body_state, BERRIES)
        .and_then(|berries| with_property(placed, BERRIES, bool_str(berries)))
        .unwrap_or(placed)
}

///
/// Two ways a head becomes body: the cell ahead of it already holds the plant
/// when the cell behind it changes (`:90-93`), and the neighbour that changed
/// *is* the cell ahead and holds the plant (`:103-105`). The first needs the
/// head to still be supported, the second does not.
pub(super) fn head_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(kind) = growing_plant_kind(block_state_id) else {
        return block_state_id;
    };
    let growth_direction = kind.growth_direction();
    if direction_to_neighbour == opposite(growth_direction) {
        // A dynamic-shape gap arms neither removal nor conversion.
        match can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) {
            Some(false) => {
                if let Some(block) = BlockTickType::of_block_state(block_state_id) {
                    level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
                }
            }
            Some(true) => {
                let ahead = level.block_state(relative(pos, growth_direction));
                if is_own_plant(kind, ahead) {
                    return body_after_converted_from_head(kind, block_state_id);
                }
            }
            None => {}
        }
    }

    if direction_to_neighbour != growth_direction || !is_own_plant(kind, neighbour_state) {
        schedule_water_tick(level, kind, pos);
        block_state_id
    } else {
        body_after_converted_from_head(kind, block_state_id)
    }
}

///
/// The mirror of the head's: a body whose growth-direction neighbour stops
/// being the plant becomes the new head, with a fresh random age.
pub(super) fn body_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(kind) = growing_plant_kind(block_state_id) else {
        return block_state_id;
    };
    let growth_direction = kind.growth_direction();
    if direction_to_neighbour == opposite(growth_direction) {
        match can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) {
            Some(false) => {
                if let Some(block) = BlockTickType::of_block_state(block_state_id) {
                    level.schedule_block_tick(pos, block, TICK_DELAY, TickPriority::Normal);
                }
            }
            Some(true) => {}
            None => {}
        }
    }

    if direction_to_neighbour == growth_direction && !is_own_plant(kind, neighbour_state) {
        return head_after_converted_from_body(level, kind, block_state_id);
    }

    schedule_water_tick(level, kind, pos);
    block_state_id
}

fn is_own_plant(kind: GrowingPlantKind, block_state_id: i32) -> bool {
    state_is_block(block_state_id, kind.head_block_type_id())
        || state_is_block(block_state_id, kind.body_block_type_id())
}

fn schedule_water_tick(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    kind: GrowingPlantKind,
    pos: BlockPos,
) {
    if !kind.schedules_fluid_ticks() {
        return;
    }
    level.schedule_fluid_tick(
        pos,
        FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
        FlowingFluid::WATER_TICK_DELAY,
        TickPriority::Normal,
    );
}

fn bool_str(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// The head a bone meal on any part of the plant grows, when the cell ahead
/// of it can take the plant: `GrowingPlantHeadBlock.isValidBonemealTarget`
/// (`GrowingPlantHeadBlock.java:114-117`) and the body's walk to its head
/// (`GrowingPlantBodyBlock.java:68-76`, `BlockUtil.getTopConnectedBlock`).
pub(super) fn bonemeal_head(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) -> Option<(BlockPos, GrowingPlantKind)> {
    let kind = growing_plant_kind(state)?;
    let direction = kind.growth_direction();
    let mut head = pos;
    if !is_head(state) {
        loop {
            head = relative(head, direction);
            if !state_is_block(level.block_state(head), kind.body_block_type_id()) {
                break;
            }
        }
        if !state_is_block(level.block_state(head), kind.head_block_type_id()) {
            return None;
        }
    }
    let growth = relative(head, direction);
    (kind.can_grow_into(level.block_state(growth))
        && growth.y >= level.get_min_y()
        && growth.y <= level.get_max_y())
    .then_some((head, kind))
}

/// `GrowingPlantHeadBlock.performBonemeal` (`:125-135`) with
/// `getBlocksToGrowWhenBonemealed`: one for kelp and cave vines,
/// `NetherVines.getBlocksToGrowWhenBonemealed` for the nether vines.
pub(super) fn perform_head_bonemeal(
    level: &mut LevelCtx<'_, '_>,
    head: BlockPos,
    kind: GrowingPlantKind,
) {
    let state = level.block_state(head);
    let direction = kind.growth_direction();
    let blocks_to_grow = match kind {
        GrowingPlantKind::Kelp | GrowingPlantKind::CaveVines => 1,
        GrowingPlantKind::WeepingVines | GrowingPlantKind::TwistingVines => {
            let random = level.level_random();
            let mut probability = 1.0;
            let mut count = 0;
            while random.next_double() < probability {
                probability *= 0.826;
                count += 1;
            }
            count
        }
    };
    let mut forward = relative(head, direction);
    let mut next_age = int_property(state, AGE).map_or(MAX_AGE, |age| (age + 1).min(MAX_AGE));
    for _ in 0..blocks_to_grow {
        if !kind.can_grow_into(level.block_state(forward))
            || forward.y < level.get_min_y()
            || forward.y > level.get_max_y()
        {
            break;
        }
        let Some(grown) = with_property(state, AGE, &next_age.to_string()) else {
            break;
        };
        level.set_block(
            forward,
            grown,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        forward = relative(forward, direction);
        next_age = (next_age + 1).min(MAX_AGE);
    }
}
