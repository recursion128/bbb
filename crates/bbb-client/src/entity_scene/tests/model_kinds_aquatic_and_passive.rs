use super::*;

#[test]
fn entity_model_kind_uses_exact_models_for_base_zombie_and_skeleton() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID, &[]),
        EntityModelKind::Zombie { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                true
            )]
        ),
        EntityModelKind::Zombie { baby: true }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_ID, &[]),
        EntityModelKind::Skeleton
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HUSK_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HUSK_ID,
            &[protocol_zombie_baby_data(VANILLA_ENTITY_TYPE_HUSK_ID, true)]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DROWNED_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                true
            )]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                true,
            )]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            &[protocol_bool_data(piglin_baby_data_id(), true)]
        ),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                true,
            )]
        ),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_STRAY_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PARCHED_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: false }
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID, &[bogged_sheared_data(true)]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_armor_stands() {
    let accessors = armor_stand_metadata_accessors();
    let default_pose = armor_stand_default_model_pose();
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, &[]),
        EntityModelKind::ArmorStand {
            small: false,
            marker: false,
            show_arms: false,
            show_base_plate: true,
            pose: default_pose,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            &[protocol_byte_data(
                accessors.client_flags_data_id,
                accessors.small_mask
                    | accessors.show_arms_mask
                    | accessors.no_baseplate_mask
                    | accessors.marker_mask,
            )],
        ),
        EntityModelKind::ArmorStand {
            small: true,
            marker: true,
            show_arms: true,
            show_base_plate: false,
            pose: default_pose,
        }
    );

    let mut pose = default_pose;
    pose.head = [1.0, 2.0, 3.0];
    pose.right_arm = [-20.0, 5.0, 10.0];
    pose.left_leg = [4.0, 5.0, 6.0];
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            &[
                protocol_rotations_data(accessors.head_pose_data_id, pose.head),
                protocol_rotations_data(accessors.right_arm_pose_data_id, pose.right_arm),
                protocol_rotations_data(accessors.left_leg_pose_data_id, pose.left_leg),
            ],
        ),
        EntityModelKind::ArmorStand {
            small: false,
            marker: false,
            show_arms: false,
            show_base_plate: true,
            pose,
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_slime_and_magma_cube() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SLIME_ID, &[]),
        EntityModelKind::Slime { size: 1 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SLIME_ID,
            &[slime_size_data(VANILLA_ENTITY_TYPE_SLIME_ID, 4)]
        ),
        EntityModelKind::Slime { size: 4 }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, &[]),
        EntityModelKind::MagmaCube { size: 1 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            &[slime_size_data(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, 3)]
        ),
        EntityModelKind::MagmaCube { size: 3 }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_ghast() {
    // The ghast was a placeholder render box; it now resolves to the real model. The `charging` flag
    // (vanilla `Ghast.DATA_IS_CHARGING`, BOOLEAN at index 16) swaps to the shooting texture.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GHAST_ID, &[]),
        EntityModelKind::Ghast { charging: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GHAST_ID,
            &[protocol_bool_data(ghast_charging_data_id(), true)]
        ),
        EntityModelKind::Ghast { charging: true }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_happy_ghast() {
    // The happy ghast was a placeholder render box; it now resolves to the real model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, &[]),
        EntityModelKind::HappyGhast { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, true)]
        ),
        EntityModelKind::HappyGhast { baby: true }
    );
}

#[test]
fn entity_model_kind_projects_pufferfish_puff_state_from_data() {
    // The pufferfish was a placeholder render box; it now resolves to the real model and
    // projects its synced `PUFF_STATE` (whose generated default is deflated).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, &[]),
        EntityModelKind::Pufferfish { puff_state: 0 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            &[pufferfish_puff_state_data(2)]
        ),
        EntityModelKind::Pufferfish { puff_state: 2 }
    );
}

#[test]
fn entity_model_kind_maps_vex_to_real_model() {
    // The vex resolves to the real `VexModel`. Its idle wing flap / arm bob / head look read the
    // projected age and look angles. The generated `Vex.DATA_FLAGS_ID` and named charging mask
    // project to `charging`, which vanilla `VexRenderer.getTextureLocation` swaps to
    // `vex_charging.png`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &[]),
        EntityModelKind::Vex { charging: false }
    );
    let (_, default, charging_mask) =
        bbb_client_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VEX_ID,
        )
        .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    let charging_values = vec![vex_flags_data(charging_mask)];
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &charging_values),
        EntityModelKind::Vex { charging: true }
    );
}

#[test]
fn entity_model_kind_maps_dragon_fireball_to_billboard_quad() {
    // The dragon fireball was a placeholder render box; it now resolves to the camera-facing
    // `DragonFireballRenderer` full-sheet quad. Its constant block light 15 already rides the
    // generated static-full-bright renderer catalog.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, &[]),
        EntityModelKind::DragonFireball
    );
}

