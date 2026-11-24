//! Big dripleaf columns, growth and leaf tilt.

use super::BlockTags;
use super::{block_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};

use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_BIG_DRIPLEAF_ID,
    VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID, VANILLA_BLOCK_SMALL_DRIPLEAF_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::{
    block_state_fluid_state, block_state_is_air, FluidFamily, FluidState,
};

use super::{
    air_block_state_id, block_type_of, bool_property, can_survive_override, default_block_state_id,
    get_state_for_placement_override, state_is_block, string_property, BlockStateAt,
    VanillaGameEvent, SET_BLOCK_UPDATE_CLIENTS,
};

/// `BlockStateProperties.TILT` (`BigDripleafBlock.java:48`).
const TILT: &str = "tilt";
const WATERLOGGED: &str = "waterlogged";
const FACING: &str = "facing";
const SUPPORTS_BIG_DRIPLEAF_TAG: &str = "minecraft:supports_big_dripleaf";

fn delay_until_next_tilt_state(tilt: &str) -> Option<i32> {
    match tilt {
        "unstable" | "partial" => Some(10),
        "full" => Some(100),
        _ => None,
    }
}

fn causes_vibration(tilt: &str) -> bool {
    tilt != "unstable"
}

/// The delay `BigDripleafStemBlock.updateShape` arms its own tick with
/// (`BigDripleafStemBlock.java:87`).
const STEM_TICK_DELAY: i32 = 1;

/// `SoundEvents.BIG_DRIPLEAF_TILT_DOWN` (`SoundEvents.java:553`).
const TILT_DOWN_SOUND: &str = "minecraft:block.big_dripleaf.tilt_down";

/// `SoundEvents.BIG_DRIPLEAF_TILT_UP` (`SoundEvents.java:554`).
const TILT_UP_SOUND: &str = "minecraft:block.big_dripleaf.tilt_up";

fn declares(block_state_id: i32, class: VanillaCanSurviveOverride) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override) == Some(class)
}

#[must_use]
pub fn is_leaf(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::BigDripleafBlock)
}

#[must_use]
pub fn is_stem(block_state_id: i32) -> bool {
    declares(
        block_state_id,
        VanillaCanSurviveOverride::BigDripleafStemBlock,
    )
}

fn tilt(block_state_id: i32) -> Option<String> {
    string_property(block_state_id, TILT)
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
// The leaf
// ---------------------------------------------------------------------------

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::BigDripleafBlock)
    {
        return None;
    }
    let below_state = block_state_at(below(pos));
    let replaced = block_state_at(pos);
    block_type_of(below_state)?;
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    let facing = if state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID)
        || state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
    {
        string_property(below_state, FACING)?
    } else {
        if matches!(horizontal_direction, Direction::Down | Direction::Up) {
            return None;
        }
        super::direction_serialized_name(opposite(horizontal_direction)).to_owned()
    };
    let state = default_block_state_id(block_type_id);
    if !is_leaf(state) {
        return None;
    }
    let state = with_property(
        state,
        WATERLOGGED,
        if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )?;
    with_property(state, FACING, &facing)
}

pub(super) fn can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    let below_state = block_state_at(below(pos));
    state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID)
        || state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
        || tags.block_in_tag(below_state, "minecraft:supports_big_dripleaf")
}

pub(super) fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    let below_state = level.block_state(below(pos));
    if state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID)
        || state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
    {
        Some(true)
    } else {
        Some(block_tag_matches(
            below_state,
            SUPPORTS_BIG_DRIPLEAF_TAG,
            membership,
        ))
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if direction_to_neighbour == Direction::Down {
        match can_survive_at(level, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) {
            Some(false) => return air_block_state_id(),
            Some(true) => {}
            None => return block_state_id,
        }
    }
    re_arm_water(level, pos, block_state_id);
    if direction_to_neighbour == Direction::Up
        && state_is_block(neighbour_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID)
    {
        return stem_like(block_state_id).unwrap_or(block_state_id);
    }
    block_state_id
}

