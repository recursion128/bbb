use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type_for_resource_id, VANILLA_MOB_EFFECT_TYPES,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::damage::builtin_damage_type_registry;
use bbb_server_types::food::FoodData;

use super::*;
use crate::entity::damage::{StoredEntityDamageRules, StoredEntityDamageSource};
use crate::entity::general_living::hurt_server_with_runtime;
use crate::entity::projectile_action::mob_effect::{
    PendingMobEffectAction, PendingRemovalMobEffect,
};
use crate::entity::{AttributeSupplier, EntityData, EntityIdCounter, EntityStore, RemovalReason};

fn zombie() -> LivingEntityState {
    LivingEntityState::new("minecraft:zombie").expect("zombie default attributes")
}

fn entity_tag_snapshot(tags: &[(&str, &[&str])]) -> RegistryTagSnapshot {
    crate::entity::tests::registry_tag_snapshot(
        &tags
            .iter()
            .map(|(tag, members)| ("minecraft:entity_type", *tag, *members))
            .collect::<Vec<_>>(),
    )
}

#[test]
fn player_effect_admission_follows_the_ignores_poison_tag() {
    let member =
        crate::entity::EntityRegistryTagView::Frozen(std::sync::Arc::new(entity_tag_snapshot(&[
            ("minecraft:ignores_poison_and_regen", &["minecraft:player"]),
        ])));
    let not_member =
        crate::entity::EntityRegistryTagView::Frozen(std::sync::Arc::new(entity_tag_snapshot(&[])));

    assert!(
        !crate::entity::player_mob_effect_admitted_with_registry_tag_view(
            &member,
            "minecraft:poison"
        )
    );
    assert!(
        crate::entity::player_mob_effect_admitted_with_registry_tag_view(
            &not_member,
            "minecraft:poison"
        )
    );
}

#[test]
fn mob_effect_runtime_uses_generated_attribute_descriptors() {
    let mut living = zombie();
    let before = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    let speed = vanilla_mob_effect_type_for_resource_id("minecraft:speed").unwrap();
    let descriptor = speed.attribute_modifiers[0];
    assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:speed", 200, 1).unwrap()));
    assert_eq!(
        living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        before * (1.0 + f64::from_bits(descriptor.amount_bits) * 2.0)
    );
    assert!(living
        .packed_attributes()
        .iter()
        .flat_map(|attribute| &attribute.modifiers)
        .any(|modifier| modifier.id == descriptor.modifier_id));

    assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:slowness", 200, 0).unwrap()));
    let slowness = vanilla_mob_effect_type_for_resource_id("minecraft:slowness").unwrap();
    assert_eq!(
        living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        before
            * (1.0 + f64::from_bits(descriptor.amount_bits) * 2.0)
            * (1.0 + f64::from_bits(slowness.attribute_modifiers[0].amount_bits))
    );

    let mut replaced = zombie();
    replaced.replace_active_effects(vec![
        ActiveMobEffect::new("minecraft:speed", 200, 1).unwrap()
    ]);
    assert_eq!(
        replaced.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        before * (1.0 + f64::from_bits(descriptor.amount_bits) * 2.0)
    );
}

#[test]
fn registry_tag_effect_admission_follows_entity_immunity_tags() {
    let mut native = NativeLevel::new();
    native.install_tags(entity_tag_snapshot(&[
        ("minecraft:immune_to_infested", &["minecraft:zombie"]),
        ("minecraft:immune_to_oozing", &["minecraft:zombie"]),
        ("minecraft:ignores_poison_and_regen", &["minecraft:zombie"]),
    ]));
    native.with_level(|level| {
        assert!(!can_be_affected_in_level(
            level,
            "minecraft:zombie",
            "minecraft:infested"
        ));
        assert!(can_be_affected_in_level(
            level,
            "minecraft:zombie",
            "minecraft:oozing"
        ));
    });
    native.install_tags(entity_tag_snapshot(&[(
        "minecraft:ignores_poison_and_regen",
        &["minecraft:zombie"],
    )]));
    native.with_level(|level| {
        assert!(!can_be_affected_in_level(
            level,
            "minecraft:zombie",
            "minecraft:poison"
        ));
        assert!(can_be_affected_in_level(
            level,
            "minecraft:zombie",
            "minecraft:speed"
        ));
    });
}

#[test]
fn fixed_subclass_effect_rejections_hold_without_tags() {
    let mut native = NativeLevel::new();
    native.install_tags(entity_tag_snapshot(&[]));
    native.with_level(|level| {
        for (entity, effect) in [
            ("minecraft:wither_skeleton", "minecraft:wither"),
            ("minecraft:parched", "minecraft:weakness"),
            ("minecraft:spider", "minecraft:poison"),
        ] {
            assert!(
                !can_be_affected_in_level(&*level, entity, effect),
                "{entity} rejects {effect}"
            );
        }
        assert!(can_be_affected_in_level(
            &*level,
            "minecraft:spider",
            "minecraft:speed"
        ));
    });
}

#[test]
fn boss_add_effect_overrides_reject_every_active_effect() {
    let mut store = EntityStore::new(0x57_17_E2, EntityIdCounter::new());
    let mut native = NativeLevel::new();
    native.install_tags(entity_tag_snapshot(&[]));
    native.with_level(|level| {
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_WITHER_ID,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        ] {
            let id = store
                .spawn_general_living(
                    entity_type_id,
                    Vec3d {
                        x: 0.5,
                        y: 64.0,
                        z: 0.5,
                    },
                )
                .unwrap();
            let mut boss = store.detach(id).unwrap();
            assert_eq!(
                add_active_effect_in_level(
                    &mut boss,
                    &mut store,
                    &*level,
                    ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap(),
                ),
                Some(false)
            );
            assert!(boss
                .living()
                .unwrap()
                .active_effect("minecraft:speed")
                .is_none());
        }
    });
}

