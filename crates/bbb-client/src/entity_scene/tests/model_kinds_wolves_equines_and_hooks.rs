use super::*;

#[test]
fn entity_model_kind_uses_exact_models_for_wolves() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)]
        ),
        EntityModelKind::Wolf {
            baby: true,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                true,
                false,
            )]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
            ]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Blue),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                entity_invisible_data(),
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
            ]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Blue),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_int_data(
                wolf_metadata_accessors().collar_color_data_id,
                11,
            )]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    // The adult cat, ocelot, and fox render through their dedicated models (cat = the shared
    // `AdultFelineModel` scaled 0.8, ocelot = the unscaled feline, fox = `AdultFoxModel`); each baby
    // now renders through its own dedicated vanilla mesh.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[]),
        EntityModelKind::Feline {
            cat: true,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_OCELOT_ID, &[]),
        EntityModelKind::Feline {
            cat: false,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FOX_ID, &[]),
        EntityModelKind::Fox {
            baby: false,
            variant: FoxModelVariant::Red
        }
    );
    // The cat/ocelot babies now render through the dedicated `BabyFelineModel` layout, as does the
    // fox baby through its own `BabyFoxModel`.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAT_ID, true)]
        ),
        EntityModelKind::Feline {
            cat: true,
            baby: true,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_OCELOT_ID, true)]
        ),
        EntityModelKind::Feline {
            cat: false,
            baby: true,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_FOX_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_FOX_ID, true)]
        ),
        EntityModelKind::Fox {
            baby: true,
            variant: FoxModelVariant::Red
        }
    );
    // The fox `DATA_TYPE_ID` (18, int) selects the RED/SNOW variant via `Fox.Variant.byId`.
    let fox_accessors =
        vanilla_entity_metadata_fox_accessors_for_entity_type(VANILLA_ENTITY_TYPE_FOX_ID)
            .expect("generated Fox metadata accessors must resolve");
    for (id, variant) in [
        (-1, FoxModelVariant::Red),
        (0, FoxModelVariant::Red),
        (1, FoxModelVariant::Snow),
        (2, FoxModelVariant::Red),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FOX_ID,
                &[protocol_int_data(fox_accessors.type_data_id, id)]
            ),
            EntityModelKind::Fox {
                baby: false,
                variant
            }
        );
    }
    // The rabbit (adult and baby) renders through its dedicated `AdultRabbitModel` / `BabyRabbitModel`,
    // textured by the `Rabbit.Variant` colour (`DATA_TYPE_ID`) plus the Toast name override.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[]),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_RABBIT_ID, true)]
        ),
        EntityModelKind::Rabbit {
            baby: true,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
    // `DATA_TYPE_ID` selects the colour via `Rabbit.Variant.byId` (sparse; EVIL = 99).
    for (id, variant) in [
        (0, RabbitModelVariant::Brown),
        (1, RabbitModelVariant::White),
        (2, RabbitModelVariant::Black),
        (3, RabbitModelVariant::WhiteSplotched),
        (4, RabbitModelVariant::Gold),
        (5, RabbitModelVariant::Salt),
        (99, RabbitModelVariant::Evil),
        (7, RabbitModelVariant::Brown),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[rabbit_type_data(id)]),
            EntityModelKind::Rabbit {
                baby: false,
                variant,
                toast: false
            }
        );
    }
    // The custom name "Toast" flips the toast override; any other name does not.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[entity_custom_name_data(Some("Toast"))]
        ),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[entity_custom_name_data(Some("toast"))]
        ),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
}

#[test]
fn entity_model_kind_projects_wolf_identity_and_fallback() {
    // The world-bound metadata identity records that registry id 1 meant `ashen`; consumers do
    // not reinterpret the ordinal through a separately supplied registry generation.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_wolf_variant_data(1),
                "minecraft:ashen",
            )]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Ashen,
        }
    );

    // No dynamic registry → the static vanilla order: id 3 is `black`.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_wolf_variant_data(3)]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Black,
        }
    );

    // No variant holder at all → the vanilla `WolfVariants.DEFAULT` (`Pale`).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
}

