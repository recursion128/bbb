use super::*;
use bbb_pack::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier,
    SpriteAnimation, SpriteAnimationFrame,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn legacy_random_gaussian_matches_java_samples() {
    let mut random = LegacyRandom::new(0);
    assert_close(random.next_gaussian(), 0.8025330637390305);
    assert_close(random.next_gaussian(), -0.9015460884175122);
    assert_close(random.next_gaussian(), 2.080920790428163);
}

#[test]
fn legacy_random_float_matches_java_samples() {
    let mut random = LegacyRandom::new(0);
    assert_close_f32(random.next_float(), 0.730_967_76);
    assert_close_f32(random.next_float(), 0.831_441);
    assert_close_f32(random.next_float(), 0.240_536_39);
}

#[test]
fn firework_rocket_trail_batch_matches_vanilla_client_tick_particle() {
    let mut resolver = test_resolver(0);
    let batch = resolver.firework_rocket_trail_particle_batch(FireworkRocketTrailParticleState {
        entity_id: 7,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        delta_movement: bbb_world::EntityVec3 {
            x: 0.2,
            y: 0.8,
            z: -0.4,
        },
    });

    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 0);
    assert_eq!(batch.commands.len(), 1);
    let mut expected_random = LegacyRandom::new(0);
    let command = &batch.commands[0];
    assert_particle_command(
        command,
        FIREWORK_PARTICLE_TYPE_ID,
        "minecraft:firework",
        [10.0, 64.0, -3.0],
        [
            expected_random.next_gaussian() * 0.05,
            -0.4,
            expected_random.next_gaussian() * 0.05,
        ],
        false,
    );
    assert_eq!(
        command.sprite_ids,
        vec![
            "minecraft:firework_0".to_string(),
            "minecraft:firework_1".to_string()
        ]
    );
    assert!(!command.option_firework_trail);
    assert!(!command.option_firework_twinkle);
}

#[test]
fn ominous_item_spawner_batch_matches_vanilla_client_tick_particles() {
    let mut resolver = test_resolver(0);
    let state = OminousItemSpawnerParticleState {
        entity_id: 11,
        position: bbb_world::EntityVec3 {
            x: -1.5,
            y: 80.0,
            z: 2.25,
        },
    };

    let batch = resolver.ominous_item_spawner_particle_batch(state);

    let mut expected_random = LegacyRandom::new(0);
    let particle_count = expected_random.next_i32(3) + 1;
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 0);
    assert_eq!(batch.commands.len(), particle_count as usize);
    for command in &batch.commands {
        let velocity = [
            0.4 * (expected_random.next_gaussian() - expected_random.next_gaussian()),
            0.4 * (expected_random.next_gaussian() - expected_random.next_gaussian()),
            0.4 * (expected_random.next_gaussian() - expected_random.next_gaussian()),
        ];
        assert_particle_command(
            command,
            OMINOUS_SPAWNING_PARTICLE_TYPE_ID,
            "minecraft:ominous_spawning",
            [-1.5, 80.0, 2.25],
            velocity,
            true,
        );
        assert_eq!(
            command.sprite_ids,
            vec!["minecraft:ominous_spawning_0".to_string()]
        );
    }
}

#[test]
fn tracking_emitter_batch_spawns_totem_particles_around_entity_bounds() {
    let mut resolver = test_resolver(0);
    let batch = resolver.tracking_emitter_particle_batch(TrackingEmitterParticleState {
        particle_type_id: TOTEM_OF_UNDYING_PARTICLE_TYPE_ID,
        position: [10.0, 64.0, -3.0],
        width: 0.6,
        height: 1.8,
        lifetime_ticks: 2,
    });

    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 0);
    assert_eq!(batch.unknown_particle_type_count, 0);
    assert!(!batch.commands.is_empty());
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_type_id == TOTEM_OF_UNDYING_PARTICLE_TYPE_ID));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:totem_of_undying"));
    assert!(batch
        .commands
        .iter()
        .any(|command| command.initial_delay_ticks == 0));
    assert!(batch
        .commands
        .iter()
        .any(|command| command.initial_delay_ticks == 1));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.initial_delay_ticks <= 1));

    assert_particle_command_with_delay(
        &batch.commands[0],
        TOTEM_OF_UNDYING_PARTICLE_TYPE_ID,
        "minecraft:totem_of_undying",
        [
            10.069_290_330_779_168,
            65.198_296_854_938_49,
            -3.077_839_085_572_524,
        ],
        [
            0.461_935_520_172_119_14,
            0.862_881_970_405_578_6,
            -0.518_927_216_529_846_2,
        ],
        false,
        0,
    );
}

#[test]
fn take_item_entity_pickup_batch_preserves_source_and_target_context() {
    let mut resolver = test_resolver(0);
    let state = TakeItemEntityPickupParticleState {
        item_entity_id: 10,
        item_entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        item_position: bbb_world::EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        item_delta_movement: bbb_world::EntityVec3 {
            x: 0.1,
            y: 0.2,
            z: -0.3,
        },
        item_y_rot: 20.0,
        item_x_rot: -10.0,
        item_age_ticks: 9.5,
        item_light: TerrainLight { block: 6, sky: 12 },
        target_entity_id: 20,
        target_position: bbb_world::EntityVec3 {
            x: 4.0,
            y: 70.0,
            z: 8.0,
        },
        target_eye_height: 1.62,
        item_stack: Some(ItemStackSummary {
            item_id: Some(42),
            count: 5,
            component_patch: Default::default(),
        }),
        experience_orb_icon: None,
        projectile_model: None,
    };

    let batch = resolver.take_item_entity_pickup_particle_batch(&state);

    assert_eq!(batch.commands.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, ITEM_PICKUP_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, ITEM_PICKUP_PARTICLE_ID);
    assert!(command.sprite_ids.is_empty());
    assert_eq!(command.position, [1.0, 64.0, -2.0]);
    assert_eq!(command.velocity, [0.1, 0.2, -0.3]);
    assert!(command.override_limiter);
    assert!(!command.always_show);
    assert_eq!(command.raw_options_len, 0);
    let target = command.option_target.expect("pickup target");
    assert_eq!([target[0], target[2]], [4.0, 8.0]);
    assert_close(target[1], 70.0 + f64::from(1.62_f32 * 0.5));
    assert_eq!(
        command.option_entity_target_source,
        Some(ParticleEntityTargetSource {
            entity_id: 20,
            y_offset: 1.62 * 0.5,
        })
    );
    assert_eq!(command.option_item_pickup_source_entity_id, Some(10));
    assert_eq!(command.option_item_pickup_age_ticks, Some(9.5));
    assert_eq!(
        command.option_item_pickup_light,
        Some([6.0 / 15.0, 12.0 / 15.0])
    );
    assert_eq!(command.option_item_pickup_experience_orb_icon, None);
    assert_eq!(command.option_item_pickup_projectile_model, None);
    assert_eq!(
        command.option_item,
        Some(ParticleItemOptionState {
            item_id: 42,
            count: 5,
            component_patch_len: 0,
        })
    );
}

#[test]
fn take_item_entity_pickup_batch_preserves_experience_orb_icon() {
    let mut resolver = test_resolver(0);
    let state = TakeItemEntityPickupParticleState {
        item_entity_id: 11,
        item_entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        item_position: bbb_world::EntityVec3 {
            x: 2.0,
            y: 65.0,
            z: -3.0,
        },
        item_delta_movement: bbb_world::EntityVec3 {
            x: 0.0,
            y: 0.1,
            z: 0.0,
        },
        item_y_rot: 0.0,
        item_x_rot: 0.0,
        item_age_ticks: 4.5,
        item_light: TerrainLight { block: 15, sky: 9 },
        target_entity_id: 20,
        target_position: bbb_world::EntityVec3 {
            x: 4.0,
            y: 70.0,
            z: 8.0,
        },
        target_eye_height: 1.62,
        item_stack: None,
        experience_orb_icon: Some(8),
        projectile_model: None,
    };

    let batch = resolver.take_item_entity_pickup_particle_batch(&state);

    let command = &batch.commands[0];
    assert_eq!(command.option_item, None);
    assert_eq!(command.option_item_pickup_source_entity_id, Some(11));
    assert_eq!(command.option_item_pickup_age_ticks, Some(4.5));
    assert_eq!(command.option_item_pickup_light, Some([1.0, 9.0 / 15.0]));
    assert_eq!(command.option_item_pickup_experience_orb_icon, Some(8));
    assert_eq!(command.option_item_pickup_projectile_model, None);
}

#[test]
fn take_item_entity_pickup_batch_projects_projectile_models() {
    // Vanilla `ItemPickupParticle` extracts the picked entity's render state
    // (`extractEntity(entity, 1.0F)`); `ArrowRenderer` / `ThrownTridentRenderer`
    // consume its `yRot`/`xRot`, so the carried-model command field bundles the
    // kind with the extracted rotations.
    let mut resolver = test_resolver(0);
    let base_state = TakeItemEntityPickupParticleState {
        item_entity_id: 12,
        item_entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRIDENT_ID,
        item_position: bbb_world::EntityVec3 {
            x: 2.0,
            y: 65.0,
            z: -3.0,
        },
        item_delta_movement: bbb_world::EntityVec3 {
            x: 0.0,
            y: 0.1,
            z: 0.0,
        },
        item_y_rot: 35.0,
        item_x_rot: -12.0,
        item_age_ticks: 4.5,
        item_light: TerrainLight { block: 6, sky: 12 },
        target_entity_id: 20,
        target_position: bbb_world::EntityVec3 {
            x: 4.0,
            y: 70.0,
            z: 8.0,
        },
        target_eye_height: 1.62,
        item_stack: None,
        experience_orb_icon: None,
        projectile_model: Some(TakeItemEntityPickupProjectileModel::Trident { foil: true }),
    };

    let batch = resolver.take_item_entity_pickup_particle_batch(&base_state);
    let command = &batch.commands[0];
    assert_eq!(command.option_item, None);
    assert_eq!(command.option_item_pickup_experience_orb_icon, None);
    assert_eq!(
        command.option_item_pickup_projectile_model,
        Some(ParticleItemPickupProjectileModel {
            kind: ParticleItemPickupProjectileKind::Trident { foil: true },
            y_rot: 35.0,
            x_rot: -12.0,
        })
    );

    for (projectile_model, expected_kind) in [
        (
            TakeItemEntityPickupProjectileModel::Arrow { tipped: false },
            ParticleItemPickupProjectileKind::Arrow,
        ),
        (
            TakeItemEntityPickupProjectileModel::Arrow { tipped: true },
            ParticleItemPickupProjectileKind::TippedArrow,
        ),
        (
            TakeItemEntityPickupProjectileModel::SpectralArrow,
            ParticleItemPickupProjectileKind::SpectralArrow,
        ),
        (
            TakeItemEntityPickupProjectileModel::Trident { foil: false },
            ParticleItemPickupProjectileKind::Trident { foil: false },
        ),
    ] {
        let state = TakeItemEntityPickupParticleState {
            projectile_model: Some(projectile_model),
            ..base_state.clone()
        };
        let batch = resolver.take_item_entity_pickup_particle_batch(&state);
        let command = &batch.commands[0];
        assert_eq!(
            command.option_item_pickup_projectile_model,
            Some(ParticleItemPickupProjectileModel {
                kind: expected_kind,
                y_rot: 35.0,
                x_rot: -12.0,
            }),
            "kind {projectile_model:?}"
        );
    }
}

#[test]
fn ravager_roar_batch_spawns_poof_at_aabb_center_with_gaussian_velocity() {
    let mut resolver = test_resolver(0);
    let state = RavagerRoarParticleState {
        entity_id: 77,
        center: bbb_world::EntityVec3 {
            x: 1.0,
            y: 65.1,
            z: -2.0,
        },
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * 0.2,
        expected_random.next_gaussian() * 0.2,
        expected_random.next_gaussian() * 0.2,
    ];

    let batch = resolver.ravager_roar_particle_batch(state);

    assert_eq!(batch.len(), 40);
    assert_particle_command(
        &batch.commands[0],
        POOF_PARTICLE_TYPE_ID,
        "minecraft:poof",
        [1.0, 65.1, -2.0],
        expected_velocity,
        true,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.position == [1.0, 65.1, -2.0]));
}

#[test]
fn witch_magic_batch_matches_vanilla_entity_event_particles() {
    let mut resolver = test_resolver(0);
    let state = WitchMagicParticleState {
        entity_id: 71,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        bounding_box_max_y: 65.95,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_count = expected_random.next_i32(35) + 10;
    let expected_position = [
        10.0 + expected_random.next_gaussian() * 0.13_f32 as f64,
        65.95 + 0.5 + expected_random.next_gaussian() * 0.13_f32 as f64,
        -3.0 + expected_random.next_gaussian() * 0.13_f32 as f64,
    ];

    let batch = resolver.witch_magic_particle_batch(state);

    assert_eq!(batch.len(), expected_count as usize);
    assert_particle_command(
        &batch.commands[0],
        WITCH_PARTICLE_TYPE_ID,
        "minecraft:witch",
        expected_position,
        [0.0, 0.0, 0.0],
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.velocity == [0.0, 0.0, 0.0]));
}

#[test]
fn living_entity_poof_batch_matches_vanilla_make_poof_particles() {
    let mut resolver = test_resolver(0);
    let state = LivingEntityPoofParticleState {
        entity_id: 73,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.6,
        height: 1.95,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
    ];
    let width = f64::from(state.width);
    let height = f64::from(state.height);
    let expected_position = [
        10.0 + width * (2.0 * expected_random.next_f64() - 1.0) - expected_velocity[0] * 10.0,
        64.0 + height * expected_random.next_f64() - expected_velocity[1] * 10.0,
        -3.0 + width * (2.0 * expected_random.next_f64() - 1.0) - expected_velocity[2] * 10.0,
    ];

    let batch = resolver.living_entity_poof_particle_batch(state);

    assert_eq!(batch.len(), 20);
    assert_particle_command(
        &batch.commands[0],
        POOF_PARTICLE_TYPE_ID,
        "minecraft:poof",
        expected_position,
        expected_velocity,
        true,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:poof"));
}

#[test]
fn living_entity_drown_batch_matches_vanilla_make_drown_particles() {
    let mut resolver = test_resolver(0);
    let state = LivingEntityDrownParticleState {
        entity_id: 74,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        delta_movement: bbb_world::EntityVec3 {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        },
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_position = [
        10.0 + expected_random.next_f64() - expected_random.next_f64(),
        64.0 + expected_random.next_f64() - expected_random.next_f64(),
        -3.0 + expected_random.next_f64() - expected_random.next_f64(),
    ];

    let batch = resolver.living_entity_drown_particle_batch(state);

    assert_eq!(batch.len(), 8);
    assert_particle_command(
        &batch.commands[0],
        BUBBLE_PARTICLE_TYPE_ID,
        "minecraft:bubble",
        expected_position,
        [0.1, -0.2, 0.3],
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:bubble"));
}

#[test]
fn living_entity_portal_batch_matches_vanilla_event_particles() {
    let mut resolver = test_resolver(0);
    let state = LivingEntityPortalParticleState {
        entity_id: 75,
        previous_position: bbb_world::EntityVec3 {
            x: 9.0,
            y: 63.5,
            z: -4.0,
        },
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.6,
        height: 1.95,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        f64::from((expected_random.next_float() - 0.5) * 0.2),
        f64::from((expected_random.next_float() - 0.5) * 0.2),
        f64::from((expected_random.next_float() - 0.5) * 0.2),
    ];
    let width = f64::from(state.width);
    let height = f64::from(state.height);
    let expected_position = [
        state.previous_position.x + (expected_random.next_f64() - 0.5) * width * 2.0,
        state.previous_position.y + expected_random.next_f64() * height,
        state.previous_position.z + (expected_random.next_f64() - 0.5) * width * 2.0,
    ];

    let batch = resolver.living_entity_portal_particle_batch(state);

    assert_eq!(batch.len(), 128);
    assert_particle_command(
        &batch.commands[0],
        PORTAL_PARTICLE_TYPE_ID,
        "minecraft:portal",
        expected_position,
        expected_velocity,
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:portal"));
}

#[test]
fn snowball_hit_batch_matches_vanilla_event_particles() {
    let resolver = test_resolver(0);
    let state = SnowballHitParticleState {
        entity_id: 76,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        item_stack: Some(ItemStackSummary {
            item_id: Some(1017),
            count: 1,
            component_patch: Default::default(),
        }),
    };

    let batch = resolver.snowball_hit_particle_batch(state, None);

    assert_eq!(batch.len(), 8);
    assert_particle_command(
        &batch.commands[0],
        ITEM_PARTICLE_TYPE_ID,
        "minecraft:item",
        [10.0, 64.0, -3.0],
        [0.0, 0.0, 0.0],
        false,
    );
    assert_eq!(
        batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 1017,
            count: 1,
            component_patch_len: 0,
        })
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:item"));
}

#[test]
fn snowball_hit_batch_uses_item_snowball_for_empty_stack() {
    let resolver = test_resolver(0);
    let state = SnowballHitParticleState {
        entity_id: 77,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        item_stack: None,
    };

    let batch = resolver.snowball_hit_particle_batch(state, None);

    assert_eq!(batch.len(), 8);
    assert_particle_command(
        &batch.commands[0],
        ITEM_SNOWBALL_PARTICLE_TYPE_ID,
        "minecraft:item_snowball",
        [10.0, 64.0, -3.0],
        [0.0, 0.0, 0.0],
        false,
    );
    assert_eq!(batch.commands[0].option_item, None);
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:item_snowball"));
}

#[test]
fn thrown_egg_hit_batch_matches_vanilla_event_particles() {
    let mut resolver = test_resolver(0);
    let state = ThrownEggHitParticleState {
        entity_id: 78,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        item_stack: ItemStackSummary {
            item_id: Some(1032),
            count: 1,
            component_patch: Default::default(),
        },
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        f64::from((expected_random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
        f64::from((expected_random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
        f64::from((expected_random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
    ];

    let batch = resolver.thrown_egg_hit_particle_batch(state, None);

    assert_eq!(batch.len(), 8);
    assert_particle_command(
        &batch.commands[0],
        ITEM_PARTICLE_TYPE_ID,
        "minecraft:item",
        [10.0, 64.0, -3.0],
        expected_velocity,
        false,
    );
    assert_eq!(
        batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 1032,
            count: 1,
            component_patch_len: 0,
        })
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:item"));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.velocity != [0.0, 0.0, 0.0]));
}

#[test]
fn arrow_effect_batch_matches_vanilla_event_particles() {
    let mut resolver = test_resolver(0);
    let state = ArrowEffectParticleState {
        entity_id: 79,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.5,
        height: 0.5,
        color_rgb: 0x0033_66cc,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_position = [
        state.position.x
            + f64::from(state.width) * ((2.0 * expected_random.next_f64() - 1.0) * 0.5),
        state.position.y + f64::from(state.height) * expected_random.next_f64(),
        state.position.z
            + f64::from(state.width) * ((2.0 * expected_random.next_f64() - 1.0) * 0.5),
    ];
    let expected_color = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ];

    let batch = resolver.arrow_effect_particle_batch(state);

    assert_eq!(batch.len(), ARROW_EFFECT_PARTICLE_COUNT);
    assert_particle_command(
        &batch.commands[0],
        ENTITY_EFFECT_PARTICLE_TYPE_ID,
        "minecraft:entity_effect",
        expected_position,
        [0.0, 0.0, 0.0],
        false,
    );
    assert_eq!(batch.commands[0].option_color, Some(expected_color));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:entity_effect"));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.option_color == Some(expected_color)));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.velocity == [0.0, 0.0, 0.0]));
}

#[test]
fn animal_love_batch_matches_vanilla_event_particles() {
    let mut resolver = test_resolver(0);
    let state = AnimalLoveParticleState {
        entity_id: 80,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.9,
        height: 1.4,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
    ];
    let expected_position = [
        state.position.x + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
        state.position.y + f64::from(state.height) * expected_random.next_f64() + 0.5,
        state.position.z + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
    ];

    let batch = resolver.animal_love_particle_batch(state);

    assert_eq!(batch.len(), ANIMAL_LOVE_PARTICLE_COUNT);
    assert_particle_command(
        &batch.commands[0],
        HEART_PARTICLE_TYPE_ID,
        "minecraft:heart",
        expected_position,
        expected_velocity,
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:heart"));
    assert!(batch
        .commands
        .iter()
        .all(|command| command.velocity != [0.0, 0.0, 0.0]));
}

#[test]
fn allay_duplication_batch_matches_vanilla_event_particles() {
    let mut resolver = test_resolver(0);
    let state = AllayDuplicationParticleState {
        entity_id: 81,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.35,
        height: 0.6,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
    ];
    let expected_position = [
        state.position.x + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
        state.position.y + f64::from(state.height) * expected_random.next_f64() + 0.5,
        state.position.z + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
    ];

    let batch = resolver.allay_duplication_particle_batch(state);

    assert_eq!(batch.len(), ALLAY_DUPLICATION_PARTICLE_COUNT);
    assert_particle_command(
        &batch.commands[0],
        HEART_PARTICLE_TYPE_ID,
        "minecraft:heart",
        expected_position,
        expected_velocity,
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:heart"));
}

#[test]
fn entity_taming_batch_matches_vanilla_success_and_failure_particles() {
    let state = EntityTamingParticleState {
        entity_id: 82,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.6,
        height: 0.7,
        success: true,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
        expected_random.next_gaussian() * 0.02,
    ];
    let expected_position = [
        state.position.x + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
        state.position.y + f64::from(state.height) * expected_random.next_f64() + 0.5,
        state.position.z + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
    ];

    let mut success_resolver = test_resolver(0);
    let success_batch = success_resolver.entity_taming_particle_batch(state);
    let mut failure_resolver = test_resolver(0);
    let failure_batch = failure_resolver.entity_taming_particle_batch(EntityTamingParticleState {
        success: false,
        ..state
    });

    assert_eq!(success_batch.len(), ENTITY_TAMING_PARTICLE_COUNT);
    assert_particle_command(
        &success_batch.commands[0],
        HEART_PARTICLE_TYPE_ID,
        "minecraft:heart",
        expected_position,
        expected_velocity,
        false,
    );
    assert_eq!(failure_batch.len(), ENTITY_TAMING_PARTICLE_COUNT);
    assert_particle_command(
        &failure_batch.commands[0],
        SMOKE_PARTICLE_TYPE_ID,
        "minecraft:smoke",
        expected_position,
        expected_velocity,
        false,
    );
    assert!(failure_batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:smoke"));
}

#[test]
fn villager_event_batches_match_vanilla_particles() {
    for (kind, particle_type_id, particle_name) in [
        (
            VillagerParticleKind::Heart,
            HEART_PARTICLE_TYPE_ID,
            "minecraft:heart",
        ),
        (
            VillagerParticleKind::Angry,
            ANGRY_VILLAGER_PARTICLE_TYPE_ID,
            "minecraft:angry_villager",
        ),
        (
            VillagerParticleKind::Happy,
            HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            "minecraft:happy_villager",
        ),
        (
            VillagerParticleKind::Splash,
            SPLASH_PARTICLE_TYPE_ID,
            "minecraft:splash",
        ),
    ] {
        let state = VillagerParticleState {
            entity_id: 83,
            position: bbb_world::EntityVec3 {
                x: 10.0,
                y: 64.0,
                z: -3.0,
            },
            width: 0.6,
            height: 1.95,
            kind,
        };
        let mut expected_random = LegacyRandom::new(0);
        let expected_velocity = [
            expected_random.next_gaussian() * ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
            expected_random.next_gaussian() * ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
            expected_random.next_gaussian() * ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        ];
        let expected_position = [
            state.position.x + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
            state.position.y
                + f64::from(state.height) * expected_random.next_f64()
                + VILLAGER_PARTICLE_Y_OFFSET,
            state.position.z + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
        ];
        let mut resolver = test_resolver(0);

        let batch = resolver.villager_particle_batch(state);

        assert_eq!(batch.len(), VILLAGER_PARTICLE_COUNT);
        assert_particle_command(
            &batch.commands[0],
            particle_type_id,
            particle_name,
            expected_position,
            expected_velocity,
            false,
        );
        assert!(batch
            .commands
            .iter()
            .all(|command| command.particle_id == particle_name));
    }
}

#[test]
fn dolphin_happy_batch_matches_vanilla_event_particles() {
    let state = DolphinHappyParticleState {
        entity_id: 84,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        width: 0.9,
        height: 0.6,
    };
    let mut expected_random = LegacyRandom::new(0);
    let expected_velocity = [
        expected_random.next_gaussian() * DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
        expected_random.next_gaussian() * DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
        expected_random.next_gaussian() * DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
    ];
    let expected_position = [
        state.position.x + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
        state.position.y
            + f64::from(state.height) * expected_random.next_f64()
            + DOLPHIN_HAPPY_PARTICLE_Y_OFFSET,
        state.position.z + f64::from(state.width) * (2.0 * expected_random.next_f64() - 1.0),
    ];
    let mut resolver = test_resolver(0);

    let batch = resolver.dolphin_happy_particle_batch(state);

    assert_eq!(batch.len(), DOLPHIN_HAPPY_PARTICLE_COUNT);
    assert_particle_command(
        &batch.commands[0],
        HAPPY_VILLAGER_PARTICLE_TYPE_ID,
        "minecraft:happy_villager",
        expected_position,
        expected_velocity,
        false,
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:happy_villager"));
}

#[test]
fn fox_eat_batch_matches_vanilla_item_particles() {
    let state = FoxEatParticleState {
        entity_id: 85,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        y_rot: 45.0,
        x_rot: -30.0,
        item_stack: ItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    };
    let look = fox_look_vector(state.x_rot, state.y_rot);
    let expected_position = [
        state.position.x + look[0] * 0.5,
        state.position.y,
        state.position.z + look[2] * 0.5,
    ];
    let mut expected_random = LegacyRandom::new(0);
    let local_velocity = [
        f64::from((expected_random.next_float() - 0.5) * FOX_EAT_HORIZONTAL_VELOCITY_RANGE),
        f64::from(
            expected_random.next_float() * FOX_EAT_VERTICAL_VELOCITY_RANGE
                + FOX_EAT_VERTICAL_VELOCITY_BASE,
        ),
        0.0,
    ];
    let mut expected_velocity = fox_rotate_velocity(local_velocity, state.x_rot, state.y_rot);
    expected_velocity[1] += FOX_EAT_VERTICAL_VELOCITY_OFFSET;
    let mut resolver = test_resolver(0);

    let batch = resolver.fox_eat_particle_batch(state, None);

    assert_eq!(batch.len(), FOX_EAT_PARTICLE_COUNT);
    assert_particle_command(
        &batch.commands[0],
        ITEM_PARTICLE_TYPE_ID,
        "minecraft:item",
        expected_position,
        expected_velocity,
        false,
    );
    assert_eq!(
        batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 42,
            count: 3,
            component_patch_len: 0,
        })
    );
    assert!(batch
        .commands
        .iter()
        .all(|command| command.particle_id == "minecraft:item"));
}

#[test]
fn honey_block_batch_matches_vanilla_show_particles() {
    let resolver = test_resolver(0);
    let honey_block_state_id = test_block_state_id("minecraft:honey_block", []);
    let state = HoneyBlockParticleState {
        entity_id: 76,
        position: bbb_world::EntityVec3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        count: 10,
        block_state_id: honey_block_state_id,
    };

    let batch = resolver.honey_block_particle_batch(state);

    assert_eq!(batch.len(), 10);
    assert_block_destroy_particle_command(
        &batch.commands[0],
        honey_block_state_id,
        [10.0, 64.0, -3.0],
        [0.0, 0.0, 0.0],
    );
    assert!(batch.commands.iter().all(|command| command.option_block
        == Some(ParticleBlockOptionState {
            block_state_id: honey_block_state_id
        })));
}

#[test]
fn count_zero_emits_single_spawn_with_offset_velocity() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(4, 0));

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.unknown_particle_type_count, 0);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, 4);
    assert_eq!(command.particle_id, "minecraft:cloud");
    assert_eq!(
        command.sprite_ids,
        vec![
            "minecraft:generic_7".to_string(),
            "minecraft:generic_6".to_string(),
        ]
    );
    assert_eq!(command.position, [10.0, 64.5, -3.25]);
    assert_close(command.velocity[0], 0.15);
    assert_close(command.velocity[1], 0.30);
    assert_close(command.velocity[2], 0.45);
    assert!(command.override_limiter);
    assert!(command.always_show);
    assert_eq!(command.raw_options_len, 2);
    assert_eq!(command.initial_delay_ticks, 0);
}

#[test]
fn lava_level_particle_command_carries_smoke_child_template() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(LAVA_PARTICLE_TYPE_ID, 0));

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, LAVA_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:lava");
    assert_eq!(command.child_spawn_templates.len(), 1);
    let child = &command.child_spawn_templates[0];
    assert_eq!(child.particle_type_id, SMOKE_PARTICLE_TYPE_ID);
    assert_eq!(child.particle_id, "minecraft:smoke");
    assert_eq!(child.sprite_ids, vec!["minecraft:smoke_0".to_string()]);
}

#[test]
fn drip_particle_commands_carry_vanilla_child_templates() {
    let mut resolver = test_resolver(0);
    for (particle_type_id, child_particle_type_ids) in [
        (
            DRIPPING_HONEY_PARTICLE_TYPE_ID,
            &[
                FALLING_HONEY_PARTICLE_TYPE_ID,
                LANDING_HONEY_PARTICLE_TYPE_ID,
            ][..],
        ),
        (
            FALLING_HONEY_PARTICLE_TYPE_ID,
            &[LANDING_HONEY_PARTICLE_TYPE_ID][..],
        ),
        (
            DRIPPING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
            &[
                FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
                LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
            ][..],
        ),
        (
            FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
            &[LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID][..],
        ),
        (
            DRIPPING_LAVA_PARTICLE_TYPE_ID,
            &[FALLING_LAVA_PARTICLE_TYPE_ID, LANDING_LAVA_PARTICLE_TYPE_ID][..],
        ),
        (
            FALLING_LAVA_PARTICLE_TYPE_ID,
            &[LANDING_LAVA_PARTICLE_TYPE_ID][..],
        ),
        (
            DRIPPING_WATER_PARTICLE_TYPE_ID,
            &[FALLING_WATER_PARTICLE_TYPE_ID, SPLASH_PARTICLE_TYPE_ID][..],
        ),
        (
            FALLING_WATER_PARTICLE_TYPE_ID,
            &[SPLASH_PARTICLE_TYPE_ID][..],
        ),
        (
            DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
            &[
                FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
                LANDING_LAVA_PARTICLE_TYPE_ID,
            ][..],
        ),
        (
            FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
            &[LANDING_LAVA_PARTICLE_TYPE_ID][..],
        ),
        (
            DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
            &[
                FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
                SPLASH_PARTICLE_TYPE_ID,
            ][..],
        ),
        (
            FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
            &[SPLASH_PARTICLE_TYPE_ID][..],
        ),
    ] {
        let batch = resolver.resolve_level_particles(&level_particles_packet(particle_type_id, 0));

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, particle_type_id);
        assert_eq!(
            command.child_spawn_templates.len(),
            child_particle_type_ids.len()
        );
        for (child, child_particle_type_id) in command
            .child_spawn_templates
            .iter()
            .zip(child_particle_type_ids)
        {
            let child_type = vanilla_particle_type(*child_particle_type_id).unwrap();
            assert_eq!(child.particle_type_id, *child_particle_type_id);
            assert_eq!(child.particle_id, child_type.name);
            assert!(!child.sprite_ids.is_empty(), "{}", child_type.name);
        }
    }
}

#[test]
fn explosion_emitter_particle_commands_carry_explosion_child_template_without_definition() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(
        EXPLOSION_EMITTER_PARTICLE_TYPE_ID,
        0,
    ));

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.missing_definition_count, 0);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, EXPLOSION_EMITTER_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:explosion_emitter");
    assert!(command.sprite_ids.is_empty());
    assert_eq!(command.child_spawn_templates.len(), 1);
    let child = &command.child_spawn_templates[0];
    assert_eq!(child.particle_type_id, EXPLOSION_PARTICLE_TYPE_ID);
    assert_eq!(child.particle_id, "minecraft:explosion");
    assert_eq!(child.sprite_ids, vec!["minecraft:explosion_0".to_string()]);
}