#[test]
fn instant_damage_heals_inverted_healing_members() {
    let mut native = NativeLevel::new();
    let mut living = zombie();
    living.set_health(10.0);
    living.add_active_effect(ActiveMobEffect::new("minecraft:instant_damage", 1, 0).unwrap());
    native.install_tags(entity_tag_snapshot(&[(
        "minecraft:inverted_healing_and_harm",
        &["minecraft:zombie"],
    )]));
    let result =
        native.with_level(|level| living.tick_active_effects(level, "minecraft:zombie", 1));
    assert!(result.hurts.is_empty());
    assert_eq!(living.health(), 14.0);
    living.add_active_effect(ActiveMobEffect::new("minecraft:instant_damage", 1, 0).unwrap());
    native.install_tags(entity_tag_snapshot(&[(
        "minecraft:inverted_healing_and_harm",
        &[],
    )]));
    let result =
        native.with_level(|level| living.tick_active_effects(level, "minecraft:zombie", 2));
    assert_eq!(
        result.hurts,
        [MobEffectHurt {
            damage_type: "minecraft:magic",
            amount: 6.0
        }]
    );
}

/// `InstantenousMobEffect` has exactly three registrations
/// (`MobEffects.java:46-47`, `:88`), so the catalog sweep pins both a missing
/// one and a spurious one. `SaturationMobEffect` (`SaturationMobEffect.java:7`)
/// is the one an id list written from the heal-or-harm pair leaves out.
#[test]
fn the_instantaneous_effects_are_the_three_vanilla_registers() {
    let instantaneous = VANILLA_MOB_EFFECT_TYPES
        .iter()
        .map(|facts| facts.resource_id)
        .filter(|resource_id| is_instantaneous(resource_id))
        .collect::<Vec<_>>();
    assert_eq!(
        instantaneous,
        [
            "minecraft:instant_health",
            "minecraft:instant_damage",
            "minecraft:saturation"
        ]
    );
    // `InstantenousMobEffect.shouldApplyEffectTickThisTick`
    // (`InstantenousMobEffect.java:14-16`) holds at any remaining duration,
    // where an effect with no tick of its own never applies one.
    for resource_id in instantaneous {
        assert!(should_apply(resource_id, 7, 0), "{resource_id}");
        assert!(should_apply(resource_id, 1, 3), "{resource_id}");
    }
    assert!(!should_apply("minecraft:speed", 7, 0));
}

/// `/effect give` builds `new MobEffectInstance(holder, duration, amplifier, false, particles)`
/// (`EffectCommands.java:166`), which sets `showIcon = particles`
/// (`MobEffectInstance.java:60-61`). Duration `-1` is `INFINITE_DURATION`
/// (`MobEffectInstance.java:27`); duration `0` is refused the same way
/// `from_saved` refuses it. Amplifier is clamped (`:28-29`).
#[test]
fn give_builds_the_effect_instance_the_command_would() {
    let infinite = ActiveMobEffect::give("minecraft:speed", -1, 0, true).unwrap();
    assert_eq!(infinite.duration(), -1);
    assert!(infinite.show_particles());
    assert!(infinite.show_icon());
    assert!(!infinite.ambient());

    let hidden = ActiveMobEffect::give("minecraft:speed", 20, 0, false).unwrap();
    assert!(!hidden.show_particles());
    assert!(!hidden.show_icon());

    assert!(ActiveMobEffect::give("minecraft:speed", 0, 0, true).is_none());
    assert!(ActiveMobEffect::give("minecraft:not_an_effect", 20, 0, true).is_none());
    assert_eq!(
        ActiveMobEffect::give("minecraft:speed", 20, 300, true)
            .unwrap()
            .amplifier(),
        255
    );
    assert!(ActiveMobEffect::new("minecraft:speed", -1, 0).is_none());
}

/// The whole registry partitioned by what this crate ticks
/// (`MobEffects.java:15-123`). Every registration lands in exactly one of the
/// three, so a vanilla bump that adds an effect fails here instead of ticking
/// as a silent no-op.
#[test]
fn every_registered_effect_is_classified() {
    // `applyEffectTick` bodies `apply_effect_tick` runs.
    const RUNS_HERE: &[&str] = &[
        "minecraft:instant_health",
        "minecraft:instant_damage",
        "minecraft:regeneration",
        "minecraft:poison",
        "minecraft:wither",
        "minecraft:absorption",
        "minecraft:saturation",
        "minecraft:hunger",
    ];
    // Bodies blocked on one dependency each, named in `apply_effect_tick`.
    const BLOCKED: &[&str] = &["minecraft:bad_omen", "minecraft:raid_omen"];

    let mut inherits_the_default = Vec::new();
    for facts in VANILLA_MOB_EFFECT_TYPES {
        let resource_id = facts.resource_id;
        if RUNS_HERE.contains(&resource_id) || BLOCKED.contains(&resource_id) {
            continue;
        }
        // `MobEffect.shouldApplyEffectTickThisTick` (`MobEffect.java:91-93`)
        // keeps the rest out of `applyEffectTick` at every duration.
        for tick_count in 0..64 {
            for amplifier in 0..4 {
                assert!(
                    !should_apply(resource_id, tick_count, amplifier),
                    "{resource_id} {tick_count} {amplifier}"
                );
            }
        }
        inherits_the_default.push(resource_id);
    }
    assert_eq!(inherits_the_default.len(), 30);
    assert_eq!(
        RUNS_HERE.len() + BLOCKED.len() + inherits_the_default.len(),
        VANILLA_MOB_EFFECT_TYPES.len()
    );

    // Nothing may sit in the first list unreachably.
    for resource_id in RUNS_HERE {
        assert!(
            (0..64).any(|tick_count| should_apply(resource_id, tick_count, 0)),
            "{resource_id}"
        );
    }

    // `BadOmenMobEffect.java:15-17` and `RaidOmenMobEffect.java:15-17`: the
    // gates run even where the body does not.
    assert!(should_apply("minecraft:bad_omen", 7, 0));
    assert!(should_apply("minecraft:raid_omen", 1, 0));
    assert!(!should_apply("minecraft:raid_omen", 2, 0));
}

