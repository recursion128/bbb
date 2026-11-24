use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::*;
use crate::entity::experience_orb::spawn_experience_orb;
use crate::entity::falling_block::FallingBlockData;
use crate::entity::item::{plain_stack, spawn_item_entity};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{EntityIdCounter, EntityLifecycleEvent, RemovalReason};
use bbb_server_datapack::damage::{DamageEffect, DamageScaling, DeathMessageType};

#[test]
fn armor_stand_explosion_source_is_living_but_not_a_mob() {
    let position = Vec3d {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let entity = crate::entity::Entity::new(
        1,
        uuid::Uuid::from_u128(1),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        position,
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );

    let source = explosion_source_entity(&entity);
    assert_eq!(source.position, position);
    assert_eq!(source.kind, ExplosionSourceEntityKind::LivingOther);
    assert!(!source.is_mob());
    assert!(source.is_living_non_player());
}

fn items() -> &'static dyn bbb_registry_codec::PersistentItemStackHolderResolver {
    crate::world::registry_runtime::item_stack_resolver_for_test()
}

fn position() -> Vec3d {
    Vec3d {
        x: 1.5,
        y: 64.0,
        z: -2.5,
    }
}

fn damage_type(resource_id: &str) -> DamageTypeFacts {
    DamageTypeFacts {
        resource_id: resource_id.to_string(),
        registry_id: 0,
        message_id: resource_id.into(),
        scaling: DamageScaling::Never,
        exhaustion: 0.0,
        effects: DamageEffect::Hurt,
        death_message_type: DeathMessageType::Default,
        bypasses_invulnerability: false,
        bypasses_cooldown: false,
        is_drowning: false,
        is_fall: false,
        is_fire: false,
        is_freezing: false,
        is_explosion: false,
        ignites_armor_stands: false,
        burns_armor_stands: false,
        can_break_armor_stand: false,
        always_kills_armor_stands: false,
        always_most_significant_fall: false,
    }
}

fn source(damage_type: &DamageTypeFacts) -> StoredEntityDamageSource<'_> {
    StoredEntityDamageSource {
        damage_type,
        direct_entity_id: None,
        causing_entity: None,
        attacker_views: Default::default(),
    }
}

fn one_item(resource_id: &str) -> (EntityStore, i32) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let stack = plain_stack(resource_id, 1).expect("the test item is built in");
    let id = spawn_item_entity(&mut store, position(), stack).expect("the item spawns");
    store.drain_lifecycle_events();
    (store, id)
}

fn one_armor_stand() -> (EntityStore, i32) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = crate::entity::armor_stand::spawn(&mut store, position()).expect("the stand spawns");
    store.drain_lifecycle_events();
    (store, id)
}

fn item_health(store: &EntityStore, id: i32) -> i32 {
    let EntityData::Item(item) = store.get(id).expect("the item remains live").data() else {
        panic!("the id belongs to an item")
    };
    item.health()
}

#[test]
fn entity_damage_zero_negative_and_fractional_amounts_use_java_float_narrowing() {
    for (damage, expected) in [(0.0, 5), (-2.0, 7), (1.5, 3)] {
        let (mut store, id) = one_item("minecraft:stone");
        let mut events = LevelEventSink::default();
        let damage_type = damage_type("minecraft:generic");
        let outcome = store
            .hurt_server(
                id,
                source(&damage_type),
                damage,
                StoredEntityDamageRules::default(),
                items(),
                &mut events,
            )
            .expect("facts resolve")
            .expect("the entity exists");
        assert!(outcome.succeeded);
        assert!(outcome.applied);
        assert_eq!(item_health(&store, id), expected);
        assert!(store.get(id).expect("the item remains").hurt_marked());
        assert_eq!(events.game_event_count(), 1);
    }
}

#[test]
fn entity_damage_java_narrowing_handles_nan_and_infinities() {
    assert_eq!(java_health_after_damage(5, f32::NAN), 0);
    assert_eq!(java_health_after_damage(5, f32::INFINITY), i32::MIN);
    assert_eq!(java_health_after_damage(5, f32::NEG_INFINITY), i32::MAX);
}

