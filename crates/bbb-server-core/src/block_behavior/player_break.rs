//! Level bodies a player's break reaches beside the base removal: the
//! `Block.playerWillDestroy` subclass arms, the `BlockBehaviour.attack`
//! overrides and the `Block.playerDestroy` tails of `IceBlock` and
//! `TurtleEggBlock`. Facts read off the `Player` arrive as arguments.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VANILLA_BLOCK_CHAIN_COMMAND_BLOCK_ID,
    VANILLA_BLOCK_COMMAND_BLOCK_ID, VANILLA_BLOCK_DRAGON_EGG_ID, VANILLA_BLOCK_FROSTED_ICE_ID,
    VANILLA_BLOCK_ICE_ID, VANILLA_BLOCK_JIGSAW_ID, VANILLA_BLOCK_NOTE_BLOCK_ID,
    VANILLA_BLOCK_REPEATING_COMMAND_BLOCK_ID, VANILLA_BLOCK_STRUCTURE_BLOCK_ID,
    VANILLA_BLOCK_TEST_BLOCK_ID, VANILLA_BLOCK_TEST_INSTANCE_BLOCK_ID, VANILLA_BLOCK_TNT_ID,
    VANILLA_BLOCK_TURTLE_EGG_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_blocks_motion, FluidFamily};

use super::double_plant::{self, DoubleBlockHalf};
use super::level::SET_BLOCK_UPDATE_NONE;
use super::{
    air_block_state_id, below, block_state_fluid_state, block_type_of, bool_property,
    can_survive_override, default_block_state_id, direction_property, int_property, note_block,
    redstone_ore, state_liquid, string_property, vegetation, BlockInteractionPlayer,
    SET_BLOCK_UPDATE_CLIENTS,
};

/// `Block.UPDATE_SUPPRESS_DROPS | Block.UPDATE_ALL` (`Block.java:96`, `:104`).
const UPDATE_ALL_SUPPRESS_DROPS: i32 = 35;
const TURTLE_EGG_BREAK_SOUND: &str = "minecraft:entity.turtle.egg_break";

#[must_use]
pub fn is_game_master_block(block_state_id: i32) -> bool {
    matches!(
        block_type_of(block_state_id),
        Some(
            VANILLA_BLOCK_COMMAND_BLOCK_ID
                | VANILLA_BLOCK_CHAIN_COMMAND_BLOCK_ID
                | VANILLA_BLOCK_REPEATING_COMMAND_BLOCK_ID
                | VANILLA_BLOCK_STRUCTURE_BLOCK_ID
                | VANILLA_BLOCK_JIGSAW_ID
                | VANILLA_BLOCK_TEST_BLOCK_ID
                | VANILLA_BLOCK_TEST_INSTANCE_BLOCK_ID
        )
    )
}

#[must_use]
pub fn is_double_plant_block(block_state_id: i32) -> bool {
    double_plant::is_double_plant(block_state_id)
        || double_plant::is_tall_seagrass(block_state_id)
        || block_type_of(block_state_id).and_then(can_survive_override)
            == Some(VanillaCanSurviveOverride::PitcherCropBlock)
        || vegetation::is_small_dripleaf(block_state_id)
}

#[must_use]
pub fn is_ice_block(block_state_id: i32) -> bool {
    matches!(
        block_type_of(block_state_id),
        Some(VANILLA_BLOCK_ICE_ID | VANILLA_BLOCK_FROSTED_ICE_ID)
    )
}

#[must_use]
pub fn is_turtle_egg(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_TURTLE_EGG_ID)
}

#[must_use]
pub fn is_dragon_egg(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_DRAGON_EGG_ID)
}

#[must_use]
pub fn is_unstable_tnt(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_TNT_ID)
        && bool_property(block_state_id, "unstable") == Some(true)
}