#[test]
fn entity_model_kind_maps_experience_orb_to_billboard_quad() {
    // The orb was a placeholder render box; it now resolves to the camera-facing
    // `ExperienceOrbRenderer` icon quad. The generated `ExperienceOrb.DATA_VALUE` projects
    // through vanilla `ExperienceOrb.getIcon()` thresholds to the 4×4 sheet cell.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, &[]),
        EntityModelKind::ExperienceOrb { icon: 0 }
    );
    let (value_data_id, default) =
        bbb_client_world::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        )
        .expect("generated ExperienceOrb value accessor must resolve");
    assert_eq!(default, 0);
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            &[protocol_int_data(value_data_id, 149)]
        ),
        EntityModelKind::ExperienceOrb { icon: 6 }
    );
}

#[test]
fn entity_model_kind_projects_bee_baby_from_data() {
    // The bee was a placeholder render box; it now resolves to the real `AdultBeeModel` /
    // `BabyBeeModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult).
    // The procedural airborne flap / bob reads the projected age and ground state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BEE_ID, &[]),
        EntityModelKind::Bee {
            baby: false,
            angry: false,
            has_nectar: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_BEE_ID, true)]
        ),
        EntityModelKind::Bee {
            baby: true,
            angry: false,
            has_nectar: false,
        }
    );
}

#[test]
fn entity_model_kind_projects_bee_nectar_and_angry_texture_flags() {
    // Vanilla `BeeRenderer.getTextureLocation` keys on `hasNectar` (the synced
    // `DATA_FLAGS_ID & 8`, index 18) and `isAngry` (the synced `DATA_ANGER_END_TIME`, index 19,
    // in the future). A bee carrying nectar swaps to the `*_nectar*` texture.
    let accessors = bee_metadata_accessors();
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[protocol_byte_data(
                accessors.flags_data_id,
                accessors.has_nectar_mask,
            )],
            0.0,
            0
        ),
        EntityModelKind::Bee {
            baby: false,
            angry: false,
            has_nectar: true,
        }
    );
    // An anger-end time past the current game time makes the bee angry (and the roll/stung
    // bits in the flags byte do not flip `hasNectar`).
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[protocol_long_data(accessors.anger_end_time_data_id, 100)],
            0.0,
            10
        ),
        EntityModelKind::Bee {
            baby: false,
            angry: true,
            has_nectar: false,
        }
    );
}

#[test]
fn entity_model_kind_projects_dolphin_baby_from_data() {
    // The dolphin was a placeholder render box; it now resolves to the real `DolphinModel`,
    // keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult). Its swim body
    // tilt / tail wave reads the projected `isMoving` (the synced velocity); the held-item carry
    // layer remains a separate equipment concern.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DOLPHIN_ID, &[]),
        EntityModelKind::Dolphin { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_DOLPHIN_ID, true)]
        ),
        EntityModelKind::Dolphin { baby: true }
    );
}

#[test]
fn guardian_instance_carries_world_projected_camera_eye_target() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        330,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        [0.0, 64.0, 0.0],
        0.0,
        0.0,
        0.0,
    ));
    world.apply_add_entity(protocol_add_entity(
        331,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 66.0, 0.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 331 }));

    let projected = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 330)
        .unwrap()
        .render
        .kind
        .guardian()
        .eye_tracking;
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 330)
        .unwrap();

    assert!(projected.is_some());
    assert_eq!(
        instance.render_state.kind.guardian().eye_tracking,
        projected
    );
}

#[test]
fn entity_model_kind_maps_creaking_to_real_model() {
    // The data-only selector resolves the real `CreakingModel` and uses synced `IS_ACTIVE` for the
    // ordinary emissive-eye state. `entity_model_instance` overrides this with the world-projected
    // client flicker during teardown, when the exact walk/combat/death states are already projected.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CREAKING_ID, &[]),
        EntityModelKind::Creaking {
            eyes_glowing: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            &[protocol_bool_data(creaking_active_data_id(), true)]
        ),
        EntityModelKind::Creaking { eyes_glowing: true }
    );
}

#[test]
fn entity_model_kind_maps_frog_to_real_model() {
    // The frog resolves to the real `FrogModel` at its rest pose, textured by temperature
    // variant. With no synced `DATA_VARIANT_ID` it defaults to TEMPERATE; otherwise the
    // `Holder<FrogVariant>` registry id selects the colour. Without a synced `frog_variant`
    // registry, the static `FrogVariants.bootstrap` order (TEMPERATE=0, WARM=1, COLD=2) applies.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FROG_ID, &[]),
        EntityModelKind::Frog {
            variant: FrogModelVariant::Temperate
        }
    );
    for (id, variant) in [
        (0, FrogModelVariant::Temperate),
        (1, FrogModelVariant::Warm),
        (2, FrogModelVariant::Cold),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FROG_ID,
                &[protocol_frog_variant_data(id)]
            ),
            EntityModelKind::Frog { variant }
        );
    }

    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_FROG_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_frog_variant_data(0),
                "minecraft:cold",
            )]
        ),
        EntityModelKind::Frog {
            variant: FrogModelVariant::Cold
        }
    );
}