/// `Blocks.BIG_DRIPLEAF_STEM.withPropertiesOf(state)` (`:165`) over the two
/// properties the leaf and the stem share.
fn stem_like(block_state_id: i32) -> Option<i32> {
    let stem = default_block_state_id(VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID);
    let waterlogged = bool_property(block_state_id, WATERLOGGED)?;
    let facing = string_property(block_state_id, FACING)?;
    let stem = with_property(
        stem,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )?;
    with_property(stem, FACING, &facing)
}

fn can_grow_into(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    pos.y >= level.get_min_y()
        && pos.y <= level.get_max_y()
        && (block_state_is_air(state)
            || state_is_block(state, VANILLA_BLOCK_WATER_ID)
            || state_is_block(state, VANILLA_BLOCK_SMALL_DRIPLEAF_ID))
}

fn head_above_stem(level: &LevelCtx<'_, '_>, mut pos: BlockPos) -> Option<BlockPos> {
    loop {
        pos = above(pos);
        if pos.y > level.get_max_y() {
            return None;
        }
        let state = level.block_state(pos);
        if !is_stem(state) {
            return is_leaf(state).then_some(pos);
        }
    }
}

fn bonemeal_head(level: &LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> Option<BlockPos> {
    if is_stem(state) {
        head_above_stem(level, pos)
    } else {
        Some(pos)
    }
}

/// `BigDripleafBlock.isValidBonemealTarget` / `BigDripleafStemBlock.isValidBonemealTarget`.
pub(super) fn is_valid_bonemeal_target(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) -> bool {
    bonemeal_head(level, pos, state).is_some_and(|head| can_grow_into(level, above(head)))
}

fn place_growth(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block: i32,
    fluid: FluidState,
    facing: &str,
) {
    let state = with_property(default_block_state_id(block), FACING, facing).and_then(|state| {
        with_property(
            state,
            WATERLOGGED,
            if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
                "true"
            } else {
                "false"
            },
        )
    });
    if let Some(state) = state {
        level.set_block(
            pos,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// `BigDripleafBlock.performBonemeal` / `BigDripleafStemBlock.performBonemeal`.
pub(super) fn perform_bonemeal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let Some(head) = bonemeal_head(level, pos, state) else {
        return;
    };
    let Some(facing) = string_property(state, FACING) else {
        return;
    };
    let next = above(head);
    if can_grow_into(level, next) {
        let fluid = level.fluid_state(head);
        place_growth(
            level,
            head,
            VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID,
            fluid,
            &facing,
        );
        let fluid = level.fluid_state(next);
        place_growth(level, next, VANILLA_BLOCK_BIG_DRIPLEAF_ID, fluid, &facing);
    }
}

/// `SmallDripleafBlock.performBonemeal` and `BigDripleafBlock.placeWithRandomHeight`.
pub(super) fn small_perform_bonemeal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let lower = if string_property(state, "half").as_deref() == Some("upper") {
        below(pos)
    } else {
        pos
    };
    let Some(facing) = string_property(level.block_state(lower), FACING) else {
        return;
    };
    let upper = above(lower);
    level.set_block(
        upper,
        level.fluid_state(upper).create_legacy_block(),
        18,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    let desired_height = 2 + level.level_random().next_int(4);
    let mut height = 0;
    let mut next = lower;
    while height < desired_height && can_grow_into(level, next) {
        height += 1;
        next = above(next);
    }
    for offset in 0..height {
        let at = BlockPos {
            y: lower.y + offset,
            ..lower
        };
        let block = if offset + 1 == height {
            VANILLA_BLOCK_BIG_DRIPLEAF_ID
        } else {
            VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID
        };
        let fluid = level.fluid_state(at);
        place_growth(level, at, block, fluid, &facing);
    }
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let powered = level.has_neighbor_signal(pos);
    if powered {
        reset_tilt(level, pos, block_state_id);
        return;
    }
    match tilt(block_state_id).as_deref() {
        Some("unstable") => {
            set_tilt_and_schedule_tick(
                level,
                pos,
                block_state_id,
                "partial",
                Some(TILT_DOWN_SOUND),
            );
        }
        Some("partial") => {
            set_tilt_and_schedule_tick(level, pos, block_state_id, "full", Some(TILT_DOWN_SOUND));
        }
        Some("full") => reset_tilt(level, pos, block_state_id),
        _ => {}
    }
}

/// `BigDripleafBlock.entityInside` (`BigDripleafBlock.java:190-198`) over
/// `canEntityTilt` (`:230-232`).
pub(crate) fn entity_inside(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    on_ground: bool,
    entity_y: f64,
) {
    if tilt(block_state_id).as_deref() == Some("none")
        && on_ground
        && entity_y > f64::from(pos.y) + f64::from(0.6875_f32)
        && !level.has_neighbor_signal(pos)
    {
        set_tilt_and_schedule_tick(level, pos, block_state_id, "unstable", None);
    }
}

/// `BigDripleafBlock.onProjectileHit` (`BigDripleafBlock.java:129-131`).
pub(super) fn on_projectile_hit(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    set_tilt_and_schedule_tick(level, pos, block_state_id, "full", Some(TILT_DOWN_SOUND));
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.has_neighbor_signal(pos) {
        reset_tilt(level, pos, block_state_id);
    }
}

fn set_tilt_and_schedule_tick(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    tilt: &str,
    sound: Option<&str>,
) {
    set_tilt(level, pos, block_state_id, tilt);
    if let Some(sound) = sound {
        play_tilt_sound(level, pos, sound);
    }
    if let Some(delay) = delay_until_next_tilt_state(tilt) {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, delay, TickPriority::Normal);
        }
    }
}

