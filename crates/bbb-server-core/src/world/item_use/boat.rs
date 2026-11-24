//! `BoatItem.use` (`BoatItem.java:31-87`): a boat or raft set where the
//! player's look meets a block or any fluid.

use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_id_for_resource_id,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::entity::minecart::is_minecart;
use crate::entity::{spawn_boat, spawn_chest_boat};
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::spawn_egg::apply_default_stack_config;
use crate::world::entity_placement::{any_entity_in, has_block_collision};
use crate::world::pov_clip::{player_pov_clip, view_vector, ClipFluid};
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    // Each boat item shares its entity type's id (`Items.java`).
    let Some(entity_type_id) = context
        .item
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| vanilla_entity_type_id_for_resource_id(item.resource_id))
    else {
        return ItemUseResult::Pass;
    };
    // `:33-36`.
    let Some(hit) = player_pov_clip(world, context.connection_id, ClipFluid::Any) else {
        return ItemUseResult::Pass;
    };
    let player = &world.levels[context.connection_id];
    let player_entity_id = player.entity_id();
    let y_rot = player.rotation().y_rot;
    let (Ok(eye), Ok(player_box)) = (player.eye_position(), player.bounding_box()) else {
        return ItemUseResult::Pass;
    };
    // `:38-50`: a pickable entity whose box holds the player's eye refuses.
    let view = view_vector(player.rotation().x_rot, y_rot);
    let reach = player_box
        .expand_towards(view.scale(5.0))
        .inflate(1.0, 1.0, 1.0);
    let entities = &world.level_by_id(&context.dimension).entities;
    let eye_inside = entities
        .first_entity_in_box(reach, |entity| {
            let bounds = entity.bounding_box();
            entity.id() != player_entity_id
                && entity.is_pickable()
                && (bounds.min_x..bounds.max_x).contains(&eye.x)
                && (bounds.min_y..bounds.max_y).contains(&eye.y)
                && (bounds.min_z..bounds.max_z).contains(&eye.z)
        })
        .is_some();
    if eye_inside {
        return ItemUseResult::Pass;
    }
    // `:53-61`: `level.noCollision(boat, boat.getBoundingBox())`, whose entity
    // half is what `AbstractBoat.canCollideWith` accepts.
    let Some([width, height]) = vanilla_entity_type_dimensions(entity_type_id) else {
        return ItemUseResult::Fail;
    };
    let bounds = Aabb::of_entity(hit.location, width, height);
    if has_block_collision(world, context.dimension, bounds)
        || any_entity_in(world, context.dimension, bounds, |entity| {
            entity.can_be_collided_with()
                || is_minecart(entity.data())
                || entity
                    .living()
                    .is_some_and(|living| !living.is_dead_or_dying())
        })
    {
        return ItemUseResult::Fail;
    }
    // `:63-69`.
    let store = &mut world.level_mut_by_id(&context.dimension).entities;
    let Some(entity_id) = spawn_boat(store, entity_type_id, hit.location, y_rot)
        .or_else(|| spawn_chest_boat(store, entity_type_id, hit.location, y_rot))
    else {
        return ItemUseResult::Fail;
    };
    apply_default_stack_config(
        world,
        Some(context.connection_id),
        context.dimension,
        entity_id,
        &context.item,
    );
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source_at(
            VanillaGameEvent::ENTITY_PLACE,
            hit.location,
            Some(player_entity_id),
            None,
        );
    let player = &mut world.levels[context.connection_id];
    let infinite = player.abilities.instabuild;
    player.inventory.consume_in_hand(context.hand, 1, infinite);
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    ItemUseResult::Success
}