#[test]
fn elder_guardian_particle_command_is_definitionless_special_group_input() {
    let mut resolver = test_resolver(0);
    let batch = resolver
        .resolve_level_particles(&level_particles_packet(ELDER_GUARDIAN_PARTICLE_TYPE_ID, 0));

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.missing_definition_count, 0);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, ELDER_GUARDIAN_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:elder_guardian");
    assert!(command.sprite_ids.is_empty());
    assert!(command.child_spawn_templates.is_empty());
}

#[test]
fn terrain_and_item_atlas_particles_are_definitionless_submission_inputs() {
    let mut resolver = test_resolver(0);
    for (particle_type_id, particle_id, raw_options, block_state_id, item) in [
        (
            BLOCK_PARTICLE_TYPE_ID,
            "minecraft:block",
            block_particle_options(129),
            Some(129),
            None,
        ),
        (
            BLOCK_MARKER_PARTICLE_TYPE_ID,
            "minecraft:block_marker",
            block_particle_options(2),
            Some(2),
            None,
        ),
        (
            DUST_PILLAR_PARTICLE_TYPE_ID,
            "minecraft:dust_pillar",
            block_particle_options(3),
            Some(3),
            None,
        ),
        (
            BLOCK_CRUMBLE_PARTICLE_TYPE_ID,
            "minecraft:block_crumble",
            block_particle_options(4),
            Some(4),
            None,
        ),
        (
            ITEM_PARTICLE_TYPE_ID,
            "minecraft:item",
            item_particle_options(5, 6, 0),
            None,
            Some(ParticleItemOptionState {
                item_id: 5,
                count: 6,
                component_patch_len: 2,
            }),
        ),
        (
            ITEM_SLIME_PARTICLE_TYPE_ID,
            "minecraft:item_slime",
            Vec::new(),
            None,
            None,
        ),
        (
            ITEM_COBWEB_PARTICLE_TYPE_ID,
            "minecraft:item_cobweb",
            Vec::new(),
            None,
            None,
        ),
        (
            ITEM_SNOWBALL_PARTICLE_TYPE_ID,
            "minecraft:item_snowball",
            Vec::new(),
            None,
            None,
        ),
    ] {
        let mut packet = level_particles_packet(particle_type_id, 0);
        packet.particle.raw_options = raw_options.clone();
        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{particle_id}");
        assert_eq!(batch.missing_definition_count, 0, "{particle_id}");
        assert_eq!(batch.unknown_particle_type_count, 0, "{particle_id}");
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, particle_type_id, "{particle_id}");
        assert_eq!(command.particle_id, particle_id, "{particle_id}");
        assert!(command.sprite_ids.is_empty(), "{particle_id}");
        assert_eq!(command.raw_options_len, raw_options.len(), "{particle_id}");
        assert_eq!(
            command.option_block.map(|option| option.block_state_id),
            block_state_id,
            "{particle_id}"
        );
        assert_eq!(command.option_item, item, "{particle_id}");
    }
}

#[test]
fn terrain_particle_commands_use_installed_block_sprite_ids() {
    let mut resolver = test_resolver(0);
    let stone_id = test_block_state_id("minecraft:stone", []);
    resolver
        .terrain_particle_sprite_ids
        .insert(stone_id, "minecraft:block/stone".to_string());

    for (particle_type_id, particle_id) in [
        (BLOCK_PARTICLE_TYPE_ID, "minecraft:block"),
        (BLOCK_MARKER_PARTICLE_TYPE_ID, "minecraft:block_marker"),
        (DUST_PILLAR_PARTICLE_TYPE_ID, "minecraft:dust_pillar"),
        (BLOCK_CRUMBLE_PARTICLE_TYPE_ID, "minecraft:block_crumble"),
    ] {
        let mut packet = level_particles_packet(particle_type_id, 0);
        packet.particle.raw_options = block_particle_options(stone_id);
        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{particle_id}");
        assert_eq!(
            batch.commands[0].sprite_ids,
            vec!["minecraft:block/stone".to_string()],
            "{particle_id}"
        );
    }
}

#[test]
fn terrain_particle_commands_use_installed_block_tint_colors() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());
    let redstone_id = test_block_state_id(
        "minecraft:redstone_wire",
        [
            ("east", "up"),
            ("north", "up"),
            ("power", "15"),
            ("south", "up"),
            ("west", "up"),
        ],
    );

    for (particle_type_id, particle_id) in [
        (BLOCK_PARTICLE_TYPE_ID, "minecraft:block"),
        (DUST_PILLAR_PARTICLE_TYPE_ID, "minecraft:dust_pillar"),
        (BLOCK_CRUMBLE_PARTICLE_TYPE_ID, "minecraft:block_crumble"),
    ] {
        let mut packet = level_particles_packet(particle_type_id, 0);
        packet.particle.raw_options = block_particle_options(redstone_id);
        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{particle_id}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option_06(255, 50, 0)),
            "{particle_id}"
        );
    }

    let mut marker = level_particles_packet(BLOCK_MARKER_PARTICLE_TYPE_ID, 0);
    marker.particle.raw_options = block_particle_options(redstone_id);
    let marker_batch = resolver.resolve_level_particles(&marker);
    assert_eq!(marker_batch.len(), 1);
    assert_eq!(marker_batch.commands[0].option_color, None);
}

#[test]
fn terrain_particle_tint_samples_biome_at_each_spawn_position() {
    let mut resolver = test_resolver(0);
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        test_biome_color_profile(7, [10, 20, 30], [40, 50, 60], [70, 80, 90], [1, 2, 3]),
        test_biome_color_profile(
            11,
            [100, 110, 120],
            [130, 140, 150],
            [160, 170, 180],
            [4, 5, 6],
        ),
    ]));
    resolver.set_terrain_particle_sprite_ids(&textures);
    let short_grass_id = test_block_state_id("minecraft:short_grass", []);
    let sampler = SplitXBiomeSampler {
        split_x: 0,
        left_biome_id: 7,
        right_biome_id: 11,
    };
    let mut packet = level_particles_packet(BLOCK_PARTICLE_TYPE_ID, 4);
    packet.position.x = 0.0;
    packet.offset.x = 8.0;
    packet.offset.y = 0.0;
    packet.offset.z = 0.0;
    packet.max_speed = 0.0;
    packet.particle.raw_options = block_particle_options(short_grass_id);

    let batch = resolver.resolve_level_particles_with_context(
        &packet,
        LevelParticleSpawnContext::default(),
        Some(&sampler),
        None,
    );

    assert_eq!(batch.len(), 4);
    let mut saw_left = false;
    let mut saw_right = false;
    for command in &batch.commands {
        let block_pos = block_pos_containing(Vec3d {
            x: command.position[0],
            y: command.position[1],
            z: command.position[2],
        });
        let expected_color = if block_pos.x < 0 {
            saw_left = true;
            rgb_option_06(10, 20, 30)
        } else {
            saw_right = true;
            rgb_option_06(100, 110, 120)
        };
        assert_eq!(command.option_color, Some(expected_color));
    }
    assert!(saw_left);
    assert!(saw_right);
}

#[test]
fn falling_dust_tint_samples_biome_foliage_color_at_spawn_position() {
    let mut resolver = test_resolver(0);
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        test_biome_color_profile(3, [10, 20, 30], [40, 50, 60], [70, 80, 90], [4, 5, 6]),
    ]));
    resolver.set_terrain_particle_sprite_ids(&textures);
    let oak_leaves_id = test_block_state_id(
        "minecraft:oak_leaves",
        [
            ("distance", "1"),
            ("persistent", "false"),
            ("waterlogged", "false"),
        ],
    );
    let sampler = SplitXBiomeSampler {
        split_x: 0,
        left_biome_id: 3,
        right_biome_id: 3,
    };
    let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
    packet.position.x = -0.25;
    packet.particle.raw_options = block_particle_options(oak_leaves_id);

    let batch = resolver.resolve_level_particles_with_context(
        &packet,
        LevelParticleSpawnContext::default(),
        Some(&sampler),
        None,
    );

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.commands[0].option_color, Some(rgb_option(40, 50, 60)));
}

#[test]
fn generic_item_particle_uses_installed_default_item_sprite_for_empty_component_patch() {
    let mut resolver = test_resolver(0);
    resolver.default_item_particle_sprite_ids.insert(
        5,
        vec![
            "minecraft:item/apple".to_string(),
            "minecraft:item/apple_overlay".to_string(),
        ],
    );

    let mut empty_patch = level_particles_packet(ITEM_PARTICLE_TYPE_ID, 0);
    empty_patch.particle.raw_options = item_particle_options(5, 6, 0);
    let empty_batch = resolver.resolve_level_particles(&empty_patch);

    assert_eq!(empty_batch.len(), 1);
    assert_eq!(
        empty_batch.commands[0].sprite_ids,
        vec![
            "minecraft:item/apple".to_string(),
            "minecraft:item/apple_overlay".to_string()
        ]
    );
    assert_eq!(
        empty_batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 5,
            count: 6,
            component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
        })
    );

    let mut non_empty_patch = level_particles_packet(ITEM_PARTICLE_TYPE_ID, 0);
    non_empty_patch.particle.raw_options = item_particle_options(5, 6, 1);
    let non_empty_component_patch_len = non_empty_patch.particle.raw_options.len()
        - positive_var_i32_len(5)
        - positive_var_i32_len(6);
    let non_empty_batch = resolver.resolve_level_particles(&non_empty_patch);

    assert_eq!(non_empty_batch.len(), 1);
    assert!(non_empty_batch.commands[0].sprite_ids.is_empty());
    assert_eq!(
        non_empty_batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 5,
            count: 6,
            component_patch_len: non_empty_component_patch_len,
        })
    );
}

#[test]
fn generic_item_particle_uses_item_runtime_for_component_stack_sprite() {
    let root = unique_temp_dir("particle-item-component-sprite");
    write_item_particle_item_model_fixture(&root);
    let items = NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(ITEM_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = item_particle_options(0, 1, 1);
    let component_patch_len =
        packet.particle.raw_options.len() - positive_var_i32_len(0) - positive_var_i32_len(1);

    let batch = resolver.resolve_level_particles_with_context(
        &packet,
        LevelParticleSpawnContext::default(),
        None,
        Some(&items),
    );

    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch.commands[0].sprite_ids,
        vec!["minecraft:item/alternate_model_component".to_string()]
    );
    assert_eq!(
        batch.commands[0].option_item,
        Some(ParticleItemOptionState {
            item_id: 0,
            count: 1,
            component_patch_len,
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn terrain_particle_providers_reject_vanilla_filtered_block_states() {
    let air_id = test_block_state_id("minecraft:air", []);
    let moving_piston_id = test_block_state_id(
        "minecraft:moving_piston",
        [("facing", "north"), ("type", "normal")],
    );
    let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
    let structure_void_id = test_block_state_id("minecraft:structure_void", []);
    let stone_id = test_block_state_id("minecraft:stone", []);

    for particle_type_id in [
        BLOCK_PARTICLE_TYPE_ID,
        DUST_PILLAR_PARTICLE_TYPE_ID,
        BLOCK_CRUMBLE_PARTICLE_TYPE_ID,
    ] {
        for block_state_id in [air_id, moving_piston_id, barrier_id, structure_void_id] {
            let mut resolver = test_resolver(0);
            let mut packet = level_particles_packet(particle_type_id, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 0, "{particle_type_id} {block_state_id}");
            assert_eq!(batch.missing_definition_count, 0);
            assert_eq!(batch.unknown_particle_type_count, 0);
        }

        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(particle_type_id, 0);
        packet.particle.raw_options = block_particle_options(stone_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{particle_type_id}");
        assert_eq!(
            batch.commands[0].option_block,
            Some(ParticleBlockOptionState {
                block_state_id: stone_id
            })
        );
    }
}

#[test]
fn block_marker_provider_keeps_invisible_and_no_terrain_particle_states() {
    let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(BLOCK_MARKER_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = block_particle_options(barrier_id);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.commands[0].particle_id, "minecraft:block_marker");
    assert_eq!(
        batch.commands[0].option_block,
        Some(ParticleBlockOptionState {
            block_state_id: barrier_id
        })
    );
}

#[test]
fn terrain_particle_provider_rejection_preserves_packet_random_sequence() {
    let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
    let stone_id = test_block_state_id("minecraft:stone", []);
    let mut rejected_resolver = test_resolver(42);
    let mut accepted_resolver = test_resolver(42);
    let mut rejected = level_particles_packet(BLOCK_PARTICLE_TYPE_ID, 2);
    rejected.particle.raw_options = block_particle_options(barrier_id);
    let mut accepted = level_particles_packet(BLOCK_PARTICLE_TYPE_ID, 2);
    accepted.particle.raw_options = block_particle_options(stone_id);

    let rejected_batch = rejected_resolver.resolve_level_particles(&rejected);
    let accepted_batch = accepted_resolver.resolve_level_particles(&accepted);
    assert_eq!(rejected_batch.len(), 0);
    assert_eq!(accepted_batch.len(), 2);

    let next_rejected = rejected_resolver
        .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));
    let next_accepted = accepted_resolver
        .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));

    assert_eq!(next_rejected.len(), 1);
    assert_eq!(next_accepted.len(), 1);
    assert_eq!(
        next_rejected.commands[0].position,
        next_accepted.commands[0].position
    );
    assert_eq!(
        next_rejected.commands[0].velocity,
        next_accepted.commands[0].velocity
    );
}

#[test]
fn falling_dust_decodes_block_particle_option_metadata() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = block_particle_options(321);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.missing_definition_count, 0);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, FALLING_DUST_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:falling_dust");
    assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
    assert_eq!(
        command.option_block,
        Some(ParticleBlockOptionState {
            block_state_id: 321
        })
    );
    assert_eq!(command.option_item, None);
    assert_eq!(command.option_color, None);
    assert_eq!(command.raw_options_len, block_particle_options(321).len());
}

#[test]
fn falling_dust_decodes_falling_block_dust_colors() {
    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:sand", []),
            "minecraft:sand",
            rgb_option(0xDB, 0xD3, 0xA0),
        ),
        (
            test_block_state_id("minecraft:red_sand", []),
            "minecraft:red_sand",
            rgb_option(0xA9, 0x58, 0x21),
        ),
        (
            test_block_state_id("minecraft:gravel", []),
            "minecraft:gravel",
            rgb_option(0x80, 0x7C, 0x7B),
        ),
        (
            test_block_state_id("minecraft:dragon_egg", []),
            "minecraft:dragon_egg",
            rgb_option(0x00, 0x00, 0x00),
        ),
        (
            test_block_state_id("minecraft:anvil", [("facing", "north")]),
            "minecraft:anvil",
            rgb_option(0xA7, 0xA7, 0xA7),
        ),
        (
            test_block_state_id("minecraft:red_concrete_powder", []),
            "minecraft:red_concrete_powder",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:black_concrete_powder", []),
            "minecraft:black_concrete_powder",
            rgb_option(0x19, 0x19, 0x19),
        ),
    ] {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_block,
            Some(ParticleBlockOptionState { block_state_id }),
            "{block_name}"
        );
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_installed_block_tint_for_non_falling_block_colors() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());
    let lily_pad_id = test_block_state_id("minecraft:lily_pad", []);
    let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = block_particle_options(lily_pad_id);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch.commands[0].option_color,
        Some(rgb_option(0x20, 0x80, 0x30))
    );
}

