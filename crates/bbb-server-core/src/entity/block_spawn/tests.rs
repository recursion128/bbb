use bbb_protocol::entity_synched_data::{
    VANILLA_AGEABLE_MOB_BABY_DATA_ID, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::NbtSummaryValue;

use crate::entity::save::write_entity_chunk;
use crate::entity::store::EntityLifecycleEvent;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

fn position() -> Vec3d {
    Vec3d {
        x: 4.3,
        y: 70.0,
        z: -2.7,
    }
}

fn home() -> BlockPos {
    BlockPos { x: 4, y: 70, z: -3 }
}

fn field<'a>(entries: &'a [bbb_protocol::NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

#[test]
fn block_spawn_hatched_turtle_publishes_complete_snapshot_and_save() {
    let mut store = EntityStore::new(61, EntityIdCounter::new());
    let id = store.spawn_hatched_turtle(position(), home()).unwrap();
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
        panic!("factory must publish one complete Added snapshot")
    };
    assert_eq!(added.id(), id);
    assert_eq!(added.entity_type_id(), VANILLA_ENTITY_TYPE_TURTLE_ID);
    assert_eq!(added.position(), position());
    assert_eq!((added.y_rot(), added.x_rot()), (0.0, 0.0));
    let EntityData::GeneralLiving(data) = added.data() else {
        unreachable!()
    };
    assert_eq!(data.ageable_retained().unwrap().age(), -24_000);
    assert_eq!(data.turtle_retained().unwrap().home_pos(), home());
    assert!(non_default_synched_values(added).iter().any(|value| {
        value.data_id == VANILLA_AGEABLE_MOB_BABY_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));

    let record = write_entity_chunk(&store, (0, -1)).unwrap();
    let fields = record.entities().next().unwrap().raw();
    assert!(fields.iter().any(|entry| {
        entry.name == "Age" && entry.value == bbb_protocol::NbtSummaryValue::Int(-24_000)
    }));
    assert!(fields.iter().any(|entry| {
        entry.name == "home_pos"
            && entry.value == bbb_protocol::NbtSummaryValue::IntArray(vec![4, 70, -3])
    }));
}

#[test]
fn block_spawn_hatched_sniffer_keeps_constructor_head_yaw_and_saves_baby() {
    let seed = 70;
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    let id = store.spawn_hatched_sniffer(position(), -179.5).unwrap();
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
        panic!("factory must publish one complete Added snapshot")
    };
    assert_eq!(added.id(), id);
    assert_eq!(added.entity_type_id(), VANILLA_ENTITY_TYPE_SNIFFER_ID);
    assert_eq!(added.position(), position());
    assert_eq!((added.y_rot(), added.x_rot()), (-179.5, 0.0));
    let EntityData::GeneralLiving(data) = added.data() else {
        unreachable!()
    };
    assert_eq!(data.ageable_retained().unwrap().age(), -24_000);
    assert!(non_default_synched_values(added).iter().any(|value| {
        value.data_id == VANILLA_AGEABLE_MOB_BABY_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));
    let record = write_entity_chunk(&store, (0, -1)).unwrap();
    assert_eq!(
        field(record.entities().next().unwrap().raw(), "Age"),
        &NbtSummaryValue::Int(-24_000)
    );
}

#[test]
fn block_spawn_hydrated_happy_ghast_adds_before_entity_sound() {
    let seed = 71;
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    let id = store.spawn_hydrated_happy_ghast(position(), 90.0).unwrap();
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }, EntityLifecycleEvent::SoundFromEntity {
        sound,
        position_at_emit,
    }] = events.as_slice()
    else {
        panic!("Happy Ghast must publish Added before its entity sound")
    };
    assert_eq!(added.id(), id);
    assert_eq!(added.entity_type_id(), VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
    assert_eq!(added.position(), position());
    assert_eq!((added.y_rot(), added.x_rot()), (90.0, 0.0));
    assert_eq!(added.y_head_rot(), 90.0);
    let EntityData::GeneralLiving(data) = added.data() else {
        unreachable!()
    };
    assert_eq!(data.ageable_retained().unwrap().age(), -24_000);
    assert_eq!(*position_at_emit, position());
    assert_eq!(sound.entity_id, id);
    assert_eq!(sound.source, SoundSource::Blocks);
    assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.ghastling.spawn"
            )
            .unwrap()
        }
    );
    let record = write_entity_chunk(&store, (0, -1)).unwrap();
    assert_eq!(
        field(record.entities().next().unwrap().raw(), "Age"),
        &NbtSummaryValue::Int(-24_000)
    );
}

