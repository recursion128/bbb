use super::*;

#[test]
fn piglin_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_PIGLIN_HEAD,
        [
            ModelCubeDesc {
                min: [-5.0, -8.0, -4.0],
                size: [10.0, 8.0, 8.0],
                color: PIGLIN_SKIN,
            },
            ModelCubeDesc {
                min: [-2.0, -4.0, -5.0],
                size: [4.0, 4.0, 1.0],
                color: PIGLIN_SKIN,
            },
            ModelCubeDesc {
                min: [2.0, -2.0, -5.0],
                size: [1.0, 2.0, 1.0],
                color: PIGLIN_SKIN,
            },
            ModelCubeDesc {
                min: [-3.0, -2.0, -5.0],
                size: [1.0, 2.0, 1.0],
                color: PIGLIN_SKIN,
            },
        ]
    );
    assert_eq!(ADULT_PIGLIN_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_PIGLIN_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_HEAD.as_slice(),
        ADULT_PIGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_HEAD_CHILDREN[0],
        [4.5, -6.0, 0.0],
        [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
        ADULT_PIGLIN_LEFT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_HEAD_CHILDREN[1],
        [-4.5, -6.0, 0.0],
        [0.0, 0.0, std::f32::consts::FRAC_PI_6],
        ADULT_PIGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_BODY.as_slice(),
    );
    assert_part_tree(
        &ADULT_PIGLIN_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_RIGHT_ARM.as_slice(),
        ADULT_PIGLIN_RIGHT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_RIGHT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_RIGHT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &ADULT_PIGLIN_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_LEFT_ARM.as_slice(),
        ADULT_PIGLIN_LEFT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_LEFT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_LEFT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &ADULT_PIGLIN_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_LEG.as_slice(),
        ADULT_PIGLIN_LEG_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_PIGLIN_LEG_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_PANTS.as_slice(),
    );
    assert_part_tree(
        &ADULT_PIGLIN_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_PIGLIN_LEG.as_slice(),
        ADULT_PIGLIN_LEG_CHILDREN.as_slice(),
    );

    assert_eq!(BABY_PIGLIN_PARTS.len(), 6);
    assert_part(
        &BABY_PIGLIN_PARTS[0],
        [0.0, 18.0, -0.5],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_BODY.as_slice(),
    );
    assert_part_tree(
        &BABY_PIGLIN_PARTS[1],
        [0.0, 15.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_HEAD.as_slice(),
        BABY_PIGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
    );
    assert_part_tree(
        &BABY_PIGLIN_HEAD_CHILDREN[1],
        [4.2, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN[0],
        [1.0, 1.75, 0.0],
        [0.0, 0.0, -0.6109],
        BABY_PIGLIN_LEFT_EAR.as_slice(),
    );
    assert_part_tree(
        &BABY_PIGLIN_HEAD_CHILDREN[2],
        [-4.2, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN[0],
        [-1.0, 1.75, 0.0],
        [0.0, 0.0, 0.6109],
        BABY_PIGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_PARTS[2],
        [4.0, 15.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_LEFT_ARM.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_PARTS[3],
        [-4.0, 15.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_PARTS[4],
        [-1.5, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_LEG.as_slice(),
    );
    assert_part(
        &BABY_PIGLIN_PARTS[5],
        [1.5, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_PIGLIN_LEG.as_slice(),
    );
}

#[test]
fn piglin_meshes_use_vanilla_body_layer_geometry() {
    let piglin = entity_model_mesh(&[EntityModelInstance::piglin(
        101,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        false,
    )]);
    assert_eq!(piglin.opaque_faces, 90);
    assert_eq!(piglin.vertices.len(), 360);
    assert_eq!(piglin.indices.len(), 540);
    assert!(piglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
    let (piglin_min, piglin_max) = mesh_extents(&piglin);
    assert_close3(piglin_min, [-0.515625, 63.985374, -0.25000003]);
    assert_close3(piglin_max, [0.515625, 66.001, 0.31250003]);

    let baby_piglin = entity_model_mesh(&[EntityModelInstance::piglin(
        101,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        true,
    )]);
    assert_eq!(baby_piglin.opaque_faces, 54);
    assert_eq!(baby_piglin.vertices.len(), 216);
    assert_eq!(baby_piglin.indices.len(), 324);
    assert!(baby_piglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
    let (baby_piglin_min, baby_piglin_max) = mesh_extents(&baby_piglin);
    assert_close3(baby_piglin_min, [-0.45814878, 64.001, -0.21875003]);
    assert_close3(baby_piglin_max, [0.45814878, 64.9385, 0.28125]);

    let brute = entity_model_mesh(&[EntityModelInstance::piglin(
        102,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::PiglinBrute,
        false,
    )]);
    assert_same_geometry(&brute, &piglin);
    assert!(brute
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_BRUTE_SKIN, 0.78)));

    let zombified = entity_model_mesh(&[EntityModelInstance::piglin(
        154,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        false,
    )]);
    assert_same_geometry(&zombified, &piglin);
    assert!(zombified
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));

    let baby_zombified = entity_model_mesh(&[EntityModelInstance::piglin(
        154,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        true,
    )]);
    assert_same_geometry(&baby_zombified, &baby_piglin);
    assert!(baby_zombified
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));
}

#[test]
fn piglin_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin_brute.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/zombified_piglin.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/zombified_piglin_baby.png",
            size: [64, 64],
        })
    );
}

#[test]
fn piglin_family_swings_its_arms_when_walking() {
    // `AbstractPiglinModel extends HumanoidModel`, so `super.setupAnim` gives the default
    // arms the inherited counter-swing `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`
    // (the arms are overridden only by `PiglinModel`'s deferred dance/attack/crossbow/
    // admire poses). The zombified piglin instead overwrites the arms via
    // `AnimationUtils.animateZombieArms` (the held-out zombie pose, deferred), so its arms
    // must stay at rest. In the adult layer (15 cubes) the head/snout/ears and body fill
    // 24-vertex blocks [0, 7); the two arms (each a cube plus its sleeve child) fill blocks
    // [7, 11) = vertices [168, 264); the legs fill [11, 15). The baby layer's arms (no
    // sleeve children) fill blocks [5, 7) = vertices [120, 168).
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let arm_slice = |baby: bool| -> std::ops::Range<usize> {
        if baby {
            120..168
        } else {
            168..264
        }
    };
    // Families whose default arms swing: adult and baby piglin, and the brute (which
    // reuses the adult piglin body layer).
    let swings: [(&str, PiglinModelFamily, bool); 3] = [
        ("piglin", PiglinModelFamily::Piglin, false),
        ("piglin_baby", PiglinModelFamily::Piglin, true),
        ("piglin_brute", PiglinModelFamily::PiglinBrute, false),
    ];
    for (name, family, baby) in swings {
        let base = EntityModelInstance::piglin(95, [0.0, 64.0, 0.0], 0.0, family, baby);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let arms = arm_slice(baby);
        assert_ne!(
            rest.vertices[arms.clone()],
            walking.vertices[arms.clone()],
            "{name}: arms swing when walking"
        );
        let rest_arm_z = z_extent(&rest.vertices[arms.clone()]);
        let walk_arm_z = z_extent(&walking.vertices[arms.clone()]);
        assert!(
            walk_arm_z > rest_arm_z + 0.1,
            "{name}: a forward/back arm swing deepens the arm Z footprint: {rest_arm_z} -> {walk_arm_z}"
        );
    }
    // The zombified piglin overwrites its arms with the deferred zombie pose, so the arm
    // region is byte-identical between standing and walking — only its legs swing.
    for (name, baby) in [("zombified_piglin", false), ("zombified_piglin_baby", true)] {
        let base = EntityModelInstance::piglin(
            96,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::ZombifiedPiglin,
            baby,
        );
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let arms = arm_slice(baby);
        assert_eq!(
            rest.vertices[arms.clone()],
            walking.vertices[arms.clone()],
            "{name}: the deferred zombie arm pose keeps the arms at rest"
        );
        assert_ne!(
            rest.vertices, walking.vertices,
            "{name}: the legs still swing"
        );
    }
}

#[test]
fn piglin_family_swings_its_legs_when_walking() {
    // `AbstractPiglinModel extends HumanoidModel`: its `setupAnim` runs
    // `super.setupAnim` (the inherited leg swing) then sways only the ears, so the
    // piglin family inherits the `HumanoidModel` legs unchanged (the default arm swing
    // is covered by `piglin_family_swings_its_arms_when_walking`). A standing piglin is
    // inert; a walking one lifts its feet (a shorter model) and splays along Z, for
    // every family and the baby layout. The ear sway and override arm poses are deferred.
    let instances: [(&str, EntityModelInstance); 5] = [
        (
            "piglin",
            EntityModelInstance::piglin(
                90,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::Piglin,
                false,
            ),
        ),
        (
            "piglin_baby",
            EntityModelInstance::piglin(91, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, true),
        ),
        (
            "piglin_brute",
            EntityModelInstance::piglin(
                92,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::PiglinBrute,
                false,
            ),
        ),
        (
            "zombified_piglin",
            EntityModelInstance::piglin(
                93,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                false,
            ),
        ),
        (
            "zombified_piglin_baby",
            EntityModelInstance::piglin(
                94,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                true,
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
            "{name}: a walking piglin's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{name}: a walking piglin's legs should splay along Z"
        );
    }
}
