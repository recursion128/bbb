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
