use super::*;
use serde_json::json;

#[test]
fn advancement_player_accepts_entity_objects_and_loot_condition_lists() {
    for value in [
        json!({}),
        json!([]),
        json!({"flags":{"is_sneaking":true},"vehicle":{"type":"oak_boat"}}),
        json!([{"condition":"entity_properties","entity":"this","predicate":{"periodic_tick":5}}]),
        json!([{"condition":"random_chance","chance":0.5}]),
        json!([[{"condition":"inverted","term":{"condition":"survives_explosion"}}]]),
    ] {
        let data =
            crate::decode_direct_registry_json(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            canonical_advancement_player_predicate_from_source(data.root(), data.source).is_ok(),
            "{value}"
        );
    }
}

#[test]
fn advancement_player_delegates_nested_validation_to_existing_owners() {
    for value in [
        json!(null),
        json!(42),
        json!([null]),
        json!([{}]),
        json!({"periodic_tick":0}),
        json!({"flags":false}),
        json!({"vehicle":{"periodic_tick":0}}),
        json!([{"condition":"entity_properties","entity":"this","predicate":{"periodic_tick":0}}]),
        json!([{"condition":"random_chance","chance":"bad"}]),
        json!([{"condition":"not_a_condition"}]),
    ] {
        let decoded = crate::decode_direct_registry_json(&serde_json::to_vec(&value).unwrap());
        if value.is_null() {
            assert!(decoded.is_err(), "{value}");
            continue;
        }
        let data = decoded.unwrap();
        assert!(
            canonical_advancement_player_predicate_from_source(data.root(), data.source).is_err(),
            "{value}"
        );
    }
}

#[test]
fn direct_player_source_preserves_raw_ignored_field_budgets() {
    let wide = vec!["0"; 4097].join(",");
    let deep = format!("{}0{}", "[".repeat(80), "]".repeat(80));
    for (depth, accepted) in [(63, true), (64, false)] {
        let empty = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let raw = format!(r#"{{"ignored":{empty}}}"#);
        let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
        assert_eq!(
            canonical_advancement_player_predicate_from_source(data.root(), data.source).is_ok(),
            accepted
        );
    }

    let within_collection = format!("[{}]", vec!["0"; 4096].join(","));
    let raw = format!(
        r#"{{"ignored":[{}]}}"#,
        vec![within_collection; 17].join(",")
    );
    let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
    assert!(canonical_advancement_player_predicate_from_source(data.root(), data.source).is_ok());
    for raw in [
        format!(r#"{{"ignored":[{wide}]}}"#),
        format!(r#"{{"ignored":{deep}}}"#),
    ] {
        let data = crate::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
        assert!(
            canonical_advancement_player_predicate_from_source(data.root(), data.source).is_err()
        );
    }
}