#[test]
fn entity_model_kind_projects_turtle_baby_from_data() {
    // The turtle was a placeholder render box; it now resolves to the real
    // `AdultTurtleModel` / `BabyTurtleModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
    // (index 16, default adult). The head look, land-walk / water-swim branch, egg-laying amplitude,
    // and egg-belly shell read projected look, walk, environment, and synced turtle state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TURTLE_ID, &[]),
        EntityModelKind::Turtle { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true)]
        ),
        EntityModelKind::Turtle { baby: true }
    );
}

#[test]
fn entity_model_kind_projects_strider_baby_and_cold_from_data() {
    // Resolves `AdultStriderModel` / `BabyStriderModel` from `AgeableMob.DATA_BABY_ID` (16) and
    // cold state from `DATA_SUFFOCATING` (19).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_STRIDER_ID, &[]),
        EntityModelKind::Strider {
            baby: false,
            cold: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_STRIDER_ID, true)]
        ),
        EntityModelKind::Strider {
            baby: true,
            cold: false
        }
    );
    // A suffocating strider carries the cold texture.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            &[strider_suffocating_data(
                VANILLA_ENTITY_TYPE_STRIDER_ID,
                true,
            )]
        ),
        EntityModelKind::Strider {
            baby: false,
            cold: true
        }
    );
}

#[test]
fn entity_model_kind_projects_salmon_size_from_variant_data() {
    // The salmon was a placeholder render box; it now resolves to the real `SalmonModel`
    // and projects its synced `DATA_TYPE` size variant (`Salmon.Variant` ids
    // SMALL=0/MEDIUM=1/LARGE=2 clamped, with a generated MEDIUM default).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Medium,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(0)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Small,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(2)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Large,
        }
    );
    // Out-of-range ids clamp to the large body, matching `ByIdMap.continuous(CLAMP)`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(9)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Large,
        }
    );
}

#[test]
fn entity_model_kind_projects_tropical_fish_shape_from_packed_variant() {
    // The tropical fish was a placeholder render box; it now resolves to the real model
    // and decodes the body shape from the synced packed variant (`DATA_ID_TYPE_VARIANT`,
    // index 17). The default 0 (KOB/white/white) is the small body; a LARGE-base pattern
    // selects the flopper body.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, &[]),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Small,
            base_color: EntityDyeColor::White,
            // pattern bits = 0 → Pattern.byId(0) = KOB; pattern color = (0 >> 24) = WHITE.
            pattern: TropicalFishPattern::Kob,
            pattern_color: EntityDyeColor::White,
        }
    );
    // FLOPPER (LARGE base, index 0) with arbitrary base/pattern color bytes → large body.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[tropical_fish_variant_data(0x0405_0001)]
        ),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Large,
            // base byte = (0x0405_0001 >> 16) & 0xFF = 0x05 → DyeColor.byId(5) = LIME.
            base_color: EntityDyeColor::Lime,
            // pattern bits = 0x0405_0001 & 0xFFFF = 1 → Pattern.byId(1) = FLOPPER.
            pattern: TropicalFishPattern::Flopper,
            // pattern color = (0x0405_0001 >> 24) & 0xFF = 0x04 → DyeColor.byId(4) = YELLOW.
            pattern_color: EntityDyeColor::Yellow,
        }
    );
    // SPOTTY (SMALL base, index 5 → 0x0500) stays the small body.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[tropical_fish_variant_data(0x0500)]
        ),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Small,
            // base byte = (0x0500 >> 16) & 0xFF = 0 → DyeColor.byId(0) = WHITE.
            base_color: EntityDyeColor::White,
            // pattern bits = 0x0500 = 1280 → Pattern.byId(1280) = SPOTTY (small, index 5).
            pattern: TropicalFishPattern::Spotty,
            pattern_color: EntityDyeColor::White,
        }
    );
}

#[test]
fn entity_model_kind_projects_tropical_fish_base_color_from_packed_variant() {
    // Vanilla `TropicalFish.getBaseColor(packedVariant) = DyeColor.byId(packedVariant >> 16
    // & 0xFF)`, surfaced by `TropicalFishRenderer.getModelTint = state.baseColor`. Each dye
    // id occupies bits 16..24 of the packed variant; the low 16 bits (pattern) and high 8
    // bits (pattern color) must not bleed into the base color.
    let base_color_of = |packed: i32| match entity_model_kind(
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        &[tropical_fish_variant_data(packed)],
    ) {
        EntityModelKind::TropicalFish { base_color, .. } => base_color,
        other => panic!("expected tropical fish, got {other:?}"),
    };
    // id 0 → WHITE, id 11 → BLUE, id 15 → BLACK, with noise in the other byte ranges.
    assert_eq!(
        base_color_of(0x00FF_FFFF & !0x00FF_0000),
        EntityDyeColor::White
    );
    assert_eq!(base_color_of(0x000B_0000), EntityDyeColor::Blue);
    assert_eq!(base_color_of(0xFF0F_FFFFu32 as i32), EntityDyeColor::Black);
    // Out-of-range base byte (16) falls back to WHITE like `DyeColor.byId` (ZERO strategy).
    assert_eq!(base_color_of(0x0010_0000), EntityDyeColor::White);
}

