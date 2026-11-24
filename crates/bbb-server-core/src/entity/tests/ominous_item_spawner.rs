use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::entity_synched_data::VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, SoundSource, Vec3d};

use crate::entity::item::{plain_stack, stack_count};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{spawn_ominous_item_spawner, tick_entities, ArrowPickup, EntityData};
use crate::level_event::VanillaGameEvent;

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn spawner_data(store: &EntityStore, id: i32) -> crate::entity::OminousItemSpawnerData {
    match store.get(id).expect("spawner is live").data() {
        EntityData::OminousItemSpawner(data) => data.clone(),
        other => panic!("expected ominous item spawner, found {other:?}"),
    }
}

fn run_until_removed(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..=120 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("ominous item spawner did not finish by its maximum delay")
}

#[test]
fn ominous_item_spawner_plays_its_sound_spawns_its_item_and_dies() {
    let mut store = EntityStore::new(71, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let stack = plain_stack("minecraft:stick", 3).unwrap();
    let id = spawn_ominous_item_spawner(&mut store, position(), stack.clone()).unwrap();
    let delay = spawner_data(&store, id).spawn_item_after_ticks();
    assert!((60..=120).contains(&delay));
    assert_eq!(
        store.get(id).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID
    );
    assert!(store.get(id).unwrap().is_ignoring_block_triggers());
    assert!(!store.get(id).unwrap().is_pickable());
    assert_eq!(store.get(id).unwrap().gravity(), 0.0);
    store.drain_lifecycle_events();
    run_until_removed(&mut store, &mut level, id);
    let sound_events = store.drain_lifecycle_events();
    assert!(sound_events.iter().any(|event| matches!(event, EntityLifecycleEvent::Sound { sound } if sound.source == SoundSource::Neutral && sound.position == (Vec3d { x: 0.5, y: 64.5, z: 0.5 }) && sound.volume == 1.0 && sound.pitch == 1.0)));
    let item_id = store
        .ids()
        .into_iter()
        .find(|&candidate| {
            store.get(candidate).unwrap().entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID
        })
        .unwrap();
    let EntityData::Item(item) = store.get(item_id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(item.item(), &stack);
    assert!(level
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 3021
            && event.pos == BlockPos { x: 0, y: 64, z: 0 }
            && event.data == 1));
    let raised = level.world.positioned_game_events_for_test(DIMENSION);
    assert!(raised.iter().any(
        |event| event.recorded.game_event == VanillaGameEvent::ENTITY_PLACE
            && event.recorded.source_entity_id == Some(item_id)
    ));
    assert!(raised.iter().any(
        |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
            && event.recorded.source_entity_id == Some(id)
    ));
}

#[test]
fn ominous_item_spawner_arrow_uses_down_dispense_shot_and_owner() {
    let mut store = EntityStore::new(72, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = spawn_ominous_item_spawner(
        &mut store,
        position(),
        plain_stack("minecraft:arrow", 5).unwrap(),
    )
    .unwrap();
    let owner = store.get(id).unwrap().uuid();
    run_until_removed(&mut store, &mut level, id);

    let arrow_id = store
        .ids()
        .into_iter()
        .find(|&candidate| {
            store.get(candidate).unwrap().entity_type_id() == VANILLA_ENTITY_TYPE_ARROW_ID
        })
        .unwrap();
    let arrow = store.get(arrow_id).unwrap();
    let EntityData::Arrow(data) = arrow.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(data.pickup(), ArrowPickup::Allowed);
    assert_eq!(stack_count(data.item()), 1);
    assert!(arrow.delta_movement().y < 0.0);
    assert!(level
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 3021));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.source_entity_id == Some(arrow_id)));
}

#[test]
fn ominous_item_spawner_firework_is_shot_at_an_angle_by_its_owner() {
    let mut store = EntityStore::new(76, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = spawn_ominous_item_spawner(
        &mut store,
        position(),
        plain_stack("minecraft:firework_rocket", 1).unwrap(),
    )
    .unwrap();
    let owner = store.get(id).unwrap().uuid();
    run_until_removed(&mut store, &mut level, id);

    let rocket_id = store
        .ids()
        .into_iter()
        .find(|&candidate| {
            matches!(
                store.get(candidate).unwrap().data(),
                EntityData::FireworkRocket(_)
            )
        })
        .unwrap();
    let EntityData::FireworkRocket(rocket) = store.get(rocket_id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(rocket.projectile().owner(), Some(owner));
    assert!(rocket.shot_at_angle());
    let events = level.world.drain_level_events_for_test(DIMENSION);
    for kind in [1004, 3021] {
        assert!(events.iter().any(|event| event.event_type == kind));
    }
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.source_entity_id == Some(rocket_id)));
}

#[test]
fn ominous_item_spawner_accepts_prototype_potion() {
    let mut store = EntityStore::new(73, EntityIdCounter::new());
    let id = spawn_ominous_item_spawner(
        &mut store,
        position(),
        plain_stack("minecraft:splash_potion", 1).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        store.get(id).unwrap().data(),
        EntityData::OminousItemSpawner(_)
    ));
}

