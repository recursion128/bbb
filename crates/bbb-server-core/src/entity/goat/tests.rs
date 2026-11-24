use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_GOAT_LEFT_HORN_DATA_ID, VANILLA_GOAT_RIGHT_HORN_DATA_ID, VANILLA_GOAT_SCREAMING_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{
    adjust_fall_damage, age_boundary_reached, before_ai_step, read_save_data, synched_values,
    write_save_data,
};
use crate::entity::general_living::AgeableMobRetainedState;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

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

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = entries.iter_mut().find(|entry| entry.name == name) {
        field.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
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
        ),
        0,
        0,
    )
    .unwrap()
}

fn default_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x47_4f_41_54, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_GOAT_ID, position())
        .expect("Goat is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn goat_save_and_tracking_defaults() {
    let state = read_save_data(&[]);
    let mut entries = Vec::new();
    write_save_data(&mut entries, &state);
    assert_eq!(
        field(&entries, "IsScreamingGoat"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(&entries, "HasLeftHorn"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&entries, "HasRightHorn"),
        Some(&NbtSummaryValue::Byte(1))
    );

    let values = synched_values(&state);
    assert_eq!(values[0].value.data_id, VANILLA_GOAT_SCREAMING_DATA_ID);
    assert_eq!(values[1].value.data_id, VANILLA_GOAT_LEFT_HORN_DATA_ID);
    assert_eq!(values[2].value.data_id, VANILLA_GOAT_RIGHT_HORN_DATA_ID);
    assert!(values.iter().all(|value| value.is_default));
    assert_eq!(values[1].value.value, EntityDataValueKind::Boolean(true));
}

#[test]
fn goat_restore_defaults_and_baby_attack() {
    let record = default_record();
    let mut sparse = record.clone();
    sparse.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsScreamingGoat" | "HasLeftHorn" | "HasRightHorn"
        )
    });
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &chunk(sparse)).unwrap()[0];
    let EntityData::GeneralLiving(data) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(data.goat_retained().copied(), Some(read_save_data(&[])));

    let mut baby = record.clone();
    set_field(&mut baby, "Age", NbtSummaryValue::Int(-20));
    let mut restored = EntityStore::new(5, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &chunk(baby)).unwrap()[0];
    assert_eq!(
        restored
            .get(id)
            .unwrap()
            .living()
            .unwrap()
            .attributes()
            .base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        Some(1.0)
    );
}

#[test]
fn goat_runtime_head_age_and_fall_hooks_are_exact() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_GOAT_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.goat_retained_mut().unwrap().set_lowering_head(true);
    before_ai_step(entity);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    assert_eq!(data.goat_retained().unwrap().lower_head_tick, 1);

    *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(-1, 0, true);
    age_boundary_reached(entity);
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attributes()
            .base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        Some(1.0)
    );
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(0, 0, true);
    age_boundary_reached(entity);
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attributes()
            .base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        Some(2.0)
    );
    assert_eq!(adjust_fall_damage(entity, 17), 7);
}
