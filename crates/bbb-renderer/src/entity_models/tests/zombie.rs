use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

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
    // The zombie builds a named-children tree (`head` -> `hat`, `body`, the arms/legs), so the
    // head look resolves the `head` child by name; the geometry is asserted on the per-part unified
    // cube consts. Each cube carries both render paths' data (colored tint + textured uv/tex/mirror).
    assert_eq!(
        ADULT_ZOMBIE_HAT[0],
        ModelCube::new(
            [-4.5, -8.5, -4.5],
            [9.0, 9.0, 9.0],
            ZOMBIE_GREEN,
            [8.0, 8.0, 8.0],
            [32.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_ZOMBIE_HEAD[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_BODY[0].size, [8.0, 12.0, 4.0]);
    // The left arm/leg share the colored geometry but carry the mirrored zombie UV.
    assert_eq!(ADULT_ZOMBIE_RIGHT_ARM[0].size, [4.0, 12.0, 4.0]);
    assert!(!ADULT_ZOMBIE_RIGHT_ARM[0].mirror);
    assert!(ADULT_ZOMBIE_LEFT_ARM[0].mirror);
    assert_eq!(ADULT_ZOMBIE_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert!(!ADULT_ZOMBIE_RIGHT_LEG[0].mirror);
    assert!(ADULT_ZOMBIE_LEFT_LEG[0].mirror);
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

    // The held-out `animateZombieArms` pose swings the resting arms forward (xRot ≈ -80°)
    // and splays them out (yRot ±0.1) plus the idle bob, so they reach to +Z (max Z grows
    // from the ±0.28125 bind half-depth to 0.65274626) and widen X slightly. The legs/head
    // still bound Y.
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5226382, 64.001, -0.28125]);
    assert_close3(max, [0.5226382, 66.03225, 0.65274626]);
}

#[test]
fn zombie_arm_held_out_pose_matches_vanilla_resting_animate_zombie_arms() {
    // Vanilla AnimationUtils.animateZombieArms at attackTime = 0 (non-aggressive): both arms
    // drop forward to xRot = -π/2.25, splay to yRot ∓0.1 (right arm -0.1, left +0.1), zero
    // zRot, then take the idle bob. The right arm rests at x = -5, the left at x = +5.
    let right_arm_pose = PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_arm_pose = PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let arm_drop = -std::f32::consts::PI / 2.25;
    let right = zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0);
    let left = zombie_arm_held_out_pose(left_arm_pose, false, 0.0, 0.0);
    // At ageInTicks 0 the bob's xRot term is sin(0) * 0.05 = 0, so xRot is the bare arm drop.
    assert!(
        (right.rotation[0] - arm_drop).abs() < 1e-6,
        "right arm drop: {}",
        right.rotation[0]
    );
    assert!((left.rotation[0] - arm_drop).abs() < 1e-6, "left arm drop");
    // The arms splay out (yRot is untouched by the bob): right -0.1, left +0.1.
    assert!((right.rotation[1] - (-0.1)).abs() < 1e-6, "right arm splay");
    assert!((left.rotation[1] - 0.1).abs() < 1e-6, "left arm splay");
    // zRot starts at the held-out 0 and carries only the idle-bob baseline at age 0
    // (right arm +0.1, left -0.1).
    assert!((right.rotation[2] - 0.1).abs() < 1e-6, "right arm bob zRot");
    assert!(
        (left.rotation[2] - (-0.1)).abs() < 1e-6,
        "left arm bob zRot"
    );
    // The pose is set absolutely (the deep arm drop overrides the inherited swing); the
    // offset is preserved.
    assert_eq!(right.offset, right_arm_pose.offset);

    // An aggressive mob (Mob.isAggressive) raises its arms higher: armDrop = -π/1.5, deeper
    // (more negative) than the calm -π/2.25. Only xRot changes; the yRot splay and the bob
    // are unchanged.
    let aggressive_arm_drop = -std::f32::consts::PI / 1.5;
    let aggressive_right = zombie_arm_held_out_pose(right_arm_pose, true, 0.0, 0.0);
    assert!(
        (aggressive_right.rotation[0] - aggressive_arm_drop).abs() < 1e-6,
        "aggressive right arm drop: {}",
        aggressive_right.rotation[0]
    );
    assert!(
        aggressive_right.rotation[0] < right.rotation[0],
        "aggressive arms are raised higher (deeper drop) than calm: {} vs {}",
        aggressive_right.rotation[0],
        right.rotation[0]
    );
    assert!(
        (aggressive_right.rotation[1] - (-0.1)).abs() < 1e-6,
        "splay unchanged"
    );
    assert!(
        (aggressive_right.rotation[2] - 0.1).abs() < 1e-6,
        "bob zRot unchanged"
    );
}