fn apply_hunger_tick(
    food: Option<&mut FoodData>,
    abilities_invulnerable: bool,
    amplifier: i32,
) -> (EffectTickApplication, Vec<MobEffectHurt>) {
    let mut attributes = AttributeMap::new(
        AttributeSupplier::for_entity_type("minecraft:player").expect("a player has attributes"),
    );
    let mut health = 20.0;
    let mut absorption = 0.0;
    let mut body = MobEffectBody {
        attributes: &mut attributes,
        health: &mut health,
        absorption_amount: &mut absorption,
        food,
        abilities_invulnerable,
    };
    let mut hurts = Vec::new();
    let application =
        apply_effect_tick(&mut body, "minecraft:hunger", amplifier, false, &mut hurts);
    (application, hurts)
}

/// `HungerMobEffect.shouldApplyEffectTickThisTick` (`HungerMobEffect.java:22-24`)
/// is unconditionally true, so every remaining duration applies.
#[test]
fn hunger_should_apply_every_tick() {
    assert!(should_apply("minecraft:hunger", 7, 0));
    assert!(should_apply("minecraft:hunger", 1, 3));
}

/// `HungerMobEffect.applyEffectTick` (`HungerMobEffect.java:13-19`) spends
/// `0.005F * (amplification + 1)` through `causeFoodExhaustion`.
#[test]
fn hunger_exhausts_food_by_the_amplifier_factor() {
    let mut food = FoodData::created();
    let (application, hurts) = apply_hunger_tick(Some(&mut food), false, 0);
    assert_eq!(application, EffectTickApplication::Keep);
    assert!(hurts.is_empty());
    assert_eq!(food.exhaustion_level(), 0.005);

    let mut amplified = FoodData::created();
    let (application, hurts) = apply_hunger_tick(Some(&mut amplified), false, 3);
    assert_eq!(application, EffectTickApplication::Keep);
    assert!(hurts.is_empty());
    assert_eq!(amplified.exhaustion_level(), 0.02);
}

/// `Player.causeFoodExhaustion` (`Player.java:1561-1567`) spends nothing when
/// `abilities.invulnerable` is set.
#[test]
fn hunger_spares_an_invulnerable_body() {
    let mut food = FoodData::created();
    let (application, hurts) = apply_hunger_tick(Some(&mut food), true, 3);
    assert_eq!(application, EffectTickApplication::Keep);
    assert!(hurts.is_empty());
    assert_eq!(food.exhaustion_level(), 0.0);
}

/// The `Player` cast (`HungerMobEffect.java:13-19`) fails on a body with no
/// `FoodData`, so a stored living entity is a no-op that still keeps the effect.
#[test]
fn hunger_is_a_noop_without_food_data() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let (application, hurts) = apply_hunger_tick(None, false, 3);
        assert_eq!(application, EffectTickApplication::Keep);
        assert!(hurts.is_empty());

        let mut living = zombie();
        let before = living.clone();
        living.add_active_effect(ActiveMobEffect::new("minecraft:hunger", 4, 0).unwrap());
        let result = living.tick_active_effects(&*level, "minecraft:zombie", 1);
        assert!(result.hurts.is_empty());
        assert_eq!(living.health(), before.health());
        assert_eq!(
            living.active_effect("minecraft:hunger").unwrap().duration(),
            3
        );
    })
}

#[test]
fn mob_effect_runtime_updates_and_downgrades_hidden_effects() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut living = zombie();
        let base_speed = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap()));
        assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:speed", 20, 1).unwrap()));
        let effect = living.active_effect("minecraft:speed").unwrap();
        assert_eq!((effect.amplifier(), effect.duration()), (1, 20));
        assert_eq!(effect.hidden_effect.as_deref().unwrap().duration(), 200);

        for tick in 1..=20 {
            living.tick_active_effects(&*level, "minecraft:zombie", tick);
        }
        let effect = living.active_effect("minecraft:speed").unwrap();
        assert_eq!((effect.amplifier(), effect.duration()), (0, 180));
        let speed = vanilla_mob_effect_type_for_resource_id("minecraft:speed").unwrap();
        let expected =
            base_speed * (1.0 + f64::from_bits(speed.attribute_modifiers[0].amount_bits));
        assert_eq!(
            living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
            expected
        );
    })
}

#[test]
fn mob_effect_runtime_ticks_infinite_regen_and_damage_intervals() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut living = zombie();
        let max = living.max_health();
        living.set_health(max - 2.0);
        assert!(living.add_active_effect(
            ActiveMobEffect::from_saved("minecraft:regeneration", -1, 0, false, true, true)
                .unwrap()
        ));
        let result = living.tick_active_effects(&*level, "minecraft:zombie", 50);
        assert!(result.hurts.is_empty());
        assert_eq!(living.health(), max - 1.0);
        assert_eq!(
            living
                .active_effect("minecraft:regeneration")
                .unwrap()
                .duration(),
            -1
        );

        assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:poison", 25, 0).unwrap()));
        assert!(living.add_active_effect(ActiveMobEffect::new("minecraft:wither", 40, 0).unwrap()));
        let result = living.tick_active_effects(&*level, "minecraft:zombie", 51);
        assert!(result.hurts.contains(&MobEffectHurt {
            damage_type: "minecraft:magic",
            amount: 1.0,
        }));
        assert!(result.hurts.contains(&MobEffectHurt {
            damage_type: "minecraft:wither",
            amount: 1.0,
        }));

        living.set_health(1.0);
        living.replace_active_effects(vec![
            ActiveMobEffect::new("minecraft:poison", 25, 0).unwrap()
        ]);
        assert!(living
            .tick_active_effects(&*level, "minecraft:zombie", 52)
            .hurts
            .is_empty());

        living.replace_active_effects(vec![
            ActiveMobEffect::new("minecraft:instant_health", 1, 0).unwrap()
        ]);
        living.set_health(1.0);
        let result = living.tick_active_effects(&*level, "minecraft:zombie", 0);
        assert_eq!(
            result.hurts,
            [MobEffectHurt {
                damage_type: "minecraft:magic",
                amount: 6.0,
            }]
        );
        assert_eq!(living.health(), 1.0);
    })
}

