//! Damage-source predicate results over actual read-only values.

use crate::loot_runtime::{LootDamageSourceFacts, LootRuntimeError};
use bbb_server_datapack::loot::{parse_loot_condition, LootRegistries};
use serde_json::{json, Value};

use super::super::{LootDamageSource, LootEntities, LootEntity, LootOrigin, LootParams};

fn bare() -> LootDamageSourceFacts<'static> {
    LootDamageSourceFacts {
        damage_type: "minecraft:arrow",
        tags: None,
        is_direct: Some(true),
        causing: LootEntity::Absent,
        direct: Some(LootEntity::Absent),
    }
}

fn evaluate(
    predicate: Value,
    source: LootDamageSource<'_>,
    origin: bool,
) -> std::result::Result<bool, LootRuntimeError> {
    evaluate_in(predicate, source, origin, PARAM_SET)
}

const PARAM_SET: &crate::loot_runtime::LootContextParamSet =
    crate::loot_runtime::param_sets::ENTITY;

fn evaluate_in(
    predicate: Value,
    source: LootDamageSource<'_>,
    origin: bool,
    param_set: &'static crate::loot_runtime::LootContextParamSet,
) -> std::result::Result<bool, LootRuntimeError> {
    let condition = json!({
        "condition": "minecraft:damage_source_properties",
        "predicate": predicate,
    });
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let condition = parse_loot_condition(&condition, None).expect("the fixture condition decodes");
    let params = LootParams {
        origin: origin.then_some(LootOrigin {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }),
        entities: {
            let mut entities = LootEntities::default();
            if param_set.allows(crate::loot_runtime::LootContextParam::ThisEntity) {
                entities.this_entity = LootEntity::Opaque;
            }
            if param_set.allows(crate::loot_runtime::LootContextParam::LastDamagePlayer) {
                entities.last_damage_player = LootEntity::Opaque;
            }
            entities
        },
        damage_source: source,
        ..LootParams::new(param_set)
    };
    let random = crate::random::GameRandom::with_seed(0);
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:damage_source", &params, &random)
            .expect("the fixture parameters bind");
    evaluator.condition(&condition, "pools[0].conditions[0]")
}

#[test]
fn an_absent_source_is_false() {
    let source = LootDamageSourceFacts {
        damage_type: "minecraft:arrow",
        tags: None,
        is_direct: Some(true),
        causing: LootEntity::Absent,
        direct: Some(LootEntity::Absent),
    };
    assert!(!evaluate_in(
        json!({}),
        LootDamageSource::Absent,
        true,
        crate::loot_runtime::param_sets::COMMAND
    )
    .unwrap());
    assert!(evaluate(json!({}), LootDamageSource::Present(source), true).unwrap());
}

#[test]
fn an_empty_predicate_is_true_for_a_present_source() {
    assert!(evaluate(json!({}), LootDamageSource::Present(bare()), true).unwrap());
}

#[test]
fn the_tag_term_compares_membership_against_its_expected_flag() {
    let tags = bbb_server_datapack::tag_loader::LoadedRegistryTags {
        registry_id: "minecraft:damage_type".into(),
        tags: std::collections::BTreeMap::from([(
            "minecraft:is_projectile".into(),
            vec!["minecraft:arrow".into()],
        )]),
    };
    let member = LootDamageSourceFacts {
        tags: Some(Some(&tags)),
        ..bare()
    };
    assert!(evaluate(
        json!({"tags": [{"id": "minecraft:is_projectile", "expected": true}]}),
        LootDamageSource::Present(member),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"tags": [{"id": "minecraft:is_projectile", "expected": false}]}),
        LootDamageSource::Present(member),
        true
    )
    .unwrap());
    let absent = LootDamageSourceFacts {
        damage_type: "minecraft:generic",
        tags: Some(Some(&tags)),
        ..bare()
    };
    assert!(evaluate(
        json!({"tags": [{"id": "minecraft:is_projectile", "expected": false}]}),
        LootDamageSource::Present(absent),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"tags": [
            {"id": "minecraft:is_projectile", "expected": true},
            {"id": "minecraft:is_lightning", "expected": true},
        ]}),
        LootDamageSource::Present(member),
        true
    )
    .unwrap());
    let empty = LootDamageSourceFacts {
        tags: Some(None),
        ..bare()
    };
    assert!(evaluate(
        json!({"tags": [{"id": "minecraft:is_projectile", "expected": false}]}),
        LootDamageSource::Present(empty),
        true
    )
    .unwrap());
}

#[test]
fn the_two_entity_terms_read_the_sources_own_entities() {
    let frog = crate::loot_runtime::LootEntityFacts {
        entity_type: Some("minecraft:frog"),
        ..Default::default()
    };
    let source = LootDamageSourceFacts {
        causing: LootEntity::Facts(&frog),
        direct: Some(LootEntity::Absent),
        ..bare()
    };
    assert!(evaluate(
        json!({"source_entity": {"type": "minecraft:frog"}}),
        LootDamageSource::Present(source),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"source_entity": {"type": "minecraft:player"}}),
        LootDamageSource::Present(source),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"direct_entity": {"type": "minecraft:frog"}}),
        LootDamageSource::Present(source),
        true
    )
    .unwrap());
    assert!(evaluate(
        json!({"source_entity": {}}),
        LootDamageSource::Present(source),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"direct_entity": {}}),
        LootDamageSource::Present(source),
        true
    )
    .unwrap());
    let unknown = LootDamageSourceFacts {
        direct: None,
        ..source
    };
    assert!(evaluate(
        json!({"direct_entity": {}}),
        LootDamageSource::Present(unknown),
        true
    )
    .is_err());
}

#[test]
fn the_is_direct_term_compares_the_sources_own_flag() {
    let direct = LootDamageSourceFacts {
        is_direct: Some(true),
        ..bare()
    };
    assert!(evaluate(
        json!({"is_direct": true}),
        LootDamageSource::Present(direct),
        true
    )
    .unwrap());
    assert!(!evaluate(
        json!({"is_direct": false}),
        LootDamageSource::Present(direct),
        true
    )
    .unwrap());
    let unknown = LootDamageSourceFacts {
        is_direct: None,
        ..direct
    };
    assert!(evaluate(
        json!({"is_direct": false}),
        LootDamageSource::Present(unknown),
        true
    )
    .is_err());
}
