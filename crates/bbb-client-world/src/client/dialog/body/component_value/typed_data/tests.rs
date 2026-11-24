use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type_for_resource_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    },
    entity_types::vanilla_entity_type_for_resource_id,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn raw_flattened_custom_data_reaches_dialog_item_component() {
    let units = "{value:'"
        .encode_utf16()
        .chain([0xd800, 65, 0xdc00])
        .chain("'}".encode_utf16())
        .collect::<Vec<_>>();
    let value = NbtSummaryValue::string_from_utf16(units);
    let decoded = decode_component(
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        "minecraft:custom_data",
        &value,
        None,
    );
    assert_eq!(
        decoded,
        super::super::DialogDataComponentValueState::CustomData {
            data: compound(vec![value_entry(
                "value",
                NbtSummaryValue::string_from_utf16(vec![0xd800, 65, 0xdc00])
            )]),
        }
    );
    let patch =
        super::super::persistent_patch(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, &value, None);
    let super::super::DialogDataComponentValueState::CustomData { data } = decoded else {
        panic!("expected custom-data component");
    };
    assert_eq!(patch.custom_data, Some(data));
}

#[test]
fn decodes_builtin_typed_entity_data_and_strips_id() {
    let value = compound(vec![
        string_entry("id", "zombie"),
        value_entry("Health", NbtSummaryValue::Float(12.0f32.to_bits())),
    ]);
    let decoded = decode_component(
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        "minecraft:entity_data",
        &value,
        None,
    );
    let super::super::DialogDataComponentValueState::EntityData { entity } = decoded else {
        panic!("expected entity-data component");
    };
    assert_eq!(entity.entity_type.resource_id, "minecraft:zombie");
    assert_eq!(
        entity.entity_type.holder_id,
        vanilla_entity_type_for_resource_id("minecraft:zombie")
            .unwrap()
            .id
    );
    assert_compound_names(&entity.data, &["Health"]);

    let patch =
        super::super::persistent_patch(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, &value, None);
    assert_eq!(
        patch.entity_data_entity_type_id,
        Some(entity.entity_type.holder_id)
    );
    assert_eq!(patch.entity_data_tag, Some(entity.data.clone()));

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:entity_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:zombie"),
    ];
    registries
        .contents
        .insert("minecraft:entity_type".to_string(), contents);
    let session_ignored = decode_component(
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        "minecraft:entity_data",
        &value,
        Some(&registries),
    );
    assert!(matches!(
        session_ignored,
        super::super::DialogDataComponentValueState::EntityData { entity }
            if entity.entity_type.holder_id
                == vanilla_entity_type_for_resource_id("minecraft:zombie").unwrap().id
    ));
}

#[test]
fn decodes_builtin_typed_block_entity_data_and_strips_id() {
    let value = compound(vec![
        string_entry("id", "minecraft:mob_spawner"),
        value_entry("SpawnCount", NbtSummaryValue::Short(4)),
        value_entry(
            "SpawnData",
            compound(vec![value_entry(
                "entity",
                compound(vec![string_entry("id", "minecraft:pig")]),
            )]),
        ),
    ]);
    let decoded = decode_component(
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        "minecraft:block_entity_data",
        &value,
        None,
    );
    let super::super::DialogDataComponentValueState::BlockEntityData { block_entity } = decoded
    else {
        panic!("expected block-entity-data component");
    };
    assert_eq!(
        block_entity.block_entity_type.resource_id,
        "minecraft:mob_spawner"
    );
    assert_eq!(
        block_entity.block_entity_type.holder_id,
        vanilla_block_entity_type_for_resource_id("minecraft:mob_spawner")
            .unwrap()
            .id
    );
    assert_compound_names(&block_entity.data, &["SpawnCount", "SpawnData"]);

    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        &value,
        None,
    );
    assert_eq!(
        patch.block_entity_data_type_id,
        Some(block_entity.block_entity_type.holder_id)
    );
    assert_eq!(patch.block_entity_data_tag, Some(block_entity.data.clone()));
    assert_eq!(
        patch.block_entity_spawn_entity_type.as_deref(),
        Some("minecraft:pig")
    );

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:block_entity_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:mob_spawner"),
    ];
    registries
        .contents
        .insert("minecraft:block_entity_type".to_string(), contents);
    let session_ignored = decode_component(
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        "minecraft:block_entity_data",
        &value,
        Some(&registries),
    );
    assert!(matches!(
        session_ignored,
        super::super::DialogDataComponentValueState::BlockEntityData { block_entity }
            if block_entity.block_entity_type.holder_id
                == vanilla_block_entity_type_for_resource_id("minecraft:mob_spawner").unwrap().id
    ));
}