#[test]
fn falling_dust_uses_map_color_fallback_for_non_tinted_blocks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:stone", []),
            "minecraft:stone",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:dirt", []),
            "minecraft:dirt",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id("minecraft:oak_planks", []),
            "minecraft:oak_planks",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:birch_planks", []),
            "minecraft:birch_planks",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:acacia_planks", []),
            "minecraft:acacia_planks",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:cherry_planks", []),
            "minecraft:cherry_planks",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            test_block_state_id("minecraft:dark_oak_planks", []),
            "minecraft:dark_oak_planks",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id("minecraft:bamboo_mosaic", []),
            "minecraft:bamboo_mosaic",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id("minecraft:oak_log", [("axis", "y")]),
            "minecraft:oak_log axis=y",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:oak_log", [("axis", "x")]),
            "minecraft:oak_log axis=x",
            rgb_option(0x81, 0x56, 0x31),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_misc_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:redstone_block", []),
            "minecraft:redstone_block",
            rgb_option(0xff, 0x00, 0x00),
        ),
        (
            test_block_state_id("minecraft:slime_block", []),
            "minecraft:slime_block",
            rgb_option(0x7f, 0xb2, 0x38),
        ),
        (
            test_block_state_id(
                "minecraft:petrified_oak_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:petrified_oak_slab",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:dirt_path", []),
            "minecraft:dirt_path",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id("minecraft:frosted_ice", [("age", "0")]),
            "minecraft:frosted_ice",
            rgb_option(0xa0, 0xa0, 0xff),
        ),
        (
            test_block_state_id("minecraft:bone_block", [("axis", "x")]),
            "minecraft:bone_block",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_wood_log_and_stem_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:spruce_log", [("axis", "y")]),
            "minecraft:spruce_log axis=y",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            test_block_state_id("minecraft:spruce_log", [("axis", "x")]),
            "minecraft:spruce_log axis=x",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id("minecraft:birch_log", [("axis", "y")]),
            "minecraft:birch_log axis=y",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:birch_log", [("axis", "z")]),
            "minecraft:birch_log axis=z",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:acacia_log", [("axis", "y")]),
            "minecraft:acacia_log axis=y",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:acacia_log", [("axis", "x")]),
            "minecraft:acacia_log axis=x",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:cherry_log", [("axis", "y")]),
            "minecraft:cherry_log axis=y",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            test_block_state_id("minecraft:cherry_log", [("axis", "x")]),
            "minecraft:cherry_log axis=x",
            rgb_option(0x39, 0x29, 0x23),
        ),
        (
            test_block_state_id("minecraft:pale_oak_log", [("axis", "y")]),
            "minecraft:pale_oak_log axis=y",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:pale_oak_log", [("axis", "z")]),
            "minecraft:pale_oak_log axis=z",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:mangrove_log", [("axis", "y")]),
            "minecraft:mangrove_log axis=y",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:mangrove_log", [("axis", "x")]),
            "minecraft:mangrove_log axis=x",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            test_block_state_id("minecraft:bamboo_block", [("axis", "y")]),
            "minecraft:bamboo_block axis=y",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id("minecraft:bamboo_block", [("axis", "x")]),
            "minecraft:bamboo_block axis=x",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:acacia_wood", [("axis", "z")]),
            "minecraft:acacia_wood",
            rgb_option(0x4c, 0x4c, 0x4c),
        ),
        (
            test_block_state_id("minecraft:stripped_cherry_log", [("axis", "x")]),
            "minecraft:stripped_cherry_log axis=x",
            rgb_option(0xa0, 0x4d, 0x4e),
        ),
        (
            test_block_state_id("minecraft:stripped_cherry_wood", [("axis", "y")]),
            "minecraft:stripped_cherry_wood",
            rgb_option(0xa0, 0x4d, 0x4e),
        ),
        (
            test_block_state_id("minecraft:crimson_planks", []),
            "minecraft:crimson_planks",
            rgb_option(0x94, 0x3f, 0x61),
        ),
        (
            test_block_state_id("minecraft:crimson_hyphae", [("axis", "x")]),
            "minecraft:crimson_hyphae",
            rgb_option(0x5c, 0x19, 0x1d),
        ),
        (
            test_block_state_id("minecraft:warped_stem", [("axis", "z")]),
            "minecraft:warped_stem",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
        (
            test_block_state_id("minecraft:warped_hyphae", [("axis", "y")]),
            "minecraft:warped_hyphae",
            rgb_option(0x56, 0x2c, 0x3e),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_wooden_stairs_and_slabs_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:oak_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:oak_stairs",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:spruce_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:spruce_slab",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            test_block_state_id(
                "minecraft:birch_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:birch_stairs",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id(
                "minecraft:jungle_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:jungle_slab",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id(
                "minecraft:acacia_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:acacia_stairs",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:cherry_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:cherry_slab",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            test_block_state_id(
                "minecraft:dark_oak_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:dark_oak_stairs",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:pale_oak_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:pale_oak_slab",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id(
                "minecraft:mangrove_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:mangrove_stairs",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:bamboo_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:bamboo_slab",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:bamboo_mosaic_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:bamboo_mosaic_stairs",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:crimson_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "straight"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:crimson_stairs",
            rgb_option(0x94, 0x3f, 0x61),
        ),
        (
            test_block_state_id(
                "minecraft:warped_slab",
                [("type", "top"), ("waterlogged", "true")],
            ),
            "minecraft:warped_slab",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_wooden_pressure_plate_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_name, expected_color) in [
        ("minecraft:oak_pressure_plate", rgb_option(0x8f, 0x77, 0x48)),
        (
            "minecraft:spruce_pressure_plate",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            "minecraft:birch_pressure_plate",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            "minecraft:jungle_pressure_plate",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            "minecraft:acacia_pressure_plate",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            "minecraft:cherry_pressure_plate",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            "minecraft:dark_oak_pressure_plate",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            "minecraft:pale_oak_pressure_plate",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            "minecraft:mangrove_pressure_plate",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            "minecraft:bamboo_pressure_plate",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            "minecraft:crimson_pressure_plate",
            rgb_option(0x94, 0x3f, 0x61),
        ),
        (
            "minecraft:warped_pressure_plate",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
    ] {
        let block_state_id = test_block_state_id(block_name, [("powered", "true")]);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_wooden_door_trapdoor_fence_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (family, expected_color) in [
        ("oak", rgb_option(0x8f, 0x77, 0x48)),
        ("spruce", rgb_option(0x81, 0x56, 0x31)),
        ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
        ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
        ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
        ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
        ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
        ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
        ("mangrove", rgb_option(0x99, 0x33, 0x33)),
        ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
        ("crimson", rgb_option(0x94, 0x3f, 0x61)),
        ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
    ] {
        for kind in ["door", "trapdoor", "fence", "fence_gate"] {
            let block_name = format!("minecraft:{family}_{kind}");
            let block_state_id = match kind {
                "door" => test_block_state_id(
                    &block_name,
                    [
                        ("facing", "north"),
                        ("half", "upper"),
                        ("hinge", "left"),
                        ("open", "true"),
                        ("powered", "true"),
                    ],
                ),
                "trapdoor" => test_block_state_id(
                    &block_name,
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("open", "true"),
                        ("powered", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "fence" => test_block_state_id(
                    &block_name,
                    [
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("waterlogged", "true"),
                        ("west", "true"),
                    ],
                ),
                "fence_gate" => test_block_state_id(
                    &block_name,
                    [
                        ("facing", "north"),
                        ("in_wall", "true"),
                        ("open", "true"),
                        ("powered", "true"),
                    ],
                ),
                _ => unreachable!("covered test kinds"),
            };
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }
}

#[test]
fn falling_dust_uses_wooden_sign_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (family, expected_color) in [
        ("oak", rgb_option(0x8f, 0x77, 0x48)),
        ("spruce", rgb_option(0x81, 0x56, 0x31)),
        ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
        ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
        ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
        ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
        ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
        ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
        ("mangrove", rgb_option(0x99, 0x33, 0x33)),
        ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
        ("crimson", rgb_option(0x94, 0x3f, 0x61)),
        ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
    ] {
        for kind in ["sign", "wall_sign"] {
            let block_name = format!("minecraft:{family}_{kind}");
            let block_state_id = match kind {
                "sign" => {
                    test_block_state_id(&block_name, [("rotation", "0"), ("waterlogged", "true")])
                }
                "wall_sign" => {
                    test_block_state_id(&block_name, [("facing", "north"), ("waterlogged", "true")])
                }
                _ => unreachable!("covered test kinds"),
            };
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }
}

#[test]
fn falling_dust_uses_wooden_shelf_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (family, expected_color) in [
        ("oak", rgb_option(0x8f, 0x77, 0x48)),
        ("spruce", rgb_option(0x81, 0x56, 0x31)),
        ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
        ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
        ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
        ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
        ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
        ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
        ("mangrove", rgb_option(0x99, 0x33, 0x33)),
        ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
        ("crimson", rgb_option(0x94, 0x3f, 0x61)),
        ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
    ] {
        let block_name = format!("minecraft:{family}_shelf");
        let block_state_id = test_block_state_id(
            &block_name,
            [
                ("facing", "north"),
                ("powered", "true"),
                ("side_chain", "unconnected"),
                ("waterlogged", "true"),
            ],
        );
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_hanging_sign_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (family, ceiling_color, wall_color) in [
        (
            "oak",
            rgb_option(0x8f, 0x77, 0x48),
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            "spruce",
            rgb_option(0x81, 0x56, 0x31),
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            "birch",
            rgb_option(0xf7, 0xe9, 0xa3),
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            "jungle",
            rgb_option(0x97, 0x6d, 0x4d),
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            "acacia",
            rgb_option(0xd8, 0x7f, 0x33),
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            "cherry",
            rgb_option(0xa0, 0x4d, 0x4e),
            rgb_option(0xa0, 0x4d, 0x4e),
        ),
        (
            "dark_oak",
            rgb_option(0x66, 0x4c, 0x33),
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            "pale_oak",
            rgb_option(0xff, 0xfc, 0xf5),
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            "mangrove",
            rgb_option(0x99, 0x33, 0x33),
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            "bamboo",
            rgb_option(0xe5, 0xe5, 0x33),
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            "crimson",
            rgb_option(0x94, 0x3f, 0x61),
            rgb_option(0x94, 0x3f, 0x61),
        ),
        (
            "warped",
            rgb_option(0x3a, 0x8e, 0x8c),
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
    ] {
        for (kind, expected_color) in [
            ("hanging_sign", ceiling_color),
            ("wall_hanging_sign", wall_color),
        ] {
            let block_name = format!("minecraft:{family}_{kind}");
            let block_state_id = match kind {
                "hanging_sign" => test_block_state_id(
                    &block_name,
                    [
                        ("attached", "true"),
                        ("rotation", "0"),
                        ("waterlogged", "true"),
                    ],
                ),
                "wall_hanging_sign" => {
                    test_block_state_id(&block_name, [("facing", "north"), ("waterlogged", "true")])
                }
                _ => unreachable!("covered test kinds"),
            };
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }
}

#[test]
fn falling_dust_uses_button_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for block_name in [
        "minecraft:stone_button",
        "minecraft:oak_button",
        "minecraft:spruce_button",
        "minecraft:birch_button",
        "minecraft:jungle_button",
        "minecraft:acacia_button",
        "minecraft:cherry_button",
        "minecraft:dark_oak_button",
        "minecraft:pale_oak_button",
        "minecraft:mangrove_button",
        "minecraft:bamboo_button",
        "minecraft:crimson_button",
        "minecraft:warped_button",
        "minecraft:polished_blackstone_button",
    ] {
        let block_state_id = test_block_state_id(
            block_name,
            [("face", "floor"), ("facing", "north"), ("powered", "true")],
        );
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_potted_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for block_name in [
        "minecraft:flower_pot",
        "minecraft:potted_torchflower",
        "minecraft:potted_oak_sapling",
        "minecraft:potted_spruce_sapling",
        "minecraft:potted_birch_sapling",
        "minecraft:potted_jungle_sapling",
        "minecraft:potted_acacia_sapling",
        "minecraft:potted_cherry_sapling",
        "minecraft:potted_dark_oak_sapling",
        "minecraft:potted_pale_oak_sapling",
        "minecraft:potted_mangrove_propagule",
        "minecraft:potted_dandelion",
        "minecraft:potted_golden_dandelion",
        "minecraft:potted_poppy",
        "minecraft:potted_blue_orchid",
        "minecraft:potted_allium",
        "minecraft:potted_azure_bluet",
        "minecraft:potted_red_tulip",
        "minecraft:potted_orange_tulip",
        "minecraft:potted_white_tulip",
        "minecraft:potted_pink_tulip",
        "minecraft:potted_oxeye_daisy",
        "minecraft:potted_cornflower",
        "minecraft:potted_lily_of_the_valley",
        "minecraft:potted_wither_rose",
        "minecraft:potted_red_mushroom",
        "minecraft:potted_brown_mushroom",
        "minecraft:potted_dead_bush",
        "minecraft:potted_cactus",
        "minecraft:potted_bamboo",
        "minecraft:potted_crimson_fungus",
        "minecraft:potted_warped_fungus",
        "minecraft:potted_crimson_roots",
        "minecraft:potted_warped_roots",
        "minecraft:potted_azalea_bush",
        "minecraft:potted_flowering_azalea_bush",
        "minecraft:potted_open_eyeblossom",
        "minecraft:potted_closed_eyeblossom",
    ] {
        let block_state_id = test_block_state_id(block_name, []);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_cake_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (
            test_block_state_id("minecraft:cake", [("bites", "6")]),
            "minecraft:cake",
        ),
        (
            test_block_state_id("minecraft:candle_cake", [("lit", "true")]),
            "minecraft:candle_cake",
        ),
        (
            test_block_state_id("minecraft:white_candle_cake", [("lit", "false")]),
            "minecraft:white_candle_cake",
        ),
        (
            test_block_state_id("minecraft:orange_candle_cake", [("lit", "true")]),
            "minecraft:orange_candle_cake",
        ),
        (
            test_block_state_id("minecraft:magenta_candle_cake", [("lit", "false")]),
            "minecraft:magenta_candle_cake",
        ),
        (
            test_block_state_id("minecraft:light_blue_candle_cake", [("lit", "true")]),
            "minecraft:light_blue_candle_cake",
        ),
        (
            test_block_state_id("minecraft:yellow_candle_cake", [("lit", "false")]),
            "minecraft:yellow_candle_cake",
        ),
        (
            test_block_state_id("minecraft:lime_candle_cake", [("lit", "true")]),
            "minecraft:lime_candle_cake",
        ),
        (
            test_block_state_id("minecraft:pink_candle_cake", [("lit", "false")]),
            "minecraft:pink_candle_cake",
        ),
        (
            test_block_state_id("minecraft:gray_candle_cake", [("lit", "true")]),
            "minecraft:gray_candle_cake",
        ),
        (
            test_block_state_id("minecraft:light_gray_candle_cake", [("lit", "false")]),
            "minecraft:light_gray_candle_cake",
        ),
        (
            test_block_state_id("minecraft:cyan_candle_cake", [("lit", "true")]),
            "minecraft:cyan_candle_cake",
        ),
        (
            test_block_state_id("minecraft:purple_candle_cake", [("lit", "false")]),
            "minecraft:purple_candle_cake",
        ),
        (
            test_block_state_id("minecraft:blue_candle_cake", [("lit", "true")]),
            "minecraft:blue_candle_cake",
        ),
        (
            test_block_state_id("minecraft:brown_candle_cake", [("lit", "false")]),
            "minecraft:brown_candle_cake",
        ),
        (
            test_block_state_id("minecraft:green_candle_cake", [("lit", "true")]),
            "minecraft:green_candle_cake",
        ),
        (
            test_block_state_id("minecraft:red_candle_cake", [("lit", "false")]),
            "minecraft:red_candle_cake",
        ),
        (
            test_block_state_id("minecraft:black_candle_cake", [("lit", "true")]),
            "minecraft:black_candle_cake",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_colored_family_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:white_wool", []),
            "minecraft:white_wool",
            rgb_option(0xff, 0xff, 0xff),
        ),
        (
            test_block_state_id("minecraft:lime_wool", []),
            "minecraft:lime_wool",
            rgb_option(0x7f, 0xcc, 0x19),
        ),
        (
            test_block_state_id("minecraft:blue_carpet", []),
            "minecraft:blue_carpet",
            rgb_option(0x33, 0x4c, 0xb2),
        ),
        (
            test_block_state_id("minecraft:cyan_stained_glass", []),
            "minecraft:cyan_stained_glass",
            rgb_option(0x4c, 0x7f, 0x99),
        ),
        (
            test_block_state_id("minecraft:purple_glazed_terracotta", [("facing", "north")]),
            "minecraft:purple_glazed_terracotta",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id("minecraft:orange_concrete", []),
            "minecraft:orange_concrete",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:black_concrete", []),
            "minecraft:black_concrete",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id("minecraft:terracotta", []),
            "minecraft:terracotta",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:white_terracotta", []),
            "minecraft:white_terracotta",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            test_block_state_id("minecraft:light_blue_terracotta", []),
            "minecraft:light_blue_terracotta",
            rgb_option(0x70, 0x6c, 0x8a),
        ),
        (
            test_block_state_id("minecraft:red_terracotta", []),
            "minecraft:red_terracotta",
            rgb_option(0x8e, 0x3c, 0x2e),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_banner_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for color in [
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ] {
        for kind in ["banner", "wall_banner"] {
            let block_name = format!("minecraft:{color}_{kind}");
            let block_state_id = match kind {
                "banner" => test_block_state_id(&block_name, [("rotation", "0")]),
                "wall_banner" => test_block_state_id(&block_name, [("facing", "north")]),
                _ => unreachable!("covered test kinds"),
            };
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x8f, 0x77, 0x48)),
                "{block_name}"
            );
        }
    }
}

#[test]
fn falling_dust_uses_mineral_and_natural_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:gold_ore", []),
            "minecraft:gold_ore",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:deepslate_iron_ore", []),
            "minecraft:deepslate_iron_ore",
            rgb_option(0x64, 0x64, 0x64),
        ),
        (
            test_block_state_id("minecraft:nether_quartz_ore", []),
            "minecraft:nether_quartz_ore",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:lapis_block", []),
            "minecraft:lapis_block",
            rgb_option(0x4a, 0x80, 0xff),
        ),
        (
            test_block_state_id("minecraft:diamond_block", []),
            "minecraft:diamond_block",
            rgb_option(0x5c, 0xdb, 0xd5),
        ),
        (
            test_block_state_id("minecraft:emerald_block", []),
            "minecraft:emerald_block",
            rgb_option(0x00, 0xd9, 0x3a),
        ),
        (
            test_block_state_id("minecraft:raw_iron_block", []),
            "minecraft:raw_iron_block",
            rgb_option(0xd8, 0xaf, 0x93),
        ),
        (
            test_block_state_id("minecraft:suspicious_gravel", [("dusted", "0")]),
            "minecraft:suspicious_gravel",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:suspicious_sand", [("dusted", "0")]),
            "minecraft:suspicious_sand",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:sandstone", []),
            "minecraft:sandstone",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:snow_block", []),
            "minecraft:snow_block",
            rgb_option(0xff, 0xff, 0xff),
        ),
        (
            test_block_state_id("minecraft:ice", []),
            "minecraft:ice",
            rgb_option(0xa0, 0xa0, 0xff),
        ),
        (
            test_block_state_id("minecraft:clay", []),
            "minecraft:clay",
            rgb_option(0xa4, 0xa8, 0xb8),
        ),
        (
            test_block_state_id("minecraft:deepslate", [("axis", "y")]),
            "minecraft:deepslate",
            rgb_option(0x64, 0x64, 0x64),
        ),
        (
            test_block_state_id("minecraft:netherrack", []),
            "minecraft:netherrack",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:red_nether_bricks", []),
            "minecraft:red_nether_bricks",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:soul_sand", []),
            "minecraft:soul_sand",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id("minecraft:basalt", [("axis", "z")]),
            "minecraft:basalt",
            rgb_option(0x19, 0x19, 0x19),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_deepslate_construction_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    let wall = [
        ("east", "low"),
        ("north", "none"),
        ("south", "none"),
        ("up", "true"),
        ("waterlogged", "false"),
        ("west", "none"),
    ];

    for (block_state_id, block_name) in [
        (
            test_block_state_id(
                "minecraft:cobbled_deepslate_stairs",
                [
                    ("facing", "east"),
                    ("half", "bottom"),
                    ("shape", "straight"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:cobbled_deepslate_stairs",
        ),
        (
            test_block_state_id(
                "minecraft:polished_deepslate_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:polished_deepslate_slab",
        ),
        (
            test_block_state_id("minecraft:deepslate_tile_wall", wall),
            "minecraft:deepslate_tile_wall",
        ),
        (
            test_block_state_id(
                "minecraft:deepslate_brick_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "inner_left"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:deepslate_brick_stairs",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x64, 0x64, 0x64)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_infested_stone_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:infested_stone", []),
            "minecraft:infested_stone",
            rgb_option(0xa4, 0xa8, 0xb8),
        ),
        (
            test_block_state_id("minecraft:infested_cobblestone", []),
            "minecraft:infested_cobblestone",
            rgb_option(0xa4, 0xa8, 0xb8),
        ),
        (
            test_block_state_id("minecraft:infested_chiseled_stone_bricks", []),
            "minecraft:infested_chiseled_stone_bricks",
            rgb_option(0xa4, 0xa8, 0xb8),
        ),
        (
            test_block_state_id("minecraft:infested_deepslate", [("axis", "y")]),
            "minecraft:infested_deepslate",
            rgb_option(0x64, 0x64, 0x64),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_natural_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:oak_sapling", [("stage", "0")]),
            "minecraft:oak_sapling",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:cherry_sapling", [("stage", "1")]),
            "minecraft:cherry_sapling",
            rgb_option(0xf2, 0x7f, 0xa5),
        ),
        (
            test_block_state_id("minecraft:pale_oak_sapling", [("stage", "0")]),
            "minecraft:pale_oak_sapling",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:short_dry_grass", []),
            "minecraft:short_dry_grass",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:pointed_dripstone",
                [
                    ("thickness", "tip"),
                    ("vertical_direction", "down"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:pointed_dripstone",
            rgb_option(0x4c, 0x32, 0x23),
        ),
        (
            test_block_state_id("minecraft:cave_vines", [("age", "25"), ("berries", "true")]),
            "minecraft:cave_vines",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:moss_block", []),
            "minecraft:moss_block",
            rgb_option(0x66, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:hanging_roots", [("waterlogged", "false")]),
            "minecraft:hanging_roots",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id("minecraft:mud", []),
            "minecraft:mud",
            rgb_option(0x57, 0x5c, 0x5c),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_crop_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:wheat", [("age", "0")]),
            "minecraft:wheat age=0",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:wheat", [("age", "7")]),
            "minecraft:wheat age=7",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id("minecraft:carrots", [("age", "0")]),
            "minecraft:carrots",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:potatoes", [("age", "0")]),
            "minecraft:potatoes",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:beetroots", [("age", "0")]),
            "minecraft:beetroots",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:nether_wart", [("age", "0")]),
            "minecraft:nether_wart",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:torchflower_crop", [("age", "0")]),
            "minecraft:torchflower_crop",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:pitcher_crop", [("age", "0"), ("half", "upper")]),
            "minecraft:pitcher_crop",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:pitcher_plant", [("half", "upper")]),
            "minecraft:pitcher_plant",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:cactus", [("age", "0")]),
            "minecraft:cactus",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:cactus_flower", []),
            "minecraft:cactus_flower",
            rgb_option(0xf2, 0x7f, 0xa5),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_produce_and_fungus_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:brown_mushroom", []),
            "minecraft:brown_mushroom",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id("minecraft:red_mushroom", []),
            "minecraft:red_mushroom",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:brown_mushroom_block",
                [
                    ("down", "true"),
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("up", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:brown_mushroom_block",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id(
                "minecraft:red_mushroom_block",
                [
                    ("down", "true"),
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("up", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:red_mushroom_block",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:mushroom_stem",
                [
                    ("down", "true"),
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("up", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:mushroom_stem",
            rgb_option(0xc7, 0xc7, 0xc7),
        ),
        (
            test_block_state_id("minecraft:pumpkin", []),
            "minecraft:pumpkin",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:carved_pumpkin", [("facing", "north")]),
            "minecraft:carved_pumpkin",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:jack_o_lantern", [("facing", "north")]),
            "minecraft:jack_o_lantern",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:melon", []),
            "minecraft:melon",
            rgb_option(0x7f, 0xcc, 0x19),
        ),
        (
            test_block_state_id("minecraft:hay_block", [("axis", "x")]),
            "minecraft:hay_block",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id("minecraft:dried_kelp_block", []),
            "minecraft:dried_kelp_block",
            rgb_option(0x66, 0x7f, 0x33),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_static_foliage_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:cherry_leaves",
                [
                    ("distance", "1"),
                    ("persistent", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:cherry_leaves",
            rgb_option(0xf2, 0x7f, 0xa5),
        ),
        (
            test_block_state_id(
                "minecraft:pale_oak_leaves",
                [
                    ("distance", "1"),
                    ("persistent", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:pale_oak_leaves",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:azalea_leaves",
                [
                    ("distance", "1"),
                    ("persistent", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:azalea_leaves",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id(
                "minecraft:flowering_azalea_leaves",
                [
                    ("distance", "1"),
                    ("persistent", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:flowering_azalea_leaves",
            rgb_option(0x00, 0x7c, 0x00),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_utility_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:bedrock", []),
            "minecraft:bedrock",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:stone_pressure_plate", [("powered", "true")]),
            "minecraft:stone_pressure_plate",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:sticky_piston",
                [("extended", "true"), ("facing", "north")],
            ),
            "minecraft:sticky_piston",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:note_block",
                [("instrument", "harp"), ("note", "0"), ("powered", "false")],
            ),
            "minecraft:note_block",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:chiseled_bookshelf",
                [
                    ("facing", "north"),
                    ("slot_0_occupied", "false"),
                    ("slot_1_occupied", "false"),
                    ("slot_2_occupied", "false"),
                    ("slot_3_occupied", "false"),
                    ("slot_4_occupied", "false"),
                    ("slot_5_occupied", "false"),
                ],
            ),
            "minecraft:chiseled_bookshelf",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:chest",
                [
                    ("facing", "north"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:chest",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:cobweb", []),
            "minecraft:cobweb",
            rgb_option(0xc7, 0xc7, 0xc7),
        ),
        (
            test_block_state_id("minecraft:tnt", [("unstable", "false")]),
            "minecraft:tnt",
            rgb_option(0xff, 0x00, 0x00),
        ),
        (
            test_block_state_id("minecraft:light_weighted_pressure_plate", [("power", "0")]),
            "minecraft:light_weighted_pressure_plate",
            rgb_option(0xfa, 0xee, 0x4d),
        ),
        (
            test_block_state_id("minecraft:heavy_weighted_pressure_plate", [("power", "0")]),
            "minecraft:heavy_weighted_pressure_plate",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:iron_door",
                [
                    ("facing", "north"),
                    ("half", "upper"),
                    ("hinge", "left"),
                    ("open", "true"),
                    ("powered", "true"),
                ],
            ),
            "minecraft:iron_door",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:iron_trapdoor",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("open", "true"),
                    ("powered", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:iron_trapdoor",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:brewing_stand",
                [
                    ("has_bottle_0", "true"),
                    ("has_bottle_1", "true"),
                    ("has_bottle_2", "true"),
                ],
            ),
            "minecraft:brewing_stand",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:cauldron", []),
            "minecraft:cauldron",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:lava_cauldron", []),
            "minecraft:lava_cauldron",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:powder_snow_cauldron", [("level", "1")]),
            "minecraft:powder_snow_cauldron",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:hopper",
                [("enabled", "true"), ("facing", "down")],
            ),
            "minecraft:hopper",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:stonecutter", [("facing", "north")]),
            "minecraft:stonecutter",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:bell",
                [
                    ("attachment", "floor"),
                    ("facing", "north"),
                    ("powered", "true"),
                ],
            ),
            "minecraft:bell",
            rgb_option(0xfa, 0xee, 0x4d),
        ),
        (
            test_block_state_id(
                "minecraft:lantern",
                [("hanging", "true"), ("waterlogged", "true")],
            ),
            "minecraft:lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:soul_lantern",
                [("hanging", "true"), ("waterlogged", "true")],
            ),
            "minecraft:soul_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:decorated_pot",
                [
                    ("cracked", "false"),
                    ("facing", "north"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:decorated_pot",
            rgb_option(0x8e, 0x3c, 0x2e),
        ),
        (
            test_block_state_id(
                "minecraft:crafter",
                [
                    ("crafting", "false"),
                    ("orientation", "down_east"),
                    ("triggered", "false"),
                ],
            ),
            "minecraft:crafter",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:vault",
                [
                    ("facing", "north"),
                    ("ominous", "false"),
                    ("vault_state", "inactive"),
                ],
            ),
            "minecraft:vault",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:heavy_core", [("waterlogged", "false")]),
            "minecraft:heavy_core",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_skull_and_head_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (
            test_block_state_id(
                "minecraft:skeleton_skull",
                [("powered", "true"), ("rotation", "15")],
            ),
            "minecraft:skeleton_skull",
        ),
        (
            test_block_state_id(
                "minecraft:skeleton_wall_skull",
                [("facing", "east"), ("powered", "false")],
            ),
            "minecraft:skeleton_wall_skull",
        ),
        (
            test_block_state_id(
                "minecraft:wither_skeleton_skull",
                [("powered", "false"), ("rotation", "0")],
            ),
            "minecraft:wither_skeleton_skull",
        ),
        (
            test_block_state_id(
                "minecraft:wither_skeleton_wall_skull",
                [("facing", "north"), ("powered", "true")],
            ),
            "minecraft:wither_skeleton_wall_skull",
        ),
        (
            test_block_state_id(
                "minecraft:zombie_head",
                [("powered", "true"), ("rotation", "7")],
            ),
            "minecraft:zombie_head",
        ),
        (
            test_block_state_id(
                "minecraft:zombie_wall_head",
                [("facing", "west"), ("powered", "false")],
            ),
            "minecraft:zombie_wall_head",
        ),
        (
            test_block_state_id(
                "minecraft:player_head",
                [("powered", "false"), ("rotation", "12")],
            ),
            "minecraft:player_head",
        ),
        (
            test_block_state_id(
                "minecraft:player_wall_head",
                [("facing", "south"), ("powered", "true")],
            ),
            "minecraft:player_wall_head",
        ),
        (
            test_block_state_id(
                "minecraft:creeper_head",
                [("powered", "true"), ("rotation", "2")],
            ),
            "minecraft:creeper_head",
        ),
        (
            test_block_state_id(
                "minecraft:creeper_wall_head",
                [("facing", "east"), ("powered", "true")],
            ),
            "minecraft:creeper_wall_head",
        ),
        (
            test_block_state_id(
                "minecraft:dragon_head",
                [("powered", "false"), ("rotation", "10")],
            ),
            "minecraft:dragon_head",
        ),
        (
            test_block_state_id(
                "minecraft:dragon_wall_head",
                [("facing", "north"), ("powered", "false")],
            ),
            "minecraft:dragon_wall_head",
        ),
        (
            test_block_state_id(
                "minecraft:piglin_head",
                [("powered", "true"), ("rotation", "5")],
            ),
            "minecraft:piglin_head",
        ),
        (
            test_block_state_id(
                "minecraft:piglin_wall_head",
                [("facing", "west"), ("powered", "true")],
            ),
            "minecraft:piglin_wall_head",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_redstone_fixture_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (
            test_block_state_id(
                "minecraft:powered_rail",
                [
                    ("powered", "true"),
                    ("shape", "ascending_east"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:powered_rail",
        ),
        (
            test_block_state_id(
                "minecraft:detector_rail",
                [
                    ("powered", "false"),
                    ("shape", "ascending_south"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:detector_rail",
        ),
        (
            test_block_state_id(
                "minecraft:rail",
                [("shape", "north_east"), ("waterlogged", "true")],
            ),
            "minecraft:rail",
        ),
        (
            test_block_state_id(
                "minecraft:activator_rail",
                [
                    ("powered", "true"),
                    ("shape", "ascending_west"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:activator_rail",
        ),
        (
            test_block_state_id(
                "minecraft:lever",
                [("face", "ceiling"), ("facing", "east"), ("powered", "true")],
            ),
            "minecraft:lever",
        ),
        (
            test_block_state_id(
                "minecraft:repeater",
                [
                    ("delay", "4"),
                    ("facing", "north"),
                    ("locked", "true"),
                    ("powered", "false"),
                ],
            ),
            "minecraft:repeater",
        ),
        (
            test_block_state_id(
                "minecraft:comparator",
                [
                    ("facing", "east"),
                    ("mode", "subtract"),
                    ("powered", "false"),
                ],
            ),
            "minecraft:comparator",
        ),
        (
            test_block_state_id(
                "minecraft:tripwire_hook",
                [
                    ("attached", "true"),
                    ("facing", "north"),
                    ("powered", "true"),
                ],
            ),
            "minecraft:tripwire_hook",
        ),
        (
            test_block_state_id(
                "minecraft:tripwire",
                [
                    ("attached", "true"),
                    ("disarmed", "false"),
                    ("east", "true"),
                    ("north", "false"),
                    ("powered", "true"),
                    ("south", "true"),
                    ("west", "false"),
                ],
            ),
            "minecraft:tripwire",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_redstone_utility_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:redstone_lamp", [("lit", "true")]),
            "minecraft:redstone_lamp",
            rgb_option(0x9f, 0x52, 0x24),
        ),
        (
            test_block_state_id(
                "minecraft:ender_chest",
                [("facing", "north"), ("waterlogged", "true")],
            ),
            "minecraft:ender_chest",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:observer",
                [("facing", "up"), ("powered", "true")],
            ),
            "minecraft:observer",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:trapped_chest",
                [
                    ("facing", "east"),
                    ("type", "right"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:trapped_chest",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:daylight_detector",
                [("inverted", "true"), ("power", "15")],
            ),
            "minecraft:daylight_detector",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:command_block",
                [("conditional", "true"), ("facing", "up")],
            ),
            "minecraft:command_block",
            rgb_option(0x66, 0x4c, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:repeating_command_block",
                [("conditional", "false"), ("facing", "down")],
            ),
            "minecraft:repeating_command_block",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id(
                "minecraft:chain_command_block",
                [("conditional", "true"), ("facing", "east")],
            ),
            "minecraft:chain_command_block",
            rgb_option(0x66, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:structure_block", [("mode", "data")]),
            "minecraft:structure_block",
            rgb_option(0x99, 0x99, 0x99),
        ),
        (
            test_block_state_id("minecraft:jigsaw", [("orientation", "up_north")]),
            "minecraft:jigsaw",
            rgb_option(0x99, 0x99, 0x99),
        ),
        (
            test_block_state_id("minecraft:test_block", [("mode", "fail")]),
            "minecraft:test_block",
            rgb_option(0x99, 0x99, 0x99),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_aquatic_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:tube_coral_block", []),
            "minecraft:tube_coral_block",
            rgb_option(0x33, 0x4c, 0xb2),
        ),
        (
            test_block_state_id("minecraft:brain_coral", [("waterlogged", "true")]),
            "minecraft:brain_coral",
            rgb_option(0xf2, 0x7f, 0xa5),
        ),
        (
            test_block_state_id("minecraft:bubble_coral_fan", [("waterlogged", "false")]),
            "minecraft:bubble_coral_fan",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id(
                "minecraft:fire_coral_wall_fan",
                [("facing", "east"), ("waterlogged", "true")],
            ),
            "minecraft:fire_coral_wall_fan",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:horn_coral_block", []),
            "minecraft:horn_coral_block",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id("minecraft:dead_tube_coral", [("waterlogged", "false")]),
            "minecraft:dead_tube_coral",
            rgb_option(0x4c, 0x4c, 0x4c),
        ),
        (
            test_block_state_id(
                "minecraft:dead_horn_coral_wall_fan",
                [("facing", "south"), ("waterlogged", "true")],
            ),
            "minecraft:dead_horn_coral_wall_fan",
            rgb_option(0x4c, 0x4c, 0x4c),
        ),
        (
            test_block_state_id(
                "minecraft:sea_pickle",
                [("pickles", "4"), ("waterlogged", "false")],
            ),
            "minecraft:sea_pickle",
            rgb_option(0x66, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:conduit", [("waterlogged", "true")]),
            "minecraft:conduit",
            rgb_option(0x5c, 0xdb, 0xd5),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_bamboo_honey_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:bamboo_sapling", []),
            "minecraft:bamboo_sapling",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:bamboo",
                [("age", "1"), ("leaves", "large"), ("stage", "1")],
            ),
            "minecraft:bamboo",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id("minecraft:sweet_berry_bush", [("age", "3")]),
            "minecraft:sweet_berry_bush",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id(
                "minecraft:campfire",
                [
                    ("facing", "east"),
                    ("lit", "false"),
                    ("signal_fire", "true"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:campfire",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            test_block_state_id(
                "minecraft:soul_campfire",
                [
                    ("facing", "south"),
                    ("lit", "true"),
                    ("signal_fire", "false"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:soul_campfire",
            rgb_option(0x81, 0x56, 0x31),
        ),
        (
            test_block_state_id("minecraft:honey_block", []),
            "minecraft:honey_block",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:honeycomb_block", []),
            "minecraft:honeycomb_block",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:lodestone", []),
            "minecraft:lodestone",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_water_plant_and_egg_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:seagrass", []),
            "minecraft:seagrass",
            rgb_option(0x40, 0x40, 0xff),
        ),
        (
            test_block_state_id("minecraft:tall_seagrass", [("half", "upper")]),
            "minecraft:tall_seagrass",
            rgb_option(0x40, 0x40, 0xff),
        ),
        (
            test_block_state_id("minecraft:kelp", [("age", "25")]),
            "minecraft:kelp",
            rgb_option(0x40, 0x40, 0xff),
        ),
        (
            test_block_state_id("minecraft:kelp_plant", []),
            "minecraft:kelp_plant",
            rgb_option(0x40, 0x40, 0xff),
        ),
        (
            test_block_state_id("minecraft:frogspawn", []),
            "minecraft:frogspawn",
            rgb_option(0x40, 0x40, 0xff),
        ),
        (
            test_block_state_id("minecraft:turtle_egg", [("eggs", "4"), ("hatch", "2")]),
            "minecraft:turtle_egg",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:sniffer_egg", [("hatch", "2")]),
            "minecraft:sniffer_egg",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:dried_ghast",
                [
                    ("facing", "east"),
                    ("hydration", "3"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:dried_ghast",
            rgb_option(0x4c, 0x4c, 0x4c),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_flower_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for flower in [
        "dandelion",
        "golden_dandelion",
        "torchflower",
        "poppy",
        "blue_orchid",
        "allium",
        "azure_bluet",
        "red_tulip",
        "orange_tulip",
        "white_tulip",
        "pink_tulip",
        "oxeye_daisy",
        "cornflower",
        "wither_rose",
        "lily_of_the_valley",
    ] {
        let block_name = format!("minecraft:{flower}");
        let block_state_id = test_block_state_id(&block_name, []);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x7c, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_tall_flower_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (flower, half) in [
        ("sunflower", "upper"),
        ("lilac", "lower"),
        ("rose_bush", "upper"),
        ("peony", "lower"),
    ] {
        let block_name = format!("minecraft:{flower}");
        let block_state_id = test_block_state_id(&block_name, [("half", half)]);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x7c, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_fire_cocoa_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:fire",
                [
                    ("age", "0"),
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("up", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:fire",
            rgb_option(0xff, 0x00, 0x00),
        ),
        (
            test_block_state_id("minecraft:soul_fire", []),
            "minecraft:soul_fire",
            rgb_option(0x66, 0x99, 0xd8),
        ),
        (
            test_block_state_id("minecraft:cocoa", [("age", "0"), ("facing", "north")]),
            "minecraft:cocoa",
            rgb_option(0x00, 0x7c, 0x00),
        ),
        (
            test_block_state_id(
                "minecraft:creaking_heart",
                [
                    ("axis", "x"),
                    ("creaking_heart_state", "uprooted"),
                    ("natural", "true"),
                ],
            ),
            "minecraft:creaking_heart",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_vanilla_default_none_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (test_block_state_id("minecraft:air", []), "minecraft:air"),
        (
            test_block_state_id("minecraft:cave_air", []),
            "minecraft:cave_air",
        ),
        (
            test_block_state_id("minecraft:void_air", []),
            "minecraft:void_air",
        ),
        (
            test_block_state_id("minecraft:nether_portal", [("axis", "x")]),
            "minecraft:nether_portal",
        ),
        (
            test_block_state_id("minecraft:test_instance_block", []),
            "minecraft:test_instance_block",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_final_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    let fence = [
        ("east", "true"),
        ("north", "false"),
        ("south", "false"),
        ("waterlogged", "false"),
        ("west", "true"),
    ];
    let lantern = [("hanging", "false"), ("waterlogged", "false")];

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:mycelium", [("snowy", "false")]),
            "minecraft:mycelium",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id("minecraft:packed_mud", []),
            "minecraft:packed_mud",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id("minecraft:nether_brick_fence", fence),
            "minecraft:nether_brick_fence",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:stripped_pale_oak_wood", [("axis", "z")]),
            "minecraft:stripped_pale_oak_wood",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:copper_lantern", lantern),
            "minecraft:copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:exposed_copper_lantern", lantern),
            "minecraft:exposed_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:weathered_copper_lantern", lantern),
            "minecraft:weathered_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:oxidized_copper_lantern", lantern),
            "minecraft:oxidized_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:waxed_copper_lantern", lantern),
            "minecraft:waxed_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:waxed_exposed_copper_lantern", lantern),
            "minecraft:waxed_exposed_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:waxed_weathered_copper_lantern", lantern),
            "minecraft:waxed_weathered_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:waxed_oxidized_copper_lantern", lantern),
            "minecraft:waxed_oxidized_copper_lantern",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_colors_cover_all_accepted_vanilla_block_states() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());
    let mut missing = std::collections::BTreeSet::new();

    for block_state in bbb_world::BlockStateRegistry::vanilla_26_1().iter() {
        if !falling_dust_provider_accepts_block_state(block_state.id) {
            continue;
        }

        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state.id);
        let batch = resolver.resolve_level_particles(&packet);
        if batch
            .commands
            .first()
            .is_none_or(|command| command.option_color.is_none())
        {
            missing.insert(block_state.name.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "missing falling_dust colors: {missing:#?}"
    );
}

#[test]
fn falling_dust_uses_default_none_fixture_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (
            test_block_state_id(
                "minecraft:ladder",
                [("facing", "north"), ("waterlogged", "true")],
            ),
            "minecraft:ladder",
        ),
        (
            test_block_state_id("minecraft:torch", []),
            "minecraft:torch",
        ),
        (
            test_block_state_id("minecraft:wall_torch", [("facing", "north")]),
            "minecraft:wall_torch",
        ),
        (
            test_block_state_id("minecraft:redstone_torch", [("lit", "false")]),
            "minecraft:redstone_torch",
        ),
        (
            test_block_state_id(
                "minecraft:redstone_wall_torch",
                [("facing", "north"), ("lit", "true")],
            ),
            "minecraft:redstone_wall_torch",
        ),
        (
            test_block_state_id("minecraft:soul_torch", []),
            "minecraft:soul_torch",
        ),
        (
            test_block_state_id("minecraft:soul_wall_torch", [("facing", "north")]),
            "minecraft:soul_wall_torch",
        ),
        (
            test_block_state_id("minecraft:copper_torch", []),
            "minecraft:copper_torch",
        ),
        (
            test_block_state_id("minecraft:copper_wall_torch", [("facing", "north")]),
            "minecraft:copper_wall_torch",
        ),
        (
            test_block_state_id("minecraft:end_rod", [("facing", "up")]),
            "minecraft:end_rod",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_default_none_pane_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name) in [
        (
            test_block_state_id("minecraft:glass", []),
            "minecraft:glass",
        ),
        (
            test_block_state_id(
                "minecraft:glass_pane",
                [
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("waterlogged", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:glass_pane",
        ),
        (
            test_block_state_id(
                "minecraft:iron_bars",
                [
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("waterlogged", "true"),
                    ("west", "true"),
                ],
            ),
            "minecraft:iron_bars",
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_default_none_metal_bars_chain_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for block_name in [
        "minecraft:copper_bars",
        "minecraft:exposed_copper_bars",
        "minecraft:weathered_copper_bars",
        "minecraft:oxidized_copper_bars",
        "minecraft:waxed_copper_bars",
        "minecraft:waxed_exposed_copper_bars",
        "minecraft:waxed_weathered_copper_bars",
        "minecraft:waxed_oxidized_copper_bars",
    ] {
        let block_state_id = test_block_state_id(
            block_name,
            [
                ("east", "true"),
                ("north", "true"),
                ("south", "true"),
                ("waterlogged", "true"),
                ("west", "true"),
            ],
        );
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }

    for block_name in [
        "minecraft:iron_chain",
        "minecraft:copper_chain",
        "minecraft:exposed_copper_chain",
        "minecraft:weathered_copper_chain",
        "minecraft:oxidized_copper_chain",
        "minecraft:waxed_copper_chain",
        "minecraft:waxed_exposed_copper_chain",
        "minecraft:waxed_weathered_copper_chain",
        "minecraft:waxed_oxidized_copper_chain",
    ] {
        let block_state_id =
            test_block_state_id(block_name, [("axis", "x"), ("waterlogged", "true")]);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x00, 0x00, 0x00)),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_functional_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:scaffolding",
                [
                    ("bottom", "true"),
                    ("distance", "0"),
                    ("waterlogged", "true"),
                ],
            ),
            "minecraft:scaffolding",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:loom", [("facing", "north")]),
            "minecraft:loom",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:barrel", [("facing", "north"), ("open", "true")]),
            "minecraft:barrel",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:smoker", [("facing", "north"), ("lit", "true")]),
            "minecraft:smoker",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:blast_furnace",
                [("facing", "north"), ("lit", "true")],
            ),
            "minecraft:blast_furnace",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:cartography_table", []),
            "minecraft:cartography_table",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:fletching_table", []),
            "minecraft:fletching_table",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id(
                "minecraft:grindstone",
                [("face", "floor"), ("facing", "north")],
            ),
            "minecraft:grindstone",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id(
                "minecraft:lectern",
                [
                    ("facing", "north"),
                    ("has_book", "true"),
                    ("powered", "true"),
                ],
            ),
            "minecraft:lectern",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:smithing_table", []),
            "minecraft:smithing_table",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:composter", [("level", "0")]),
            "minecraft:composter",
            rgb_option(0x8f, 0x77, 0x48),
        ),
        (
            test_block_state_id("minecraft:target", [("power", "0")]),
            "minecraft:target",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id(
                "minecraft:bee_nest",
                [("facing", "north"), ("honey_level", "0")],
            ),
            "minecraft:bee_nest",
            rgb_option(0xe5, 0xe5, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:beehive",
                [("facing", "north"), ("honey_level", "0")],
            ),
            "minecraft:beehive",
            rgb_option(0x8f, 0x77, 0x48),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_magic_utility_static_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:glowstone", []),
            "minecraft:glowstone",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:enchanting_table", []),
            "minecraft:enchanting_table",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:beacon", []),
            "minecraft:beacon",
            rgb_option(0x5c, 0xdb, 0xd5),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_decorative_colored_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:candle",
                [("candles", "4"), ("lit", "false"), ("waterlogged", "false")],
            ),
            "minecraft:candle",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id(
                "minecraft:white_candle",
                [("candles", "2"), ("lit", "true"), ("waterlogged", "false")],
            ),
            "minecraft:white_candle",
            rgb_option(0xc7, 0xc7, 0xc7),
        ),
        (
            test_block_state_id(
                "minecraft:purple_candle",
                [("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
            ),
            "minecraft:purple_candle",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id(
                "minecraft:white_bed",
                [("facing", "north"), ("occupied", "false"), ("part", "foot")],
            ),
            "minecraft:white_bed foot",
            rgb_option(0xff, 0xff, 0xff),
        ),
        (
            test_block_state_id(
                "minecraft:white_bed",
                [("facing", "north"), ("occupied", "false"), ("part", "head")],
            ),
            "minecraft:white_bed head",
            rgb_option(0xc7, 0xc7, 0xc7),
        ),
        (
            test_block_state_id(
                "minecraft:red_bed",
                [("facing", "east"), ("occupied", "true"), ("part", "foot")],
            ),
            "minecraft:red_bed foot",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:shulker_box", [("facing", "up")]),
            "minecraft:shulker_box",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id("minecraft:white_shulker_box", [("facing", "north")]),
            "minecraft:white_shulker_box",
            rgb_option(0xff, 0xff, 0xff),
        ),
        (
            test_block_state_id("minecraft:purple_shulker_box", [("facing", "down")]),
            "minecraft:purple_shulker_box",
            rgb_option(0x7a, 0x49, 0x58),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_cave_and_emissive_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:amethyst_block", []),
            "minecraft:amethyst_block",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id(
                "minecraft:small_amethyst_bud",
                [("facing", "down"), ("waterlogged", "false")],
            ),
            "minecraft:small_amethyst_bud",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
        (
            test_block_state_id("minecraft:tuff_bricks", []),
            "minecraft:tuff_bricks",
            rgb_option(0x39, 0x29, 0x23),
        ),
        (
            test_block_state_id("minecraft:calcite", []),
            "minecraft:calcite",
            rgb_option(0xd1, 0xb1, 0xa1),
        ),
        (
            test_block_state_id("minecraft:tinted_glass", []),
            "minecraft:tinted_glass",
            rgb_option(0x4c, 0x4c, 0x4c),
        ),
        (
            test_block_state_id("minecraft:powder_snow", []),
            "minecraft:powder_snow",
            rgb_option(0xff, 0xff, 0xff),
        ),
        (
            test_block_state_id(
                "minecraft:sculk_sensor",
                [
                    ("power", "3"),
                    ("sculk_sensor_phase", "active"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:sculk_sensor",
            rgb_option(0x4c, 0x7f, 0x99),
        ),
        (
            test_block_state_id("minecraft:sculk", []),
            "minecraft:sculk",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id(
                "minecraft:sculk_shrieker",
                [
                    ("can_summon", "false"),
                    ("shrieking", "false"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:sculk_shrieker",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id("minecraft:ochre_froglight", [("axis", "x")]),
            "minecraft:ochre_froglight",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:verdant_froglight", [("axis", "y")]),
            "minecraft:verdant_froglight",
            rgb_option(0x7f, 0xa7, 0x96),
        ),
        (
            test_block_state_id("minecraft:pearlescent_froglight", [("axis", "z")]),
            "minecraft:pearlescent_froglight",
            rgb_option(0xf2, 0x7f, 0xa5),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_copper_weathering_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:copper_block", []),
            "minecraft:copper_block",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:exposed_copper", []),
            "minecraft:exposed_copper",
            rgb_option(0x87, 0x6b, 0x62),
        ),
        (
            test_block_state_id("minecraft:weathered_copper", []),
            "minecraft:weathered_copper",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
        (
            test_block_state_id("minecraft:oxidized_copper", []),
            "minecraft:oxidized_copper",
            rgb_option(0x16, 0x7e, 0x86),
        ),
        (
            test_block_state_id("minecraft:raw_copper_block", []),
            "minecraft:raw_copper_block",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:waxed_oxidized_cut_copper_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:waxed_oxidized_cut_copper_slab",
            rgb_option(0x16, 0x7e, 0x86),
        ),
        (
            test_block_state_id(
                "minecraft:copper_door",
                [
                    ("facing", "east"),
                    ("half", "lower"),
                    ("hinge", "right"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            ),
            "minecraft:copper_door",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id(
                "minecraft:waxed_weathered_copper_bulb",
                [("lit", "false"), ("powered", "true")],
            ),
            "minecraft:waxed_weathered_copper_bulb",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
        (
            test_block_state_id(
                "minecraft:waxed_exposed_copper_grate",
                [("waterlogged", "false")],
            ),
            "minecraft:waxed_exposed_copper_grate",
            rgb_option(0x87, 0x6b, 0x62),
        ),
        (
            test_block_state_id(
                "minecraft:oxidized_copper_chest",
                [
                    ("facing", "east"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:oxidized_copper_chest",
            rgb_option(0x16, 0x7e, 0x86),
        ),
        (
            test_block_state_id(
                "minecraft:waxed_oxidized_copper_golem_statue",
                [
                    ("copper_golem_pose", "sitting"),
                    ("facing", "west"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:waxed_oxidized_copper_golem_statue",
            rgb_option(0x16, 0x7e, 0x86),
        ),
        (
            test_block_state_id(
                "minecraft:weathered_lightning_rod",
                [
                    ("facing", "up"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:weathered_lightning_rod",
            rgb_option(0x3a, 0x8e, 0x8c),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_nether_flora_and_blackstone_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:crimson_nylium", []),
            "minecraft:crimson_nylium",
            rgb_option(0xbd, 0x30, 0x31),
        ),
        (
            test_block_state_id("minecraft:warped_nylium", []),
            "minecraft:warped_nylium",
            rgb_option(0x16, 0x7e, 0x86),
        ),
        (
            test_block_state_id("minecraft:warped_wart_block", []),
            "minecraft:warped_wart_block",
            rgb_option(0x14, 0xb4, 0x85),
        ),
        (
            test_block_state_id("minecraft:nether_wart_block", []),
            "minecraft:nether_wart_block",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:warped_fungus", []),
            "minecraft:warped_fungus",
            rgb_option(0x4c, 0x7f, 0x99),
        ),
        (
            test_block_state_id("minecraft:crimson_fungus", []),
            "minecraft:crimson_fungus",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:shroomlight", []),
            "minecraft:shroomlight",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:weeping_vines", [("age", "13")]),
            "minecraft:weeping_vines",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:twisting_vines", [("age", "13")]),
            "minecraft:twisting_vines",
            rgb_option(0x4c, 0x7f, 0x99),
        ),
        (
            test_block_state_id("minecraft:magma_block", []),
            "minecraft:magma_block",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:respawn_anchor", [("charges", "4")]),
            "minecraft:respawn_anchor",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id("minecraft:smooth_basalt", []),
            "minecraft:smooth_basalt",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id("minecraft:blackstone", []),
            "minecraft:blackstone",
            rgb_option(0x19, 0x19, 0x19),
        ),
        (
            test_block_state_id(
                "minecraft:polished_blackstone_pressure_plate",
                [("powered", "false")],
            ),
            "minecraft:polished_blackstone_pressure_plate",
            rgb_option(0x19, 0x19, 0x19),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_quartz_prismarine_and_end_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:quartz_block", []),
            "minecraft:quartz_block",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:quartz_pillar", [("axis", "x")]),
            "minecraft:quartz_pillar",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id(
                "minecraft:smooth_quartz_stairs",
                [
                    ("facing", "east"),
                    ("half", "bottom"),
                    ("shape", "straight"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:smooth_quartz_stairs",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:quartz_bricks", []),
            "minecraft:quartz_bricks",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id("minecraft:sea_lantern", []),
            "minecraft:sea_lantern",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id(
                "minecraft:prismarine_wall",
                [
                    ("east", "low"),
                    ("north", "none"),
                    ("south", "none"),
                    ("up", "true"),
                    ("waterlogged", "false"),
                    ("west", "none"),
                ],
            ),
            "minecraft:prismarine_wall",
            rgb_option(0x4c, 0x7f, 0x99),
        ),
        (
            test_block_state_id(
                "minecraft:dark_prismarine_slab",
                [("type", "top"), ("waterlogged", "false")],
            ),
            "minecraft:dark_prismarine_slab",
            rgb_option(0x5c, 0xdb, 0xd5),
        ),
        (
            test_block_state_id(
                "minecraft:prismarine_brick_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "inner_left"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:prismarine_brick_stairs",
            rgb_option(0x5c, 0xdb, 0xd5),
        ),
        (
            test_block_state_id("minecraft:end_stone", []),
            "minecraft:end_stone",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id(
                "minecraft:end_stone_brick_wall",
                [
                    ("east", "low"),
                    ("north", "none"),
                    ("south", "none"),
                    ("up", "true"),
                    ("waterlogged", "false"),
                    ("west", "none"),
                ],
            ),
            "minecraft:end_stone_brick_wall",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id(
                "minecraft:end_portal_frame",
                [("eye", "true"), ("facing", "north")],
            ),
            "minecraft:end_portal_frame",
            rgb_option(0x66, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:purpur_pillar", [("axis", "z")]),
            "minecraft:purpur_pillar",
            rgb_option(0xb2, 0x4c, 0xd8),
        ),
        (
            test_block_state_id(
                "minecraft:purpur_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:purpur_slab",
            rgb_option(0xb2, 0x4c, 0xd8),
        ),
        (
            test_block_state_id("minecraft:chorus_flower", [("age", "5")]),
            "minecraft:chorus_flower",
            rgb_option(0x7f, 0x3f, 0xb2),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_construction_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    let wall = [
        ("east", "low"),
        ("north", "none"),
        ("south", "none"),
        ("up", "true"),
        ("waterlogged", "false"),
        ("west", "none"),
    ];

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id(
                "minecraft:stone_stairs",
                [
                    ("facing", "east"),
                    ("half", "bottom"),
                    ("shape", "straight"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:stone_stairs",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id(
                "minecraft:mossy_cobblestone_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:mossy_cobblestone_slab",
            rgb_option(0x70, 0x70, 0x70),
        ),
        (
            test_block_state_id("minecraft:granite_wall", wall),
            "minecraft:granite_wall",
            rgb_option(0x97, 0x6d, 0x4d),
        ),
        (
            test_block_state_id(
                "minecraft:diorite_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:diorite_slab",
            rgb_option(0xff, 0xfc, 0xf5),
        ),
        (
            test_block_state_id(
                "minecraft:smooth_sandstone_stairs",
                [
                    ("facing", "north"),
                    ("half", "top"),
                    ("shape", "inner_left"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:smooth_sandstone_stairs",
            rgb_option(0xf7, 0xe9, 0xa3),
        ),
        (
            test_block_state_id("minecraft:red_sandstone_wall", wall),
            "minecraft:red_sandstone_wall",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:bricks", []),
            "minecraft:bricks",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:brick_wall", wall),
            "minecraft:brick_wall",
            rgb_option(0x99, 0x33, 0x33),
        ),
        (
            test_block_state_id("minecraft:mud_bricks", []),
            "minecraft:mud_bricks",
            rgb_option(0x87, 0x6b, 0x62),
        ),
        (
            test_block_state_id(
                "minecraft:nether_brick_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
            "minecraft:nether_brick_slab",
            rgb_option(0x70, 0x02, 0x00),
        ),
        (
            test_block_state_id("minecraft:red_nether_brick_wall", wall),
            "minecraft:red_nether_brick_wall",
            rgb_option(0x70, 0x02, 0x00),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_uses_resin_and_pale_garden_map_color_fallbacks() {
    let mut resolver = test_resolver(0);
    resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

    let wall = [
        ("east", "low"),
        ("north", "none"),
        ("south", "none"),
        ("up", "true"),
        ("waterlogged", "false"),
        ("west", "none"),
    ];

    for (block_state_id, block_name, expected_color) in [
        (
            test_block_state_id("minecraft:resin_block", []),
            "minecraft:resin_block",
            rgb_option(0x9f, 0x52, 0x24),
        ),
        (
            test_block_state_id(
                "minecraft:resin_clump",
                [
                    ("down", "false"),
                    ("east", "false"),
                    ("north", "true"),
                    ("south", "false"),
                    ("up", "false"),
                    ("waterlogged", "false"),
                    ("west", "false"),
                ],
            ),
            "minecraft:resin_clump",
            rgb_option(0x9f, 0x52, 0x24),
        ),
        (
            test_block_state_id("minecraft:resin_brick_wall", wall),
            "minecraft:resin_brick_wall",
            rgb_option(0x9f, 0x52, 0x24),
        ),
        (
            test_block_state_id("minecraft:chiseled_resin_bricks", []),
            "minecraft:chiseled_resin_bricks",
            rgb_option(0x9f, 0x52, 0x24),
        ),
        (
            test_block_state_id("minecraft:pale_moss_block", []),
            "minecraft:pale_moss_block",
            rgb_option(0x99, 0x99, 0x99),
        ),
        (
            test_block_state_id(
                "minecraft:pale_moss_carpet",
                [
                    ("bottom", "false"),
                    ("east", "none"),
                    ("north", "none"),
                    ("south", "none"),
                    ("west", "none"),
                ],
            ),
            "minecraft:pale_moss_carpet",
            rgb_option(0x99, 0x99, 0x99),
        ),
        (
            test_block_state_id("minecraft:pale_hanging_moss", [("tip", "true")]),
            "minecraft:pale_hanging_moss",
            rgb_option(0x99, 0x99, 0x99),
        ),
        (
            test_block_state_id("minecraft:open_eyeblossom", []),
            "minecraft:open_eyeblossom",
            rgb_option(0xd8, 0x7f, 0x33),
        ),
        (
            test_block_state_id("minecraft:closed_eyeblossom", []),
            "minecraft:closed_eyeblossom",
            rgb_option(0xa7, 0xa7, 0xa7),
        ),
        (
            test_block_state_id("minecraft:firefly_bush", []),
            "minecraft:firefly_bush",
            rgb_option(0x00, 0x7c, 0x00),
        ),
    ] {
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1, "{block_name}");
        assert_eq!(
            batch.commands[0].option_color,
            Some(expected_color),
            "{block_name}"
        );
    }
}

#[test]
fn falling_dust_rejects_non_air_invisible_render_shape_blocks() {
    for (block_state_id, block_name, expected_commands) in [
        (
            test_block_state_id("minecraft:barrier", [("waterlogged", "false")]),
            "minecraft:barrier",
            0,
        ),
        (
            test_block_state_id("minecraft:water", [("level", "0")]),
            "minecraft:water",
            0,
        ),
        (
            test_block_state_id("minecraft:lava", [("level", "0")]),
            "minecraft:lava",
            0,
        ),
        (
            test_block_state_id("minecraft:bubble_column", [("drag", "true")]),
            "minecraft:bubble_column",
            0,
        ),
        (
            test_block_state_id("minecraft:structure_void", []),
            "minecraft:structure_void",
            0,
        ),
        (
            test_block_state_id("minecraft:end_gateway", []),
            "minecraft:end_gateway",
            0,
        ),
        (
            test_block_state_id("minecraft:end_portal", []),
            "minecraft:end_portal",
            0,
        ),
        (
            test_block_state_id("minecraft:light", [("level", "0"), ("waterlogged", "true")]),
            "minecraft:light",
            0,
        ),
        (
            test_block_state_id(
                "minecraft:moving_piston",
                [("facing", "north"), ("type", "normal")],
            ),
            "minecraft:moving_piston",
            0,
        ),
        (test_block_state_id("minecraft:air", []), "minecraft:air", 1),
        (
            test_block_state_id("minecraft:cave_air", []),
            "minecraft:cave_air",
            1,
        ),
        (
            test_block_state_id("minecraft:void_air", []),
            "minecraft:void_air",
            1,
        ),
        (
            test_block_state_id("minecraft:stone", []),
            "minecraft:stone",
            1,
        ),
    ] {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(block_state_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), expected_commands, "{block_name}");
        assert_eq!(batch.missing_definition_count, 0, "{block_name}");
        assert_eq!(batch.unknown_particle_type_count, 0, "{block_name}");
        if expected_commands == 1 {
            assert_eq!(
                batch.commands[0].option_block,
                Some(ParticleBlockOptionState { block_state_id }),
                "{block_name}"
            );
        }
    }
}

#[test]
fn falling_dust_provider_rejection_preserves_packet_random_sequence() {
    let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
    let stone_id = test_block_state_id("minecraft:stone", []);
    let mut rejected_resolver = test_resolver(42);
    let mut accepted_resolver = test_resolver(42);
    let mut rejected = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 2);
    rejected.particle.raw_options = block_particle_options(barrier_id);
    let mut accepted = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 2);
    accepted.particle.raw_options = block_particle_options(stone_id);

    let rejected_batch = rejected_resolver.resolve_level_particles(&rejected);
    let accepted_batch = accepted_resolver.resolve_level_particles(&accepted);
    assert_eq!(rejected_batch.len(), 0);
    assert_eq!(accepted_batch.len(), 2);

    let next_rejected = rejected_resolver
        .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));
    let next_accepted = accepted_resolver
        .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));

    assert_eq!(next_rejected.len(), 1);
    assert_eq!(next_accepted.len(), 1);
    assert_eq!(
        next_rejected.commands[0].position,
        next_accepted.commands[0].position
    );
    assert_eq!(
        next_rejected.commands[0].velocity,
        next_accepted.commands[0].velocity
    );
}

#[test]
fn gust_seed_particle_commands_carry_gust_child_template() {
    let mut resolver = test_resolver(0);
    for particle_type_id in [
        GUST_EMITTER_LARGE_PARTICLE_TYPE_ID,
        GUST_EMITTER_SMALL_PARTICLE_TYPE_ID,
    ] {
        let batch = resolver.resolve_level_particles(&level_particles_packet(particle_type_id, 0));

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, particle_type_id);
        assert_eq!(command.child_spawn_templates.len(), 1);
        let child = &command.child_spawn_templates[0];
        assert_eq!(child.particle_type_id, GUST_PARTICLE_TYPE_ID);
        assert_eq!(child.particle_id, "minecraft:gust");
        assert_eq!(child.sprite_ids, vec!["minecraft:gust_0".to_string()]);
    }
}

#[test]
fn spell_particle_options_decode_color_and_power_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(EFFECT_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = spell_particle_options(0x0011_2233, 0.5);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:effect");
    assert_eq!(
        command.option_color,
        Some([
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            1.0,
        ])
    );
    assert_eq!(command.option_power, Some(0.5));

    let mut instant_packet = level_particles_packet(INSTANT_EFFECT_PARTICLE_TYPE_ID, 0);
    instant_packet.particle.raw_options = spell_particle_options(0x00aa_bbcc, 1.25);
    let instant = resolver.resolve_level_particles(&instant_packet);
    assert_eq!(instant.len(), 1);
    assert_eq!(instant.commands[0].particle_id, "minecraft:instant_effect");
    assert_eq!(instant.commands[0].option_power, Some(1.25));
}

#[test]
fn entity_effect_particle_options_decode_argb_color_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(ENTITY_EFFECT_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = 0x8011_2233_u32.to_be_bytes().to_vec();

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:entity_effect");
    assert_eq!(
        command.option_color,
        Some([
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            0x80 as f32 / 255.0,
        ])
    );
    assert_eq!(command.option_power, None);
}

#[test]
fn flash_particle_options_decode_argb_color_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(FLASH_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = 0x6612_3456_u32.to_be_bytes().to_vec();

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:flash");
    assert_eq!(
        command.option_color,
        Some([
            0x12 as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x56 as f32 / 255.0,
            0x66 as f32 / 255.0,
        ])
    );
    assert_eq!(command.option_power, None);
}

#[test]
fn tinted_leaves_particle_options_decode_argb_color_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(TINTED_LEAVES_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = 0x7f44_6688_u32.to_be_bytes().to_vec();

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:tinted_leaves");
    assert_eq!(
        command.sprite_ids,
        vec!["minecraft:tinted_leaf_0".to_string()]
    );
    assert_eq!(
        command.option_color,
        Some([
            0x44 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0x88 as f32 / 255.0,
            0x7f as f32 / 255.0,
        ])
    );
    assert_eq!(command.option_power, None);
}

#[test]
fn dust_particle_options_decode_color_scale_and_transition_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(DUST_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = dust_particle_options(0x0012_3456, 2.5);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:dust");
    assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
    assert_eq!(
        command.option_color,
        Some([
            0x12 as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x56 as f32 / 255.0,
            1.0,
        ])
    );
    assert_eq!(command.option_scale, Some(2.5));
    assert_eq!(command.option_color_to, None);

    let mut transition_packet = level_particles_packet(DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID, 0);
    transition_packet.particle.raw_options =
        dust_color_transition_options(0x0001_0203, 0x00a0_b0c0, 9.0);
    let transition = resolver.resolve_level_particles(&transition_packet);
    assert_eq!(transition.len(), 1);
    let transition_command = &transition.commands[0];
    assert_eq!(
        transition_command.particle_id,
        "minecraft:dust_color_transition"
    );
    assert_eq!(
        transition_command.option_color,
        Some([1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0, 1.0])
    );
    assert_eq!(
        transition_command.option_color_to,
        Some([160.0 / 255.0, 176.0 / 255.0, 192.0 / 255.0, 1.0])
    );
    assert_eq!(transition_command.option_scale, Some(4.0));
}

#[test]
fn sculk_charge_particle_options_decode_roll_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(SCULK_CHARGE_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = 0.75_f32.to_be_bytes().to_vec();

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:sculk_charge");
    assert_eq!(
        command.sprite_ids,
        vec!["minecraft:sculk_charge_0".to_string()]
    );
    assert_eq!(command.option_roll, Some(0.75));
    assert_eq!(command.option_color, None);
    assert_eq!(command.option_power, None);
}

#[test]
fn trail_particle_options_decode_target_color_and_duration_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(TRAIL_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = trail_particle_options([1.5, 65.25, -4.75], 0x0012_3456, 27);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:trail");
    assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
    assert_eq!(command.option_target, Some([1.5, 65.25, -4.75]));
    assert_eq!(
        command.option_color,
        Some([
            0x12 as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x56 as f32 / 255.0,
            1.0,
        ])
    );
    assert_eq!(command.option_duration_ticks, Some(27));
    assert_eq!(command.option_power, None);
}

#[test]
fn vibration_particle_options_decode_block_target_and_arrival_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(VIBRATION_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = vibration_particle_block_options([1, 64, -2], 27);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:vibration");
    assert_eq!(command.sprite_ids, vec!["minecraft:vibration".to_string()]);
    assert_eq!(command.option_target, Some([1.5, 64.5, -1.5]));
    assert_eq!(command.option_entity_target_source, None);
    assert_eq!(command.option_duration_ticks, Some(27));
    assert_eq!(command.option_color, None);
    assert_eq!(command.option_power, None);
}

#[test]
fn vibration_particle_options_keep_entity_source_unresolved_for_later_lookup() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(VIBRATION_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = vibration_particle_entity_options(123, 0.75, 27);

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:vibration");
    assert_eq!(command.option_target, None);
    assert_eq!(
        command.option_entity_target_source,
        Some(ParticleEntityTargetSource {
            entity_id: 123,
            y_offset: 0.75
        })
    );
    assert_eq!(command.option_duration_ticks, Some(27));
    assert_eq!(
        vibration_entity_position_source_from_options(
            packet.particle.particle_type_id,
            &packet.particle.raw_options
        ),
        Some(VibrationEntityPositionSource {
            entity_id: 123,
            y_offset: 0.75
        })
    );
    assert_eq!(
        vibration_entity_position_source_from_options(
            CLOUD_PARTICLE_TYPE_ID,
            &packet.particle.raw_options
        ),
        None
    );
}

#[test]
fn vibration_particle_options_resolve_entity_source_with_spawn_context() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(VIBRATION_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = vibration_particle_entity_options(123, 0.75, 27);
    let context = LevelParticleSpawnContext {
        vibration_entity_position: Some(LevelParticleEntityPosition {
            entity_id: 123,
            position: [4.0, 5.0, 6.0],
        }),
        ..LevelParticleSpawnContext::default()
    };

    let batch = resolver.resolve_level_particles_with_context(&packet, context, None, None);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_id, "minecraft:vibration");
    assert_eq!(command.option_target, Some([4.0, 5.75, 6.0]));
    assert_eq!(
        command.option_entity_target_source,
        Some(ParticleEntityTargetSource {
            entity_id: 123,
            y_offset: 0.75
        })
    );
    assert_eq!(command.option_duration_ticks, Some(27));
}

#[test]
fn level_particles_decodes_shriek_delay_option_into_spawn_command() {
    let mut resolver = test_resolver(0);
    let mut packet = level_particles_packet(SHRIEK_PARTICLE_TYPE_ID, 0);
    packet.particle.raw_options = vec![17];

    let batch = resolver.resolve_level_particles(&packet);

    assert_eq!(batch.len(), 1);
    let command = &batch.commands[0];
    assert_eq!(command.particle_type_id, SHRIEK_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:shriek");
    assert_eq!(command.sprite_ids, vec!["minecraft:shriek_0".to_string()]);
    assert_eq!(command.raw_options_len, 1);
    assert_eq!(command.initial_delay_ticks, 17);
}

#[test]
fn positive_count_emits_deterministic_gaussian_scatter() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(4, 2));

    assert_eq!(batch.len(), 2);
    let first = &batch.commands[0];
    assert_close(first.position[0], 10.080253306373904);
    assert_close(first.position[1], 64.3196907823165);
    assert_close(first.position[2], -2.625723762871551);
    assert_close(first.velocity[0], 1.1456561526547341);
    assert_close(first.velocity[1], 1.4768617993237692);
    assert_close(first.velocity[2], -2.525118388151014);
}

#[test]
fn negative_count_emits_no_spawn_commands() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(4, -1));

    assert!(batch.is_empty());
}

#[test]
fn missing_definition_records_diagnostic_without_spawn_commands() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(18, 1));

    assert!(batch.commands.is_empty());
    assert_eq!(batch.missing_definition_count, 1);
    assert_eq!(batch.unknown_particle_type_count, 0);
}

#[test]
fn unknown_particle_type_records_diagnostic_without_spawn_commands() {
    let mut resolver = test_resolver(0);
    let batch = resolver.resolve_level_particles(&level_particles_packet(999, 1));

    assert!(batch.commands.is_empty());
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.unknown_particle_type_count, 1);
}

#[test]
fn missing_sprite_records_diagnostic_without_dropping_spawn_command() {
    let mut resolver = test_resolver_with_cloud_textures(
        0,
        ClientParticleStatus::All,
        &["minecraft:generic_7", "minecraft:missing_particle"],
        &["generic_7"],
    );
    let batch = resolver.resolve_level_particles(&level_particles_packet(4, 1));

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 1);
    assert_eq!(
        batch.commands[0].sprite_ids,
        vec![
            "minecraft:generic_7".to_string(),
            "minecraft:missing_particle".to_string(),
        ]
    );
}

#[test]
fn level_particles_drop_non_override_spawns_beyond_vanilla_camera_distance() {
    let mut packet = level_particles_packet(4, 0);
    packet.override_limiter = false;
    packet.always_show = false;
    packet.position = Vec3d {
        x: 33.0,
        y: 0.0,
        z: 0.0,
    };
    let context = LevelParticleSpawnContext {
        camera_position: Some([0.0, 0.0, 0.0]),
        ..LevelParticleSpawnContext::default()
    };
    let mut resolver = test_resolver(0);

    let batch = resolver.resolve_level_particles_with_context(&packet, context, None, None);

    assert!(batch.commands.is_empty());
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 0);
}

#[test]
fn level_particles_override_limiter_bypasses_distance_and_particle_status() {
    let mut packet = level_particles_packet(4, 0);
    packet.override_limiter = true;
    packet.always_show = false;
    packet.position = Vec3d {
        x: 33.0,
        y: 0.0,
        z: 0.0,
    };
    let context = LevelParticleSpawnContext {
        camera_position: Some([0.0, 0.0, 0.0]),
        ..LevelParticleSpawnContext::default()
    };
    let mut resolver = test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);

    let batch = resolver.resolve_level_particles_with_context(&packet, context, None, None);

    assert_eq!(batch.len(), 1);
    assert_eq!(batch.commands[0].position, [33.0, 0.0, 0.0]);
    assert!(batch.commands[0].override_limiter);
}

#[test]
fn decreased_particle_status_uses_vanilla_next_int_three() {
    let mut packet = level_particles_packet(4, 0);
    packet.override_limiter = false;
    packet.always_show = false;
    packet.position = Vec3d::default();
    let context = LevelParticleSpawnContext {
        camera_position: Some([0.0, 0.0, 0.0]),
        ..LevelParticleSpawnContext::default()
    };
    let mut dropping_resolver =
        test_resolver_with_particle_status(0, ClientParticleStatus::Decreased);
    let mut keeping_resolver =
        test_resolver_with_particle_status(2, ClientParticleStatus::Decreased);

    assert!(dropping_resolver
        .resolve_level_particles_with_context(&packet, context, None, None)
        .commands
        .is_empty());
    assert_eq!(
        keeping_resolver
            .resolve_level_particles_with_context(&packet, context, None, None)
            .len(),
        1
    );
}

#[test]
fn minimal_particle_status_only_keeps_always_show_promoted_particles() {
    let mut packet = level_particles_packet(4, 0);
    packet.override_limiter = false;
    packet.always_show = false;
    packet.position = Vec3d::default();
    let context = LevelParticleSpawnContext {
        camera_position: Some([0.0, 0.0, 0.0]),
        ..LevelParticleSpawnContext::default()
    };
    let mut plain_minimal = test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);
    assert!(plain_minimal
        .resolve_level_particles_with_context(&packet, context, None, None)
        .commands
        .is_empty());

    packet.always_show = true;
    let mut promoted = test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);
    assert_eq!(
        promoted
            .resolve_level_particles_with_context(&packet, context, None, None)
            .len(),
        1
    );

    let mut promoted_then_dropped =
        test_resolver_with_particle_status(42, ClientParticleStatus::Minimal);
    assert!(promoted_then_dropped
        .resolve_level_particles_with_context(&packet, context, None, None)
        .commands
        .is_empty());
}

#[test]
fn level_event_particles_map_vanilla_simple_side_effects() {
    let resolver = test_resolver(0);

    let mut composter_random = LevelEventSoundRandomState::with_seed(0);
    let composter = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: COMPOSTER_FILL_LEVEL_EVENT,
            ..level_event_packet(COMPOSTER_FILL_LEVEL_EVENT)
        },
        LevelEventParticleContext {
            composter_fill_center_shape_max_y: Some(13.0 / 16.0),
            ..LevelEventParticleContext::default()
        },
        &mut composter_random,
    );
    assert_eq!(composter.len(), 10);
    let (expected_position, expected_velocity) = first_composter_fill_particle(13.0 / 16.0);
    assert_particle_command(
        &composter.commands[0],
        COMPOSTER_PARTICLE_TYPE_ID,
        "minecraft:composter",
        expected_position,
        expected_velocity,
        false,
    );

    let mut fallback_composter_random = LevelEventSoundRandomState::with_seed(0);
    let fallback_composter = resolver.resolve_level_event_particles(
        &level_event_packet(COMPOSTER_FILL_LEVEL_EVENT),
        &mut fallback_composter_random,
    );
    assert_eq!(fallback_composter.len(), 10);
    let (expected_position, expected_velocity) = first_composter_fill_particle(1.0);
    assert_particle_command(
        &fallback_composter.commands[0],
        COMPOSTER_PARTICLE_TYPE_ID,
        "minecraft:composter",
        expected_position,
        expected_velocity,
        false,
    );

    let mut lava_random = LevelEventSoundRandomState::with_seed(0);
    let lava = resolver.resolve_level_event_particles(&level_event_packet(1501), &mut lava_random);
    assert_eq!(lava.len(), 8);
    assert_particle_command(
        &lava.commands[0],
        55,
        "minecraft:large_smoke",
        [10.730_967_787_376_657, 65.2, -2.759_463_584_328_514],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut burnout_random = LevelEventSoundRandomState::with_seed(0);
    burnout_random.next_float();
    burnout_random.next_float();
    let burnout = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 1502,
            ..level_event_packet(1502)
        },
        &mut burnout_random,
    );
    assert_eq!(burnout.len(), 5);
    assert_particle_command(
        &burnout.commands[0],
        62,
        "minecraft:smoke",
        [
            10.344_321_849_402_891,
            64.582_450_455_210_06,
            -2.469_737_796_929_42,
        ],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut frame_fill_random = LevelEventSoundRandomState::with_seed(0);
    let frame_fill = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 1503,
            ..level_event_packet(1503)
        },
        &mut frame_fill_random,
    );
    assert_eq!(frame_fill.len(), 16);
    assert_particle_command(
        &frame_fill.commands[0],
        62,
        "minecraft:smoke",
        [10.586_612_920_266_246, 64.8125, -2.597_298_844_123_192_6],
        [0.0, 0.0, 0.0],
        false,
    );

    let dripstone_event = LevelEvent {
        event_type: DRIPSTONE_DRIP_LEVEL_EVENT,
        ..level_event_packet(DRIPSTONE_DRIP_LEVEL_EVENT)
    };
    let mut water_drip_random = LevelEventSoundRandomState::with_seed(0);
    let water_drip = resolver.resolve_level_event_particles_with_context(
        &dripstone_event,
        LevelEventParticleContext {
            dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Water),
            ..LevelEventParticleContext::default()
        },
        &mut water_drip_random,
    );
    assert_eq!(water_drip.len(), 1);
    assert_particle_command(
        &water_drip.commands[0],
        DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
        "minecraft:dripping_dripstone_water",
        [10.583_333_343_267_44, 64.25, -2.416_666_656_732_56],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut lava_drip_random = LevelEventSoundRandomState::with_seed(0);
    let lava_drip = resolver.resolve_level_event_particles_with_context(
        &dripstone_event,
        LevelEventParticleContext {
            dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Lava),
            ..LevelEventParticleContext::default()
        },
        &mut lava_drip_random,
    );
    assert_eq!(lava_drip.len(), 1);
    assert_eq!(
        lava_drip.commands[0].particle_type_id,
        DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID
    );
    assert_eq!(
        lava_drip.commands[0].particle_id,
        "minecraft:dripping_dripstone_lava"
    );

    let mut missing_context_random = LevelEventSoundRandomState::with_seed(0);
    let missing_context_drip =
        resolver.resolve_level_event_particles(&dripstone_event, &mut missing_context_random);
    assert!(missing_context_drip.commands.is_empty());

    let growth_event = LevelEvent {
        event_type: PLANT_GROWTH_LEVEL_EVENT,
        data: 2,
        ..level_event_packet(PLANT_GROWTH_LEVEL_EVENT)
    };
    let mut growth_in_block_random = LevelEventSoundRandomState::with_seed(0);
    let growth_in_block = resolver.resolve_level_event_particles_with_context(
        &growth_event,
        LevelEventParticleContext {
            growth_particles: Some(LevelEventGrowthParticleContext {
                pos: growth_event.pos,
                mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
            }),
            ..LevelEventParticleContext::default()
        },
        &mut growth_in_block_random,
    );
    assert_eq!(growth_in_block.len(), 2);
    assert_particle_command(
        &growth_in_block.commands[0],
        HAPPY_VILLAGER_PARTICLE_TYPE_ID,
        "minecraft:happy_villager",
        [
            10.597_545_277_797_202,
            64.333_218_399_476_65,
            -2.614_810_815_259_281_7,
        ],
        [
            0.016_050_661_274_780_612,
            -0.018_030_921_768_350_243,
            0.041_618_415_808_563_26,
        ],
        false,
    );

    let mut growth_wide_random = LevelEventSoundRandomState::with_seed(0);
    let growth_wide = resolver.resolve_level_event_particles_with_context(
        &growth_event,
        LevelEventParticleContext {
            growth_particles: Some(LevelEventGrowthParticleContext {
                pos: growth_event.pos,
                mode: LevelEventGrowthParticleMode::WideNoFloating {
                    support: LevelEventGrowthParticleSupport::full(),
                },
            }),
            ..LevelEventParticleContext::default()
        },
        &mut growth_wide_random,
    );
    let (growth_wide_position, growth_wide_velocity) = first_growth_wide_particle(growth_event.pos);
    assert_eq!(growth_wide.len(), 6);
    assert_particle_command(
        &growth_wide.commands[0],
        HAPPY_VILLAGER_PARTICLE_TYPE_ID,
        "minecraft:happy_villager",
        growth_wide_position,
        growth_wide_velocity,
        false,
    );

    let mut empty_support_growth_random = LevelEventSoundRandomState::with_seed(0);
    let empty_support_growth = resolver.resolve_level_event_particles_with_context(
        &growth_event,
        LevelEventParticleContext {
            growth_particles: Some(LevelEventGrowthParticleContext {
                pos: growth_event.pos,
                mode: LevelEventGrowthParticleMode::WideNoFloating {
                    support: LevelEventGrowthParticleSupport::empty(),
                },
            }),
            ..LevelEventParticleContext::default()
        },
        &mut empty_support_growth_random,
    );
    assert!(empty_support_growth.commands.is_empty());

    let mut shriek_random = LevelEventSoundRandomState::with_seed(0);
    let shriek = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3007,
            ..level_event_packet(3007)
        },
        &mut shriek_random,
    );
    assert_eq!(shriek.len(), 10);
    assert_particle_command(
        &shriek.commands[0],
        SHRIEK_PARTICLE_TYPE_ID,
        "minecraft:shriek",
        [10.5, 64.5, -2.5],
        [0.0, 0.0, 0.0],
        false,
    );
    assert_eq!(shriek.commands[0].initial_delay_ticks, 0);
    assert_particle_command_with_delay(
        &shriek.commands[9],
        SHRIEK_PARTICLE_TYPE_ID,
        "minecraft:shriek",
        [10.5, 64.5, -2.5],
        [0.0, 0.0, 0.0],
        false,
        45,
    );

    let mut blaze_random = LevelEventSoundRandomState::with_seed(0);
    let blaze = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2004,
            ..level_event_packet(2004)
        },
        &mut blaze_random,
    );
    assert_eq!(blaze.len(), 40);
    assert_particle_command(
        &blaze.commands[0],
        62,
        "minecraft:smoke",
        [
            10.961_935_574_753_314,
            63.981_072_831_342_97,
            -2.225_165_149_299_783_7,
        ],
        [0.0, 0.0, 0.0],
        false,
    );
    assert_particle_command(
        &blaze.commands[1],
        32,
        "minecraft:flame",
        [
            10.961_935_574_753_314,
            63.981_072_831_342_97,
            -2.225_165_149_299_783_7,
        ],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut dragon_breath_random = LevelEventSoundRandomState::with_seed(0);
    let dragon_breath = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2006,
            ..level_event_packet(2006)
        },
        &mut dragon_breath_random,
    );
    assert_eq!(dragon_breath.len(), 200);
    assert_particle_command(
        &dragon_breath.commands[0],
        8,
        "minecraft:dragon_breath",
        [10.143_172_562_122_345, 64.3, -3.254_934_978_485_107_6],
        [
            4.186_181_081_614_109,
            0.188_500_336_334_049_33,
            -7.453_970_007_633_87,
        ],
        false,
    );

    let mut potion_random = LevelEventSoundRandomState::with_seed(0);
    let potion = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2002,
            data: 0x0033_66cc,
            ..level_event_packet(2002)
        },
        &mut potion_random,
    );
    assert_eq!(potion.len(), 108);
    assert_item_break_particle_command(
        &potion.commands[0],
        VANILLA_SPLASH_POTION_ITEM_ID,
        [10.5, 64.0, -2.5],
        first_item_break_particle_velocity(0),
    );
    let (expected_position, expected_velocity, expected_color, expected_power) =
        first_potion_break_spell_particle(0x0033_66cc);
    assert_particle_command(
        &potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize],
        EFFECT_PARTICLE_TYPE_ID,
        "minecraft:effect",
        expected_position,
        expected_velocity,
        false,
    );
    assert_eq!(
        potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_color,
        Some(expected_color)
    );
    assert_eq!(
        potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_power,
        Some(expected_power)
    );

    let mut instant_potion_random = LevelEventSoundRandomState::with_seed(0);
    let instant_potion = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2007,
            data: 0x00aa_bbcc,
            ..level_event_packet(2007)
        },
        &mut instant_potion_random,
    );
    assert_eq!(instant_potion.len(), 108);
    assert_item_break_particle_command(
        &instant_potion.commands[0],
        VANILLA_SPLASH_POTION_ITEM_ID,
        [10.5, 64.0, -2.5],
        first_item_break_particle_velocity(0),
    );
    assert_eq!(
        instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].particle_type_id,
        INSTANT_EFFECT_PARTICLE_TYPE_ID
    );
    assert_eq!(
        instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].particle_id,
        "minecraft:instant_effect"
    );
    assert_eq!(
        instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_power,
        Some(first_potion_break_spell_particle(0x00aa_bbcc).3)
    );

    let mut ender_eye_random = LevelEventSoundRandomState::with_seed(0);
    let ender_eye = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2003,
            ..level_event_packet(2003)
        },
        &mut ender_eye_random,
    );
    assert_eq!(ender_eye.len(), 88);
    assert_item_break_particle_command(
        &ender_eye.commands[0],
        VANILLA_ENDER_EYE_ITEM_ID,
        [10.5, 64.0, -2.5],
        first_item_break_particle_velocity(0),
    );
    let first_portal_index = ITEM_BREAK_PARTICLE_COUNT as usize;
    assert_eq!(
        ender_eye.commands[first_portal_index].sprite_ids,
        vec![
            "minecraft:generic_0".to_string(),
            "minecraft:generic_1".to_string(),
            "minecraft:generic_2".to_string(),
            "minecraft:generic_3".to_string(),
            "minecraft:generic_4".to_string(),
            "minecraft:generic_5".to_string(),
            "minecraft:generic_6".to_string(),
            "minecraft:generic_7".to_string(),
        ]
    );
    assert_particle_command(
        &ender_eye.commands[first_portal_index],
        60,
        "minecraft:portal",
        [15.5, 63.6, -2.5],
        [-5.0, 0.0, -0.0],
        false,
    );
    assert_particle_command(
        &ender_eye.commands[first_portal_index + 1],
        60,
        "minecraft:portal",
        [15.5, 63.6, -2.5],
        [-7.0, 0.0, -0.0],
        false,
    );
    assert_particle_command(
        &ender_eye.commands[first_portal_index + 20],
        60,
        "minecraft:portal",
        [10.5, 63.6, 2.5],
        [-0.0, 0.0, -5.0],
        false,
    );

    let mut explosion_random = LevelEventSoundRandomState::with_seed(0);
    let explosion = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2008,
            ..level_event_packet(2008)
        },
        &mut explosion_random,
    );
    assert_eq!(explosion.len(), 1);
    assert_particle_command(
        &explosion.commands[0],
        23,
        "minecraft:explosion",
        [10.5, 64.5, -2.5],
        [0.0, 0.0, 0.0],
        true,
    );

    let mut gateway_random = LevelEventSoundRandomState::with_seed(0);
    let gateway = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3000,
            ..level_event_packet(3000)
        },
        &mut gateway_random,
    );
    assert_eq!(gateway.len(), 1);
    assert_particle_command_with_visibility(
        &gateway.commands[0],
        22,
        "minecraft:explosion_emitter",
        [10.5, 64.5, -2.5],
        [0.0, 0.0, 0.0],
        true,
        true,
    );

    let mut electric_x_random = LevelEventSoundRandomState::with_seed(0);
    let electric_x = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3002,
            data: 0,
            ..level_event_packet(3002)
        },
        &mut electric_x_random,
    );
    assert_eq!(electric_x.len(), 10);
    assert_particle_command(
        &electric_x.commands[0],
        103,
        "minecraft:electric_spark",
        [
            10.831_440_988_787_062,
            64.526_586_303_999_34,
            -2.547_737_357_950_073,
        ],
        [-0.765_986_782_385_549_7, 0.0, 0.0],
        true,
    );

    let mut electric_z_random = LevelEventSoundRandomState::with_seed(0);
    let electric_z = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3002,
            data: 2,
            ..level_event_packet(3002)
        },
        &mut electric_z_random,
    );
    assert_eq!(electric_z.len(), 10);
    assert_particle_command(
        &electric_z.commands[0],
        103,
        "minecraft:electric_spark",
        [
            10.582_860_247_196_765,
            64.526_586_303_999_34,
            -2.690_949_431_800_291,
        ],
        [0.0, 0.0, -0.765_986_782_385_549_7],
        true,
    );

    let mut electric_fallback_random = LevelEventSoundRandomState::with_seed(0);
    let electric_fallback = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3002,
            data: 3,
            ..level_event_packet(3002)
        },
        &mut electric_fallback_random,
    );
    assert_eq!(electric_fallback.len(), 23);
    assert_particle_command(
        &electric_fallback.commands[0],
        103,
        "minecraft:electric_spark",
        [10.117_006_608_807_225, 63.95, -2.218_465_367_954_695_3],
        [0.331_440_988_787_061_2, 0.0, -0.190_949_431_800_290_78],
        true,
    );

    let mut wax_on_random = LevelEventSoundRandomState::with_seed(0);
    let wax_on = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3003,
            ..level_event_packet(3003)
        },
        &mut wax_on_random,
    );
    assert_eq!(wax_on.len(), 23);
    assert_particle_command(
        &wax_on.commands[0],
        101,
        "minecraft:wax_on",
        [10.117_006_608_807_225, 63.95, -2.218_465_367_954_695_3],
        [0.331_440_988_787_061_2, 0.0, -0.190_949_431_800_290_78],
        true,
    );

    let mut wax_off_random = LevelEventSoundRandomState::with_seed(0);
    let wax_off = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3004,
            ..level_event_packet(3004)
        },
        &mut wax_off_random,
    );
    assert_eq!(wax_off.len(), 23);
    assert_eq!(wax_off.commands[0].particle_type_id, 102);
    assert_eq!(wax_off.commands[0].particle_id, "minecraft:wax_off");

    let mut scrape_random = LevelEventSoundRandomState::with_seed(0);
    let scrape = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3005,
            ..level_event_packet(3005)
        },
        &mut scrape_random,
    );
    assert_eq!(scrape.len(), 23);
    assert_eq!(scrape.commands[0].particle_type_id, 104);
    assert_eq!(scrape.commands[0].particle_id, "minecraft:scrape");

    let mut sculk_charge_full_random = LevelEventSoundRandomState::with_seed(0);
    let sculk_charge_full_data = 2 << 6;
    let sculk_charge_full = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3006,
            data: sculk_charge_full_data,
            ..level_event_packet(3006)
        },
        &mut sculk_charge_full_random,
    );
    let expected_sculk_charge_full = expected_sculk_charge_particles(sculk_charge_full_data);
    assert_eq!(sculk_charge_full.len(), expected_sculk_charge_full.len());
    assert_sculk_charge_command(
        &sculk_charge_full.commands[0],
        &expected_sculk_charge_full[0],
    );
    assert_eq!(
        sculk_charge_full.commands[0].sprite_ids,
        vec!["minecraft:sculk_charge_0".to_string()]
    );
    assert_eq!(
        sculk_charge_full.commands[0].option_roll,
        Some(expected_sculk_charge_full[0].roll)
    );

    let mut sculk_charge_mask_random = LevelEventSoundRandomState::with_seed(0);
    let sculk_charge_mask_data = (3 << 6) | (1 << 1) | (1 << 4);
    let sculk_charge_mask = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3006,
            data: sculk_charge_mask_data,
            ..level_event_packet(3006)
        },
        &mut sculk_charge_mask_random,
    );
    let expected_sculk_charge_mask = expected_sculk_charge_particles(sculk_charge_mask_data);
    assert_eq!(sculk_charge_mask.len(), expected_sculk_charge_mask.len());
    assert_sculk_charge_command(
        &sculk_charge_mask.commands[0],
        &expected_sculk_charge_mask[0],
    );
    assert_eq!(
        sculk_charge_mask.commands[0].option_roll,
        Some(std::f32::consts::PI)
    );
    let first_west_expected = expected_sculk_charge_mask
        .iter()
        .find(|expected| expected.direction == (-1, 0, 0))
        .expect("west multiface particle");
    let first_west_actual = sculk_charge_mask
        .commands
        .iter()
        .find(|command| {
            (command.position[0] - (10.5 - SCULK_CHARGE_MULTIFACE_FACTOR)).abs() < 1.0e-12
        })
        .expect("west multiface command");
    assert_sculk_charge_command(first_west_actual, first_west_expected);
    assert_eq!(first_west_actual.option_roll, Some(0.0));

    let mut sculk_charge_pop_random = LevelEventSoundRandomState::with_seed(0);
    let sculk_charge_pop = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3006,
            data: 0,
            ..level_event_packet(3006)
        },
        &mut sculk_charge_pop_random,
    );
    let expected_sculk_charge_pop = expected_sculk_charge_pop_particles(false);
    assert_eq!(sculk_charge_pop.len(), expected_sculk_charge_pop.len());
    assert_sculk_charge_pop_command(&sculk_charge_pop.commands[0], &expected_sculk_charge_pop[0]);
    assert_eq!(
        sculk_charge_pop.commands[0].sprite_ids,
        vec!["minecraft:sculk_charge_pop_0".to_string()]
    );

    let mut sculk_charge_pop_full_random = LevelEventSoundRandomState::with_seed(0);
    let sculk_charge_pop_full = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: 3006,
            data: 0,
            ..level_event_packet(3006)
        },
        LevelEventParticleContext {
            sculk_charge_pop_full_block: Some(true),
            ..LevelEventParticleContext::default()
        },
        &mut sculk_charge_pop_full_random,
    );
    let expected_sculk_charge_pop_full = expected_sculk_charge_pop_particles(true);
    assert_eq!(
        sculk_charge_pop_full.len(),
        expected_sculk_charge_pop_full.len()
    );
    assert_sculk_charge_pop_command(
        &sculk_charge_pop_full.commands[0],
        &expected_sculk_charge_pop_full[0],
    );

    let mut egg_crack_random = LevelEventSoundRandomState::with_seed(0);
    let egg_crack = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3009,
            ..level_event_packet(3009)
        },
        &mut egg_crack_random,
    );
    assert_eq!(egg_crack.len(), 30);
    assert_eq!(egg_crack.commands[0].particle_type_id, 106);
    assert_eq!(egg_crack.commands[0].particle_id, "minecraft:egg_crack");
    assert!(!egg_crack.commands[0].override_limiter);

    let mut trial_spawn_random = LevelEventSoundRandomState::with_seed(0);
    let trial_spawn = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3011,
            data: 0,
            ..level_event_packet(3011)
        },
        &mut trial_spawn_random,
    );
    assert_eq!(trial_spawn.len(), 40);
    assert_particle_command(
        &trial_spawn.commands[0],
        62,
        "minecraft:smoke",
        [
            10.961_935_574_753_314,
            63.981_072_831_342_97,
            -2.225_165_149_299_783_7,
        ],
        [0.0, 0.0, 0.0],
        false,
    );
    assert_particle_command(
        &trial_spawn.commands[1],
        32,
        "minecraft:flame",
        [
            10.961_935_574_753_314,
            63.981_072_831_342_97,
            -2.225_165_149_299_783_7,
        ],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut trial_spawn_ominous_random = LevelEventSoundRandomState::with_seed(0);
    let trial_spawn_ominous = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3011,
            data: 1,
            ..level_event_packet(3011)
        },
        &mut trial_spawn_ominous_random,
    );
    assert_eq!(trial_spawn_ominous.len(), 40);
    assert_eq!(trial_spawn_ominous.commands[1].particle_type_id, 40);
    assert_eq!(
        trial_spawn_ominous.commands[1].particle_id,
        "minecraft:soul_fire_flame"
    );

    let mut trial_spawn_mob_random = LevelEventSoundRandomState::with_seed(0);
    let trial_spawn_mob = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3012,
            data: 1,
            ..level_event_packet(3012)
        },
        &mut trial_spawn_mob_random,
    );
    assert_eq!(trial_spawn_mob.len(), 40);
    assert_eq!(trial_spawn_mob.commands[1].particle_type_id, 40);
    assert_eq!(
        trial_spawn_mob.commands[1].particle_id,
        "minecraft:soul_fire_flame"
    );

    let mut trial_detect_random = LevelEventSoundRandomState::with_seed(0);
    let trial_detect = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3013,
            data: 2,
            ..level_event_packet(3013)
        },
        &mut trial_detect_random,
    );
    assert_eq!(trial_detect.len(), 40);
    assert_particle_command(
        &trial_detect.commands[0],
        108,
        "minecraft:trial_spawner_detection",
        [
            10.800_258_088_111_878,
            64.292_429_113_388_05,
            -2.069_126_719_236_374,
        ],
        [0.0, 0.0, 0.0],
        true,
    );

    let mut trial_eject_random = LevelEventSoundRandomState::with_seed(0);
    let trial_eject = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3017,
            ..level_event_packet(3017)
        },
        &mut trial_eject_random,
    );
    assert_eq!(trial_eject.len(), 40);
    assert_particle_command(
        &trial_eject.commands[0],
        93,
        "minecraft:small_flame",
        [
            10.546_193_557_475_332,
            64.448_107_283_134_3,
            -2.472_516_514_929_978_4,
        ],
        [
            0.022_619_280_994_487_918,
            0.043_745_738_729_615_43,
            -0.007_831_529_827_929_628,
        ],
        false,
    );
    assert_particle_command(
        &trial_eject.commands[1],
        62,
        "minecraft:smoke",
        [
            10.546_193_557_475_332,
            64.448_107_283_134_3,
            -2.472_516_514_929_978_4,
        ],
        [
            0.022_619_280_994_487_918,
            0.043_745_738_729_615_43,
            -0.031_326_119_311_718_51,
        ],
        false,
    );

    let mut trial_eject_sound_event_random = LevelEventSoundRandomState::with_seed(0);
    let trial_eject_sound_event = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3014,
            ..level_event_packet(3014)
        },
        &mut trial_eject_sound_event_random,
    );
    assert_eq!(trial_eject_sound_event.len(), 40);
    assert_eq!(trial_eject_sound_event.commands[0].particle_type_id, 93);
    assert_eq!(trial_eject_sound_event.commands[1].particle_type_id, 62);

    let mut missing_vault_activation_random = LevelEventSoundRandomState::with_seed(0);
    let missing_vault_activation = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3015,
            data: 0,
            ..level_event_packet(3015)
        },
        &mut missing_vault_activation_random,
    );
    assert!(missing_vault_activation.is_empty());

    let vault_activation_position = first_vault_activation_particle();
    let mut vault_activation_random = LevelEventSoundRandomState::with_seed(0);
    let vault_activation = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: 3015,
            data: 0,
            ..level_event_packet(3015)
        },
        LevelEventParticleContext {
            vault_block_entity_at_event_pos: true,
            ..LevelEventParticleContext::default()
        },
        &mut vault_activation_random,
    );
    assert_eq!(vault_activation.len(), 40);
    assert_particle_command(
        &vault_activation.commands[0],
        SMOKE_PARTICLE_TYPE_ID,
        "minecraft:smoke",
        vault_activation_position,
        [0.0, 0.0, 0.0],
        false,
    );
    assert_particle_command(
        &vault_activation.commands[1],
        SMALL_FLAME_PARTICLE_TYPE_ID,
        "minecraft:small_flame",
        vault_activation_position,
        [0.0, 0.0, 0.0],
        false,
    );

    let connection_origin = [11.0, 65.75, -3.5];
    let connection_target_position = [12.25, 66.9, -2.0];
    let connection = VaultConnectionParticleState {
        origin: connection_origin,
        targets: vec![bbb_world::VaultConnectionParticleTargetState {
            entity_id: 77,
            uuid: uuid::Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff),
            target_position: connection_target_position,
        }],
    };
    let mut vault_connection_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_connection_count = expected_random.next_int_bound(4) + 2;
    let expected_connection_velocity = [
        connection_target_position[0] - connection_origin[0]
            + f64::from(expected_random.next_float() - 0.5),
        connection_target_position[1] - connection_origin[1]
            + f64::from(expected_random.next_float() - 0.5),
        connection_target_position[2] - connection_origin[2]
            + f64::from(expected_random.next_float() - 0.5),
    ];
    for _ in 1..expected_connection_count {
        let _ = expected_random.next_float();
        let _ = expected_random.next_float();
        let _ = expected_random.next_float();
    }
    let expected_smoke_position_after_connection = [
        10.0 + expected_random_between(&mut expected_random, 0.1, 0.9),
        64.0 + expected_random_between(&mut expected_random, 0.25, 0.75),
        -3.0 + expected_random_between(&mut expected_random, 0.1, 0.9),
    ];
    let vault_activation_with_connection = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: 3015,
            data: 0,
            ..level_event_packet(3015)
        },
        LevelEventParticleContext {
            vault_block_entity_at_event_pos: true,
            vault_connection_particles: Some(connection),
            ..LevelEventParticleContext::default()
        },
        &mut vault_connection_random,
    );
    assert_eq!(
        vault_activation_with_connection.len(),
        usize::try_from(expected_connection_count).unwrap() + 40
    );
    assert_particle_command(
        &vault_activation_with_connection.commands[0],
        VAULT_CONNECTION_PARTICLE_TYPE_ID,
        "minecraft:vault_connection",
        connection_origin,
        expected_connection_velocity,
        true,
    );
    assert_particle_command(
        &vault_activation_with_connection.commands
            [usize::try_from(expected_connection_count).unwrap()],
        SMOKE_PARTICLE_TYPE_ID,
        "minecraft:smoke",
        expected_smoke_position_after_connection,
        [0.0, 0.0, 0.0],
        false,
    );

    let mut ominous_vault_activation_random = LevelEventSoundRandomState::with_seed(0);
    let ominous_vault_activation = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: 3015,
            data: 1,
            ..level_event_packet(3015)
        },
        LevelEventParticleContext {
            vault_block_entity_at_event_pos: true,
            ..LevelEventParticleContext::default()
        },
        &mut ominous_vault_activation_random,
    );
    assert_eq!(ominous_vault_activation.len(), 40);
    assert_eq!(
        ominous_vault_activation.commands[1].particle_type_id,
        SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
    );
    assert_eq!(
        ominous_vault_activation.commands[1].particle_id,
        "minecraft:soul_fire_flame"
    );

    let (vault_deactivation_position, vault_deactivation_velocity) =
        first_vault_deactivation_particle();
    let mut vault_deactivation_random = LevelEventSoundRandomState::with_seed(0);
    let vault_deactivation = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3016,
            data: 0,
            ..level_event_packet(3016)
        },
        &mut vault_deactivation_random,
    );
    assert_eq!(vault_deactivation.len(), 20);
    assert_particle_command(
        &vault_deactivation.commands[0],
        SMALL_FLAME_PARTICLE_TYPE_ID,
        "minecraft:small_flame",
        vault_deactivation_position,
        vault_deactivation_velocity,
        false,
    );

    let mut ominous_vault_deactivation_random = LevelEventSoundRandomState::with_seed(0);
    let ominous_vault_deactivation = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3016,
            data: 1,
            ..level_event_packet(3016)
        },
        &mut ominous_vault_deactivation_random,
    );
    assert_eq!(ominous_vault_deactivation.len(), 20);
    assert_particle_command(
        &ominous_vault_deactivation.commands[0],
        SOUL_FIRE_FLAME_PARTICLE_TYPE_ID,
        "minecraft:soul_fire_flame",
        vault_deactivation_position,
        vault_deactivation_velocity,
        false,
    );

    let mut cobweb_poof_random = LevelEventSoundRandomState::with_seed(0);
    let cobweb_poof = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3018,
            ..level_event_packet(3018)
        },
        &mut cobweb_poof_random,
    );
    assert_eq!(cobweb_poof.len(), 10);
    assert_particle_command(
        &cobweb_poof.commands[0],
        59,
        "minecraft:poof",
        [
            10.597_545_277_797_202,
            64.333_218_399_476_65,
            -2.614_810_815_259_281_7,
        ],
        [
            0.016_050_661_274_780_612,
            -0.018_030_921_768_350_243,
            0.041_618_415_808_563_26,
        ],
        true,
    );

    let mut firework_resolver = test_resolver(0);
    let firework_poof =
        firework_resolver.firework_empty_explosion_particle_batch([10.0, 64.0, -3.0], None);
    assert_eq!(firework_poof.len(), 2);
    assert_particle_command(
        &firework_poof.commands[0],
        59,
        "minecraft:poof",
        [10.0, 64.0, -3.0],
        [0.057_302_703_614_493_14, 0.005, 0.018_385_983_023_454_71],
        true,
    );

    let mut firework_resolver = test_resolver(0);
    let firework_explosion = firework_resolver.firework_explosion_particle_batch(
        &FireworkRocketExplosionParticleState {
            entity_id: 7,
            position: bbb_world::EntityVec3 {
                x: 10.0,
                y: 64.0,
                z: -3.0,
            },
            delta_movement: bbb_world::EntityVec3 {
                x: 0.1,
                y: 0.2,
                z: -0.3,
            },
            has_explosions: true,
            explosions: vec![FireworkExplosionSummary {
                shape: FireworkExplosionShapeSummary::Star,
                colors: vec![0x112233, 0x445566],
                fade_colors: vec![0x778899],
                has_trail: true,
                has_twinkle: true,
            }],
        },
        None,
    );
    assert_eq!(firework_explosion.len(), 122);
    assert_eq!(firework_explosion.sound_events.len(), 1);
    let blast = &firework_explosion.sound_events[0];
    assert_eq!(blast.sound_event_id, FIREWORK_ROCKET_BLAST_SOUND_EVENT_ID);
    assert_eq!(blast.source, "ambient");
    assert_eq!(blast.position, [10.0, 64.0, -3.0]);
    assert_eq!(blast.volume, 20.0);
    let mut expected_starter_random = LegacyRandom::new(0);
    assert!(
        (blast.pitch - (0.95 + expected_starter_random.next_float() * 0.1)).abs() < f32::EPSILON
    );
    let mut expected_level_random = LegacyRandom::new(0);
    assert_eq!(blast.seed, expected_level_random.next_i64());
    assert!(blast.distance_delay);
    assert_eq!(firework_explosion.scheduled_sound_events.len(), 1);
    let twinkle = &firework_explosion.scheduled_sound_events[0];
    assert_eq!(
        twinkle.event.sound_event_id,
        FIREWORK_ROCKET_TWINKLE_SOUND_EVENT_ID
    );
    assert_eq!(twinkle.event.source, "ambient");
    assert_eq!(twinkle.event.position, [10.0, 64.0, -3.0]);
    assert_eq!(twinkle.event.volume, 20.0);
    assert!((0.9_f32..1.05_f32).contains(&twinkle.event.pitch));
    assert_eq!(twinkle.event.seed, expected_level_random.next_i64());
    assert!(twinkle.event.distance_delay);
    assert_eq!(twinkle.delay_ticks, 16);
    assert_eq!(
        twinkle.far_sound_event_id.as_deref(),
        Some(FIREWORK_ROCKET_TWINKLE_FAR_SOUND_EVENT_ID)
    );
    assert_eq!(twinkle.far_distance_squared, Some(256.0));
    assert_eq!(
        firework_explosion
            .commands
            .iter()
            .filter(|command| command.particle_id == "minecraft:firework")
            .count(),
        121
    );
    let first_spark = &firework_explosion.commands[0];
    assert_eq!(first_spark.particle_type_id, FIREWORK_PARTICLE_TYPE_ID);
    assert_eq!(first_spark.particle_id, "minecraft:firework");
    assert_eq!(first_spark.position, [10.0, 64.0, -3.0]);
    assert!(first_spark.option_firework_trail);
    assert!(first_spark.option_firework_twinkle);
    let first_spark_color = first_spark.option_color.unwrap();
    assert!([
        [
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            0.99,
        ],
        [
            0x44 as f32 / 255.0,
            0x55 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0.99,
        ],
    ]
    .contains(&first_spark_color));
    assert_eq!(
        first_spark.option_color_to,
        Some([
            0x77 as f32 / 255.0,
            0x88 as f32 / 255.0,
            0x99 as f32 / 255.0,
            1.0,
        ])
    );
    let flash = firework_explosion.commands.last().unwrap();
    assert_eq!(flash.particle_type_id, FLASH_PARTICLE_TYPE_ID);
    assert_eq!(flash.particle_id, "minecraft:flash");
    assert_eq!(
        flash.option_color,
        Some([
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            0.0,
        ])
    );

    let mut large_far_resolver = test_resolver(0);
    let large_far_sound = large_far_resolver.firework_blast_sound_event(
        &FireworkRocketExplosionParticleState {
            entity_id: 8,
            position: bbb_world::EntityVec3 {
                x: 10.0,
                y: 64.0,
                z: -3.0,
            },
            delta_movement: bbb_world::EntityVec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            has_explosions: true,
            explosions: vec![FireworkExplosionSummary {
                shape: FireworkExplosionShapeSummary::LargeBall,
                colors: vec![0xffffff],
                fade_colors: Vec::new(),
                has_trail: false,
                has_twinkle: false,
            }],
        },
        Some([10.0, 64.0, 14.0]),
    );
    assert_eq!(
        large_far_sound.sound_event_id,
        FIREWORK_ROCKET_LARGE_BLAST_FAR_SOUND_EVENT_ID
    );

    let mut bee_growth_random = LevelEventSoundRandomState::with_seed(0);
    let bee_growth = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2011,
            data: 3,
            ..level_event_packet(2011)
        },
        &mut bee_growth_random,
    );
    assert_eq!(bee_growth.len(), 3);
    assert_particle_command(
        &bee_growth.commands[0],
        43,
        "minecraft:happy_villager",
        [
            10.597_545_277_797_202,
            64.333_218_399_476_65,
            -2.614_810_815_259_281_7,
        ],
        [
            0.016_050_661_274_780_612,
            -0.018_030_921_768_350_243,
            0.041_618_415_808_563_26,
        ],
        false,
    );

    let mut bee_growth_half_height_random = LevelEventSoundRandomState::with_seed(0);
    let bee_growth_half_height = resolver.resolve_level_event_particles_with_context(
        &LevelEvent {
            event_type: 2011,
            data: 3,
            ..level_event_packet(2011)
        },
        LevelEventParticleContext {
            in_block_particle_spread_height: Some(0.5),
            ..LevelEventParticleContext::default()
        },
        &mut bee_growth_half_height_random,
    );
    assert_eq!(bee_growth_half_height.len(), 3);
    assert_particle_command(
        &bee_growth_half_height.commands[0],
        43,
        "minecraft:happy_villager",
        [
            10.597_545_277_797_202,
            64.166_609_199_738_33,
            -2.614_810_815_259_281_7,
        ],
        [
            0.016_050_661_274_780_612,
            -0.018_030_921_768_350_243,
            0.041_618_415_808_563_26,
        ],
        false,
    );

    let mut turtle_egg_placement_random = LevelEventSoundRandomState::with_seed(0);
    let turtle_egg_placement = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2012,
            data: 2,
            ..level_event_packet(2012)
        },
        &mut turtle_egg_placement_random,
    );
    assert_eq!(turtle_egg_placement.len(), 2);
    assert_eq!(turtle_egg_placement.commands[0].particle_type_id, 43);
    assert_eq!(
        turtle_egg_placement.commands[0].particle_id,
        "minecraft:happy_villager"
    );

    let mut zero_bee_growth_random = LevelEventSoundRandomState::with_seed(0);
    let zero_bee_growth = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2011,
            data: 0,
            ..level_event_packet(2011)
        },
        &mut zero_bee_growth_random,
    );
    assert!(zero_bee_growth.is_empty());

    let mut trial_detect_ominous_random = LevelEventSoundRandomState::with_seed(0);
    let trial_detect_ominous = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3019,
            data: 10,
            ..level_event_packet(3019)
        },
        &mut trial_detect_ominous_random,
    );
    assert_eq!(trial_detect_ominous.len(), 80);
    assert_eq!(trial_detect_ominous.commands[0].particle_type_id, 109);
    assert_eq!(
        trial_detect_ominous.commands[0].particle_id,
        "minecraft:trial_spawner_detection_ominous"
    );

    let mut trial_ominous_activate_random = LevelEventSoundRandomState::with_seed(0);
    let trial_ominous_activate = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3020,
            ..level_event_packet(3020)
        },
        &mut trial_ominous_activate_random,
    );
    assert_eq!(trial_ominous_activate.len(), 70);
    assert_eq!(trial_ominous_activate.commands[0].particle_type_id, 109);
    assert_particle_command(
        &trial_ominous_activate.commands[30],
        114,
        "minecraft:trial_omen",
        [
            11.208_974_334_084_582,
            63.519_346_994_601_946,
            -2.115_413_986_094_133_7,
        ],
        [
            0.019_195_505_076_083_332,
            0.015_047_723_904_287_527,
            -0.013_159_128_311_470_1,
        ],
        false,
    );
    assert_eq!(trial_ominous_activate.commands[31].particle_type_id, 40);
    assert_eq!(
        trial_ominous_activate.commands[31].particle_id,
        "minecraft:soul_fire_flame"
    );

    let mut trial_spawn_item_random = LevelEventSoundRandomState::with_seed(0);
    let trial_spawn_item = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 3021,
            data: 0,
            ..level_event_packet(3021)
        },
        &mut trial_spawn_item_random,
    );
    assert_eq!(trial_spawn_item.len(), 40);
    assert_eq!(trial_spawn_item.commands[1].particle_type_id, 32);

    let mut cloud_random = LevelEventSoundRandomState::with_seed(0);
    let cloud = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2009,
            ..level_event_packet(2009)
        },
        &mut cloud_random,
    );
    assert_eq!(cloud.len(), 8);
    assert_particle_command(
        &cloud.commands[0],
        4,
        "minecraft:cloud",
        [10.730_967_787_376_657, 65.2, -2.759_463_584_328_514],
        [0.0, 0.0, 0.0],
        false,
    );

    let mut dispenser_random = LevelEventSoundRandomState::with_seed(0);
    let dispenser = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2000,
            data: 5,
            ..level_event_packet(2000)
        },
        &mut dispenser_random,
    );
    assert_eq!(dispenser.len(), 10);
    assert_particle_command(
        &dispenser.commands[0],
        62,
        "minecraft:smoke",
        [11.11, 64.5, -2.474_781_497_441_183],
        [
            0.166_039_302_804_156_55,
            -0.016_834_122_587_673_427,
            -0.000_272_902_629_078_872_87,
        ],
        false,
    );

    let mut white_smoke_random = LevelEventSoundRandomState::with_seed(0);
    let white_smoke = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: 2010,
            data: 2,
            ..level_event_packet(2010)
        },
        &mut white_smoke_random,
    );
    assert_eq!(white_smoke.len(), 10);
    assert_particle_command(
        &white_smoke.commands[0],
        63,
        "minecraft:white_smoke",
        [10.629_731_792_164_257, 64.5, -3.11],
        [
            0.009_845_745_328_825_128,
            -0.016_834_122_587_673_427,
            -0.156_466_460_104_410_3,
        ],
        false,
    );
}