#[test]
fn zombie_arm_held_out_pose_swings_during_a_melee_strike() {
    use std::f32::consts::PI;
    // Vanilla AnimationUtils.animateZombieArms attack terms (attackTime > 0): attackYRot = sin(t·π)
    // lifts both arms' yRot toward center (yRot = ±(0.1 - attackYRot·0.6)) and the xRot chops
    // (`+= attackYRot·1.2 - sin((1-(1-t)²)·π)·0.4`), over the held-out arm drop.
    let right_arm_pose = PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let t = 0.5_f32;
    let arm_drop = -PI / 2.25;
    let attack_y = (t * PI).sin();
    let attack_x = ((1.0 - (1.0 - t) * (1.0 - t)) * PI).sin();

    let resting = zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0);
    let swinging = zombie_arm_held_out_pose(right_arm_pose, false, t, 0.0);

    // The right arm chops up off the rest drop (xRot rises by attackYRot·1.2 - attackX·0.4).
    assert!(
        (swinging.rotation[0] - (arm_drop + attack_y * 1.2 - attack_x * 0.4)).abs() < 1e-6,
        "right arm xRot swings: {}",
        swinging.rotation[0]
    );
    assert!(
        swinging.rotation[0] > resting.rotation[0],
        "the strike lifts the arm off the held-out drop"
    );
    // The yRot pulls in toward center: right arm -(0.1 - attackYRot·0.6) = +0.2 at t = 0.5.
    assert!(
        (swinging.rotation[1] - (-(0.1 - attack_y * 0.6))).abs() < 1e-6,
        "right arm yRot pulls in: {}",
        swinging.rotation[1]
    );

    // attackTime 0 reproduces the resting held-out pose exactly.
    assert_eq!(
        zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0),
        resting
    );
}

#[test]
fn zombie_arms_held_out_and_bob_with_age() {
    // The zombie arms are held out forward (animateZombieArms), reaching well past the body's
    // ~0.28 bind depth, and the folded-in idle bob moves them with ageInTicks even while the
    // zombie stands still.
    let base = EntityModelInstance::zombie(60, [0.0, 64.0, 0.0], 0.0, false);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(27.3)]);
    let max_z = early
        .vertices
        .iter()
        .map(|vertex| vertex.position[2])
        .fold(f32::MIN, f32::max);
    assert!(max_z > 0.5, "the held-out arms reach forward: {max_z}");
    // Standing, the only age-dependent motion is the arm idle bob, so the meshes differ.
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the held-out arms bob with ageInTicks"
    );
}

#[test]
fn aggressive_zombie_poses_its_arms_differently() {
    // An aggressive zombie (Mob.isAggressive, projected as is_aggressive) raises its held-out
    // arms higher — armDrop -π/1.5 (past horizontal) vs the calm -π/2.25 — so its arm
    // geometry differs from a calm zombie at the same age; the head/body/legs are unaffected
    // (same topology). The exact armDrop is pinned by the pose formula test above; here we
    // confirm the projected flag reaches the rendered mesh.
    let calm = entity_model_mesh(&[EntityModelInstance::zombie(
        61,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let aggressive =
        entity_model_mesh(&[
            EntityModelInstance::zombie(61, [0.0, 64.0, 0.0], 0.0, false).with_is_aggressive(true),
        ]);
    assert_eq!(calm.vertices.len(), aggressive.vertices.len());
    assert_ne!(
        calm.vertices, aggressive.vertices,
        "the aggressive flag raises the arms, changing the mesh"
    );
}

#[test]
fn drowned_raises_its_trident_to_throw() {
    use std::f32::consts::PI;
    // Vanilla `DrownedModel.setupAnim` THROW_TRIDENT: after the held-out zombie arms, the main (right)
    // arm raises the trident overhead (`rightArm.xRot = rightArm.xRot * 0.5 - π`, `rightArm.yRot = 0`).
    let drowned = |throwing: bool| {
        EntityModelInstance::zombie_variant(
            38,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        )
        .with_is_aggressive(true)
        .with_age_in_ticks(9.0)
        .with_drowned_throw_trident(throwing)
    };

    let mut not_throwing = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, false);
    not_throwing.prepare(&drowned(false));
    let held_out_x = not_throwing.root_mut().child_mut("right_arm").pose.rotation[0];

    let mut throwing = ZombieVariantModel::new(ZombieVariantModelFamily::Drowned, false);
    throwing.prepare(&drowned(true));
    let right = throwing.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (held_out_x * 0.5 - PI)).abs() < 1.0e-6,
        "the throw raises the right arm overhead off the held-out drop: {} vs {}",
        right.rotation[0],
        held_out_x * 0.5 - PI
    );
    assert!(
        right.rotation[1].abs() < 1.0e-6,
        "the throwing arm faces forward (yRot = 0): {}",
        right.rotation[1]
    );
    // The raised arm points up-and-back (xRot near -π), well off the held-out forward drop.
    assert!(
        right.rotation[0] < -2.0,
        "the trident is raised overhead: {}",
        right.rotation[0]
    );

    // The throw visibly changes the mesh versus an aggressive drowned not throwing.
    assert_ne!(
        entity_model_mesh(&[drowned(false)]).vertices,
        entity_model_mesh(&[drowned(true)]).vertices,
        "a throwing drowned no longer holds the plain held-out arms"
    );
}