#[test]
fn mob_effect_runtime_absorption_starts_and_removes_with_attribute() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut living = zombie();
        assert!(
            living.add_active_effect(ActiveMobEffect::new("minecraft:absorption", 20, 0).unwrap())
        );
        assert_eq!(
            living.attribute_value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID),
            4.0
        );
        assert_eq!(living.absorption_amount(), 4.0);

        living.set_absorption_amount(0.0);
        assert!(
            living
                .tick_active_effects(&*level, "minecraft:zombie", 1)
                .effects_changed
        );
        assert!(living.active_effect("minecraft:absorption").is_none());
        assert_eq!(
            living.attribute_value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID),
            0.0
        );
    })
}

#[test]
fn mob_effect_runtime_fire_resistance_and_resistance_share_hurt_pipeline() {
    let mut store = EntityStore::new(0x00EF_FEC7, EntityIdCounter::new());
    let fire_id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let fire = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:on_fire")
        .unwrap();
    let fire_entity = store.get_mut(fire_id).unwrap();
    fire_entity
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:fire_resistance", 200, 0).unwrap());
    let health = fire_entity.living().unwrap().health();
    let result = hurt_server_with_runtime(
        fire_entity,
        &mut EntityStore::new(0, EntityIdCounter::new()),
        StoredEntityDamageSource {
            damage_type: fire,
            direct_entity_id: None,
            causing_entity: None,
            attacker_views: Default::default(),
        },
        4.0,
        StoredEntityDamageRules::default(),
    );
    assert!(!result.outcome.applied);
    assert_eq!(fire_entity.living().unwrap().health(), health);

    let generic = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:generic")
        .unwrap();
    let mut losses = Vec::new();
    for resistant in [false, true] {
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        if resistant {
            entity
                .living_mut()
                .unwrap()
                .add_active_effect(ActiveMobEffect::new("minecraft:resistance", 200, 0).unwrap());
        }
        let before = entity.living().unwrap().health();
        hurt_server_with_runtime(
            entity,
            &mut EntityStore::new(0, EntityIdCounter::new()),
            StoredEntityDamageSource {
                damage_type: generic,
                direct_entity_id: None,
                causing_entity: None,
                attacker_views: Default::default(),
            },
            10.0,
            StoredEntityDamageRules::default(),
        );
        losses.push(before - entity.living().unwrap().health());
    }
    assert!((losses[1] - losses[0] * 0.8).abs() < 1.0e-5);
}

#[test]
fn mob_effect_runtime_water_breathing_controls_actual_eye_water_air() {
    let run = |effect: Option<ActiveMobEffect>| {
        let mut native = NativeLevel::new();
        native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
        native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:water"));
        let mut store = EntityStore::new(0xA1_2E, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .unwrap();
        let entity = store.get_mut(id).unwrap();
        entity.set_air_supply(10);
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        if let Some(effect) = effect {
            entity.living_mut().unwrap().add_active_effect(effect);
        }
        native.with_level(|level| {
            crate::entity::tick_one_entity(&mut store, level, id);
        });
        store.get(id).unwrap().air_supply()
    };
    assert_eq!(run(None), 9);
    assert_eq!(
        run(ActiveMobEffect::new("minecraft:water_breathing", 200, 0)),
        14
    );
}

#[test]
fn mob_effect_runtime_save_round_trips_hidden_details() {
    for resource_id in [
        "minecraft:speed",
        "minecraft:instant_health",
        "minecraft:bad_omen",
    ] {
        let hidden = ActiveMobEffect::from_saved(resource_id, 40, 2, true, false, false).unwrap();
        let effect = ActiveMobEffect::from_saved_with_hidden(
            resource_id,
            -1,
            255,
            false,
            true,
            true,
            Some(hidden),
        )
        .unwrap();
        let encoded = crate::entity::save::write_active_mob_effect(&effect);
        assert_eq!(saved_codec::decode_saved_effect(&encoded).unwrap(), effect);
    }
}

#[test]
fn mob_effect_runtime_particles_keep_argb_and_special_types() {
    let mut living = zombie();
    living.add_active_effect(ActiveMobEffect::new("minecraft:speed", 20, 0).unwrap());
    let speed = living.visible_effect_particles().pop().unwrap();
    let color = vanilla_mob_effect_type_for_resource_id("minecraft:speed")
        .unwrap()
        .color;
    assert_eq!(speed.raw_options, ((255_u32 << 24) | color).to_be_bytes());

    living.add_active_effect(ActiveMobEffect::new("minecraft:trial_omen", 20, 0).unwrap());
    assert!(living
        .visible_effect_particles()
        .iter()
        .any(
            |particle| particle.particle_type_id == VANILLA_PARTICLE_TYPE_TRIAL_OMEN_ID
                && particle.raw_options.is_empty()
        ));
}

/// `updateInvisibilityStatus` on an emptied map only runs
/// `removeEffectParticles` (`LivingEntity.java:870-873`, `:949-951`).
#[test]
fn mob_effect_runtime_last_effect_clears_particles_but_keeps_ambience() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut store = EntityStore::new(0x00EF_FEC8, EntityIdCounter::new());
        let id = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
            .unwrap();
        let mut entity = store.detach(id).unwrap();
        entity.living_mut().unwrap().add_active_effect(
            ActiveMobEffect::from_saved("minecraft:speed", 1, 0, true, true, true).unwrap(),
        );
        synchronize_metadata(&mut entity);
        assert!(entity.living().unwrap().effect_ambience);
        entity.take_synched_dirty();

        entity
            .living_mut()
            .unwrap()
            .tick_active_effects(&*level, "minecraft:zombie", 1);
        synchronize_metadata(&mut entity);
        assert!(entity.living().unwrap().effect_ambience);
        assert!(entity.living().unwrap().effect_particles.is_empty());
        let dirty = entity.take_synched_dirty();
        assert!(dirty.contains(&VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID));
        assert!(!dirty.contains(&VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID));
    })
}

