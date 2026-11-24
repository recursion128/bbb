//! Cauldron contents, contact shapes, and stalactite drips.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{
    VanillaTickOverride, VANILLA_BLOCK_CAULDRON_ID, VANILLA_BLOCK_LAVA_CAULDRON_ID,
    VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, VANILLA_BLOCK_WATER_CAULDRON_ID,
};
use bbb_protocol::item_types::VanillaCauldronKind;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::{static_block_collision_boxes, Aabb};

use super::{block_type_of, cycle_property, default_block_state_id, int_property, tick_override};

const LEVEL: &str = "level";

// `LevelEvent.java:40-41`.
const SOUND_DRIP_LAVA_INTO_CAULDRON: i32 = 1046;
const SOUND_DRIP_WATER_INTO_CAULDRON: i32 = 1047;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DripFluid {
    Water,
    Lava,
}

#[must_use]
pub fn is_cauldron(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(tick_override)
        == Some(VanillaTickOverride::AbstractCauldronBlock)
}

#[must_use]
pub fn cauldron_kind(block_state_id: i32) -> Option<VanillaCauldronKind> {
    match block_type_of(block_state_id)? {
        VANILLA_BLOCK_CAULDRON_ID => Some(VanillaCauldronKind::Empty),
        VANILLA_BLOCK_WATER_CAULDRON_ID => Some(VanillaCauldronKind::Water),
        VANILLA_BLOCK_LAVA_CAULDRON_ID => Some(VanillaCauldronKind::Lava),
        VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID => Some(VanillaCauldronKind::PowderSnow),
        _ => None,
    }
}

#[must_use]
pub fn cauldron_level(block_state_id: i32) -> Option<i32> {
    int_property(block_state_id, LEVEL)
}

pub fn entity_inside_collision_boxes(state: i32) -> Option<Vec<Aabb>> {
    let content_height = match cauldron_kind(state)? {
        VanillaCauldronKind::Water | VanillaCauldronKind::PowderSnow => {
            let level = cauldron_level(state)?;
            if !(1..=3).contains(&level) {
                return None;
            }
            f64::from(6 + 3 * level) / 16.0
        }
        VanillaCauldronKind::Lava => 15.0 / 16.0,
        VanillaCauldronKind::Empty => return None,
    };
    let mut boxes = static_block_collision_boxes(state)?;
    boxes.push(Aabb {
        min_x: 2.0 / 16.0,
        min_y: 4.0 / 16.0,
        min_z: 2.0 / 16.0,
        max_x: 14.0 / 16.0,
        max_y: content_height,
        max_z: 14.0 / 16.0,
    });
    Some(boxes)
}

#[must_use]
pub fn empty_cauldron_state() -> i32 {
    default_block_state_id(VANILLA_BLOCK_CAULDRON_ID)
}

#[must_use]
pub fn default_water_cauldron_state() -> i32 {
    default_block_state_id(VANILLA_BLOCK_WATER_CAULDRON_ID)
}

#[must_use]
pub fn filled_water_cauldron_state() -> i32 {
    layered_cauldron_state(VANILLA_BLOCK_WATER_CAULDRON_ID, 3)
        .expect("water cauldron carries LEVEL 1..=3")
}

#[must_use]
pub fn lowered_fill_state(block_state_id: i32) -> Option<i32> {
    let new_level = int_property(block_state_id, LEVEL)? - 1;
    if new_level == 0 {
        Some(empty_cauldron_state())
    } else {
        with_property(block_state_id, LEVEL, &new_level.to_string())
    }
}

/// `LayeredCauldronBlock.lowerFillLevel` (`LayeredCauldronBlock.java:108-113`).
pub(crate) fn lower_fill_level(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let new_state = lowered_fill_state(state).expect("a layered cauldron carries LEVEL 1..=3");
    level.set_block(
        pos,
        new_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(new_state));
}

/// `LayeredCauldronBlock.entityInside` (`LayeredCauldronBlock.java:85-105`).
pub(crate) fn lower_fill_level_on_fire(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let state = if cauldron_kind(state) == Some(VanillaCauldronKind::PowderSnow) {
        layered_cauldron_state(
            VANILLA_BLOCK_WATER_CAULDRON_ID,
            cauldron_level(state).unwrap(),
        )
        .expect("a powder snow cauldron carries LEVEL 1..=3")
    } else {
        state
    };
    lower_fill_level(level, pos, state);
}