#[test]
fn zombie_baby_model_parts_match_vanilla_26_1_body_layer() {
    // The baby zombie head carries the base cube plus the `0.25` deformation overlay (which keeps
    // the base 6x6x6 box as uv_size).
    assert_eq!(
        BABY_ZOMBIE_HEAD,
        [
            ModelCube::new(
                [-3.0, -6.25, -3.0],
                [6.0, 6.0, 6.0],
                ZOMBIE_GREEN,
                [6.0, 6.0, 6.0],
                [3.0, 3.0],
                false,
            ),
            ModelCube::new(
                [-3.25, -6.4, -3.25],
                [6.5, 6.5, 6.5],
                ZOMBIE_GREEN,
                [6.0, 6.0, 6.0],
                [35.0, 3.0],
                false,
            ),
        ]
    );
    assert_eq!(BABY_ZOMBIE_BODY[0].size, [4.0, 5.0, 2.0]);
    assert_eq!(BABY_ZOMBIE_RIGHT_ARM[0].size, [2.0, 5.0, 2.0]);
    assert_eq!(BABY_ZOMBIE_RIGHT_LEG[0].size, [2.0, 4.0, 2.0]);
}

#[test]
fn zombie_villager_model_parts_match_vanilla_26_1_body_layers() {
    // The zombie villager builds a named-children tree (adult `head` -> `hat` -> `hat_rim`; baby
    // `head` -> `hat`, `hat_rim`, `nose`), so the head look resolves the `head` child by name; the
    // geometry is asserted on the per-part unified cube consts.
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HEAD[0].size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HEAD[1].size, [2.0, 4.0, 2.0]); // nose
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT[0].size, [9.0, 11.0, 9.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT_RIM[0].size, [16.0, 16.0, 1.0]);
    // The deformed body overlay inflates its colored geometry but keeps the base box as uv_size.
    assert_eq!(ADULT_ZOMBIE_VILLAGER_BODY[1].size, [8.1, 20.1, 6.1]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_BODY[1].uv_size, [8.0, 20.0, 6.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_RIGHT_ARM[0].size, [4.0, 12.0, 4.0]);
    assert!(!ADULT_ZOMBIE_VILLAGER_RIGHT_ARM[0].mirror);
    assert!(ADULT_ZOMBIE_VILLAGER_LEFT_ARM[0].mirror);
    assert!(!ADULT_ZOMBIE_VILLAGER_RIGHT_LEG[0].mirror);
    assert!(ADULT_ZOMBIE_VILLAGER_LEFT_LEG[0].mirror);

    assert_eq!(BABY_ZOMBIE_VILLAGER_BODY[1].size, [4.2, 6.2, 3.2]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_BODY[1].uv_size, [4.0, 6.0, 3.0]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_HEAD[0].size, [8.0, 8.0, 7.0]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_HAT[0].size, [8.6, 8.6, 7.6]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_HAT_RIM[0].size, [14.0, 1.0, 12.0]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_NOSE[0].size, [2.0, 2.0, 1.0]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_RIGHT_ARM[0].size, [2.0, 5.0, 2.0]);
    assert_eq!(BABY_ZOMBIE_VILLAGER_RIGHT_LEG[0].size, [2.0, 3.0, 2.0]);
}

