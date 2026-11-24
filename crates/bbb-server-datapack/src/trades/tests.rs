use super::*;
use serde_json::{json, Value};

fn parse_villager_trade(value: &Value) -> DecodeResult<VillagerTrade> {
    super::parse_villager_trade(
        &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(value).unwrap()).unwrap(),
    )
}

fn parse_trade_set(value: &Value) -> DecodeResult<TradeSet> {
    super::parse_trade_set(
        &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(value).unwrap()).unwrap(),
    )
}

fn trade() -> Value {
    json!({"wants":{"id":"emerald"},"gives":"stone"})
}

#[test]
fn trades_defaults_and_lenient_numbers() {
    let parsed = parse_villager_trade(&trade()).unwrap();
    assert_eq!(parsed.wants.item_id, "minecraft:emerald");
    assert_eq!(parsed.wants.count, constant(1));
    assert_eq!(parsed.max_uses, constant(4));
    assert_eq!(parsed.reputation_discount, constant(0));
    assert_eq!(parsed.xp, constant(1));
    assert!(parsed.additional_wants.is_none());
    assert!(parsed.merchant_predicate.is_none());
    assert!(parsed.given_item_modifiers.is_empty());
    assert!(parsed.double_trade_price_enchantments.is_none());
    for invalid in [Value::Null, json!("bad"), json!({"type":"unknown"})] {
        let mut value = trade();
        for key in ["max_uses", "reputation_discount", "xp"] {
            value[key] = invalid.clone();
        }
        assert_eq!(parse_villager_trade(&value).unwrap(), parsed);
    }
}

#[test]
fn trades_holder_sets_preserve_order() {
    let set = |v| parse_trade_set(&json!({"trades":v,"amount":2}));
    assert_eq!(
        set(json!("#wandering_trader/common")).unwrap().trades,
        TradeHolderSet::Tag("minecraft:wandering_trader/common".into())
    );
    assert_eq!(
        set(json!("a")).unwrap().trades,
        TradeHolderSet::Items(vec!["minecraft:a".into()])
    );
    assert_eq!(
        set(json!(["b", "a", "b"])).unwrap().trades,
        TradeHolderSet::Items(vec![
            "minecraft:b".into(),
            "minecraft:a".into(),
            "minecraft:b".into()
        ])
    );
    assert_eq!(
        set(json!([])).unwrap().trades,
        TradeHolderSet::Items(vec![])
    );
}

#[test]
fn trades_template_component_source_retention() {
    for count in [1, 99] {
        let mut value = trade();
        value["gives"] = json!({"id":"stone","count":count,"components":{"!minecraft:custom_name":{},"minecraft:custom_data":{"a":1}}});
        value["wants"]["components"] = json!({"minecraft:custom_data":{"b":2}});
        let parsed = parse_villager_trade(&value).unwrap();
        assert_eq!(parsed.gives.count, count);
        assert_eq!(
            serde_json::to_value(super::Value::Object(parsed.gives.components)).unwrap(),
            value["gives"]["components"]
        );
        assert_eq!(
            serde_json::to_value(super::Value::Object(parsed.wants.components)).unwrap(),
            value["wants"]["components"]
        );
    }
}

#[test]
fn trades_reuse_nested_loot_nodes() {
    let mut value = trade();
    value["wants"]["count"] = json!({"min":1,"max":3});
    value["merchant_predicate"] =
        json!({"condition":"all_of","terms":[{"condition":"random_chance","chance":0.5}]});
    value["given_item_modifiers"] = json!([[{"function":"set_count","count":2}]]);
    value["double_trade_price_enchantments"] = json!("#double_trade_price");
    value["future_field"] = json!(true);
    let parsed = parse_villager_trade(&value).unwrap();
    assert!(matches!(
        parsed.wants.count,
        crate::loot::NumberProvider::Uniform { .. }
    ));
    assert!(matches!(
        parsed.merchant_predicate.unwrap(),
        crate::loot::LootCondition::AllOf(terms) if terms.len() == 1
    ));
    assert!(matches!(
        &parsed.given_item_modifiers[0].kind,
        crate::loot::LootFunctionKind::Sequence(functions) if functions.len() == 1
    ));
    value["given_item_modifiers"] = json!([{"function":"unknown"}]);
    assert!(parse_villager_trade(&value)
        .unwrap_err()
        .path
        .starts_with("given_item_modifiers[0]"));
}

#[test]
fn trades_builtin_wandering_trader_resources_decode() {
    let snapshot = crate::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().unwrap();
    let mut sets = 0;
    let mut trades = 0;
    for path in snapshot.paths() {
        if path.starts_with("trade_set/wandering_trader/") && path.ends_with(".json") {
            let value = serde_json::from_slice(snapshot.read_path(path).unwrap()).unwrap();
            let parsed = parse_trade_set(&value).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!parsed.allow_duplicates);
            assert!(parsed.random_sequence.is_some());
            sets += 1;
        } else if path.starts_with("villager_trade/wandering_trader/") && path.ends_with(".json") {
            let value = serde_json::from_slice(snapshot.read_path(path).unwrap()).unwrap();
            parse_villager_trade(&value).unwrap_or_else(|e| panic!("{path}: {e}"));
            trades += 1;
        }
    }
    assert_eq!(sets, 3);
    assert_eq!(trades, 96);
}
