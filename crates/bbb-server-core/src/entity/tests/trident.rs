use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::item::plain_stack;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::trident::DEFAULT_SOUND_EVENT_RESOURCE_ID;
use crate::entity::{spawn_trident, tick_entities, ArrowPickup, EntityData, TridentData};

fn trident_stack() -> RuntimeItemStack {
    plain_stack("minecraft:trident", 1).expect("trident is a built-in stack")
}

fn flying_trident() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_trident(
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
        trident_stack(),
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn trident_data(store: &EntityStore, id: i32) -> TridentData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Trident(data) => data.clone(),
        other => panic!("expected a trident, found {other:?}"),
    }
}

#[test]
fn a_spawned_trident_carries_motion_and_pickup_defaults() {
    let (store, _, id) = flying_trident();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_TRIDENT_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(trident_data(&store, id).pickup(), ArrowPickup::Disallowed);
    assert_eq!(
        trident_data(&store, id).damage(),
        crate::entity::arrow::DEFAULT_DAMAGE
    );
    assert!(!trident_data(&store, id).dealt_damage());
    assert!(non_default_synched_values(entity).is_empty());
}

fn land_trident(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store
            .get(id)
            .and_then(|entity| crate::entity::abstract_arrow::state(entity.data()))
            .is_some_and(|state| state.in_ground())
        {
            return;
        }
    }
    panic!("trident never landed");
}

#[test]
fn a_trident_falls_and_sticks_in_the_ground() {
    let (mut store, mut level, id) = flying_trident();
    land_trident(&mut store, &mut level, id);
    let entity = store.get(id).unwrap();
    let data = trident_data(&store, id);
    assert!(data.in_ground());
    assert_eq!(data.shake_time(), 7);
    assert_eq!(entity.delta_movement(), Vec3d::default());
    let synched = non_default_synched_values(entity);
    assert!(
        synched
            .iter()
            .any(|row| row.data_id == VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID),
        "in-ground is published"
    );
}

#[test]
fn a_stuck_trident_despawns_after_twelve_hundred_ticks() {
    let (mut store, mut level, id) = flying_trident();
    land_trident(&mut store, &mut level, id);
    trident_data_mut(&mut store, id).set_pickup(ArrowPickup::Allowed);
    for _ in 0..1200 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
}

#[test]
fn a_trident_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_trident();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_rot(45.0, -15.0);
        entity.set_delta_movement(Vec3d {
            x: 0.2,
            y: -0.3,
            z: 0.1,
        });
    }
    {
        let data = trident_data_mut(&mut store, id);
        data.set_pickup(ArrowPickup::Allowed);
        data.set_crit(true);
    }
    for _ in 0..2 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let before = store.get(id).unwrap().clone();
    let before_data = trident_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:trident"
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = trident_data(&reloaded, ids[0]);
    assert_eq!(after_data.pickup(), before_data.pickup());
    assert_eq!(after_data.damage(), before_data.damage());
    assert_eq!(after_data.crit(), before_data.crit());
    assert_eq!(after_data.pierce_level(), before_data.pierce_level());
    assert_eq!(
        after_data.sound_event_resource_id(),
        before_data.sound_event_resource_id()
    );
    assert_eq!(after_data.item(), before_data.item());
    assert_eq!(after_data.life(), before_data.life());
    assert_eq!(after_data.in_ground(), before_data.in_ground());
    assert_eq!(after_data.shake_time(), before_data.shake_time());
    assert_eq!(after_data.dealt_damage(), before_data.dealt_damage());
    assert_eq!(
        after_data.sound_event_resource_id(),
        DEFAULT_SOUND_EVENT_RESOURCE_ID
    );
}

#[test]
fn a_trident_stops_at_the_nearer_target_when_the_farther_one_was_filed_first() {
    let (mut store, mut level, id) = flying_trident();
    for x in [4.5, 2.5] {
        store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d { x, y: 68.0, z: 0.5 },
            )
            .unwrap();
    }
    let entity = store.get_mut(id).unwrap();
    entity.set_pos(Vec3d {
        x: 0.5,
        y: 68.5,
        z: 0.5,
    });
    entity.set_delta_movement(Vec3d {
        x: 5.0,
        y: 0.0,
        z: 0.0,
    });
    store.refile(id);

    level.with_level(|level| tick_entities(&mut store, level, true));

    let x = store.get(id).unwrap().position().x;
    assert!(
        (1.5..3.0).contains(&x),
        "the nearer cow stops the trident at {x}"
    );
}

fn trident_data_mut(store: &mut EntityStore, id: i32) -> &mut TridentData {
    match &mut store.get_mut(id).expect("entity is live").data {
        EntityData::Trident(data) => data,
        other => panic!("expected a trident, found {other:?}"),
    }
}
