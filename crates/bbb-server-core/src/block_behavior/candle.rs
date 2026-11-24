//! `CandleBlock` (`net/minecraft/world/level/block/CandleBlock.java`).
//!
//!
//! # A candle whose support is broken does not pop off
//!
//!
//! # What is not here
//!
//! * `CandleBlock.placeLiquid` (`:150-164`), which mutates the live level.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_types::{VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, FlowingFluid};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::fluid::FluidFamily;

use super::footing;
use super::level::SET_BLOCK_UPDATE_IMMEDIATE;
use super::{
    block_type_of, bool_property, can_support_center, can_support_center_at, can_survive_override,
    default_block_state_id, get_state_for_placement_override, int_property, with_property,
    BlockStateAt,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;

const CANDLES: &str = "candles";
/// `AbstractCandleBlock.LIT` (`AbstractCandleBlock.java:29`).
const LIT: &str = "lit";
/// `CandleBlock.WATERLOGGED` (`CandleBlock.java:42`).
const WATERLOGGED: &str = "waterlogged";
/// `Block.UPDATE_ALL_IMMEDIATE` (`Block.java:106`).
const UPDATE_ALL_IMMEDIATE: i32 = SET_BLOCK_UPDATE_ALL | SET_BLOCK_UPDATE_IMMEDIATE;
/// `SoundEvents.CANDLE_EXTINGUISH` (`SoundEvents.java:296`).
pub const EXTINGUISH_SOUND: &str = "minecraft:block.candle.extinguish";
/// `SoundEvents.FLINTANDSTEEL_USE` (`SoundEvents.java:635`).
pub const FLINT_AND_STEEL_SOUND: &str = "minecraft:item.flintandsteel.use";
/// `SoundEvents.FIRECHARGE_USE` (`SoundEvents.java:619`).
pub const FIRE_CHARGE_SOUND: &str = "minecraft:item.firecharge.use";

pub(super) fn state_for_placement(
    block_type_id: i32,
    replaced_state_id: i32,
    source_water: bool,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::CandleBlock)
    {
        return None;
    }
    if block_type_of(replaced_state_id)? == block_type_id {
        let candles = int_property(replaced_state_id, CANDLES)?;
        let next = match candles {
            1..=3 => candles + 1,
            4 => 1,
            _ => return None,
        };
        return with_property(replaced_state_id, CANDLES, &next.to_string());
    }
    with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if source_water { "true" } else { "false" },
    )
}

/// Whether this slice owns a stored state's survival.
///
#[must_use]
pub fn is_candle(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::CandleBlock)
}

///
/// `None` carries `Block.canSupportCenter`'s gap: a below-state whose block has
/// a dynamic shape has no cached support answer, so this refuses to guess one.
#[must_use]
pub fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> Option<bool> {
    can_support_center(block_state_at(below(pos)), Direction::Up)
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let support_pos = below(pos);
    Some(can_support_center_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Up,
    ))
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

/// `AbstractCandleBlock.LIT` on a stored candle or candle cake.
#[must_use]
pub fn is_lit(block_state_id: i32) -> bool {
    bool_property(block_state_id, LIT) == Some(true)
}

#[must_use]
pub fn can_light(block_state_id: i32) -> bool {
    is_candle(block_state_id)
        && bool_property(block_state_id, LIT) == Some(false)
        && bool_property(block_state_id, WATERLOGGED) == Some(false)
}

#[must_use]
pub fn candle_cake_can_light(block_state_id: i32) -> bool {
    footing::is_candle_cake(block_state_id) && bool_property(block_state_id, LIT) == Some(false)
}

fn set_lit(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    lit: bool,
    flags: i32,
) -> bool {
    let Some(next) = with_property(block_state_id, LIT, if lit { "true" } else { "false" }) else {
        return false;
    };
    level.set_block(pos, next, flags, crate::neighbor_updater::UPDATE_LIMIT)
}

pub(crate) fn extinguish(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    set_lit(level, pos, block_state_id, false, UPDATE_ALL_IMMEDIATE);
    level.play_block_sound(pos, EXTINGUISH_SOUND);
    level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
}

pub(crate) fn light_with_flint(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    let pitch = level.level_random().next_float() * 0.4 + 0.8;
    // `FlintAndSteelItem.java:49` plays with the player, who predicted it.
    level.play_block_sound_except(pos, FLINT_AND_STEEL_SOUND, 1.0, pitch, source_entity_id);
    set_lit(level, pos, block_state_id, true, UPDATE_ALL_IMMEDIATE);
    level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
}

pub(crate) fn light_with_fire_charge(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    let pitch = {
        let random = level.level_random();
        (random.next_float() - random.next_float()) * 0.2 + 1.0
    };
    level.play_block_sound_with_volume_pitch(pos, FIRE_CHARGE_SOUND, 1.0, pitch);
    set_lit(level, pos, block_state_id, true, SET_BLOCK_UPDATE_ALL);
    level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, pos, source_entity_id, None);
}

/// `AbstractCandleBlock.onProjectileHit` (`AbstractCandleBlock.java:45-49`)
/// for a burning projectile.
pub(super) fn light_by_projectile(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if can_light(block_state_id) || candle_cake_can_light(block_state_id) {
        set_lit(level, pos, block_state_id, true, UPDATE_ALL_IMMEDIATE);
    }
}