pub(crate) fn prevent_drop_from_bottom_part(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<(BlockPos, i32)> {
    if double_plant::half(block_state_id)? != DoubleBlockHalf::Upper {
        return None;
    }
    let bottom_pos = below(pos);
    let bottom_state = level.block_state(bottom_pos);
    if block_type_of(bottom_state) != block_type_of(block_state_id)
        || double_plant::half(bottom_state) != Some(DoubleBlockHalf::Lower)
    {
        return None;
    }
    let fluid = block_state_fluid_state(bottom_state);
    let replacement = if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
        default_block_state_id(VANILLA_BLOCK_WATER_ID)
    } else {
        air_block_state_id()
    };
    level.set_block(
        bottom_pos,
        replacement,
        UPDATE_ALL_SUPPRESS_DROPS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    Some((bottom_pos, bottom_state))
}

pub(crate) fn remove_creative_bed_head(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<(BlockPos, i32)> {
    if string_property(block_state_id, "part")? != "foot" {
        return None;
    }
    let head_pos = relative(pos, direction_property(block_state_id, "facing")?);
    let head_state = level.block_state(head_pos);
    if block_type_of(head_state) != block_type_of(block_state_id)
        || string_property(head_state, "part").as_deref() != Some("head")
    {
        return None;
    }
    level.set_block(
        head_pos,
        air_block_state_id(),
        UPDATE_ALL_SUPPRESS_DROPS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    Some((head_pos, head_state))
}

pub(crate) fn destroy_creative_piston_base(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(facing) = direction_property(block_state_id, "facing") else {
        return;
    };
    let base_pos = relative(pos, opposite(facing));
    if super::piston_head::is_fitting_base(block_state_id, level.block_state(base_pos))
        == Some(true)
    {
        level.destroy_block(base_pos, false);
    }
}

pub(crate) fn crack_decorated_pot(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> i32 {
    let Some(cracked) = with_property(block_state_id, "cracked", "true") else {
        return block_state_id;
    };
    level.set_block(
        pos,
        cracked,
        SET_BLOCK_UPDATE_NONE,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    cracked
}

pub(crate) fn disarm_tripwire(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) {
    if let Some(disarmed) = with_property(block_state_id, "disarmed", "true") {
        level.set_block(
            pos,
            disarmed,
            SET_BLOCK_UPDATE_NONE,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
    level.game_event_from_entity(
        VanillaGameEvent::SHEAR,
        pos,
        player.source_entity_id(),
        None,
    );
}

pub(crate) fn attack(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    if block_type_of(block_state_id) == Some(VANILLA_BLOCK_NOTE_BLOCK_ID) {
        note_block::attack(level, pos, block_state_id, player);
        return true;
    }
    if redstone_ore::is_redstone_ore(block_state_id) {
        let _ = redstone_ore::interact(level, pos, block_state_id);
    }
    false
}

pub(crate) fn ice_player_destroy(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    prevents_ice_melting: bool,
) {
    if prevents_ice_melting {
        return;
    }
    if level.water_evaporates(pos) {
        level.remove_block(pos);
        return;
    }
    let below_state = level.block_state(below(pos));
    if block_state_blocks_motion(below_state) || state_liquid(below_state) {
        level.set_block(
            pos,
            super::melting::ice_melts_into(),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

pub(crate) fn decrease_turtle_eggs(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let pitch = 0.9 + level.level_random().next_float() * 0.2;
    level.play_block_sound_with_volume_pitch(pos, TURTLE_EGG_BREAK_SOUND, 0.7, pitch);
    let Some(eggs) = int_property(block_state_id, "eggs") else {
        return;
    };
    if eggs <= 1 {
        level.destroy_block(pos, false);
        return;
    }
    let Some(fewer) = with_property(block_state_id, "eggs", &(eggs - 1).to_string()) else {
        return;
    };
    level.set_block(
        pos,
        fewer,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.block_game_event(VanillaGameEvent::BLOCK_DESTROY, pos, Some(block_state_id));
    level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, block_state_id);
}
