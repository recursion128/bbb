//! `WindChargeItem.use` (`WindChargeItem.java:27-53`).

use bbb_protocol::packets::{SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;

use crate::entity::hurting_projectile::wind_charge::spawn_wind_charge;
use crate::entity::projectile_shoot::{shoot_from_rotation, ShotParameters, ShotSource};
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let player = world
        .levels
        .player(context.connection_id)
        .expect("connection presence was checked");
    let dimension = context.dimension;
    let position = player.position();
    let rotation = player.rotation();
    let owner = player.profile.uuid;
    let source = ShotSource {
        known_movement: player.known_movement,
        on_ground: player.on_ground(),
    };
    let has_infinite_materials = player.abilities.instabuild;

    // `WindChargeItem.java:29-38` spawns before its sound and uses eye Y
    // without the throwable-item constructor's `-0.1F` offset.
    if let Ok(eye) = player.eye_position() {
        let projectile_position = Vec3d {
            x: position.x,
            y: eye.y,
            z: position.z,
        };
        let store = &mut world.level_mut_by_id(&dimension).entities;
        shoot_from_rotation(
            store,
            source,
            ShotParameters {
                x_rot: rotation.x_rot,
                y_rot: rotation.y_rot,
                y_offset: 0.0,
                pow: 1.5,
                uncertainty: 1.0,
            },
            |store, shot| {
                let id = spawn_wind_charge(store, projectile_position, shot, Some(owner))?;
                crate::entity::projectile_weapon::queue_spawn_effects(
                    store,
                    id,
                    bbb_server_types::item_stack::RuntimeItemStack::from_summary(
                        context.item.clone(),
                    ),
                );
                Some(id)
            },
        );
    }

    // `WindChargeItem.java:41-50`.
    let pitch = 0.4 / (world.level_mut_by_id(&dimension).random.next_float() * 0.4 + 0.8);
    if let Some(registry_id) = vanilla_sound_event_registry_id("minecraft:entity.wind_charge.throw")
    {
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Neutral,
            position,
            volume: 0.5,
            pitch,
            seed: world.random.next_long(),
        };
        world.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }

    // `WindChargeItem.java:51-53`.
    let player = world
        .levels
        .player_mut(context.connection_id)
        .expect("connection presence was checked");
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    player
        .inventory
        .consume_in_hand(context.hand, 1, has_infinite_materials);
    ItemUseResult::Success
}