#[test]
fn special_mob_effect_actions_distinguish_removal_reasons_and_non_mob_living() {
    let mut store = EntityStore::new(0x5EED_EFFE, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    store.drain_lifecycle_events();
    for effect in [
        "minecraft:wind_charged",
        "minecraft:weaving",
        "minecraft:oozing",
    ] {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .add_active_effect(ActiveMobEffect::new(effect, 200, 0).unwrap());
    }
    let removed = store.remove(id, RemovalReason::Killed).unwrap();
    assert_eq!(removed.living().unwrap().active_effects().count(), 0);
    assert!(store.drain_lifecycle_events().is_empty());
    let actions = store.drain_mob_effect_actions();
    let PendingMobEffectAction::Removed(action) = &actions[0] else {
        panic!("special removal action")
    };
    assert_eq!(action.effects.len(), 3);
    for effect in [
        PendingRemovalMobEffect::Oozing,
        PendingRemovalMobEffect::Weaving,
        PendingRemovalMobEffect::WindCharged,
    ] {
        assert!(action.effects.contains(&effect));
    }

    let discarded = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    store
        .get_mut(discarded)
        .unwrap()
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:wind_charged", 200, 0).unwrap());
    let removed = store.remove(discarded, RemovalReason::Discarded).unwrap();
    assert_eq!(removed.living().unwrap().active_effects().count(), 0);
    assert!(store.drain_commands().is_empty());

    let unloaded = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    store
        .get_mut(unloaded)
        .unwrap()
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:wind_charged", 200, 0).unwrap());
    let removed = store
        .remove(unloaded, RemovalReason::UnloadedToChunk)
        .unwrap();
    assert!(removed
        .living()
        .unwrap()
        .active_effect("minecraft:wind_charged")
        .is_some());
    assert!(store.drain_commands().is_empty());

    let armor_stand = crate::entity::spawn_armor_stand(&mut store, Vec3d::default()).unwrap();
    store
        .get_mut(armor_stand)
        .unwrap()
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:wind_charged", 200, 0).unwrap());
    store.remove(armor_stand, RemovalReason::Killed).unwrap();
    assert!(matches!(
        store.drain_mob_effect_actions().as_slice(),
        [PendingMobEffectAction::Removed(_)]
    ));
}

#[test]
fn special_mob_effect_infested_real_hurt_spawns_silverfish() {
    let mut native = NativeLevel::new();
    for _ in 0..100 {
        let (id, _) = native.world.spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        );
        let entity = native
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap();
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.set_no_ai(true);
        }
        entity
            .living_mut()
            .unwrap()
            .add_active_effect(ActiveMobEffect::new("minecraft:infested", 200, 0).unwrap());
        assert!(
            native
                .world
                .hurt_stored_entity_by_command_for_test(
                    DIMENSION,
                    id,
                    "minecraft:generic",
                    1.0,
                    None
                )
                .unwrap()
                .succeeded
        );
    }
    native.world.tick(Vec::new());
    assert!(native
        .world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .any(|id| {
            native
                .world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID
        }));
}

#[test]
fn mob_effect_runtime_expires_a_hidden_effect_that_outlived_its_own_duration() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        // `tickDownDuration` recurses into the hidden effect unconditionally
        // (`MobEffectInstance.java:255-261`), so a hidden effect the main instance
        // was extended past keeps being ticked after it reaches zero. `mapDuration`
        // pins an exhausted duration at zero (`:195-197`) instead of letting it
        // reach the `-1` infinite sentinel (`:181-183`), so the downgrade
        // (`:263-270`) promotes an exhausted instance that `hasRemainingDuration`
        // (`:251-253`) then drops.
        let mut living = zombie();
        assert!(
            living.add_active_effect(ActiveMobEffect::new("minecraft:glowing", 300, 0).unwrap())
        );
        assert!(
            living.add_active_effect(ActiveMobEffect::new("minecraft:glowing", 100, 1).unwrap())
        );
        // Amplifier 2 is stronger and not shorter, so it replaces the details
        // without pushing a second hidden entry (`:138-147`).
        assert!(
            living.add_active_effect(ActiveMobEffect::new("minecraft:glowing", 301, 2).unwrap())
        );
        let effect = living.active_effect("minecraft:glowing").unwrap();
        assert_eq!((effect.amplifier(), effect.duration()), (2, 301));
        assert_eq!(effect.hidden_effect.as_deref().unwrap().duration(), 300);

        for tick in 1..=300 {
            living.tick_active_effects(&*level, "minecraft:zombie", tick);
        }
        let effect = living.active_effect("minecraft:glowing").unwrap();
        assert_eq!((effect.amplifier(), effect.duration()), (2, 1));
        assert_eq!(effect.hidden_effect.as_deref().unwrap().duration(), 0);

        let result = living.tick_active_effects(&*level, "minecraft:zombie", 301);
        assert!(result.effects_changed);
        assert!(
            living.active_effect("minecraft:glowing").is_none(),
            "the promoted hidden effect was already exhausted"
        );
    })
}

#[test]
fn mob_effect_update_drops_the_incoming_instance_own_hidden_effect() {
    // `this.hiddenEffect = new MobEffectInstance(takeOver)`
    // (`MobEffectInstance.java:148-157`) is the details-only copy constructor
    // (`:88-91`), and `setDetailsFrom` (`:124-130`) leaves `hiddenEffect` out.
    let mut current = ActiveMobEffect::new("minecraft:speed", 100, 1).unwrap();
    let incoming = ActiveMobEffect::from_saved_with_hidden(
        "minecraft:speed",
        400,
        0,
        false,
        true,
        true,
        Some(ActiveMobEffect::from_saved("minecraft:speed", 900, 0, false, true, true).unwrap()),
    )
    .unwrap();

    // A weaker but longer instance is filed underneath and reports no change.
    assert!(!current.update(incoming));
    assert_eq!((current.amplifier(), current.duration()), (1, 100));
    let hidden = current.hidden_effect.as_deref().unwrap();
    assert_eq!((hidden.amplifier(), hidden.duration()), (0, 400));
    assert!(
        hidden.hidden_effect.is_none(),
        "the incoming instance's own hidden effect is not carried across"
    );
}

