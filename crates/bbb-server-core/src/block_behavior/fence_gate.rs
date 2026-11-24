use super::{BlockShapeCtx, BlockTags, LevelCtx};

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_ACACIA_FENCE_GATE_ID,
    VANILLA_BLOCK_BAMBOO_FENCE_GATE_ID, VANILLA_BLOCK_BIRCH_FENCE_GATE_ID,
    VANILLA_BLOCK_CHERRY_FENCE_GATE_ID, VANILLA_BLOCK_CRIMSON_FENCE_GATE_ID,
    VANILLA_BLOCK_DARK_OAK_FENCE_GATE_ID, VANILLA_BLOCK_JUNGLE_FENCE_GATE_ID,
    VANILLA_BLOCK_MANGROVE_FENCE_GATE_ID, VANILLA_BLOCK_OAK_FENCE_GATE_ID,
    VANILLA_BLOCK_PALE_OAK_FENCE_GATE_ID, VANILLA_BLOCK_SPRUCE_FENCE_GATE_ID,
    VANILLA_BLOCK_WARPED_FENCE_GATE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};

use crate::redstone::{self, SignalSources};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, get_state_for_placement_override, with_property,
    BlockInteractionPlayer, BlockStateAt, SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_IMMEDIATE,
};

const FACING: &str = "facing";
const IN_WALL: &str = "in_wall";
const OPEN: &str = "open";
const POWERED: &str = "powered";
const WALLS_TAG: &str = "minecraft:walls";

pub(super) fn is_fence_gate(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::FenceGateBlock)
}

fn horizontal_axis(direction: Direction) -> Option<char> {
    match direction {
        Direction::North | Direction::South => Some('z'),
        Direction::West | Direction::East => Some('x'),
        Direction::Down | Direction::Up => None,
    }
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
    sources: SignalSources<'_>,
) -> Option<i32> {
    let is_wall = |state| tags.block_in_tag(state, WALLS_TAG);
    let state = default_block_state_id(block_type_id);
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::FenceGateBlock)
    {
        return None;
    }
    let powered = redstone::has_neighbor_signal(pos, &mut sources.emitting(&mut *block_state_at));
    let in_wall = match horizontal_axis(horizontal_direction)? {
        'z' => {
            let west = block_state_at(relative(pos, Direction::West));
            block_type_of(west)?;
            if is_wall(west) {
                true
            } else {
                let east = block_state_at(relative(pos, Direction::East));
                block_type_of(east)?;
                is_wall(east)
            }
        }
        'x' => {
            let north = block_state_at(relative(pos, Direction::North));
            block_type_of(north)?;
            if is_wall(north) {
                true
            } else {
                let south = block_state_at(relative(pos, Direction::South));
                block_type_of(south)?;
                is_wall(south)
            }
        }
        _ => return None,
    };
    let state = with_property(
        state,
        FACING,
        direction_serialized_name(horizontal_direction),
    )?;
    let state = with_property(state, OPEN, if powered { "true" } else { "false" })?;
    let state = with_property(state, POWERED, if powered { "true" } else { "false" })?;
    with_property(state, IN_WALL, if in_wall { "true" } else { "false" })
}

#[derive(Clone, Copy)]
enum FenceGateSoundFamily {
    CherryWood,
    BambooWood,
    NetherWood,
    Default,
}