#[test]
fn entity_damage_invulnerability_refuses_except_for_a_creative_causing_player() {
    let (mut store, id) = one_item("minecraft:stone");
    store
        .get_mut(id)
        .expect("the item exists")
        .set_invulnerable(true);
    let generic = damage_type("minecraft:generic");
    let mut events = LevelEventSink::default();
    let refused = store
        .hurt_server(
            id,
            source(&generic),
            1.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(!refused.succeeded);
    assert!(!refused.applied);
    assert_eq!(item_health(&store, id), 5);

    let creative = StoredEntityDamageSource {
        damage_type: &generic,
        direct_entity_id: Some(99),
        causing_entity: Some(StoredDamageCausingEntity::Player {
            uuid: uuid::Uuid::nil(),
            entity_id: 99,
            instabuild: true,
        }),
        attacker_views: Default::default(),
    };
    let accepted = store
        .hurt_server(
            id,
            creative,
            1.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(accepted.succeeded);
    assert_eq!(item_health(&store, id), 4);

    let mut bypass = damage_type("minecraft:generic_kill");
    bypass.bypasses_invulnerability = true;
    let accepted = store
        .hurt_server(
            id,
            source(&bypass),
            1.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(accepted.succeeded);
    assert_eq!(item_health(&store, id), 3);
}

#[test]
fn armor_stand_prediction_and_live_apply_share_the_explicit_context() {
    let (mut store, id) = one_armor_stand();
    let mut arrow = damage_type("minecraft:arrow");
    arrow.always_kills_armor_stands = true;
    let source = StoredEntityDamageSource {
        damage_type: &arrow,
        direct_entity_id: Some(99),
        causing_entity: Some(StoredDamageCausingEntity::Player {
            uuid: uuid::Uuid::nil(),
            entity_id: 99,
            instabuild: false,
        }),
        attacker_views: Default::default(),
    };
    let context = StoredEntityDamageContext {
        block_drops: true,
        attacker_may_build: false,
        attacker_spectator: Some(false),
    };
    let predicted = evaluate_stored_entity_damage_in_context(
        store.get(id).expect("the stand exists"),
        source,
        1.0,
        StoredEntityDamageRules::default(),
        context,
        items(),
    )
    .expect("armor stands need no item facts");
    let mut events = LevelEventSink::default();
    let applied = store
        .hurt_server_in_context(
            id,
            source,
            1.0,
            StoredEntityDamageRules::default(),
            context,
            items(),
            &mut events,
        )
        .expect("armor stands need no item facts")
        .expect("the stand exists");
    assert_eq!(predicted, applied);
    assert_eq!(
        applied,
        StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        }
    );
    assert!(store.get(id).is_some());

    let allowed = evaluate_stored_entity_damage(
        store.get(id).expect("the stand remains"),
        source,
        1.0,
        StoredEntityDamageRules::default(),
        items(),
    )
    .expect("the default convenience path remains available");
    assert!(allowed.applied && allowed.succeeded);
}

#[test]
fn entity_damage_item_fixed_in_fire_resistance_is_part_of_fire_immunity() {
    let (mut store, id) = one_item("minecraft:netherite_ingot");
    let mut fire = damage_type("minecraft:on_fire");
    fire.is_fire = true;
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            source(&fire),
            2.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(!outcome.succeeded);
    assert!(!outcome.applied);
    assert_eq!(item_health(&store, id), 5);
    assert!(!store.get(id).expect("the item remains").hurt_marked());
    assert_eq!(events.game_event_count(), 0);
}

#[test]
fn entity_damage_mob_griefing_and_current_source_resistance_refuse_before_marking() {
    let (mut store, id) = one_item("minecraft:stone");
    let damage_type = damage_type("minecraft:mob_attack");
    let mob = StoredEntityDamageSource {
        damage_type: &damage_type,
        direct_entity_id: Some(7),
        causing_entity: Some(StoredDamageCausingEntity::Mob { entity_id: 7 }),
        attacker_views: Default::default(),
    };
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            mob,
            1.0,
            StoredEntityDamageRules {
                mob_griefing: false,
                ..StoredEntityDamageRules::default()
            },
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(!outcome.succeeded);
    assert!(!outcome.applied);

    let (mut store, id) = one_item("minecraft:netherite_ingot");
    let lava = self::damage_type("minecraft:lava");
    let outcome = store
        .hurt_server(
            id,
            source(&lava),
            1.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("facts resolve")
        .expect("the entity exists");
    assert!(!outcome.succeeded);
    assert!(!outcome.applied);
    assert_eq!(item_health(&store, id), 5);
    assert!(!store.get(id).expect("the item remains").hurt_marked());
    assert_eq!(events.game_event_count(), 0);
}

#[test]
fn entity_damage_destroyed_contents_are_added_before_the_original_is_removed() {
    let contents = [
        plain_stack("minecraft:stone", 2).expect("stone stack"),
        plain_stack("minecraft:dirt", 3).expect("dirt stack"),
    ];
    let filled = plain_stack("minecraft:shulker_box", 1)
        .unwrap()
        .with_container_contents(&contents, items())
        .unwrap();
    let destroyed_source_stack = filled.with_container_contents(&[], items()).unwrap();
    {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let id = spawn_item_entity(&mut store, position(), filled).expect("the item spawns");
        store.drain_lifecycle_events();
        let damage_type = damage_type("minecraft:generic");
        let mut events = LevelEventSink::default();
        let outcome = store
            .hurt_server(
                id,
                StoredEntityDamageSource {
                    damage_type: &damage_type,
                    direct_entity_id: Some(17),
                    causing_entity: Some(StoredDamageCausingEntity::Other { entity_id: 18 }),
                    attacker_views: Default::default(),
                },
                5.0,
                StoredEntityDamageRules::default(),
                items(),
                &mut events,
            )
            .expect("facts resolve")
            .expect("the entity exists");
        assert!(outcome.succeeded);
        assert!(store.get(id).is_none());
        let lifecycle = store.drain_lifecycle_events();
        assert!(matches!(lifecycle.as_slice(), [
            EntityLifecycleEvent::Added { .. },
            EntityLifecycleEvent::Added { .. },
            EntityLifecycleEvent::Removed { entity, .. },
        ] if entity.id() == id
            && matches!(entity.data(), EntityData::Item(item) if item.item() == &destroyed_source_stack)));
        let recorded: Vec<_> = events.recent_game_events().collect();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].game_event, VanillaGameEvent::ENTITY_DAMAGE);
        assert_eq!(recorded[0].source_entity_id, Some(18));
        assert_eq!(recorded[0].pos, BlockPos { x: 1, y: 64, z: -3 });
        let positioned: Vec<_> = events.recent_positioned_game_events().collect();
        assert_eq!(positioned[0].position, position());
    }
}

#[test]
fn entity_damage_experience_orb_marks_health_and_discards_without_a_game_event() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_experience_orb(&mut store, position(), 3).expect("the orb spawns");
    store.drain_lifecycle_events();
    let damage_type = damage_type("minecraft:generic");
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            source(&damage_type),
            5.0,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("no item facts are read")
        .expect("the orb exists");
    assert!(outcome.succeeded);
    assert!(store.get(id).is_none());
    assert_eq!(events.game_event_count(), 0);
    assert!(matches!(
        store.drain_lifecycle_events().as_slice(),
        [EntityLifecycleEvent::Removed { entity, .. }]
            if entity.id() == id && entity.hurt_marked()
    ));
}