#[test]
fn remove_effect_drops_the_instance_and_its_attribute_modifiers() {
    // `LivingEntity.removeEffect` (`LivingEntity.java:1044-1052`) over
    // `onEffectsRemoved` (`:1084-1100`).
    let mut store = EntityStore::new(0x2E_10_1E, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let mut zombie = store.detach(id).unwrap();
    let base_speed = zombie
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    assert!(zombie
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:speed", 200, 1).unwrap()));
    assert_ne!(
        zombie
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        base_speed
    );
    zombie.living_mut().unwrap().effects.clear_dirty();

    let removed = remove_active_effect(&mut zombie, &mut store, "minecraft:speed").unwrap();

    assert_eq!((removed.amplifier(), removed.duration()), (1, 200));
    assert!(zombie
        .living()
        .unwrap()
        .active_effect("minecraft:speed")
        .is_none());
    assert_eq!(
        zombie
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        base_speed
    );
    assert!(zombie.living().unwrap().effects.is_dirty());
}

#[test]
fn remove_effect_answers_nothing_for_an_effect_that_was_not_active() {
    // `removeEffectNoUpdate` answers null and `removeEffect` returns false
    // without touching anything (`LivingEntity.java:1040-1052`).
    let mut store = EntityStore::new(0x2E_10_1F, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let mut zombie = store.detach(id).unwrap();
    assert!(zombie
        .living_mut()
        .unwrap()
        .add_active_effect(ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap()));
    zombie.living_mut().unwrap().effects.clear_dirty();

    assert!(remove_active_effect(&mut zombie, &mut store, "minecraft:poison").is_none());
    assert!(
        !zombie.living().unwrap().effects.is_dirty(),
        "a miss runs none of onEffectsRemoved"
    );
    assert!(zombie
        .living()
        .unwrap()
        .active_effect("minecraft:speed")
        .is_some());
}

#[test]
fn remove_all_effects_clears_every_instance_and_answers_them() {
    // `LivingEntity.removeAllEffects` (`LivingEntity.java:953-966`).
    let mut store = EntityStore::new(0x2E_10_20, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let mut zombie = store.detach(id).unwrap();
    let base_speed = zombie
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    for effect in ["minecraft:speed", "minecraft:glowing"] {
        assert!(zombie
            .living_mut()
            .unwrap()
            .add_active_effect(ActiveMobEffect::new(effect, 200, 0).unwrap()));
    }

    let removed = remove_all_active_effects(&mut zombie, &mut store);

    let mut removed = removed
        .iter()
        .map(ActiveMobEffect::resource_id)
        .collect::<Vec<_>>();
    removed.sort_unstable();
    assert_eq!(removed, ["minecraft:glowing", "minecraft:speed"]);
    assert_eq!(zombie.living().unwrap().active_effects().count(), 0);
    assert_eq!(
        zombie
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        base_speed
    );
}

#[test]
fn remove_all_effects_on_an_unaffected_entity_answers_nothing() {
    // `removeAllEffects` returns false before it touches `onEffectsRemoved`
    // when the map is empty (`LivingEntity.java:958-960`).
    let mut store = EntityStore::new(0x2E_10_21, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, Vec3d::default())
        .unwrap();
    let mut zombie = store.detach(id).unwrap();
    zombie.living_mut().unwrap().effects.clear_dirty();

    assert!(remove_all_active_effects(&mut zombie, &mut store).is_empty());
    assert!(!zombie.living().unwrap().effects.is_dirty());
}

/// A session avatar watching from the origin. The Player body itself is not a
/// stored entity here, so the viewer is all this crate holds of it.
fn session_viewer(connection: u64, entity_id: i32) -> crate::entity::tracking::TrackingViewer {
    crate::entity::tracking::TrackingViewer {
        connection_id: crate::connection::ConnectionId(connection),
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        view_distance: 32,
        uuid: uuid::Uuid::from_u128(
            0xB000_0000_0000_0000_0000_0000_0000_0000 + u128::from(connection),
        ),
        entity_id,
    }
}

/// A pig the session rides, with the mount's own `PassengersChanged` edge
/// withheld from the tracker: its published passenger list stays empty while
/// `store.passengers` already answers with the rider.
fn pig_ridden_by(
    store: &mut EntityStore,
    rider: &crate::entity::tracking::TrackingViewer,
) -> (i32, crate::entity::tracking::EntityTracker) {
    let pig = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
        )
        .expect("the store accepts a pig");
    assert!(store.attach_test_player(
        rider.entity_id,
        pig,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    let mut tracker = crate::entity::tracking::EntityTracker::new();
    let mut out = Vec::new();
    let mut events = store.drain_lifecycle_events();
    let queued = events.len();
    events.retain(|event| {
        !matches!(
            event,
            crate::entity::EntityLifecycleEvent::PassengersChanged { .. }
        )
    });
    assert_eq!(
        queued - events.len(),
        1,
        "the mount queued one passenger edge, and the tracker is not given it"
    );
    tracker.apply_lifecycle_events(
        &events,
        std::slice::from_ref(rider),
        |_, _, _| true,
        &mut out,
    );
    (pig, tracker)
}

fn effect_packets_seen_by(
    store: &mut EntityStore,
    tracker: &mut crate::entity::tracking::EntityTracker,
    viewers: &[crate::entity::tracking::TrackingViewer],
) -> Vec<(
    crate::connection::ConnectionId,
    crate::connection::ClientboundPlayPacket,
)> {
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        viewers,
        |_, _, _| true,
        &mut out,
    );
    out.retain(|targeted| {
        matches!(
            targeted.packet,
            crate::connection::ClientboundPlayPacket::UpdateMobEffect(_)
                | crate::connection::ClientboundPlayPacket::RemoveMobEffect(_)
        )
    });
    out.iter()
        .flat_map(|targeted| {
            targeted
                .recipients
                .iter()
                .map(|recipient| (*recipient, targeted.packet.clone()))
        })
        .collect()
}

fn effect_packets(
    store: &mut EntityStore,
    tracker: &mut crate::entity::tracking::EntityTracker,
    rider: &crate::entity::tracking::TrackingViewer,
) -> Vec<(
    crate::connection::ConnectionId,
    crate::connection::ClientboundPlayPacket,
)> {
    effect_packets_seen_by(store, tracker, std::slice::from_ref(rider))
}

/// Whether the store is holding a queued effect send, which is what the
/// `ServerPlayer` filter in `sendEffectToPassengers` decides
/// (`LivingEntity.java:1062-1068`).
fn queued_mob_effect_change(store: &EntityStore) -> bool {
    store.peek_lifecycle_events().iter().any(|event| {
        matches!(
            event,
            crate::entity::EntityLifecycleEvent::MobEffectChanged { .. }
        )
    })
}

fn add_effect_to_detached(
    store: &mut EntityStore,
    pig: i32,
    effect: ActiveMobEffect,
) -> Option<bool> {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut target = store.detach(pig).expect("the pig is stored");
        let answer = add_active_effect_in_level(&mut target, store, &*level, effect);
        store.attach(target);
        answer
    })
}

/// `sendEffectToPassengers` reads `this.getPassengers()` live
/// (`LivingEntity.java:1062-1068`), so an effect added after the session
/// mounted reaches it even though the tracker has published no passenger.
#[test]
fn mob_effect_update_reaches_a_passenger_the_tracker_has_not_published() {
    let mut store = EntityStore::new(0x3F_FE_C7, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);

    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::new("minecraft:speed", 200, 1).unwrap()
        ),
        Some(true)
    );

    let sent = effect_packets(&mut store, &mut tracker, &rider);
    let update = match sent.as_slice() {
        [(connection, crate::connection::ClientboundPlayPacket::UpdateMobEffect(update))] => {
            assert_eq!(*connection, rider.connection_id);
            *update
        }
        other => panic!("the mounted session is the whole audience, got {other:?}"),
    };
    assert_eq!(update.entity_id, pig);
    assert_eq!(
        update.effect_id,
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID
    );
    assert_eq!((update.amplifier, update.duration_ticks), (1, 200));
    // `blend` is `ServerPlayer.onEffectAdded`'s own-connection argument
    // (`ServerPlayer.java:1636-1645`); the passenger send passes false
    // (`LivingEntity.java:1062-1068`).
    assert!(!update.flags.blend);
}