#[test]
fn decodes_bucket_entity_data_as_unmodified_compound() {
    let value = compound(vec![
        value_entry("Health", NbtSummaryValue::Float(3.5f32.to_bits())),
        value_entry("Variant", NbtSummaryValue::Int(2)),
    ]);
    let decoded = decode_component(
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        "minecraft:bucket_entity_data",
        &value,
        None,
    );
    assert_eq!(
        decoded,
        super::super::DialogDataComponentValueState::BucketEntityData {
            data: value.clone()
        }
    );
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        &value,
        None,
    );
    assert_eq!(
        patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID),
        Some(&value)
    );
    assert_eq!(
        decode_custom_data(&compound(Vec::new()), "bucket entity data").unwrap(),
        compound(Vec::new())
    );
}

#[test]
fn decodes_custom_data_as_unmodified_compound() {
    let value = compound(vec![
        string_entry("owner", "Alex"),
        value_entry(
            "nested",
            compound(vec![value_entry("flag", NbtSummaryValue::Byte(1))]),
        ),
    ]);
    assert_eq!(
        decode_component(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            "minecraft:custom_data",
            &value,
            None,
        ),
        super::super::DialogDataComponentValueState::CustomData {
            data: value.clone()
        }
    );
    assert_eq!(
        decode_component(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            "minecraft:custom_data",
            &compound(Vec::new()),
            None,
        ),
        super::super::DialogDataComponentValueState::CustomData {
            data: compound(Vec::new())
        }
    );
}

#[test]
fn decodes_flattened_snbt_alternatives_for_typed_and_custom_data() {
    let super::super::DialogDataComponentValueState::EntityData { entity } = decode_component(
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        "minecraft:entity_data",
        &NbtSummaryValue::string("{id:'minecraft:zombie',Health:12.5f,Tags:['first','second']}"),
        None,
    ) else {
        panic!("expected entity data");
    };
    assert_eq!(entity.entity_type.resource_id, "minecraft:zombie");
    assert_compound_names(&entity.data, &["Health", "Tags"]);

    let super::super::DialogDataComponentValueState::BlockEntityData { block_entity } =
        decode_component(
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            "minecraft:block_entity_data",
            &NbtSummaryValue::string(
                "{id:'minecraft:mob_spawner',SpawnCount:4s,SpawnPotentials:[]}",
            ),
            None,
        )
    else {
        panic!("expected block entity data");
    };
    assert_eq!(
        block_entity.block_entity_type.resource_id,
        "minecraft:mob_spawner"
    );
    assert_compound_names(&block_entity.data, &["SpawnCount", "SpawnPotentials"]);

    let custom = decode_custom_data(
        &NbtSummaryValue::string("{owner:'\\N{LATIN CAPITAL LETTER A}lex',flags:[B;1b,0],uuid:uuid('00112233-4455-6677-8899-aabbccddeeff')}"),
        "custom data",
    )
    .unwrap();
    assert_compound_names(&custom, &["owner", "flags", "uuid"]);
    let NbtSummaryValue::Compound(entries) = custom else {
        panic!("expected compound");
    };
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "owner")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string("Alex"))
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "flags")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::ByteArray(vec![1, 0]))
    );

    for type_id in [
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    ] {
        let decoded = super::super::decode_supported_component_value(
            type_id,
            if type_id == VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID {
                "minecraft:bucket_entity_data"
            } else {
                "minecraft:custom_data"
            },
            &NbtSummaryValue::string("{Health:3.5f,Variant:2}"),
            None,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            decoded,
            super::super::DialogDataComponentValueState::BucketEntityData { .. }
                | super::super::DialogDataComponentValueState::CustomData { .. }
        ));
    }
}

#[test]
fn rejects_malformed_or_unknown_typed_entity_data() {
    for value in [
        NbtSummaryValue::List(Vec::new()),
        compound(Vec::new()),
        compound(vec![value_entry("id", NbtSummaryValue::Int(0))]),
        compound(vec![string_entry("id", "Bad Namespace:zombie")]),
        compound(vec![string_entry("id", "minecraft:missing")]),
    ] {
        assert!(super::super::decode_supported_component_value(
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            "minecraft:entity_data",
            &value,
            None,
        )
        .is_err());
    }
    assert!(super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        "minecraft:block_entity_data",
        &compound(vec![string_entry("id", "minecraft:missing")]),
        None,
    )
    .is_err());
    assert!(decode_custom_data(&NbtSummaryValue::List(Vec::new()), "bucket entity data").is_err());
    let malformed_flattened = NbtSummaryValue::string("{id:'minecraft:pig' trailing}");
    for (type_id, name) in [
        (
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            "minecraft:entity_data",
        ),
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            "minecraft:block_entity_data",
        ),
    ] {
        assert!(super::super::decode_supported_component_value(
            type_id,
            name,
            &malformed_flattened,
            None
        )
        .is_err());
    }
    assert!(decode_custom_data(&NbtSummaryValue::string("[1,2,3]"), "bucket entity data").is_err());
}

fn decode_component(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> super::super::DialogDataComponentValueState {
    super::super::decode_supported_component_value(type_id, resource_id, value, registries)
        .unwrap()
        .unwrap()
}

fn assert_compound_names(value: &NbtSummaryValue, expected: &[&str]) {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected compound");
    };
    assert_eq!(
        entries.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
        expected
    );
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::string(value))
}