#[test]
fn block_spawn_frogspawn_tadpole_publishes_persistent_exact_snapshot() {
    let position = Vec3d {
        x: 1.2,
        y: 63.5,
        z: 2.8,
    };
    let mut store = EntityStore::new(64, EntityIdCounter::new());
    let result = store.spawn_frogspawn_tadpole(position, 271.0).unwrap();
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
        panic!("factory must publish one complete Added snapshot")
    };
    assert_eq!(result.entity_id(), added.id());
    assert_eq!(added.entity_type_id(), VANILLA_ENTITY_TYPE_TADPOLE_ID);
    assert_eq!(added.position(), position);
    assert_eq!((added.y_rot(), added.x_rot()), (271.0, 0.0));
    let EntityData::GeneralLiving(data) = added.data() else {
        unreachable!()
    };
    assert!(data.mob_retained().unwrap().persistence_required());
    let tadpole = data.tadpole_retained().unwrap();
    assert_eq!(tadpole.age(), 0);
    assert!(!tadpole.age_locked());
    let changed = non_default_synched_values(added);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].data_id, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
    assert_eq!(changed[0].value, EntityDataValueKind::Float(6.0));

    let record = write_entity_chunk(&store, (0, 0)).unwrap();
    let fields = record.entities().next().unwrap().raw();
    assert_eq!(
        field(fields, "PersistenceRequired"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(field(fields, "Age"), &NbtSummaryValue::Int(0));
    assert_eq!(field(fields, "AgeLocked"), &NbtSummaryValue::Byte(0));
}

#[test]
fn block_spawn_portal_zombified_piglin_is_added_after_finalize() {
    let position = Vec3d {
        x: 10.5,
        y: 65.0,
        z: 8.5,
    };
    let mut store = EntityStore::new(65, EntityIdCounter::new());
    let fresh = store
        .create_portal_zombified_piglin(position, -91.0)
        .unwrap();
    let staged = &fresh;
    let staged_id = staged.id();
    assert!(store.ids().is_empty());
    assert!(store.drain_lifecycle_events().is_empty());
    assert_eq!(
        staged.entity_type_id(),
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    );
    assert_eq!(staged.position(), position);
    assert_eq!((staged.y_rot(), staged.x_rot()), (-91.0, 0.0));
    assert_eq!(staged.y_head_rot(), -91.0);
    assert_eq!(staged.living().unwrap().y_body_rot, -91.0);
    assert_eq!(staged.portal_cooldown(), 0);

    assert_eq!(store.add_fresh(fresh), staged_id);
    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
        panic!("publish must expose only the finalized staged root")
    };
    assert_eq!(staged_id, added.id());
    assert_eq!(
        added.entity_type_id(),
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    );
    assert_eq!(added.position(), position);
    assert_eq!((added.y_rot(), added.x_rot()), (-91.0, 0.0));
    assert_eq!(added.y_head_rot(), -91.0);
    assert_eq!(added.living().unwrap().y_body_rot, -91.0);
    assert_eq!(added.portal_cooldown(), 0);
    let changed = non_default_synched_values(added);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].data_id, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
    assert_eq!(changed[0].value, EntityDataValueKind::Float(20.0));

    store.get_mut(staged_id).unwrap().set_portal_cooldown();
    assert_eq!(store.get(staged_id).unwrap().portal_cooldown(), 300);
    let record = write_entity_chunk(&store, (0, 0)).unwrap();
    let fields = record.entities().next().unwrap().raw();
    assert_eq!(field(fields, "PortalCooldown"), &NbtSummaryValue::Int(300));
    assert_eq!(field(fields, "IsBaby"), &NbtSummaryValue::Byte(0));
    assert_eq!(field(fields, "CanBreakDoors"), &NbtSummaryValue::Byte(0));
    assert_eq!(field(fields, "anger_end_time"), &NbtSummaryValue::Long(0));
    assert!(fields.iter().all(|entry| entry.name != "angry_at"));
}