#[must_use]
pub fn cycled_fill_level(block_state_id: i32) -> Option<i32> {
    cycle_property(block_state_id, LEVEL)
}

#[must_use]
pub fn lava_cauldron_state() -> i32 {
    default_block_state_id(VANILLA_BLOCK_LAVA_CAULDRON_ID)
}

#[must_use]
pub fn filled_powder_snow_cauldron_state() -> i32 {
    layered_cauldron_state(VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, 3)
        .expect("powder snow cauldron carries LEVEL 1..=3")
}

#[must_use]
pub fn layered_cauldron_state(block_type_id: i32, level: i32) -> Option<i32> {
    with_property(
        default_block_state_id(block_type_id),
        LEVEL,
        &level.to_string(),
    )
}

/// `CauldronBlock.handlePrecipitation` (`CauldronBlock.java:33-52`) and
/// `LayeredCauldronBlock.handlePrecipitation` (`LayeredCauldronBlock.java:116-122`):
/// rain fills with a 5% chance, snow with 10%, a layered cauldron only from
/// its own precipitation kind.
pub(crate) fn handle_precipitation(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    snow: bool,
) {
    let new_state = match cauldron_kind(block_state_id) {
        Some(VanillaCauldronKind::Empty) if snow => Some(default_block_state_id(
            VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID,
        )),
        Some(VanillaCauldronKind::Empty) => Some(default_water_cauldron_state()),
        Some(VanillaCauldronKind::Water) if !snow => cycled_fill_level(block_state_id),
        Some(VanillaCauldronKind::PowderSnow) if snow => cycled_fill_level(block_state_id),
        _ => return,
    };
    let chance = if snow { 0.1 } else { 0.05 };
    if level.level_random().next_float() >= chance || cauldron_level(block_state_id) == Some(3) {
        return;
    }
    let Some(new_state) = new_state else {
        return;
    };
    level.set_block(
        pos,
        new_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(new_state));
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(tip) = super::pointed_dripstone::find_stalactite_tip_above_cauldron(level, pos) else {
        return;
    };
    let Some(fluid) = super::pointed_dripstone::cauldron_fill_fluid_type(level, tip) else {
        return;
    };
    let Some(new_state) = receive_drip(block_state_id, fluid) else {
        return;
    };

    level.set_block(
        pos,
        new_state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, pos, Some(new_state));
    level.level_event(
        match fluid {
            DripFluid::Water => SOUND_DRIP_WATER_INTO_CAULDRON,
            DripFluid::Lava => SOUND_DRIP_LAVA_INTO_CAULDRON,
        },
        pos,
        0,
    );
}

/// `canReceiveStalactiteDrip` (`CauldronBlock.java:55-57`,
/// `LayeredCauldronBlock.java:66-68`): a full water cauldron still qualifies.
pub(super) fn can_receive_drip(block_state_id: i32, fluid: DripFluid) -> bool {
    match block_type_of(block_state_id) {
        Some(VANILLA_BLOCK_CAULDRON_ID) => true,
        Some(VANILLA_BLOCK_WATER_CAULDRON_ID) => fluid == DripFluid::Water,
        _ => false,
    }
}

pub(super) fn receive_drip(block_state_id: i32, fluid: DripFluid) -> Option<i32> {
    match (block_type_of(block_state_id)?, fluid) {
        (VANILLA_BLOCK_CAULDRON_ID, DripFluid::Water) => {
            Some(default_block_state_id(VANILLA_BLOCK_WATER_CAULDRON_ID))
        }
        (VANILLA_BLOCK_CAULDRON_ID, DripFluid::Lava) => {
            Some(default_block_state_id(VANILLA_BLOCK_LAVA_CAULDRON_ID))
        }
        (VANILLA_BLOCK_WATER_CAULDRON_ID, DripFluid::Water) => {
            let level = int_property(block_state_id, LEVEL)?;
            if level >= 3 {
                return None;
            }
            with_property(block_state_id, LEVEL, &(level + 1).to_string())
        }
        (VANILLA_BLOCK_LAVA_CAULDRON_ID | VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID, _)
        | (VANILLA_BLOCK_WATER_CAULDRON_ID, DripFluid::Lava) => None,
        _ => None,
    }
}
