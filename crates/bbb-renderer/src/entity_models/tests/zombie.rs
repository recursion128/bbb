use super::*;

#[test]
fn zombie_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Zombie { baby: false }.model_key(),
        "zombie"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/zombie.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/zombie_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/husk.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/husk_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/drowned.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/drowned_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie_villager/zombie_villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie_villager/zombie_villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: false,
        }
        .vanilla_texture_ref(),
        None
    );
}

#[test]
fn zombie_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_ZOMBIE_HAT[0],
        ModelCubeDesc {
            min: [-4.5, -8.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: ZOMBIE_GREEN,
        }
    );
    assert_eq!(ADULT_ZOMBIE_PARTS.len(), 6);
    assert_eq!(ADULT_ZOMBIE_PARTS[0].pose, PART_POSE_ZERO);
    assert_eq!(ADULT_ZOMBIE_PARTS[0].cubes, ADULT_ZOMBIE_HEAD.as_slice());
    assert_eq!(
        ADULT_ZOMBIE_PARTS[0].children,
        ADULT_ZOMBIE_HEAD_CHILDREN.as_slice()
    );
    assert_part(
        &ADULT_ZOMBIE_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_HAT.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_BODY.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEG.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_LEG.as_slice(),
    );
}

#[test]
fn zombie_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::zombie(
        54,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5, 64.001, -0.28125]);
    assert_close3(max, [0.5, 66.03225, 0.28125]);
}

#[test]
fn zombie_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_ZOMBIE_HEAD,
        [
            ModelCubeDesc {
                min: [-3.0, -6.25, -3.0],
                size: [6.0, 6.0, 6.0],
                color: ZOMBIE_GREEN,
            },
            ModelCubeDesc {
                min: [-3.25, -6.4, -3.25],
                size: [6.5, 6.5, 6.5],
                color: ZOMBIE_GREEN,
            },
        ]
    );
    assert_eq!(BABY_ZOMBIE_PARTS.len(), 6);
    assert_part(
        &BABY_ZOMBIE_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_BODY.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[1],
        [0.0, 15.25, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_HEAD.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[2],
        [-3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[3],
        [3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[4],
        [-1.0, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_LEG.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_PARTS[5],
        [1.0, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_LEG.as_slice(),
    );
}

#[test]
fn zombie_villager_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_ZOMBIE_VILLAGER_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -10.0, -4.0],
                size: [8.0, 10.0, 8.0],
                color: ZOMBIE_VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [-1.0, -3.0, -6.0],
                size: [2.0, 4.0, 2.0],
                color: ZOMBIE_VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        ADULT_ZOMBIE_VILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-4.05, -0.05, -3.05],
            size: [8.1, 20.1, 6.1],
            color: ZOMBIE_VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_ZOMBIE_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_ZOMBIE_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HEAD.as_slice(),
        ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HAT.as_slice(),
        ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEFT_ARM.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[4],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_ZOMBIE_VILLAGER_PARTS[5],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
    );

    assert_eq!(
        BABY_ZOMBIE_VILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-2.1, -2.85, -1.6],
            size: [4.2, 6.2, 3.2],
            color: ZOMBIE_VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_ZOMBIE_VILLAGER_PARTS.len(), 6);
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[0],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_BODY.as_slice(),
    );
    assert_part_tree(
        &BABY_ZOMBIE_VILLAGER_PARTS[1],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HEAD.as_slice(),
        BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -1.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[2],
        [-3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[3],
        [3.0, 15.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[4],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_ZOMBIE_VILLAGER_PARTS[5],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
    );
}

#[test]
fn zombie_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::zombie(55, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.203125]);
    assert_close3(max, [0.25, 64.947876, 0.203125]);
}

