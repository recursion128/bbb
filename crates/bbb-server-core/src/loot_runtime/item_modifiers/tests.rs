use super::*;
use crate::random::GameRandom;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID;
use bbb_server_datapack::loot::{
    parse_loot_function, parse_loot_function_nbt, HolderSet, ItemPredicate, LootRegistries,
};
use serde_json::json;
use std::cell::RefCell;

#[derive(Default)]
struct Environment(RefCell<Vec<i32>>, Option<RuntimeItemStack>);
impl LootItemEnvironment for Environment {
    fn random_enchantment_candidates(
        &self,
        stack: &ItemStackSummary,
        options: Option<&HolderSet>,
        only_compatible: bool,
    ) -> std::result::Result<Vec<LootRandomEnchantment>, LootRuntimeReason> {
        assert_eq!(
            stack.item_id,
            Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_PICKAXE_ID)
        );
        assert!(only_compatible);
        assert_eq!(
            options,
            Some(&HolderSet::Ids(vec![
                "test:unbreaking".into(),
                "test:fortune".into(),
                "test:mending".into()
            ]))
        );
        Ok(vec![
            LootRandomEnchantment {
                id: "test:unbreaking".into(),
                min_level: 1,
                max_level: 3,
            },
            LootRandomEnchantment {
                id: "test:fortune".into(),
                min_level: 2,
                max_level: 5,
            },
            LootRandomEnchantment {
                id: "test:mending".into(),
                min_level: 1,
                max_level: 1,
            },
        ])
    }
    fn with_random_enchantment(
        &self,
        mut stack: ItemStackSummary,
        enchantment: &LootRandomEnchantment,
        level: i32,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        let holder_id = ["test:unbreaking", "test:fortune", "test:mending"]
            .iter()
            .position(|id| *id == enchantment.id)
            .unwrap() as i32;
        stack
            .component_patch
            .enchantments
            .push(bbb_protocol::packets::ItemEnchantmentSummary { holder_id, level });
        Ok(stack)
    }
    fn enchant_with_levels(
        &self,
        stack: ItemStackSummary,
        levels: i32,
        _: Option<&HolderSet>,
        random: &GameRandom,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        self.0.borrow_mut().push(levels);
        self.0.borrow_mut().push(random.next_int(10));
        Ok(stack)
    }
    /// The fixture filter matches when it names no items.
    fn matches_item(
        &self,
        predicate: &ItemPredicate,
        _: &ItemStackSummary,
    ) -> std::result::Result<bool, LootRuntimeReason> {
        Ok(predicate.items.is_none())
    }
    fn smelt_result(
        &self,
        stack: &ItemStackSummary,
    ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
        // The fixture records the call and answers the same stack, which is
        // what a smelting recipe whose result is its own input would produce.
        self.0.borrow_mut().push(stack.count);
        Ok(Some(self.1.clone().unwrap_or_else(|| {
            RuntimeItemStack::from_summary(stack.clone())
        })))
    }
}