/// The same live read the other way: a session that dismounted is no longer in
/// `getPassengers()`, even though the tracker was told it was riding.
#[test]
fn mob_effect_update_skips_a_passenger_that_has_already_dismounted() {
    let mut store = EntityStore::new(0x3F_FE_C8, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &[crate::entity::EntityLifecycleEvent::PassengersChanged {
            vehicle_id: pig,
            passenger_ids: vec![rider.entity_id],
        }],
        std::slice::from_ref(&rider),
        |_, _, _| true,
        &mut out,
    );
    // Positive control on the same call site: while mounted, the effect lands.
    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap()
        ),
        Some(true)
    );
    assert_eq!(effect_packets(&mut store, &mut tracker, &rider).len(), 1);

    assert_eq!(store.detach_external_player(rider.entity_id), Some(pig));
    store.drain_lifecycle_events();

    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::new("minecraft:speed", 400, 1).unwrap()
        ),
        Some(true)
    );

    assert!(
        !queued_mob_effect_change(&store),
        "a dismounted session is not in getPassengers, so nothing is queued"
    );
    assert!(
        effect_packets(&mut store, &mut tracker, &rider).is_empty(),
        "the dismounted session is no longer a passenger"
    );
}

/// `sendEffectToPassengers` sends only to a passenger that is a `ServerPlayer`
/// (`LivingEntity.java:1062-1068`); a mob passenger has no connection.
#[test]
fn mob_effect_update_skips_a_passenger_that_is_not_a_session() {
    let mut store = EntityStore::new(0x3F_FE_C9, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);
    assert_eq!(store.detach_external_player(rider.entity_id), Some(pig));
    let chicken = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID,
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
        )
        .expect("the store accepts a chicken");
    assert!(store.link_existing_passenger(pig, chicken));
    store.drain_lifecycle_events();

    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap()
        ),
        Some(true)
    );

    assert_eq!(store.passengers(pig), [chicken]);
    assert!(
        !queued_mob_effect_change(&store),
        "a mob passenger is filtered out before the send is queued"
    );
    assert!(
        effect_packets(&mut store, &mut tracker, &rider).is_empty(),
        "a mob passenger has no connection to send to"
    );
}

/// `onEffectsRemoved` sends one `ClientboundRemoveMobEffectPacket` per removed
/// instance, inside the loop (`LivingEntity.java:1084-1100`).
#[test]
fn removing_every_effect_sends_one_removal_packet_per_instance() {
    let mut store = EntityStore::new(0x3F_FE_CA, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);
    for resource_id in [
        "minecraft:speed",
        "minecraft:glowing",
        "minecraft:regeneration",
    ] {
        assert_eq!(
            add_effect_to_detached(
                &mut store,
                pig,
                ActiveMobEffect::new(resource_id, 200, 0).unwrap()
            ),
            Some(true)
        );
    }
    store.drain_lifecycle_events();

    let mut target = store.detach(pig).expect("the pig is stored");
    let removed = remove_all_active_effects(&mut target, &mut store);
    store.attach(target);

    assert_eq!(removed.len(), 3);
    let mut effect_ids = effect_packets(&mut store, &mut tracker, &rider)
        .into_iter()
        .map(|(_, packet)| match packet {
            crate::connection::ClientboundPlayPacket::RemoveMobEffect(remove) => {
                assert_eq!(remove.entity_id, pig);
                remove.effect_id
            }
            other => panic!("only removals are queued here, got {other:?}"),
        })
        .collect::<Vec<_>>();
    let mut expected = vec![
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_GLOWING_ID,
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_REGENERATION_ID,
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID,
    ];
    effect_ids.sort_unstable();
    expected.sort_unstable();
    assert_eq!(effect_ids, expected);
}