#[test]
fn zombie_variant_meshes_use_vanilla_body_layer_geometry() {
    let zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let baby_zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    let husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        67,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        false,
    )]);
    assert_eq!(husk.opaque_faces, 42);
    assert_eq!(husk.vertices.len(), 168);
    assert_eq!(husk.indices.len(), 252);
    assert!(husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));
    let (husk_min, husk_max) = mesh_extents(&husk);
    assert_close3(husk_min, [-0.53125, 64.00106, -0.29882815]);
    assert_close3(husk_max, [0.53125, 66.15926, 0.29882815]);

    let baby_husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        67,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        true,
    )]);
    assert_same_geometry(&baby_husk, &baby_zombie);
    assert!(baby_husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));

    let drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        38,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    )]);
    assert_same_geometry(&drowned, &zombie);
    assert!(drowned
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

    let baby_drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        38,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        true,
    )]);
    assert_same_geometry(&baby_drowned, &baby_zombie);
    assert!(baby_drowned
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

    let zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        153,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        false,
    )]);
    assert_eq!(zombie_villager.opaque_faces, 60);
    assert_eq!(zombie_villager.vertices.len(), 240);
    assert_eq!(zombie_villager.indices.len(), 360);
    assert!(zombie_villager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
    let (zombie_villager_min, zombie_villager_max) = mesh_extents(&zombie_villager);
    assert_close3(zombie_villager_min, [-0.50000006, 64.001, -0.50000006]);
    assert_close3(zombie_villager_max, [0.50000006, 66.15725, 0.50000006]);

    let baby_zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
        153,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        true,
    )]);
    assert_eq!(baby_zombie_villager.opaque_faces, 60);
    assert_eq!(baby_zombie_villager.vertices.len(), 240);
    assert_eq!(baby_zombie_villager.indices.len(), 360);
    assert!(baby_zombie_villager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
    let (baby_zombie_villager_min, baby_zombie_villager_max) = mesh_extents(&baby_zombie_villager);
    assert_close3(baby_zombie_villager_min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(baby_zombie_villager_max, [0.43750003, 65.01975, 0.37500003]);
}

#[test]
fn humanoid_limb_swing_parts_assign_vanilla_leg_phases_by_side() {
    use std::borrow::Cow;

    // Vanilla HumanoidModel.setupAnim: rightLeg.xRot = cos(pos * 0.6662) * 1.4 *
    // speed (in phase), leftLeg.xRot = cos(pos * 0.6662 + π) * 1.4 * speed (out of
    // phase). The adult zombie lists rightLeg (offset x = -1.9) at index 4 and
    // leftLeg (x = +1.9) at index 5. With pos = 0, speed = 1: rightLeg = 1.4,
    // leftLeg = -1.4.
    let posed = humanoid_limb_swing_parts(
        Cow::Borrowed(&ADULT_ZOMBIE_PARTS),
        HUMANOID_LEG_PART_INDICES,
        0.0,
        1.0,
    );
    assert!(
        (posed[4].pose.rotation[0] - 1.4).abs() < 1e-5,
        "right leg in phase: {}",
        posed[4].pose.rotation[0]
    );
    assert!(
        (posed[5].pose.rotation[0] + 1.4).abs() < 1e-5,
        "left leg out of phase: {}",
        posed[5].pose.rotation[0]
    );
    // The arms (indices 2, 3) are left to the zombie arm pose, untouched here.
    assert_eq!(posed[2].pose.rotation, ADULT_ZOMBIE_PARTS[2].pose.rotation);
    assert_eq!(posed[3].pose.rotation, ADULT_ZOMBIE_PARTS[3].pose.rotation);

    // A general (pos, speed) reproduces the cos(pos * 0.6662 [+ π]) * 1.4 * speed
    // formula including the 0.6662 frequency factor.
    let posed = humanoid_limb_swing_parts(
        Cow::Borrowed(&ADULT_ZOMBIE_PARTS),
        HUMANOID_LEG_PART_INDICES,
        1.5,
        0.5,
    );
    let phase = 1.5_f32 * 0.6662;
    assert!((posed[4].pose.rotation[0] - phase.cos() * 1.4 * 0.5).abs() < 1e-5);
    assert!(
        (posed[5].pose.rotation[0] - (phase + std::f32::consts::PI).cos() * 1.4 * 0.5).abs() < 1e-5
    );
}

#[test]
fn zombie_family_swings_its_legs_when_walking() {
    // Vanilla HumanoidModel.setupAnim swings the legs `cos(pos * 0.6662 [+ π]) * 1.4
    // * speed` (the right leg in phase, the left out of phase) before the zombie arm
    // pose runs, and zombies inherit those legs unchanged. A standing zombie is
    // inert, a walking one lifts its feet (a shorter model) and splays its legs
    // forward/back (a deeper footprint). The held-out zombie arm pose is a separate
    // deferred feature, so the arms stay put.
    let instances: [(&str, EntityModelInstance); 5] = [
        (
            "zombie",
            EntityModelInstance::zombie(60, [0.0, 64.0, 0.0], 0.0, false),
        ),
        (
            "zombie_baby",
            EntityModelInstance::zombie(61, [0.0, 64.0, 0.0], 0.0, true),
        ),
        (
            "husk",
            EntityModelInstance::zombie_variant(
                62,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::Husk,
                false,
            ),
        ),
        (
            "drowned",
            EntityModelInstance::zombie_variant(
                63,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::Drowned,
                false,
            ),
        ),
        (
            "zombie_villager",
            EntityModelInstance::zombie_variant(
                64,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::ZombieVillager,
                false,
            ),
        ),
    ];
    for (name, base) in instances {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{name}: a walking zombie's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{name}: a walking zombie's legs should splay along Z"
        );
    }
}