#[test]
fn copy_components_transfers_native_values_from_block_entities_and_tools() {
    use crate::block_entity::BlockEntity;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID as LOCK;
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
    let environment = RegistryItemEnvironment::new(&lookup);
    let read = |value: serde_json::Value| {
        let input =
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&value).unwrap())
                .unwrap();
        let raw = input.source_order_nbt_projection().unwrap();
        RuntimeItemStack::try_read(&raw, &lookup).unwrap()
    };
    let source = read(json!({"id":"stone","components":{"lock":{"predicates":{"max_damage":{}}}}}));
    let target = read(json!({"id":"stone","count":3,"components":{
        "custom_name":"Keep", "custom_data":{"target":1}, "lock":{"predicates":{"custom_name":{}}}
    }}));
    let tool = LootTool {
        stack: source.clone(),
        item_id: source.item_id(),
        count: 1,
        enchantments: Default::default(),
    };
    let block_id =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:decorated_pot")
            .unwrap()
            .id;
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block_id).unwrap();
    let raw = bbb_registry_codec::decode_direct_registry_json(br#"{"id":"minecraft:decorated_pot","components":{"lock":{"predicates":{"max_damage":{}}}}}"#)
        .unwrap().source_order_nbt_projection().unwrap();
    let bbb_protocol::NbtSummaryValue::Compound(record) = raw else {
        unreachable!()
    };
    let block = BlockEntity::load_static_with_resolver(
        BlockPos { x: 0, y: 64, z: 0 },
        state,
        &record,
        &lookup,
    )
    .unwrap()
    .unwrap();
    static SET: LootContextParamSet = LootContextParamSet {
        constant_name: "TEST",
        resource_id: "test:copy",
        required: &[],
        optional: LOOT_CONTEXT_PARAMS,
    };
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let params = LootParams {
        tool: Some(&tool),
        item_environment: Some(&environment),
        block_entity: Some(LootBlockEntity {
            block_entity: &block,
            projection: crate::block_entity::BlockEntityItemProjection::with_resolver(&lookup),
        }),
        ..LootParams::new(&SET)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:copy", &params, &random).unwrap();
    for source_id in ["block_entity", "tool"] {
        let function = parse_loot_function(
            &json!({"function":"copy_components", "source":source_id,
            "include":["lock","custom_name","lock","custom_data"], "exclude":["custom_data"]}),
            None,
        )
        .unwrap();
        let copied = evaluator
            .function_runtime(&function, target.clone(), "copy")
            .unwrap();
        let lock = if source_id == "block_entity" {
            block
                .collect_components(
                    crate::block_entity::BlockEntityItemProjection::with_resolver(&lookup),
                )
                .get("minecraft:lock")
                .unwrap()
                .clone()
                .unwrap()
        } else {
            source.component_nbt(LOCK, &lookup).unwrap().unwrap()
        };
        let expected = target
            .clone()
            .with_component_nbt(LOCK, Some(&lock), &lookup)
            .unwrap();
        assert!(copied.same_item_same_components(&expected), "{source_id}");
        assert_eq!(copied.count(), 3);
    }
}
fn stack() -> ItemStackSummary {
    ItemStackSummary {
        count: 1,
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_PICKAXE_ID),
        component_patch: Default::default(),
    }
}
#[test]
fn item_modifiers_random_enchant_adds_an_enchantment_and_trade_cost() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Opaque,
            ..Default::default()
        },
        additional_cost_component_allowed: true,
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::VILLAGER_TRADE)
    };
    let function = parse_loot_function(
        &json!({
            "function": "enchant_randomly",
            "options": ["test:unbreaking", "test:fortune", "test:mending"],
            "include_additional_cost_component": true
        }),
        None,
    )
    .unwrap();
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..300 {
        let enchanted = evaluator.function(&function, stack(), "modifier").unwrap();
        let [enchantment] = &enchanted.component_patch.enchantments[..] else {
            panic!("one enchantment is added")
        };
        // The candidates' level ranges: unbreaking 1-3, fortune 2-5, mending 1.
        let levels = [1..=3, 2..=5, 1..=1][enchantment.holder_id as usize].clone();
        assert!(levels.contains(&enchantment.level));
        seen.insert((enchantment.holder_id, enchantment.level));
        // `2 + nextInt(5 + 10 * level) + 3 * level`.
        let level = enchantment.level;
        let cost = enchanted.component_patch.additional_trade_cost.unwrap();
        assert!((2 + 3 * level..2 + 3 * level + 5 + 10 * level).contains(&cost));
    }
    assert_eq!(seen.len(), 3 + 4 + 1, "every candidate level comes up");
}

#[test]
fn item_modifiers_enchant_with_levels_carries_trade_cost() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Opaque,
            ..Default::default()
        },
        additional_cost_component_allowed: true,
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::VILLAGER_TRADE)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();
    let function = parse_loot_function(&json!({"function":"enchant_with_levels","levels":{"type":"uniform","min":5,"max":19},"include_additional_cost_component":true}), None).unwrap();
    let enchanted = evaluator.function(&function, stack(), "modifier").unwrap();
    let cost = enchanted.component_patch.additional_trade_cost.unwrap();
    assert!((5..=19).contains(&cost), "the cost is the drawn level");
    assert_eq!(
        enchanted.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID]
    );
}
#[test]
fn item_modifiers_filtered_executes_only_selected_branch_and_null_is_absent() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();
    for on_fail in [
        json!(null),
        json!({"function":"set_potion","id":"long_invisibility"}),
    ] {
        let function = parse_loot_function(
            &json!({"function":"filtered","item_filter":{},"on_pass":null,"on_fail":on_fail}),
            None,
        )
        .unwrap();
        assert_eq!(
            evaluator.function(&function, stack(), "modifier").unwrap(),
            stack()
        );
    }
    let function = parse_loot_function(&json!({"function":"filtered","item_filter":{"items":"stone"},"on_fail":{"function":"discard"}}), None).unwrap();
    assert_eq!(
        evaluator.function(&function, stack(), "modifier").unwrap(),
        ItemStackSummary::empty()
    );
}
#[test]
fn item_modifiers_no_additional_cost_without_flag() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let random = GameRandom::with_seed(0);
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let function = parse_loot_function(&json!({"function":"enchant_with_levels","levels":5,"include_additional_cost_component":null}), None).unwrap();
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();
    assert_eq!(
        evaluator
            .function(&function, stack(), "modifier")
            .unwrap()
            .component_patch
            .additional_trade_cost,
        None
    );
}