#[test]
fn entity_model_kind_uses_vanilla_cat_variant_metadata() {
    // Without the dynamic `cat_variant` registry the bootstrap order (tabby=0..all_black=10) is
    // the static fallback; the vanilla default is BLACK. The ocelot has no breed.
    for (id, variant) in [
        (0, CatModelVariant::Tabby),
        (1, CatModelVariant::Black),
        (2, CatModelVariant::Red),
        (3, CatModelVariant::Siamese),
        (4, CatModelVariant::BritishShorthair),
        (5, CatModelVariant::Calico),
        (6, CatModelVariant::Persian),
        (7, CatModelVariant::Ragdoll),
        (8, CatModelVariant::White),
        (9, CatModelVariant::Jellie),
        (10, CatModelVariant::AllBlack),
        (99, CatModelVariant::Black),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[protocol_cat_variant_data(id)]),
            EntityModelKind::Feline {
                cat: true,
                baby: false,
                cat_variant: variant,
                collar: None
            }
        );
    }
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[protocol_cat_variant_data(0)]
        ),
        EntityModelKind::Feline {
            cat: false,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
}

#[test]
fn entity_model_kind_projects_cat_collar_from_tame_and_color() {
    // Vanilla `CatRenderer`: `state.collarColor = isTame() ? getCollarColor() : null`, and
    // `getCollarColor() = DyeColor.byId(DATA_COLLAR_COLOR)` (default RED). The ocelot has no collar.
    fn collar_of(kind: &EntityModelKind) -> Option<EntityDyeColor> {
        match kind {
            EntityModelKind::Feline { collar, .. } => *collar,
            other => panic!("expected feline, got {other:?}"),
        }
    }

    // An untamed cat carries no collar even with a color set.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[protocol_int_data(
                cat_metadata_accessors().collar_color_data_id,
                5,
            )],
            true
        )),
        None
    );
    // A tame cat with no explicit color defaults to RED (14).
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_CAT_ID,
                true,
                false,
            )],
            true
        )),
        Some(EntityDyeColor::Red)
    );
    // A tame cat shows its dyed collar color.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, true, false),
                protocol_int_data(cat_metadata_accessors().collar_color_data_id, 5),
            ],
            true
        )),
        Some(EntityDyeColor::Lime)
    );
    // A tame ocelot still has no collar.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_CAT_ID,
                true,
                false,
            )],
            false
        )),
        None
    );
}

#[test]
fn entity_model_instances_project_cat_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:cat_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:jellie"),
            RegistryPacketEntry::stub("minecraft:calico"),
            RegistryPacketEntry::stub("minecraft:white"),
        ],
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[protocol_cat_variant_data(99)]),
        EntityModelKind::Feline {
            cat: true,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    world.apply_add_entity(protocol_add_entity(
        41,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        42,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 41,
        values: vec![protocol_cat_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![
            protocol_cat_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_CAT_ID, true),
        ],
    }));

    let start_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        start_instances, instances,
        "feline geometry does not consume the generic per-frame age"
    );

    assert_eq!(
        without_bounds(&instances),
        vec![
            EntityModelInstance::feline(
                41,
                [1.0, 64.0, -2.0],
                0.0,
                true,
                false,
                CatModelVariant::Jellie,
                None,
            )
            .with(|s| s.kind = EntityKindRenderState::Feline(Default::default())),
            EntityModelInstance::feline(
                42,
                [3.0, 64.0, -2.0],
                0.0,
                true,
                true,
                CatModelVariant::White,
                None,
            )
            .with(|s| s.kind = EntityKindRenderState::Feline(Default::default()))
            .with(|s| s.age_scale = 0.5),
        ]
    );
}

