use super::native_level::{NativeLevel, DIMENSION};

use bbb_protocol::entity_synched_data::VANILLA_EYE_OF_ENDER_ITEM_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::eye_of_ender::signal_to;
use crate::entity::item::{empty_stack, plain_stack, stack_count, stack_with_count};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{spawn_eye_of_ender, tick_entities, EntityData, EyeOfEnderData, RemovalReason};

fn eye_stack(count: i32) -> RuntimeItemStack {
    plain_stack("minecraft:ender_eye", count).expect("ender eye is a built-in stack")
}

fn custom_eye_stack(count: i32) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:ender_eye"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(count),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:custom_data".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "bbb".into(),
                        value: NbtSummaryValue::Int(26),
                    }]),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .expect("the custom eye stack is writer-canonical")
}

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn eye_data(store: &EntityStore, id: i32) -> EyeOfEnderData {
    match store.get(id).expect("eye is live").data() {
        EntityData::EyeOfEnder(data) => data.clone(),
        other => panic!("expected eye of ender, found {other:?}"),
    }
}

#[test]
fn spawn_normalizes_empty_and_nonempty_items_before_the_construct_edge() {
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    let empty_id = spawn_eye_of_ender(
        &mut store,
        position(),
        empty_stack().expect("empty stack form is valid"),
    )
    .expect("empty input constructs an eye");
    let custom = custom_eye_stack(7);
    let custom_id = spawn_eye_of_ender(&mut store, position(), custom.clone())
        .expect("custom input constructs an eye");

    assert_eq!(eye_data(&store, empty_id).item(), &eye_stack(1));
    let expected = stack_with_count(&custom, 1).expect("count one is valid");
    assert_eq!(eye_data(&store, custom_id).item(), &expected);
    assert_eq!(stack_count(eye_data(&store, custom_id).item()), 1);
    assert_eq!(store.get(empty_id).unwrap().gravity(), 0.0);
    assert_eq!(
        store.get(custom_id).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID
    );
}

#[test]
fn signal_to_keeps_near_targets_caps_far_targets_and_resets_life() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_eye_of_ender(&mut store, position(), eye_stack(1)).unwrap();
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(eye_data(&store, id).life(), 2);

    let near = Vec3d {
        x: 10.0,
        y: 70.0,
        z: 1.0,
    };
    assert!(signal_to(&mut store, id, near));
    let data = eye_data(&store, id);
    assert_eq!(data.target(), Some(near));
    assert_eq!(data.life(), 0);
    assert!(data.survive_after_death());

    let far = Vec3d {
        x: 30.5,
        y: 10.0,
        z: 0.5,
    };
    assert!(signal_to(&mut store, id, far));
    assert_eq!(
        eye_data(&store, id).target(),
        Some(Vec3d {
            x: 12.5,
            y: 72.0,
            z: 0.5,
        })
    );
    assert_eq!(eye_data(&store, id).life(), 0);
}

#[test]
fn tick_moves_with_the_old_velocity_then_updates_the_next_velocity() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_eye_of_ender(&mut store, position(), eye_stack(1)).unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 1.0,
        y: 2.0,
        z: 0.0,
    });
    assert!(signal_to(
        &mut store,
        id,
        Vec3d {
            x: 100.5,
            y: 64.0,
            z: 0.5,
        }
    ));
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();

    level.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 1.5,
            y: 66.0,
            z: 0.5,
        }
    );
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 1.025,
            y: 1.985,
            z: 0.0,
        }
    );
    assert_eq!(eye_data(&store, id).life(), 1);
}

fn tick_until_death(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..80 {
        level.with_level(|level| tick_entities(store, level, true));
    }
    assert!(store.get(id).is_some(), "life 80 is still alive");
    assert!(store
        .drain_lifecycle_events()
        .into_iter()
        .all(|event| !matches!(event, EntityLifecycleEvent::Sound { .. })));
    level.with_level(|level| tick_entities(store, level, true));
    assert!(store.get(id).is_none(), "life 81 is discarded");
}