#[test]
fn item_modifiers_nbt_nested_execution_and_numeric_ops() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:nbt", &params, &random).unwrap();
    for (flag, expected) in [("256", 2), ("257", 3), ("0.5f", 2)] {
        let source = format!(
            r#"{{function: "filtered", item_filter: {{}}, on_pass: {{function: "set_count", count: {{type: "constant", value: 2}}, add: {flag}}}}}"#
        );
        let nbt = bbb_protocol::nbt_summary::parse_snbt_compound(&source).unwrap();
        let function = parse_loot_function_nbt(&nbt, None).unwrap();
        assert_eq!(
            evaluator
                .function(&function, stack(), "modifier")
                .unwrap()
                .count,
            expected
        );
    }
    let nbt = bbb_protocol::nbt_summary::parse_snbt_compound(
        "{function: limit_count, limit: 4294967297L}",
    )
    .unwrap();
    let function = parse_loot_function_nbt(&nbt, None).unwrap();
    let mut input = stack();
    input.count = 5;
    assert_eq!(
        evaluator
            .function(&function, input, "modifier")
            .unwrap()
            .count,
        1
    );
    for (source, expected) in [
        ("{function: limit_count, limit: {min: 7, max: 2}}", 2),
        (
            "{function: set_count, count: 2147483647, add: true}",
            i32::MIN,
        ),
    ] {
        let nbt = bbb_protocol::nbt_summary::parse_snbt_compound(source).unwrap();
        let function = parse_loot_function_nbt(&nbt, None).unwrap();
        assert_eq!(
            evaluator
                .function(&function, stack(), "modifier")
                .unwrap()
                .count,
            expected
        );
    }
}
#[test]
fn a_count_function_sequence_can_revive_a_zero_count_item() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let params = LootParams::new(param_sets::EMPTY);
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:counts", &params, &random).unwrap();
    let function = parse_loot_function(
        &json!([
            {"function": "set_count", "count": 0},
            {"function": "set_count", "count": 1}
        ]),
        None,
    )
    .unwrap();
    let input = stack();
    assert_eq!(
        evaluator
            .function(&function, input.clone(), "modifier")
            .unwrap(),
        input,
    );
    assert_eq!(
        evaluator
            .function_runtime(
                &function,
                RuntimeItemStack::from_summary(input.clone()),
                "modifier",
            )
            .unwrap()
            .into_summary(),
        input,
    );
}

