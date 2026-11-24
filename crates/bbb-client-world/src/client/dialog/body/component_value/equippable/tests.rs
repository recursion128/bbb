use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    entity_types::{VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID},
    packets::{EquippableSummary, HolderSetSummary, SoundEventSummary},
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

    let mut registries = RegistrySet::default();
    let mut contents = crate::RegistryContentState::new(ENTITY_TYPE_REGISTRY);
    contents.entries = vec![
        crate::RegistryPacketEntry::stub("minecraft:trader_llama"),
        crate::RegistryPacketEntry::stub("minecraft:llama"),
    ];
    registries
        .contents
        .insert(ENTITY_TYPE_REGISTRY.to_string(), contents);
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
        [(1, "minecraft:llama"), (0, "minecraft:trader_llama"),]
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
        RegistryTagState::from_materialized(
            ENTITY_TYPE_REGISTRY.to_string(),
            BTreeMap::from([(
                "example:llamas".to_string(),
                vec![
                    VANILLA_ENTITY_TYPE_LLAMA_ID,
                    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                ],
            )]),
        ),
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
fn projects_equippable_into_protocol_summary() {
    let registries = RegistrySet::default();
    let value = compound([
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
    ]);
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        &value,
        Some(&registries),
    );
    assert_eq!(
        patch.equippable,
        Some(EquippableSummary {
            slot_id: 6,
            equip_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:equip".to_string()),
                fixed_range_bits: Some(6.5f32.to_bits()),
            },
            asset_id: Some("example:llama_carpet".to_string()),
            camera_overlay: Some("minecraft:overlay/powder_snow".to_string()),
            allowed_entities: Some(HolderSetSummary {
                registry_ids: vec![
                    VANILLA_ENTITY_TYPE_LLAMA_ID,
                    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
                ],
                tag: None,
            }),
            dispensable: false,
            swappable: false,
            damage_on_hurt: false,
            equip_on_interact: true,
            can_be_sheared: true,
            shearing_sound: SoundEventSummary {
                registry_id: Some(
                    vanilla_sound_event_registry_id("minecraft:item.shears.snip").unwrap(),
                ),
                sound_id: None,
                fixed_range_bits: None,
            },
        })
    );
}

#[test]
fn rejects_malformed_equippable_records() {
    let registries = RegistrySet::default();
    for (value, context) in [
        (NbtSummaryValue::Int(0), None),
        (compound([]), None),
        (compound([("slot", NbtSummaryValue::Int(4))]), None),
        (compound([("slot", string("armor"))]), None),
        (
            compound([
                ("slot", string("head")),
                ("asset_id", NbtSummaryValue::Int(0)),
            ]),
            None,
        ),
        (
            compound([("slot", string("head")), ("camera_overlay", string("Bad"))]),
            None,
        ),
        (
            compound([
                ("slot", string("head")),
                ("equip_sound", string("example:missing")),
            ]),
            None,
        ),
        (
            compound([("slot", string("head")), ("dispensable", string("yes"))]),
            None,
        ),
        (
            compound([
                ("slot", string("head")),
                ("allowed_entities", string("llama")),
            ]),
            None,
        ),
        (
            compound([
                ("slot", string("head")),
                ("allowed_entities", string("missing")),
            ]),
            Some(&registries),
        ),
    ] {
        let _ = decode(&value, context).unwrap_err();
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
    NbtSummaryValue::string(value)
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}