fn reset_tilt(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    set_tilt(level, pos, block_state_id, "none");
    if tilt(block_state_id).as_deref() != Some("none") {
        play_tilt_sound(level, pos, TILT_UP_SOUND);
    }
}

fn set_tilt(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32, tilt_value: &str) {
    let previous = tilt(block_state_id);
    let Some(tilted) = with_property(block_state_id, TILT, tilt_value) else {
        return;
    };
    level.set_block(
        pos,
        tilted,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    if causes_vibration(tilt_value) && previous.as_deref() != Some(tilt_value) {
        level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, None);
    }
}

// BigDripleafBlock.java:225-228: `Mth.randomBetween(random, 0.8F, 1.2F)`.
fn play_tilt_sound(level: &mut LevelCtx<'_, '_>, pos: BlockPos, sound: &str) {
    let pitch = level.level_random().next_float() * 0.4 + 0.8;
    level.play_block_sound_with_volume_pitch(pos, sound, 1.0, pitch);
}

// ---------------------------------------------------------------------------
// The stem
// ---------------------------------------------------------------------------

pub(super) fn stem_can_survive(
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt,
) -> bool {
    let below_state = block_state_at(below(pos));
    let above_state = block_state_at(above(pos));
    let footed = state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
        || tags.block_in_tag(below_state, "minecraft:supports_big_dripleaf");
    let topped = state_is_block(above_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
        || state_is_block(above_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID);
    footed && topped
}

pub(super) fn stem_can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    let below_state = level.block_state(below(pos));
    let above_state = level.block_state(above(pos));
    let topped = state_is_block(above_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID)
        || state_is_block(above_state, VANILLA_BLOCK_BIG_DRIPLEAF_ID);
    if !topped {
        return Some(false);
    }
    let footed = if state_is_block(below_state, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID) {
        true
    } else {
        block_tag_matches(below_state, SUPPORTS_BIG_DRIPLEAF_TAG, membership)
    };
    Some(footed)
}

pub(super) fn stem_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    let vertical =
        direction_to_neighbour == Direction::Down || direction_to_neighbour == Direction::Up;
    if vertical
        && stem_can_survive_at(level, pos, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        }) == Some(false)
    {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, STEM_TICK_DELAY, TickPriority::Normal);
        }
    }
    re_arm_water(level, pos, block_state_id);
    block_state_id
}

pub(super) fn stem_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if stem_can_survive_at(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) == Some(false)
    {
        level.destroy_block(pos, true);
    }
}