#[test]
fn list_modifiers_preserve_book_fields_and_component_values() {
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
    let environment = RegistryItemEnvironment::new(&lookup);
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&startup.loot, "test:lists", &params, &random).unwrap();
    let read = |value: serde_json::Value| {
        let raw =
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .source_order_nbt_projection()
                .unwrap();
        RuntimeItemStack::try_read(&raw, &lookup).unwrap()
    };
    let input = read(json!({"id":"written_book", "components":{
        "written_book_content":{"title":"Cover", "author":"Writer", "generation":2,
            "resolved":true, "pages":["First", "Last"]},
        "lore":["Before"],
        "enchantments":{"unbreaking":2,"mending":1},
        "attribute_modifiers":[{"type":"attack_damage", "id":"test:power", "amount":1.0,
            "operation":"add_value", "slot":"mainhand"}]
    }}));
    let function = parse_loot_function(
        &json!([
            {"function":"set_written_book_pages", "mode":"insert", "offset":1, "pages":["Middle"]},
            {"function":"set_written_book_pages", "mode":"insert", "offset":99, "pages":["Unused"]},
            {"function":"set_book_cover", "title":"Rebound"},
            {"function":"set_lore", "mode":"append", "lore":["After"]},
            {"function":"set_enchantments", "enchantments":{"unbreaking":3}, "add":true},
            {"function":"set_enchantments", "enchantments":{"mending":0}},
            {"function":"set_banner_pattern", "append":false,
                "patterns":[{"pattern":"stripe_bottom", "color":"red"}]},
            {"function":"set_banner_pattern", "append":true,
                "patterns":[{"pattern":"stripe_top", "color":"blue"}]},
            {"function":"set_attributes", "replace":false, "modifiers":[{
                "attribute":"attack_damage", "id":"test:power", "amount":3.0,
                "operation":"add_value", "slot":"mainhand"}]}
        ]),
        None,
    )
    .unwrap();
    let actual = evaluator
        .function_runtime(&function, input, "modifier")
        .unwrap();
    let expected = read(json!({"id":"written_book", "components":{
        "written_book_content":{"title":"Rebound", "author":"Writer", "generation":2,
            "resolved":false, "pages":["First", "Middle", "Last"]},
        "lore":["Before", "After"],
        "enchantments":{"unbreaking":5},
        "banner_patterns":[{"pattern":"stripe_bottom", "color":"red"},
            {"pattern":"stripe_top", "color":"blue"}],
        "attribute_modifiers":[{"type":"attack_damage", "id":"test:power", "amount":3.0,
            "operation":"add_value", "slot":"mainhand"}]
    }}));
    assert_eq!(actual, expected);
}

#[test]
fn damage_modifiers_use_durability_and_preserve_other_components() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as DAMAGE,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID as UNBREAKABLE,
    };
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
    let environment = RegistryItemEnvironment::new(&lookup);
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&startup.loot, "test:damage", &params, &random).unwrap();
    let read = |value: serde_json::Value| {
        let raw =
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .source_order_nbt_projection()
                .unwrap();
        RuntimeItemStack::try_read(&raw, &lookup).unwrap()
    };
    let input_json = json!({"id":"iron_pickaxe", "components":{
        "max_damage":8, "damage":3, "custom_data":{"kept":17}
    }});
    let input = read(input_json.clone());
    for (fraction, add, expected_damage) in [(0.4, false, 4), (0.125, true, 2)] {
        let function = parse_loot_function(
            &json!({"function":"set_damage", "damage":fraction, "add":add}),
            None,
        )
        .unwrap();
        let actual = evaluator
            .function_runtime(&function, input.clone(), "modifier")
            .unwrap();
        let mut expected = input_json.clone();
        expected["components"]["damage"] = json!(expected_damage);
        assert_eq!(actual, read(expected));
    }
    let unavailable_provider = parse_loot_function(
        &json!({"function":"set_damage", "damage":{
            "type":"enchantment_level", "amount":1
        }}),
        None,
    )
    .unwrap();
    assert!(evaluator
        .function_runtime(&unavailable_provider, input.clone(), "modifier")
        .is_err());
    let removed = input
        .clone()
        .with_component_nbt(DAMAGE, None, &lookup)
        .unwrap();
    let unbreakable = input
        .with_component_nbt(
            UNBREAKABLE,
            Some(&bbb_protocol::NbtSummaryValue::Compound(Vec::new())),
            &lookup,
        )
        .unwrap();
    for unchanged in [removed, unbreakable, read(json!({"id":"stone", "count":3}))] {
        let actual = evaluator
            .function_runtime(&unavailable_provider, unchanged.clone(), "modifier")
            .unwrap();
        assert_eq!(actual, unchanged);
    }
}

#[test]
fn item_modifiers_set_components_applies_the_patch() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = Environment::default();
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();

    // `!` marks a removal (`SetComponentsFunction.java:29-32`).
    let function = parse_loot_function(
        &json!({
            "function": "set_components",
            "components": {
                "minecraft:repair_cost": 7,
                "!minecraft:damage": {}
            }
        }),
        None,
    )
    .unwrap();
    let result = evaluator.function(&function, stack(), "modifier").unwrap();
    assert_eq!(result.component_patch.repair_cost, Some(7));
    assert!(result
        .component_patch
        .removed_type_ids
        .contains(&bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID));
}