#[test]
fn entity_model_instances_project_feline_crouch_sprint_and_cat_sitting_from_world() {
    // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copy `Entity.isCrouching()`
    // (Pose.CROUCHING, ordinal 5) and `Entity.isSprinting()` (shared flags bit 3). Cat also copies
    // `TamableAnimal.isInSittingPose()`; ocelot leaves `isSitting` at the default false.
    let pose_data = |pose| entity_pose_data(pose);
    let feline_state = |world: &WorldStore, id: i32| {
        let instance = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap();
        (
            instance.render_state.kind.feline().is_crouching,
            instance.render_state.kind.feline().is_sprinting,
            instance.render_state.kind.feline().is_sitting,
        )
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        43,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        44,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        [3.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        45,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [5.0, 64.0, -2.0],
    ));

    assert_eq!(feline_state(&world, 43), (false, false, false));
    assert_eq!(feline_state(&world, 44), (false, false, false));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 43,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_sprinting_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, true),
        ],
    }));
    assert_eq!(feline_state(&world, 43), (true, true, true));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 44,
        values: vec![pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
    }));
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 44,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_CAT_ID,
            false,
            true,
        )],
    }));
    assert_eq!(feline_state(&world, 44), (true, false, false));

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 45,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_sprinting_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, true),
        ],
    }));
    assert_eq!(feline_state(&world, 45), (false, false, false));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 43,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            entity_default_shared_flags_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, false),
        ],
    }));
    assert_eq!(feline_state(&world, 43), (false, false, false));
}

#[test]
fn entity_model_kind_uses_vanilla_wolf_anger_end_time_metadata() {
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_long_data(
                wolf_metadata_accessors().anger_end_time_data_id,
                200,
            )],
            0.0,
            199
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: true,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_long_data(
                wolf_metadata_accessors().anger_end_time_data_id,
                200,
            )],
            0.0,
            200
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_long_data(wolf_metadata_accessors().anger_end_time_data_id, 200),
            ],
            0.0,
            199
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: true,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
    );
}

#[test]
fn entity_model_instances_project_wolf_anger_from_world_game_time() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![protocol_long_data(
            wolf_metadata_accessors().anger_end_time_data_id,
            130,
        )],
    }));
    world.apply_world_time(PlayTime {
        game_time: 120,
        clock_updates: Vec::new(),
    });

    let angry_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&angry_instances),
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                true,
                false,
                None,
            )
            .with(|s| s.kind = EntityKindRenderState::Wolf(Default::default()))
            // Vanilla `Wolf.getTailAngle()` angry branch raises the tail to 1.5393804.
            .with(|s| s.kind.wolf_mut().tail_angle = 1.5393804)],
            1.0,
        )
    );

    world.apply_world_time(PlayTime {
        game_time: 130,
        clock_updates: Vec::new(),
    });

    let calm_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&calm_instances),
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                false,
                false,
                None,
            )
            .with(|s| s.kind = EntityKindRenderState::Wolf(Default::default()))],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_wolf_invisible_shared_flag_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![
            entity_invisible_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
            protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                true,
                false,
                true,
                Some(EntityDyeColor::Blue),
            )
            // A tame wolf with no synced health defaults to full (maxHealth 40), so
            // `Wolf.getTailAngle()` = (0.55 - 0) * π.
            .with(|s| s.kind.wolf_mut().tail_angle = 0.55 * std::f32::consts::PI)],
            1.0,
        )
    );
    // The shared invisible flag is now projected uniformly into the render state.
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_wolf_tame_tail_angle_from_health() {
    // Vanilla `Wolf.getTailAngle()` for a tame wolf droops the tail with damage:
    // (0.55 - damageRatio * 0.4) * π, damageRatio = (maxHealth - health) / maxHealth,
    // with the tame maxHealth constant 40. A hurt tame wolf (health 8/40) lowers its
    // tail off the healthy raise.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
            living_entity_health_data(VANILLA_ENTITY_TYPE_WOLF_ID, 8.0),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let tail_angle = instances[0].render_state.kind.wolf().tail_angle;
    let expected = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // damageRatio 0.8
    assert!(
        (tail_angle - expected).abs() < 1e-6,
        "tame wolf tail droops with health: {tail_angle} vs {expected}"
    );

    // An untamed wolf keeps the π/5 default no matter its health.
    let mut wild = WorldStore::new();
    wild.apply_add_entity(protocol_add_entity(
        149,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(wild.apply_set_entity_data(SetEntityData {
        id: 149,
        values: vec![living_entity_health_data(VANILLA_ENTITY_TYPE_WOLF_ID, 4.0)],
    }));
    let wild_instances = entity_model_instances_from_world_at_partial_tick(&wild, None, 1.0);
    assert_eq!(
        wild_instances[0].render_state.kind.wolf().tail_angle,
        std::f32::consts::PI / 5.0
    );
}

#[test]
fn entity_model_instances_project_wolf_sitting_flag_from_world() {
    // Vanilla `WolfRenderState.isSitting = Wolf.isInSittingPose()` = `TamableAnimal`
    // generated `DATA_FLAGS_ID` sitting mask. A sitting (tame) wolf projects `wolf_sitting`; clearing the
    // bit projects `false`.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            true,
        )],
    }));
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        instances[0].render_state.kind.wolf().sitting,
        "a sitting wolf projects wolf_sitting"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            false,
        )],
    }));
    let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        !standing[0].render_state.kind.wolf().sitting,
        "a standing wolf does not project wolf_sitting"
    );
}

