use std::collections::BTreeMap;

use super::*;
use crate::loot_runtime::{LootRaiderFlags, LootRuntimeError, LootTool};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_server_datapack::loot::{parse_loot_condition, LootRegistries};
use serde_json::{json, Value};

use super::super::{
    LootDamageSource, LootEntities, LootEntity, LootEntityAnswer, LootEntityFacts, LootOrigin,
    LootParams,
};

fn all_flags(value: bool) -> LootEntityFlags {
    LootEntityFlags {
        on_ground: Some(value),
        on_fire: Some(value),
        crouching: Some(value),
        sprinting: Some(value),
        swimming: Some(value),
        in_water: Some(value),
        fall_flying: Some(value),
        ability_flying: Some(value),
        baby: Some(value),
    }
}

fn tool(name: &str) -> LootTool {
    LootTool {
        stack: Default::default(),
        item_id: Some(
            vanilla_item_type_for_resource_id(name)
                .expect("builtin item")
                .id,
        ),
        count: 1,
        enchantments: BTreeMap::new(),
    }
}

/// `LootItemEntityPropertyCondition.test` (`LootItemEntityPropertyCondition.java:33-37`)
/// for one condition document and one entity, which is bound to the very slot the
/// condition names.
fn evaluate(
    condition: Value,
    view: &LootEntityFacts,
) -> std::result::Result<bool, LootRuntimeError> {
    let target = condition["entity"]
        .as_str()
        .or_else(|| condition["term"]["entity"].as_str())
        .expect("the fixture names a slot");
    // `THIS_ENTITY` is required by the ENTITY set, so it is always present;
    // only the slot the condition names carries the entity under test.
    let mut entities = LootEntities {
        this_entity: LootEntity::Opaque,
        ..LootEntities::default()
    };
    match target {
        "this" => entities.this_entity = LootEntity::Facts(view),
        "attacker" => entities.attacking_entity = LootEntity::Facts(view),
        "direct_attacker" => entities.direct_attacking_entity = LootEntity::Facts(view),
        other => panic!("the fixture names one of the three slots, not {other}"),
    }
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let condition = parse_loot_condition(&condition, None).expect("the fixture condition decodes");
    let item_environment =
        crate::loot_runtime::RegistryItemEnvironment::new(crate::inventory::test_item_resolver());
    let params = LootParams {
        item_environment: Some(&item_environment),
        origin: Some(LootOrigin {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }),
        entities,
        damage_source: LootDamageSource::Present(crate::loot_runtime::LootDamageSourceFacts {
            damage_type: "minecraft:generic",
            is_direct: Some(true),
            tags: None,
            causing: LootEntity::Absent,
            direct: Some(LootEntity::Absent),
        }),
        ..LootParams::new(crate::loot_runtime::param_sets::ENTITY)
    };
    let random = crate::random::GameRandom::with_seed(0);
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:entity", &params, &random)
            .expect("the ENTITY parameter set binds");
    evaluator.condition(&condition, "pools[0].conditions[0]")
}

fn entity_properties(entity: &str, predicate: Value) -> Value {
    json!({
        "condition": "minecraft:entity_properties",
        "entity": entity,
        "predicate": predicate,
    })
}

#[test]
fn missing_components_cannot_satisfy_inverted_predicates() {
    for (species, components) in [
        (
            "minecraft:shulker",
            json!({"minecraft:shulker/color":"red"}),
        ),
        (
            "minecraft:mannequin",
            json!({"minecraft:profile":{"name":"capture_probe"}}),
        ),
        (
            "minecraft:area_effect_cloud",
            json!({"minecraft:potion_duration_scale":0.5}),
        ),
    ] {
        let condition = json!({"condition":"minecraft:inverted","term":
            entity_properties("this", json!({"components":components}))});
        let missing = LootEntityFacts {
            entity_type: Some(species),
            ..LootEntityFacts::default()
        };
        assert!(evaluate(condition.clone(), &missing).is_err());
        assert!(evaluate(condition.clone(), &LootEntityFacts::default()).is_err());
        let pig = LootEntityFacts {
            entity_type: Some("minecraft:pig"),
            ..LootEntityFacts::default()
        };
        assert!(evaluate(condition, &pig).unwrap());
    }
}

