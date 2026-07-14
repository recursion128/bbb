use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    entity_types::{VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID},
    sound_event_types::vanilla_sound_event_registry_id,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_equippable_defaults_and_optional_fields() {
    let decoded = decode(&compound([("slot", string("head"))]), None).unwrap();
    let Some(DialogDataComponentValueState::Equippable { equippable }) = decoded else {
        panic!("expected equippable component");
    };
    assert_eq!(equippable.slot_id, 4);
    assert_eq!(equippable.slot, "head");
    assert_eq!(
        equippable.equip_sound,
        named_sound("minecraft:item.armor.equip_generic")
    );
    assert_eq!(equippable.asset_id, None);
    assert_eq!(equippable.camera_overlay, None);
    assert_eq!(equippable.allowed_entities, None);
    assert!(equippable.dispensable);
    assert!(equippable.swappable);
    assert!(equippable.damage_on_hurt);
    assert!(!equippable.equip_on_interact);
    assert!(!equippable.can_be_sheared);
    assert_eq!(
        equippable.shearing_sound,
        named_sound("minecraft:item.shears.snip")
    );

    let registries = RegistrySet::default();
    let decoded = decode(
        &compound([
            ("slot", string("body")),
            ("equip_sound", direct_sound("example:equip", Some(6.5))),
            ("asset_id", string("example:llama_carpet")),
            ("camera_overlay", string("overlay/powder_snow")),
            (
                "allowed_entities",
                NbtSummaryValue::List(vec![string("llama"), string("trader_llama")]),
            ),
            ("dispensable", NbtSummaryValue::Byte(0)),
            ("swappable", NbtSummaryValue::Byte(0)),
            ("damage_on_hurt", NbtSummaryValue::Byte(0)),
            ("equip_on_interact", NbtSummaryValue::Byte(1)),
            ("can_be_sheared", NbtSummaryValue::Byte(1)),
            ("shearing_sound", string("item.shears.snip")),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Equippable { equippable }) = decoded else {
        panic!("expected equippable component");
    };
    assert_eq!(equippable.slot_id, 6);
    assert_eq!(equippable.asset_id.as_deref(), Some("example:llama_carpet"));
    assert_eq!(
        equippable.camera_overlay.as_deref(),
        Some("minecraft:overlay/powder_snow")
    );
    assert_eq!(
        equippable.equip_sound,
        DialogSoundEventHolderState::Direct {
            sound_id: "example:equip".to_string(),
            fixed_range_bits: Some(6.5f32.to_bits()),
        }
    );
    let allowed = equippable.allowed_entities.unwrap();
    assert_eq!(allowed.holder_set_kind, "direct");
    assert_eq!(
        allowed
            .entries
            .iter()
            .map(|entry| (entry.holder_id, entry.resource_id.as_str()))
            .collect::<Vec<_>>(),
        [
            (VANILLA_ENTITY_TYPE_LLAMA_ID, "minecraft:llama"),
            (
                VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                "minecraft:trader_llama"
            ),
        ]
    );
    assert!(!equippable.dispensable);
    assert!(!equippable.swappable);
    assert!(!equippable.damage_on_hurt);
    assert!(equippable.equip_on_interact);
    assert!(equippable.can_be_sheared);
}

#[test]
fn decodes_all_equipment_slot_codec_names() {
    for (name, id) in [
        ("mainhand", 0),
        ("offhand", 5),
        ("feet", 1),
        ("legs", 2),
        ("chest", 3),
        ("head", 4),
        ("body", 6),
        ("saddle", 7),
    ] {
        let decoded = decode(&compound([("slot", string(name))]), None).unwrap();
        assert!(matches!(
            decoded,
            Some(DialogDataComponentValueState::Equippable {
                equippable: DialogEquippableState {
                    slot_id,
                    slot,
                    ..
                },
            }) if slot_id == id && slot == name
        ));
    }
}

#[test]
fn decodes_equippable_entity_tags_and_context_free_empty_sets() {
    use std::collections::BTreeMap;

    use crate::RegistryTagState;

    let mut registries = RegistrySet::default();
    registries.tags.insert(
        ENTITY_TYPE_REGISTRY.to_string(),
        RegistryTagState {
            registry: ENTITY_TYPE_REGISTRY.to_string(),
            tags: BTreeMap::from([(
                "example:llamas".to_string(),
                vec![
                    VANILLA_ENTITY_TYPE_LLAMA_ID,
                    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                ],
            )]),
        },
    );
    let decoded = decode(
        &compound([
            ("slot", string("body")),
            ("allowed_entities", string("#example:llamas")),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Equippable { equippable }) = decoded else {
        panic!("expected equippable component");
    };
    let allowed = equippable.allowed_entities.unwrap();
    assert_eq!(allowed.holder_set_kind, "named");
    assert_eq!(allowed.tag.as_deref(), Some("example:llamas"));

    let decoded = decode(
        &compound([
            ("slot", string("saddle")),
            ("allowed_entities", NbtSummaryValue::List(Vec::new())),
        ]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Equippable { equippable }) = decoded else {
        panic!("expected equippable component");
    };
    assert!(equippable.allowed_entities.unwrap().entries.is_empty());
}

#[test]
fn rejects_malformed_equippable_records() {
    let registries = RegistrySet::default();
    for (value, context, expected) in [
        (NbtSummaryValue::Int(0), None, "is not a compound"),
        (compound([]), None, "missing field slot"),
        (
            compound([("slot", NbtSummaryValue::Int(4))]),
            None,
            "slot is not an equipment-slot name",
        ),
        (
            compound([("slot", string("armor"))]),
            None,
            "unknown dialog item component minecraft:equippable equipment slot",
        ),
        (
            compound([
                ("slot", string("head")),
                ("asset_id", NbtSummaryValue::Int(0)),
            ]),
            None,
            "asset_id is not a resource id",
        ),
        (
            compound([("slot", string("head")), ("camera_overlay", string("Bad"))]),
            None,
            "field camera_overlay resource id Bad",
        ),
        (
            compound([
                ("slot", string("head")),
                ("equip_sound", string("example:missing")),
            ]),
            None,
            "missing minecraft:sound_event resource",
        ),
        (
            compound([("slot", string("head")), ("dispensable", string("yes"))]),
            None,
            "dispensable is not a boolean",
        ),
        (
            compound([
                ("slot", string("head")),
                ("allowed_entities", string("llama")),
            ]),
            None,
            "without registry context",
        ),
        (
            compound([
                ("slot", string("head")),
                ("allowed_entities", string("missing")),
            ]),
            Some(&registries),
            "missing minecraft:entity_type resource",
        ),
    ] {
        let error = decode(&value, context).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn decode(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        "minecraft:equippable",
        value,
        registries,
    )
}

fn named_sound(resource_id: &str) -> DialogSoundEventHolderState {
    DialogSoundEventHolderState::Named {
        registry_id: vanilla_sound_event_registry_id(resource_id).unwrap(),
        resource_id: resource_id.to_string(),
    }
}

fn direct_sound(sound_id: &str, range: Option<f32>) -> NbtSummaryValue {
    let mut entries = vec![("sound_id", string(sound_id))];
    if let Some(range) = range {
        entries.push(("range", NbtSummaryValue::Float(range.to_bits())));
    }
    compound(entries)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.to_string(),
                value,
            })
            .collect(),
    )
}
