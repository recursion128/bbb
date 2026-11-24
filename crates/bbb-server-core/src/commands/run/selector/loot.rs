//! `register("predicate")`'s filter (EntitySelectorOptions.java:460-479): the
//! current datapack's loot predicate under the `SELECTOR` parameter set.

use crate::loot_runtime::{
    matches_conditions, param_sets, LootBlockEntityDataAt, LootBlockStateAt, LootEntities,
    LootEntity, LootOrigin, LootParams, LootRuntimeReason,
};
use bbb_server_datapack::loot::LootElementValue;

use super::*;

pub(super) fn predicate_holds<'a>(
    id: &str,
    inverted: bool,
    entity: CommandDamageEntityRef<'a>,
    world: &EntityReads<'a>,
) -> Result<bool, Refusal> {
    let Some(tables) = world.loot else {
        return Ok(false);
    };
    // `reloadableRegistries().lookup().get(id)` answering empty is false
    // whatever the inversion (`:464-466`).
    let Some(condition) = tables
        .registry("minecraft:predicate")
        .and_then(|registry| registry.elements.iter().find(|element| element.id == id))
        .and_then(|element| match &element.value {
            LootElementValue::Predicate(condition) => Some(condition),
            _ => None,
        })
    else {
        return Ok(false);
    };
    let [x, y, z] = entity.position();
    let facts;
    let this_entity = match entity {
        CommandDamageEntityRef::Player(player) => {
            facts = world.player_loot_facts(player);
            LootEntity::Facts(&facts)
        }
        CommandDamageEntityRef::Stored(stored) => {
            LootEntity::Stored(world.stored_loot_entity(stored))
        }
    };
    let loaded_block_state = world.blocks(entity.dimension()).map(|_| {
        move |pos| {
            let blocks = world
                .blocks(entity.dimension())
                .expect("served predicate chunks");
            (blocks.is_in_world_bounds(pos) && blocks.has_chunk_at(pos))
                .then(|| blocks.block_state_at(pos))
        }
    });
    let block_entity_data = world.blocks(entity.dimension()).map(|_| {
        move |pos| {
            let blocks = world
                .blocks(entity.dimension())
                .expect("served predicate chunks");
            blocks
                .block_entity_loot_data(pos, Some(world.resolver()))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        }
    });
    let player_slots = |uuid, slot| world.player_item_slot(uuid, slot);
    let player_nbt = |uuid| world.player_loot_nbt(uuid);
    let item_environment =
        world.predicate_environment(entity.dimension(), &player_slots, &player_nbt);
    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
        world.loot_vehicle_slots(requested, selected)
    };
    let params = LootParams {
        dimension: Some(entity.dimension().as_str()),
        item_environment: match &item_environment {
            Some(environment) => Some(environment),
            None => None,
        },
        origin: Some(LootOrigin { x, y, z }),
        entities: LootEntities {
            this_entity,
            ..LootEntities::default()
        },
        weather: Some(world.weather(entity.dimension())),
        vehicle_slots: Some(&vehicle_slots),
        level: loaded_block_state
            .as_ref()
            .map(|read| read as &LootBlockStateAt<'_>),
        block_entity_data: block_entity_data
            .as_ref()
            .map(|read| read as &LootBlockEntityDataAt<'_>),
        ..LootParams::new(param_sets::SELECTOR)
    };
    let random = world.random(entity.dimension());
    matches_conditions(tables, id, std::slice::from_ref(condition), &params, random)
        .map(|held| inverted ^ held)
        .map_err(|error| unanswerable(error.to_string()))
}
