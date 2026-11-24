use super::*;
use serde_json::json;

#[test]
fn nbt_and_json_tables_decode_to_the_same_values() {
    let nbt = bbb_protocol::nbt_summary::parse_snbt_compound(
        r#"{pools: [{rolls: {type: "sum", summands: [1, 2]}, entries: [{
            type: "item", name: "minecraft:stone", weight: 3,
            functions: [{function: "set_count", count: 2b,
                conditions: [{condition: "random_chance", chance: 0.5f}]}]
        }]}]}"#,
    )
    .unwrap();
    let from_nbt = parse_loot_table_nbt(&nbt, None).unwrap();
    let from_json = parse_loot_table(
        &json!({"pools": [{
            "rolls": {"type": "sum", "summands": [1, 2]},
            "entries": [{"type": "item", "name": "stone", "weight": 3, "functions": [{
                "function": "set_count", "count": 2,
                "conditions": [{"condition": "random_chance", "chance": 0.5}]
            }]}]
        }]}),
        None,
    )
    .unwrap();
    assert_eq!(from_nbt, from_json);
    let pool = &from_json.pools[0];
    assert_eq!(
        pool.rolls,
        NumberProvider::Sum(vec![
            NumberProvider::Constant(1.0),
            NumberProvider::Constant(2.0)
        ])
    );
    assert_eq!(pool.bonus_rolls, NumberProvider::Constant(0.0));
    assert_eq!(pool.entries[0].weight, 3);
    let function = &pool.entries[0].functions[0];
    assert_eq!(
        function.kind,
        LootFunctionKind::SetCount {
            count: NumberProvider::Constant(2.0),
            add: false
        }
    );
    assert_eq!(
        function.conditions,
        vec![LootCondition::RandomChance(NumberProvider::Constant(0.5))]
    );
}

#[test]
fn inline_arrays_are_all_of_and_sequence() {
    let condition = parse_loot_condition(
        &json!([
            {"condition": "survives_explosion"},
            {"condition": "minecraft:inverted", "term": {"condition": "killed_by_player"}}
        ]),
        None,
    )
    .unwrap();
    assert_eq!(
        condition,
        LootCondition::AllOf(vec![
            LootCondition::SurvivesExplosion,
            LootCondition::Inverted(Box::new(LootCondition::KilledByPlayer)),
        ])
    );
    let function = parse_loot_function(&json!([{"function": "explosion_decay"}]), None).unwrap();
    assert!(
        matches!(&function.kind, LootFunctionKind::Sequence(functions)
        if functions[0].kind == LootFunctionKind::ExplosionDecay)
    );
    let source = bbb_registry_codec::decode_direct_registry_json(
        br#"{"function":"set_custom_model_data","strings":{"mode":"replace_all","values":["\ud800"]}}"#,
    ).unwrap();
    let from_json = LootFunction::decode(source.root(), None).unwrap();
    let from_nbt =
        parse_loot_function_nbt(&source.source_order_nbt_projection().unwrap(), None).unwrap();
    assert_eq!(from_json, from_nbt);
    let LootFunctionKind::SetCustomModelData {
        strings: Some((_, strings)),
        ..
    } = from_json.kind
    else {
        panic!("custom model data strings");
    };
    assert_eq!(
        strings,
        vec![NbtSummaryValue::string_from_utf16_slice(&[0xd800])]
    );
}

#[test]
fn number_providers_accept_constants_inline_uniform_and_reject_unknown_types() {
    assert_eq!(
        parse_loot_number_provider(&json!(3)).unwrap(),
        NumberProvider::Constant(3.0)
    );
    assert_eq!(
        parse_loot_number_provider(&json!({"min": 1, "max": 2})).unwrap(),
        NumberProvider::Uniform {
            min: Box::new(NumberProvider::Constant(1.0)),
            max: Box::new(NumberProvider::Constant(2.0)),
        }
    );
    assert!(parse_loot_number_provider(&json!({"foo": 1})).is_err());
    assert!(parse_loot_number_provider(&json!({"type": "scoreboard"})).is_err());
}

