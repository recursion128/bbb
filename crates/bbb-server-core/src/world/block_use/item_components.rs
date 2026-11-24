use crate::block_behavior::{self, BlockPlacedBy};
use crate::block_entity::BlockEntityItemProjection;
use crate::connection::ConnectionId;
use crate::world::LevelId;
use bbb_protocol::block_entity_types::vanilla_block_entity_type_id_for_resource_id;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::{normalize_resource_location, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::world::{game_master_blocks, ServerWorld};

/// BlockItem.java:120-132; BlockItemStateProperties.java:48-63.
pub(in crate::world) fn apply_block_state(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: BlockPos,
    held: &RuntimeItemStack,
) {
    let dimension = dimension.into();
    let properties = match held.component_nbt(
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
        &world.item_stack_resolver(),
    ) {
        Ok(Some(NbtSummaryValue::Compound(properties))) => properties,
        Ok(_) => return,
        Err(error) => {
            tracing::warn!("block item state: {error}");
            return;
        }
    };
    let Some(placed) = world.block_state(dimension, pos) else {
        return;
    };
    let mut state = placed;
    for property in properties {
        let NbtSummaryValue::String(value) = property.value else {
            continue;
        };
        state = property
            .name
            .with_utf8(|name| {
                value
                    .with_utf8(|value| crate::fluid::with_property(state, name, value))
                    .flatten()
            })
            .flatten()
            .unwrap_or(state);
    }
    if state != placed {
        world.with_level(dimension, |level| {
            level.set_block(
                pos,
                state,
                crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        });
    }
}

pub(in crate::world) fn apply_custom_data(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: BlockPos,
    connection_id: Option<ConnectionId>,
    held: &RuntimeItemStack,
) -> bool {
    let dimension = dimension.into();
    let loaded = (|| -> bbb_server_types::Result<_> {
        let raw = held.component_nbt(
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            &world.item_stack_resolver(),
        )?;
        let Some(NbtSummaryValue::Compound(mut entries)) = raw else {
            return Ok(None);
        };
        let declared = entries
            .iter()
            .find(|entry| entry.name == "id")
            .and_then(|entry| {
                let NbtSummaryValue::String(id) = &entry.value else {
                    return None;
                };
                normalize_resource_location(id.try_to_string().ok()?)
                    .ok()
                    .and_then(|id| vanilla_block_entity_type_id_for_resource_id(&id))
            });
        let Some(entity) = world
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
        else {
            return Ok(None);
        };
        let type_id = entity.type_id();
        if declared != Some(type_id)
            || (bbb_protocol::block_entity_types::vanilla_block_entity_type_only_op_can_set_nbt(
                type_id,
            ) && !connection_id.is_some_and(|id| world.can_use_game_master_blocks(id)))
        {
            return Ok(None);
        }
        let previous_automatic = entity.command_block().map(|block| block.automatic());
        let previous_jukebox_item = world.playing_jukebox_item(&dimension, pos);
        entries.retain(|entry| entry.name != "id");
        let resolver = world.item_stack_resolver();
        let changed = world
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .expect("placed block entity remains present")
            .merge_item_custom_data_with_resolver(&entries, &resolver)?;
        Ok(changed.then_some((previous_automatic, previous_jukebox_item)))
    })();
    match loaded {
        Ok(Some((previous_automatic, previous_jukebox_item))) => {
            let state = world.with_level(dimension, |level| {
                game_master_blocks::loaded::after_load(level, pos, previous_automatic);
                level.block_state(pos)
            });
            world.finish_loaded_block_entity_side_effects(
                dimension,
                pos,
                state,
                previous_jukebox_item,
            );
            set_changed(world, dimension, pos);
            true
        }
        Ok(None) => false,
        Err(error) => {
            tracing::warn!("{error}");
            false
        }
    }
}

pub(super) fn set_placed_by(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: BlockPos,
    connection_id: ConnectionId,
    held: &RuntimeItemStack,
    item: &ItemStackSummary,
) {
    let dimension = dimension.into();
    let Some(state) = world.block_state(dimension, pos) else {
        return;
    };
    if super::game_master::is_game_master_block(state) {
        super::game_master::set_placed_by(world, dimension, pos, connection_id, item);
        return;
    }
    let block_entity_data = held
        .component_nbt(
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            &world.item_stack_resolver(),
        )
        .ok()
        .flatten();
    let block_entity_data = match &block_entity_data {
        Some(NbtSummaryValue::Compound(entries)) => Some(entries.as_slice()),
        _ => None,
    };
    let peaceful = world.admin.settings().difficulty == Difficulty::Peaceful;
    world.with_level(dimension, |level| {
        block_behavior::set_placed_by(
            level,
            pos,
            state,
            &BlockPlacedBy {
                block_entity_data,
                peaceful,
            },
        );
    });
}

pub(in crate::world) fn apply_components(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: BlockPos,
    held: &RuntimeItemStack,
) {
    let dimension = dimension.into();
    let resolver = world.item_stack_resolver();
    let Some(entity) = world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
    else {
        return;
    };
    if let Err(error) = entity
        .apply_components_from_item_stack(held, BlockEntityItemProjection::with_resolver(&resolver))
    {
        tracing::warn!("{error}");
    }
    set_changed(world, dimension, pos);
}

fn set_changed(world: &mut ServerWorld, dimension: impl Into<LevelId>, pos: BlockPos) {
    let dimension = dimension.into();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .set_changed(pos);
    if let Some(block) = world
        .block_state(dimension, pos)
        .and_then(vanilla_block_state_block_type_id)
    {
        world.with_level(dimension, |level| {
            level.update_neighbour_for_output_signal(pos, block);
        });
    }
}
