use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, FlowingFluid};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_ACACIA_TRAPDOOR_ID,
    VANILLA_BLOCK_BAMBOO_TRAPDOOR_ID, VANILLA_BLOCK_BIRCH_TRAPDOOR_ID,
    VANILLA_BLOCK_CHERRY_TRAPDOOR_ID, VANILLA_BLOCK_COPPER_TRAPDOOR_ID,
    VANILLA_BLOCK_CRIMSON_TRAPDOOR_ID, VANILLA_BLOCK_DARK_OAK_TRAPDOOR_ID,
    VANILLA_BLOCK_EXPOSED_COPPER_TRAPDOOR_ID, VANILLA_BLOCK_IRON_TRAPDOOR_ID,
    VANILLA_BLOCK_JUNGLE_TRAPDOOR_ID, VANILLA_BLOCK_MANGROVE_TRAPDOOR_ID,
    VANILLA_BLOCK_OAK_TRAPDOOR_ID, VANILLA_BLOCK_OXIDIZED_COPPER_TRAPDOOR_ID,
    VANILLA_BLOCK_PALE_OAK_TRAPDOOR_ID, VANILLA_BLOCK_SPRUCE_TRAPDOOR_ID,
    VANILLA_BLOCK_WARPED_TRAPDOOR_ID, VANILLA_BLOCK_WAXED_COPPER_TRAPDOOR_ID,
    VANILLA_BLOCK_WAXED_EXPOSED_COPPER_TRAPDOOR_ID,
    VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_TRAPDOOR_ID,
    VANILLA_BLOCK_WAXED_WEATHERED_COPPER_TRAPDOOR_ID, VANILLA_BLOCK_WEATHERED_COPPER_TRAPDOOR_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use crate::redstone::{self, SignalSources};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    BlockInteractionPlayer, BlockStateAt, SET_BLOCK_UPDATE_CLIENTS,
};

const FACING: &str = "facing";
const HALF: &str = "half";
const OPEN: &str = "open";
const POWERED: &str = "powered";
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub(super) fn is_trapdoor(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::TrapDoorBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    (clicked_face, click_y, replacing_clicked, horizontal_direction): (
        Direction,
        f64,
        bool,
        Direction,
    ),
    block_state_at: BlockStateAt<'_>,
    sources: SignalSources<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);

    let mut state = default_block_state_id(block_type_id);
    if !is_trapdoor(state) {
        return None;
    }
    if !replacing_clicked && !matches!(clicked_face, Direction::Down | Direction::Up) {
        state = with_property(state, FACING, direction_serialized_name(clicked_face))?;
        state = with_property(state, HALF, if click_y > 0.5 { "top" } else { "bottom" })?;
    } else {
        if matches!(horizontal_direction, Direction::Down | Direction::Up) {
            return None;
        }
        state = with_property(
            state,
            FACING,
            direction_serialized_name(opposite(horizontal_direction)),
        )?;
        state = with_property(
            state,
            HALF,
            if clicked_face == Direction::Up {
                "bottom"
            } else {
                "top"
            },
        )?;
    }

    let powered = redstone::has_neighbor_signal(pos, &mut sources.emitting(block_state_at));
    if powered {
        state = with_property(state, OPEN, "true")?;
        state = with_property(state, POWERED, "true")?;
    }
    with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}

fn schedule_water_tick(level: &mut BlockShapeCtx<'_, '_, '_>, block_state_id: i32, pos: BlockPos) {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
}

#[derive(Clone, Copy)]
enum TrapdoorSoundFamily {
    Iron,
    Copper,
    CherryWood,
    BambooWood,
    NetherWood,
    Wooden,
}