// SetOminousBottleAmplifierFunction.java:40-44.
#[test]
fn item_modifiers_ominous_bottle_amplifier_clamps() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID;

    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let environment = RegistryItemEnvironment::new(crate::inventory::test_item_resolver());
    let input = RuntimeItemStack::from_summary(stack());
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(0);
    let mut evaluator = LootEvaluator::new(&tables, "test:trade", &params, &random).unwrap();
    for (amplifier, expected) in [(json!(3), 3), (json!(9), 4), (json!(-3), 0)] {
        let function = parse_loot_function(
            &json!({
                "function": "set_ominous_bottle_amplifier",
                "amplifier": amplifier,
            }),
            None,
        )
        .unwrap();
        let modified = evaluator
            .function_runtime(&function, input.clone(), "modifier")
            .unwrap();
        assert_eq!(
            environment
                .component_runtime(
                    &modified,
                    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
                )
                .unwrap(),
            Some(bbb_protocol::NbtSummaryValue::Int(expected)),
            "{amplifier}"
        );
    }
}

#[test]
fn item_modifiers_smelt_keeps_recipe_output_through_filtered_decorations_and_split() {
    use bbb_server_datapack::{
        frozen_lookup::FrozenDatapackRegistryLookup,
        recipe::{parse_recipe_source_with_lookup, CookingKind},
        startup::{DatapackStartup, DatapackStartupOptions},
    };
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
    let parse_recipe = |value: &serde_json::Value| {
        let bytes = serde_json::to_vec(value).unwrap();
        let decoded = bbb_registry_codec::decode_direct_registry_json(&bytes).unwrap();
        parse_recipe_source_with_lookup(decoded.root(), &lookup)
    };
    let held = parse_recipe(&json!({
        "type": "smelting", "ingredient": "iron_pickaxe",
        "result": {"id": "iron_ingot", "count": 2,
            "components": {"max_stack_size": 16, "custom_data": {"from_recipe": 1},
                "lock": {"predicates": {"custom_name": {}}}}}
    }))
    .unwrap()
    .cooking_of_kind(CookingKind::Smelting)
    .unwrap()
    .assemble();
    let held = RuntimeItemStack::from_summary(held);
    let environment = Environment(RefCell::new(Vec::new()), Some(held.clone()));
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let params = LootParams {
        item_environment: Some(&environment),
        ..LootParams::new(param_sets::EMPTY)
    };
    let table = bbb_server_datapack::loot::parse_loot_table(&json!({
        "pools": [{"rolls": 1, "entries": [{"type": "item", "name": "minecraft:iron_pickaxe", "functions": [
            {"function": "filtered", "item_filter": {}, "on_pass":
                {"function": "furnace_smelt", "use_input_count": true,
                    "conditions": [{"condition": "random_chance", "chance": 1}]}}
        ]}], "functions": [{"function": "set_count", "count": 33}]}],
        "functions": [{"function": "set_count", "count": 2, "add": true}]
    }), None)
    .unwrap();
    let random = GameRandom::with_seed(0);
    let mut eval = Eval::new("test:smelt", &tables, &params, &random).unwrap();
    let fresh = eval.get_random_items(&table).unwrap();
    let input = parse_recipe(&json!({
        "type": "smelting", "ingredient": "stone",
        "result": {"id": "iron_pickaxe", "components": {"custom_data": {"input_only": 1}}}
    }))
    .unwrap()
    .cooking_of_kind(CookingKind::Smelting)
    .unwrap()
    .assemble();
    let outer = Decoration {
        functions: &table.functions,
        path: "functions",
        outer: None,
    };
    let pool = Decoration {
        functions: &table.pools[0].functions,
        path: "pool.functions",
        outer: Some(&outer),
    };
    let decoration = Decoration {
        functions: &table.pools[0].entries[0].functions,
        path: "entry.functions",
        outer: Some(&pool),
    };
    let mut existing = Vec::new();
    eval.emit(
        RuntimeItemStack::from_summary(input).into_summary(),
        Some(&decoration),
        "entry",
        &mut existing,
    )
    .unwrap();
    for output in [&fresh, &existing] {
        assert_eq!(
            output.iter().map(stack_count).collect::<Vec<_>>(),
            [16, 16, 3]
        );
        for stack in output {
            let stack = RuntimeItemStack::from_summary(stack.clone());
            assert!(stack.same_item_same_components(&held));
        }
    }
}
