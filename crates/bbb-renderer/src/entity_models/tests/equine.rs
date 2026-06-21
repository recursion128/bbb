use super::*;

#[test]
fn horse_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_HORSE_BODY[0],
        ModelCubeDesc {
            min: [-5.05, -8.05, -17.05],
            size: [10.1, 10.1, 22.1],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(
        ADULT_HORSE_EAR[0],
        ModelCubeDesc {
            min: [0.551, -12.999, 4.001],
            size: [1.998, 2.998, 0.998],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(ADULT_HORSE_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_HORSE_PARTS[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_BODY_CHILDREN[0],
        [0.0, -5.0, 2.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_TAIL.as_slice(),
    );
    assert_part_tree(
        &ADULT_HORSE_PARTS[1],
        [0.0, 4.0, -12.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_NECK.as_slice(),
        ADULT_HORSE_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_HEAD.as_slice(),
        ADULT_HORSE_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_MANE.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_HEAD_PARTS_CHILDREN[2],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_UPPER_MOUTH.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[2],
        [4.0, 14.0, 7.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_LEFT_HIND_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[3],
        [-4.0, 14.0, 7.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_HIND_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[4],
        [4.0, 14.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_LEFT_FRONT_LEG.as_slice(),
    );
    assert_part(
        &ADULT_HORSE_PARTS[5],
        [-4.0, 14.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_RIGHT_FRONT_LEG.as_slice(),
    );

    assert_eq!(
        BABY_HORSE_HEAD[0],
        ModelCubeDesc {
            min: [-3.0, -3.9484, -6.705],
            size: [6.0, 4.0, 9.0],
            color: HORSE_BROWN,
        }
    );
    assert_eq!(BABY_HORSE_PARTS.len(), 6);
    assert_part_tree(
        &BABY_HORSE_PARTS[0],
        [0.0, 12.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_HORSE_BODY.as_slice(),
        BABY_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HORSE_BODY_CHILDREN[0],
        [0.0, -1.0, 7.0],
        [-0.7418, 0.0, 0.0],
        BABY_HORSE_TAIL.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &BABY_HORSE_PARTS[1],
            [2.4, 16.0, 5.4],
            BABY_HORSE_LEFT_HIND_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[2],
            [-2.4, 16.0, 5.4],
            BABY_HORSE_RIGHT_HIND_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[3],
            [2.4, 16.0, -5.4],
            BABY_HORSE_LEFT_FRONT_LEG.as_slice(),
        ),
        (
            &BABY_HORSE_PARTS[4],
            [-2.4, 16.0, -5.4],
            BABY_HORSE_RIGHT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
    assert_part_tree(
        &BABY_HORSE_PARTS[5],
        [0.0, 10.0, -6.0],
        [0.6109, 0.0, 0.0],
        BABY_HORSE_NECK.as_slice(),
        BABY_HORSE_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &BABY_HORSE_HEAD_PARTS_CHILDREN[0],
        [0.0, -6.0516, -0.2951],
        [0.0, 0.0, 0.0],
        BABY_HORSE_HEAD.as_slice(),
        BABY_HORSE_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HORSE_HEAD_CHILDREN[0],
        [2.0, -4.2484, 1.9451],
        [0.0, 0.0, 0.2618],
        BABY_HORSE_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HORSE_HEAD_CHILDREN[1],
        [-2.0, -4.2484, 1.645],
        [0.0, 0.0, -0.2618],
        BABY_HORSE_RIGHT_EAR.as_slice(),
    );
}

#[test]
fn horse_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::horse(
        150,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    let (adult_min, adult_max) = mesh_extents(&adult);
    assert_close3(adult_min, [-0.34718758, 64.001785, -1.200657]);
    assert_close3(adult_max, [0.34718758, 66.32189, 1.6198997]);

    let baby = entity_model_mesh(&[EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    // Vanilla runs `setupAnim` every frame, so a standing baby horse's tail sits at the
    // overridden `getTailXRotOffset() + π/6 = −π/2 + π/6 = −1.0472`, not the layer's
    // `−0.7418`. The steeper tail tucks the tail box in (less reach back, less reach down),
    // shifting the back/forward extents from the un-posed layer box.
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert_close3(baby_min, [-0.25000003, 64.001, -0.7374399]);
    assert_close3(baby_max, [0.25000003, 65.636024, 1.0663916]);
}

#[test]
fn horse_texture_refs_match_vanilla_renderer_defaults() {
    assert_eq!(EntityModelKind::Horse { baby: false }.model_key(), "horse");
    assert_eq!(
        EntityModelKind::Horse { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/horse/horse_white.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Horse { baby: true }.model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Horse { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/horse/horse_white_baby.png",
            size: [64, 64],
        })
    );
}

#[test]
fn donkey_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_DONKEY_CHEST[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 8.0, 3.0],
            color: DONKEY_GRAY,
        }
    );
    assert_eq!(
        ADULT_DONKEY_EAR[0],
        ModelCubeDesc {
            min: [-1.0, -7.0, 0.0],
            size: [2.0, 7.0, 1.0],
            color: DONKEY_GRAY,
        }
    );
    assert_eq!(ADULT_DONKEY_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_DONKEY_PARTS[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_HORSE_BODY_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_PARTS_WITH_CHEST[0],
        [0.0, 11.0, 5.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_BODY.as_slice(),
        ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[0],
        [0.0, -5.0, 2.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_TAIL.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[1],
        [6.0, -8.0, 0.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        ADULT_DONKEY_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_BODY_CHILDREN_WITH_CHEST[2],
        [-6.0, -8.0, 0.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        ADULT_DONKEY_CHEST.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_PARTS[1],
        [0.0, 4.0, -12.0],
        [std::f32::consts::FRAC_PI_6, 0.0, 0.0],
        ADULT_HORSE_NECK.as_slice(),
        ADULT_DONKEY_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_DONKEY_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HORSE_HEAD.as_slice(),
        ADULT_DONKEY_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_HEAD_CHILDREN[0],
        [1.25, -10.0, 4.0],
        [0.2617994, 0.0, 0.2617994],
        ADULT_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &ADULT_DONKEY_HEAD_CHILDREN[1],
        [-1.25, -10.0, 4.0],
        [0.2617994, 0.0, -0.2617994],
        ADULT_DONKEY_EAR.as_slice(),
    );

    assert_eq!(BABY_DONKEY_PARTS.len(), 1);
    assert_part_tree(
        &BABY_DONKEY_PARTS[0],
        [1.0, 14.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_DONKEY_BODY.as_slice(),
        BABY_DONKEY_BODY_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &BABY_DONKEY_BODY_CHILDREN[0],
        [0.0, -1.5, 6.5],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_TAIL_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-0.7418, 0.0, 0.0],
        BABY_DONKEY_TAIL_R1.as_slice(),
    );
    for (part, expected_offset) in [
        (&BABY_DONKEY_BODY_CHILDREN[1], [2.25, 3.5, 5.25]),
        (&BABY_DONKEY_BODY_CHILDREN[2], [-2.4, 3.5, 5.4]),
        (&BABY_DONKEY_BODY_CHILDREN[3], [2.4, 3.5, -5.3]),
        (&BABY_DONKEY_BODY_CHILDREN[4], [-2.4, 3.5, -5.4]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_DONKEY_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &BABY_DONKEY_BODY_CHILDREN[5],
        [0.0, -3.0, -5.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_HEAD_PARTS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_PARTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.3927, 0.0, 0.0],
        BABY_DONKEY_NECK_R1.as_slice(),
    );
    assert_part_tree(
        &BABY_DONKEY_HEAD_PARTS_CHILDREN[1],
        [0.0, -5.0, -3.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_DONKEY_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[0],
        [0.0, -1.0, 1.0],
        [0.3927, 0.0, 0.0],
        BABY_DONKEY_HEAD_R1.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[1],
        [2.0, -3.5, -1.0],
        [0.48, 0.0, 0.48],
        BABY_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_HEAD_CHILDREN[2],
        [-2.0, -3.5, -1.0],
        [0.48, 0.0, -0.48],
        BABY_DONKEY_EAR.as_slice(),
    );
    assert_part(
        &BABY_DONKEY_BODY_CHILDREN[6],
        [-1.0, 10.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
    );
    assert_part(
        &BABY_DONKEY_BODY_CHILDREN[7],
        [-1.0, 10.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
    );
}

#[test]
fn donkey_meshes_use_vanilla_body_layer_geometry_and_chest_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::donkey(
        160,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DONKEY_GRAY, 0.78)));

    let with_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        161,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        true,
    )]);
    assert_eq!(with_chest.opaque_faces, 84);
    assert_eq!(with_chest.vertices.len(), 336);
    assert_eq!(with_chest.indices.len(), 504);

    let mule = entity_model_mesh(&[EntityModelInstance::donkey(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    )]);
    assert_eq!(mule.opaque_faces, 72);
    assert!(mule
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(MULE_BROWN, 0.78)));
    let (donkey_min, donkey_max) = mesh_extents(&adult);
    let (mule_min, mule_max) = mesh_extents(&mule);
    assert!(mule_max[1] > donkey_max[1]);
    assert!(mule_min[2] < donkey_min[2]);

    let baby_without_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    )]);
    let baby_with_chest = entity_model_mesh(&[EntityModelInstance::donkey(
        164,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        true,
    )]);
    assert_eq!(baby_without_chest.opaque_faces, 60);
    assert_eq!(baby_without_chest.vertices.len(), 240);
    assert_eq!(baby_without_chest.indices.len(), 360);
    assert_same_geometry(&baby_with_chest, &baby_without_chest);
}

#[test]
fn donkey_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            DonkeyModelFamily::Donkey,
            false,
            "donkey",
            EntityModelTextureRef {
                path: "textures/entity/horse/donkey.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Donkey,
            true,
            "donkey_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/donkey_baby.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Mule,
            false,
            "mule",
            EntityModelTextureRef {
                path: "textures/entity/horse/mule.png",
                size: [64, 64],
            },
        ),
        (
            DonkeyModelFamily::Mule,
            true,
            "mule_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/mule_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Donkey {
            family,
            baby,
            has_chest: true,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn undead_horse_meshes_use_unscaled_vanilla_horse_layers() {
    let skeleton_adult = entity_model_mesh(&[EntityModelInstance::undead_horse(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )]);
    assert_eq!(skeleton_adult.opaque_faces, 72);
    assert_eq!(skeleton_adult.vertices.len(), 288);
    assert_eq!(skeleton_adult.indices.len(), 432);
    assert!(skeleton_adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SKELETON_HORSE_BONE, 0.78)));
    let (skeleton_min, skeleton_max) = mesh_extents(&skeleton_adult);
    assert_close3(skeleton_min, [-0.31562507, 64.001625, -1.0915062]);
    assert_close3(skeleton_max, [0.31562507, 66.11081, 1.4726361]);

    let base_horse_adult = entity_model_mesh(&[EntityModelInstance::horse(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (horse_min, horse_max) = mesh_extents(&base_horse_adult);
    assert!(horse_max[1] > skeleton_max[1]);
    assert!(horse_min[2] < skeleton_min[2]);

    let zombie_baby = entity_model_mesh(&[EntityModelInstance::undead_horse(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        true,
    )]);
    let base_horse_baby =
        entity_model_mesh(&[EntityModelInstance::horse(173, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(zombie_baby.opaque_faces, 60);
    assert_same_geometry(&zombie_baby, &base_horse_baby);
    assert!(zombie_baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIE_HORSE_GREEN, 0.78)));
}

#[test]
fn undead_horse_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            UndeadHorseModelFamily::Skeleton,
            false,
            "skeleton_horse",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_skeleton.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Skeleton,
            true,
            "skeleton_horse_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_skeleton_baby.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Zombie,
            false,
            "zombie_horse",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_zombie.png",
                size: [64, 64],
            },
        ),
        (
            UndeadHorseModelFamily::Zombie,
            true,
            "zombie_horse_baby",
            EntityModelTextureRef {
                path: "textures/entity/horse/horse_zombie_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::UndeadHorse { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn equine_swings_its_legs_when_walking() {
    // Vanilla `AbstractEquineModel.setupAnim` swings the four legs with the equine gait
    // (front amplitude 0.8, hind 0.5), applies the head look/bob to the neck, and lifts the
    // tail with the gait. A standing equine with a level head is inert (the adult tail rest
    // equals the layer pose); a walking one differs. Covers horse (adult + the re-parented
    // baby layout), donkey/mule (adult + with-chest), and the undead horses.
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            37,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            true,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
        EntityModelInstance::undead_horse(
            151,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Zombie,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices.len(),
            walking.vertices.len(),
            "{:?}",
            base.kind
        );
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );
    }
}

#[test]
fn adult_equine_swings_its_legs_and_keeps_its_body_still() {
    // The adult horse/donkey/mule/undead-horse layers list the body first (its cube is
    // the first 24 vertices) and the four single-cube legs last (the final 96 vertices).
    // A walking adult equine swings those legs while the body cube stays put. (The neck
    // bobs too — checked by `adult_horse_turns_and_bobs_its_neck`; the re-parented baby
    // horse layout lists its head last, so these contiguous checks are adult-only.)
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            37,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            true,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let leg_start = rest.vertices.len() - 96;
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[leg_start..],
            walking.vertices[leg_start..],
            "{:?} the four legs swing",
            base.kind
        );
    }
}

#[test]
fn adult_horse_turns_and_bobs_its_neck() {
    // Adult horse layer (288 verts): the body and its tail child occupy blocks [0, 2) =
    // vertices [0, 48); the neck (`head_parts`) and its head/mane/upper_mouth/ear children
    // occupy blocks [2, 8) = vertices [48, 192); the four legs occupy blocks [8, 12) =
    // vertices [192, 288). The vanilla `AbstractEquineModel.setupAnim` head look turns and
    // tilts the neck subtree, and the walk bob also moves it, while neither touches the
    // body; the legs move only when walking.
    let base = EntityModelInstance::horse(160, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);

    // Standing, head yawed (30° clamps to the equine ±20° limit, still a turn): only the
    // neck subtree moves.
    let yawed = entity_model_mesh(&[base.with_head_look(30.0, 0.0)]);
    assert_eq!(
        rest.vertices[0..48],
        yawed.vertices[0..48],
        "body/tail stay put when looking"
    );
    assert_ne!(
        rest.vertices[48..192],
        yawed.vertices[48..192],
        "the neck turns"
    );
    assert_eq!(
        rest.vertices[192..288],
        yawed.vertices[192..288],
        "legs stay put when standing"
    );

    // Standing, head pitched: the neck tilts, the legs stay put.
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);
    assert_ne!(
        rest.vertices[48..192],
        pitched.vertices[48..192],
        "the neck tilts"
    );
    assert_eq!(
        rest.vertices[192..288],
        pitched.vertices[192..288],
        "legs stay put when standing"
    );

    // Walking with a level head: the body cube stays put (block 0 = vertices [0, 24)), but
    // the tail lifts with the gait (`tail.xRot += speed * 0.75`, block 1 = vertices
    // [24, 48)), the neck bobs (speed 1 > 0.2), and the legs swing.
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        rest.vertices[0..24],
        walking.vertices[0..24],
        "the body cube stays put when walking"
    );
    assert_ne!(
        rest.vertices[24..48],
        walking.vertices[24..48],
        "the tail lifts when walking"
    );
    assert_ne!(
        rest.vertices[48..192],
        walking.vertices[48..192],
        "the neck bobs when walking"
    );
    assert_ne!(
        rest.vertices[192..288],
        walking.vertices[192..288],
        "the legs swing when walking"
    );
}

#[test]
fn equine_head_look_pose_clamps_yaw_and_tilts_pitch() {
    use std::f32::consts::FRAC_PI_6;

    // ADULT_HORSE_PARTS[1] is the neck (`head_parts`); its rest xRot is the layer's π/6
    // tilt, onto which the look pitch (and walk bob) add.
    let base = ADULT_HORSE_PARTS[1].pose;
    assert!((base.rotation[0] - FRAC_PI_6).abs() < 1e-6);

    // Yaw clamps to ±20° then converts to radians; pitch adds onto the π/6 neck tilt.
    let look = equine_head_look_pose(base, 45.0, -25.0, 0.0, 0.0);
    assert!(
        (look.rotation[1] - 20.0_f32.to_radians()).abs() < 1e-6,
        "yaw clamps to +20: {}",
        look.rotation[1]
    );
    assert!((look.rotation[0] - (FRAC_PI_6 + (-25.0_f32).to_radians())).abs() < 1e-6);
    let look = equine_head_look_pose(base, -50.0, 0.0, 0.0, 0.0);
    assert!(
        (look.rotation[1] - (-20.0_f32).to_radians()).abs() < 1e-6,
        "yaw clamps to -20: {}",
        look.rotation[1]
    );
    // Within ±20° the yaw passes through unchanged.
    let look = equine_head_look_pose(base, 12.0, 0.0, 0.0, 0.0);
    assert!((look.rotation[1] - 12.0_f32.to_radians()).abs() < 1e-6);

    // The walk bob adds cos(pos * 0.8) * 0.15 * speed onto the pitch when speed > 0.2.
    let look = equine_head_look_pose(base, 0.0, 0.0, 0.0, 1.0);
    assert!(
        (look.rotation[0] - (FRAC_PI_6 + 0.15)).abs() < 1e-6,
        "bob at pos 0, speed 1: {}",
        look.rotation[0]
    );
    // A slow gait (speed <= 0.2) adds no bob.
    let look = equine_head_look_pose(base, 0.0, 0.0, 0.0, 0.2);
    assert!(
        (look.rotation[0] - FRAC_PI_6).abs() < 1e-6,
        "no bob at speed 0.2"
    );
    // A general (pos, speed) bob.
    let pos = 2.0_f32;
    let speed = 0.5_f32;
    let look = equine_head_look_pose(base, 0.0, 0.0, pos, speed);
    assert!((look.rotation[0] - (FRAC_PI_6 + (pos * 0.8).cos() * 0.15 * speed)).abs() < 1e-6);

    // zRot and offset are preserved.
    assert_eq!(look.rotation[2], base.rotation[2]);
    assert_eq!(look.offset, base.offset);
}

#[test]
fn equine_tail_swing_pose_matches_vanilla_formula() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

    // Vanilla `AbstractEquineModel.setupAnim` tail animation (default branch):
    //   tail.xRot = getTailXRotOffset() + π/6 + speed * 0.75
    //   tail.y   += speed * ageScale
    //   tail.z   += speed * 2 * ageScale
    // The adult horse tail rest pose carries the layer's π/6 xRot and offset [0, -5, 2].
    let base = ADULT_HORSE_BODY_CHILDREN[0].pose;
    assert_eq!(base.offset, [0.0, -5.0, 2.0]);
    assert!((base.rotation[0] - FRAC_PI_6).abs() < 1e-6);

    // Adult (offset 0, ageScale 1), standing: the pose equals the layer rest pose exactly.
    let rest = equine_tail_swing_pose(base, 0.0, 0.0, 1.0);
    assert_eq!(rest, base);

    // Adult, walking (speed 1): the tail lifts (+0.75 xRot) and shifts back/up.
    let walking = equine_tail_swing_pose(base, 0.0, 1.0, 1.0);
    assert!((walking.rotation[0] - (FRAC_PI_6 + 0.75)).abs() < 1e-6);
    assert!((walking.offset[1] - (-5.0 + 1.0)).abs() < 1e-6);
    assert!((walking.offset[2] - (2.0 + 2.0)).abs() < 1e-6);
    assert_eq!(walking.offset[0], base.offset[0]);
    assert_eq!(walking.rotation[1], base.rotation[1]);
    assert_eq!(walking.rotation[2], base.rotation[2]);

    // A general (offset, speed, ageScale) sample.
    let speed = 0.4_f32;
    let sample = equine_tail_swing_pose(base, -FRAC_PI_2, speed, 0.5);
    assert!((sample.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6 + speed * 0.75)).abs() < 1e-6);
    assert!((sample.offset[1] - (-5.0 + speed * 0.5)).abs() < 1e-6);
    assert!((sample.offset[2] - (2.0 + speed * 2.0 * 0.5)).abs() < 1e-6);

    // Baby horse: getTailXRotOffset = −π/2 overrides the layer's −0.7418 rest angle even
    // when standing, and ageScale = 0.5 halves the walk translation.
    let baby_base = BABY_HORSE_BODY_CHILDREN[0].pose;
    assert!((baby_base.rotation[0] - (-0.7418)).abs() < 1e-4);
    let baby_rest = equine_tail_swing_pose(baby_base, -FRAC_PI_2, 0.0, 0.5);
    assert!(
        (baby_rest.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6)).abs() < 1e-6,
        "baby tail rest overridden to −π/2 + π/6: {}",
        baby_rest.rotation[0]
    );
    assert_ne!(
        baby_rest.rotation[0], baby_base.rotation[0],
        "the override differs from the baked layer rest angle"
    );
    let baby_walking = equine_tail_swing_pose(baby_base, -FRAC_PI_2, 1.0, 0.5);
    assert!((baby_walking.rotation[0] - (-FRAC_PI_2 + FRAC_PI_6 + 0.75)).abs() < 1e-6);
    assert!((baby_walking.offset[1] - (baby_base.offset[1] + 0.5)).abs() < 1e-6);
    assert!((baby_walking.offset[2] - (baby_base.offset[2] + 1.0)).abs() < 1e-6);
}

#[test]
fn adult_equine_swings_its_tail_when_walking() {
    // Every adult equine layer lists the body cube first (block 0 = vertices [0, 24)) and
    // its tail child next (block 1 = vertices [24, 48)). A walking adult equine lifts the
    // tail (`tail.xRot += speed * 0.75`, plus a back/up shift) while the body cube stays
    // put. Covers the colored horse path and the uniform-color donkey/mule and undead-horse
    // paths (all share `emit_equine_posed`).
    for base in [
        EntityModelInstance::horse(150, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            false,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            false,
            false,
        ),
        EntityModelInstance::undead_horse(
            116,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices.len(),
            walking.vertices.len(),
            "{:?}",
            base.kind
        );
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[24..48],
            walking.vertices[24..48],
            "{:?} the tail lifts with the gait",
            base.kind
        );
    }
}

#[test]
fn baby_horse_swings_and_overrides_its_tail() {
    // `BabyHorseModel` inherits `AbstractEquineModel.setupAnim`, which both lifts the tail
    // with the gait and overrides its rest angle (`getTailXRotOffset() + π/6 = −1.0472`,
    // vs the layer's baked `−0.7418`). The baby body cube is block 0 ([0, 24)); the tail is
    // block 1 ([24, 48)). A walking baby horse lifts the tail while its body cube stays put;
    // the overridden standing rest angle is checked by `horse_meshes_use_vanilla_body_layer_geometry`
    // and `equine_tail_swing_pose_matches_vanilla_formula`. Covers the baby skeleton horse too.
    for base in [
        EntityModelInstance::horse(151, [0.0, 64.0, 0.0], 0.0, true),
        EntityModelInstance::undead_horse(
            152,
            [0.0, 64.0, 0.0],
            0.0,
            UndeadHorseModelFamily::Skeleton,
            true,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices[0..24],
            walking.vertices[0..24],
            "{:?} the baby body cube stays put",
            base.kind
        );
        assert_ne!(
            rest.vertices[24..48],
            walking.vertices[24..48],
            "{:?} the baby tail lifts when walking",
            base.kind
        );
    }
}

#[test]
fn baby_donkey_leg_swing_is_deferred() {
    // The baby donkey/mule layer re-parents its legs under the body
    // (`BabyDonkeyModel.createBabyLayer`) and overrides `setupAnim` (forcing `xRot = -30°`),
    // unlike the top-level adult layout, so its leg swing, head look, and tail lift are all
    // deferred: a walking baby donkey is unchanged for now.
    for base in [
        EntityModelInstance::donkey(
            36,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            true,
            false,
        ),
        EntityModelInstance::donkey(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Mule,
            true,
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices, walking.vertices,
            "{:?} baby leg swing deferred",
            base.kind
        );
    }
}

#[test]
fn equine_leg_swing_pose_matches_vanilla_gait() {
    // Vanilla AbstractEquineModel.setupAnim (non-standing, land): with legAnim =
    // cos(pos*0.6662 + π) * speed, leftHind = -0.5*legAnim, rightHind = +0.5*legAnim,
    // leftFront = +0.8*legAnim, rightFront = -0.8*legAnim. ADULT_HORSE_PARTS lists
    // left_hind [2], right_hind [3], left_front [4], right_front [5].
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let leg_anim = (pos * 0.6662 + std::f32::consts::PI).cos() * speed;
    let left_hind = equine_leg_swing_pose(ADULT_HORSE_PARTS[2].pose, pos, speed);
    let right_hind = equine_leg_swing_pose(ADULT_HORSE_PARTS[3].pose, pos, speed);
    let left_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, speed);
    let right_front = equine_leg_swing_pose(ADULT_HORSE_PARTS[5].pose, pos, speed);
    assert!(
        (left_hind.rotation[0] - (-0.5 * leg_anim)).abs() < 1e-6,
        "left hind"
    );
    assert!(
        (right_hind.rotation[0] - (0.5 * leg_anim)).abs() < 1e-6,
        "right hind"
    );
    assert!(
        (left_front.rotation[0] - (0.8 * leg_anim)).abs() < 1e-6,
        "left front"
    );
    assert!(
        (right_front.rotation[0] - (-0.8 * leg_anim)).abs() < 1e-6,
        "right front"
    );

    // Only xRot changes; offset and yRot/zRot are preserved.
    for (posed, index) in [(left_hind, 2), (right_front, 5)] {
        let base = ADULT_HORSE_PARTS[index].pose;
        assert_eq!(posed.offset, base.offset);
        assert_eq!(posed.rotation[1], base.rotation[1]);
        assert_eq!(posed.rotation[2], base.rotation[2]);
    }

    // At rest (speed 0) every leg holds its body-layer pose.
    assert_eq!(
        equine_leg_swing_pose(ADULT_HORSE_PARTS[4].pose, pos, 0.0),
        ADULT_HORSE_PARTS[4].pose
    );
}
