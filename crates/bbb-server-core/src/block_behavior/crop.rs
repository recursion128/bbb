//! `CropBlock` (`net/minecraft/world/level/block/CropBlock.java`) and its three
//! 26.1 subclasses that change behaviour.
//!
//!
//! The membership test is the generated `can_survive_override` class tag,
//! which names `CropBlock` for exactly those five blocks; the per-subclass
//! differences are then transcribed one by one from the subclass files into
//! [`CropKind`], because a class override is not something any generated fact
//! carries.

use super::{block_tag_matches, BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VANILLA_BLOCK_BEETROOTS_ID, VANILLA_BLOCK_TORCHFLOWER_CROP_ID,
    VANILLA_BLOCK_TORCHFLOWER_ID,
};
use bbb_protocol::packets::BlockPos;

use super::vegetation::VegetationSupport;
use super::{
    below, block_state_with_int, block_type_of, can_survive_override, default_block_state_id,
    int_property, offset, state_is_block, SET_BLOCK_UPDATE_CLIENTS,
};

/// The raw brightness `randomTick` demands before a crop grows at all
/// (`CropBlock.java:80`).
pub const GROWTH_LIGHT_THRESHOLD: i32 = 9;

/// The raw brightness `hasSufficientLight` demands for a crop to survive
/// (`CropBlock.java:150`). Note it is one lower than
/// [`GROWTH_LIGHT_THRESHOLD`], so there is a one-level band in which a crop
/// stays put without ever ageing.
pub const SURVIVE_LIGHT_THRESHOLD: i32 = 8;

/// The numerator of the growth draw (`CropBlock.java:84`):
/// `random.nextInt((int)(25.0F / growthSpeed) + 1) == 0`.
pub const GROWTH_DRAW_NUMERATOR: f32 = 25.0;

const AGE: &str = "age";
const GROWS_CROPS_TAG: &str = "minecraft:grows_crops";
pub(super) const SUPPORTS_CROPS_TAG: &str = "minecraft:supports_crops";

/// The three distinct `CropBlock` shapes 26.1 registers.
///
/// The variants differ only in the three things the subclasses override:
/// `getMaxAge`, `getStateForAge` and the gate in front of `randomTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropKind {
    Plain,
    Beetroot,
    Torchflower,
}

impl CropKind {
    /// `CropBlock.getMaxAge` (`CropBlock.java:57-59`) resolved through the two
    /// overrides.
    #[must_use]
    pub const fn max_age(self) -> i32 {
        match self {
            Self::Plain => 7,
            Self::Beetroot => 3,
            Self::Torchflower => 2,
        }
    }

    /// The `random.nextInt(3) != 0` gate `BeetrootBlock` and
    /// `TorchflowerCropBlock` put in front of the inherited `randomTick`.
    #[must_use]
    pub const fn draws_before_growth(self) -> bool {
        match self {
            Self::Plain => false,
            Self::Beetroot | Self::Torchflower => true,
        }
    }
}

/// Whether a stored state is one of the five `CropBlock` blocks.
#[must_use]
pub fn is_crop(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        can_survive_override(block_type_id) == Some(VanillaCanSurviveOverride::CropBlock)
    })
}

/// The subclass shape a stored state belongs to.
#[must_use]
pub fn crop_kind(block_state_id: i32) -> Option<CropKind> {
    if !is_crop(block_state_id) {
        return None;
    }
    match block_type_of(block_state_id)? {
        VANILLA_BLOCK_BEETROOTS_ID => Some(CropKind::Beetroot),
        VANILLA_BLOCK_TORCHFLOWER_CROP_ID => Some(CropKind::Torchflower),
        _ => Some(CropKind::Plain),
    }
}

#[must_use]
pub fn age(block_state_id: i32) -> Option<i32> {
    int_property(block_state_id, AGE)
}

#[must_use]
pub fn state_for_age(block_state_id: i32, new_age: i32) -> Option<i32> {
    let block_type_id = block_type_of(block_state_id)?;
    if crop_kind(block_state_id)? == CropKind::Torchflower
        && new_age == CropKind::Torchflower.max_age()
    {
        return Some(default_block_state_id(VANILLA_BLOCK_TORCHFLOWER_ID));
    }
    block_state_with_int(block_type_id, AGE, new_age)
}

#[must_use]
pub fn is_max_age(block_state_id: i32) -> bool {
    match (age(block_state_id), crop_kind(block_state_id)) {
        (Some(age), Some(kind)) => age >= kind.max_age(),
        _ => false,
    }
}

