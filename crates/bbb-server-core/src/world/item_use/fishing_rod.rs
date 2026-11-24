//! `FishingRodItem.use` (`FishingRodItem.java:22-62`): cast a hook, or pull
//! the one out back in with whatever it caught.

use bbb_protocol::packets::{BlockPos, SoundEvent, SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::{custom_stat, StatKey};

use super::{ItemUseContext, ItemUseResult};
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::entity::fishing_bobber::{owned_bobber, retrieve, spawn_fishing_bobber};
use crate::entity::EntityData;
use crate::item_durability::{process_value_effects_in, ValueContext};
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::LootEntity;
use crate::world::{ServerWorld, WorldEvent};

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let player = &world.levels[context.connection_id];
    let owner = player.profile.uuid;
    let player_entity_id = player.entity_id();
    let position = player.position();
    let rotation = player.rotation();
    let eye_y = player.eye_position().map_or(position.y + 1.62, |eye| eye.y);
    let at_player = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    let hook = owned_bobber(&world.level_by_id(&context.dimension).entities, owner);
    if let Some(hook_id) = hook {
        // `:23-27`.
        let damage = reel_in(world, context, hook_id, events);
        if damage > 0 {
            world.hurt_and_break_in_hand(context.connection_id, context.hand, damage, events);
        }
        play_sound(
            world,
            context,
            "minecraft:entity.fishing_bobber.retrieve",
            1.0,
            events,
        );
        world
            .level_mut_by_id(&context.dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::ITEM_INTERACT_FINISH,
                at_player,
                Some(player_entity_id),
                None,
            );
        return ItemUseResult::Success;
    }
    // `:40-57`.
    play_sound(
        world,
        context,
        "minecraft:entity.fishing_bobber.throw",
        0.5,
        events,
    );
    let (lure_speed, luck) = fishing_bonuses(world, context);
    let store = &mut world.level_mut_by_id(&context.dimension).entities;
    if let Some(id) = spawn_fishing_bobber(
        store,
        owner,
        position,
        eye_y,
        rotation.y_rot,
        rotation.x_rot,
        luck,
        lure_speed,
    ) {
        if crate::enchantment_weapon_effects::projectile_spawned_conditions(
            store.weapon_effects(),
            &context.item,
        )
        .next()
        .is_some()
        {
            // Projectile.java:234-235: the input has no equipped-item break callback.
            let rod = std::mem::replace(
                world.levels[context.connection_id]
                    .inventory
                    .item_in_hand_mut(context.hand),
                bbb_server_types::item_stack::RuntimeItemStack::empty(),
            );
            let rod = world.projectile_spawned_effects(&context.dimension, id, rod, events);
            world.update_hand_stack(context.connection_id, context.hand, rod);
        }
    }
    if let Some(item_id) = context.item.item_id {
        world.levels[context.connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::ITEM_INTERACT_START,
            at_player,
            Some(player_entity_id),
            None,
        );
    ItemUseResult::Success
}

/// `FishingHook.retrieve` with the owner's side of it: `fishing_rod_hooked`
/// and `FISH_CAUGHT`. Answers the rod damage.
fn reel_in(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    hook_id: i32,
    events: &mut Vec<WorldEvent>,
) -> i32 {
    let owner = world.levels[context.connection_id].profile.uuid;
    let rod_stack = crate::world::using_item::runtime_item_in_hand(
        &world.levels[context.connection_id],
        context.hand,
    )
    .clone();
    let rod = loot_tool_for_runtime_stack(&rod_stack, &world.datapack_tables.enchantment_registry);
    let biome = world
        .level_by_id(&context.dimension)
        .entities
        .get(hook_id)
        .map(|hook| hook.block_position())
        .and_then(|pos| {
            let id = world
                .level_by_id(&context.dimension)
                .chunks
                .biome_id_at(pos.x, pos.y, pos.z)?;
            Some((
                pos,
                world
                    .registries
                    .registry_element_id(BIOME_REGISTRY, id)?
                    .to_owned(),
            ))
        });
    let biome_at = |pos: BlockPos| {
        biome
            .as_ref()
            .filter(|(at, _)| *at == pos)
            .map(|(_, id)| id.clone())
    };
    let hook = world.level_by_id(&context.dimension).entities.get(hook_id);
    let hooked = hook.and_then(|hook| match hook.data() {
        EntityData::FishingBobber(data)
            if data.hooked_entity_id().is_some() || data.nibble() > 0 =>
        {
            Some(data.hooked_entity_id().unwrap_or(hook_id))
        }
        _ => None,
    });
    let entity = hooked.and_then(|id| world.fishing_rod_hooked_entity(context.connection_id, id));
    let outcome = world.with_level_and_entities(context.dimension, |level, store| {
        let owner = level.player(owner)?;
        let mut hook = store.detach(hook_id)?;
        let outcome = retrieve(&mut hook, store, level, &owner, &rod, &biome_at);
        let hooked_item = outcome
            .pulled_entity_id
            .and_then(|id| store.get(id))
            .and_then(|entity| match entity.data() {
                EntityData::Item(item) => Some(item.item().clone()),
                _ => None,
            });
        store.attach(hook);
        Some((outcome, hooked_item))
    });
    let Some((outcome, hooked_item)) = outcome else {
        return 0;
    };
    if outcome.retrieved && hooked.is_some() {
        let items: Vec<_> = hooked_item
            .into_iter()
            .chain(outcome.caught.iter().cloned())
            .collect();
        world.trigger_fishing_rod_hooked(context.connection_id, &rod_stack, &items, entity, events);
    }
    if outcome.fish_caught > 0 {
        world.levels[context.connection_id].award_stat(
            &mut world.scoreboard,
            custom_stat("minecraft:fish_caught"),
            outcome.fish_caught,
        );
    }
    outcome.rod_damage
}

/// `EnchantmentHelper.getFishingTimeReduction` / `getFishingLuckBonus`
/// (`EnchantmentHelper.java:403-413`) as the cast reads them (`:49-50`).
fn fishing_bonuses(world: &ServerWorld, context: &ItemUseContext) -> (i32, i32) {
    let fisher = world.session_loot_view(context.connection_id);
    let player = &world.levels[context.connection_id];
    let origin = player.position();
    let value_level = world.value_effect_level(&player.dimension);
    let bonus = |table| {
        let effects = ServerWorld::resolve_value_effects(&context.item, table);
        let fisher = ValueContext::Entity {
            this: fisher
                .as_ref()
                .map_or(LootEntity::Opaque, LootEntity::Facts),
            origin,
        };
        process_value_effects_in(fisher, 0.0, &effects, &world.random, &value_level)
    };
    let lure_speed =
        (bonus(&world.datapack_tables.enchantment_fishing_time_reduction).max(0.0) * 20.0) as i32;
    let luck = (bonus(&world.datapack_tables.enchantment_fishing_luck_bonus) as i32).max(0);
    (lure_speed, luck)
}

fn play_sound(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    resource_id: &str,
    volume: f32,
    events: &mut Vec<WorldEvent>,
) {
    let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
        return;
    };
    let pitch = 0.4 / (world.random.next_float() * 0.4 + 0.8);
    let sound = SoundEvent {
        sound: SoundEventHolder::Reference { registry_id },
        source: SoundSource::Neutral,
        position: world.levels[context.connection_id].position(),
        volume,
        pitch,
        seed: world.random.next_long(),
    };
    world.broadcast_level_sound_by_key(&context.dimension, &sound, None, events);
}