#[test]
fn level_event_item_break_particles_use_installed_item_sprite_ids() {
    let mut resolver = test_resolver(0);
    resolver.default_item_particle_sprite_ids.insert(
        VANILLA_SPLASH_POTION_ITEM_ID,
        vec!["minecraft:item/splash_potion".to_string()],
    );
    resolver.default_item_particle_sprite_ids.insert(
        VANILLA_ENDER_EYE_ITEM_ID,
        vec!["minecraft:item/ender_eye".to_string()],
    );
    let assert_item_sprites = |command: &ParticleSpawnCommand, item_id: i32, sprite_id: &str| {
        assert_eq!(command.particle_type_id, ITEM_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:item");
        assert_eq!(
            command.option_item,
            Some(ParticleItemOptionState {
                item_id,
                count: 1,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            })
        );
        assert_eq!(command.sprite_ids, vec![sprite_id.to_string()]);
    };

    let mut potion_random = LevelEventSoundRandomState::with_seed(0);
    let potion = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: POTION_BREAK_LEVEL_EVENT,
            data: 0x0033_66cc,
            ..level_event_packet(POTION_BREAK_LEVEL_EVENT)
        },
        &mut potion_random,
    );
    assert!(potion.len() >= POTION_BREAK_ITEM_PARTICLE_COUNT as usize);
    for command in potion
        .commands
        .iter()
        .take(POTION_BREAK_ITEM_PARTICLE_COUNT as usize)
    {
        assert_item_sprites(
            command,
            VANILLA_SPLASH_POTION_ITEM_ID,
            "minecraft:item/splash_potion",
        );
    }

    let mut instant_potion_random = LevelEventSoundRandomState::with_seed(0);
    let instant_potion = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: INSTANT_POTION_BREAK_LEVEL_EVENT,
            data: 0x00ff_cc00,
            ..level_event_packet(INSTANT_POTION_BREAK_LEVEL_EVENT)
        },
        &mut instant_potion_random,
    );
    assert!(instant_potion.len() >= POTION_BREAK_ITEM_PARTICLE_COUNT as usize);
    for command in instant_potion
        .commands
        .iter()
        .take(POTION_BREAK_ITEM_PARTICLE_COUNT as usize)
    {
        assert_item_sprites(
            command,
            VANILLA_SPLASH_POTION_ITEM_ID,
            "minecraft:item/splash_potion",
        );
    }

    let mut ender_eye_random = LevelEventSoundRandomState::with_seed(0);
    let ender_eye = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: ENDER_EYE_BREAK_LEVEL_EVENT,
            ..level_event_packet(ENDER_EYE_BREAK_LEVEL_EVENT)
        },
        &mut ender_eye_random,
    );
    assert!(ender_eye.len() >= ITEM_BREAK_PARTICLE_COUNT as usize);
    for command in ender_eye
        .commands
        .iter()
        .take(ITEM_BREAK_PARTICLE_COUNT as usize)
    {
        assert_item_sprites(
            command,
            VANILLA_ENDER_EYE_ITEM_ID,
            "minecraft:item/ender_eye",
        );
    }
}