#[test]
fn flags_terms_read_the_entity_in_vanillas_order() {
    let view = LootEntityFacts {
        living: Some(true),
        flags: all_flags(true),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_on_fire": true}})),
        &view
    )
    .unwrap());
    assert!(!evaluate(
        entity_properties("this", json!({"flags": {"is_on_fire": false}})),
        &view
    )
    .unwrap());
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_baby": true}})),
        &view
    )
    .unwrap());
    assert!(evaluate(
        entity_properties(
            "this",
            json!({"flags": {"is_on_ground": true, "is_in_water": true}})
        ),
        &view
    )
    .unwrap());
    // `is_flying` reads `isFallFlying() || abilities.flying` (`:57-63`).
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_flying": true}})),
        &view
    )
    .unwrap());
    let grounded = LootEntityFacts {
        living: Some(true),
        flags: LootEntityFlags {
            fall_flying: Some(false),
            ability_flying: Some(false),
            ..all_flags(true)
        },
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(
        entity_properties("this", json!({"flags": {"is_flying": true}})),
        &grounded
    )
    .unwrap());
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_flying": false}})),
        &grounded
    )
    .unwrap());
}

#[test]
fn living_gated_flag_terms_answer_a_non_living_entity_without_reading_it() {
    // An entity that is not a `LivingEntity` passes `is_baby` and
    // `is_fall_flying` (`EntityFlagsPredicate.java:68-70`) and is simply not
    // flying (`:57-63`), whatever the flag values underneath say.
    let view = LootEntityFacts {
        living: Some(false),
        flags: all_flags(true),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_baby": true}})),
        &view
    )
    .unwrap());
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_baby": false}})),
        &view
    )
    .unwrap());
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_fall_flying": true}})),
        &view
    )
    .unwrap());
    assert!(!evaluate(
        entity_properties("this", json!({"flags": {"is_flying": true}})),
        &view
    )
    .unwrap());
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_flying": false}})),
        &view
    )
    .unwrap());
    // The ungated terms are still read off the entity.
    assert!(evaluate(
        entity_properties("this", json!({"flags": {"is_sprinting": true}})),
        &view
    )
    .unwrap());
}

#[test]
fn equipment_terms_test_the_main_hand_through_the_item_predicate() {
    let mut sword = tool("minecraft:diamond_sword");
    sword.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(
        bbb_protocol::packets::ItemStackSummary {
            item_id: sword.item_id,
            count: 1,
            component_patch: bbb_registry_codec::read_item_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                &bbb_protocol::json_value_to_nbt_summary(&json!({"minecraft:fire_aspect":2}))
                    .unwrap(),
                crate::inventory::test_item_resolver(),
            )
            .unwrap()
            .into(),
        },
    );
    let view = LootEntityFacts {
        living: Some(true),
        equipment: Some(BTreeMap::from([(EquipmentSlot::MainHand, sword)])),
        ..LootEntityFacts::default()
    };
    let smelts = json!({
        "equipment": {
            "mainhand": {"predicates": {"minecraft:enchantments": [{"enchantments": "#minecraft:smelts_loot"}]}}
        }
    });
    assert!(evaluate(entity_properties("direct_attacker", smelts.clone()), &view).unwrap());
    let empty_hand = LootEntityFacts {
        living: Some(true),
        equipment: Some(BTreeMap::new()),
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(
        entity_properties("direct_attacker", smelts.clone()),
        &empty_hand
    )
    .unwrap());
    // The predicate's own item term is read off the same stack.
    let named = json!({"equipment": {"mainhand": {"items": "minecraft:diamond_sword"}}});
    assert!(evaluate(entity_properties("direct_attacker", named.clone()), &view).unwrap());
    assert!(!evaluate(entity_properties("direct_attacker", named), &empty_hand).unwrap());
    // A non-living entity fails `EntityEquipmentPredicate.matches` outright.
    let item = LootEntityFacts {
        living: Some(false),
        equipment: Some(BTreeMap::from([(
            EquipmentSlot::MainHand,
            tool("minecraft:diamond_sword"),
        )])),
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(entity_properties("direct_attacker", smelts), &item).unwrap());
}

/// `EntityPredicate.matches` returns at the first false term, so a later term
/// this view cannot read is never asked.
#[test]
fn a_false_term_answers_before_an_unreadable_one() {
    let player = LootEntityFacts {
        entity_type: Some("minecraft:player"),
        ..LootEntityFacts::default()
    };
    let predicate = json!({"type": "minecraft:zombie", "flags": {"is_on_fire": true}});
    assert!(!evaluate(entity_properties("attacker", predicate), &player).unwrap());
    let predicate = json!({"type": "minecraft:player", "flags": {"is_on_fire": true}});
    assert!(evaluate(entity_properties("attacker", predicate), &player).is_err());
}