#[test]
fn entity_model_instances_project_parrot_sitting_flag_from_world() {
    // Vanilla `ParrotModel.getPose == SITTING` = `Parrot.isInSittingPose()` = the same
    // The generated `TamableAnimal.DATA_FLAGS_ID` sitting bit inherited by the wolf. A sitting parrot projects
    // `parrot_sitting`; the projection is gated to the parrot, so the same flag byte on
    // another tamable (wolf) never sets `parrot_sitting`.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [2.0, 64.0, -2.0],
    ));

    let parrot_sitting = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .parrot()
            .sitting
    };

    // A standing parrot projects parrot_sitting = false.
    assert!(!parrot_sitting(&world, 150));

    // Setting the TamableAnimal sitting bit projects through to the perch pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 150,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_PARROT_ID,
            false,
            true,
        )],
    }));
    assert!(parrot_sitting(&world, 150));

    // The same sitting bit on a non-parrot (wolf) does NOT project parrot_sitting — the
    // derivation is gated to the parrot type.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            true,
        )],
    }));
    assert!(!parrot_sitting(&world, 151));
}

#[test]
fn entity_model_instances_project_illager_spellcasting_flag_from_world() {
    // Vanilla `SpellcasterIllager.isCastingSpell()` = the synced `DATA_SPELL_CASTING_ID`
    // byte > 0 (the byte holds the spell id). A casting evoker/illusioner projects
    // `illager_spellcasting`; the projection is gated to the spellcaster illagers, so the
    // same byte on a vindicator never sets it.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        [3.0, 64.0, -2.0],
    ));

    let casting = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .illager()
            .spellcasting
    };

    // An idle evoker/illusioner projects illager_spellcasting = false.
    assert!(!casting(&world, 160));
    assert!(!casting(&world, 161));

    // Setting the spell-casting byte > 0 (here 2 = FANGS) projects through to the cast pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_EVOKER_ID),
            2,
        )],
    }));
    assert!(casting(&world, 160));
    // Any non-zero spell id (1 = SUMMON_VEX, the lowest) also counts as casting.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 161,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
            1,
        )],
    }));
    assert!(casting(&world, 161));

    // The same byte on a non-spellcaster (vindicator) does NOT project illager_spellcasting.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_EVOKER_ID),
            2,
        )],
    }));
    assert!(!casting(&world, 162));
}