#[test]
fn entity_model_kind_projects_tropical_fish_pattern_and_pattern_color_from_packed_variant() {
    // Vanilla `getPattern(packed) = Pattern.byId(packed & 0xFFFF)` (sparse, default KOB) and
    // `getPatternColor(packed) = DyeColor.byId(packed >> 24 & 0xFF)`. The pattern occupies the
    // low 16 bits and the pattern color the top byte; the base color byte (bits 16..24) must
    // not bleed into either.
    let decode = |packed: i32| match entity_model_kind(
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        &[tropical_fish_variant_data(packed)],
    ) {
        EntityModelKind::TropicalFish {
            shape,
            pattern,
            pattern_color,
            ..
        } => (shape, pattern, pattern_color),
        other => panic!("expected tropical fish, got {other:?}"),
    };

    // All twelve patterns map from their `base.id | index << 8` packed id.
    for (packed_pattern, expected, shape) in [
        (
            0x0000,
            TropicalFishPattern::Kob,
            TropicalFishModelShape::Small,
        ),
        (
            0x0100,
            TropicalFishPattern::Sunstreak,
            TropicalFishModelShape::Small,
        ),
        (
            0x0200,
            TropicalFishPattern::Snooper,
            TropicalFishModelShape::Small,
        ),
        (
            0x0300,
            TropicalFishPattern::Dasher,
            TropicalFishModelShape::Small,
        ),
        (
            0x0400,
            TropicalFishPattern::Brinely,
            TropicalFishModelShape::Small,
        ),
        (
            0x0500,
            TropicalFishPattern::Spotty,
            TropicalFishModelShape::Small,
        ),
        (
            0x0001,
            TropicalFishPattern::Flopper,
            TropicalFishModelShape::Large,
        ),
        (
            0x0101,
            TropicalFishPattern::Stripey,
            TropicalFishModelShape::Large,
        ),
        (
            0x0201,
            TropicalFishPattern::Glitter,
            TropicalFishModelShape::Large,
        ),
        (
            0x0301,
            TropicalFishPattern::Blockfish,
            TropicalFishModelShape::Large,
        ),
        (
            0x0401,
            TropicalFishPattern::Betty,
            TropicalFishModelShape::Large,
        ),
        (
            0x0501,
            TropicalFishPattern::Clayfish,
            TropicalFishModelShape::Large,
        ),
    ] {
        // Mix in a non-zero base color byte (GRAY = 7) and pattern color byte (BLUE = 11) to
        // prove neither disturbs the pattern decode.
        let packed = packed_pattern | (0x07 << 16) | (0x0B << 24);
        let (got_shape, got_pattern, got_pattern_color) = decode(packed);
        assert_eq!(got_pattern, expected);
        assert_eq!(got_shape, shape);
        assert_eq!(got_shape, expected.shape(), "shape mirrors pattern.shape()");
        assert_eq!(got_pattern_color, EntityDyeColor::Blue);
    }

    // Unknown pattern id falls back to KOB (small body) like `ByIdMap.sparse(..., KOB)`.
    let (shape, pattern, _) = decode(0x00AB);
    assert_eq!(pattern, TropicalFishPattern::Kob);
    assert_eq!(shape, TropicalFishModelShape::Small);
}

#[test]
fn entity_model_kind_projects_squid_glow_and_baby_from_data() {
    // The squid and glow squid were placeholder render boxes; they now resolve to the
    // real `SquidModel`. The glow variant follows the registered renderer class and the baby
    // flag uses the generated inherited `AgeableMob.DATA_BABY_ID` accessor. The
    // tentacle sweep / body tilt are projected by the world-side squid animation accumulator.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SQUID_ID, &[]),
        EntityModelKind::Squid {
            glow: false,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &[]),
        EntityModelKind::Squid {
            glow: true,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SQUID_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SQUID_ID, true)]
        ),
        EntityModelKind::Squid {
            glow: false,
            baby: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, true)]
        ),
        EntityModelKind::Squid {
            glow: true,
            baby: true,
        }
    );
}

