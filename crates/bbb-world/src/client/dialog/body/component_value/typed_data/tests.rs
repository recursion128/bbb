use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type_for_resource_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    },
    entity_types::vanilla_entity_type_for_resource_id,
    packets::inventory::DataComponentPatchSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::apply_item_patch_value;
use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn decodes_builtin_typed_entity_data_and_strips_id() {
    let value = compound(vec![
        string_entry("id", "minecraft:missing"),
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

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        &super::super::DialogDataComponentValueState::EntityData {
            entity: entity.clone(),
        },
        &mut patch,
    ));
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
        string_entry("id", "minecraft:missing"),
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

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        &super::super::DialogDataComponentValueState::BlockEntityData {
            block_entity: block_entity.clone(),
        },
        &mut patch,
    ));
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
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        &decoded,
        &mut patch,
    ));
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
    let entity = decode_builtin_entity_data(
        &NbtSummaryValue::String(
            "{id:'minecraft:zombie',Health:12.5f,Tags:['first','second']}".to_string(),
        ),
        "entity data",
    )
    .unwrap();
    assert_eq!(entity.entity_type.resource_id, "minecraft:zombie");
    assert_compound_names(&entity.data, &["Health", "Tags"]);

    let block_entity = decode_builtin_block_entity_data(
        &NbtSummaryValue::String(
            "{id:'minecraft:mob_spawner',SpawnCount:4s,SpawnPotentials:[]}".to_string(),
        ),
        "block entity data",
    )
    .unwrap();
    assert_eq!(
        block_entity.block_entity_type.resource_id,
        "minecraft:mob_spawner"
    );
    assert_compound_names(&block_entity.data, &["SpawnCount", "SpawnPotentials"]);

    let custom = decode_custom_data(
        &NbtSummaryValue::String(
            "{owner:'\\N{LATIN CAPITAL LETTER A}lex',flags:[B;1b,0],uuid:uuid('00112233-4455-6677-8899-aabbccddeeff')}"
                .to_string(),
        ),
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
        Some(&NbtSummaryValue::String("Alex".to_string()))
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
            &NbtSummaryValue::String("{Health:3.5f,Variant:2}".to_string()),
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
    for (decode, value, expected) in [
        (
            decode_builtin_entity_data as fn(&NbtSummaryValue, &str) -> _,
            NbtSummaryValue::List(Vec::new()),
            "is not a compound",
        ),
        (
            decode_builtin_entity_data,
            compound(Vec::new()),
            "missing field id",
        ),
        (
            decode_builtin_entity_data,
            compound(vec![value_entry("id", NbtSummaryValue::Int(0))]),
            "id is not a resource id",
        ),
        (
            decode_builtin_entity_data,
            compound(vec![string_entry("id", "Bad Namespace:zombie")]),
            "invalid",
        ),
        (
            decode_builtin_entity_data,
            compound(vec![string_entry("id", "minecraft:missing")]),
            "missing minecraft:entity_type resource",
        ),
    ] {
        let error = decode(&value, "entity data").unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }

    let error = decode_builtin_block_entity_data(
        &compound(vec![string_entry("id", "minecraft:missing")]),
        "block entity data",
    )
    .unwrap_err();
    assert!(error.contains("missing minecraft:block_entity_type resource"));
    assert!(
        decode_custom_data(&NbtSummaryValue::List(Vec::new()), "bucket entity data")
            .unwrap_err()
            .contains("is not a compound")
    );
    let malformed_flattened = NbtSummaryValue::String("{id:'minecraft:pig' trailing}".to_string());
    assert!(
        decode_builtin_entity_data(&malformed_flattened, "entity data")
            .unwrap_err()
            .contains("invalid entity data flattened SNBT string")
    );
    assert!(
        decode_builtin_block_entity_data(&malformed_flattened, "block entity data")
            .unwrap_err()
            .contains("invalid block entity data flattened SNBT string")
    );
    assert!(decode_custom_data(
        &NbtSummaryValue::String("[1,2,3]".to_string()),
        "bucket entity data"
    )
    .unwrap_err()
    .contains("flattened SNBT root is not a compound"));
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
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}