#[test]
fn entity_damage_falling_block_marks_but_returns_false_and_remains_live() {
    let sand = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:sand")
            .expect("sand is built in")
            .id,
    )
    .expect("sand has a default state");
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position(),
        EntityData::FallingBlock(FallingBlockData::new(sand, BlockPos { x: 1, y: 64, z: -3 })),
    );
    store.drain_lifecycle_events();
    let damage_type = damage_type("minecraft:generic");
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            source(&damage_type),
            f32::INFINITY,
            StoredEntityDamageRules::default(),
            items(),
            &mut events,
        )
        .expect("no item facts are read")
        .expect("the falling block exists");
    assert!(!outcome.succeeded);
    assert!(outcome.applied);
    assert!(store.get(id).expect("the block remains live").hurt_marked());
    assert_eq!(events.game_event_count(), 0);
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn entity_damage_runtime_fall_tag_projection_refuses_before_marking() {
    let (mut store, id) = one_item("minecraft:stone");
    let mut fall = damage_type("minecraft:fall");
    fall.is_fall = true;
    let mut events = LevelEventSink::default();
    let outcome = store
        .hurt_server(
            id,
            source(&fall),
            1.0,
            StoredEntityDamageRules {
                fall_damage_immune: true,
                ..StoredEntityDamageRules::default()
            },
            items(),
            &mut events,
        )
        .expect("the tag refusal reads no item facts")
        .expect("the item exists");
    assert_eq!(
        outcome,
        StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        }
    );
    assert_eq!(item_health(&store, id), 5);
    assert!(!store.get(id).expect("the item remains").hurt_marked());
    assert_eq!(events.game_event_count(), 0);
}

#[test]
fn entity_damage_fire_immunity_query_preserves_creaking_virtual_fallback() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
            position(),
        )
        .unwrap();
    assert!(!fire_immune(store.get(id).unwrap(), items()).unwrap());
    assert!(store
        .get_mut(id)
        .unwrap()
        .set_creaking_transient(BlockPos { x: 1, y: 64, z: 0 }));
    assert_eq!(
        fire_immune_with_item_query(store.get(id).unwrap(), |_, _| -> Result<bool, ()> {
            panic!("Creaking fire immunity does not resolve Item components")
        }),
        Ok(true)
    );
    assert!(fire_immune(store.get(id).unwrap(), items()).unwrap());
}

