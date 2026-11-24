use super::*;
use crate::loot_runtime::{LootEntityFacts, LootRuntimeError};
use crate::random::GameRandom;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_server_datapack::loot::{parse_loot_condition, parse_loot_function, LootRegistries};
use serde_json::{json, Value};

use super::super::{LootDamageSource, LootEntities, LootEntity, LootOrigin, LootParams};

/// A living attacker holding a main-hand stack with `minecraft:looting` at
/// `level`.
fn attacker(level: i32) -> LootEntityFacts {
    use bbb_protocol::equipment_slots::EquipmentSlot;
    LootEntityFacts {
        living: Some(true),
        equipment: Some(std::collections::BTreeMap::from([(
            EquipmentSlot::MainHand,
            crate::loot_runtime::LootTool {
                stack: Default::default(),
                item_id: Some(1),
                count: 1,
                enchantments: std::collections::BTreeMap::from([(
                    "minecraft:looting".into(),
                    level,
                )]),
            },
        )])),
        enchantment_slots: Some(std::sync::Arc::new(vec![
            crate::enchantment_facts::EnchantmentSlots {
                resource_id: "minecraft:looting".into(),
                slots: vec![EquipmentSlot::MainHand],
            },
        ])),
        ..LootEntityFacts::default()
    }
}

fn stack(name: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            vanilla_item_type_for_resource_id(name)
                .expect("builtin item")
                .id,
        ),
        count,
        ..ItemStackSummary::empty()
    }
}

fn tables() -> LootRegistries {
    LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    }
}

fn entity_params<'a>(
    attacker: LootEntity<'a>,
    last_damage_player: LootEntity<'a>,
) -> LootParams<'a> {
    LootParams {
        origin: Some(LootOrigin {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Opaque,
            attacking_entity: attacker,
            last_damage_player,
            ..LootEntities::default()
        },
        damage_source: LootDamageSource::Present(crate::loot_runtime::LootDamageSourceFacts {
            damage_type: "minecraft:generic",
            is_direct: Some(true),
            tags: None,
            causing: LootEntity::Absent,
            direct: Some(LootEntity::Absent),
        }),
        ..LootParams::new(super::super::param_sets::ENTITY)
    }
}

fn apply<'a>(
    function: Value,
    stack: ItemStackSummary,
    params: &'a LootParams<'a>,
    random: &'a GameRandom,
) -> std::result::Result<ItemStackSummary, LootRuntimeError> {
    let tables = tables();
    let function = parse_loot_function(&function, None).expect("the fixture function decodes");
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:entity", params, random).unwrap();
    evaluator.function(&function, stack, "pools[0].entries[0].functions[1]")
}

fn test<'a>(
    condition: Value,
    params: &'a LootParams<'a>,
    random: &'a GameRandom,
) -> std::result::Result<bool, LootRuntimeError> {
    let tables = tables();
    let condition = parse_loot_condition(&condition, None).expect("the fixture condition decodes");
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:entity", params, random).unwrap();
    evaluator.condition(&condition, "pools[0].conditions[0]")
}

/// The zombie table's own function (`entities/zombie.json`).
fn count_increase(limit: Option<i32>) -> Value {
    let mut function = json!({
        "function": "minecraft:enchanted_count_increase",
        "enchantment": "minecraft:looting",
        "count": {"type": "minecraft:uniform", "min": 0.0, "max": 1.0},
    });
    if let Some(limit) = limit {
        function["limit"] = json!(limit);
    }
    function
}

/// Every count `function` turns a one-item stack into over many draws.
fn counts(function: Value, start: i32, params: &LootParams<'_>) -> std::collections::BTreeSet<i32> {
    let random = GameRandom::with_seed(7);
    (0..200)
        .map(|_| {
            apply(
                function.clone(),
                stack("minecraft:arrow", start),
                params,
                &random,
            )
            .unwrap()
            .count
        })
        .collect()
}