fn fence_gate_sound(block_state_id: i32, opening: bool) -> Option<&'static str> {
    let family = match block_type_of(block_state_id)? {
        VANILLA_BLOCK_CHERRY_FENCE_GATE_ID => FenceGateSoundFamily::CherryWood,
        VANILLA_BLOCK_BAMBOO_FENCE_GATE_ID => FenceGateSoundFamily::BambooWood,
        VANILLA_BLOCK_CRIMSON_FENCE_GATE_ID | VANILLA_BLOCK_WARPED_FENCE_GATE_ID => {
            FenceGateSoundFamily::NetherWood
        }
        VANILLA_BLOCK_OAK_FENCE_GATE_ID
        | VANILLA_BLOCK_SPRUCE_FENCE_GATE_ID
        | VANILLA_BLOCK_BIRCH_FENCE_GATE_ID
        | VANILLA_BLOCK_JUNGLE_FENCE_GATE_ID
        | VANILLA_BLOCK_ACACIA_FENCE_GATE_ID
        | VANILLA_BLOCK_DARK_OAK_FENCE_GATE_ID
        | VANILLA_BLOCK_PALE_OAK_FENCE_GATE_ID
        | VANILLA_BLOCK_MANGROVE_FENCE_GATE_ID => FenceGateSoundFamily::Default,
        _ => return None,
    };
    Some(match (family, opening) {
        (FenceGateSoundFamily::CherryWood, true) => "minecraft:block.cherry_wood_fence_gate.open",
        (FenceGateSoundFamily::CherryWood, false) => "minecraft:block.cherry_wood_fence_gate.close",
        (FenceGateSoundFamily::BambooWood, true) => "minecraft:block.bamboo_wood_fence_gate.open",
        (FenceGateSoundFamily::BambooWood, false) => "minecraft:block.bamboo_wood_fence_gate.close",
        (FenceGateSoundFamily::NetherWood, true) => "minecraft:block.nether_wood_fence_gate.open",
        (FenceGateSoundFamily::NetherWood, false) => "minecraft:block.nether_wood_fence_gate.close",
        (FenceGateSoundFamily::Default, true) => "minecraft:block.fence_gate.open",
        (FenceGateSoundFamily::Default, false) => "minecraft:block.fence_gate.close",
    })
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    let player_direction = player.horizontal_direction;
    let Some(open) = bool_property(block_state_id, OPEN) else {
        return false;
    };
    let updated = if open {
        with_property(block_state_id, OPEN, "false")
    } else {
        let turned =
            if direction_property(block_state_id, FACING) == Some(opposite(player_direction)) {
                with_property(
                    block_state_id,
                    FACING,
                    direction_serialized_name(player_direction),
                )
            } else {
                Some(block_state_id)
            };
        turned.and_then(|state| with_property(state, OPEN, "true"))
    };
    let Some(updated) = updated else {
        return false;
    };
    let opens = !open;
    let Some(sound) = fence_gate_sound(block_state_id, opens) else {
        return false;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS | SET_BLOCK_UPDATE_IMMEDIATE,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    let pitch = level.level_random().next_float() * 0.1 + 0.9;
    // FenceGateBlock.java:162-164: the acting player predicted its own sound.
    level.play_block_sound_except(pos, sound, 1.0, pitch, player.source_entity_id());
    level.game_event_from_entity(
        if opens {
            VanillaGameEvent::BLOCK_OPEN
        } else {
            VanillaGameEvent::BLOCK_CLOSE
        },
        pos,
        player.source_entity_id(),
        None,
    );
    true
}

pub(super) fn toggle_from_explosion(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let Some(open) = bool_property(state, OPEN) else {
        return;
    };
    let Some(updated) = with_property(state, OPEN, if open { "false" } else { "true" }) else {
        return;
    };
    let Some(sound) = fence_gate_sound(state, !open) else {
        return;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    let pitch = level.level_random().next_float() * 0.1 + 0.9;
    level.play_block_sound_with_volume_pitch(pos, sound, 1.0, pitch);
    level.block_game_event(
        if open {
            VanillaGameEvent::BLOCK_CLOSE
        } else {
            VanillaGameEvent::BLOCK_OPEN
        },
        pos,
        Some(state),
    );
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let has_power = level.has_neighbor_signal(pos);
    let Some(powered) = bool_property(block_state_id, POWERED) else {
        return;
    };
    if powered == has_power {
        return;
    }
    let Some(open) = bool_property(block_state_id, OPEN) else {
        return;
    };
    let value = if has_power { "true" } else { "false" };
    let Some(updated) = with_property(block_state_id, POWERED, value)
        .and_then(|state| with_property(state, OPEN, value))
    else {
        return;
    };

    // Unlike Door/TrapDoor, `:192-202` writes before comparing the old OPEN.
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    if open == has_power {
        return;
    }
    let Some(sound) = fence_gate_sound(block_state_id, has_power) else {
        return;
    };
    level.play_block_sound(pos, sound);
    level.block_game_event(
        if has_power {
            VanillaGameEvent::BLOCK_OPEN
        } else {
            VanillaGameEvent::BLOCK_CLOSE
        },
        pos,
        None,
    );
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(facing_axis) = direction_property(block_state_id, FACING).and_then(horizontal_axis)
    else {
        return block_state_id;
    };
    let Some(update_axis) = horizontal_axis(direction_to_neighbour) else {
        return block_state_id;
    };
    if facing_axis == update_axis {
        return block_state_id;
    }

    let opposite_state = level.block_state(relative(pos, opposite(direction_to_neighbour)));
    let neighbour_is_wall = level.block_state_in_tag(neighbour_state, WALLS_TAG);
    let in_wall = if neighbour_is_wall {
        true
    } else {
        level.block_state_in_tag(opposite_state, WALLS_TAG)
    };
    with_property(
        block_state_id,
        IN_WALL,
        if in_wall { "true" } else { "false" },
    )
    .unwrap_or(block_state_id)
}