#[test]
fn level_event_destroy_block_particles_use_block_particle_options() {
    let resolver = test_resolver(0);
    let stone_id = test_block_state_id("minecraft:stone", []);
    let bottom_slab_id = test_block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
    let structure_void_id = test_block_state_id("minecraft:structure_void", []);
    let moving_piston_id = test_block_state_id(
        "minecraft:moving_piston",
        [("facing", "north"), ("type", "normal")],
    );
    let event = LevelEvent {
        event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
        data: stone_id,
        ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
    };
    let mut random = LevelEventSoundRandomState::with_seed(0);

    let batch = resolver.resolve_level_event_particles(&event, &mut random);

    assert_eq!(batch.len(), 64);
    assert_eq!(batch.missing_definition_count, 0);
    assert_eq!(batch.missing_sprite_count, 0);
    assert_block_destroy_particle_command(
        &batch.commands[0],
        stone_id,
        [10.125, 64.125, -2.875],
        [-0.375, -0.375, -0.375],
    );
    assert_block_destroy_particle_command(
        &batch.commands[63],
        stone_id,
        [10.875, 64.875, -2.125],
        [0.375, 0.375, 0.375],
    );

    let mut brush_random = LevelEventSoundRandomState::with_seed(0);
    let brush = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: BRUSH_BLOCK_COMPLETE_LEVEL_EVENT,
            data: stone_id,
            ..level_event_packet(BRUSH_BLOCK_COMPLETE_LEVEL_EVENT)
        },
        &mut brush_random,
    );
    assert_eq!(brush.len(), 64);
    assert_block_destroy_particle_command(
        &brush.commands[0],
        stone_id,
        [10.125, 64.125, -2.875],
        [-0.375, -0.375, -0.375],
    );

    let mut slab_random = LevelEventSoundRandomState::with_seed(0);
    let slab = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
            data: bottom_slab_id,
            ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
        },
        &mut slab_random,
    );
    assert_eq!(slab.len(), 32);
    assert_block_destroy_particle_command(
        &slab.commands[0],
        bottom_slab_id,
        [10.125, 64.125, -2.875],
        [-0.375, -0.25, -0.375],
    );
    assert_block_destroy_particle_command(
        &slab.commands[31],
        bottom_slab_id,
        [10.875, 64.375, -2.125],
        [0.375, 0.25, 0.375],
    );

    let mut air_random = LevelEventSoundRandomState::with_seed(0);
    let air = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
            data: AIR_BLOCK_STATE_ID,
            ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
        },
        &mut air_random,
    );
    assert!(air.is_empty());

    for block_state_id in [barrier_id, structure_void_id] {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let batch = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                data: block_state_id,
                ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
            },
            &mut random,
        );
        assert!(batch.is_empty(), "{block_state_id}");
    }

    let mut moving_piston_random = LevelEventSoundRandomState::with_seed(0);
    let moving_piston = resolver.resolve_level_event_particles(
        &LevelEvent {
            event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
            data: moving_piston_id,
            ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
        },
        &mut moving_piston_random,
    );
    assert_eq!(moving_piston.len(), 64);
    assert_block_destroy_particle_command(
        &moving_piston.commands[0],
        moving_piston_id,
        [10.125, 64.125, -2.875],
        [-0.375, -0.375, -0.375],
    );
}

