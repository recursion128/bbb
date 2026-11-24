use bbb_protocol::NbtSummaryValue;
use serde_json::json;

use super::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_exact_synchronized_registry_element, RegistryElementData, RegistryReferenceLookup,
};

const REGISTRY: &str = "minecraft:trim_material";
const COMPONENT: &str = "minecraft:attribute_modifiers";

struct MaxHealthLookup;

impl RegistryReferenceLookup for MaxHealthLookup {
    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        registry == "minecraft:attribute" && resource_id == "minecraft:max_health"
    }

    fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
        false
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected compound containing {name}")
    };
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn encoded_amount(data: &RegistryElementData) -> f64 {
    let decoded = decode_exact_synchronized_registry_element(REGISTRY, data).unwrap();
    let output = decoded.encode_network(&MaxHealthLookup).unwrap();
    assert_eq!(field(&output, "asset_name"), &string("quartz"));
    let description = field(&output, "description");
    assert_eq!(field(description, "text"), &string("x"));
    let hover = field(description, "hover_event");
    assert_eq!(field(hover, "action"), &string("show_item"));
    assert_eq!(field(hover, "id"), &string("minecraft:stone"));
    let NbtSummaryValue::List(modifiers) = field(field(hover, "components"), COMPONENT) else {
        panic!("expected attribute modifier list")
    };
    assert_eq!(modifiers.len(), 1);
    let modifier = &modifiers[0];
    assert_eq!(field(modifier, "type"), &string("minecraft:max_health"));
    assert_eq!(field(modifier, "id"), &string("minecraft:test"));
    assert_eq!(field(modifier, "operation"), &string("add_value"));
    let NbtSummaryValue::Double(bits) = field(modifier, "amount") else {
        panic!("Codec.DOUBLE must retain a double NBT tag")
    };
    f64::from_bits(*bits)
}

#[test]
fn component_numeric_both_json_sources_publish_finite_amounts() {
    for amount in [0.0_f64, 1.5, -2.25] {
        let bytes = serde_json::to_vec(&json!({
            "asset_name": "quartz",
            "description": {
                "text": "x",
                "hover_event": {
                    "action": "show_item", "id": "stone",
                    "components": {"minecraft:attribute_modifiers": [{
                        "type": "max_health", "id": "test",
                        "amount": amount, "operation": "add_value"
                    }]}
                }
            }
        }))
        .unwrap();
        for data in [
            decode_direct_registry_json(&bytes).unwrap(),
            decode_client_known_pack_registry_json(&bytes).unwrap(),
        ] {
            assert_eq!(encoded_amount(&data).to_bits(), amount.to_bits());
        }
    }
}