#[test]
fn type_terms_read_the_entity_type_and_its_tag() {
    let player = LootEntityFacts {
        entity_type: Some("minecraft:player"),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties("attacker", json!({"type": "minecraft:player"})),
        &player
    )
    .unwrap());
    assert!(!evaluate(
        entity_properties("attacker", json!({"type": "minecraft:zombie"})),
        &player
    )
    .unwrap());
    // `#minecraft:skeletons` is the creeper table's own term.
    assert!(!evaluate(
        entity_properties("attacker", json!({"type": "#minecraft:skeletons"})),
        &player
    )
    .unwrap());
    let skeleton = LootEntityFacts {
        entity_type: Some("minecraft:skeleton"),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties("attacker", json!({"type": "#minecraft:skeletons"})),
        &skeleton
    )
    .unwrap());
    assert!(evaluate(
        entity_properties(
            "attacker",
            json!({"type": ["minecraft:creeper", "minecraft:skeleton"]})
        ),
        &skeleton
    )
    .unwrap());
}

#[test]
fn vehicle_terms_recurse_into_the_vehicle_entity() {
    let on_a_chicken = LootEntityFacts {
        entity_type: Some("minecraft:zombie"),
        vehicle: Some(Some(Box::new(LootEntityFacts {
            entity_type: Some("minecraft:chicken"),
            ..LootEntityFacts::default()
        }))),
        ..LootEntityFacts::default()
    };
    let chicken = json!({"vehicle": {"type": "minecraft:chicken"}});
    assert!(evaluate(entity_properties("this", chicken.clone()), &on_a_chicken).unwrap());
    assert!(!evaluate(
        entity_properties(
            "this",
            json!({"vehicle": {"type": "minecraft:zombie_horse"}})
        ),
        &on_a_chicken
    )
    .unwrap());
    // `Entity.getVehicle() == null` is a null entity, which `matches` rejects.
    let alone = LootEntityFacts {
        entity_type: Some("minecraft:zombie"),
        vehicle: Some(None),
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(entity_properties("this", chicken), &alone).unwrap());
}

#[test]
fn type_specific_terms_read_the_slime_size_and_the_raider_flags() {
    let slime = LootEntityFacts {
        slime_size: Some(LootEntityAnswer::Value(2)),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:slime", "size": {"min": 2}}})
        ),
        &slime
    )
    .unwrap());
    assert!(!evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:slime", "size": 1}})
        ),
        &slime
    )
    .unwrap());
    // `instanceof Slime` fails for every other entity, so the term is false.
    let not_a_slime = LootEntityFacts {
        slime_size: Some(LootEntityAnswer::NotApplicable),
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:slime", "size": {"min": 1}}})
        ),
        &not_a_slime
    )
    .unwrap());
    let captain = LootEntityFacts {
        raider_flags: Some(LootEntityAnswer::Value(LootRaiderFlags {
            has_raid: Some(false),
            is_captain: Some(true),
        })),
        ..LootEntityFacts::default()
    };
    assert!(evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:raider", "is_captain": true}})
        ),
        &captain
    )
    .unwrap());
    // `RaiderPredicate`'s two fields both default to false (`:15-16`).
    assert!(!evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:raider", "has_raid": true}})
        ),
        &captain
    )
    .unwrap());
    // An answered flag that disagrees decides the conjunction false whatever
    // the other flag is, so the term needs no raid read.
    let captain_without_raid = LootEntityFacts {
        raider_flags: Some(LootEntityAnswer::Value(LootRaiderFlags {
            has_raid: None,
            is_captain: Some(false),
        })),
        ..LootEntityFacts::default()
    };
    assert!(!evaluate(
        entity_properties(
            "this",
            json!({"type_specific": {"type": "minecraft:raider", "is_captain": true}})
        ),
        &captain_without_raid
    )
    .unwrap());
}