fn trapdoor_sound(block_state_id: i32, opening: bool) -> Option<&'static str> {
    let family = match block_type_of(block_state_id)? {
        VANILLA_BLOCK_IRON_TRAPDOOR_ID => TrapdoorSoundFamily::Iron,
        VANILLA_BLOCK_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_EXPOSED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_WEATHERED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_OXIDIZED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_WAXED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_WAXED_EXPOSED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_WAXED_WEATHERED_COPPER_TRAPDOOR_ID
        | VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_TRAPDOOR_ID => TrapdoorSoundFamily::Copper,
        VANILLA_BLOCK_CHERRY_TRAPDOOR_ID => TrapdoorSoundFamily::CherryWood,
        VANILLA_BLOCK_BAMBOO_TRAPDOOR_ID => TrapdoorSoundFamily::BambooWood,
        VANILLA_BLOCK_CRIMSON_TRAPDOOR_ID | VANILLA_BLOCK_WARPED_TRAPDOOR_ID => {
            TrapdoorSoundFamily::NetherWood
        }
        VANILLA_BLOCK_OAK_TRAPDOOR_ID
        | VANILLA_BLOCK_SPRUCE_TRAPDOOR_ID
        | VANILLA_BLOCK_BIRCH_TRAPDOOR_ID
        | VANILLA_BLOCK_JUNGLE_TRAPDOOR_ID
        | VANILLA_BLOCK_ACACIA_TRAPDOOR_ID
        | VANILLA_BLOCK_DARK_OAK_TRAPDOOR_ID
        | VANILLA_BLOCK_PALE_OAK_TRAPDOOR_ID
        | VANILLA_BLOCK_MANGROVE_TRAPDOOR_ID => TrapdoorSoundFamily::Wooden,
        _ => return None,
    };
    Some(match (family, opening) {
        (TrapdoorSoundFamily::Iron, true) => "minecraft:block.iron_trapdoor.open",
        (TrapdoorSoundFamily::Iron, false) => "minecraft:block.iron_trapdoor.close",
        (TrapdoorSoundFamily::Copper, true) => "minecraft:block.copper_trapdoor.open",
        (TrapdoorSoundFamily::Copper, false) => "minecraft:block.copper_trapdoor.close",
        (TrapdoorSoundFamily::CherryWood, true) => "minecraft:block.cherry_wood_trapdoor.open",
        (TrapdoorSoundFamily::CherryWood, false) => "minecraft:block.cherry_wood_trapdoor.close",
        (TrapdoorSoundFamily::BambooWood, true) => "minecraft:block.bamboo_wood_trapdoor.open",
        (TrapdoorSoundFamily::BambooWood, false) => "minecraft:block.bamboo_wood_trapdoor.close",
        (TrapdoorSoundFamily::NetherWood, true) => "minecraft:block.nether_wood_trapdoor.open",
        (TrapdoorSoundFamily::NetherWood, false) => "minecraft:block.nether_wood_trapdoor.close",
        (TrapdoorSoundFamily::Wooden, true) => "minecraft:block.wooden_trapdoor.open",
        (TrapdoorSoundFamily::Wooden, false) => "minecraft:block.wooden_trapdoor.close",
    })
}

fn can_open_by_hand(block_state_id: i32) -> Option<bool> {
    block_type_of(block_state_id).map(|id| id != VANILLA_BLOCK_IRON_TRAPDOOR_ID)
}

/// Registered 26.1 trapdoor types differ only for IRON, which rejects both.
pub(super) fn can_open_by_wind_charge(block_state_id: i32) -> Option<bool> {
    can_open_by_hand(block_state_id)
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    if can_open_by_hand(block_state_id) != Some(true) {
        return false;
    }
    toggle(level, pos, block_state_id, player.source_entity_id())
}

pub(super) fn toggle(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) -> bool {
    let Some(open) = bool_property(block_state_id, OPEN) else {
        return false;
    };
    let opening = !open;
    let Some(updated) = with_property(block_state_id, OPEN, if opening { "true" } else { "false" })
    else {
        return false;
    };
    let Some(sound) = trapdoor_sound(updated, opening) else {
        return false;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    schedule_water_tick(&mut BlockShapeCtx::Live(level), updated, pos);
    let pitch = level.level_random().next_float() * 0.1 + 0.9;
    // TrapDoorBlock.java:119-125: the acting player predicted its own sound.
    level.play_block_sound_except(pos, sound, 1.0, pitch, source_entity_id);
    level.game_event_from_entity(
        if opening {
            VanillaGameEvent::BLOCK_OPEN
        } else {
            VanillaGameEvent::BLOCK_CLOSE
        },
        pos,
        source_entity_id,
        None,
    );
    true
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    let Some(powered) = bool_property(block_state_id, POWERED) else {
        return;
    };
    if signal == powered {
        return;
    }

    let value = if signal { "true" } else { "false" };
    let mut updated = block_state_id;
    let Some(open) = bool_property(updated, OPEN) else {
        return;
    };
    if open != signal {
        let Some(opened) = with_property(updated, OPEN, value) else {
            return;
        };
        let Some(sound) = trapdoor_sound(opened, signal) else {
            return;
        };
        updated = opened;
        level.play_block_sound(pos, sound);
        level.block_game_event(
            if signal {
                VanillaGameEvent::BLOCK_OPEN
            } else {
                VanillaGameEvent::BLOCK_CLOSE
            },
            pos,
            None,
        );
    }

    let Some(updated) = with_property(updated, POWERED, value) else {
        return;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    schedule_water_tick(&mut BlockShapeCtx::Live(level), updated, pos);
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    schedule_water_tick(level, block_state_id, pos);
    block_state_id
}
