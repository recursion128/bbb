use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::NbtSummaryValue;

use crate::entity::item::plain_stack;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{
    spawn_spectral_arrow, tick_entities, ArrowPickup, EntityData, SpectralArrowData,
};

fn spectral_arrow_stack() -> RuntimeItemStack {
    plain_stack("minecraft:spectral_arrow", 1).expect("spectral arrow is a built-in stack")
}

fn flying_spectral_arrow() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_spectral_arrow(
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
        spectral_arrow_stack(),
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn spectral_arrow_data(store: &EntityStore, id: i32) -> SpectralArrowData {
    match store.get(id).expect("entity is live").data() {
        EntityData::SpectralArrow(data) => data.clone(),
        other => panic!("expected a spectral arrow, found {other:?}"),
    }
}

#[test]
fn a_spawned_spectral_arrow_carries_motion_and_duration_defaults() {
    let (store, _, id) = flying_spectral_arrow();
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID
    );
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(
        spectral_arrow_data(&store, id).duration(),
        crate::entity::spectral_arrow::DEFAULT_DURATION
    );
    assert_eq!(
        spectral_arrow_data(&store, id).pickup(),
        ArrowPickup::Disallowed
    );
    assert!(non_default_synched_values(entity).is_empty());
}

fn land_spectral_arrow(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
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
    panic!("spectral arrow never landed");
}

#[test]
fn a_spectral_arrow_falls_and_sticks_in_the_ground() {
    let (mut store, mut level, id) = flying_spectral_arrow();
    land_spectral_arrow(&mut store, &mut level, id);
    let entity = store.get(id).unwrap();
    let data = spectral_arrow_data(&store, id);
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
fn a_spectral_arrow_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_spectral_arrow();
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        panic!("the spectral arrow command data is a compound");
    };
    entries
        .iter_mut()
        .find(|entry| entry.name == "SoundEvent")
        .unwrap()
        .value = NbtSummaryValue::String("block.bell.use".into());
    store.load_command_data(id, &entries).unwrap();
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
        let data = spectral_arrow_data_mut(&mut store, id);
        data.set_pickup(ArrowPickup::Allowed);
        data.set_crit(true);
    }
    for _ in 0..2 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let before = store.get(id).unwrap().clone();
    let before_data = spectral_arrow_data(&store, id);
    assert_eq!(
        before_data.sound_event_resource_id(),
        "minecraft:block.bell.use"
    );

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:spectral_arrow"
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = spectral_arrow_data(&reloaded, ids[0]);
    assert_eq!(after_data.duration(), before_data.duration());
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
    let NbtSummaryValue::Compound(entries) = reloaded.command_data(ids[0]).unwrap() else {
        panic!("the restored spectral arrow command data is a compound");
    };
    assert!(entries.iter().any(|entry| entry.name == "SoundEvent"
        && entry.value == NbtSummaryValue::String("minecraft:block.bell.use".into())));
}

fn spectral_arrow_data_mut(store: &mut EntityStore, id: i32) -> &mut SpectralArrowData {
    match &mut store.get_mut(id).expect("entity is live").data {
        EntityData::SpectralArrow(data) => data,
        other => panic!("expected a spectral arrow, found {other:?}"),
    }
}
