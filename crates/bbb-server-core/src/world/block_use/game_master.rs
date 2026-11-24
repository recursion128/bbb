use crate::block_behavior::command_block;
use crate::block_entity::block_entity_custom_only_packet_with_resolver;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_JIGSAW_ID, VANILLA_BLOCK_STRUCTURE_BLOCK_ID};
use bbb_protocol::packets::{BlockPos, ItemStackSummary};

use super::BlockUseContext;
use crate::world::{send, ServerWorld, WorldEvent};

pub(super) fn is_game_master_block(state: i32) -> bool {
    command_block::is_command_block(state)
        || matches!(
            vanilla_block_state_block_type_id(state),
            Some(VANILLA_BLOCK_STRUCTURE_BLOCK_ID | VANILLA_BLOCK_JIGSAW_ID)
        )
}

pub(super) fn set_placed_by(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: BlockPos,
    connection_id: ConnectionId,
    item: &ItemStackSummary,
) {
    let dimension = dimension.into();
    let Some(state) = world.block_state(dimension, pos) else {
        return;
    };
    if !command_block::is_command_block(state)
        && vanilla_block_state_block_type_id(state) != Some(VANILLA_BLOCK_STRUCTURE_BLOCK_ID)
    {
        return;
    }
    let name = world.levels[connection_id].profile.name.clone();
    let block_entity_data =
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID;
    let has_data = item
        .component_patch
        .added_type_ids
        .contains(&block_entity_data)
        && !item
            .component_patch
            .removed_type_ids
            .contains(&block_entity_data);
    let track_output = matches!(
        world
            .admin
            .game_rule_value("minecraft:send_command_feedback"),
        Some(bbb_protocol::game_rules::GameRuleValue::Boolean(true))
    );
    world.with_level(dimension, |level| {
        if command_block::is_command_block(state) {
            if !has_data {
                if let Some(block) = level.command_block_mut(pos) {
                    block.set_track_output(track_output);
                }
                command_block::set_automatic(
                    level,
                    pos,
                    state,
                    command_block::mode(state) == bbb_protocol::packets::CommandBlockMode::Sequence,
                );
            }
            command_block::neighbor_changed(level, pos, state);
        } else if let Some(block) = level
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(crate::block_entity::BlockEntity::structure_block_mut)
        {
            block.set_author(name);
            level.chunks.block_entities_mut().set_changed(pos);
        }
    });
}

pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if !world.can_use_game_master_blocks(context.connection_id) {
        return false;
    }
    let Some(entity) = world
        .level_by_id(&context.dimension)
        .chunks
        .block_entities()
        .get(context.update.hit.pos)
    else {
        return false;
    };
    if entity.command_block().is_some() {
        let packet =
            block_entity_custom_only_packet_with_resolver(entity, &world.item_stack_resolver());
        let packet = match packet {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!("block-entity custom update: {error}");
                return false;
            }
        };
        if let Some(packet) = packet {
            events.push(send(
                context.connection_id,
                ClientboundPlayPacket::BlockEntityData(packet),
            ));
        }
        return true;
    }
    entity.structure_block().is_some() || entity.jigsaw().is_some()
}