#[test]
fn entity_model_kind_projects_phantom_size_from_data() {
    // The phantom was a placeholder render box; it now resolves to the real model and
    // projects its synced `ID_SIZE` and generated default.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PHANTOM_ID, &[]),
        EntityModelKind::Phantom { size: 0 }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PHANTOM_ID, &[phantom_size_data(5)]),
        EntityModelKind::Phantom { size: 5 }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_base_cow_and_sheep() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COW_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_COW_ID, true)]
        ),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[]),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SHEEP_ID, true)]
        ),
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    // The mooshroom shares the cow body, so it renders through the dedicated `Mooshroom` model
    // (the real cow mesh) rather than the generic quadruped stand-in — adult and baby alike. The
    // default variant (no `DATA_TYPE`) is the vanilla `Red`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
        EntityModelKind::Mooshroom {
            baby: false,
            variant: MooshroomVariant::Red,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, true)]
        ),
        EntityModelKind::Mooshroom {
            baby: true,
            variant: MooshroomVariant::Red,
        }
    );
    // The generated synced `MushroomCow.DATA_TYPE` accessor selects the brown coat (id 1;
    // `ByIdMap` CLAMP folds out-of-range ids to the nearest endpoint).
    for (id, variant) in [
        (-1, MooshroomVariant::Red),
        (1, MooshroomVariant::Brown),
        (2, MooshroomVariant::Brown),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
                &[mushroom_cow_variant_data(id)]
            ),
            EntityModelKind::Mooshroom {
                baby: false,
                variant,
            }
        );
    }
}

#[test]
fn entity_model_kind_uses_vanilla_sheep_wool_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[sheep_wool_data(14, true)]),
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[
                sheep_wool_data(15, false),
                baby_data(VANILLA_ENTITY_TYPE_SHEEP_ID, true),
            ]
        ),
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::Black,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_invisible_data(), sheep_wool_data(14, false),]
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
    );
}

#[test]
fn entity_model_kind_projects_sheep_jeb_custom_name_and_age() {
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[
                entity_custom_name_data(Some("jeb_")),
                sheep_wool_data(0, false),
            ],
            12.5,
            0
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: true,
            age_ticks: 12.5,
        }
    );
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_custom_name_data(Some("Not jeb_"))],
            25.0,
            0
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 25.0,
        }
    );
    assert_eq!(
        entity_model_kind_with_time(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_custom_name_data(None)],
            25.0,
            0
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 25.0,
        }
    );
}

#[test]
fn entity_model_instances_project_sheep_wool_metadata_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 111,
        values: vec![sheep_wool_data(14, true)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        entity_model_instances_from_world_at_partial_tick(&world, None, 0.25)
    );
    assert_eq!(
        without_bounds(&instances),
        vec![EntityModelInstance::sheep_render_state(
            111,
            [1.0, 64.0, -2.0],
            0.0,
            false,
            true,
            SheepWoolColor::Red,
            false,
            false,
            0.0,
        )
        .with(|s| s.kind = EntityKindRenderState::Sheep(Default::default()))]
    );
    // A visible entity (no invisible shared flag) projects `invisible == false`.
    assert!(!instances[0].render_state.invisible);
}

#[test]
fn entity_model_instances_project_sheep_jeb_custom_name_and_age_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 112,
        values: vec![entity_custom_name_data(Some("jeb_"))],
    }));
    world.advance_entity_client_animations(12);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![EntityModelInstance::sheep_render_state(
                112,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                SheepWoolColor::White,
                false,
                true,
                12.5,
            )
            .with(|s| s.kind = EntityKindRenderState::Sheep(Default::default()))],
            12.5,
        )
    );
}

#[test]
fn entity_model_instances_project_sheep_invisible_shared_flag_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_data(), sheep_wool_data(14, false),],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(
        without_bounds(&instances),
        vec![EntityModelInstance::sheep_render_state(
            113,
            [1.0, 64.0, -2.0],
            0.0,
            false,
            false,
            SheepWoolColor::Red,
            true,
            false,
            0.0,
        )
        .with(|s| s.kind = EntityKindRenderState::Sheep(Default::default()))]
    );
    // The shared invisible flag is now projected uniformly into the render state.
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_spectator_visible_invisible_sheep_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_data(), sheep_wool_data(14, false),],
    }));
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 3.0,
    });

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(instances.len(), 1);
    assert!(instances[0].render_state.invisible);
    assert!(!instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_glowing_shared_flag_from_world() {
    let mut world = WorldStore::new();
    let sheep = protocol_add_entity(113, VANILLA_ENTITY_TYPE_SHEEP_ID, [1.0, 64.0, -2.0]);
    let sheep_uuid = sheep.uuid;
    world.apply_add_entity(sheep);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_glowing_data(), sheep_wool_data(14, false),],
    }));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "green".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![sheep_uuid.to_string().into()],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(instances.len(), 1);
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
    assert!(instances[0].render_state.appears_glowing);
    assert_eq!(instances[0].render_state.outline_color, 0xff55_ff55);
}

