//! `EnderEyeItem.useOn` (`EnderEyeItem.java:36-70`): an eye set into an end
//! portal frame, and the portal a complete ring opens.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_END_PORTAL_FRAME_ID, VANILLA_BLOCK_END_PORTAL_ID};
use bbb_protocol::packets::{BlockPos, Direction};

use super::ItemUseOnContext;
use crate::block_behavior::{default_block_state_id, string_property, SET_BLOCK_UPDATE_CLIENTS};
use crate::fluid::with_property;
use crate::world::{LevelId, ServerWorld, WorldEvent};
use bbb_server_types::block_queries::direction_serialized_name;

/// `LevelEvent.END_PORTAL_FRAME_FILL`.
const END_PORTAL_FRAME_FILL: i32 = 1503;
/// `LevelEvent.SOUND_END_PORTAL_SPAWN`.
const END_PORTAL_SPAWN: i32 = 1038;

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let dimension = context.dimension;
    let pos = context.pos();
    let state = world.block_state_or_air(dimension, pos);
    // `:35-38`.
    if !is_frame(state, false) {
        return false;
    }
    let Some(filled) = with_property(state, "eye", "true") else {
        return false;
    };
    // `:44-50`.
    let _ = world.push_live_entities_up(&dimension, state, filled, pos, events);
    world.set_block(dimension, pos, filled, SET_BLOCK_UPDATE_CLIENTS);
    world.with_level(dimension, |level| {
        level.update_neighbour_for_output_signal(pos, VANILLA_BLOCK_END_PORTAL_FRAME_ID);
    });
    world.shrink_used_stack(context);
    world
        .level_mut_by_id(&dimension)
        .event_sink
        .level_event(END_PORTAL_FRAME_FILL, pos, 0);
    // `:51-64`.
    if let Some(corner) = complete_ring_interior(world, dimension, pos) {
        let portal = default_block_state_id(VANILLA_BLOCK_END_PORTAL_ID);
        for x in 0..3 {
            for z in 0..3 {
                let cell = BlockPos {
                    x: corner.x + x,
                    z: corner.z + z,
                    ..corner
                };
                world.with_level(dimension, |level| level.destroy_block(cell, true));
                world.set_block(dimension, cell, portal, SET_BLOCK_UPDATE_CLIENTS);
            }
        }
        world
            .level_mut_by_id(&dimension)
            .event_sink
            .global_level_event(
                END_PORTAL_SPAWN,
                BlockPos {
                    x: corner.x + 1,
                    z: corner.z + 1,
                    ..corner
                },
                0,
            );
    }
    true
}

fn is_frame(state: i32, with_eye: bool) -> bool {
    vanilla_block_state_block_type_id(state) == Some(VANILLA_BLOCK_END_PORTAL_FRAME_ID)
        && string_property(state, "eye").as_deref() == Some(if with_eye { "true" } else { "false" })
}

/// `EndPortalFrameBlock.getOrCreatePortalShape().find(level, pos)`
/// (`EndPortalFrameBlock.java:84-130`): twelve filled frames around a 3x3
/// interior, each facing into it. Answers the interior's north-west cell.
fn complete_ring_interior(
    world: &mut ServerWorld,
    dimension: LevelId,
    filled: BlockPos,
) -> Option<BlockPos> {
    // Every interior whose ring the filled frame can sit on.
    let mut corners = Vec::new();
    for offset in 0..3 {
        corners.push((filled.x - offset, filled.z + 1));
        corners.push((filled.x - offset, filled.z - 3));
        corners.push((filled.x + 1, filled.z - offset));
        corners.push((filled.x - 3, filled.z - offset));
    }
    corners.into_iter().find_map(|(x, z)| {
        let corner = BlockPos { x, y: filled.y, z };
        ring(corner)
            .into_iter()
            .all(|(cell, facing)| {
                let state = world.block_state_or_air(dimension, cell);
                is_frame(state, true)
                    && string_property(state, "facing").as_deref()
                        == Some(direction_serialized_name(facing))
            })
            .then_some(corner)
    })
}

/// The twelve ring cells around the interior at `corner` and the facing each
/// frame needs.
fn ring(corner: BlockPos) -> Vec<(BlockPos, Direction)> {
    let at = |x, z| BlockPos {
        x: corner.x + x,
        z: corner.z + z,
        ..corner
    };
    (0..3)
        .flat_map(|i| {
            [
                (at(i, -1), Direction::South),
                (at(i, 3), Direction::North),
                (at(-1, i), Direction::East),
                (at(3, i), Direction::West),
            ]
        })
        .collect()
}
