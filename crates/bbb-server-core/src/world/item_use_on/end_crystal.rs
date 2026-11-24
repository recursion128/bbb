//! `EndCrystalItem.useOn` (`EndCrystalItem.java:23-57`): a crystal set on
//! obsidian or bedrock.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_OBSIDIAN_ID};
use bbb_protocol::packets::{Direction, Vec3d};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::Aabb;

use super::ItemUseOnContext;
use crate::entity::spawn_end_crystal;
use crate::level_event::VanillaGameEvent;
use crate::world::entity_placement::any_entity_in;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    let pos = context.pos();
    let block = vanilla_block_state_block_type_id(world.block_state_or_air(context.dimension, pos));
    if !matches!(
        block,
        Some(VANILLA_BLOCK_OBSIDIAN_ID | VANILLA_BLOCK_BEDROCK_ID)
    ) {
        return false;
    }
    let above = relative(pos, Direction::Up);
    if !block_state_is_air(world.block_state_or_air(context.dimension, above)) {
        return false;
    }
    let (x, y, z) = (f64::from(above.x), f64::from(above.y), f64::from(above.z));
    let area = Aabb {
        min_x: x,
        min_y: y,
        min_z: z,
        max_x: x + 1.0,
        max_y: y + 2.0,
        max_z: z + 1.0,
    };
    if any_entity_in(world, context.dimension, area, |_| true) {
        return false;
    }
    let position = Vec3d {
        x: x + 0.5,
        y,
        z: z + 0.5,
    };
    if spawn_end_crystal(
        &mut world.level_mut_by_id(&context.dimension).entities,
        position,
        false,
    )
    .is_none()
    {
        return false;
    }
    let player_entity_id = world.player_entity_id(context);
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::ENTITY_PLACE,
            above,
            Some(player_entity_id),
            None,
        );
    // `:49-52`: `EnderDragonFight.tryRespawn`.
    world.try_respawn_dragon(&context.dimension);
    world.shrink_used_stack(context);
    true
}