#[test]
fn entity_model_kind_uses_exact_models_for_goats() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GOAT_ID, &[]),
        EntityModelKind::Goat {
            baby: false,
            left_horn: true,
            right_horn: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GOAT_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true),
                goat_left_horn_data(false),
                goat_right_horn_data(true),
            ]
        ),
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GOAT_ID,
            &[goat_left_horn_data(false), goat_right_horn_data(false),]
        ),
        EntityModelKind::Goat {
            baby: false,
            left_horn: false,
            right_horn: false,
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_hoglins_and_zoglins() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HOGLIN_ID, &[]),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Hoglin,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HOGLIN_ID, true)]
        ),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Hoglin,
            baby: true,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOGLIN_ID, &[]),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Zoglin,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ZOGLIN_ID, true)]
        ),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Zoglin,
            baby: true,
        }
    );
}

#[test]
fn entity_model_kind_maps_sniffer_to_real_model() {
    // The sniffer was approximated by the cow quadruped model; it now resolves to the real
    // `SnifferModel` on the adult `ModelLayers.SNIFFER` or baby `ModelLayers.SNIFFER_BABY`
    // baked layer. Vanilla keys the baby renderer off `AgeableMob.DATA_BABY_ID` (index 16) and
    // uses `snifflet.png` while still constructing a `SnifferModel`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNIFFER_ID, &[]),
        EntityModelKind::Sniffer { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SNIFFER_ID, true)]
        ),
        EntityModelKind::Sniffer { baby: true }
    );
}

#[test]
fn entity_model_kind_projects_armadillo_baby_from_data() {
    // The armadillo was a placeholder bounds box; it now resolves to the real
    // `AdultArmadilloModel` / `BabyArmadilloModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
    // (index 16, default adult), as in the vanilla `AgeableMobRenderer`. The clamped head look,
    // `applyWalk` leg sway, and the roll-out / roll-up / peek keyframe transitions are projected
    // separately from world animation state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARMADILLO_ID, &[]),
        EntityModelKind::Armadillo {
            baby: false,
            rolled_up: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ARMADILLO_ID, true)]
        ),
        EntityModelKind::Armadillo {
            baby: true,
            rolled_up: false
        }
    );
}

#[test]
fn entity_model_kind_projects_armadillo_rolled_up_from_state() {
    // Vanilla `Armadillo.ARMADILLO_STATE` is an `ArmadilloState` enum. Only the steady SCARED state
    // is `shouldHideInShell` for every tick, so it maps to `rolled_up`; the tick-gated ROLLING /
    // UNROLLING transitions and IDLE stay not-rolled-up.
    let kind = |id: i32| {
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[protocol_armadillo_state_data(id)],
        )
    };
    assert_eq!(
        kind(ARMADILLO_STATE_SCARED_ID),
        EntityModelKind::Armadillo {
            baby: false,
            rolled_up: true
        }
    );
    for non_scared in [
        bbb_client_world::VANILLA_ARMADILLO_STATE_IDLE_ID,
        bbb_client_world::VANILLA_ARMADILLO_STATE_ROLLING_ID,
        bbb_client_world::VANILLA_ARMADILLO_STATE_UNROLLING_ID,
    ] {
        assert_eq!(
            kind(non_scared),
            EntityModelKind::Armadillo {
                baby: false,
                rolled_up: false
            },
            "state {non_scared} is not the steady SCARED ball"
        );
    }

    // A baby armadillo can roll up too — the state and the baby flag compose.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_ARMADILLO_ID, true),
                protocol_armadillo_state_data(ARMADILLO_STATE_SCARED_ID),
            ]
        ),
        EntityModelKind::Armadillo {
            baby: true,
            rolled_up: true
        }
    );
}

#[test]
fn entity_model_kind_projects_axolotl_baby_from_data() {
    // The axolotl was a placeholder bounds box; it now resolves to the real `AdultAxolotlModel`
    // / `BabyAxolotlModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default
    // adult), as in the vanilla `AgeableMobRenderer`, and textured by the `Axolotl.Variant`
    // colour read from the generated `DATA_VARIANT` accessor. Adult procedural factors and baby
    // AnimationState elapsed clocks project independently through the selected model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[]),
        EntityModelKind::Axolotl {
            baby: false,
            variant: AxolotlModelVariant::Lucy
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_AXOLOTL_ID, true)]
        ),
        EntityModelKind::Axolotl {
            baby: true,
            variant: AxolotlModelVariant::Lucy
        }
    );
    // `DATA_VARIANT` selects the colour via `Axolotl.Variant.byId`.
    for (id, variant) in [
        (-1, AxolotlModelVariant::Lucy),
        (0, AxolotlModelVariant::Lucy),
        (1, AxolotlModelVariant::Wild),
        (2, AxolotlModelVariant::Gold),
        (3, AxolotlModelVariant::Cyan),
        (4, AxolotlModelVariant::Blue),
        (5, AxolotlModelVariant::Lucy),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[axolotl_variant_data(id)]),
            EntityModelKind::Axolotl {
                baby: false,
                variant
            }
        );
    }
}

