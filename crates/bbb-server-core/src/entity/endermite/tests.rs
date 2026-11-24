use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMITE_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use crate::entity::tests::native_level::NativeLevel;

use super::{after_body_rotation, before_super, EndermiteRetainedState};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{tick_one_entity, EntityData, EntityIdCounter, EntityStore};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn default_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0xE_D3_42, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMITE_ID, position())
        .expect("Endermite is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
            entry(
                "Entities",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
            ),
        ],
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0).unwrap()
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = entries.iter_mut().find(|field| field.name == name) {
        field.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

fn restored_store(entries: Vec<NbtSummaryEntry>, seed: i64) -> (EntityStore, i32) {
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(entries)).unwrap()[0];
    (store, id)
}

fn endermite_state(store: &EntityStore, id: i32) -> EndermiteRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("Endermite uses GeneralLiving")
    };
    data.endermite_retained().unwrap()
}

#[test]
fn endermite_lifetime_save_and_restore() {
    let record = default_record();
    assert_eq!(
        record
            .iter()
            .filter(|field| field.name == "Lifetime")
            .map(|field| &field.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Int(0)]
    );

    let mut missing = record.clone();
    missing.retain(|field| field.name != "Lifetime");
    missing.push(entry("FutureEndermitePayload", NbtSummaryValue::Long(42)));
    let (restored, id) = restored_store(missing, 0xE_D3_43);
    assert_eq!(endermite_state(&restored, id).life(), 0);
    let EntityData::GeneralLiving(data) = restored.get(id).unwrap().data() else {
        unreachable!("Endermite uses GeneralLiving")
    };
    assert!(data
        .opaque_fields()
        .iter()
        .any(|field| field.name == "FutureEndermitePayload"));
    assert!(data
        .opaque_fields()
        .iter()
        .all(|field| field.name != "Lifetime"));
    assert_eq!(
        write_entity_chunk(&restored, (0, 0))
            .unwrap()
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .filter(|field| field.name == "Lifetime")
            .map(|field| &field.value)
            .collect::<Vec<_>>(),
        vec![&NbtSummaryValue::Int(0)]
    );
}

#[test]
fn endermite_lifetime_runtime_ticks_persistence_no_ai_and_threshold_exactly() {
    let mut native = NativeLevel::new();
    let mut expiring = default_record();
    set_field(&mut expiring, "Lifetime", NbtSummaryValue::Int(2399));
    let (mut expiring, id) = restored_store(expiring, 0xE_D3_44);
    native.with_level(|level| tick_one_entity(&mut expiring, level, id));
    assert!(expiring.get(id).is_none());

    let mut persistent = default_record();
    set_field(&mut persistent, "Lifetime", NbtSummaryValue::Int(2399));
    set_field(
        &mut persistent,
        "PersistenceRequired",
        NbtSummaryValue::Byte(1),
    );
    let (mut persistent, id) = restored_store(persistent, 0xE_D3_45);
    native.with_level(|level| tick_one_entity(&mut persistent, level, id));
    assert_eq!(endermite_state(&persistent, id).life(), 2399);

    let mut persistent_at_limit = default_record();
    set_field(
        &mut persistent_at_limit,
        "Lifetime",
        NbtSummaryValue::Int(2400),
    );
    set_field(
        &mut persistent_at_limit,
        "PersistenceRequired",
        NbtSummaryValue::Byte(1),
    );
    let (mut persistent_at_limit, id) = restored_store(persistent_at_limit, 0xE_D3_46);
    native.with_level(|level| tick_one_entity(&mut persistent_at_limit, level, id));
    assert!(persistent_at_limit.get(id).is_none());

    let mut no_ai = default_record();
    set_field(&mut no_ai, "Lifetime", NbtSummaryValue::Int(2399));
    set_field(&mut no_ai, "NoAI", NbtSummaryValue::Byte(1));
    let (mut no_ai, id) = restored_store(no_ai, 0xE_D3_47);
    native.with_level(|level| tick_one_entity(&mut no_ai, level, id));
    assert!(no_ai.get(id).is_none());
}

#[test]
fn endermite_lifetime_runtime_mirrors_body_rotation_into_entity_yaw() {
    let mut store = EntityStore::new(0xE_D3_48, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMITE_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.set_rot(37.0, -5.0);
    entity.living_mut().unwrap().y_body_rot = -90.0;

    before_super(&mut entity);
    assert_eq!(entity.living().unwrap().y_body_rot, 37.0);
    entity.living_mut().unwrap().y_body_rot = 82.0;
    after_body_rotation(&mut entity);
    assert_eq!(entity.y_rot(), 82.0);
    assert_eq!(entity.x_rot(), -5.0);
}