#[test]
fn an_absent_entity_is_false_and_an_empty_entity_properties_is_true() {
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let random = crate::random::GameRandom::with_seed(0);
    // The null entity of an absent slot is false, and an
    // `entity_properties` with no predicate at all is true without the entity
    // being consulted (`LootItemEntityPropertyCondition.java:36`).
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Opaque,
            ..LootEntities::default()
        },
        damage_source: LootDamageSource::Present(crate::loot_runtime::LootDamageSourceFacts {
            damage_type: "minecraft:generic",
            is_direct: Some(true),
            tags: None,
            causing: LootEntity::Absent,
            direct: Some(LootEntity::Absent),
        }),
        ..LootParams::new(crate::loot_runtime::param_sets::ENTITY)
    };
    for (condition, expected) in [
        (
            json!({"condition": "minecraft:entity_properties", "entity": "attacker"}),
            true,
        ),
        (entity_properties("attacker", json!({})), false),
    ] {
        let condition = parse_loot_condition(&condition, None).unwrap();
        let mut evaluator =
            crate::loot_runtime::LootEvaluator::new(&tables, "test:entity", &params, &random)
                .unwrap();
        assert_eq!(
            evaluator.condition(&condition, "pools[0]").unwrap(),
            expected
        );
    }
}

#[test]
fn effects_predicate_reads_active_instances() {
    use crate::entity::living::mob_effects::{ActiveMobEffect, ActiveMobEffects};

    let facts = LootEntityFacts {
        living: Some(true),
        effects: Some(ActiveMobEffects::from_decoded_saved_effects(vec![
            ActiveMobEffect::with_visibility("minecraft:speed", -1, 2, true, false, true).unwrap(),
        ])),
        ..LootEntityFacts::default()
    };
    let matches = |effects, facts: &LootEntityFacts| {
        evaluate(
            entity_properties("this", json!({"effects": effects})),
            facts,
        )
    };
    assert!(matches(
        json!({"speed":{"amplifier":2,"duration":-1,"ambient":true,"visible":false}}),
        &facts
    )
    .unwrap());
    assert!(matches(json!({}), &facts).unwrap());
    let nonliving = LootEntityFacts {
        living: Some(false),
        ..LootEntityFacts::default()
    };
    assert!(!matches(json!({}), &nonliving).unwrap());
    let unknown = LootEntityFacts {
        living: Some(true),
        ..LootEntityFacts::default()
    };
    assert!(matches(json!({"speed":{}}), &unknown).is_err());
}