#[test]
fn zombie_textured_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_ZOMBIE, "minecraft:zombie#main");
    assert_eq!(MODEL_LAYER_ZOMBIE_BABY, "minecraft:zombie_baby#main");

    // Adult: vanilla HumanoidModel.createMesh UVs (texture 64x64). The deformed hat keeps the
    // base 8x8x8 box as its uv_size; the left arm/leg mirror the right's texOffs.
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS.len(), 6);
    let head = &ADULT_ZOMBIE_TEXTURED_PARTS[0];
    assert_eq!(head.cubes[0].tex, [0.0, 0.0]);
    assert_eq!(head.cubes[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(head.children[0].cubes[0].tex, [32.0, 0.0]);
    assert_eq!(head.children[0].cubes[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(head.children[0].cubes[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS[1].cubes[0].tex, [16.0, 16.0]);
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS[2].cubes[0].tex, [40.0, 16.0]);
    assert!(!ADULT_ZOMBIE_TEXTURED_PARTS[2].cubes[0].mirror);
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS[3].cubes[0].tex, [40.0, 16.0]);
    assert!(ADULT_ZOMBIE_TEXTURED_PARTS[3].cubes[0].mirror);
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS[4].cubes[0].tex, [0.0, 16.0]);
    assert!(!ADULT_ZOMBIE_TEXTURED_PARTS[4].cubes[0].mirror);
    assert_eq!(ADULT_ZOMBIE_TEXTURED_PARTS[5].cubes[0].tex, [0.0, 16.0]);
    assert!(ADULT_ZOMBIE_TEXTURED_PARTS[5].cubes[0].mirror);

    // Baby: vanilla BabyZombieModel.createBodyLayer UVs. Each limb has its own texOffs (no
    // mirroring); the head carries the base cube plus the 0.25 deformation overlay.
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS.len(), 6);
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS[0].cubes[0].tex, [16.0, 16.0]);
    let baby_head = &BABY_ZOMBIE_TEXTURED_PARTS[1];
    assert_eq!(baby_head.cubes[0].tex, [3.0, 3.0]);
    assert_eq!(baby_head.cubes[0].uv_size, [6.0, 6.0, 6.0]);
    assert_eq!(baby_head.cubes[1].tex, [35.0, 3.0]);
    assert_eq!(baby_head.cubes[1].uv_size, [6.0, 6.0, 6.0]);
    assert_eq!(baby_head.cubes[1].size, [6.5, 6.5, 6.5]);
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS[2].cubes[0].tex, [36.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS[3].cubes[0].tex, [28.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS[4].cubes[0].tex, [8.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_TEXTURED_PARTS[5].cubes[0].tex, [0.0, 16.0]);
    for part in &BABY_ZOMBIE_TEXTURED_PARTS {
        for cube in part.cubes {
            assert!(!cube.mirror, "baby zombie cubes are never mirrored");
        }
    }
}

#[test]
fn zombie_textured_layer_passes_match_vanilla_renderer() {
    for (baby, model_layer, texture) in [
        (false, "minecraft:zombie#main", ZOMBIE_TEXTURE_REF),
        (true, "minecraft:zombie_baby#main", ZOMBIE_BABY_TEXTURE_REF),
    ] {
        let passes = zombie_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::ZombieBase);
        assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    assert!(entity_model_texture_refs().contains(&ZOMBIE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_BABY_TEXTURE_REF));
    assert_eq!(
        zombie_entity_texture_refs(),
        &[ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF]
    );
}

#[test]
fn zombie_textured_mesh_matches_colored_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&zombie_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [EntityModelInstance::zombie(55, [0.0, 64.0, 0.0], 0.0, baby)];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured zombie shares the colored geometry exactly: same cube count and bounds.
        assert_eq!(textured.cutout_faces, colored.opaque_faces, "baby={baby}");
        assert_eq!(textured.vertices.len(), colored.vertices.len());
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        let (cmin, cmax) = mesh_extents(&colored);
        let (tmin, tmax) = textured_mesh_extents(&textured);
        assert_close3(tmin, cmin);
        assert_close3(tmax, cmax);

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the
        // legs (the held-out arms stay deferred, like the colored path).
        let walking = [instances[0]
            .with_walk_animation(2.0, 1.0)
            .with_age_in_ticks(8.0)];
        let textured_walk = entity_model_textured_mesh(&walking, &atlas);
        assert_ne!(
            textured.vertices, textured_walk.vertices,
            "legs swing (baby={baby})"
        );
    }
}

fn zombie_texture_images() -> Vec<EntityModelTextureImage> {
    zombie_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
