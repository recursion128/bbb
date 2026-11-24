use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::level_event::VanillaGameEvent;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{spawn_llama_spit, tick_entities, EntityData, LlamaSpitData};

fn flying_llama_spit() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_llama_spit(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn llama_spit_data(store: &EntityStore, id: i32) -> LlamaSpitData {
    match store.get(id).expect("entity is live").data() {
        EntityData::LlamaSpit(data) => *data,
        other => panic!("expected llama spit, found {other:?}"),
    }
}

#[test]
fn a_spawned_llama_spit_carries_motion_and_projectile_defaults() {
    let (store, _, id) = flying_llama_spit();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert!(!llama_spit_data(&store, id).has_been_shot());
}

fn land_llama_spit(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("llama spit never hit the ground");
}

#[test]
fn a_llama_spit_falls_and_discards_on_block_hit() {
    let (mut store, mut level, id) = flying_llama_spit();
    land_llama_spit(&mut store, &mut level, id);
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
    let events: Vec<_> = level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .into_iter()
        .map(|event| event.recorded.game_event)
        .collect();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == VanillaGameEvent::PROJECTILE_SHOOT)
            .count(),
        1
    );
    assert!(events.contains(&VanillaGameEvent::PROJECTILE_LAND));
}

#[test]
fn a_llama_spit_discards_when_spawned_inside_water() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level.set(pos(0, 63, 0), state_id("minecraft:water"));
    let id = spawn_llama_spit(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 63.5,
            z: 0.5,
        },
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
}

#[test]
fn a_llama_spit_crosses_a_portal_without_entering_it() {
    let mut store = EntityStore::new(13, EntityIdCounter::new());
    let id = spawn_llama_spit(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
    )
    .unwrap();
    let mut level = NativeLevel::new();
    level.set(
        BlockPos { x: 1, y: 68, z: 0 },
        state_id("minecraft:nether_portal"),
    );

    level.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get(id).unwrap();
    assert!(entity.position().x > 2.0);
    assert!(entity.portal_process().is_none());
}

#[test]
fn mixed_air_and_solid_overlap_does_not_take_the_all_non_air_discard_branch() {
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level.set(pos(1, 64, 0), state_id("minecraft:stone"));
    let id = spawn_llama_spit(
        &mut store,
        Vec3d {
            x: 0.99,
            y: 64.0,
            z: 0.5,
        },
        Vec3d::default(),
    )
    .unwrap();
    store.drain_lifecycle_events();

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert!(store.get(id).is_some());
}

#[test]
fn a_llama_spit_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_llama_spit();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_delta_movement(Vec3d {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        });
    }
    for _ in 0..2 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let before_data = llama_spit_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:llama_spit"
    );

    let mut reloaded = EntityStore::new(11, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("entity chunk read succeeds");
    assert_eq!(ids.len(), 1);
    let after_data = llama_spit_data(&reloaded, ids[0]);
    assert_eq!(after_data.owner(), before_data.owner());
    assert_eq!(after_data.left_owner(), before_data.left_owner());
    assert_eq!(after_data.has_been_shot(), before_data.has_been_shot());
}

#[test]
fn a_llama_spit_landing_on_a_block_plays_no_step_sound_and_raises_no_step() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    for x in 2..=10 {
        let floor = pos(x, 63, 0);
        let stone = state_id("minecraft:stone");
        level.world.set_block(
            DIMENSION,
            floor,
            stone,
            crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
        );
        assert_eq!(level.world.block_state(DIMENSION, floor), Some(stone));
    }
    let start = Vec3d {
        x: 2.5,
        y: 64.7,
        z: 0.5,
    };
    let id = spawn_llama_spit(
        &mut store,
        start,
        Vec3d {
            x: 1.2,
            y: -0.3,
            z: 0.0,
        },
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();

    for _ in 0..200 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        let Some(entity) = store.get(id) else {
            break;
        };
        assert!(entity.position().x >= start.x);
    }

    assert!(
        store.get(id).is_none(),
        "the spit ended its flight on the plate"
    );
    assert!(
        !store
            .drain_lifecycle_events()
            .into_iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })),
        "a spit plays no step sound"
    );
    assert!(
        level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .all(|event| event.recorded.game_event != VanillaGameEvent::STEP),
        "a spit raises no step game event"
    );
}