#[test]
fn level_event_destroy_block_particles_use_event_pos_terrain_tint() {
    let mut resolver = test_resolver(0);
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        test_biome_color_profile(7, [10, 20, 30], [40, 50, 60], [70, 80, 90], [1, 2, 3]),
    ]));
    resolver.set_terrain_particle_sprite_ids(&textures);
    let short_grass_id = test_block_state_id("minecraft:short_grass", []);
    let event = LevelEvent {
        event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
        data: short_grass_id,
        ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
    };
    let mut random = LevelEventSoundRandomState::with_seed(0);

    let batch = resolver.resolve_level_event_particles_with_context(
        &event,
        LevelEventParticleContext {
            biome_id_at_event_pos: Some(7),
            ..LevelEventParticleContext::default()
        },
        &mut random,
    );

    assert!(!batch.is_empty());
    assert_eq!(
        batch.commands[0].option_color,
        Some(rgb_option_06(10, 20, 30))
    );
    assert_eq!(
        batch
            .commands
            .last()
            .and_then(|command| command.option_color),
        Some(rgb_option_06(10, 20, 30))
    );
}

#[test]
fn level_event_smash_attack_particles_use_vanilla_dust_pillar_context() {
    let resolver = test_resolver(0);
    let event = LevelEvent {
        event_type: SMASH_ATTACK_PARTICLES_LEVEL_EVENT,
        data: 6,
        ..level_event_packet(SMASH_ATTACK_PARTICLES_LEVEL_EVENT)
    };
    let context = LevelEventParticleContext {
        block_state_id_at_event_pos: Some(9),
        ..LevelEventParticleContext::default()
    };
    let mut random = LevelEventSoundRandomState::with_seed(0);

    let batch =
        resolver.resolve_level_event_particles_with_context(&event, context.clone(), &mut random);

    let expected = expected_smash_attack_particles(event.data);
    assert_eq!(batch.len(), 6);
    assert_eq!(batch.len(), expected.len());
    assert_eq!(batch.missing_sprite_count, 0);
    assert_particle_command(
        &batch.commands[0],
        DUST_PILLAR_PARTICLE_TYPE_ID,
        "minecraft:dust_pillar",
        expected[0].0,
        expected[0].1,
        false,
    );
    assert_eq!(batch.commands[0].sprite_ids, Vec::<String>::new());
    assert_particle_command(
        &batch.commands[2],
        DUST_PILLAR_PARTICLE_TYPE_ID,
        "minecraft:dust_pillar",
        expected[2].0,
        expected[2].1,
        false,
    );
    for command in &batch.commands {
        assert_eq!(
            command.option_block,
            Some(ParticleBlockOptionState { block_state_id: 9 })
        );
        assert_eq!(command.option_item, None);
    }

    let mut rejected_random = LevelEventSoundRandomState::with_seed(0);
    let rejected = resolver.resolve_level_event_particles_with_context(
        &LevelEvent { data: 1, ..event },
        LevelEventParticleContext::default(),
        &mut rejected_random,
    );
    assert!(rejected.is_empty());

    let mut accepted_random = LevelEventSoundRandomState::with_seed(0);
    let accepted = resolver.resolve_level_event_particles_with_context(
        &LevelEvent { data: 1, ..event },
        context,
        &mut accepted_random,
    );
    assert_eq!(accepted.len(), 2);

    let cloud_event = LevelEvent {
        event_type: SPLASH_CLOUD_LEVEL_EVENT,
        ..level_event_packet(SPLASH_CLOUD_LEVEL_EVENT)
    };
    let rejected_cloud = resolver.resolve_level_event_particles(&cloud_event, &mut rejected_random);
    let accepted_cloud = resolver.resolve_level_event_particles(&cloud_event, &mut accepted_random);
    assert_eq!(rejected_cloud.len(), 8);
    assert_eq!(accepted_cloud.len(), 8);
    assert_eq!(
        rejected_cloud.commands[0].position,
        accepted_cloud.commands[0].position
    );
    assert_eq!(
        rejected_cloud.commands[0].velocity,
        accepted_cloud.commands[0].velocity
    );
}