///
/// The rest of the bonemeal triple is not here: `isBonemealSuccess` is a
/// constant true (`:178-181`) and `performBonemeal` (`:183-186`) draws
/// `Mth.nextInt(random, 2, 5)` through `getBonemealAgeIncrease` (`:96-98`),
/// which belongs with the bonemeal item slice.
#[must_use]
pub fn is_valid_bonemeal_target(block_state_id: i32) -> bool {
    is_crop(block_state_id) && !is_max_age(block_state_id)
}

///
/// The nine blocks under the 3×3 footprint each score 0, 1 or 3 — 3 once the
/// farmland below is moist at all — and the eight that are not directly
/// beneath the crop are quartered before being added to a base of 1. The
/// result is then halved once if the crop has same-block neighbours on both
/// horizontal axes, or, failing that, on any diagonal.
///
/// The arithmetic is Java `float` throughout, so `f32` here is the same
/// sequence of roundings, and the diagonal branch really is an `else`: a crop
/// in a full field is halved once, never twice.
#[must_use]
pub(crate) fn growth_speed(
    level: &LevelCtx<'_, '_>,
    block_type_id: i32,
    pos: BlockPos,
) -> Option<f32> {
    let mut speed = 1.0_f32;
    let below_pos = below(pos);

    for xx in -1..=1 {
        for zz in -1..=1 {
            let mut block_speed = 0.0_f32;
            let state = level.block_state(offset(below_pos, xx, 0, zz));
            if level.block_state_in_tag(state, GROWS_CROPS_TAG) {
                block_speed = 1.0;
                // `getValueOrElse(FarmlandBlock.MOISTURE, 0)`: any block in the
                // tag without the property scores as dry.
                if int_property(state, super::farmland::MOISTURE).unwrap_or(0) > 0 {
                    block_speed = 3.0;
                }
            }
            if xx != 0 || zz != 0 {
                block_speed /= 4.0;
            }
            speed += block_speed;
        }
    }

    let north = offset(pos, 0, 0, -1);
    let south = offset(pos, 0, 0, 1);
    let west = offset(pos, -1, 0, 0);
    let east = offset(pos, 1, 0, 0);
    let same = |at: BlockPos| state_is_block(level.block_state(at), block_type_id);
    let horizontal = same(west) || same(east);
    let vertical = same(north) || same(south);
    if horizontal && vertical {
        speed /= 2.0;
    } else {
        let diagonal = same(offset(west, 0, 0, -1))
            || same(offset(east, 0, 0, -1))
            || same(offset(east, 0, 0, 1))
            || same(offset(west, 0, 0, 1));
        if diagonal {
            speed /= 2.0;
        }
    }

    Some(speed)
}

///
/// The write is flag 2, not `UPDATE_ALL`: a crop that ages tells the clients
/// and runs no neighbour fan-out (`:85`).
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(kind) = crop_kind(block_state_id) else {
        return;
    };
    if kind.draws_before_growth() && level.level_random().next_int(3) == 0 {
        return;
    }
    if level.raw_brightness(pos, 0) < GROWTH_LIGHT_THRESHOLD {
        return;
    }
    let Some(current_age) = age(block_state_id) else {
        return;
    };
    if current_age >= kind.max_age() {
        return;
    }
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    let Some(speed) = growth_speed(level, block_type_id, pos) else {
        return;
    };
    // Java truncates the float division towards zero before adding one.
    let bound = (GROWTH_DRAW_NUMERATOR / speed) as i32 + 1;
    if level.level_random().next_int(bound) == 0 {
        if let Some(grown) = state_for_age(block_state_id, current_age + 1) {
            level.set_block(
                pos,
                grown,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

#[must_use]
pub fn can_survive<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if level.raw_brightness(pos, 0).unwrap_or(0) < SURVIVE_LIGHT_THRESHOLD {
        Some(false)
    } else {
        Some(block_tag_matches(
            level.block_state(below(pos)),
            SUPPORTS_CROPS_TAG,
            membership,
        ))
    }
}

/// The same body on the shape seam, where the light read is the one thing a
/// stored-state view may not answer.
pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let brightness = level.raw_brightness(pos, 0)?;
    if brightness < SURVIVE_LIGHT_THRESHOLD {
        return Some(false);
    }
    super::vegetation::may_place_on(VegetationSupport::Crops, pos, level.tags(), &mut |at| {
        level.block_state(at)
    })
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if can_survive(level, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) == Some(false)
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}