#[test]
fn environment_attribute_values_resolve_and_validate_at_decode() {
    use bbb_facts::environment_attribute_types::{
        vanilla_environment_attribute, VanillaEnvironmentScalarValue as Scalar,
        VANILLA_ENVIRONMENT_ATTRIBUTES,
    };
    for attribute in &VANILLA_ENVIRONMENT_ATTRIBUTES {
        let number = parse_loot_number_provider(&json!({
            "type": "environment_attribute", "attribute": attribute.resource_id
        }));
        match attribute.scalar_default {
            Some(scalar) => {
                let value = match scalar {
                    Scalar::Boolean(value) => json!(value),
                    Scalar::FloatBits(bits) => json!(f32::from_bits(bits)),
                };
                assert_eq!(
                    parse_loot_condition(
                        &json!({"condition": "environment_attribute_check",
                        "attribute": attribute.resource_id, "value": value}),
                        None
                    )
                    .unwrap(),
                    LootCondition::EnvironmentAttributeCheck {
                        attribute,
                        value: EnvironmentAttributeExpectedValue::Scalar(scalar),
                    }
                );
                assert_eq!(number.is_ok(), matches!(scalar, Scalar::FloatBits(_)));
                if let Ok(number) = number {
                    assert_eq!(number, NumberProvider::EnvironmentAttribute(attribute));
                }
            }
            None => assert!(number.is_err()),
        }
    }
    assert!(matches!(
        parse_loot_condition(&json!({"condition":"environment_attribute_check",
            "attribute":"gameplay/villager_activity","value":"work"}), None).unwrap(),
        LootCondition::EnvironmentAttributeCheck {
            value: EnvironmentAttributeExpectedValue::Activity(activity), ..
        } if activity.resource_id == "minecraft:work"
    ));
    assert_eq!(
        parse_loot_condition(
            &json!({"condition":"environment_attribute_check",
            "attribute":"gameplay/eyeblossom_open","value":"default"}),
            None
        )
        .unwrap(),
        LootCondition::EnvironmentAttributeCheck {
            attribute: vanilla_environment_attribute("minecraft:gameplay/eyeblossom_open").unwrap(),
            value: EnvironmentAttributeExpectedValue::TriState(
                bbb_server_types::environment::TriState::Default,
            ),
        }
    );
    assert!(matches!(
        parse_loot_condition(&json!({"condition":"environment_attribute_check",
            "attribute":"gameplay/bed_rule","value":{"can_sleep":"when_dark","can_set_spawn":"always"}}), None).unwrap(),
        LootCondition::EnvironmentAttributeCheck {
            value: EnvironmentAttributeExpectedValue::BedRule(
                bbb_server_types::environment::BedRule { explodes: false, error_message: None, .. }
            ), ..
        }
    ));
    let color = parse_loot_condition(
        &json!({"condition":"environment_attribute_check",
        "attribute":"visual/sky_color","value":123}),
        None,
    )
    .unwrap();
    assert_eq!(
        parse_loot_condition(
            &json!({"condition":"environment_attribute_check",
            "attribute":"visual/moon_phase","value":"third_quarter"}),
            None
        )
        .unwrap(),
        LootCondition::EnvironmentAttributeCheck {
            attribute: vanilla_environment_attribute("minecraft:visual/moon_phase").unwrap(),
            value: EnvironmentAttributeExpectedValue::MoonPhase(
                bbb_server_types::environment::MoonPhase::from_serialized_name("third_quarter")
                    .unwrap()
            ),
        }
    );
    assert!(matches!(color, LootCondition::EnvironmentAttributeCheck {
        attribute, value: EnvironmentAttributeExpectedValue::Color(123)
    } if attribute == vanilla_environment_attribute("minecraft:visual/sky_color").unwrap()));
    for (attribute, value) in [
        ("gameplay/fast_lava", json!(1)),
        ("visual/sun_angle", json!(true)),
        ("visual/fog_end_distance", json!(-1.0)),
        ("gameplay/sky_light_level", json!(16.0)),
        ("audio/music_volume", json!(1.1)),
        ("gameplay/villager_activity", json!("bbb:missing")),
        ("visual/moon_phase", json!("missing")),
        ("gameplay/eyeblossom_open", json!("missing")),
        (
            "gameplay/bed_rule",
            json!({"can_sleep":"missing","can_set_spawn":"always"}),
        ),
        ("bbb:missing", json!(false)),
    ] {
        assert!(
            parse_loot_condition(
                &json!({"condition":"environment_attribute_check",
            "attribute":attribute,"value":value}),
                None
            )
            .is_err(),
            "{attribute}"
        );
    }
    assert!(parse_loot_number_provider(
        &json!({"type":"environment_attribute", "attribute":"bbb:missing"})
    )
    .is_err());

    use bbb_facts::environment_attribute_types::{
        VanillaEnvironmentAttributeRange as Range, VanillaEnvironmentAttributeType as AttrType,
    };
    let integer_attr = |range| -> &'static _ {
        let mut attr =
            *vanilla_environment_attribute("minecraft:gameplay/sky_light_level").unwrap();
        attr.value_type = AttrType::Integer;
        attr.value_range = range;
        attr.scalar_default = None;
        Box::leak(Box::new(attr))
    };
    let decode = |attr: &'static _, raw: serde_json::Value| {
        let reg = if raw.is_null() {
            bbb_registry_codec::RegistryJsonValue::Null
        } else {
            super::codec::registry_json(&raw).unwrap()
        };
        super::condition::decode_environment_attribute_value(attr, V::Json(&reg), None)
    };
    let any_attr = integer_attr(Range::Any);
    for valid in [i32::MIN, -42, 0, 1000, i32::MAX] {
        assert_eq!(
            decode(any_attr, json!(valid)).unwrap(),
            EnvironmentAttributeExpectedValue::Integer(valid)
        );
    }
    for invalid in [json!(true), json!("4"), json!({}), json!([]), json!(null)] {
        assert!(decode(any_attr, invalid).is_err());
    }
    let non_negative = integer_attr(Range::NonNegativeFloat);
    assert_eq!(
        decode(non_negative, json!(0)).unwrap(),
        EnvironmentAttributeExpectedValue::Integer(0)
    );
    assert!(decode(non_negative, json!(-1)).is_err());

    let sky_light = integer_attr(Range::SkyLight);
    assert_eq!(
        decode(sky_light, json!(15)).unwrap(),
        EnvironmentAttributeExpectedValue::Integer(15)
    );
    assert!(decode(sky_light, json!(-1)).is_err());
    assert!(decode(sky_light, json!(16)).is_err());

    let unit = integer_attr(Range::UnitFloat);
    assert_eq!(
        decode(unit, json!(1)).unwrap(),
        EnvironmentAttributeExpectedValue::Integer(1)
    );
    assert!(decode(unit, json!(-1)).is_err());
    assert!(decode(unit, json!(2)).is_err());
}