#[test]
fn ominous_item_spawner_save_restore_and_tracking_keep_item_and_delay() {
    let mut store = EntityStore::new(74, EntityIdCounter::new());
    let stack = plain_stack("minecraft:stick", 2).unwrap();
    let id = spawn_ominous_item_spawner(&mut store, position(), stack.clone()).unwrap();
    let before = spawner_data(&store, id);
    let values = non_default_synched_values(store.get(id).unwrap());
    assert!(values.iter().any(|value| {
        value.data_id == VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID
            && matches!(&value.value, EntityDataValueKind::ItemStack(item) if item.count == 2)
    }));

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(75, EntityIdCounter::new());
    let restored_id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
    let after = spawner_data(&restored, restored_id);
    assert_eq!(after.item(), &stack);
    assert_eq!(
        after.spawn_item_after_ticks(),
        before.spawn_item_after_ticks()
    );
}

#[test]
fn ominous_item_spawner_spawn_and_canonical_reload_move_through_solid_blocks() {
    let mut spawned = EntityStore::new(78, EntityIdCounter::new());
    let id = spawn_ominous_item_spawner(
        &mut spawned,
        position(),
        plain_stack("minecraft:stick", 2).unwrap(),
    )
    .unwrap();
    let saved = write_entity_chunk(&spawned, (0, 0)).unwrap();
    let mut reloaded = EntityStore::new(79, EntityIdCounter::new());
    let restored_id = read_entity_chunk(&mut reloaded, &saved).unwrap()[0];
    for (mut store, id) in [(spawned, id), (reloaded, restored_id)] {
        let mut entity = store.detach(id).unwrap();
        let mut level = NativeLevel::new();
        let velocity = Vec3d {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        };
        let delta = Vec3d {
            x: 0.5,
            y: -1.0,
            z: 0.25,
        };
        entity.set_delta_movement(velocity);
        entity.on_ground = true;
        entity.stuck_speed_multiplier = Vec3d {
            x: 0.2,
            y: 0.3,
            z: 0.4,
        };
        store.drain_lifecycle_events();
        for mover in [
            crate::entity::MoverType::Self_,
            crate::entity::MoverType::Piston,
        ] {
            let before = entity.position();
            entity.horizontal_collision = true;
            entity.vertical_collision = true;
            entity.vertical_collision_below = true;
            entity.minor_horizontal_collision = true;
            level.with_level(|level| {
                crate::entity::move_entity(&mut entity, &mut store, level, mover, delta)
            });
            assert_eq!(
                entity.position(),
                Vec3d {
                    x: before.x + delta.x,
                    y: before.y + delta.y,
                    z: before.z + delta.z,
                }
            );
            assert!(entity.no_physics());
            assert_eq!(entity.delta_movement(), velocity);
            assert!(entity.on_ground());
            assert_eq!(
                entity.stuck_speed_multiplier,
                Vec3d {
                    x: 0.2,
                    y: 0.3,
                    z: 0.4
                }
            );
            assert!(!entity.horizontal_collision());
            assert!(!entity.vertical_collision());
            assert!(!entity.vertical_collision_below());
            assert!(!entity.minor_horizontal_collision());
        }
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .is_empty());
    }
}

/// `EggItem` (`EggItem.java:16`) also backs `blue_egg` and `brown_egg`
/// (`Items.java:1371`, `:1374`), so the spawner throws them as eggs.
#[test]
fn ominous_item_spawner_throws_colored_eggs() {
    for egg in ["minecraft:blue_egg", "minecraft:brown_egg"] {
        let mut store = EntityStore::new(77, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        let id = spawn_ominous_item_spawner(&mut store, position(), plain_stack(egg, 1).unwrap())
            .unwrap();
        run_until_removed(&mut store, &mut level, id);
        assert!(
            store
                .ids()
                .into_iter()
                .any(|candidate| store.get(candidate).unwrap().entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EGG_ID),
            "{egg}"
        );
    }
}

#[test]
fn ominous_item_spawner_fire_charge_is_shot_at_full_speed() {
    let mut store = EntityStore::new(73, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = spawn_ominous_item_spawner(
        &mut store,
        position(),
        plain_stack("minecraft:fire_charge", 1).unwrap(),
    )
    .unwrap();
    run_until_removed(&mut store, &mut level, id);

    let fireball = store
        .ids()
        .into_iter()
        .filter_map(|candidate| store.get(candidate))
        .find(|entity| {
            entity.entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID
        })
        .expect("the fire charge is shot");
    let movement = fireball.delta_movement();
    let speed = bbb_server_types::geometry::Vec3Ops::length_sqr(movement).sqrt();
    assert!(
        movement.y < 0.0 && (0.8..1.3).contains(&speed),
        "{movement:?}"
    );
}