#[test]
fn enchanted_count_increase_grows_by_the_rounded_level_times_count() {
    // `Math.round(2 * uniform(0, 1))` adds 0, 1 or 2.
    let view = attacker(2);
    let params = entity_params(LootEntity::Facts(&view), LootEntity::Absent);
    assert_eq!(counts(count_increase(None), 1, &params), [1, 2, 3].into());
}

#[test]
fn enchanted_count_increase_clamps_to_the_limit() {
    // Level 3 adds up to 3, and `limitSize` (`ItemStack.java:1068-1072`) caps
    // the grown stack at 4.
    let view = attacker(3);
    let params = entity_params(LootEntity::Facts(&view), LootEntity::Absent);
    assert_eq!(counts(count_increase(Some(4)), 3, &params), [3, 4].into());
}

#[test]
fn enchanted_count_increase_leaves_the_stack_alone_at_level_zero_and_without_an_attacker() {
    let view = attacker(0);
    let params = entity_params(LootEntity::Facts(&view), LootEntity::Absent);
    assert_eq!(counts(count_increase(Some(4)), 2, &params), [2].into());
    // `getOptionalParameter` is null and `instanceof LivingEntity` fails.
    let params = entity_params(LootEntity::Absent, LootEntity::Absent);
    assert_eq!(counts(count_increase(None), 2, &params), [2].into());
}

/// The zombie table's own condition (`entities/zombie.json`).
fn enchanted_bonus() -> Value {
    json!({
        "condition": "minecraft:random_chance_with_enchanted_bonus",
        "enchantment": "minecraft:looting",
        "enchanted_chance": {
            "type": "minecraft:linear",
            "base": 0.035,
            "per_level_above_first": 0.01,
        },
        "unenchanted_chance": 0.025,
    })
}

/// How many of 40 000 tests `condition` passes.
fn passes(condition: &Value, params: &LootParams<'_>) -> usize {
    let random = GameRandom::with_seed(11);
    (0..40_000)
        .filter(|_| test(condition.clone(), params, &random).unwrap())
        .count()
}

#[test]
fn random_chance_with_enchanted_bonus_uses_the_unenchanted_chance_at_level_zero() {
    // 0.025 of 40 000 is 1 000 (sd ~31).
    let view = attacker(0);
    let params = entity_params(LootEntity::Facts(&view), LootEntity::Absent);
    assert!((850..=1_150).contains(&passes(&enchanted_bonus(), &params)));
    // An absent attacker is the same level 0.
    let params = entity_params(LootEntity::Absent, LootEntity::Absent);
    assert!((850..=1_150).contains(&passes(&enchanted_bonus(), &params)));
}

#[test]
fn random_chance_with_enchanted_bonus_scales_the_enchanted_chance_by_level() {
    // `LevelBasedValue.Linear` (`LevelBasedValue.java:156-158`) is
    // `base + per_level_above_first * (level - 1)`: 0.055 at level 3, so 2 200
    // of 40 000 (sd ~46).
    let view = attacker(3);
    let params = entity_params(LootEntity::Facts(&view), LootEntity::Absent);
    assert!((2_000..=2_400).contains(&passes(&enchanted_bonus(), &params)));
}

#[test]
fn killed_by_player_is_the_presence_of_the_last_damage_player() {
    let view = attacker(0);
    let killed = json!({"condition": "minecraft:killed_by_player"});
    let params = entity_params(LootEntity::Absent, LootEntity::Facts(&view));
    let random = GameRandom::with_seed(0);
    assert!(test(killed.clone(), &params, &random).unwrap());
    // `hasParameter` (`LootContext.java:87-89`) asks whether the key holds a
    // value, not what it is, so an unshowable player still passes.
    let params = entity_params(LootEntity::Absent, LootEntity::Opaque);
    assert!(test(killed.clone(), &params, &random).unwrap());
    let params = entity_params(LootEntity::Absent, LootEntity::Absent);
    assert!(!test(killed, &params, &random).unwrap());
}