#[test]
fn entity_model_kind_maps_parrot_to_real_model() {
    // The parrot resolves to the real `ParrotModel` at its STANDING rest pose, textured per the
    // five `Parrot.Variant` colours read from the generated synced `DATA_VARIANT_ID` via
    // `Parrot.Variant.byId`. Projected render state drives the head look, pose branch, wing flap,
    // and dance; the player shoulder layer constructs the separate ON_SHOULDER pose.
    // No synced data → the vanilla DEFAULT (RED_BLUE).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[]),
        EntityModelKind::Parrot {
            variant: ParrotModelVariant::RedBlue,
        }
    );
    // Each id selects its colour; out-of-range values clamp to the nearest endpoint.
    for (id, variant) in [
        (-99, ParrotModelVariant::RedBlue),
        (0, ParrotModelVariant::RedBlue),
        (1, ParrotModelVariant::Blue),
        (2, ParrotModelVariant::Green),
        (3, ParrotModelVariant::YellowBlue),
        (4, ParrotModelVariant::Gray),
        (99, ParrotModelVariant::Gray),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[parrot_variant_data(id)]),
            EntityModelKind::Parrot { variant }
        );
    }
}

#[test]
fn entity_model_kind_maps_shulker_to_real_model() {
    let accessors = shulker_metadata_accessors();
    // The shulker resolves to the real `ShulkerModel` at its closed rest pose, textured by the
    // dye colour read from `DATA_COLOR_ID`. With no synced colour it defaults to the uncolored
    // texture (`None`). Projected render state drives peek open/close, head look, and attach-face
    // rotation.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_ID, &[]),
        EntityModelKind::Shulker { color: None }
    );
    // The generated vanilla default is the uncolored shulker; 0..=15 select a dye.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            &[protocol_byte_data(
                accessors.color_data_id,
                accessors.color_default,
            )]
        ),
        EntityModelKind::Shulker { color: None }
    );
    for (id, color) in [
        (0, EntityDyeColor::White),
        (4, EntityDyeColor::Yellow),
        (11, EntityDyeColor::Blue),
        (15, EntityDyeColor::Black),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHULKER_ID,
                &[protocol_byte_data(accessors.color_data_id, id)]
            ),
            EntityModelKind::Shulker { color: Some(color) }
        );
    }
}

#[test]
fn entity_model_kind_maps_arrows_to_real_model() {
    // The arrow and spectral arrow were placeholder boxes; they now resolve to the real
    // `ArrowModel`, sharing one model but binding different images. A plain arrow is `Normal`; a
    // tipped arrow (`ID_EFFECT_COLOR` 11 > 0) is `Tipped`; the spectral arrow type is `Spectral`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARROW_ID, &[]),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &[protocol_arrow_effect_color_data(0x385dc6)]
        ),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Tipped
        }
    );
    // A potionless arrow (`getColor()` returns the `-1` sentinel) is not tipped.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &[protocol_arrow_effect_color_data(-1)]
        ),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, &[]),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Spectral
        }
    );
}

#[test]
fn entity_model_instances_project_arrow_impact_shake() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        60,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        [1.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(1);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 60,
        values: vec![protocol_abstract_arrow_in_ground_data(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            true,
        )],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    let arrow = instances
        .iter()
        .find(|instance| instance.entity_id == 60)
        .expect("arrow instance");
    assert_eq!(
        arrow.kind,
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(arrow.render_state.kind.arrow().shake, 6.75);
}

#[test]
fn entity_model_instances_project_thrown_trident_foil_flag() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        135,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        [1.0, 64.0, -2.0],
    ));

    let default_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let trident = default_instances
        .iter()
        .find(|instance| instance.entity_id == 135)
        .expect("trident instance");
    assert_eq!(trident.kind, EntityModelKind::Trident);
    assert!(!trident.render_state.kind.trident().foil);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 135,
        values: vec![protocol_thrown_trident_foil_data(true)],
    }));

    let foiled_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let trident = foiled_instances
        .iter()
        .find(|instance| instance.entity_id == 135)
        .expect("foiled trident instance");
    assert_eq!(trident.kind, EntityModelKind::Trident);
    assert!(trident.render_state.kind.trident().foil);
}

#[test]
fn entity_model_kind_skips_thrown_item_projectiles_for_the_billboard_layer() {
    // The thrown-item projectiles (vanilla `ThrownItemRenderer`) render as a camera-facing item
    // sprite via the item-entity billboard layer, so the 3D model scene draws nothing for them — the
    // model kind is `NoRender` rather than the former placeholder box.
    for (type_id, _scale) in bbb_client_world::vanilla_entity_renderer_thrown_item_scales() {
        assert_eq!(
            entity_model_kind(type_id, &[]),
            EntityModelKind::NoRender,
            "thrown-item projectile type {type_id} should be NoRender",
        );
    }
}