fn environmental_stack(component: &str, value: bbb_protocol::NbtSummaryValue) -> RuntimeItemStack {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &Value::Compound(vec![
            Entry {
                name: "id".into(),
                value: Value::String("minecraft:stone".into()),
            },
            Entry {
                name: "count".into(),
                value: Value::Int(1),
            },
            Entry {
                name: "components".into(),
                value: Value::Compound(vec![Entry {
                    name: component.into(),
                    value,
                }]),
            },
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .unwrap()
}

#[test]
fn entity_damage_environment_item_and_orb_health_and_removal() {
    let (mut store, id) = one_item("minecraft:stone");
    let mut body = store.detach(id).unwrap();
    let mut native = NativeLevel::new();
    assert_eq!(
        native.with_level(|level| hurt_detached_from_environment(
            &mut body,
            &mut store,
            level,
            "minecraft:cactus",
            1.25,
        )),
        Some(true)
    );
    let EntityData::Item(item) = body.data() else {
        unreachable!()
    };
    assert_eq!(item.health(), 3);
    assert!(body.hurt_marked());
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.position == body.position()
                && event.recorded.source_entity_id.is_none()
        }));
    store.attach(body);

    let orb_id = spawn_experience_orb(&mut store, position(), 1).unwrap();
    let mut orb = store.detach(orb_id).unwrap();
    let before = native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .len();
    assert_eq!(
        native.with_level(|level| hurt_detached_from_environment(
            &mut orb,
            &mut store,
            level,
            "minecraft:lava",
            5.0,
        )),
        Some(true)
    );
    let EntityData::ExperienceOrb(data) = orb.data() else {
        unreachable!()
    };
    assert_eq!(data.health(), 0);
    assert!(orb.hurt_marked());
    assert!(orb.removal_reason().is_some());
    assert!(
        !native.world.positioned_game_events_for_test(DIMENSION)[before..]
            .iter()
            .any(|event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE)
    );
    store.attach(orb);
    assert!(store.get(orb_id).is_none());
}

#[test]
fn entity_damage_environment_live_item_resistance_uses_fixed_fire_then_actual_source() {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    let stack = environmental_stack(
        "minecraft:damage_resistant",
        Value::Compound(vec![Entry {
            name: "types".into(),
            value: Value::String("#test:resistant".into()),
        }]),
    );
    for (members, succeeds) in [
        (vec!["minecraft:in_fire".to_owned()], false),
        (vec!["minecraft:lava".to_owned()], false),
        (vec![], true),
    ] {
        let mut datapacks = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
        datapacks
            .tags
            .registries
            .iter_mut()
            .find(|tags| tags.registry_id == "minecraft:damage_type")
            .unwrap()
            .tags
            .insert("test:resistant".into(), members);
        let mut native = NativeLevel::with_datapacks(std::sync::Arc::new(datapacks));
        let mut store = EntityStore::new(31, EntityIdCounter::new());
        let id = spawn_item_entity(&mut store, position(), stack.clone()).unwrap();
        let mut body = store.detach(id).unwrap();
        assert_eq!(
            native.with_level(|level| hurt_detached_from_environment(
                &mut body,
                &mut store,
                level,
                "minecraft:lava",
                1.0,
            )),
            Some(succeeds)
        );
        let EntityData::Item(item) = body.data() else {
            unreachable!()
        };
        assert_eq!(item.health(), if succeeds { 4 } else { 5 });
        assert_eq!(body.hurt_marked(), succeeds);
        assert_eq!(
            native
                .world
                .positioned_game_events_for_test(DIMENSION)
                .iter()
                .any(|event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE),
            succeeds
        );
    }
}

#[test]
fn entity_damage_environment_lethal_container_releases_contents_and_removes_parent() {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    let stack = environmental_stack(
        "minecraft:container",
        Value::List(vec![Value::Compound(vec![
            Entry {
                name: "slot".into(),
                value: Value::Int(0),
            },
            Entry {
                name: "item".into(),
                value: Value::Compound(vec![
                    Entry {
                        name: "id".into(),
                        value: Value::String("minecraft:dirt".into()),
                    },
                    Entry {
                        name: "count".into(),
                        value: Value::Int(1),
                    },
                ]),
            },
        ])]),
    );
    let mut store = EntityStore::new(32, EntityIdCounter::new());
    let id = spawn_item_entity(&mut store, position(), stack).unwrap();
    let mut body = store.detach(id).unwrap();
    let mut native = NativeLevel::new();
    assert_eq!(
        native.with_level(|level| hurt_detached_from_environment(
            &mut body,
            &mut store,
            level,
            "minecraft:cactus",
            5.0,
        )),
        Some(true)
    );
    assert!(body.removal_reason().is_some());
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE));
    store.attach(body);
    assert!(store.get(id).is_none());
    let items: Vec<_> = store
        .ids()
        .into_iter()
        .filter_map(|id| store.get(id))
        .filter_map(|body| {
            let EntityData::Item(item) = body.data() else {
                return None;
            };
            Some((body.position(), item.item().clone()))
        })
        .collect();
    assert_eq!(
        items,
        [(position(), plain_stack("minecraft:dirt", 1).unwrap())]
    );
}

mod simple_environment;

mod shulker_environment;
