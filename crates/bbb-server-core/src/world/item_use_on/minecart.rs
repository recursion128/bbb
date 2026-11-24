//! `MinecartItem.useOn` (`MinecartItem.java:28-68`): a cart set on the clicked
//! rail.

use bbb_protocol::entity_types::{
    vanilla_entity_type_id_for_resource_id, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
    VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
    VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{Direction, Vec3d};

use super::ItemUseOnContext;
use crate::block_behavior::rail::is_rail;
use crate::block_behavior::string_property;
use crate::entity::store::EntityStore;
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::spawn_egg::apply_default_stack_config;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::directions::relative;

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    let pos = context.pos();
    let state = world.block_state_or_air(context.dimension, pos);
    // `:32-34`.
    if !is_rail(state) {
        return false;
    }
    // Each minecart item shares its entity type's id (`Items.java`).
    let Some(entity_type_id) = context
        .stack
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| vanilla_entity_type_id_for_resource_id(item.resource_id))
    else {
        return false;
    };
    // `:37-45`.
    let slope = string_property(state, "shape").is_some_and(|shape| shape.starts_with("ascending"));
    let position = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.0625 + if slope { 0.5 } else { 0.0 },
        z: f64::from(pos.z) + 0.5,
    };
    let Some(entity_id) = spawn_cart(
        &mut world.level_mut_by_id(&context.dimension).entities,
        entity_type_id,
        position,
    ) else {
        return false;
    };
    apply_default_stack_config(
        world,
        Some(context.connection_id),
        context.dimension,
        entity_id,
        context.stack,
    );
    // `:61-66`.
    let below = world.block_state_or_air(context.dimension, relative(pos, Direction::Down));
    let player_entity_id = world.player_entity_id(context);
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::ENTITY_PLACE,
            pos,
            Some(player_entity_id),
            Some(below),
        );
    world.shrink_used_stack(context);
    true
}

/// `EntityType.create` for the cart types a minecart item names.
pub(in crate::world) fn spawn_cart(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
) -> Option<i32> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_MINECART_ID => crate::entity::spawn_minecart(store, position, 0.0),
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => {
            crate::entity::spawn_furnace_minecart(store, position, 0.0)
        }
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => {
            crate::entity::spawn_tnt_minecart(store, position, 0.0)
        }
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => {
            crate::entity::spawn_command_block_minecart(store, position, 0.0)
        }
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => {
            crate::entity::spawn_minecart_container(store, entity_type_id, position, 0.0)
        }
        _ => None,
    }
}
