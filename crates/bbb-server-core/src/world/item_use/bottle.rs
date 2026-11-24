//! `BottleItem.use` (`BottleItem.java:30-67`): a glass bottle takes dragon's
//! breath from a dragon's cloud around the player, else water from the cell
//! the player looks at.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_DRAGON_BREATH_ID, VANILLA_ITEM_GLASS_BOTTLE_ID};
use bbb_protocol::packets::{ItemStackSummary, SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::stats::StatKey;

use super::{single_item, ItemUseContext, ItemUseResult};
use crate::entity::area_effect_cloud::{area_effect_cloud, set_radius};
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::water_potion_stack;
use crate::world::pov_clip::{player_pov_clip, ClipFluid};
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let player = &world.levels[context.connection_id];
    let position = player.position();
    let player_entity_id = player.entity_id();
    let Ok(area) = player.bounding_box() else {
        return ItemUseResult::Pass;
    };
    // `:31-33`.
    let area = area.inflate(2.0, 2.0, 2.0);
    let entities = &world.level_by_id(&context.dimension).entities;
    let cloud = entities.ids_in_box_sections(area).into_iter().find(|&id| {
        let Some(entity) = entities.get(id) else {
            return false;
        };
        entity.is_alive()
            && entity.bounding_box().intersects(area)
            && area_effect_cloud(entity)
                .and_then(|cloud| cloud.owner())
                .and_then(|owner| entities.entity_id_for_uuid(owner))
                .and_then(|owner| entities.get(owner))
                .is_some_and(|owner| owner.entity_type_id() == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID)
    });
    if let Some(cloud) = cloud {
        // `:36-44`.
        if let Some(entity) = world
            .level_mut_by_id(&context.dimension)
            .entities
            .get_mut(cloud)
        {
            let radius = area_effect_cloud(entity).map_or(0.0, |cloud| cloud.radius());
            set_radius(entity, radius - 0.5);
        }
        play_fill_sound(
            world,
            context,
            "minecraft:item.bottle.fill_dragonbreath",
            None,
            events,
        );
        world
            .level_mut_by_id(&context.dimension)
            .event_sink
            .game_event_with_source_at(
                VanillaGameEvent::FLUID_PICKUP,
                position,
                Some(player_entity_id),
                None,
            );
        world.trigger_player_entity_interaction(
            context.connection_id,
            "minecraft:player_interacted_with_entity",
            context.dimension,
            cloud,
            &context.item,
            events,
        );
        turn_bottle_into(world, context, single_item(VANILLA_ITEM_DRAGON_BREATH_ID));
        return ItemUseResult::Success;
    }
    // `:46-65`.
    let Some(hit) = player_pov_clip(world, context.connection_id, ClipFluid::SourceOnly) else {
        return ItemUseResult::Pass;
    };
    // `:53`: `Level.mayInteract`.
    let in_border = world.player_may_interact_block(context.connection_id, hit.pos);
    let state = world.block_state_or_air(context.dimension, hit.pos);
    if !in_border || !block_state_fluid_state(state).is_same_family(FluidFamily::Water) {
        return ItemUseResult::Pass;
    }
    play_fill_sound(
        world,
        context,
        "minecraft:item.bottle.fill",
        Some(context.connection_id),
        events,
    );
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::FLUID_PICKUP,
            hit.pos,
            Some(player_entity_id),
            None,
        );
    turn_bottle_into(world, context, water_potion_stack());
    ItemUseResult::Success
}

/// `turnBottleIntoItem` (`:69-72`).
fn turn_bottle_into(world: &mut ServerWorld, context: &ItemUseContext, filled: ItemStackSummary) {
    world.levels[context.connection_id].award_stat_once(
        &mut world.scoreboard,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_GLASS_BOTTLE_ID),
    );
    world.create_filled_result(context.connection_id, context.hand, filled);
}

fn play_fill_sound(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    resource_id: &str,
    except: Option<crate::connection::ConnectionId>,
    events: &mut Vec<WorldEvent>,
) {
    let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
        return;
    };
    let position = world.levels[context.connection_id].position();
    world.play_player_sound(
        context.connection_id,
        except,
        SoundEventHolder::Reference { registry_id },
        SoundSource::Neutral,
        position,
        1.0,
        1.0,
        events,
    );
}