/// `sendEffectToPassengers` reaches the vehicle's passengers, not everyone who
/// can see it (`LivingEntity.java:1062-1068`): a bystanding session tracking
/// the same pig is sent nothing.
#[test]
fn mob_effect_update_reaches_only_the_riding_session() {
    let mut store = EntityStore::new(0x3F_FE_CB, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let bystander = session_viewer(2, -3);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);

    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap()
        ),
        Some(true)
    );

    let sent = effect_packets_seen_by(&mut store, &mut tracker, &[rider, bystander]);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, rider.connection_id);
}

/// `onEffectUpdated` is handed the existing instance it merged into, not the
/// incoming one (`LivingEntity.java:998-1008`, `:1070-1082`). A weaker but
/// longer instance is filed underneath, leaving the live amplifier and
/// duration alone while its show flags are taken.
#[test]
fn mob_effect_update_publishes_the_merged_instance_not_the_incoming_one() {
    let mut store = EntityStore::new(0x3F_FE_CC, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);
    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::from_saved("minecraft:speed", 500, 2, false, true, true).unwrap(),
        ),
        Some(true)
    );
    store.drain_lifecycle_events();

    assert_eq!(
        add_effect_to_detached(
            &mut store,
            pig,
            ActiveMobEffect::from_saved("minecraft:speed", 900, 0, false, false, false).unwrap(),
        ),
        Some(true)
    );

    let sent = effect_packets(&mut store, &mut tracker, &rider);
    let update = match sent.as_slice() {
        [(_, crate::connection::ClientboundPlayPacket::UpdateMobEffect(update))] => *update,
        other => panic!("one merge, one packet, got {other:?}"),
    };
    // The merged instance keeps amplifier 2 and duration 500; only the show
    // flags moved. The incoming instance was amplifier 0 for 900 ticks.
    assert_eq!((update.amplifier, update.duration_ticks), (2, 500));
    assert_eq!(update.flags.raw, 0);
}

/// `LivingEntity.addAdditionalSaveData` (`LivingEntity.java:732-734`) over
/// `readAdditionalSaveData` (`:783-789`).
#[test]
fn an_effect_map_round_trips_through_the_saved_list() {
    let mut effects = ActiveMobEffects::default();
    assert!(
        effects.to_saved_list().is_empty(),
        "an empty map is the absent field, not an empty one",
    );

    let mut attributes = AttributeMap::new(
        AttributeSupplier::for_entity_type("minecraft:zombie").expect("a zombie has attributes"),
    );
    let mut health = 20.0;
    let mut absorption = 0.0;
    let mut body = MobEffectBody {
        attributes: &mut attributes,
        health: &mut health,
        absorption_amount: &mut absorption,
        food: None,
        abilities_invulnerable: false,
    };
    effects.add(
        &mut body,
        ActiveMobEffect::new("minecraft:speed", 200, 1).expect("a vanilla effect"),
    );
    effects.add(
        &mut body,
        ActiveMobEffect::new("minecraft:poison", 600, 0).expect("a vanilla effect"),
    );
    // The overwrite that hides the weaker instance under the stronger one
    // (`MobEffectInstance.update`, `MobEffectInstance.java:139-146`), so the
    // round trip has a `hidden_effect` to carry.
    effects.add(
        &mut body,
        ActiveMobEffect::new("minecraft:speed", 20, 3).expect("a vanilla effect"),
    );

    let saved = effects.to_saved_list();
    assert_eq!(saved.len(), 2);
    let restored = ActiveMobEffects::from_saved_list(&saved);
    assert_eq!(
        restored.iter().collect::<Vec<_>>(),
        effects.iter().collect::<Vec<_>>()
    );
    assert!(
        restored.is_dirty(),
        "`readAdditionalSaveData` sets `effectsDirty` for every restored effect (`:788`)",
    );
}

/// `tickEffects` runs two callbacks per instance and both owe the mount's
/// `ServerPlayer` passengers a packet: `onEffectsRemoved` on expiry
/// (`LivingEntity.java:837-838`) and `onEffectUpdated` on the six-hundredth
/// tick (`:839-840`). Only the add and remove entry points sent before.
#[test]
fn a_ticking_mount_tells_its_rider_what_expired_and_what_refreshed() {
    let mut store = EntityStore::new(0x3F_FE_11, EntityIdCounter::new());
    let rider = session_viewer(1, -2);
    let (pig, mut tracker) = pig_ridden_by(&mut store, &rider);
    // One instance the tick exhausts, one it carries to a duration divisible
    // by six hundred.
    for (resource_id, duration) in [("minecraft:glowing", 1), ("minecraft:speed", 601)] {
        assert_eq!(
            add_effect_to_detached(
                &mut store,
                pig,
                ActiveMobEffect::new(resource_id, duration, 0).unwrap()
            ),
            Some(true)
        );
    }
    store.drain_lifecycle_events();

    let mut native = NativeLevel::new();
    native.with_level(|level| {
        crate::entity::tick_one_entity(&mut store, level, pig);
    });

    let packets = effect_packets(&mut store, &mut tracker, &rider)
        .into_iter()
        .map(|(_, packet)| match packet {
            crate::connection::ClientboundPlayPacket::UpdateMobEffect(update) => {
                assert_eq!(update.entity_id, pig);
                assert!(!update.flags.blend, "`sendEffectToPassengers` passes false");
                ("update", update.effect_id, update.duration_ticks)
            }
            crate::connection::ClientboundPlayPacket::RemoveMobEffect(remove) => {
                assert_eq!(remove.entity_id, pig);
                ("remove", remove.effect_id, 0)
            }
            other => panic!("only effect changes are queued here, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(packets.len(), 2);
    assert!(packets.contains(&(
        "update",
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID,
        600
    )));
    assert!(packets.contains(&(
        "remove",
        bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_GLOWING_ID,
        0
    )));
    assert!(store
        .get(pig)
        .and_then(|entity| entity.living())
        .expect("the pig is still stored")
        .active_effects()
        .any(|effect| effect.resource_id() == "minecraft:speed"));
}