#[test]
fn entity_model_kind_uses_exact_models_for_horses() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID, &[]),
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HORSE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)]
        ),
        EntityModelKind::Horse {
            baby: true,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
    );
    // The packed `DATA_ID_TYPE_VARIANT` (id 19) carries the coat color in the low byte
    // (`& 0xFF`) and the markings in the next nibble (`>> 8`): id `4 | (2 << 8)` = black coat +
    // white-field markings.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HORSE_ID,
            &[horse_variant_data(4 | (2 << 8))]
        ),
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::Black,
            markings: HorseMarkings::WhiteField
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DONKEY_ID, &[]),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
            ]
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MULE_ID,
            &[chested_horse_chest_data(VANILLA_ENTITY_TYPE_MULE_ID, true)]
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, &[]),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, true)]
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, &[]),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
    );
    // The nautilus (adult and baby) renders through its dedicated `NautilusModel`
    // (`createBodyMesh` / `createBabyBodyLayer`). The zombie nautilus maps to the dedicated
    // `ZombieNautilus` kind, selected by the synced `ZombieNautilusVariant` holder: the default
    // TEMPERATE → the `NautilusModel` body + `zombie_nautilus.png`; WARM → the
    // `ZombieNautilusCoralModel` + `zombie_nautilus_coral.png`. It is a plain `MobRenderer`, so
    // always adult.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[]),
        EntityModelKind::Nautilus { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)]
        ),
        EntityModelKind::Nautilus { baby: true }
    );
    // No variant data → the TEMPERATE default (no corals).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, &[]),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(0)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
    // WARM (registry id 1) → the coral model.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(1)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm
        }
    );
    // The zombie nautilus is never a baby, so the baby flag in its metadata is ignored.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, true)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
}

#[test]
fn entity_model_kind_projects_zombie_nautilus_identity_and_fallback() {
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_zombie_nautilus_variant_data(0),
                "minecraft:warm",
            )]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(99)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_camels() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_ID, &[]),
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAMEL_ID, true)]
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, &[]),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, true)]
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: false
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_llamas() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_ID, &[]),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            &[
                llama_variant_data(VANILLA_ENTITY_TYPE_LLAMA_ID, 2),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
            ]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
                llama_variant_data(VANILLA_ENTITY_TYPE_LLAMA_ID, 3),
            ]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            &[llama_variant_data(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, 99)]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            &[llama_variant_data(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, -1)]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: false
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_illagers_and_witch() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITCH_ID, &[]),
        EntityModelKind::Witch
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_EVOKER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ILLUSIONER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PILLAGER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VINDICATOR_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_spiders() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SPIDER_ID, &[]),
        EntityModelKind::Spider
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, &[]),
        EntityModelKind::CaveSpider
    );
}

#[test]
fn entity_model_kind_uses_avatar_model_part_visibility_for_players_and_mannequins() {
    let hat_and_left_sleeve = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PLAYER_ID, &[]),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, &[]),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
            parts: PlayerModelPartVisibility::from_vanilla_mask(
                PlayerModelPartVisibility::ALL_MASK,
            ),
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            &[protocol_byte_data(
                AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                hat_and_left_sleeve.vanilla_mask() as i8,
            )],
        ),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: hat_and_left_sleeve,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            &[protocol_byte_data(AVATAR_MODEL_CUSTOMIZATION_DATA_ID, 0)],
        ),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_boats_and_rafts() {
    let cases = [
        (
            VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID,
            BoatModelFamily::Acacia,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID,
            BoatModelFamily::Acacia,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
            BoatModelFamily::Bamboo,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
            BoatModelFamily::Bamboo,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID,
            BoatModelFamily::Birch,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID,
            BoatModelFamily::Cherry,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID,
            BoatModelFamily::DarkOak,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID,
            BoatModelFamily::Jungle,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID,
            BoatModelFamily::Mangrove,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            BoatModelFamily::Oak,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID,
            BoatModelFamily::PaleOak,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID,
            BoatModelFamily::Spruce,
            true,
        ),
    ];

    for (entity_type_id, family, chest) in cases {
        assert_eq!(
            entity_model_kind(entity_type_id, &[]),
            EntityModelKind::Boat { family, chest }
        );
    }
}

#[test]
fn entity_model_kind_uses_exact_model_for_iron_golem() {
    // No synced health → the default full-health golem is uncracked.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, &[]),
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::None,
        }
    );
    // Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(health / 100)`: at 40/100
    // (= 0.4) the medium cracks show.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            &[living_entity_health_data(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                40.0,
            )]
        ),
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::Medium,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
        EntityModelKind::SnowGolem
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_copper_golem_weathering() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        }
    );
    for (id, weathering) in [
        (-5, CopperGolemWeathering::Unaffected),
        (0, CopperGolemWeathering::Unaffected),
        (1, CopperGolemWeathering::Exposed),
        (2, CopperGolemWeathering::Weathered),
        (3, CopperGolemWeathering::Oxidized),
        (99, CopperGolemWeathering::Oxidized),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
                &[protocol_copper_golem_weathering_data(id)],
            ),
            EntityModelKind::CopperGolem { weathering }
        );
    }
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            &[protocol_armadillo_state_data(2)],
        ),
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        }
    );
}