#[test]
fn level_event_particle_resolver_covers_vanilla_26_1_particle_events() {
    let resolver = test_resolver(0);
    let stone_id = test_block_state_id("minecraft:stone", []);
    let cases = [
        (COMPOSTER_FILL_LEVEL_EVENT, 0, "composter fill"),
        (LAVA_EXTINGUISH_LEVEL_EVENT, 0, "lava extinguish"),
        (
            REDSTONE_TORCH_BURNOUT_LEVEL_EVENT,
            0,
            "redstone torch burnout",
        ),
        (
            END_PORTAL_FRAME_FILL_LEVEL_EVENT,
            0,
            "end portal frame fill",
        ),
        (DRIPSTONE_DRIP_LEVEL_EVENT, 0, "pointed dripstone drip"),
        (PLANT_GROWTH_LEVEL_EVENT, 2, "plant growth"),
        (DISPENSER_SMOKE_LEVEL_EVENT, 0, "dispenser smoke"),
        (
            DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
            stone_id,
            "destroy block",
        ),
        (POTION_BREAK_LEVEL_EVENT, 0x0033_66cc, "potion break"),
        (
            INSTANT_POTION_BREAK_LEVEL_EVENT,
            0x0033_66cc,
            "instant potion break",
        ),
        (ENDER_EYE_BREAK_LEVEL_EVENT, 0, "ender eye break"),
        (BLAZE_SMOKE_LEVEL_EVENT, 0, "blaze smoke"),
        (
            DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT,
            0,
            "dragon fireball explode",
        ),
        (EXPLOSION_LEVEL_EVENT, 0, "explosion"),
        (SPLASH_CLOUD_LEVEL_EVENT, 0, "splash cloud"),
        (
            DISPENSER_WHITE_SMOKE_LEVEL_EVENT,
            0,
            "dispenser white smoke",
        ),
        (BEE_GROWTH_PARTICLES_LEVEL_EVENT, 1, "bee growth"),
        (
            TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT,
            1,
            "turtle egg placement",
        ),
        (SMASH_ATTACK_PARTICLES_LEVEL_EVENT, 3, "smash attack"),
        (END_GATEWAY_SPAWN_LEVEL_EVENT, 0, "end gateway spawn"),
        (ELECTRIC_SPARK_LEVEL_EVENT, 0, "electric spark"),
        (WAX_ON_LEVEL_EVENT, 0, "wax on"),
        (WAX_OFF_LEVEL_EVENT, 0, "wax off"),
        (SCRAPE_LEVEL_EVENT, 0, "scrape"),
        (SCULK_CHARGE_LEVEL_EVENT, 2 << 6, "sculk charge"),
        (SCULK_SHRIEK_PARTICLES_LEVEL_EVENT, 0, "sculk shriek"),
        (
            BRUSH_BLOCK_COMPLETE_LEVEL_EVENT,
            stone_id,
            "brush block complete",
        ),
        (EGG_CRACK_LEVEL_EVENT, 0, "egg crack"),
        (
            TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT,
            0,
            "trial spawner spawn particles",
        ),
        (
            TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT,
            1,
            "trial spawner spawn mob",
        ),
        (
            TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT,
            2,
            "trial spawner detect player",
        ),
        (
            TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT,
            0,
            "trial spawner eject item",
        ),
        (VAULT_ACTIVATE_LEVEL_EVENT, 0, "vault activate"),
        (VAULT_DEACTIVATE_LEVEL_EVENT, 0, "vault deactivate"),
        (
            TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT,
            0,
            "trial spawner eject item particles",
        ),
        (COBWEB_PLACE_PARTICLES_LEVEL_EVENT, 0, "cobweb place"),
        (
            TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT,
            2,
            "ominous trial spawner detect player",
        ),
        (
            TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT,
            1,
            "trial spawner ominous activate",
        ),
        (
            TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT,
            1,
            "trial spawner spawn item",
        ),
    ];

    for (event_type, data, label) in cases {
        let event = LevelEvent {
            event_type,
            data,
            ..level_event_packet(event_type)
        };
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let batch = resolver.resolve_level_event_particles_with_context(
            &event,
            representative_level_event_particle_context(&event, stone_id),
            &mut random,
        );

        assert!(
            !batch.commands.is_empty(),
            "vanilla LevelEvent particle case {event_type} ({label}) must be mapped"
        );
        assert_eq!(batch.missing_definition_count, 0, "{label}");
        assert_eq!(batch.missing_sprite_count, 0, "{label}");
        assert_eq!(batch.unknown_particle_type_count, 0, "{label}");
    }
}

#[test]
fn direction_normal_from_3d_data_value_matches_vanilla_wrapping() {
    assert_eq!(direction_normal_from_3d_data_value(0), (0, -1, 0));
    assert_eq!(direction_normal_from_3d_data_value(1), (0, 1, 0));
    assert_eq!(direction_normal_from_3d_data_value(2), (0, 0, -1));
    assert_eq!(direction_normal_from_3d_data_value(3), (0, 0, 1));
    assert_eq!(direction_normal_from_3d_data_value(4), (-1, 0, 0));
    assert_eq!(direction_normal_from_3d_data_value(5), (1, 0, 0));
    assert_eq!(direction_normal_from_3d_data_value(7), (0, 1, 0));
    assert_eq!(direction_normal_from_3d_data_value(-1), (0, 1, 0));
}

#[test]
fn particle_atlas_from_images_exports_renderer_uvs() {
    let image = SpriteImage::new(
        "minecraft:generic_0",
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .unwrap();

    let atlas = particle_atlas_from_images(vec![image]).unwrap();

    assert_eq!(atlas.width, 4);
    assert_eq!(atlas.height, 4);
    assert_eq!(atlas.rgba.len(), 4 * 4 * 4);
    assert_eq!(
        atlas.sprite_uvs,
        vec![ParticleSpriteUv {
            id: "minecraft:generic_0".to_string(),
            uv: ParticleUvRect {
                min: [0.375, 0.375],
                max: [0.625, 0.625],
            },
            has_translucent: false,
        }]
    );
}

#[test]
fn particle_atlas_animation_frame_uses_sprite_animation_tick() {
    let mut image = SpriteImage::new("minecraft:vibration", 1, 1, vec![10, 0, 0, 255]).unwrap();
    image.animation = Some(SpriteAnimation {
        frame_count: 2,
        default_frame_time: 1,
        interpolate: false,
        frames: vec![
            SpriteAnimationFrame { index: 0, time: 2 },
            SpriteAnimationFrame { index: 1, time: 1 },
        ],
    });
    image.animation_frames_rgba = vec![vec![10, 0, 0, 255], vec![20, 0, 0, 255]];

    let atlas = particle_atlas_from_images(vec![image]).unwrap();
    let tick_zero = atlas.animation_atlas_frame(0).unwrap().unwrap();
    let tick_two = atlas.animation_atlas_frame(2).unwrap().unwrap();

    assert!(atlas.has_animation());
    assert_eq!(
        (tick_zero.width, tick_zero.height),
        (atlas.width, atlas.height)
    );
    assert_eq!(
        (tick_two.width, tick_two.height),
        (atlas.width, atlas.height)
    );
    assert_eq!(
        atlas_pixel(&tick_zero.rgba, tick_zero.width, 1, 1),
        [10, 0, 0, 255]
    );
    assert_eq!(
        atlas_pixel(&tick_two.rgba, tick_two.width, 1, 1),
        [20, 0, 0, 255]
    );
}

#[test]
fn particle_texture_animation_tick_advances_at_vanilla_interval() {
    let atlas = particle_atlas_from_images(vec![SpriteImage::new(
        "minecraft:generic_0",
        1,
        1,
        vec![10, 0, 0, 255],
    )
    .unwrap()])
    .unwrap();
    let mut runtime = NativeParticleRuntime {
        resolver: test_resolver(0),
        atlas,
        texture_animation_tick: 0,
        last_texture_animation_at: None,
    };
    let start = Instant::now();

    assert_eq!(
        advance_particle_texture_animation_tick(&mut runtime, start),
        None
    );
    assert_eq!(
        advance_particle_texture_animation_tick(
            &mut runtime,
            start + PARTICLE_TEXTURE_ANIMATION_INTERVAL - Duration::from_millis(1),
        ),
        None
    );
    assert_eq!(
        advance_particle_texture_animation_tick(
            &mut runtime,
            start + PARTICLE_TEXTURE_ANIMATION_INTERVAL * 3,
        ),
        Some(3)
    );
    assert_eq!(runtime.texture_animation_tick, 3);
}

fn atlas_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = usize::try_from((y * width + x) * 4).unwrap();
    rgba[offset..offset + 4].try_into().unwrap()
}

fn test_resolver(seed: i64) -> ParticleCommandResolver {
    test_resolver_with_particle_status(seed, ClientParticleStatus::All)
}

fn representative_level_event_particle_context(
    event: &LevelEvent,
    block_state_id: i32,
) -> LevelEventParticleContext {
    match event.event_type {
        DRIPSTONE_DRIP_LEVEL_EVENT => LevelEventParticleContext {
            dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Water),
            ..LevelEventParticleContext::default()
        },
        PLANT_GROWTH_LEVEL_EVENT => LevelEventParticleContext {
            growth_particles: Some(LevelEventGrowthParticleContext {
                pos: event.pos,
                mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
            }),
            ..LevelEventParticleContext::default()
        },
        SMASH_ATTACK_PARTICLES_LEVEL_EVENT => LevelEventParticleContext {
            block_state_id_at_event_pos: Some(block_state_id),
            ..LevelEventParticleContext::default()
        },
        VAULT_ACTIVATE_LEVEL_EVENT => LevelEventParticleContext {
            vault_block_entity_at_event_pos: true,
            ..LevelEventParticleContext::default()
        },
        _ => LevelEventParticleContext::default(),
    }
}

fn test_resolver_with_particle_status(
    seed: i64,
    particle_status: ClientParticleStatus,
) -> ParticleCommandResolver {
    test_resolver_with_cloud_textures(
        seed,
        particle_status,
        &["minecraft:generic_7", "minecraft:generic_6"],
        &[
            "generic_7",
            "generic_6",
            "generic_0",
            "generic_1",
            "generic_2",
            "generic_3",
            "generic_4",
            "generic_5",
            "effect_0",
            "spell_0",
            "tinted_leaf_0",
            "dragon_breath_0",
            "flash",
            "vibration",
            "sculk_charge_0",
            "sculk_charge_pop_0",
            "flame",
            "soul_fire_flame",
            "explosion_0",
            "firework_0",
            "firework_1",
            "gust_0",
            "ominous_spawning_0",
            "smoke_0",
            "large_smoke_0",
            "lava",
            "white_smoke_0",
            "dripping_dripstone_lava",
            "dripping_dripstone_water",
            "bubble_0",
            "poof_0",
            "angry_villager_0",
            "happy_villager_0",
            "heart_0",
            "composter_0",
            "totem_0",
            "witch_0",
            "small_flame",
            "electric_spark_0",
            "wax_on_0",
            "wax_off_0",
            "scrape_0",
            "shriek_0",
            "egg_crack_0",
            "trial_spawner_detection_0",
            "trial_spawner_detection_ominous_0",
            "trial_omen_0",
        ],
    )
}

