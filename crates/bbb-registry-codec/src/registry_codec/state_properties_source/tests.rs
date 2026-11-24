use super::*;
use bbb_protocol::{json_value_to_nbt_summary, NbtSummaryEntry};
use serde_json::json;

#[test]
fn state_properties_canonicalizes_exact_ranges_and_optional_null_bounds() {
    for (input, expected) in [
        (json!({}), json!({})),
        (json!({"facing":"north"}), json!({"facing":"north"})),
        (json!({"age":{"min":null}}), json!({"age":{}})),
    ] {
        let data =
            crate::decode_direct_registry_json(&serde_json::to_vec(&input).unwrap()).unwrap();
        assert_eq!(
            canonical_state_properties_predicate_from_source(data.root(), data.source).unwrap(),
            json_value_to_nbt_summary(&expected).unwrap(),
        );
    }
    let data =
        crate::decode_direct_registry_json(br#"{"age":{"min":"2","max":"7","ignored":true}}"#)
            .unwrap();
    assert_eq!(
        canonical_state_properties_predicate_from_source(data.root(), data.source).unwrap(),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "age".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "min".into(),
                    value: NbtSummaryValue::String("2".into())
                },
                NbtSummaryEntry {
                    name: "max".into(),
                    value: NbtSummaryValue::String("7".into())
                },
            ]),
        }]),
    );
}

#[test]
fn state_properties_preserves_raw_java_property_names_and_values() {
    let data = crate::decode_direct_registry_json(br#"{"\ud800":"\udfff"}"#).unwrap();
    assert_eq!(
        canonical_state_properties_predicate_from_source(data.root(), data.source).unwrap(),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: vec![0xd800].into(),
            value: NbtSummaryValue::string_from_utf16(vec![0xdfff]),
        }]),
    );
}

#[test]
fn direct_state_source_preserves_ignored_matcher_field_budgets() {
    let wide = vec!["0"; 4097].join(",");
    let raw = format!(r#"{{"age":{{"min":"2","ignored":[{wide}]}}}}"#);
    let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
    assert!(canonical_state_properties_predicate_from_source(data.root(), data.source).is_err());
}
