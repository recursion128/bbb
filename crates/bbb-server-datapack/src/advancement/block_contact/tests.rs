use super::*;
use crate::advancement::parse_advancement;
macro_rules! json {
        (null) => { RegistryJsonValue::Null };
    ($($token:tt)*) => {
        crate::loot::codec::registry_json(&serde_json::json!($($token)*)).unwrap()
    };
}

fn advancement(conditions: RegistryJsonValue) -> RegistryJsonValue {
    json!({"criteria":{"slide":{"trigger":"slide_down_block","conditions":conditions}}})
}

#[test]
fn slide_down_block_defaults_missing_and_null_conditions_and_optional_fields() {
    for body in [
        json!({"criteria":{"slide":{"trigger":"slide_down_block"}}}),
        advancement(json!(null)),
        advancement(json!({})),
        advancement(json!({"player":null,"block":null,"state":null})),
    ] {
        let parsed = parse_advancement(&body).unwrap();
        assert_eq!(
            parsed.criteria[0].block_contact.clone().unwrap(),
            BlockContactCondition::default()
        );
    }
    assert!(parse_block_contact_condition(&RegistryJsonValue::Null).is_err());
}

#[test]
fn slide_down_block_normalizes_block_and_canonicalizes_state_without_value_validation() {
    let condition = parse_block_contact_condition(&json!({
        "player":{"flags":{"is_sneaking":true}},
        "block":"wheat","state":{"age":{"min":"2","max":"7","ignored":true}}
    }))
    .unwrap();
    assert_eq!(condition.block.as_deref(), Some("minecraft:wheat"));
    assert_eq!(
        condition.player,
        Some(bbb_registry_codec::json_to_nbt(&json!({"flags":{"is_sneaking":true}})).unwrap())
    );
    assert_eq!(
        condition.state,
        Some(
            crate::loot::parse_state_properties_predicate(
                &serde_json::json!({"age":{"min":"2","max":"7"}})
            )
            .unwrap()
        )
    );
    for state in [
        json!({"any":"exact"}),
        json!({"any":{"min":"2"}}),
        json!({"any":{}}),
    ] {
        assert_eq!(
            parse_block_contact_condition(&json!({"state":state}))
                .unwrap()
                .state,
            Some(StatePropertiesPredicate::decode(V::Json(&state)).unwrap())
        );
    }
    // StatePropertiesPredicate.checkState checks property names, not matcher values.
    for state in [
        json!({"age":"not_an_age"}),
        json!({"age":{"min":"7","max":"2"}}),
    ] {
        assert!(parse_block_contact_condition(&json!({"block":"wheat","state":state})).is_ok());
    }
    assert!(parse_block_contact_condition(&json!({"block":"honey_block"})).is_ok());
    assert!(parse_block_contact_condition(&json!({"block":"honey_block","state":{}})).is_ok());
}

#[test]
fn slide_down_block_accepts_player_loot_lists_and_retains_canonical_predicates() {
    for player in [
        json!([]),
        json!([{"condition":"random_chance","chance":0.5}]),
    ] {
        assert_eq!(
            parse_block_contact_condition(&json!({"player":player}))
                .unwrap()
                .player,
            Some(
                canonical_advancement_player_predicate_from_source(
                    V::Json(&player),
                    crate::loot::codec::source(V::Json(&player)),
                )
                .unwrap()
            )
        );
    }
}