#[test]
fn zombie_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::zombie(55, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    // The baby zombie's held-out arms reach forward the same way (scaled down): max Z grows
    // from 0.203125 to 0.29263186 and X widens slightly.
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25911528, 64.001, -0.203125]);
    assert_close3(max, [0.25911525, 64.947876, 0.29263186]);
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
    // The held-out arms reach forward on the husk too (max Z 0.29882815 -> 0.6935429).
    let (husk_min, husk_max) = mesh_extents(&husk);
    assert_close3(husk_min, [-0.5553031, 64.00106, -0.29882815]);
    assert_close3(husk_max, [0.5553031, 66.15926, 0.6935429]);

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
    // The zombie villager's held-out arms reach forward (max Z 0.5 -> 0.65274626); its robe
    // still bounds min Z at -0.5.
    let (zombie_villager_min, zombie_villager_max) = mesh_extents(&zombie_villager);
    assert_close3(zombie_villager_min, [-0.5226382, 64.001, -0.50000006]);
    assert_close3(zombie_villager_max, [0.5226382, 66.15725, 0.65274626]);

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
    use crate::entity_models::geometry::{ModelCubeDesc, ModelPartDesc};
    use std::borrow::Cow;

    // The desc-level `humanoid_limb_swing_parts` reference helper is exercised against an inline
    // humanoid body-layer fixture (the zombie now builds a named tree, so it has no `*_PARTS` desc
    // const). The arm part offsets sit at indices 2/3 (x = ∓5), the leg parts at 4/5 (x = ∓1.9),
    // matching the vanilla `HumanoidModel.createMesh` layout.
    fn humanoid_part(offset: [f32; 3]) -> ModelPartDesc {
        ModelPartDesc {
            pose: PartPose {
                offset,
                rotation: [0.0, 0.0, 0.0],
            },
            cubes: &[ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, 12.0, 4.0],
                color: ZOMBIE_GREEN,
            }],
            children: &[],
        }
    }
    let fixture: [ModelPartDesc; 6] = [
        humanoid_part([0.0, 0.0, 0.0]),
        humanoid_part([0.0, 0.0, 0.0]),
        humanoid_part([-5.0, 2.0, 0.0]),
        humanoid_part([5.0, 2.0, 0.0]),
        humanoid_part([-1.9, 12.0, 0.0]),
        humanoid_part([1.9, 12.0, 0.0]),
    ];

    // Vanilla HumanoidModel.setupAnim: rightLeg.xRot = cos(pos * 0.6662) * 1.4 *
    // speed (in phase), leftLeg.xRot = cos(pos * 0.6662 + π) * 1.4 * speed (out of
    // phase). rightLeg (offset x = -1.9) is index 4 and leftLeg (x = +1.9) index 5.
    // With pos = 0, speed = 1: rightLeg = 1.4, leftLeg = -1.4.
    let posed =
        humanoid_limb_swing_parts(Cow::Borrowed(&fixture), HUMANOID_LEG_PART_INDICES, 0.0, 1.0);
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
    assert_eq!(posed[2].pose.rotation, fixture[2].pose.rotation);
    assert_eq!(posed[3].pose.rotation, fixture[3].pose.rotation);

    // A general (pos, speed) reproduces the cos(pos * 0.6662 [+ π]) * 1.4 * speed
    // formula including the 0.6662 frequency factor.
    let posed =
        humanoid_limb_swing_parts(Cow::Borrowed(&fixture), HUMANOID_LEG_PART_INDICES, 1.5, 0.5);
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

    // Adult: vanilla HumanoidModel.createMesh UVs (texture 64x64), carried on the unified cubes'
    // `.tex` field. The deformed hat keeps the base 8x8x8 box as its uv_size; the left arm/leg
    // mirror the right's texOffs.
    assert_eq!(ADULT_ZOMBIE_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(ADULT_ZOMBIE_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_HAT[0].tex, [32.0, 0.0]);
    assert_eq!(ADULT_ZOMBIE_HAT[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_HAT[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(ADULT_ZOMBIE_BODY[0].tex, [16.0, 16.0]);
    assert_eq!(ADULT_ZOMBIE_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert!(!ADULT_ZOMBIE_RIGHT_ARM[0].mirror);
    assert_eq!(ADULT_ZOMBIE_LEFT_ARM[0].tex, [40.0, 16.0]);
    assert!(ADULT_ZOMBIE_LEFT_ARM[0].mirror);
    assert_eq!(ADULT_ZOMBIE_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert!(!ADULT_ZOMBIE_RIGHT_LEG[0].mirror);
    assert_eq!(ADULT_ZOMBIE_LEFT_LEG[0].tex, [0.0, 16.0]);
    assert!(ADULT_ZOMBIE_LEFT_LEG[0].mirror);

    // Baby: vanilla BabyZombieModel.createBodyLayer UVs. Each limb has its own texOffs (no
    // mirroring); the head carries the base cube plus the 0.25 deformation overlay.
    assert_eq!(BABY_ZOMBIE_BODY[0].tex, [16.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_HEAD[0].tex, [3.0, 3.0]);
    assert_eq!(BABY_ZOMBIE_HEAD[0].uv_size, [6.0, 6.0, 6.0]);
    assert_eq!(BABY_ZOMBIE_HEAD[1].tex, [35.0, 3.0]);
    assert_eq!(BABY_ZOMBIE_HEAD[1].uv_size, [6.0, 6.0, 6.0]);
    assert_eq!(BABY_ZOMBIE_HEAD[1].size, [6.5, 6.5, 6.5]);
    assert_eq!(BABY_ZOMBIE_RIGHT_ARM[0].tex, [36.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_LEFT_ARM[0].tex, [28.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_RIGHT_LEG[0].tex, [8.0, 16.0]);
    assert_eq!(BABY_ZOMBIE_LEFT_LEG[0].tex, [0.0, 16.0]);
    for cube in BABY_ZOMBIE_BODY
        .iter()
        .chain(&BABY_ZOMBIE_HEAD)
        .chain(&BABY_ZOMBIE_RIGHT_ARM)
        .chain(&BABY_ZOMBIE_LEFT_ARM)
        .chain(&BABY_ZOMBIE_RIGHT_LEG)
        .chain(&BABY_ZOMBIE_LEFT_LEG)
    {
        assert!(!cube.mirror, "baby zombie cubes are never mirrored");
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

#[test]
fn husk_textured_layer_passes_reuse_the_zombie_body_layer() {
    // Vanilla `HuskRenderer extends ZombieRenderer`: `ModelLayers.HUSK` is the shared
    // `humanoidBodyLayer` (the adult husk mesh is scaled at the root, not in its UVs) and
    // `HUSK_BABY` is the shared `babyZombieLayer`, so the husk's textured parts are byte-for-byte
    // the zombie's, with only the texture and the adult scale changing.
    assert_eq!(MODEL_LAYER_HUSK, "minecraft:husk#main");
    assert_eq!(MODEL_LAYER_HUSK_BABY, "minecraft:husk_baby#main");

    // The husk reuses the unified `ZombieVariantModel` (plain-zombie) tree, so the layer-pass
    // geometry is vestigial.
    let adult = husk_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    let baby = husk_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
}

#[test]
fn husk_textured_layer_passes_match_vanilla_renderer() {
    for (baby, model_layer, texture) in [
        (false, "minecraft:husk#main", HUSK_TEXTURE_REF),
        (true, "minecraft:husk_baby#main", HUSK_BABY_TEXTURE_REF),
    ] {
        let passes = husk_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::HuskBase);
        assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    assert!(entity_model_texture_refs().contains(&HUSK_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&HUSK_BABY_TEXTURE_REF));
    assert_eq!(
        husk_entity_texture_refs(),
        &[HUSK_TEXTURE_REF, HUSK_BABY_TEXTURE_REF]
    );
}

#[test]
fn husk_textured_mesh_matches_colored_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&husk_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [EntityModelInstance::zombie_variant(
            56,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Husk,
            baby,
        )];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured husk shares the colored geometry exactly, including the adult's 1.0625
        // root scale (`huskScale`): same cube count and bounds.
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

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the legs
        // (the held-out arms stay deferred, like the colored path).
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

fn husk_texture_images() -> Vec<EntityModelTextureImage> {
    husk_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn drowned_textured_layer_passes_reuse_the_zombie_body_layer() {
    assert_eq!(MODEL_LAYER_DROWNED, "minecraft:drowned#main");
    assert_eq!(MODEL_LAYER_DROWNED_BABY, "minecraft:drowned_baby#main");
    assert_eq!(MODEL_LAYER_DROWNED_OUTER_LAYER, "minecraft:drowned#outer");
    assert_eq!(
        MODEL_LAYER_DROWNED_BABY_OUTER_LAYER,
        "minecraft:drowned_baby#outer"
    );

    // Vanilla `DrownedModel.createBodyLayer extends ZombieModel`; the non-swimming drowned reuses
    // the unified `ZombieVariantModel` (plain-zombie) tree for the base body, so the base layer-pass
    // geometry is vestigial. The drowned's distinct left-limb `texOffs(32, 48)`/`texOffs(16, 48)`
    // live on the adult outer-layer overlay; the swim re-pose defers.
    assert_eq!(DROWNED_OUTER_LEFT_ARM.tex, [32.0, 48.0]);
    assert!(!DROWNED_OUTER_LEFT_ARM.mirror);
    assert_eq!(DROWNED_OUTER_LEFT_LEG.tex, [16.0, 48.0]);
    assert!(!DROWNED_OUTER_LEFT_LEG.mirror);
    // The shared `createMesh(0.25)` cubes are byte-identical to the stray frost overlay.
    assert_eq!(DROWNED_OUTER_LEFT_ARM.size, STRAY_OUTER_LEFT_ARM.size);
    assert_eq!(DROWNED_OUTER_LEFT_ARM.uv_size, STRAY_OUTER_LEFT_ARM.uv_size);

    // The baby outer layer is the distinct `BabyZombieModel.createBodyLayer(0.25)` mesh: the baby
    // zombie's own un-mirrored arm/leg `texOffs` (NOT the drowned overrides), each box inflated by
    // 0.25 over the base baby box while keeping the base box as `uv_size`.
    assert_eq!(BABY_DROWNED_OUTER_LEFT_ARM[0].tex, [28.0, 16.0]);
    assert_eq!(BABY_DROWNED_OUTER_LEFT_LEG[0].tex, [0.0, 16.0]);
    assert_eq!(BABY_DROWNED_OUTER_BODY[0].uv_size, [4.0, 5.0, 2.0]);
    assert_eq!(BABY_DROWNED_OUTER_BODY[0].size, [4.5, 5.5, 2.5]);
    assert_eq!(BABY_DROWNED_OUTER_HEAD.len(), 2);
    assert_eq!(BABY_DROWNED_OUTER_HEAD[1].tex, [35.0, 3.0]);
    for cube in BABY_DROWNED_OUTER_BODY
        .iter()
        .chain(&BABY_DROWNED_OUTER_HEAD)
        .chain(&BABY_DROWNED_OUTER_RIGHT_ARM)
        .chain(&BABY_DROWNED_OUTER_LEFT_ARM)
        .chain(&BABY_DROWNED_OUTER_RIGHT_LEG)
        .chain(&BABY_DROWNED_OUTER_LEFT_LEG)
    {
        assert!(!cube.mirror, "baby drowned outer cubes are never mirrored");
    }
}

#[test]
fn drowned_textured_layer_passes_match_vanilla_renderer() {
    // Adult: base body (`drowned.png`) + the always-on white `DrownedOuterLayer` overlay
    // (`drowned_outer_layer.png`). Baby: base body only — the baby outer layer defers.
    let adult = drowned_textured_layer_passes(false);
    assert_eq!(adult.len(), 2);
    assert_eq!(adult[0].kind, EntityModelLayerKind::DrownedBase);
    assert_eq!(adult[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(adult[0].model_layer, "minecraft:drowned#main");
    assert_eq!(adult[0].texture, DROWNED_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);

    assert_eq!(adult[1].kind, EntityModelLayerKind::DrownedOuter);
    assert_eq!(adult[1].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(adult[1].model_layer, "minecraft:drowned#outer");
    assert_eq!(adult[1].texture, DROWNED_OUTER_LAYER_TEXTURE_REF);
    assert_eq!(adult[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[1].collector_order, adult[1].submit_sequence), (1, 1));

    let baby = drowned_textured_layer_passes(true);
    assert_eq!(baby.len(), 2);
    assert_eq!(baby[0].kind, EntityModelLayerKind::DrownedBase);
    assert_eq!(baby[0].model_layer, "minecraft:drowned_baby#main");
    assert_eq!(baby[0].texture, DROWNED_BABY_TEXTURE_REF);
    assert_eq!(baby[1].kind, EntityModelLayerKind::DrownedOuter);
    assert_eq!(baby[1].model_layer, "minecraft:drowned_baby#outer");
    assert_eq!(baby[1].texture, DROWNED_OUTER_LAYER_BABY_TEXTURE_REF);
    assert_eq!((baby[1].collector_order, baby[1].submit_sequence), (1, 1));

    assert!(entity_model_texture_refs().contains(&DROWNED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_OUTER_LAYER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_OUTER_LAYER_BABY_TEXTURE_REF));
    assert_eq!(
        drowned_entity_texture_refs(),
        &[DROWNED_TEXTURE_REF, DROWNED_BABY_TEXTURE_REF]
    );
}

#[test]
fn drowned_textured_mesh_matches_colored_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&drowned_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [EntityModelInstance::zombie_variant(
            57,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            baby,
        )];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured drowned shares the colored geometry exactly (drowned only changes the left
        // arm/leg UVs, not their geometry): same cube count and bounds.
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

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the legs
        // (the held-out arms and the swim/outer layers stay deferred, like the colored path).
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

fn drowned_texture_images() -> Vec<EntityModelTextureImage> {
    drowned_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn drowned_outer_layer_overlays_the_inflated_white_shell() {
    // Vanilla `DrownedOuterLayer`: an always-on white cutout copy of the inflated
    // `DrownedModel.createBodyLayer(0.25)` shell (`drowned_outer_layer.png`), posed by the same
    // animator as the base so it tracks the limbs. The seven inflated boxes add 42 cutout faces.
    let mut images = drowned_texture_images();
    for outer in [
        DROWNED_OUTER_LAYER_TEXTURE_REF,
        DROWNED_OUTER_LAYER_BABY_TEXTURE_REF,
    ] {
        let len = usize::try_from(outer.size[0] * outer.size[1] * 4).unwrap();
        images.push(EntityModelTextureImage::new(outer, vec![200u8; len]));
    }
    let (with_outer, _) = build_entity_model_texture_atlas(&images).unwrap();
    let (base_only, _) = build_entity_model_texture_atlas(&drowned_texture_images()).unwrap();

    let instances = [EntityModelInstance::zombie_variant(
        57,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    )];
    let base = entity_model_textured_mesh(&instances, &base_only);
    let outer = entity_model_textured_mesh(&instances, &with_outer);

    // The outer adds exactly the inflated humanoid shell (7 boxes * 6 faces).
    assert_eq!(outer.cutout_faces, base.cutout_faces + 42);
    assert!(outer.vertices.len() > base.vertices.len());
    // The white outer copy keeps the [1, 1, 1, 1] tint.
    assert!(outer
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The 0.25 inflation makes the shell strictly larger than the base body in every axis (a
    // rotation-independent claim: every inflated cube grows by 0.5 about its own centre).
    let (bmin, bmax) = textured_mesh_extents(&base);
    let (omin, omax) = textured_mesh_extents(&outer);
    for axis in 0..3 {
        assert!(
            omax[axis] - omin[axis] > bmax[axis] - bmin[axis],
            "outer shell wider on axis {axis}"
        );
    }

    // The overlay is posed by the same animator, so advancing the walk re-poses it too.
    let walking = [instances[0]
        .with_walk_animation(2.0, 1.0)
        .with_age_in_ticks(8.0)];
    let outer_walk = entity_model_textured_mesh(&walking, &with_outer);
    assert_ne!(
        outer.vertices, outer_walk.vertices,
        "outer shell tracks the limbs"
    );

    // The baby drowned carries its own distinct outer shell (`BabyDrownedModel.createBodyLayer(0.25)`),
    // which adds the same seven inflated boxes (42 faces) over the baby body and tracks it past the
    // base baby body in every direction.
    let baby = [EntityModelInstance::zombie_variant(
        58,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        true,
    )];
    let baby_base = entity_model_textured_mesh(&baby, &base_only);
    let baby_outer = entity_model_textured_mesh(&baby, &with_outer);
    assert_eq!(baby_outer.cutout_faces, baby_base.cutout_faces + 42);
    assert!(baby_outer
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (bbmin, bbmax) = textured_mesh_extents(&baby_base);
    let (bomin, bomax) = textured_mesh_extents(&baby_outer);
    for axis in 0..3 {
        assert!(
            bomax[axis] - bomin[axis] > bbmax[axis] - bbmin[axis],
            "baby outer shell wider on axis {axis}"
        );
    }
}

#[test]
fn zombie_villager_textured_parts_match_vanilla_body_layer_uv_sources() {
    assert_eq!(
        MODEL_LAYER_ZOMBIE_VILLAGER,
        "minecraft:zombie_villager#main"
    );
    assert_eq!(
        MODEL_LAYER_ZOMBIE_VILLAGER_BABY,
        "minecraft:zombie_villager_baby#main"
    );

    // Adult: vanilla `ZombieVillagerModel.createBodyLayer` UVs (64x64), carried on the unified
    // cubes' `.tex` field. Head + nose, hat (deform 0.5, base 8x10x8) with the rotated hat rim
    // child, body inner + 0.05 robe overlay, arms (left mirrors the right's texOffs(44, 22)), legs
    // (left mirrors texOffs(0, 22)).
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HEAD[0].tex, [0.0, 0.0]); // head
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HEAD[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HEAD[1].tex, [24.0, 0.0]); // nose
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT[0].tex, [32.0, 0.0]); // hat texOffs(32, 0)
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT[0].size, [9.0, 11.0, 9.0]); // deform 0.5 geometry
    assert_eq!(ADULT_ZOMBIE_VILLAGER_HAT_RIM[0].tex, [30.0, 47.0]); // hat rim texOffs(30, 47)
    assert_eq!(ADULT_ZOMBIE_VILLAGER_BODY[0].tex, [16.0, 20.0]); // body
    assert_eq!(ADULT_ZOMBIE_VILLAGER_BODY[1].tex, [0.0, 38.0]); // robe overlay
    assert_eq!(ADULT_ZOMBIE_VILLAGER_BODY[1].uv_size, [8.0, 20.0, 6.0]);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_RIGHT_ARM[0].tex, [44.0, 22.0]); // right arm
    assert!(!ADULT_ZOMBIE_VILLAGER_RIGHT_ARM[0].mirror);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_LEFT_ARM[0].tex, [44.0, 22.0]); // left arm mirror
    assert!(ADULT_ZOMBIE_VILLAGER_LEFT_ARM[0].mirror);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_RIGHT_LEG[0].tex, [0.0, 22.0]); // right leg
    assert!(!ADULT_ZOMBIE_VILLAGER_RIGHT_LEG[0].mirror);
    assert_eq!(ADULT_ZOMBIE_VILLAGER_LEFT_LEG[0].tex, [0.0, 22.0]); // left leg mirror
    assert!(ADULT_ZOMBIE_VILLAGER_LEFT_LEG[0].mirror);

    // Baby: vanilla `BabyZombieVillagerModel.createBodyLayer`. Body, head with hat/hat-rim/nose
    // children; each limb has its own texOffs (no mirroring).
    assert_eq!(BABY_ZOMBIE_VILLAGER_BODY[0].tex, [0.0, 15.0]); // body
    assert_eq!(BABY_ZOMBIE_VILLAGER_BODY[1].tex, [16.0, 22.0]); // body overlay
    assert_eq!(BABY_ZOMBIE_VILLAGER_HEAD[0].tex, [0.0, 0.0]); // head
    assert_eq!(BABY_ZOMBIE_VILLAGER_HAT[0].tex, [0.0, 31.0]); // hat
    assert_eq!(BABY_ZOMBIE_VILLAGER_HAT_RIM[0].tex, [0.0, 46.0]); // hat rim
    assert_eq!(BABY_ZOMBIE_VILLAGER_NOSE[0].tex, [23.0, 0.0]); // nose
    assert_eq!(BABY_ZOMBIE_VILLAGER_RIGHT_ARM[0].tex, [24.0, 15.0]); // right arm
    assert_eq!(BABY_ZOMBIE_VILLAGER_LEFT_ARM[0].tex, [16.0, 15.0]); // left arm
    assert_eq!(BABY_ZOMBIE_VILLAGER_RIGHT_LEG[0].tex, [8.0, 23.0]); // right leg
    assert_eq!(BABY_ZOMBIE_VILLAGER_LEFT_LEG[0].tex, [0.0, 23.0]); // left leg
    for cube in BABY_ZOMBIE_VILLAGER_BODY
        .iter()
        .chain(&BABY_ZOMBIE_VILLAGER_HEAD)
        .chain(&BABY_ZOMBIE_VILLAGER_HAT)
        .chain(&BABY_ZOMBIE_VILLAGER_HAT_RIM)
        .chain(&BABY_ZOMBIE_VILLAGER_NOSE)
        .chain(&BABY_ZOMBIE_VILLAGER_RIGHT_ARM)
        .chain(&BABY_ZOMBIE_VILLAGER_LEFT_ARM)
        .chain(&BABY_ZOMBIE_VILLAGER_RIGHT_LEG)
        .chain(&BABY_ZOMBIE_VILLAGER_LEFT_LEG)
    {
        assert!(
            !cube.mirror,
            "baby zombie villager cubes are never mirrored"
        );
    }
}

#[test]
fn zombie_villager_textured_layer_passes_match_vanilla_renderer() {
    for (baby, model_layer, texture) in [
        (
            false,
            "minecraft:zombie_villager#main",
            ZOMBIE_VILLAGER_TEXTURE_REF,
        ),
        (
            true,
            "minecraft:zombie_villager_baby#main",
            ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
        ),
    ] {
        let passes = zombie_villager_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::ZombieVillagerBase);
        assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    assert!(entity_model_texture_refs().contains(&ZOMBIE_VILLAGER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_VILLAGER_BABY_TEXTURE_REF));
    assert_eq!(
        zombie_villager_entity_texture_refs(),
        &[
            ZOMBIE_VILLAGER_TEXTURE_REF,
            ZOMBIE_VILLAGER_BABY_TEXTURE_REF
        ]
    );
}

#[test]
fn zombie_villager_textured_mesh_matches_colored_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&zombie_villager_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [EntityModelInstance::zombie_variant(
            58,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            baby,
        )];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured zombie villager shares the colored geometry exactly.
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

        // Walking re-poses the legs on both render paths (the held-out arms stay deferred).
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

fn zombie_villager_texture_images() -> Vec<EntityModelTextureImage> {
    zombie_villager_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
