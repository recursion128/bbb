use serde_json::{json, Value};

use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element, registry_codec::RegistryReferenceKind,
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry};

use super::*;

fn server(value: &Value) -> RegistryElementData {
    decode_direct_registry_json(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn known(value: &Value) -> RegistryElementData {
    decode_client_known_pack_registry_json(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    let wire = encode_nbt_summary(value, 8 * 1024 * 1024).unwrap();
    decode_network_registry_element(&wire).unwrap()
}

fn decode_data(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_enchantment_element(data)
}

fn minimal_json() -> Value {
    json!({
        "description": "Example",
        "supported_items": "minecraft:stick",
        "weight": 1,
        "max_level": 1,
        "min_cost": {
            "base": 0,
            "per_level_above_first": 0
        },
        "max_cost": {
            "base": 0,
            "per_level_above_first": 0
        },
        "anvil_cost": 0,
        "slots": []
    })
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn list(values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    NbtSummaryValue::List(values)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn int(value: i32) -> NbtSummaryValue {
    NbtSummaryValue::Int(value)
}

fn cost(base: i32, per_level_above_first: i32) -> NbtSummaryValue {
    compound(vec![
        entry("base", int(base)),
        entry("per_level_above_first", int(per_level_above_first)),
    ])
}

fn minimal_expected() -> NbtSummaryValue {
    compound(vec![
        entry("description", string("Example")),
        entry("supported_items", string("minecraft:stick")),
        entry("weight", int(1)),
        entry("max_level", int(1)),
        entry("min_cost", cost(0, 0)),
        entry("max_cost", cost(0, 0)),
        entry("anvil_cost", int(0)),
        entry("slots", list(Vec::new())),
    ])
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[test]
fn direct_known_pack_and_network_sources_write_the_full_flattened_root() {
    let json = json!({
        "description": {"translate": "enchantment.example.root"},
        "supported_items": ["minecraft:stick", "minecraft:bow"],
        "primary_items": "#example:primary_items",
        "weight": 10,
        "max_level": 3,
        "min_cost": {"base": 1, "per_level_above_first": 2},
        "max_cost": {"base": 8, "per_level_above_first": 3},
        "anvil_cost": 4,
        "slots": ["mainhand", "offhand"],
        "exclusive_set": ["example:exclusive"],
        "effects": {"minecraft:prevent_armor_change": {}}
    });
    let expected = compound(vec![
        entry(
            "description",
            compound(vec![entry("translate", string("enchantment.example.root"))]),
        ),
        entry(
            "supported_items",
            list(vec![string("minecraft:stick"), string("minecraft:bow")]),
        ),
        entry("primary_items", string("#example:primary_items")),
        entry("weight", int(10)),
        entry("max_level", int(3)),
        entry("min_cost", cost(1, 2)),
        entry("max_cost", cost(8, 3)),
        entry("anvil_cost", int(4)),
        entry("slots", list(vec![string("mainhand"), string("offhand")])),
        entry("exclusive_set", string("example:exclusive")),
        entry(
            "effects",
            compound(vec![entry(
                "minecraft:prevent_armor_change",
                compound(Vec::new()),
            )]),
        ),
    ]);

    for data in [server(&json), known(&json), network(&expected)] {
        let decoded = decode_data(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 4);
        assert_eq!(references[0].target_registry, ITEM_REGISTRY);
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "minecraft:stick");
        assert_eq!(references[1].target_registry, ITEM_REGISTRY);
        assert_eq!(references[1].kind, RegistryReferenceKind::Element);
        assert_eq!(references[1].resource_id.as_str(), "minecraft:bow");
        assert_eq!(references[2].target_registry, ITEM_REGISTRY);
        assert_eq!(references[2].kind, RegistryReferenceKind::Tag);
        assert_eq!(references[2].resource_id.as_str(), "example:primary_items");
        assert_eq!(references[3].target_registry, ENCHANTMENT_REGISTRY);
        assert_eq!(references[3].kind, RegistryReferenceKind::Element);
        assert_eq!(references[3].resource_id.as_str(), "example:exclusive");
    }
}

#[test]
fn optional_defaults_are_omitted_but_present_primary_empty_set_is_not() {
    let mut defaults = minimal_json();
    let fields = defaults.as_object_mut().unwrap();
    fields.insert("primary_items".to_string(), Value::Null);
    fields.insert("exclusive_set".to_string(), json!([]));
    fields.insert("effects".to_string(), json!({}));
    let defaults = decode_data(&server(&defaults)).unwrap();
    assert_eq!(defaults.network_nbt_unchecked(), &minimal_expected());
    assert!(field(defaults.network_nbt_unchecked(), "primary_items").is_none());
    assert!(field(defaults.network_nbt_unchecked(), "exclusive_set").is_none());
    assert!(field(defaults.network_nbt_unchecked(), "effects").is_none());

    let mut present = minimal_json();
    present
        .as_object_mut()
        .unwrap()
        .insert("primary_items".to_string(), json!([]));
    let present = decode_data(&server(&present)).unwrap();
    assert_eq!(
        field(present.network_nbt_unchecked(), "primary_items"),
        Some(&list(Vec::new()))
    );
}

#[test]
fn compact_holder_sets_keep_tags_empty_sets_and_reference_encounter_order() {
    let mut value = minimal_json();
    let fields = value.as_object_mut().unwrap();
    fields.insert(
        "description".to_string(),
        json!({
            "text": "Example",
            "click_event": {
                "action": "show_dialog",
                "dialog": "example:next"
            }
        }),
    );
    fields.insert("supported_items".to_string(), json!(["stick"]));
    fields.insert("primary_items".to_string(), json!("#example:primary"));
    fields.insert(
        "exclusive_set".to_string(),
        json!(["minecraft:mending", "example:custom"]),
    );
    let decoded = decode_data(&server(&value)).unwrap();
    assert_eq!(
        field(decoded.network_nbt_unchecked(), "supported_items"),
        Some(&string("minecraft:stick"))
    );
    assert_eq!(
        field(decoded.network_nbt_unchecked(), "primary_items"),
        Some(&string("#example:primary"))
    );
    assert_eq!(
        field(decoded.network_nbt_unchecked(), "exclusive_set"),
        Some(&list(vec![
            string("minecraft:mending"),
            string("example:custom"),
        ]))
    );
    let references = decoded.references().collect::<Vec<_>>();
    assert_eq!(
        references
            .iter()
            .map(|reference| (
                reference.target_registry,
                reference.kind,
                reference.resource_id.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "minecraft:dialog",
                RegistryReferenceKind::Element,
                "example:next",
            ),
            (
                ITEM_REGISTRY,
                RegistryReferenceKind::Element,
                "minecraft:stick"
            ),
            (ITEM_REGISTRY, RegistryReferenceKind::Tag, "example:primary"),
            (
                ENCHANTMENT_REGISTRY,
                RegistryReferenceKind::Element,
                "minecraft:mending",
            ),
            (
                ENCHANTMENT_REGISTRY,
                RegistryReferenceKind::Element,
                "example:custom",
            ),
        ]
    );

    let mut empty = minimal_json();
    empty
        .as_object_mut()
        .unwrap()
        .insert("supported_items".to_string(), json!([]));
    let empty = decode_data(&server(&empty)).unwrap();
    assert_eq!(
        field(empty.network_nbt_unchecked(), "supported_items"),
        Some(&list(Vec::new()))
    );
    assert_eq!(empty.references().count(), 0);
}

#[test]
fn description_uses_the_shared_recursive_component_owner() {
    let mut value = minimal_json();
    value
        .as_object_mut()
        .unwrap()
        .insert("description".to_string(), json!(["first", "second"]));
    let decoded = decode_data(&server(&value)).unwrap();
    assert_eq!(
        field(decoded.network_nbt_unchecked(), "description"),
        Some(&compound(vec![
            entry("text", string("first")),
            entry("extra", list(vec![string("second")])),
        ]))
    );
}