#[test]
fn surviving_death_plays_the_sound_and_spawns_the_carried_item() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let carried = custom_eye_stack(9);
    let expected = stack_with_count(&carried, 1).unwrap();
    let id = spawn_eye_of_ender(&mut store, position(), carried).unwrap();
    assert!(signal_to(
        &mut store,
        id,
        Vec3d {
            x: 10.0,
            y: 10_000.0,
            z: 0.5,
        }
    ));
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();

    tick_until_death(&mut store, &mut level, id);

    let item_id = store.ids().into_iter().next().expect("the item survives");
    let item = store.get(item_id).unwrap();
    assert_eq!(item.entity_type_id(), VANILLA_ENTITY_TYPE_ITEM_ID);
    let EntityData::Item(item_data) = item.data() else {
        panic!("surviving eye spawned a non-item")
    };
    assert_eq!(item_data.item(), &expected);
    assert!(level
        .world
        .drain_level_events_for_test(DIMENSION)
        .is_empty());

    let events = store.drain_lifecycle_events();
    assert!(events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Added { entity, .. }
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Removed { entity, .. }
            if entity.id() == id && entity.removal_reason() == Some(RemovalReason::Discarded)
    )));
    let sound = events
        .iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .expect("death sound is queued");
    assert_eq!(sound.source, SoundSource::Neutral);
    assert_eq!(sound.volume, 1.0);
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.ender_eye.death"
            )
            .unwrap()
        }
    );
}

#[test]
fn shattered_death_plays_the_sound_and_emits_level_event_2003() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_eye_of_ender(&mut store, position(), eye_stack(1)).unwrap();
    assert!(signal_to(
        &mut store,
        id,
        Vec3d {
            x: 10.0,
            y: 10_000.0,
            z: 0.5,
        }
    ));
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();

    tick_until_death(&mut store, &mut level, id);

    assert!(store.is_empty());
    assert!(level
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 2003 && event.data == 0));
    let events = store.drain_lifecycle_events();
    assert!(events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    assert!(events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Removed { entity, .. } if entity.id() == id
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        EntityLifecycleEvent::Added { entity, .. }
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID
    )));
}

#[test]
fn save_and_region_reload_keep_the_item_and_reset_transient_flight_state() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let carried = custom_eye_stack(5);
    let id = spawn_eye_of_ender(&mut store, position(), carried.clone()).unwrap();
    assert!(signal_to(
        &mut store,
        id,
        Vec3d {
            x: 10.0,
            y: 80.0,
            z: 0.5,
        }
    ));
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    let before = store.get(id).unwrap().clone();
    let expected = stack_with_count(&carried, 1).unwrap();

    let document = write_entity_chunk(&store, (0, 0)).expect("eye save writes");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:eye_of_ender"
    );
    let mut reloaded = EntityStore::new(99, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("eye save reads");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());
    let data = eye_data(&reloaded, ids[0]);
    assert_eq!(data.item(), &expected);
    assert_eq!(data.target(), None);
    assert_eq!(data.life(), 0);
    assert!(!data.survive_after_death());

    let missing = crate::entity::eye_of_ender::read_save_data(
        &[],
        &bbb_registry_codec::UnavailableHolderResolver,
    );
    assert_eq!(missing.item(), &eye_stack(1));
}

#[test]
fn tracking_filters_the_default_item_and_publishes_a_custom_stack() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let default_id = spawn_eye_of_ender(
        &mut store,
        position(),
        empty_stack().expect("empty stack form is valid"),
    )
    .unwrap();
    let custom_id = spawn_eye_of_ender(&mut store, position(), custom_eye_stack(3)).unwrap();

    assert!(!non_default_synched_values(store.get(default_id).unwrap())
        .iter()
        .any(|value| value.data_id == VANILLA_EYE_OF_ENDER_ITEM_DATA_ID));
    let item = non_default_synched_values(store.get(custom_id).unwrap())
        .into_iter()
        .find(|value| value.data_id == VANILLA_EYE_OF_ENDER_ITEM_DATA_ID)
        .expect("custom item metadata is published");
    let EntityDataValueKind::ItemStack(stack) = item.value else {
        panic!("eye metadata used a non-item serializer")
    };
    assert_eq!(stack.count, 1);
    assert!(stack.component_patch.custom_data.is_some());
}

#[test]
fn base_tick_still_discards_an_eye_below_the_world() {
    let mut level = NativeLevel::new();
    let min_y = level.with_level(|level| level.min_y());
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = spawn_eye_of_ender(
        &mut store,
        Vec3d {
            x: 0.5,
            y: f64::from(min_y) - 100.0,
            z: 0.5,
        },
        eye_stack(1),
    )
    .unwrap();
    store.drain_lifecycle_events();

    level.with_level(|level| tick_entities(&mut store, level, true));

    assert!(store.get(id).is_none());
    assert!(store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(
            event,
            EntityLifecycleEvent::Removed { entity, .. }
                if entity.removal_reason() == Some(RemovalReason::Discarded)
        )));
}