#[test]
fn entity_model_kind_maps_wither_skull_to_real_model() {
    // The wither skull was a placeholder box; it now resolves to the real `SkullModel`. Its flight
    // facing comes from the projected yaw/pitch (a plain `EntityRenderer`). Vanilla
    // `WitherSkull.DATA_DANGEROUS` is the synced boolean at id 8 and swaps to
    // `wither_invulnerable.png`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, &[]),
        EntityModelKind::WitherSkull { dangerous: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            &[wither_skull_dangerous_data(true)]
        ),
        EntityModelKind::WitherSkull { dangerous: true }
    );
}

#[test]
fn entity_model_kind_maps_llama_spit_to_real_model() {
    // The llama spit was a placeholder box; it now resolves to the real `LlamaSpitModel`. The
    // model has no `setupAnim`, so the geometry is complete; only the texture is deferred
    // entity-side state, so no synced data is read.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, &[]),
        EntityModelKind::LlamaSpit
    );
}

#[test]
fn entity_model_kind_maps_shulker_bullet_to_real_model() {
    // The shulker bullet was a placeholder box; it now resolves to the real `ShulkerBulletModel`.
    // Its facing and age-driven tumble come from projected render state; the textured path adds the
    // translucent outer-shell pass. The kind mapping itself reads no synced data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, &[]),
        EntityModelKind::ShulkerBullet
    );
}

#[test]
fn entity_model_kind_maps_wind_charges_to_real_model() {
    // The wind charge and breeze wind charge were placeholder boxes; both share the real
    // `WindChargeModel` (vanilla registers `WindChargeRenderer` for both). Projected age drives the
    // counter-rotation and scrolling translucent texture; the kind mapping itself reads no synced
    // data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, &[]),
        EntityModelKind::WindCharge
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, &[]),
        EntityModelKind::WindCharge
    );
}

#[test]
fn entity_model_kind_renders_nothing_for_noop_renderer_entities() {
    // The area effect cloud, marker, and interaction use vanilla `NoopRenderer` — they render no
    // model — so they resolve to `EntityModelKind::NoRender`, replacing the former placeholder
    // boxes (which incorrectly drew a debug box where vanilla draws nothing).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, &[]),
        EntityModelKind::NoRender
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_INTERACTION_ID, &[]),
        EntityModelKind::NoRender
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MARKER_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_polar_bears() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, &[]),
        EntityModelKind::PolarBear { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, true)]
        ),
        EntityModelKind::PolarBear { baby: true }
    );
    // The panda (adult and baby) renders through its dedicated `PandaModel` / `BabyPandaModel`;
    // with no gene metadata the displayed variant is the vanilla default `NORMAL`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PANDA_ID, &[]),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Normal
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_PANDA_ID, true)]
        ),
        EntityModelKind::Panda {
            baby: true,
            variant: PandaModelVariant::Normal
        }
    );
}

#[test]
fn entity_model_kind_projects_panda_gene_variant_from_data() {
    // Vanilla `Panda.getVariant()` = `Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two
    // synced gene bytes. A dominant main gene always shows.
    let accessors = panda_metadata_accessors();
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 6),
                protocol_byte_data(accessors.hidden_gene_data_id, 0),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Aggressive
        }
    );
    // A recessive main gene (BROWN=4) shows only when both genes match.
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 4),
                protocol_byte_data(accessors.hidden_gene_data_id, 4),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Brown
        }
    );
    // An unmatched recessive main gene falls back to NORMAL.
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 4),
                protocol_byte_data(accessors.hidden_gene_data_id, 1),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Normal
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_villagers() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[]),
        EntityModelKind::Villager { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, true)]
        ),
        EntityModelKind::Villager { baby: true }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, &[]),
        EntityModelKind::WanderingTrader
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, true)]
        ),
        EntityModelKind::WanderingTrader
    );
}

#[test]
fn villager_model_data_reads_vanilla_serializer_and_static_fallback_order() {
    assert_eq!(
        villager_model_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[]),
        VillagerModelData::DEFAULT
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data_at_unhappy_counter(6, 14, 9)]
        ),
        VillagerModelData::DEFAULT,
        "id 18 is AbstractVillager.DATA_UNHAPPY_COUNTER, not Villager.DATA_VILLAGER_DATA"
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                6,
                14,
                9,
            )]
        ),
        VillagerModelData::new(
            VillagerModelType::Taiga,
            VillagerModelProfession::Weaponsmith,
            9,
        )
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                4,
                11,
                2
            )]
        ),
        VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Nitwit, 2,)
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                4,
                11,
                2,
            )]
        ),
        VillagerModelData::DEFAULT,
        "zombie villager data lives at id 20, not the villager id 19"
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                6,
                14,
                9,
            )]
        ),
        VillagerModelData::DEFAULT,
        "wandering traders do not own DATA_VILLAGER_DATA"
    );
}

#[test]
fn villager_model_data_uses_built_in_identities() {
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                1,
                0,
                5,
            )]
        ),
        VillagerModelData::new(VillagerModelType::Jungle, VillagerModelProfession::None, 5,)
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                99,
                -1,
                3,
            )]
        ),
        VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::None, 3,)
    );
}
