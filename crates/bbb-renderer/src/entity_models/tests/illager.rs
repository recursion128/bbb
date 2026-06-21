use super::*;

#[test]
fn illager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ILLAGER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: ILLAGER_ROBE,
        }
    );
    assert_eq!(
        ILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.45, -10.45, -4.45],
            size: [8.9, 12.9, 8.9],
            color: ILLAGER_HAT_COLOR,
        }
    );
    assert_eq!(
        ILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: ILLAGER_ROBE,
        }
    );

    assert_eq!(ILLAGER_SHARED_CROSSED_PARTS.len(), 5);
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_CROSSED_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_BODY.as_slice(),
    );
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ILLAGER_CROSSED_ARMS.as_slice(),
        ILLAGER_CROSSED_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_CROSSED_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_SHOULDER.as_slice(),
    );

    assert_eq!(ILLAGER_SHARED_UNCROSSED_PARTS.len(), 6);
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[4],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[5],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_ARM.as_slice(),
    );

    assert_part_tree(
        &ILLAGER_ILLUSIONER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_WITH_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HAT.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
}

#[test]
fn illager_model_meshes_use_vanilla_scaled_body_layer_geometry() {
    let evoker = entity_model_mesh(&[EntityModelInstance::illager(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Evoker,
    )]);
    assert_eq!(evoker.opaque_faces, 54);
    assert_eq!(evoker.vertices.len(), 216);
    assert_eq!(evoker.indices.len(), 324);
    let (evoker_min, evoker_max) = mesh_extents(&evoker);
    assert_close3(evoker_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(evoker_max, [0.46875003, 65.993126, 0.3839772]);

    let illusioner = entity_model_mesh(&[EntityModelInstance::illager(
        68,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Illusioner,
    )]);
    assert_eq!(illusioner.opaque_faces, 60);
    assert_eq!(illusioner.vertices.len(), 240);
    assert_eq!(illusioner.indices.len(), 360);
    let (illusioner_min, illusioner_max) = mesh_extents(&illusioner);
    assert_close3(illusioner_min, [-0.46875, 64.00094, -0.26074222]);
    assert_close3(illusioner_max, [0.46875003, 66.01949, 0.3839772]);

    let pillager = entity_model_mesh(&[EntityModelInstance::illager(
        103,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Pillager,
    )]);
    assert_eq!(pillager.opaque_faces, 48);
    assert_eq!(pillager.vertices.len(), 192);
    assert_eq!(pillager.indices.len(), 288);
    let (pillager_min, pillager_max) = mesh_extents(&pillager);
    assert_close3(pillager_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(pillager_max, [0.46875, 65.993126, 0.3515625]);

    let vindicator = entity_model_mesh(&[EntityModelInstance::illager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Vindicator,
    )]);
    assert_eq!(vindicator.vertices, evoker.vertices);
    assert_eq!(vindicator.indices, evoker.indices);
}

#[test]
fn illager_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            IllagerModelFamily::Evoker,
            "evoker",
            EntityModelTextureRef {
                path: "textures/entity/illager/evoker.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Illusioner,
            "illusioner",
            EntityModelTextureRef {
                path: "textures/entity/illager/illusioner.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Pillager,
            "pillager",
            EntityModelTextureRef {
                path: "textures/entity/illager/pillager.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Vindicator,
            "vindicator",
            EntityModelTextureRef {
                path: "textures/entity/illager/vindicator.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, model_key, texture) in cases {
        let kind = EntityModelKind::Illager { family };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn half_amplitude_leg_swing_pose_applies_vanilla_half_amplitude() {
    // Vanilla IllagerModel.setupAnim (non-riding): rightLeg.xRot =
    // cos(pos * 0.6662) * 1.4 * speed * 0.5 (in phase), leftLeg.xRot =
    // cos(pos * 0.6662 + π) * 1.4 * speed * 0.5 (out of phase). The extra 0.5 factor
    // (vs HumanoidModel's 1.4 * speed) is what makes the illager-specific pose. The
    // illager body layers place the right leg at offset x = -2 and the left at x = +2.
    let right = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        0.0,
        1.0,
    );
    let left = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        0.0,
        1.0,
    );
    assert!(
        (right.rotation[0] - 0.7).abs() < 1e-6,
        "right leg in phase at half amplitude: {}",
        right.rotation[0]
    );
    assert!(
        (left.rotation[0] + 0.7).abs() < 1e-6,
        "left leg out of phase at half amplitude: {}",
        left.rotation[0]
    );

    // A general (pos, speed) reproduces cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5.
    let phase = 1.5_f32 * 0.6662;
    let right = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        1.5,
        0.5,
    );
    let left = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        1.5,
        0.5,
    );
    assert!((right.rotation[0] - phase.cos() * 1.4 * 0.5 * 0.5).abs() < 1e-6);
    assert!(
        (left.rotation[0] - (phase + std::f32::consts::PI).cos() * 1.4 * 0.5 * 0.5).abs() < 1e-6
    );
}

#[test]
fn illager_family_swings_its_legs_when_walking() {
    // IllagerModel is not a HumanoidModel but its non-riding setupAnim swings the
    // legs `cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5`. A standing illager is inert;
    // a walking one lifts its feet (a shorter model) and splays its legs along Z, for
    // every family (the crossed-arms evoker/vindicator/illusioner lists legs at
    // [3, 4], the uncrossed pillager at [2, 3]). The pillager also swings its separate arms
    // (see `pillager_swings_its_arms_when_walking`); the arm-pose overrides and the riding
    // sit pose are deferred.
    let families = [
        ("evoker", IllagerModelFamily::Evoker),
        ("vindicator", IllagerModelFamily::Vindicator),
        ("illusioner", IllagerModelFamily::Illusioner),
        ("pillager", IllagerModelFamily::Pillager),
    ];
    for (name, family) in families {
        let base = EntityModelInstance::illager(200, [0.0, 64.0, 0.0], 0.0, family);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{name}: a walking illager's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{name}: a walking illager's legs should splay along Z"
        );
    }
}

#[test]
fn pillager_swings_its_arms_when_walking() {
    // Vanilla `IllagerModel.setupAnim` swings the separate arms with the `HumanoidModel`
    // amplitude `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5` (right arm a half-cycle out of
    // phase). The pillager renders the uncrossed layout head/body/leg/leg/right_arm/left_arm
    // (192 verts, 8 cubes), so the two arm cubes occupy vertices [144, 192). A walking
    // pillager swings them; a standing one keeps them at rest.
    let base =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(rest.vertices.len(), 192);
    assert_ne!(
        rest.vertices[144..192],
        walking.vertices[144..192],
        "the pillager swings its separate arms when walking"
    );
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(
        rest.vertices[144..192],
        still.vertices[144..192],
        "a standing pillager's arms are inert"
    );
}

#[test]
fn crossed_arm_illagers_keep_their_arms_still_when_walking() {
    // The evoker/vindicator/illusioner show the static crossed `arms` part: vanilla swings
    // the *invisible* separate arms, so the visible crossed part holds still. The evoker
    // layout is head/body/crossed_arm/leg/leg (216 verts, 9 cubes): the crossed arm part
    // (3 cubes) occupies vertices [96, 168) and the two legs [168, 216). A walking evoker
    // swings only its legs.
    let base = EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(rest.vertices.len(), 216);
    assert_eq!(
        rest.vertices[96..168],
        walking.vertices[96..168],
        "the crossed arms part stays still when walking"
    );
    assert_ne!(
        rest.vertices[168..216],
        walking.vertices[168..216],
        "the legs still swing when walking"
    );
}