#[test]
fn table_param_set_is_lenient_and_defaults_to_generic() {
    // `lenientOptionalFieldOf("type", DEFAULT_PARAM_SET)` answers `ALL_PARAMS`
    // (`generic`) for an absent or unregistered set.
    let param_set = |value| parse_loot_table(&value, None).unwrap().param_set;
    assert_eq!(
        param_set(json!({"type": "modded:context"})),
        "minecraft:generic"
    );
    assert_eq!(param_set(json!({"type": "block"})), "minecraft:block");
    assert_eq!(param_set(json!({})), "minecraft:generic");
}

#[test]
fn illegal_documents_are_rejected_at_decode() {
    for table in [
        json!({"pools": [{"rolls": 1, "entries": [{"type": "item", "name": "minecraft:air"}]}]}),
        json!({"pools": [{"rolls": 1, "entries": [{"type": "item", "name": "bbb:missing"}]}]}),
        json!({"pools": [{"rolls": 1, "entries": [{"type": "minecraft:mystery"}]}]}),
        json!({"functions": [{"function": "set_count"}]}),
        json!({"functions": [{"function": "apply_bonus", "enchantment": "fortune",
            "formula": "minecraft:uniform_bonus_count"}]}),
        json!({"pools": [{"rolls": 1, "entries": [],
            "conditions": [{"condition": "table_bonus", "enchantment": "fortune", "chances": []}]}]}),
    ] {
        assert!(parse_loot_table(&table, None).is_err(), "{table}");
    }
}

#[test]
fn entity_and_block_components_retain_the_same_typed_matchers_as_items() {
    let matchers = json!({
        "components": {"custom_data": {"seal": 7}},
        "predicates": {"custom_data": {"seal": 7}, "custom_name": {}}
    });
    let item = parse_item_predicate(&matchers).unwrap();
    let entity = from_json(&matchers, EntityPredicate::decode).unwrap();
    let block = parse_location_predicate(&json!({"block": matchers}))
        .unwrap()
        .block
        .unwrap();
    let expected = bbb_protocol::nbt_summary::parse_snbt_compound("{seal: 7b}").unwrap();
    assert_eq!(
        block.components,
        vec![("minecraft:custom_data".to_string(), expected.clone())]
    );
    assert_eq!(
        block.predicates,
        vec![
            ComponentPredicate::CustomData(expected),
            ComponentPredicate::AnyValue("minecraft:custom_name".to_string()),
        ]
    );
    assert_eq!(block.components, item.components);
    assert_eq!(block.predicates, item.predicates);
    assert_eq!(entity.components, item.components);
    assert_eq!(entity.predicates, item.predicates);
    assert!(from_json(&json!({}), EntityPredicate::decode)
        .unwrap()
        .is_empty());
    assert!(from_json(
        &json!({"components": {}, "predicates": {}}),
        EntityPredicate::decode
    )
    .unwrap()
    .is_empty());
    assert_eq!(
        parse_location_predicate(&json!({"block": {}}))
            .unwrap()
            .block,
        Some(BlockPredicate::default())
    );
    assert_eq!(
        parse_location_predicate(&json!({"block": {"components": {}, "predicates": {}}}))
            .unwrap()
            .block,
        Some(BlockPredicate::default())
    );
}

#[test]
fn direct_predicate_sources_preserve_ignored_field_collection_budgets() {
    let wide = vec!["0"; 4097].join(",");
    let raw = format!(r#"{{"ignored":[{wide}]}}"#);
    let data = bbb_registry_codec::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
    assert!(parse_item_predicate_source(data.root()).is_err());
    assert!(parse_location_predicate_source(data.root()).is_err());
    let raw = format!(r#"{{"condition":"random_chance","chance":1,"ignored":[{wide}]}}"#);
    let data = bbb_registry_codec::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
    assert!(parse_loot_condition_source(data.root(), None).is_err());
    let raw = format!(r#"{{"speed":{{"ignored":[{wide}]}}}}"#);
    let data = bbb_registry_codec::decode_loaded_registry_json(raw.as_bytes(), raw.len()).unwrap();
    assert!(parse_mob_effects_predicate_source(data.root()).is_err());
}
