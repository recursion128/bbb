use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
};
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_RABBIT_VARIANT_BROWN_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
};
use bbb_protocol::component::{component_translatable_nbt, ComponentStyle};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID;
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{read_save_data, set_variant, synched_values};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityStore};
use bbb_protocol::entity_synched_data::VANILLA_RABBIT_TYPE_DATA_ID;

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
    let mut store = EntityStore::new(0x52_41_42_42, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_RABBIT_ID, position())
        .unwrap();
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn rabbit_autonomous_runtime_census_save_tracking_and_legacy_default_are_exact() {
    let record = default_record();
    assert_eq!(
        field(&record, "RabbitType"),
        Some(&NbtSummaryValue::Int(VANILLA_RABBIT_VARIANT_BROWN_ID))
    );
    assert_eq!(
        field(&record, "MoreCarrotTicks"),
        Some(&NbtSummaryValue::Int(0))
    );
    let state = read_save_data(&[]);
    let value = synched_values(state)[0].clone();
    assert_eq!(value.value.data_id, VANILLA_RABBIT_TYPE_DATA_ID);
    assert_eq!(value.value.value, EntityDataValueKind::Int(0));
    assert!(value.is_default);

    for unknown in [i32::MIN, -1, 6, 98, 100, i32::MAX] {
        let state = read_save_data(&[entry("RabbitType", NbtSummaryValue::Int(unknown))]);
        assert_eq!(state.variant(), VANILLA_RABBIT_VARIANT_BROWN_ID);
    }
}

#[test]
fn rabbit_autonomous_runtime_restore_applies_killer_bunny_state() {
    let mut evil = default_record();
    set_field(
        &mut evil,
        "RabbitType",
        NbtSummaryValue::Int(VANILLA_RABBIT_VARIANT_EVIL_ID),
    );
    set_field(&mut evil, "MoreCarrotTicks", NbtSummaryValue::Int(40));
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(evil)).unwrap()[0];
    let entity = store.get(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert_eq!(
        data.rabbit_retained().unwrap().variant(),
        VANILLA_RABBIT_VARIANT_EVIL_ID
    );
    assert_eq!(data.rabbit_retained().unwrap().more_carrot_ticks(), 40);
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID),
        8.0
    );
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        8.0
    );
    assert_eq!(
        entity.custom_name().unwrap().component(),
        &component_translatable_nbt(
            "entity.minecraft.killer_bunny",
            Vec::new(),
            &ComponentStyle::default(),
        )
        .unwrap()
    );

    let mut legacy_unknown = default_record();
    set_field(&mut legacy_unknown, "RabbitType", NbtSummaryValue::Int(100));
    let mut normalized = EntityStore::new(2, EntityIdCounter::new());
    let id = read_entity_chunk(&mut normalized, &chunk(legacy_unknown)).unwrap()[0];
    let EntityData::GeneralLiving(data) = normalized.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        data.rabbit_retained().unwrap().variant(),
        VANILLA_RABBIT_VARIANT_BROWN_ID
    );
}

#[test]
fn rabbit_autonomous_runtime_variant_dirty_and_toast_name_are_independent() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_RABBIT_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_custom_name(Some(bbb_protocol::RichTextComponent::plain("Toast")));
    entity.take_synched_dirty();
    assert!(set_variant(entity, VANILLA_RABBIT_VARIANT_EVIL_ID));
    assert_eq!(entity.custom_name().unwrap().text(), "Toast");
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_RABBIT_TYPE_DATA_ID]
    );
    assert!(set_variant(entity, VANILLA_RABBIT_VARIANT_EVIL_ID));
    assert!(entity.take_synched_dirty().is_empty());
}