#[test]
fn entity_model_kind_uses_explicit_unknown_future_fallback() {
    let kind = entity_model_kind(9999, &[]);

    assert_eq!(kind, EntityModelKind::NoRender);
}

#[test]
fn entity_model_instances_filter_local_player_and_camera_entity() {
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(10));
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [2.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::chicken(12, [2.0, 64.0, 0.0], 0.0, false)
                    .with(|s| s.kind = EntityKindRenderState::Chicken(Default::default()))
            ],
            1.0,
        )
    );
}

#[test]
fn camera_perspective_detached_camera_entity_is_visible() {
    assert!(attached_camera_hides_entity(Some(11), 11, false, false));
    assert!(!attached_camera_hides_entity(Some(11), 11, true, false));
    assert!(!attached_camera_hides_entity(Some(11), 11, false, true));
    assert!(!attached_camera_hides_entity(Some(11), 12, false, false));
}

#[test]
fn fishing_hook_third_person_hand_offset_matches_vanilla_projection() {
    assert_selection_box_close(
        fishing_hook_third_person_line_origin_offset(fishing_hook_render_source()),
        [1.65, 1.3, 4.8],
    );

    let mut source = fishing_hook_render_source();
    source.holding_arm_left = true;
    source.owner_is_crouching = true;
    assert_selection_box_close(
        fishing_hook_third_person_line_origin_offset(source),
        [2.35, 1.1125, 4.8],
    );
}

#[test]
fn fishing_hook_world_source_reaches_dedicated_instance_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        4,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 3.0],
    ));
    let mut hook = protocol_add_entity(5, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, [6.0, 62.0, -1.0]);
    hook.data = 4;
    world.apply_add_entity(hook);

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 5)
        .expect("dedicated fishing-hook instance");
    assert_eq!(instance.kind, EntityModelKind::FishingHook);
    assert_selection_box_close(
        instance
            .render_state
            .kind
            .fishing_hook()
            .line_origin_offset
            .expect("player-owner hand offset"),
        [-3.65, 2.92, 4.8],
    );
}

#[test]
fn fishing_hook_first_person_hand_offset_uses_camera_near_plane() {
    let pose = CameraPose {
        position: [10.0, 20.0, 30.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    assert_selection_box_close(
        fishing_hook_first_person_line_origin_offset(
            fishing_hook_render_source(),
            pose,
            [1920, 1080],
            70.0,
        ),
        [1.5518672, 1.7019858, 4.6857147],
    );

    let mut source = fishing_hook_render_source();
    source.holding_arm_left = true;
    assert_selection_box_close(
        fishing_hook_first_person_line_origin_offset(source, pose, [1920, 1080], 70.0),
        [2.4481325, 1.7019858, 4.6857147],
    );
}

#[test]
fn entity_model_kind_uses_dedicated_fishing_hook_renderer() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, &[]),
        EntityModelKind::FishingHook
    );
}

pub(super) const VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS: &[i32] = &[
    0, 1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34, 35, 36, 38, 40, 41, 42, 45, 46, 51, 52, 54, 55, 56, 57, 58, 59, 61, 62,
    63, 64, 65, 66, 67, 68, 70, 74, 75, 78, 80, 81, 82, 83, 85, 86, 87, 88, 89, 90, 91, 94, 95, 96,
    97, 98, 99, 100, 101, 102, 103, 104, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    119, 121, 122, 124, 125, 126, 127, 128, 129, 130, 132, 133, 134, 136, 137, 138, 139, 140, 141,
    142, 143, 144, 145, 146, 148, 149, 150, 151, 152, 153, 154, 155,
];
