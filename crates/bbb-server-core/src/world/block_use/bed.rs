use super::BlockUseContext;
use crate::block_behavior::{bed, direction_property, string_property};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::air_block_state_id;

pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let dimension = context.dimension;
    let mut pos = context.update.hit.pos;
    let mut state = context.block_state_id;
    let Some(facing) = direction_property(state, "facing") else {
        return true;
    };
    if string_property(state, "part").as_deref() != Some("head") {
        pos = relative(pos, facing);
        let Some(head) = world.block_state(dimension, pos) else {
            return true;
        };
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(head)
            != bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        {
            return true;
        }
        state = head;
    }
    let key = dimension;
    let Some(rule) = world.bed_rule_at(&key, pos) else {
        return true;
    };
    if rule.explodes {
        world.bed_rule_problem(context.connection_id, &rule, events);
        world.set_block(dimension, pos, air_block_state_id(), SET_BLOCK_UPDATE_ALL);
        let foot = relative(pos, opposite(facing));
        if world
            .block_state(dimension, foot)
            .is_some_and(|foot_state| {
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(foot_state)
                    == bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            })
        {
            world.set_block(dimension, foot, air_block_state_id(), SET_BLOCK_UPDATE_ALL);
        }
        world.explode_bad_respawn_point(dimension, pos, false, events);
    } else if bool_property(state, "occupied") == Some(true) {
        if !world.kick_sleeping_villager(&key, pos) {
            world.sleep_problem(
                context.connection_id,
                "block.minecraft.bed.occupied",
                events,
            );
        }
    } else if bed::is_bed(state) {
        world.start_player_sleep(context.connection_id, pos, rule, events);
    }
    // `BedBlock.java:105`, `:111`, `:118`.
    world.mark_use_item_on_server_swing();
    true
}