fn test_resolver_with_cloud_textures(
    seed: i64,
    particle_status: ClientParticleStatus,
    cloud_textures: &[&str],
    particle_textures: &[&str],
) -> ParticleCommandResolver {
    let root = unique_temp_dir("particle-runtime");
    let assets_dir = assets_dir(&root);
    write_particle_atlas(&assets_dir);
    for texture in particle_textures {
        write_test_png(
            &assets_dir
                .join("textures")
                .join("particle")
                .join(format!("{texture}.png")),
            8,
            8,
        );
    }
    write_json(
        &particle_dir(&root).join("cloud.json"),
        &particle_definition_json(cloud_textures),
    );
    write_json(
        &particle_dir(&root).join("flame.json"),
        r#"{
          "textures": [
            "minecraft:flame"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("effect.json"),
        r#"{
          "textures": [
            "minecraft:effect_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("entity_effect.json"),
        r#"{
          "textures": [
            "minecraft:effect_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("instant_effect.json"),
        r#"{
          "textures": [
            "minecraft:spell_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("tinted_leaves.json"),
        r#"{
          "textures": [
            "minecraft:tinted_leaf_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("dragon_breath.json"),
        r#"{
          "textures": [
            "minecraft:dragon_breath_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("gust.json"),
        r#"{
          "textures": [
            "minecraft:gust_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("flash.json"),
        r#"{
          "textures": [
            "minecraft:flash"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("vibration.json"),
        r#"{
          "textures": [
            "minecraft:vibration"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("dust.json"),
        r#"{
          "textures": [
            "minecraft:generic_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("dust_color_transition.json"),
        r#"{
          "textures": [
            "minecraft:generic_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("falling_dust.json"),
        r#"{
          "textures": [
            "minecraft:generic_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("firework.json"),
        r#"{
          "textures": [
            "minecraft:firework_0",
            "minecraft:firework_1"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("trail.json"),
        r#"{
          "textures": [
            "minecraft:generic_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("sculk_charge.json"),
        r#"{
          "textures": [
            "minecraft:sculk_charge_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("sculk_charge_pop.json"),
        r#"{
          "textures": [
            "minecraft:sculk_charge_pop_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("soul_fire_flame.json"),
        r#"{
          "textures": [
            "minecraft:soul_fire_flame"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("explosion.json"),
        r#"{
          "textures": [
            "minecraft:explosion_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("smoke.json"),
        r#"{
          "textures": [
            "minecraft:smoke_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("large_smoke.json"),
        r#"{
          "textures": [
            "minecraft:large_smoke_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("lava.json"),
        r#"{
          "textures": [
            "minecraft:lava"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("white_smoke.json"),
        r#"{
          "textures": [
            "minecraft:white_smoke_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("dripping_dripstone_lava.json"),
        r#"{
          "textures": [
            "minecraft:dripping_dripstone_lava"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("dripping_dripstone_water.json"),
        r#"{
          "textures": [
            "minecraft:dripping_dripstone_water"
          ]
        }"#,
    );
    for particle_name in [
        "dripping_lava",
        "falling_lava",
        "landing_lava",
        "dripping_water",
        "falling_water",
        "splash",
        "dripping_honey",
        "falling_honey",
        "landing_honey",
        "dripping_obsidian_tear",
        "falling_obsidian_tear",
        "landing_obsidian_tear",
        "falling_dripstone_lava",
        "falling_dripstone_water",
    ] {
        write_json(
            &particle_dir(&root).join(format!("{particle_name}.json")),
            r#"{
              "textures": [
                "minecraft:generic_0"
              ]
            }"#,
        );
    }
    write_json(
        &particle_dir(&root).join("bubble.json"),
        r#"{
          "textures": [
            "minecraft:bubble_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("poof.json"),
        r#"{
          "textures": [
            "minecraft:poof_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("portal.json"),
        r#"{
          "textures": [
            "minecraft:generic_0",
            "minecraft:generic_1",
            "minecraft:generic_2",
            "minecraft:generic_3",
            "minecraft:generic_4",
            "minecraft:generic_5",
            "minecraft:generic_6",
            "minecraft:generic_7"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("angry_villager.json"),
        r#"{
          "textures": [
            "minecraft:angry_villager_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("happy_villager.json"),
        r#"{
          "textures": [
            "minecraft:happy_villager_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("heart.json"),
        r#"{
          "textures": [
            "minecraft:heart_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("composter.json"),
        r#"{
          "textures": [
            "minecraft:composter_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("totem_of_undying.json"),
        r#"{
          "textures": [
            "minecraft:totem_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("witch.json"),
        r#"{
          "textures": [
            "minecraft:witch_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("small_flame.json"),
        r#"{
          "textures": [
            "minecraft:small_flame"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("electric_spark.json"),
        r#"{
          "textures": [
            "minecraft:electric_spark_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("wax_on.json"),
        r#"{
          "textures": [
            "minecraft:wax_on_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("wax_off.json"),
        r#"{
          "textures": [
            "minecraft:wax_off_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("scrape.json"),
        r#"{
          "textures": [
            "minecraft:scrape_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("shriek.json"),
        r#"{
          "textures": [
            "minecraft:shriek_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("egg_crack.json"),
        r#"{
          "textures": [
            "minecraft:egg_crack_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("trial_spawner_detection.json"),
        r#"{
          "textures": [
            "minecraft:trial_spawner_detection_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("trial_spawner_detection_ominous.json"),
        r#"{
          "textures": [
            "minecraft:trial_spawner_detection_ominous_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("vault_connection.json"),
        r#"{
          "textures": [
            "minecraft:vault_connection_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("ominous_spawning.json"),
        r#"{
          "textures": [
            "minecraft:ominous_spawning_0"
          ]
        }"#,
    );
    write_json(
        &particle_dir(&root).join("trial_omen.json"),
        r#"{
          "textures": [
            "minecraft:trial_omen_0"
          ]
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_particle_definition_catalog()
        .unwrap();
    let sprites = PackRoots::from_root(&root)
        .unwrap()
        .load_particle_sprite_catalog()
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    ParticleCommandResolver::with_seed_and_particle_status(catalog, sprites, seed, particle_status)
}

fn level_particles_packet(particle_type_id: i32, count: i32) -> LevelParticles {
    LevelParticles {
        override_limiter: true,
        always_show: true,
        position: Vec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: Vec3d {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        },
        max_speed: 1.5,
        count,
        particle: bbb_protocol::packets::ParticlePayload {
            particle_type_id,
            raw_options: vec![0xaa, 0xbb],
        },
    }
}

fn spell_particle_options(color: i32, power: f32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&color.to_be_bytes());
    out.extend_from_slice(&power.to_be_bytes());
    out
}

fn dust_particle_options(color: i32, scale: f32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&color.to_be_bytes());
    out.extend_from_slice(&scale.to_be_bytes());
    out
}

fn dust_color_transition_options(from_color: i32, to_color: i32, scale: f32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&from_color.to_be_bytes());
    out.extend_from_slice(&to_color.to_be_bytes());
    out.extend_from_slice(&scale.to_be_bytes());
    out
}

fn block_particle_options(block_state_id: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_positive_var_i32(&mut out, block_state_id);
    out
}

fn item_particle_options(item_id: i32, count: i32, added_components: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_positive_var_i32(&mut out, item_id);
    write_positive_var_i32(&mut out, count);
    write_positive_var_i32(&mut out, added_components);
    write_positive_var_i32(&mut out, 0);
    for _ in 0..added_components {
        write_positive_var_i32(&mut out, 10);
        write_string(&mut out, "minecraft:alternate_model_component");
    }
    out
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_positive_var_i32(out, value.len() as i32);
    out.extend_from_slice(value.as_bytes());
}

fn trail_particle_options(target: [f64; 3], color: i32, duration: i32) -> Vec<u8> {
    let mut out = Vec::new();
    for coordinate in target {
        out.extend_from_slice(&coordinate.to_be_bytes());
    }
    out.extend_from_slice(&color.to_be_bytes());
    write_positive_var_i32(&mut out, duration);
    out
}

fn vibration_particle_block_options(pos: [i32; 3], arrival_ticks: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_positive_var_i32(&mut out, 0);
    out.extend_from_slice(&encode_test_block_pos(pos).to_be_bytes());
    write_positive_var_i32(&mut out, arrival_ticks);
    out
}

fn vibration_particle_entity_options(entity_id: i32, y_offset: f32, arrival_ticks: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_positive_var_i32(&mut out, 1);
    write_positive_var_i32(&mut out, entity_id);
    out.extend_from_slice(&y_offset.to_be_bytes());
    write_positive_var_i32(&mut out, arrival_ticks);
    out
}

fn encode_test_block_pos(pos: [i32; 3]) -> i64 {
    (((pos[0] as i64) & 0x3ffffff) << 38)
        | (((pos[2] as i64) & 0x3ffffff) << 12)
        | ((pos[1] as i64) & 0xfff)
}

fn write_positive_var_i32(out: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        if value & !0x7f == 0 {
            out.push(value as u8);
            return;
        }
        out.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
}

fn level_event_packet(event_type: i32) -> LevelEvent {
    LevelEvent {
        event_type,
        pos: bbb_protocol::packets::BlockPos {
            x: 10,
            y: 64,
            z: -3,
        },
        data: 0,
        global: false,
    }
}

fn test_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = props
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    bbb_world::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing test block state {name} {properties:?}"))
        .id
}

fn first_composter_fill_particle(center_shape_max_y: f64) -> ([f64; 3], [f64; 3]) {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let center_height = center_shape_max_y + COMPOSTER_FILL_CENTER_HEIGHT_OFFSET;
    let velocity = [
        random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
        random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
        random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
    ];
    let position = [
        10.0 + COMPOSTER_FILL_SIDE_OFFSET + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
        64.0 + center_height + f64::from(random.next_float()) * (1.0 - center_height),
        -3.0 + COMPOSTER_FILL_SIDE_OFFSET + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
    ];
    (position, velocity)
}

fn first_potion_break_spell_particle(data: i32) -> ([f64; 3], [f64; 3], [f32; 4], f32) {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    for _ in 0..POTION_BREAK_ITEM_PARTICLE_COUNT {
        random.next_gaussian();
        random.next_double();
        random.next_gaussian();
    }
    let dist = random.next_double() * 4.0;
    let angle = random.next_double() * std::f64::consts::TAU;
    let velocity = [
        angle.cos() * dist,
        0.01 + random.next_double() * 0.5,
        angle.sin() * dist,
    ];
    let random_brightness = 0.75 + random.next_float() * 0.25;
    let red = ((data >> 16) & 0xFF) as f32 / 255.0;
    let green = ((data >> 8) & 0xFF) as f32 / 255.0;
    let blue = (data & 0xFF) as f32 / 255.0;
    (
        [10.5 + velocity[0] * 0.1, 64.3, -2.5 + velocity[2] * 0.1],
        velocity,
        [
            red * random_brightness,
            green * random_brightness,
            blue * random_brightness,
            1.0,
        ],
        dist as f32,
    )
}

fn first_item_break_particle_velocity(seed: i64) -> [f64; 3] {
    let mut random = LevelEventSoundRandomState::with_seed(seed);
    [
        random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
        random.next_double() * ITEM_BREAK_VERTICAL_VELOCITY_SCALE,
        random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
    ]
}

fn first_vault_deactivation_particle() -> ([f64; 3], [f64; 3]) {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    (
        [
            10.0 + expected_random_between(&mut random, 0.4, 0.6),
            64.0 + expected_random_between(&mut random, 0.4, 0.6),
            -3.0 + expected_random_between(&mut random, 0.4, 0.6),
        ],
        [
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
        ],
    )
}

fn first_vault_activation_particle() -> [f64; 3] {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    [
        10.0 + expected_random_between(&mut random, 0.1, 0.9),
        64.0 + expected_random_between(&mut random, 0.25, 0.75),
        -3.0 + expected_random_between(&mut random, 0.1, 0.9),
    ]
}

fn first_growth_wide_particle(pos: BlockPos) -> ([f64; 3], [f64; 3]) {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let velocity = [
        random.next_gaussian() * 0.02,
        random.next_gaussian() * 0.02,
        random.next_gaussian() * 0.02,
    ];
    let position = [
        f64::from(pos.x)
            + GROWTH_PARTICLE_WIDE_START_OFFSET
            + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
        f64::from(pos.y) + random.next_double() * GROWTH_PARTICLE_WIDE_HEIGHT,
        f64::from(pos.z)
            + GROWTH_PARTICLE_WIDE_START_OFFSET
            + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
    ];
    (position, velocity)
}

fn expected_smash_attack_particles(count: i32) -> Vec<([f64; 3], [f64; 3])> {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let center = [10.5, 65.0, -2.5];
    let mut particles = Vec::new();

    for _ in 0..smash_attack_particle_loop_count(count, 3.0) {
        particles.push((
            [
                center[0] + random.next_gaussian() / 2.0,
                center[1],
                center[2] + random.next_gaussian() / 2.0,
            ],
            [
                random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
            ],
        ));
    }

    for i in 0..smash_attack_particle_loop_count(count, 1.5) {
        let angle = i as f64;
        particles.push((
            [
                center[0] + 3.5 * angle.cos() + random.next_gaussian() / 2.0,
                center[1],
                center[2] + 3.5 * angle.sin() + random.next_gaussian() / 2.0,
            ],
            [
                random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
            ],
        ));
    }

    particles
}

#[derive(Debug)]
struct ExpectedSculkChargeParticle {
    direction: (i32, i32, i32),
    position: [f64; 3],
    velocity: [f64; 3],
    roll: f32,
}

#[derive(Debug)]
struct ExpectedSculkChargePopParticle {
    position: [f64; 3],
    velocity: [f64; 3],
}

fn expected_sculk_charge_particles(data: i32) -> Vec<ExpectedSculkChargeParticle> {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let count = data >> 6;
    if count <= 0 {
        return Vec::new();
    }

    let mut particles = Vec::new();
    let particle_data = data & 63;
    if particle_data == 0 {
        for direction in BLOCK_FACE_DIRECTIONS {
            let roll = if *direction == BLOCK_FACE_DIRECTION_DOWN {
                std::f32::consts::PI
            } else {
                0.0
            };
            let step_factor = if direction.1 != 0 {
                SCULK_CHARGE_FULL_BLOCK_Y_FACTOR
            } else {
                SCULK_CHARGE_FULL_BLOCK_SIDE_FACTOR
            };
            append_expected_sculk_charge_face_particles(
                &mut particles,
                *direction,
                step_factor,
                roll,
                count,
                &mut random,
            );
        }
    } else {
        for (direction_index, direction) in BLOCK_FACE_DIRECTIONS.iter().enumerate() {
            if particle_data & (1 << direction_index) == 0 {
                continue;
            }
            let roll = if *direction == BLOCK_FACE_DIRECTION_UP {
                std::f32::consts::PI
            } else {
                0.0
            };
            append_expected_sculk_charge_face_particles(
                &mut particles,
                *direction,
                SCULK_CHARGE_MULTIFACE_FACTOR,
                roll,
                count,
                &mut random,
            );
        }
    }
    particles
}

fn append_expected_sculk_charge_face_particles(
    particles: &mut Vec<ExpectedSculkChargeParticle>,
    direction: (i32, i32, i32),
    step_factor: f64,
    roll: f32,
    count: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let particle_count = random.next_int_bound(count + 1);
    for _ in 0..particle_count {
        let speed = [
            expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
        ];
        let position = expected_block_face_position(direction, step_factor, random);
        let velocity = [
            if direction.0 == 0 { speed[0] } else { 0.0 },
            if direction.1 == 0 { speed[1] } else { 0.0 },
            if direction.2 == 0 { speed[2] } else { 0.0 },
        ];
        particles.push(ExpectedSculkChargeParticle {
            direction,
            position,
            velocity,
            roll,
        });
    }
}

fn expected_sculk_charge_pop_particles(is_full_block: bool) -> Vec<ExpectedSculkChargePopParticle> {
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let particle_count = if is_full_block { 40 } else { 20 };
    let spread = if is_full_block {
        SCULK_CHARGE_POP_FULL_BLOCK_SPREAD
    } else {
        SCULK_CHARGE_POP_PARTIAL_BLOCK_SPREAD
    };
    (0..particle_count)
        .map(|_| {
            let velocity_x = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_y = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_z = 2.0 * f64::from(random.next_float()) - 1.0;
            ExpectedSculkChargePopParticle {
                position: [
                    10.5 + velocity_x * spread,
                    64.5 + velocity_y * spread,
                    -2.5 + velocity_z * spread,
                ],
                velocity: [
                    velocity_x * SCULK_CHARGE_POP_SPEED,
                    velocity_y * SCULK_CHARGE_POP_SPEED,
                    velocity_z * SCULK_CHARGE_POP_SPEED,
                ],
            }
        })
        .collect()
}

fn expected_block_face_position(
    (step_x, step_y, step_z): (i32, i32, i32),
    step_factor: f64,
    random: &mut LevelEventSoundRandomState,
) -> [f64; 3] {
    [
        10.5 + if step_x == 0 {
            expected_random_between(random, -0.5, 0.5)
        } else {
            f64::from(step_x) * step_factor
        },
        64.5 + if step_y == 0 {
            expected_random_between(random, -0.5, 0.5)
        } else {
            f64::from(step_y) * step_factor
        },
        -2.5 + if step_z == 0 {
            expected_random_between(random, -0.5, 0.5)
        } else {
            f64::from(step_z) * step_factor
        },
    ]
}

fn expected_random_between(random: &mut LevelEventSoundRandomState, min: f64, max: f64) -> f64 {
    min + random.next_double() * (max - min)
}

fn rgb_option(r: u8, g: u8, b: u8) -> [f32; 4] {
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ]
}

fn rgb_option_06(r: u8, g: u8, b: u8) -> [f32; 4] {
    [
        f32::from(r) / 255.0 * 0.6,
        f32::from(g) / 255.0 * 0.6,
        f32::from(b) / 255.0 * 0.6,
        1.0,
    ]
}

fn test_biome_color_profile(
    id: i32,
    grass_color: [u8; 3],
    foliage_color: [u8; 3],
    dry_foliage_color: [u8; 3],
    water_color: [u8; 3],
) -> BiomeColorProfile {
    BiomeColorProfile {
        id,
        name: format!("minecraft:test_biome_{id}"),
        temperature: 0.5,
        temperature_modifier: BiomeTemperatureModifier::None,
        downfall: 0.5,
        has_precipitation: true,
        grass_color: Some(grass_color),
        foliage_color: Some(foliage_color),
        dry_foliage_color: Some(dry_foliage_color),
        water_color: Some(water_color),
        fog_color: None,
        sky_color: None,
        water_fog_color: None,
        water_fog_end_distance: None,
        grass_color_modifier: GrassColorModifier::None,
    }
}

struct SplitXBiomeSampler {
    split_x: i32,
    left_biome_id: i32,
    right_biome_id: i32,
}

impl ParticleBiomeSampler for SplitXBiomeSampler {
    fn biome_id_at(&self, pos: WorldBlockPos) -> Option<i32> {
        Some(if pos.x < self.split_x {
            self.left_biome_id
        } else {
            self.right_biome_id
        })
    }
}

fn assert_sculk_charge_command(
    command: &ParticleSpawnCommand,
    expected: &ExpectedSculkChargeParticle,
) {
    assert_particle_command(
        command,
        SCULK_CHARGE_PARTICLE_TYPE_ID,
        "minecraft:sculk_charge",
        expected.position,
        expected.velocity,
        true,
    );
    assert_eq!(command.option_roll, Some(expected.roll));
}

fn assert_sculk_charge_pop_command(
    command: &ParticleSpawnCommand,
    expected: &ExpectedSculkChargePopParticle,
) {
    assert_particle_command(
        command,
        SCULK_CHARGE_POP_PARTICLE_TYPE_ID,
        "minecraft:sculk_charge_pop",
        expected.position,
        expected.velocity,
        true,
    );
    assert_eq!(command.option_roll, None);
}

fn assert_item_break_particle_command(
    command: &ParticleSpawnCommand,
    item_id: i32,
    position: [f64; 3],
    velocity: [f64; 3],
) {
    assert_eq!(command.particle_type_id, ITEM_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:item");
    assert!(command.sprite_ids.is_empty());
    for (actual, expected) in command.position.iter().zip(position) {
        assert_close(*actual, expected);
    }
    for (actual, expected) in command.velocity.iter().zip(velocity) {
        assert_close(*actual, expected);
    }
    assert_eq!(command.override_limiter, false);
    assert_eq!(command.always_show, false);
    assert_eq!(
        command.raw_options_len,
        item_particle_raw_options_len(item_id, 1)
    );
    assert_eq!(command.initial_delay_ticks, 0);
    assert_eq!(
        command.option_item,
        Some(ParticleItemOptionState {
            item_id,
            count: 1,
            component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
        })
    );
}

fn assert_block_destroy_particle_command(
    command: &ParticleSpawnCommand,
    block_state_id: i32,
    position: [f64; 3],
    velocity: [f64; 3],
) {
    assert_eq!(command.particle_type_id, BLOCK_PARTICLE_TYPE_ID);
    assert_eq!(command.particle_id, "minecraft:block");
    assert!(command.sprite_ids.is_empty());
    for (actual, expected) in command.position.iter().zip(position) {
        assert_close(*actual, expected);
    }
    for (actual, expected) in command.velocity.iter().zip(velocity) {
        assert_close(*actual, expected);
    }
    assert_eq!(command.override_limiter, false);
    assert_eq!(command.always_show, false);
    assert_eq!(
        command.raw_options_len,
        block_particle_options(block_state_id).len()
    );
    assert_eq!(command.initial_delay_ticks, 0);
    assert_eq!(
        command.option_block,
        Some(ParticleBlockOptionState { block_state_id })
    );
    assert_eq!(command.option_item, None);
}

fn assert_particle_command(
    command: &ParticleSpawnCommand,
    particle_type_id: i32,
    particle_id: &str,
    position: [f64; 3],
    velocity: [f64; 3],
    override_limiter: bool,
) {
    assert_particle_command_with_visibility_and_delay(
        command,
        particle_type_id,
        particle_id,
        position,
        velocity,
        override_limiter,
        false,
        0,
    );
}

fn assert_particle_command_with_delay(
    command: &ParticleSpawnCommand,
    particle_type_id: i32,
    particle_id: &str,
    position: [f64; 3],
    velocity: [f64; 3],
    override_limiter: bool,
    initial_delay_ticks: u32,
) {
    assert_particle_command_with_visibility_and_delay(
        command,
        particle_type_id,
        particle_id,
        position,
        velocity,
        override_limiter,
        false,
        initial_delay_ticks,
    );
}

fn assert_particle_command_with_visibility(
    command: &ParticleSpawnCommand,
    particle_type_id: i32,
    particle_id: &str,
    position: [f64; 3],
    velocity: [f64; 3],
    override_limiter: bool,
    always_show: bool,
) {
    assert_particle_command_with_visibility_and_delay(
        command,
        particle_type_id,
        particle_id,
        position,
        velocity,
        override_limiter,
        always_show,
        0,
    );
}

fn assert_particle_command_with_visibility_and_delay(
    command: &ParticleSpawnCommand,
    particle_type_id: i32,
    particle_id: &str,
    position: [f64; 3],
    velocity: [f64; 3],
    override_limiter: bool,
    always_show: bool,
    initial_delay_ticks: u32,
) {
    assert_eq!(command.particle_type_id, particle_type_id);
    assert_eq!(command.particle_id, particle_id);
    for (actual, expected) in command.position.iter().zip(position) {
        assert_close(*actual, expected);
    }
    for (actual, expected) in command.velocity.iter().zip(velocity) {
        assert_close(*actual, expected);
    }
    assert_eq!(command.override_limiter, override_limiter);
    assert_eq!(command.always_show, always_show);
    assert_eq!(command.raw_options_len, 0);
    assert_eq!(command.initial_delay_ticks, initial_delay_ticks);
}

fn particle_dir(root: &Path) -> PathBuf {
    assets_dir(root).join("particles")
}

fn assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn particle_definition_json(textures: &[&str]) -> String {
    let textures = textures
        .iter()
        .map(|texture| format!("\"{texture}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(r#"{{ "textures": [{textures}] }}"#)
}

fn write_particle_atlas(assets_dir: &Path) {
    write_json(
        &assets_dir.join("atlases").join("particles.json"),
        r#"{
          "sources": [
            {
              "type": "minecraft:directory",
              "prefix": "",
              "source": "particle"
            }
          ]
        }"#,
    );
}

fn write_item_particle_item_model_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlas(&assets);
    write_item_registry_source(root, "model_component");
    write_json(
        &assets.join("items").join("model_component.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/model_component"
            }
        }"#,
    );
    write_json(
        &assets.join("items").join("alternate_model_component.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/alternate_model_component"
            }
        }"#,
    );
    write_flat_item_model_and_texture(&assets, "model_component");
    write_flat_item_model_and_texture(&assets, "alternate_model_component");
}

fn write_item_atlas(assets_dir: &Path) {
    write_json(
        &assets_dir.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_json(
        &assets_dir.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
}

fn write_item_registry_source(root: &Path, item_id: &str) {
    let constant = item_id.to_ascii_uppercase();
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        &format!(
            r#"public class Items {{
                public static final Item {constant} = registerItem("{item_id}");
            }}"#,
        ),
    );
}

fn write_flat_item_model_and_texture(assets_dir: &Path, model_id: &str) {
    write_json(
        &assets_dir
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
                "textures": {{
                    "layer0": "minecraft:item/{model_id}"
                }}
            }}"#
        ),
    );
    write_test_png(
        &assets_dir
            .join("textures")
            .join("item")
            .join(format!("{model_id}.png")),
        8,
        8,
    );
}

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_test_png(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut image = image::RgbaImage::new(width, height);
    for (index, pixel) in image.pixels_mut().enumerate() {
        let shade = (index % 255) as u8;
        *pixel = image::Rgba([shade, 255 - shade, 64, 255]);
    }
    image.save(path).unwrap();
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}-{id}"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "expected {expected}, got {actual}"
    );
}

fn assert_close_f32(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}