#[test]
fn movement_affected_by_resolves_from_supporting_block_fallback() {
    use crate::loot_runtime::{LootBlockStateAt, LootEntityPosition};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_COBBLESTONE_WALL_ID, VANILLA_BLOCK_SOUL_SAND_ID,
    };

    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let random = crate::random::GameRandom::with_seed(1);
    let soul_sand = vanilla_block_default_state_id(VANILLA_BLOCK_SOUL_SAND_ID).unwrap();
    let wall = vanilla_block_default_state_id(VANILLA_BLOCK_COBBLESTONE_WALL_ID).unwrap();
    let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();

    // Standing on soul sand at y = 64.875: supporting block (0, 64, 0) is not a
    // wall/fence gate, so `getOnPos(0.500001F)` yields y = floor(64.875 - 0.500001) = 64.
    let facts = LootEntityFacts {
        position: Some(LootEntityPosition {
            x: 0.5,
            y: 64.875,
            z: 0.5,
        }),
        supporting_block: Some([0, 64, 0]),
        ..LootEntityFacts::default()
    };
    let blocks = |pos: bbb_protocol::packets::BlockPos| {
        Some(if pos.x == 0 && pos.y == 64 && pos.z == 0 {
            soul_sand
        } else {
            air
        })
    };
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.5,
            y: 64.875,
            z: 0.5,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Facts(&facts),
            ..LootEntities::default()
        },
        level: Some(&blocks as &LootBlockStateAt<'_>),
        ..LootParams::new(crate::loot_runtime::param_sets::ADVANCEMENT_ENTITY)
    };
    let cond = parse_loot_condition(
        &entity_properties(
            "this",
            json!({"movement_affected_by": {"block": {"blocks": "minecraft:soul_sand"}}}),
        ),
        None,
    )
    .unwrap();
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:movement", &params, &random)
            .unwrap();
    assert!(evaluator.condition(&cond, "pools[0]").unwrap());

    // Standing on a cobblestone wall (height 1.5, feet at y = 65.5): supporting
    // block (0, 64, 0) IS in `#minecraft:walls`, so `body_on_pos` keeps (0, 64, 0)
    // instead of floor(65.5 - 0.500001) = 64... wait test y = 65.6 so floor = 65!
    let wall_facts = LootEntityFacts {
        position: Some(LootEntityPosition {
            x: 0.5,
            y: 65.6,
            z: 0.5,
        }),
        supporting_block: Some([0, 64, 0]),
        ..LootEntityFacts::default()
    };
    let wall_blocks = |pos: bbb_protocol::packets::BlockPos| {
        Some(if pos.x == 0 && pos.y == 64 && pos.z == 0 {
            wall
        } else {
            air
        })
    };
    let wall_params = LootParams {
        origin: Some(LootOrigin {
            x: 0.5,
            y: 65.6,
            z: 0.5,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Facts(&wall_facts),
            ..LootEntities::default()
        },
        level: Some(&wall_blocks as &LootBlockStateAt<'_>),
        ..LootParams::new(crate::loot_runtime::param_sets::ADVANCEMENT_ENTITY)
    };
    let wall_cond = parse_loot_condition(
        &entity_properties(
            "this",
            json!({"movement_affected_by": {"block": {"blocks": "minecraft:cobblestone_wall"}}}),
        ),
        None,
    )
    .unwrap();
    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:wall", &wall_params, &random)
            .unwrap();
    assert!(evaluator.condition(&wall_cond, "pools[0]").unwrap());
}

#[test]
fn snapshot_live_vehicle_resolves_nbt_and_slots_under_registry_item_environment() {
    use bbb_protocol::equipment_slots::EquipmentSlot;
    use bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID;
    use bbb_protocol::packets::{ItemStackSummary, Vec3d};
    use bbb_server_types::item_stack::RuntimeItemStack;

    let mut store = crate::entity::EntityStore::new(7, crate::entity::EntityIdCounter::new());
    let horse_id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 2.0,
            },
        )
        .unwrap();
    let saddle = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_SADDLE_ID),
        count: 1,
        component_patch: Default::default(),
    });
    store
        .get_mut(horse_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .equipment_mut()
        .set(EquipmentSlot::Saddle, Some(saddle));
    store
        .get_mut(horse_id)
        .unwrap()
        .set_remaining_fire_ticks(40);

    let resolver = bbb_registry_codec::UnavailableHolderResolver;
    let bodies = crate::entity::store::EntityBodyReadView::new(&store, None, None);
    let vehicle = crate::loot_runtime::StoredLootEntity {
        entity: store.get(horse_id).unwrap(),
        bodies: Some(bodies),
        raid_at_position: None,
    }
    .snapshot_live(&resolver);
    assert!(vehicle.nbt.is_none());
    assert!(vehicle.slots.is_none());

    let rider = LootEntityFacts {
        entity_type: Some("minecraft:player"),
        living: Some(true),
        vehicle: Some(Some(Box::new(vehicle))),
        ..LootEntityFacts::default()
    };
    let value_level =
        crate::world::ValueEffectLevel::weather_only(crate::loot_runtime::LootWeather {
            raining: false,
            thundering: false,
        })
        .with_entities(&store);
    let env =
        crate::loot_runtime::RegistryItemEnvironment::for_enchantment(&resolver, &value_level);
    let tables = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let random = crate::random::GameRandom::with_seed(0);
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 1.0,
            y: 64.0,
            z: 2.0,
        }),
        item_environment: Some(&env),
        enchantment_level: Some(1),
        entities: LootEntities {
            this_entity: LootEntity::Facts(&rider),
            ..LootEntities::default()
        },
        ..LootParams::new(crate::loot_runtime::param_sets::ENCHANTED_ENTITY)
    };

    let matching = parse_loot_condition(
        &entity_properties(
            "this",
            json!({
                "vehicle": {
                    "nbt": "{Fire:40s}",
                    "slots": {
                        "saddle": {"items": "minecraft:saddle"}
                    }
                }
            }),
        ),
        None,
    )
    .unwrap();
    let mismatching = parse_loot_condition(
        &entity_properties(
            "this",
            json!({
                "vehicle": {
                    "nbt": "{Fire:10s}",
                    "slots": {
                        "saddle": {"items": "minecraft:saddle"}
                    }
                }
            }),
        ),
        None,
    )
    .unwrap();

    let mut evaluator =
        crate::loot_runtime::LootEvaluator::new(&tables, "test:vehicle_live", &params, &random)
            .unwrap();
    assert!(evaluator.condition(&matching, "pools[0]").unwrap());
    assert!(!evaluator.condition(&mismatching, "pools[0]").unwrap());
}
