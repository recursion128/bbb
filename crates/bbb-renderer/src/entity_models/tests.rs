use super::*;

mod armor_stand;
mod chicken;
mod enderman;
mod sheep;
mod skeleton;
mod spider;
mod zombie;

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
fn hoglin_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ADULT_HOGLIN_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_HOGLIN_PARTS[0],
        [0.0, 7.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_BODY.as_slice(),
        ADULT_HOGLIN_BODY_CHILDREN.as_slice(),
    );
    assert_eq!(
        ADULT_HOGLIN_MANE[0],
        ModelCubeDesc {
            min: [-0.001, -0.001, -9.001],
            size: [0.002, 10.002, 19.002],
            color: HOGLIN_RED,
        }
    );
    assert_part(
        &ADULT_HOGLIN_BODY_CHILDREN[0],
        [0.0, -14.0, -7.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_MANE.as_slice(),
    );
    assert_part_tree(
        &ADULT_HOGLIN_PARTS[1],
        [0.0, 2.0, -12.0],
        [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        ADULT_HOGLIN_HEAD.as_slice(),
        ADULT_HOGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[0],
        [-6.0, -2.0, -3.0],
        [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
        ADULT_HOGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[1],
        [6.0, -2.0, -3.0],
        [0.0, 0.0, HOGLIN_EAR_Z_ROT],
        ADULT_HOGLIN_LEFT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[2],
        [-7.0, 2.0, -12.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_HORN.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[3],
        [7.0, 2.0, -12.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_HORN.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_HOGLIN_PARTS[2],
            [-4.0, 10.0, -8.5],
            ADULT_HOGLIN_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[3],
            [4.0, 10.0, -8.5],
            ADULT_HOGLIN_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[4],
            [-5.0, 13.0, 10.0],
            ADULT_HOGLIN_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[5],
            [5.0, 13.0, 10.0],
            ADULT_HOGLIN_HIND_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_HOGLIN_PARTS.len(), 6);
    assert_part_tree(
        &BABY_HOGLIN_PARTS[0],
        [0.0, 13.0, -7.0],
        [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        BABY_HOGLIN_HEAD.as_slice(),
        BABY_HOGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_HEAD_CHILDREN[0],
        [-5.0, -1.0, -1.5],
        [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
        BABY_HOGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_HEAD_CHILDREN[1],
        [5.0, -1.0, -1.5],
        [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
        BABY_HOGLIN_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_PARTS[1],
        [0.0, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_HOGLIN_BODY.as_slice(),
    );
    assert_eq!(
        BABY_HOGLIN_BODY[0],
        ModelCubeDesc {
            min: [-4.02, -14.02, -7.02],
            size: [8.04, 8.04, 14.04],
            color: HOGLIN_RED,
        }
    );
    assert_eq!(
        BABY_HOGLIN_BODY[1],
        ModelCubeDesc {
            min: [-0.02, -18.02, -8.02],
            size: [0.04, 6.04, 11.04],
            color: HOGLIN_RED,
        }
    );
    for (part, expected_offset) in [
        (&BABY_HOGLIN_PARTS[2], [-2.5, 18.0, 4.5]),
        (&BABY_HOGLIN_PARTS[3], [2.5, 18.0, 4.5]),
        (&BABY_HOGLIN_PARTS[4], [-2.5, 18.0, -4.5]),
        (&BABY_HOGLIN_PARTS[5], [2.5, 18.0, -4.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_HOGLIN_LEG.as_slice(),
        );
    }
}

#[test]
fn hoglin_meshes_use_vanilla_body_layers_for_hoglins_and_zoglins() {
    let adult_hoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        220,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        false,
    )]);
    assert_eq!(adult_hoglin.opaque_faces, 66);
    assert_eq!(adult_hoglin.vertices.len(), 264);
    assert_eq!(adult_hoglin.indices.len(), 396);
    assert!(adult_hoglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HOGLIN_RED, 0.78)));

    let adult_zoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        221,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Zoglin,
        false,
    )]);
    assert_same_geometry(&adult_zoglin, &adult_hoglin);
    assert!(adult_zoglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOGLIN_GREEN, 0.78)));

    let baby_hoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        222,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        true,
    )]);
    assert_eq!(baby_hoglin.opaque_faces, 66);
    assert_eq!(baby_hoglin.vertices.len(), 264);
    assert_eq!(baby_hoglin.indices.len(), 396);

    let baby_zoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        223,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Zoglin,
        true,
    )]);
    assert_same_geometry(&baby_zoglin, &baby_hoglin);

    let (adult_min, adult_max) = mesh_extents(&adult_hoglin);
    let (baby_min, baby_max) = mesh_extents(&baby_hoglin);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn hoglin_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            HoglinModelFamily::Hoglin,
            false,
            "hoglin",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin.png",
                size: [128, 64],
            },
        ),
        (
            HoglinModelFamily::Hoglin,
            true,
            "hoglin_baby",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin_baby.png",
                size: [64, 64],
            },
        ),
        (
            HoglinModelFamily::Zoglin,
            false,
            "zoglin",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin.png",
                size: [128, 64],
            },
        ),
        (
            HoglinModelFamily::Zoglin,
            true,
            "zoglin_baby",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Hoglin { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn ravager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(RAVAGER_PARTS.len(), 6);
    assert_part_tree(
        &RAVAGER_PARTS[0],
        [0.0, -7.0, 5.5],
        [0.0, 0.0, 0.0],
        RAVAGER_NECK.as_slice(),
        RAVAGER_NECK_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &RAVAGER_NECK_CHILDREN[0],
        [0.0, 16.0, -17.0],
        [0.0, 0.0, 0.0],
        RAVAGER_HEAD.as_slice(),
        RAVAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[0],
        [-10.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[1],
        [8.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, 2.0],
        [0.0, 0.0, 0.0],
        RAVAGER_MOUTH.as_slice(),
    );
    assert_part(
        &RAVAGER_PARTS[1],
        [0.0, 1.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        RAVAGER_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &RAVAGER_PARTS[2],
            [-8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[3],
            [8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[4],
            [-8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[5],
            [8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
}

#[test]
fn ravager_mesh_uses_vanilla_body_layer_geometry() {
    let ravager = entity_model_mesh(&[EntityModelInstance::ravager(224, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(ravager.opaque_faces, 72);
    assert_eq!(ravager.vertices.len(), 288);
    assert_eq!(ravager.indices.len(), 432);
    assert!(ravager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(RAVAGER_GRAY, 0.78)));

    let (min, max) = mesh_extents(&ravager);
    assert!(max[1] - min[1] > 2.0);
    assert!(max[2] - min[2] > 2.0);
}

#[test]
fn ravager_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Ravager;
    assert_eq!(kind.model_key(), "ravager");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/illager/ravager.png",
            size: [128, 128],
        })
    );
}

#[test]
fn player_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(PLAYER_WIDE_PARTS.len(), 6);
    assert_part_tree(
        &PLAYER_WIDE_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_HEAD.as_slice(),
        PLAYER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_HAT.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_BODY.as_slice(),
        PLAYER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_JACKET.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_RIGHT_ARM.as_slice(),
        PLAYER_WIDE_RIGHT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_WIDE_RIGHT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_RIGHT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_LEFT_ARM.as_slice(),
        PLAYER_WIDE_LEFT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_WIDE_LEFT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_LEFT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_LEG.as_slice(),
        PLAYER_RIGHT_PANTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_LEG.as_slice(),
        PLAYER_LEFT_PANTS_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_RIGHT_PANTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_PANTS.as_slice(),
    );
    assert_part(
        &PLAYER_LEFT_PANTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_PANTS.as_slice(),
    );

    assert_eq!(PLAYER_SLIM_PARTS.len(), 6);
    assert_part_tree(
        &PLAYER_SLIM_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_RIGHT_ARM.as_slice(),
        PLAYER_SLIM_RIGHT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_SLIM_RIGHT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_RIGHT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_SLIM_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_LEFT_ARM.as_slice(),
        PLAYER_SLIM_LEFT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_SLIM_LEFT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_LEFT_SLEEVE.as_slice(),
    );
}

#[test]
fn player_mesh_uses_vanilla_body_layer_geometry_and_avatar_scale() {
    let wide = entity_model_mesh(&[EntityModelInstance::player(
        155,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let slim = entity_model_mesh(&[EntityModelInstance::player(
        156,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.opaque_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PLAYER_BLUE, 0.78)));
    }

    let (wide_min, wide_max) = mesh_extents(&wide);
    let (slim_min, slim_max) = mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_texture_refs_match_vanilla_default_assets() {
    let cases = [
        (
            false,
            "player",
            EntityModelTextureRef {
                path: "textures/entity/player/wide/steve.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "player_slim",
            EntityModelTextureRef {
                path: "textures/entity/player/slim/steve.png",
                size: [64, 64],
            },
        ),
    ];

    for (slim, model_key, texture) in cases {
        let kind = EntityModelKind::Player { slim };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn player_textured_layer_passes_match_vanilla_avatar_renderer_model_layers() {
    let wide = player_textured_layer_passes(false);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(wide[0].model_layer, MODEL_LAYER_PLAYER);
    assert_eq!(wide[0].texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(wide[0].parts, PLAYER_WIDE_TEXTURED_PARTS.as_slice());
    assert_eq!(wide[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wide[0].collector_order, wide[0].submit_sequence), (0, 0));

    let slim = player_textured_layer_passes(true);
    assert_eq!(slim.len(), 1);
    assert_eq!(slim[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(slim[0].model_layer, MODEL_LAYER_PLAYER_SLIM);
    assert_eq!(slim[0].texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(slim[0].parts, PLAYER_SLIM_TEXTURED_PARTS.as_slice());
    assert_eq!(slim[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slim[0].collector_order, slim[0].submit_sequence), (0, 0));
}

#[test]
fn player_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PLAYER, "minecraft:player#main");
    assert_eq!(MODEL_LAYER_PLAYER_SLIM, "minecraft:player_slim#main");
    assert_eq!(PLAYER_WIDE_TEXTURED_PARTS.len(), 6);
    assert_eq!(PLAYER_SLIM_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        PLAYER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-4.5, -8.5, -4.5],
            size: [9.0, 9.0, 9.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [32.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_JACKET[0],
        TexturedModelCubeDesc {
            min: [-4.25, -0.25, -2.25],
            size: [8.5, 12.5, 4.5],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(PLAYER_WIDE_TEXTURED_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_LEFT_ARM[0].tex, [32.0, 48.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE[0].tex, [40.0, 32.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_LEFT_SLEEVE[0].tex, [48.0, 48.0]);
    assert_eq!(PLAYER_SLIM_TEXTURED_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_TEXTURED_LEFT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(
        PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE[0].uv_size,
        [3.0, 12.0, 4.0]
    );
    assert_eq!(
        PLAYER_SLIM_TEXTURED_LEFT_SLEEVE[0].uv_size,
        [3.0, 12.0, 4.0]
    );
    assert_eq!(PLAYER_TEXTURED_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert_eq!(PLAYER_TEXTURED_LEFT_LEG[0].tex, [16.0, 48.0]);
    assert_eq!(PLAYER_TEXTURED_RIGHT_PANTS[0].tex, [0.0, 32.0]);
    assert_eq!(PLAYER_TEXTURED_LEFT_PANTS[0].tex, [0.0, 48.0]);
    assert_eq!(
        PLAYER_WIDE_TEXTURED_PARTS[0].pose,
        PLAYER_WIDE_PARTS[0].pose
    );
    assert_eq!(
        PLAYER_SLIM_TEXTURED_PARTS[2].pose,
        PLAYER_SLIM_PARTS[2].pose
    );
}

#[test]
fn entity_texture_atlas_stitches_official_player_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/player/wide/steve.png",
            "textures/entity/player/slim/steve.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    let slim_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(&rgba[0..4], &[0; 4]);
    assert_eq!(&rgba[slim_first_pixel..slim_first_pixel + 4], &[1; 4]);
}

#[test]
fn player_textured_mesh_uses_vanilla_uvs_tints_and_avatar_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let wide = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            901,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    let slim = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            902,
            [0.0, 64.0, 0.0],
            0.0,
            true,
        )],
        &atlas,
    );

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.cutout_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
    assert_close2(wide.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(slim.vertices[0].uv, [16.0 / 64.0, 0.5]);

    let (wide_min, wide_max) = textured_mesh_extents(&wide);
    let (slim_min, slim_max) = textured_mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
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
fn zombie_and_piglin_texture_refs_match_vanilla_renderers() {
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
fn pig_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_PIG_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -8.0],
                size: [8.0, 8.0, 8.0],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-2.0, 0.0, -9.0],
                size: [4.0, 3.0, 1.0],
                color: PIG_PINK,
            },
        ]
    );
    assert_eq!(
        ADULT_PIG_BODY[0],
        ModelCubeDesc {
            min: [-5.0, -10.0, -7.0],
            size: [10.0, 16.0, 8.0],
            color: PIG_PINK,
        }
    );
    assert_eq!(
        ADULT_PIG_LEG[0],
        ModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            color: PIG_PINK,
        }
    );

    assert_eq!(ADULT_PIG_PARTS.len(), 6);
    assert_part(
        &ADULT_PIG_PARTS[0],
        [0.0, 12.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_PIG_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_PIG_PARTS[1],
        [0.0, 11.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_PIG_BODY.as_slice(),
    );

    for (part, expected_offset) in ADULT_PIG_PARTS[2..].iter().zip([
        [-3.0, 18.0, 7.0],
        [3.0, 18.0, 7.0],
        [-3.0, 18.0, -5.0],
        [3.0, 18.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_PIG_LEG.as_slice(),
        );
    }
}

#[test]
fn pig_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        COLD_PIG_BODY,
        [
            ModelCubeDesc {
                min: [-5.0, -10.0, -7.0],
                size: [10.0, 16.0, 8.0],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-5.5, -10.5, -7.5],
                size: [11.0, 17.0, 9.0],
                color: PIG_COLD_FUR,
            },
        ]
    );

    assert_eq!(COLD_PIG_PARTS.len(), 6);
    assert_part(
        &COLD_PIG_PARTS[0],
        [0.0, 12.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_PIG_HEAD.as_slice(),
    );
    assert_part(
        &COLD_PIG_PARTS[1],
        [0.0, 11.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        COLD_PIG_BODY.as_slice(),
    );

    for (part, expected_offset) in COLD_PIG_PARTS[2..].iter().zip([
        [-3.0, 18.0, 7.0],
        [3.0, 18.0, 7.0],
        [-3.0, 18.0, -5.0],
        [3.0, 18.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_PIG_LEG.as_slice(),
        );
    }

    assert_eq!(
        pig_model_parts(PigModelVariant::Temperate, false),
        ADULT_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Warm, false),
        ADULT_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Cold, false),
        COLD_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Cold, true),
        BABY_PIG_PARTS.as_slice()
    );
}

#[test]
fn pig_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.3125, 64.001, -0.5625]);
    assert_close3(max, [0.3125, 65.001, 0.9375]);
}

#[test]
fn pig_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        92,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.34375, 64.001, -0.5625]);
    assert_close3(max, [0.34375, 65.001, 0.9375]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIG_COLD_FUR, 0.78)));
}

#[test]
fn pig_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_PIG_BODY[0],
        ModelCubeDesc {
            min: [-3.5, -3.0, -4.5],
            size: [7.0, 6.0, 9.0],
            color: PIG_PINK,
        }
    );
    assert_eq!(
        BABY_PIG_HEAD,
        [
            ModelCubeDesc {
                min: [-3.525, -5.025, -5.025],
                size: [7.05, 6.05, 6.05],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-1.515, -1.99, -6.015],
                size: [3.03, 2.03, 1.03],
                color: PIG_PINK,
            },
        ]
    );
    assert_eq!(
        BABY_PIG_LEG[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 2.0, 2.0],
            color: PIG_PINK,
        }
    );

    assert_eq!(BABY_PIG_PARTS.len(), 6);
    assert_part(
        &BABY_PIG_PARTS[0],
        [0.0, 19.0, 0.5],
        [0.0, 0.0, 0.0],
        BABY_PIG_BODY.as_slice(),
    );
    assert_part(
        &BABY_PIG_PARTS[1],
        [0.0, 19.0, -2.0],
        [0.0, 0.0, 0.0],
        BABY_PIG_HEAD.as_slice(),
    );

    for (part, expected_offset) in BABY_PIG_PARTS[2..].iter().zip([
        [2.5, 22.0, -3.0],
        [-2.5, 22.0, -3.0],
        [2.5, 22.0, 4.0],
        [-2.5, 22.0, 4.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_PIG_LEG.as_slice(),
        );
    }
}

#[test]
fn pig_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        91,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Warm,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.2203125, 64.001, -0.3125]);
    assert_close3(max, [0.2203125, 64.62756, 0.5009375]);
}

#[test]
fn pig_texture_refs_match_vanilla_variant_assets() {
    let cases = [
        (
            PigModelVariant::Temperate,
            false,
            "pig_temperate",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Temperate,
            true,
            "pig_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Warm,
            false,
            "pig_warm",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Warm,
            true,
            "pig_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Cold,
            false,
            "pig_cold",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Cold,
            true,
            "pig_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold_baby.png",
                size: [32, 32],
            },
        ),
    ];

    for (variant, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Pig { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn pig_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = pig_textured_layer_passes(PigModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::PigBase);
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_PIG);
    assert_eq!(temperate[0].texture, PIG_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].parts, ADULT_PIG_TEXTURED_PARTS.as_slice());
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (temperate[0].collector_order, temperate[0].submit_sequence),
        (0, 0)
    );

    let warm_baby = pig_textured_layer_passes(PigModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_PIG_BABY);
    assert_eq!(warm_baby[0].texture, PIG_WARM_BABY_TEXTURE_REF);
    assert_eq!(warm_baby[0].parts, BABY_PIG_TEXTURED_PARTS.as_slice());

    let cold_adult = pig_textured_layer_passes(PigModelVariant::Cold, false);
    assert_eq!(cold_adult[0].model_layer, MODEL_LAYER_COLD_PIG);
    assert_eq!(cold_adult[0].texture, PIG_COLD_TEXTURE_REF);
    assert_eq!(cold_adult[0].parts, COLD_PIG_TEXTURED_PARTS.as_slice());
}

#[test]
fn pig_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PIG, "minecraft:pig#main");
    assert_eq!(MODEL_LAYER_PIG_BABY, "minecraft:pig_baby#main");
    assert_eq!(MODEL_LAYER_COLD_PIG, "minecraft:cold_pig#main");
    assert_eq!(ADULT_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(COLD_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(BABY_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        ADULT_PIG_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -4.0, -8.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_PIG_TEXTURED_HEAD[1].tex, [16.0, 16.0]);
    assert_eq!(ADULT_PIG_TEXTURED_BODY[0].tex, [28.0, 8.0]);
    assert_eq!(
        COLD_PIG_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [-5.5, -10.5, -7.5],
            size: [11.0, 17.0, 9.0],
            uv_size: [10.0, 16.0, 8.0],
            tex: [28.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_PIG_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.525, -5.025, -5.025],
            size: [7.05, 6.05, 6.05],
            uv_size: [7.0, 6.0, 6.0],
            tex: [0.0, 15.0],
            mirror: false,
        }
    );
    assert_eq!(BABY_PIG_TEXTURED_HEAD[1].tex, [6.0, 27.0]);
    assert_eq!(BABY_PIG_TEXTURED_RIGHT_FRONT_LEG[0].tex, [23.0, 0.0]);
    assert_eq!(BABY_PIG_TEXTURED_RIGHT_HIND_LEG[0].tex, [23.0, 4.0]);
    assert_eq!(ADULT_PIG_TEXTURED_PARTS[0].pose, ADULT_PIG_PARTS[0].pose);
    assert_eq!(COLD_PIG_TEXTURED_PARTS[1].pose, COLD_PIG_PARTS[1].pose);
    assert_eq!(BABY_PIG_TEXTURED_PARTS[1].pose, BABY_PIG_PARTS[1].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_pig_png_slots() {
    let images = pig_texture_images();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 288);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/pig/pig_temperate.png",
            "textures/entity/pig/pig_temperate_baby.png",
            "textures/entity/pig/pig_warm.png",
            "textures/entity/pig/pig_warm_baby.png",
            "textures/entity/pig/pig_cold.png",
            "textures/entity/pig/pig_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 96.0 / 288.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 192.0 / 288.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 256.0 / 288.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 160, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn pig_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::pig(
                501,
                [0.0, 64.0, 0.0],
                0.0,
                PigModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::pig(502, [1.0, 64.0, 0.0], 0.0, PigModelVariant::Cold, false),
            EntityModelInstance::pig(503, [2.0, 64.0, 0.0], 0.0, PigModelVariant::Warm, true),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[168].uv, [16.0 / 64.0, 192.0 / 288.0]);
    assert_eq!(mesh.vertices[168].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[360].uv, [16.0 / 64.0, 160.0 / 288.0]);
    assert_eq!(mesh.vertices[360].tint, [1.0, 1.0, 1.0, 1.0]);
    let (min, max) = textured_mesh_extents(&mesh);
    assert!(max[0] - min[0] > 2.0);
    assert_close3([min[1], max[1], max[2] - min[2]], [64.001, 65.001, 1.5]);
}

#[test]
fn entity_texture_atlas_stitches_official_cow_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 384);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/cow/cow_temperate.png",
            "textures/entity/cow/cow_temperate_baby.png",
            "textures/entity/cow/cow_warm.png",
            "textures/entity/cow/cow_warm_baby.png",
            "textures/entity/cow/cow_cold.png",
            "textures/entity/cow/cow_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 384.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 192.0 / 384.0]);
    assert_close2(layout.entries[3].uv.max, [1.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 320.0 / 384.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 256, 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn cow_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::cow_variant(
                601,
                [0.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::cow_variant(
                602,
                [1.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Cold,
                false,
            ),
            EntityModelInstance::cow_variant(
                603,
                [2.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Warm,
                true,
            ),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 180);
    assert_eq!(mesh.vertices.len(), 720);
    assert_eq!(mesh.indices.len(), 1080);
    assert_close2(mesh.vertices[0].uv, [14.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[240].uv, [14.0 / 64.0, 256.0 / 384.0]);
    assert_eq!(mesh.vertices[240].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[504].uv, [11.0 / 64.0, 210.0 / 384.0]);
    assert_eq!(mesh.vertices[504].tint, [1.0, 1.0, 1.0, 1.0]);
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.65625]);
    assert_close3(max, [2.25, 65.5635, 1.0]);
}

#[test]
fn cow_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -6.0],
                size: [8.0, 8.0, 6.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-3.0, 1.0, -7.0],
                size: [6.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-5.0, -5.0, -5.0],
                size: [1.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [4.0, -5.0, -5.0],
                size: [1.0, 3.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(ADULT_COW_PARTS.len(), 6);
    assert_part(
        &ADULT_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        ADULT_COW_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_COW_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_COW_PARTS[2..].iter().zip([
        [-4.0, 12.0, 7.0],
        [4.0, 12.0, 7.0],
        [-4.0, 12.0, -5.0],
        [4.0, 12.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_COW_LEG.as_slice(),
        );
    }
}

#[test]
fn cow_warm_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        WARM_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -6.0],
                size: [8.0, 8.0, 6.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-3.0, 1.0, -7.0],
                size: [6.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-8.0, -3.0, -5.0],
                size: [4.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-8.0, -5.0, -5.0],
                size: [2.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [4.0, -3.0, -5.0],
                size: [4.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [6.0, -5.0, -5.0],
                size: [2.0, 2.0, 2.0],
                color: COW_BROWN,
            },
        ]
    );

    assert_eq!(WARM_COW_PARTS.len(), 6);
    assert_part(
        &WARM_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        WARM_COW_HEAD.as_slice(),
    );
    assert_part(
        &WARM_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_COW_BODY.as_slice(),
    );
}

#[test]
fn cow_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        COLD_COW_BODY,
        [
            ModelCubeDesc {
                min: [-6.5, -10.5, -7.5],
                size: [13.0, 19.0, 11.0],
                color: COW_COLD_FUR,
            },
            ModelCubeDesc {
                min: [-6.0, -10.0, -7.0],
                size: [12.0, 18.0, 10.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-2.0, 2.0, -8.0],
                size: [4.0, 6.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(
        COLD_COW_HEAD_CHILDREN,
        [
            ModelPartDesc {
                pose: PartPose {
                    offset: [-4.5, -2.5, -3.5],
                    rotation: [1.5708, 0.0, 0.0],
                },
                cubes: &COLD_COW_RIGHT_HORN,
                children: &[],
            },
            ModelPartDesc {
                pose: PartPose {
                    offset: [5.5, -2.5, -5.0],
                    rotation: [1.5708, 0.0, 0.0],
                },
                cubes: &COLD_COW_LEFT_HORN,
                children: &[],
            },
        ]
    );

    assert_eq!(COLD_COW_PARTS.len(), 6);
    assert_part_tree(
        &COLD_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        COLD_COW_HEAD.as_slice(),
        COLD_COW_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &COLD_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        COLD_COW_BODY.as_slice(),
    );

    assert_eq!(
        cow_model_parts(CowModelVariant::Temperate, false),
        ADULT_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Warm, false),
        WARM_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Cold, false),
        COLD_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Cold, true),
        BABY_COW_PARTS.as_slice()
    );
}

#[test]
fn cow_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(92, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.625]);
    assert_close3(max, [0.375, 65.5635, 0.9375]);
}

#[test]
fn cow_warm_adult_model_mesh_uses_vanilla_warm_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        94,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Warm,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5, 64.001, -0.625]);
    assert_close3(max, [0.5, 65.5635, 0.9375]);
}

#[test]
fn cow_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        95,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.40625, 64.001, -0.65625]);
    assert_close3(max, [0.40625, 65.501, 1.0]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COW_COLD_FUR, 0.78)));
}

#[test]
fn cow_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-3.0, -4.569, -4.8333],
                size: [6.0, 6.0, 5.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [3.0, -5.569, -3.8333],
                size: [1.0, 2.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-4.0, -5.569, -3.8333],
                size: [1.0, 2.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-2.0, -1.569, -5.8333],
                size: [4.0, 3.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(BABY_COW_PARTS.len(), 6);
    assert_part(
        &BABY_COW_PARTS[0],
        [0.0, 13.569, -5.1667],
        [0.0, 0.0, 0.0],
        BABY_COW_HEAD.as_slice(),
    );
    assert_part(
        &BABY_COW_PARTS[1],
        [3.0, 19.0, -5.0],
        [0.0, 0.0, 0.0],
        BABY_COW_BODY.as_slice(),
    );
    for (part, expected_offset) in BABY_COW_PARTS[2..].iter().zip([
        [-2.5, 18.0, -3.5],
        [2.5, 18.0, -3.5],
        [-2.5, 18.0, 3.5],
        [2.5, 18.0, 3.5],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_COW_LEG.as_slice(),
        );
    }
}

#[test]
fn cow_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(93, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 54);
    assert_eq!(mesh.vertices.len(), 216);
    assert_eq!(mesh.indices.len(), 324);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.001, 0.6875]);
}

#[test]
fn cow_texture_refs_match_vanilla_renderers() {
    let cow_cases = [
        (
            CowModelVariant::Temperate,
            false,
            "cow_temperate",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Temperate,
            true,
            "cow_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            false,
            "cow_warm",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            true,
            "cow_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            false,
            "cow_cold",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            true,
            "cow_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold_baby.png",
                size: [64, 64],
            },
        ),
    ];
    for (variant, baby, model_key, texture) in cow_cases {
        let kind = EntityModelKind::Cow { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn cow_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = cow_textured_layer_passes(CowModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::CowBase);
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_COW);
    assert_eq!(temperate[0].texture, COW_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].parts, ADULT_COW_TEXTURED_PARTS.as_slice());
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (temperate[0].collector_order, temperate[0].submit_sequence),
        (0, 0)
    );

    let temperate_baby = cow_textured_layer_passes(CowModelVariant::Temperate, true);
    assert_eq!(temperate_baby[0].model_layer, MODEL_LAYER_COW_BABY);
    assert_eq!(temperate_baby[0].texture, COW_TEMPERATE_BABY_TEXTURE_REF);
    assert_eq!(temperate_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());

    let warm = cow_textured_layer_passes(CowModelVariant::Warm, false);
    assert_eq!(warm[0].model_layer, MODEL_LAYER_WARM_COW);
    assert_eq!(warm[0].texture, COW_WARM_TEXTURE_REF);
    assert_eq!(warm[0].parts, WARM_COW_TEXTURED_PARTS.as_slice());

    let warm_baby = cow_textured_layer_passes(CowModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_WARM_COW_BABY);
    assert_eq!(warm_baby[0].texture, COW_WARM_BABY_TEXTURE_REF);
    assert_eq!(warm_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());

    let cold = cow_textured_layer_passes(CowModelVariant::Cold, false);
    assert_eq!(cold[0].model_layer, MODEL_LAYER_COLD_COW);
    assert_eq!(cold[0].texture, COW_COLD_TEXTURE_REF);
    assert_eq!(cold[0].parts, COLD_COW_TEXTURED_PARTS.as_slice());

    let cold_baby = cow_textured_layer_passes(CowModelVariant::Cold, true);
    assert_eq!(cold_baby[0].model_layer, MODEL_LAYER_COLD_COW_BABY);
    assert_eq!(cold_baby[0].texture, COW_COLD_BABY_TEXTURE_REF);
    assert_eq!(cold_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());
}

#[test]
fn cow_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_COW, "minecraft:cow#main");
    assert_eq!(MODEL_LAYER_COW_BABY, "minecraft:cow_baby#main");
    assert_eq!(MODEL_LAYER_WARM_COW, "minecraft:warm_cow#main");
    assert_eq!(MODEL_LAYER_WARM_COW_BABY, "minecraft:warm_cow_baby#main");
    assert_eq!(MODEL_LAYER_COLD_COW, "minecraft:cold_cow#main");
    assert_eq!(MODEL_LAYER_COLD_COW_BABY, "minecraft:cold_cow_baby#main");
    assert_eq!(
        ADULT_COW_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -4.0, -6.0],
            size: [8.0, 8.0, 6.0],
            uv_size: [8.0, 8.0, 6.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_COW_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [-2.0, 2.0, -8.0],
            size: [4.0, 6.0, 1.0],
            uv_size: [4.0, 6.0, 1.0],
            tex: [52.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        WARM_COW_TEXTURED_HEAD[4],
        TexturedModelCubeDesc {
            min: [4.0, -3.0, -5.0],
            size: [4.0, 2.0, 2.0],
            uv_size: [4.0, 2.0, 2.0],
            tex: [27.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(
        COLD_COW_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-6.5, -10.5, -7.5],
            size: [13.0, 19.0, 11.0],
            uv_size: [12.0, 18.0, 10.0],
            tex: [20.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(
        COLD_COW_TEXTURED_HEAD_CHILDREN[0],
        TexturedModelPartDesc {
            pose: COLD_COW_HEAD_CHILDREN[0].pose,
            cubes: &COLD_COW_TEXTURED_RIGHT_HORN,
            children: &[],
        }
    );
    assert_eq!(
        BABY_COW_TEXTURED_HEAD[2],
        TexturedModelCubeDesc {
            min: [-4.0, -5.569, -3.8333],
            size: [1.0, 2.0, 1.0],
            uv_size: [1.0, 2.0, 1.0],
            tex: [4.0, 29.0],
            mirror: true,
        }
    );
    assert_eq!(
        BABY_COW_TEXTURED_LEFT_HIND_LEG[0],
        TexturedModelCubeDesc {
            min: [-1.5, 0.0, -1.5],
            size: [3.0, 6.0, 3.0],
            uv_size: [3.0, 6.0, 3.0],
            tex: [34.0, 27.0],
            mirror: false,
        }
    );
}

#[test]
fn wolf_textured_mesh_uses_vanilla_uvs_and_collar_tint() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            305,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            Some(EntityDyeColor::Blue),
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [10.0 / 64.0, 32.0 / 256.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[144].uv, [4.0 / 64.0, 52.0 / 256.0]);
    assert_close2(mesh.vertices[264].uv, [10.0 / 64.0, 192.0 / 256.0]);
    assert_eq!(
        mesh.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );

    let untamed_with_collar_metadata = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            Some(EntityDyeColor::Red),
        )],
        &atlas,
    );
    assert_eq!(untamed_with_collar_metadata.cutout_faces, 66);
    assert!(untamed_with_collar_metadata
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn runtime_colored_mesh_excludes_texture_backed_entities() {
    let chicken = EntityModelInstance::chicken(303, [-2.0, 64.0, 0.0], 0.0, false);
    let sheep = EntityModelInstance::sheep(304, [0.0, 64.0, 0.0], 0.0, false);
    let wolf = EntityModelInstance::wolf(305, [2.0, 64.0, 0.0], 0.0, false);
    let boat = EntityModelInstance::boat(306, [4.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true);
    let pig = EntityModelInstance::pig(
        307,
        [6.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let cow =
        EntityModelInstance::cow_variant(308, [8.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, false);
    let player = EntityModelInstance::player(309, [10.0, 64.0, 0.0], 0.0, false);
    let creeper = EntityModelInstance::new(310, EntityModelKind::Creeper, [12.0, 64.0, 0.0], 0.0);
    let spider = EntityModelInstance::spider(311, [14.0, 64.0, 0.0], 0.0);
    let cave_spider = EntityModelInstance::cave_spider(312, [16.0, 64.0, 0.0], 0.0);
    let enderman = EntityModelInstance::enderman(313, [18.0, 64.0, 0.0], 0.0);
    let colored = entity_model_colored_runtime_mesh(&[
        chicken,
        sheep,
        wolf,
        boat,
        pig,
        cow,
        player,
        creeper,
        spider,
        cave_spider,
        enderman,
    ]);
    assert!(colored.vertices.is_empty());
    assert!(colored.indices.is_empty());
    let legacy_chicken_geometry_guard = entity_model_mesh(&[chicken]);
    assert!(!legacy_chicken_geometry_guard.vertices.is_empty());
    let legacy_geometry_guard = entity_model_mesh(&[sheep]);
    assert!(!legacy_geometry_guard.vertices.is_empty());
    let legacy_wolf_geometry_guard = entity_model_mesh(&[wolf]);
    assert!(!legacy_wolf_geometry_guard.vertices.is_empty());
    let legacy_boat_geometry_guard = entity_model_mesh(&[boat]);
    assert!(!legacy_boat_geometry_guard.vertices.is_empty());
    let legacy_pig_geometry_guard = entity_model_mesh(&[pig]);
    assert!(!legacy_pig_geometry_guard.vertices.is_empty());
    let legacy_cow_geometry_guard = entity_model_mesh(&[cow]);
    assert!(!legacy_cow_geometry_guard.vertices.is_empty());
    let legacy_player_geometry_guard = entity_model_mesh(&[player]);
    assert!(!legacy_player_geometry_guard.vertices.is_empty());
    let legacy_creeper_geometry_guard = entity_model_mesh(&[creeper]);
    assert!(!legacy_creeper_geometry_guard.vertices.is_empty());
    let legacy_spider_geometry_guard = entity_model_mesh(&[spider]);
    assert!(!legacy_spider_geometry_guard.vertices.is_empty());
    let legacy_cave_spider_geometry_guard = entity_model_mesh(&[cave_spider]);
    assert!(!legacy_cave_spider_geometry_guard.vertices.is_empty());
    let legacy_enderman_geometry_guard = entity_model_mesh(&[enderman]);
    assert!(!legacy_enderman_geometry_guard.vertices.is_empty());
}

#[test]
fn entity_textured_shader_samples_bound_texture_and_discards_alpha() {
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4]
    );
}

#[test]
fn entity_eyes_shader_samples_bound_texture_without_alpha_cutout() {
    assert!(ENTITY_MODEL_EYES_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4]
    );
}

#[test]
fn wolf_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_WOLF_REAL_HEAD,
        [
            ModelCubeDesc {
                min: [-2.0, -3.0, -2.0],
                size: [6.0, 6.0, 4.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [-2.0, -5.0, 0.0],
                size: [2.0, 2.0, 1.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [2.0, -5.0, 0.0],
                size: [2.0, 2.0, 1.0],
                color: WOLF_GRAY,
            },
            ModelCubeDesc {
                min: [-0.5, -0.001, -5.0],
                size: [3.0, 3.0, 4.0],
                color: WOLF_GRAY,
            },
        ]
    );
    assert_eq!(ADULT_WOLF_PARTS.len(), 8);
    assert_part_tree(
        &ADULT_WOLF_PARTS[0],
        [-1.0, 13.5, -7.0],
        [0.0, 0.0, 0.0],
        &[],
        ADULT_WOLF_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_WOLF_REAL_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_PARTS[1],
        [0.0, 14.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_WOLF_BODY.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_PARTS[2],
        [-1.0, 14.0, -3.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_WOLF_UPPER_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_WOLF_PARTS[3..7].iter().zip([
        [-2.5, 16.0, 7.0],
        [0.5, 16.0, 7.0],
        [-2.5, 16.0, -4.0],
        [0.5, 16.0, -4.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_WOLF_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &ADULT_WOLF_PARTS[7],
        [-1.0, 12.0, 8.0],
        [0.62831855, 0.0, 0.0],
        &[],
        ADULT_WOLF_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_WOLF_TAIL_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_WOLF_REAL_TAIL.as_slice(),
    );

    assert_eq!(
        BABY_WOLF_HEAD[0],
        ModelCubeDesc {
            min: [-3.015, -3.275, -3.025],
            size: [6.05, 5.05, 5.05],
            color: WOLF_GRAY,
        }
    );
    assert_eq!(BABY_WOLF_PARTS.len(), 7);
    assert_part_tree(
        &BABY_WOLF_PARTS[0],
        [0.0, 18.25, -4.0],
        [0.0, 0.0, 0.0],
        BABY_WOLF_HEAD.as_slice(),
        BABY_WOLF_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_WOLF_HEAD_CHILDREN[0],
        [-2.0, -4.25, -0.5],
        [0.0, 0.0, 0.0],
        BABY_WOLF_EAR.as_slice(),
    );
    assert_part(
        &BABY_WOLF_HEAD_CHILDREN[1],
        [2.0, -4.25, -0.5],
        [0.0, 0.0, 0.0],
        BABY_WOLF_EAR.as_slice(),
    );
    assert_part(
        &BABY_WOLF_PARTS[1],
        [0.0, 19.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_WOLF_BODY.as_slice(),
    );
    for (part, expected_offset) in BABY_WOLF_PARTS[2..6].iter().zip([
        [-1.5, 21.0, 3.0],
        [1.5, 21.0, 3.0],
        [-1.5, 21.0, -3.0],
        [1.5, 21.0, -3.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_WOLF_LEG.as_slice(),
        );
    }
    assert_part_tree(
        &BABY_WOLF_PARTS[6],
        [0.0, 19.0, 3.0],
        [-0.5236, 0.0, 0.0],
        &[],
        BABY_WOLF_TAIL_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_WOLF_TAIL_CHILDREN[0],
        [0.0, -0.6, 0.2],
        [-3.1, 0.0, 0.0],
        BABY_WOLF_TAIL_R1.as_slice(),
    );
}

#[test]
fn wolf_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::wolf(148, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(adult.opaque_faces, 66);
    assert_eq!(adult.vertices.len(), 264);
    assert_eq!(adult.indices.len(), 396);
    let (adult_min, adult_max) = mesh_extents(&adult);
    assert_close3(adult_min, [-0.25, 64.001, -0.8444562]);
    assert_close3(adult_max, [0.25000006, 64.96975, 0.75]);

    let baby = entity_model_mesh(&[EntityModelInstance::wolf(149, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert_close3(baby_min, [-0.1884375, 63.995087, -0.28114623]);
    assert_close3(baby_max, [0.18968754, 64.6885, 0.5625]);
}

#[test]
fn wolf_texture_refs_match_vanilla_renderer_pale_variant_assets() {
    let cases = [
        (
            false,
            false,
            false,
            "wolf",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf.png",
                size: [64, 32],
            },
        ),
        (
            false,
            true,
            false,
            "wolf_tame",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame.png",
                size: [64, 32],
            },
        ),
        (
            false,
            false,
            true,
            "wolf_angry",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry.png",
                size: [64, 32],
            },
        ),
        (
            true,
            false,
            false,
            "wolf_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            true,
            false,
            "wolf_tame_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            false,
            true,
            "wolf_angry_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry_baby.png",
                size: [32, 32],
            },
        ),
    ];
    for (baby, tame, angry, model_key, texture) in cases {
        let kind = EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color: None,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_COLLAR_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Wolf {
            baby: true,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_BABY_COLLAR_TEXTURE_REF]
    );
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        collar_color: None,
    }
    .vanilla_layer_texture_refs()
    .is_empty());
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        collar_color: Some(EntityDyeColor::Red),
    }
    .vanilla_layer_texture_refs()
    .is_empty());
}

#[test]
fn wolf_textured_layer_passes_match_vanilla_renderer_layers() {
    let wild = wolf_textured_layer_passes(false, false, false, None);
    assert_eq!(
        wild.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![EntityModelLayerKind::WolfBase]
    );
    assert_eq!(wild[0].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(wild[0].texture, WOLF_TEXTURE_REF);
    assert_eq!(wild[0].parts, ADULT_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(wild[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wild[0].collector_order, wild[0].submit_sequence), (0, 0));

    let tame_blue = wolf_textured_layer_passes(false, true, false, Some(EntityDyeColor::Blue));
    assert_eq!(
        tame_blue.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::WolfBase,
            EntityModelLayerKind::WolfCollar
        ]
    );
    assert_eq!(tame_blue[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_blue[1].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(tame_blue[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(tame_blue[1].parts, ADULT_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(
        tame_blue[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(
        (tame_blue[1].collector_order, tame_blue[1].submit_sequence),
        (1, 1)
    );

    let angry = wolf_textured_layer_passes(false, false, true, None);
    assert_eq!(angry[0].texture, WOLF_ANGRY_TEXTURE_REF);
    assert_eq!(angry.len(), 1);

    let tame_angry = wolf_textured_layer_passes(false, true, true, Some(EntityDyeColor::Red));
    assert_eq!(tame_angry[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_angry.len(), 2);

    let baby_tame = wolf_textured_layer_passes(true, true, false, Some(EntityDyeColor::Red));
    assert_eq!(baby_tame[0].model_layer, MODEL_LAYER_WOLF_BABY);
    assert_eq!(baby_tame[0].texture, WOLF_TAME_BABY_TEXTURE_REF);
    assert_eq!(baby_tame[0].parts, BABY_WOLF_TEXTURED_PARTS.as_slice());
    assert_eq!(baby_tame[1].texture, WOLF_BABY_COLLAR_TEXTURE_REF);
    assert_eq!(baby_tame[1].parts, BABY_WOLF_TEXTURED_PARTS.as_slice());
}

#[test]
fn wolf_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_WOLF, "minecraft:wolf#main");
    assert_eq!(MODEL_LAYER_WOLF_BABY, "minecraft:wolf_baby#main");
    assert_eq!(
        ADULT_WOLF_TEXTURED_REAL_HEAD[0],
        TexturedModelCubeDesc {
            min: [-2.0, -3.0, -2.0],
            size: [6.0, 6.0, 4.0],
            uv_size: [6.0, 6.0, 4.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_WOLF_TEXTURED_RIGHT_LEG[0],
        TexturedModelCubeDesc {
            min: [0.0, 0.0, -1.0],
            size: [2.0, 8.0, 2.0],
            uv_size: [2.0, 8.0, 2.0],
            tex: [0.0, 18.0],
            mirror: true,
        }
    );
    assert_eq!(
        BABY_WOLF_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.015, -3.275, -3.025],
            size: [6.05, 5.05, 5.05],
            uv_size: [6.0, 5.0, 5.0],
            tex: [0.0, 12.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_WOLF_TEXTURED_LEFT_EAR[0],
        TexturedModelCubeDesc {
            min: [-1.0, -1.0, -0.5],
            size: [2.0, 2.0, 1.0],
            uv_size: [2.0, 2.0, 1.0],
            tex: [20.0, 5.0],
            mirror: false,
        }
    );
}

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
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert_close3(baby_min, [-0.25000003, 64.001, -0.8233875]);
    assert_close3(baby_max, [0.25000003, 65.60652, 1.0918784]);
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
fn camel_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_CAMEL_TAIL[0],
        ModelCubeDesc {
            min: [-1.5, 0.0, 0.0],
            size: [3.0, 14.0, 0.0],
            color: CAMEL_TAN,
        }
    );
    assert_eq!(ADULT_CAMEL_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_CAMEL_PARTS[0],
        [0.0, 4.0, 9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_BODY.as_slice(),
        ADULT_CAMEL_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_BODY_CHILDREN[0],
        [0.0, -12.0, -10.0],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_HUMP.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_BODY_CHILDREN[1],
        [0.0, -9.0, 3.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_TAIL.as_slice(),
    );
    assert_part_tree(
        &ADULT_CAMEL_BODY_CHILDREN[2],
        [0.0, -3.0, -19.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_HEAD.as_slice(),
        ADULT_CAMEL_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_HEAD_CHILDREN[0],
        [2.5, -21.0, -9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_LEFT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_CAMEL_HEAD_CHILDREN[1],
        [-2.5, -21.0, -9.5],
        [0.0, 0.0, 0.0],
        ADULT_CAMEL_RIGHT_EAR.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_CAMEL_PARTS[1],
            [4.9, 1.0, 9.5],
            ADULT_CAMEL_LEFT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[2],
            [-4.9, 1.0, 9.5],
            ADULT_CAMEL_RIGHT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[3],
            [4.9, 1.0, -10.5],
            ADULT_CAMEL_LEFT_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_CAMEL_PARTS[4],
            [-4.9, 1.0, -10.5],
            ADULT_CAMEL_RIGHT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(
        BABY_CAMEL_TAIL[0],
        ModelCubeDesc {
            min: [-1.5, -0.5, 0.0],
            size: [3.0, 9.0, 0.0],
            color: CAMEL_TAN,
        }
    );
    assert_eq!(BABY_CAMEL_PARTS.len(), 5);
    assert_part_tree(
        &BABY_CAMEL_PARTS[0],
        [0.0, 7.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_BODY.as_slice(),
        BABY_CAMEL_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_BODY_CHILDREN[0],
        [0.0, -1.5, 8.05],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_TAIL.as_slice(),
    );
    assert_part_tree(
        &BABY_CAMEL_BODY_CHILDREN[1],
        [0.0, 1.0, -7.5],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_HEAD.as_slice(),
        BABY_CAMEL_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_HEAD_CHILDREN[0],
        [-2.5, -11.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_CAMEL_HEAD_CHILDREN[1],
        [2.5, -11.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_CAMEL_LEFT_EAR.as_slice(),
    );
    for (part, expected_offset) in [
        (&BABY_CAMEL_PARTS[1], [-3.0, 11.5, -5.5]),
        (&BABY_CAMEL_PARTS[2], [3.0, 11.5, -5.5]),
        (&BABY_CAMEL_PARTS[3], [3.0, 11.5, 5.5]),
        (&BABY_CAMEL_PARTS[4], [-3.0, 11.5, 5.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_CAMEL_LEG.as_slice(),
        );
    }
}

#[test]
fn camel_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::camel(
        180,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CAMEL_TAN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::camel(
        181,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::Camel,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 66);
    assert_eq!(baby.vertices.len(), 264);
    assert_eq!(baby.indices.len(), 396);

    let husk = entity_model_mesh(&[EntityModelInstance::camel(
        182,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        true,
    )]);
    assert_eq!(husk.opaque_faces, 72);
    assert_same_geometry(&husk, &adult);
    assert!(husk
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CAMEL_HUSK_BROWN, 0.78)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn camel_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            CamelModelFamily::Camel,
            false,
            "camel",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel.png",
                size: [128, 128],
            },
        ),
        (
            CamelModelFamily::Camel,
            true,
            "camel_baby",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_baby.png",
                size: [64, 64],
            },
        ),
        (
            CamelModelFamily::CamelHusk,
            false,
            "camel_husk",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_husk.png",
                size: [128, 128],
            },
        ),
        (
            CamelModelFamily::CamelHusk,
            true,
            "camel_husk",
            EntityModelTextureRef {
                path: "textures/entity/camel/camel_husk.png",
                size: [128, 128],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Camel { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn llama_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_LLAMA_HEAD[0],
        ModelCubeDesc {
            min: [-2.0, -14.0, -10.0],
            size: [4.0, 4.0, 9.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(
        ADULT_LLAMA_HEAD[1],
        ModelCubeDesc {
            min: [-4.0, -16.0, -6.0],
            size: [8.0, 18.0, 6.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(ADULT_LLAMA_PARTS.len(), 6);
    assert_part(
        &ADULT_LLAMA_PARTS[0],
        [0.0, 7.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_LLAMA_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_LLAMA_BODY.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_RIGHT_CHEST_PART,
        [-8.5, 3.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        LLAMA_CHEST.as_slice(),
    );
    assert_part(
        &ADULT_LLAMA_LEFT_CHEST_PART,
        [5.5, 3.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        LLAMA_CHEST.as_slice(),
    );
    assert_eq!(ADULT_LLAMA_PARTS_WITH_CHEST.len(), 8);
    for (part, expected_offset) in [
        (&ADULT_LLAMA_PARTS[2], [-3.5, 10.0, 6.0]),
        (&ADULT_LLAMA_PARTS[3], [3.5, 10.0, 6.0]),
        (&ADULT_LLAMA_PARTS[4], [-3.5, 10.0, -5.0]),
        (&ADULT_LLAMA_PARTS[5], [3.5, 10.0, -5.0]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_LLAMA_LEG.as_slice(),
        );
    }

    assert_eq!(
        BABY_LLAMA_HEAD[0],
        ModelCubeDesc {
            min: [-3.0, -9.0, -4.0],
            size: [6.0, 11.0, 4.0],
            color: LLAMA_CREAMY,
        }
    );
    assert_eq!(BABY_LLAMA_PARTS.len(), 6);
    assert_part(
        &BABY_LLAMA_PARTS[0],
        [0.0, 12.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_LLAMA_HEAD.as_slice(),
    );
    assert_part(
        &BABY_LLAMA_PARTS[5],
        [0.0, 14.0, 2.5],
        [0.0, 0.0, 0.0],
        BABY_LLAMA_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &BABY_LLAMA_PARTS[1],
            [-2.5, 16.5, 4.5],
            BABY_LLAMA_RIGHT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[2],
            [2.5, 16.5, 4.5],
            BABY_LLAMA_LEFT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[3],
            [-2.5, 16.5, -3.5],
            BABY_LLAMA_RIGHT_LEG.as_slice(),
        ),
        (
            &BABY_LLAMA_PARTS[4],
            [2.5, 16.5, -3.5],
            BABY_LLAMA_LEFT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
}

#[test]
fn llama_meshes_use_vanilla_body_layer_geometry_and_chest_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::llama(
        190,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Creamy,
        false,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 54);
    assert_eq!(adult.vertices.len(), 216);
    assert_eq!(adult.indices.len(), 324);

    let adult_with_chest = entity_model_mesh(&[EntityModelInstance::llama(
        191,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Brown,
        false,
        true,
    )]);
    assert_eq!(adult_with_chest.opaque_faces, 66);
    assert_eq!(adult_with_chest.vertices.len(), 264);
    assert_eq!(adult_with_chest.indices.len(), 396);
    assert!(adult_with_chest
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LLAMA_BROWN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::llama(
        192,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::Llama,
        LlamaVariant::Gray,
        true,
        false,
    )]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
    assert_eq!(baby.indices.len(), 324);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LLAMA_GRAY, 0.78)));

    let baby_with_chest_metadata = entity_model_mesh(&[EntityModelInstance::llama(
        193,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Gray,
        true,
        true,
    )]);
    assert_same_geometry(&baby_with_chest_metadata, &baby);

    let trader = entity_model_mesh(&[EntityModelInstance::llama(
        194,
        [0.0, 64.0, 0.0],
        0.0,
        LlamaModelFamily::TraderLlama,
        LlamaVariant::Creamy,
        false,
        false,
    )]);
    assert_same_geometry(&trader, &adult);

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn llama_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            LlamaVariant::Creamy,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_creamy.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Creamy,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_creamy_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::White,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_white.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::White,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_white_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::Brown,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_brown.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Brown,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_brown_baby.png",
                size: [64, 64],
            },
        ),
        (
            LlamaVariant::Gray,
            false,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_gray.png",
                size: [128, 64],
            },
        ),
        (
            LlamaVariant::Gray,
            true,
            EntityModelTextureRef {
                path: "textures/entity/llama/llama_gray_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (variant, baby, texture) in cases {
        let llama = EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant,
            baby,
            has_chest: false,
        };
        let trader = EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant,
            baby,
            has_chest: false,
        };
        assert_eq!(llama.vanilla_texture_ref(), Some(texture));
        assert_eq!(trader.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn goat_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_GOAT_HEAD[2],
        ModelCubeDesc {
            min: [-0.5, -3.0, -14.0],
            size: [0.0, 7.0, 5.0],
            color: GOAT_BEARD,
        }
    );
    assert_eq!(ADULT_GOAT_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_GOAT_PARTS[ADULT_GOAT_HEAD_INDEX],
        [1.0, 14.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_HEAD.as_slice(),
        ADULT_GOAT_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[ADULT_GOAT_LEFT_HORN_CHILD_INDEX],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_LEFT_HORN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[ADULT_GOAT_RIGHT_HORN_CHILD_INDEX],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_RIGHT_HORN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[2],
        [0.0, -8.0, -8.0],
        [0.9599, 0.0, 0.0],
        ADULT_GOAT_NOSE.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_PARTS[1],
        [0.0, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_GOAT_PARTS[2],
            [1.0, 14.0, 4.0],
            ADULT_GOAT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[3],
            [-3.0, 14.0, 4.0],
            ADULT_GOAT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[4],
            [1.0, 14.0, -6.0],
            ADULT_GOAT_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[5],
            [-3.0, 14.0, -6.0],
            ADULT_GOAT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_GOAT_PARTS.len(), 6);
    for (part, expected_offset) in [
        (&BABY_GOAT_PARTS[0], [1.5, 19.5, 3.0]),
        (&BABY_GOAT_PARTS[1], [-1.5, 19.5, 3.0]),
        (&BABY_GOAT_PARTS[2], [-1.5, 19.5, -2.0]),
        (&BABY_GOAT_PARTS[3], [1.5, 19.5, -2.0]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_GOAT_LEG.as_slice(),
        );
    }
    assert_part(
        &BABY_GOAT_PARTS[4],
        [0.0, 17.8, 0.0],
        [0.0, 0.0, 0.0],
        BABY_GOAT_BODY.as_slice(),
    );
    assert_part_tree(
        &BABY_GOAT_PARTS[BABY_GOAT_HEAD_INDEX],
        [0.0, 15.5, -3.0],
        [0.4363, 0.0, 0.0],
        BABY_GOAT_HEAD.as_slice(),
        BABY_GOAT_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[BABY_GOAT_RIGHT_HORN_CHILD_INDEX],
        [-1.5, -1.5, -1.0],
        [-0.3926991, 0.0, 0.0],
        BABY_GOAT_RIGHT_HORN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[BABY_GOAT_LEFT_HORN_CHILD_INDEX],
        [-1.5, -1.5, -1.0],
        [-0.3926991, 0.0, 0.0],
        BABY_GOAT_LEFT_HORN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[2],
        [-1.7, -2.3126, 0.1452],
        [0.0, -0.5236, 0.0],
        BABY_GOAT_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[3],
        [1.7, -2.3126, 0.1452],
        [0.0, 0.5236, 0.0],
        BABY_GOAT_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[4],
        [0.0, -1.3126, -1.1548],
        [0.0, 0.0, 0.0],
        BABY_GOAT_HEAD_MAIN.as_slice(),
    );
}

#[test]
fn goat_meshes_use_vanilla_body_layers_and_horn_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::goat(
        200,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let adult_left_horn_only = entity_model_mesh(&[EntityModelInstance::goat(
        201,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
    )]);
    assert_eq!(adult_left_horn_only.opaque_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);
    assert_eq!(adult_left_horn_only.indices.len(), 396);

    let adult_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        202,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        false,
    )]);
    assert_eq!(adult_no_horns.opaque_faces, 60);
    assert!(!adult_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::goat(
        203,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 72);
    assert_eq!(baby.vertices.len(), 288);
    assert_eq!(baby.indices.len(), 432);

    let baby_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        204,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        false,
    )]);
    assert_eq!(baby_no_horns.opaque_faces, 60);
    assert!(!baby_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn goat_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "goat",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "goat_baby",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::Goat {
            baby,
            left_horn: false,
            right_horn: true,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn polar_bear_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ADULT_POLAR_BEAR_PARTS.len(), 6);
    assert_part(
        &ADULT_POLAR_BEAR_PARTS[0],
        [0.0, 10.0, -16.0],
        [0.0, 0.0, 0.0],
        ADULT_POLAR_BEAR_HEAD.as_slice(),
    );
    assert_eq!(
        ADULT_POLAR_BEAR_HEAD[1],
        ModelCubeDesc {
            min: [-2.5, 1.0, -6.0],
            size: [5.0, 3.0, 3.0],
            color: POLAR_BEAR_WHITE,
        }
    );
    assert_part(
        &ADULT_POLAR_BEAR_PARTS[1],
        [-2.0, 9.0, 12.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_POLAR_BEAR_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_POLAR_BEAR_PARTS[2],
            [-4.5, 14.0, 6.0],
            ADULT_POLAR_BEAR_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[3],
            [4.5, 14.0, 6.0],
            ADULT_POLAR_BEAR_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[4],
            [-3.5, 14.0, -8.0],
            ADULT_POLAR_BEAR_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[5],
            [3.5, 14.0, -8.0],
            ADULT_POLAR_BEAR_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_POLAR_BEAR_PARTS.len(), 6);
    assert_part(
        &BABY_POLAR_BEAR_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_POLAR_BEAR_BODY.as_slice(),
    );
    assert_part(
        &BABY_POLAR_BEAR_PARTS[1],
        [0.0, 18.625, -5.75],
        [0.0, 0.0, 0.0],
        BABY_POLAR_BEAR_HEAD.as_slice(),
    );
    assert_eq!(
        BABY_POLAR_BEAR_HEAD[1],
        ModelCubeDesc {
            min: [-2.0, 0.375, -6.25],
            size: [4.0, 2.0, 2.0],
            color: POLAR_BEAR_WHITE,
        }
    );
    for (part, expected_offset) in [
        (&BABY_POLAR_BEAR_PARTS[2], [-2.5, 21.5, 4.5]),
        (&BABY_POLAR_BEAR_PARTS[3], [2.5, 21.5, 4.5]),
        (&BABY_POLAR_BEAR_PARTS[4], [-2.5, 21.5, -4.5]),
        (&BABY_POLAR_BEAR_PARTS[5], [2.5, 21.5, -4.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_POLAR_BEAR_LEG.as_slice(),
        );
    }
}

#[test]
fn polar_bear_meshes_use_vanilla_body_layers() {
    let adult = entity_model_mesh(&[EntityModelInstance::polar_bear(
        210,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 60);
    assert_eq!(adult.vertices.len(), 240);
    assert_eq!(adult.indices.len(), 360);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(POLAR_BEAR_WHITE, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::polar_bear(
        211,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
    assert_eq!(baby.indices.len(), 324);

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn polar_bear_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "polar_bear",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear.png",
                size: [128, 64],
            },
        ),
        (
            true,
            "polar_bear_baby",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::PolarBear { baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_VILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.51, -10.51, -4.51],
            size: [9.02, 11.02, 9.02],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(
        ADULT_VILLAGER_JACKET[0],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_VILLAGER_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HEAD.as_slice(),
        ADULT_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HAT.as_slice(),
        ADULT_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_NOSE.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_BODY.as_slice(),
        ADULT_VILLAGER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_JACKET.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ADULT_VILLAGER_ARMS.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
}

#[test]
fn villager_adult_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875003, 64.00094, -0.46875006]);
    assert_close3(max, [0.46875003, 66.02301, 0.46875003]);

    let wandering_trader_mesh = entity_model_mesh(&[EntityModelInstance::wandering_trader(
        141,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(wandering_trader_mesh.opaque_faces, mesh.opaque_faces);
    assert_eq!(wandering_trader_mesh.vertices, mesh.vertices);
    assert_eq!(wandering_trader_mesh.indices, mesh.indices);
}

#[test]
fn villager_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_VILLAGER_RIGHT_HAND,
        [
            ModelCubeDesc {
                min: [-1.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [5.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        BABY_VILLAGER_BB_MAIN[0],
        ModelCubeDesc {
            min: [-2.7, -8.2, -1.7],
            size: [4.4, 6.4, 3.4],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &BABY_VILLAGER_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_VILLAGER_ARMS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[0],
        [-3.0, 1.4025, -0.9599],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_RIGHT_HAND.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[1],
        [0.0, 0.9024, -1.8175],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_MIDDLE_ARM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[1],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[2],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part_tree(
        &BABY_VILLAGER_PARTS[3],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HEAD.as_slice(),
        BABY_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[4],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[5],
        [0.5, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BB_MAIN.as_slice(),
    );
}

#[test]
fn villager_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(max, [0.43750003, 65.01975, 0.37500003]);
}

#[test]
fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Villager { baby: false }.model_key(),
        "villager"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/wandering_trader/wandering_trader.png",
            size: [64, 64],
        })
    );
}

#[test]
fn creeper_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        CREEPER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_LEG[0],
        ModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            color: CREEPER_GREEN
        }
    );

    assert_eq!(CREEPER_PARTS.len(), 6);
    assert_eq!(CREEPER_PARTS[0].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[0].cubes, CREEPER_HEAD.as_slice());
    assert_eq!(CREEPER_PARTS[1].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[1].cubes, CREEPER_BODY.as_slice());

    let leg_offsets = [
        [-2.0, 18.0, 4.0],
        [2.0, 18.0, 4.0],
        [-2.0, 18.0, -4.0],
        [2.0, 18.0, -4.0],
    ];
    for (part, expected_offset) in CREEPER_PARTS[2..].iter().zip(leg_offsets) {
        assert_eq!(part.pose.offset, expected_offset);
        assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(part.cubes, CREEPER_LEG.as_slice());
        assert!(part.children.is_empty());
    }
}

#[test]
fn creeper_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::new(
        50,
        EntityModelKind::Creeper,
        [0.0, 64.0, 0.0],
        0.0,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Creeper.model_key(), "creeper");
    assert_eq!(
        EntityModelKind::Creeper.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/creeper/creeper.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate.png",
            size: [64, 32],
        })
    );
}

#[test]
fn creeper_textured_layer_passes_match_vanilla_renderer_model_layer() {
    let passes = creeper_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CreeperBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_CREEPER);
    assert_eq!(passes[0].texture, CREEPER_TEXTURE_REF);
    assert_eq!(passes[0].parts, CREEPER_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn creeper_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CREEPER, "minecraft:creeper#main");
    assert_eq!(CREEPER_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        CREEPER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_LEG[0],
        TexturedModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            uv_size: [4.0, 6.0, 4.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(CREEPER_TEXTURED_PARTS[0].pose, CREEPER_PARTS[0].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[1].pose, CREEPER_PARTS[1].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[5].pose, CREEPER_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_creeper_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 32);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/creeper/creeper.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn creeper_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::new(
            910,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn slime_and_magma_cube_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(SLIME_PARTS.len(), 5);
    assert_part(
        &SLIME_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_INNER_CUBE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_RIGHT_EYE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[2],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_LEFT_EYE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[3],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_MOUTH.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[4],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_OUTER_CUBE.as_slice(),
    );

    let magma_segments = [
        MAGMA_CUBE_SEGMENT_0.as_slice(),
        MAGMA_CUBE_SEGMENT_1.as_slice(),
        MAGMA_CUBE_SEGMENT_2.as_slice(),
        MAGMA_CUBE_SEGMENT_3.as_slice(),
        MAGMA_CUBE_SEGMENT_4.as_slice(),
        MAGMA_CUBE_SEGMENT_5.as_slice(),
        MAGMA_CUBE_SEGMENT_6.as_slice(),
        MAGMA_CUBE_SEGMENT_7.as_slice(),
    ];
    for (index, (part, cubes)) in MAGMA_CUBE_PARTS[..8].iter().zip(magma_segments).enumerate() {
        assert_part(part, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], cubes);
        assert_eq!(part.cubes[0].min, [-4.0, 16.0 + index as f32, -4.0]);
        assert_eq!(part.cubes[0].size, [8.0, 1.0, 8.0]);
    }
    assert_part(
        &MAGMA_CUBE_PARTS[8],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        MAGMA_CUBE_INSIDE_CUBE.as_slice(),
    );
}

#[test]
fn slime_and_magma_cube_meshes_use_vanilla_size_scaling() {
    let slime = entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 1)]);
    assert_eq!(slime.opaque_faces, 30);
    assert_eq!(slime.vertices.len(), 120);
    assert_eq!(slime.indices.len(), 180);
    let (slime_min, slime_max) = mesh_extents(&slime);
    assert_close3(slime_min, [-0.24975, 64.0, -0.24975]);
    assert_close3(slime_max, [0.24975, 64.4995, 0.24975]);

    let large_slime =
        entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 4)]);
    assert_eq!(large_slime.opaque_faces, slime.opaque_faces);
    let (large_slime_min, large_slime_max) = mesh_extents(&large_slime);
    assert_close3(large_slime_min, [-0.999, 64.00299, -0.999]);
    assert_close3(large_slime_max, [0.999, 66.00099, 0.999]);

    let magma_cube = entity_model_mesh(&[EntityModelInstance::magma_cube(
        80,
        [0.0, 64.0, 0.0],
        0.0,
        3,
    )]);
    assert_eq!(magma_cube.opaque_faces, 54);
    assert_eq!(magma_cube.vertices.len(), 216);
    assert_eq!(magma_cube.indices.len(), 324);
    let (magma_min, magma_max) = mesh_extents(&magma_cube);
    assert_close3(magma_min, [-0.75, 64.003, -0.75]);
    assert_close3(magma_max, [0.75, 65.503, 0.75]);
}

#[test]
fn slime_and_magma_cube_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::Slime { size: 4 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/slime/slime.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/slime/magmacube.png",
            size: [64, 64],
        })
    );
}

#[test]
fn iron_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        IRON_GOLEM_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -12.0, -5.5],
                size: [8.0, 10.0, 8.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-1.0, -5.0, -7.5],
                size: [2.0, 4.0, 2.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_BODY,
        [
            ModelCubeDesc {
                min: [-9.0, -2.0, -6.0],
                size: [18.0, 12.0, 11.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-5.0, 9.5, -3.5],
                size: [10.0, 6.0, 7.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_ARM[0],
        ModelCubeDesc {
            min: [-13.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_LEFT_ARM[0],
        ModelCubeDesc {
            min: [9.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_LEG[0],
        ModelCubeDesc {
            min: [-3.5, -3.0, -3.0],
            size: [6.0, 16.0, 5.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(IRON_GOLEM_LEFT_LEG, IRON_GOLEM_RIGHT_LEG);

    assert_eq!(IRON_GOLEM_PARTS.len(), 6);
    let part_specs = [
        ([0.0, -7.0, -2.0], IRON_GOLEM_HEAD.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_BODY.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_RIGHT_ARM.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_LEFT_ARM.as_slice()),
        ([-4.0, 11.0, 0.0], IRON_GOLEM_RIGHT_LEG.as_slice()),
        ([5.0, 11.0, 0.0], IRON_GOLEM_LEFT_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in IRON_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn iron_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.8125, 64.001, -0.3125]);
    assert_close3(max, [0.8125, 66.6885, 0.59375]);
}

#[test]
fn iron_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(
        EntityModelKind::IronGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/iron_golem/iron_golem.png",
            size: [128, 128],
        })
    );
}

#[test]
fn snow_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SNOW_GOLEM_HEAD[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_ARM[0],
        ModelCubeDesc {
            min: [-0.5, 0.5, -0.5],
            size: [11.0, 1.0, 1.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_UPPER_BODY[0],
        ModelCubeDesc {
            min: [-4.5, -9.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_LOWER_BODY[0],
        ModelCubeDesc {
            min: [-5.5, -11.5, -5.5],
            size: [11.0, 11.0, 11.0],
            color: SNOW_GOLEM_WHITE,
        }
    );

    assert_eq!(SNOW_GOLEM_PARTS.len(), 5);
    let part_specs = [
        ([0.0, 4.0, 0.0], [0.0, 0.0, 0.0], SNOW_GOLEM_HEAD.as_slice()),
        ([5.0, 6.0, 1.0], [0.0, 0.0, 1.0], SNOW_GOLEM_ARM.as_slice()),
        (
            [-5.0, 6.0, -1.0],
            [0.0, std::f32::consts::PI, -1.0],
            SNOW_GOLEM_ARM.as_slice(),
        ),
        (
            [0.0, 13.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_UPPER_BODY.as_slice(),
        ),
        (
            [0.0, 24.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_LOWER_BODY.as_slice(),
        ),
    ];
    for (part, (offset, rotation, cubes)) in SNOW_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, rotation, cubes);
    }
}

#[test]
fn snow_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(
        EntityModelKind::SnowGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/snow_golem/snow_golem.png",
            size: [64, 64],
        })
    );
}

#[test]
fn witch_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        WITCH_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: WITCH_ROBE,
        }
    );
    assert_eq!(
        WITCH_HAT_4[0],
        ModelCubeDesc {
            min: [-0.25, -0.25, -0.25],
            size: [1.5, 2.5, 1.5],
            color: WITCH_HAT_COLOR,
        }
    );
    assert_eq!(
        WITCH_MOLE[0],
        ModelCubeDesc {
            min: [0.25, 3.25, -6.5],
            size: [0.5, 0.5, 0.5],
            color: WITCH_ROBE,
        }
    );

    assert_eq!(WITCH_PARTS.len(), 5);
    assert_part_tree(
        &WITCH_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_HEAD.as_slice(),
        WITCH_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[0],
        [-5.0, -10.03125, -5.0],
        [0.0, 0.0, 0.0],
        WITCH_HAT.as_slice(),
        WITCH_HAT_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.05235988, 0.0, 0.02617994],
        WITCH_HAT_2.as_slice(),
        WITCH_HAT_2_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_2_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.10471976, 0.0, 0.05235988],
        WITCH_HAT_3.as_slice(),
        WITCH_HAT_3_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_HAT_3_CHILDREN[0],
        [1.75, -2.0, 2.0],
        [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
        WITCH_HAT_4.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_NOSE.as_slice(),
        WITCH_NOSE_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_NOSE_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_MOLE.as_slice(),
    );
    assert_part_tree(
        &WITCH_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_BODY.as_slice(),
        WITCH_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_JACKET.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        WITCH_ARMS.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
}

#[test]
fn witch_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

#[test]
fn witch_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Witch.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/witch/witch.png",
            size: [64, 128],
        })
    );
}

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
fn entity_model_root_transform_rotates_instances_by_body_yaw() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        26,
        [10.0, 64.0, -3.0],
        90.0,
        false,
    )]);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [9.5, 64.001, -3.25]);
    assert_close3(max, [10.25, 64.9385, -2.75]);
}

#[test]
fn humanoid_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        HumanoidModelFamily::Player,
        HumanoidModelFamily::Zombie,
        HumanoidModelFamily::Skeleton,
        HumanoidModelFamily::Villager,
        HumanoidModelFamily::Illager,
        HumanoidModelFamily::ArmorStand,
    ] {
        let instance = EntityModelInstance::humanoid(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn quadruped_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        QuadrupedModelFamily::Pig,
        QuadrupedModelFamily::Cow,
        QuadrupedModelFamily::Sheep,
        QuadrupedModelFamily::Horse,
        QuadrupedModelFamily::Wolf,
    ] {
        let instance = EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn boat_model_parts_match_vanilla_26_1_layers() {
    assert_eq!(BOAT_COMMON_PARTS.len(), 7);
    assert_part(
        &BOAT_COMMON_PARTS[0],
        [0.0, 3.0, 1.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        BOAT_BOTTOM.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[1],
        [-15.0, 4.0, 4.0],
        [0.0, std::f32::consts::PI * 1.5, 0.0],
        BOAT_BACK.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[2],
        [15.0, 4.0, 0.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_FRONT.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[3],
        [0.0, 4.0, -9.0],
        [0.0, std::f32::consts::PI, 0.0],
        BOAT_SIDE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[4],
        [0.0, 4.0, 9.0],
        [0.0, 0.0, 0.0],
        BOAT_SIDE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[5],
        [3.0, -5.0, 9.0],
        [0.0, 0.0, std::f32::consts::PI / 16.0],
        BOAT_LEFT_PADDLE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[6],
        [3.0, -5.0, -9.0],
        [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        BOAT_RIGHT_PADDLE.as_slice(),
    );

    assert_eq!(BOAT_CHEST_PARTS.len(), 3);
    assert_part(
        &BOAT_CHEST_PARTS[0],
        [-2.0, -5.0, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_BOTTOM.as_slice(),
    );
    assert_part(
        &BOAT_CHEST_PARTS[1],
        [-2.0, -9.0, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LID.as_slice(),
    );
    assert_part(
        &BOAT_CHEST_PARTS[2],
        [-1.0, -6.0, -1.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LOCK.as_slice(),
    );

    assert_eq!(RAFT_COMMON_PARTS.len(), 3);
    assert_part(
        &RAFT_COMMON_PARTS[0],
        [0.0, -2.1, 1.0],
        [1.5708, 0.0, 0.0],
        RAFT_BOTTOM.as_slice(),
    );
    assert_part(
        &RAFT_COMMON_PARTS[1],
        [3.0, -4.0, 9.0],
        [0.0, 0.0, std::f32::consts::PI / 16.0],
        BOAT_LEFT_PADDLE.as_slice(),
    );
    assert_part(
        &RAFT_COMMON_PARTS[2],
        [3.0, -4.0, -9.0],
        [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        BOAT_RIGHT_PADDLE.as_slice(),
    );

    assert_eq!(RAFT_CHEST_PARTS.len(), 3);
    assert_part(
        &RAFT_CHEST_PARTS[0],
        [-2.0, -10.1, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_BOTTOM.as_slice(),
    );
    assert_part(
        &RAFT_CHEST_PARTS[1],
        [-2.0, -14.1, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LID.as_slice(),
    );
    assert_part(
        &RAFT_CHEST_PARTS[2],
        [-1.0, -11.1, -1.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LOCK.as_slice(),
    );
}

#[test]
fn boat_meshes_use_vanilla_body_layer_geometry() {
    let oak_boat = entity_model_mesh(&[EntityModelInstance::boat(
        89,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Oak,
        false,
    )]);
    let oak_chest_boat = entity_model_mesh(&[EntityModelInstance::boat(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Oak,
        true,
    )]);
    let bamboo_raft = entity_model_mesh(&[EntityModelInstance::boat(
        9,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Bamboo,
        false,
    )]);
    let bamboo_chest_raft = entity_model_mesh(&[EntityModelInstance::boat(
        8,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Bamboo,
        true,
    )]);

    assert_eq!(oak_boat.opaque_faces, 54);
    assert_eq!(oak_boat.vertices.len(), 216);
    assert_eq!(oak_boat.indices.len(), 324);
    assert_eq!(oak_chest_boat.opaque_faces, 72);
    assert_eq!(oak_chest_boat.vertices.len(), 288);
    assert_eq!(oak_chest_boat.indices.len(), 432);
    assert_eq!(bamboo_raft.opaque_faces, 36);
    assert_eq!(bamboo_raft.vertices.len(), 144);
    assert_eq!(bamboo_raft.indices.len(), 216);
    assert_eq!(bamboo_chest_raft.opaque_faces, 54);
    assert_eq!(bamboo_chest_raft.vertices.len(), 216);
    assert_eq!(bamboo_chest_raft.indices.len(), 324);
    assert_ne!(oak_boat.vertices, bamboo_raft.vertices);

    let (min, max) = mesh_extents(&oak_boat);
    assert!(max[0] - min[0] > 1.0);
    assert!(max[2] - min[2] > 1.0);
}

#[test]
fn boat_texture_refs_match_vanilla_model_layer_paths() {
    let cases = [
        (
            BoatModelFamily::Acacia,
            false,
            "boat_acacia",
            "textures/entity/boat/acacia.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Bamboo,
            true,
            "chest_boat_bamboo",
            "textures/entity/chest_boat/bamboo.png",
            [128, 128],
        ),
        (
            BoatModelFamily::DarkOak,
            false,
            "boat_dark_oak",
            "textures/entity/boat/dark_oak.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Mangrove,
            true,
            "chest_boat_mangrove",
            "textures/entity/chest_boat/mangrove.png",
            [128, 128],
        ),
        (
            BoatModelFamily::PaleOak,
            false,
            "boat_pale_oak",
            "textures/entity/boat/pale_oak.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Spruce,
            true,
            "chest_boat_spruce",
            "textures/entity/chest_boat/spruce.png",
            [128, 128],
        ),
    ];

    for (family, chest, model_key, path, size) in cases {
        let kind = EntityModelKind::Boat { family, chest };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(
            kind.vanilla_texture_ref(),
            Some(EntityModelTextureRef { path, size })
        );
    }
}

#[test]
fn boat_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let oak_boat = boat_textured_layer_passes(BoatModelFamily::Oak, false);
    assert_eq!(oak_boat.len(), 1);
    assert_eq!(oak_boat[0].kind, EntityModelLayerKind::BoatBase);
    assert_eq!(oak_boat[0].model_layer, MODEL_LAYER_OAK_BOAT);
    assert_eq!(oak_boat[0].texture, BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_boat[0].parts, BOAT_TEXTURED_PARTS.as_slice());
    assert_eq!(oak_boat[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (oak_boat[0].collector_order, oak_boat[0].submit_sequence),
        (0, 0)
    );

    let oak_chest_boat = boat_textured_layer_passes(BoatModelFamily::Oak, true);
    assert_eq!(oak_chest_boat[0].kind, EntityModelLayerKind::BoatBase);
    assert_eq!(oak_chest_boat[0].model_layer, MODEL_LAYER_OAK_CHEST_BOAT);
    assert_eq!(oak_chest_boat[0].texture, CHEST_BOAT_OAK_TEXTURE_REF);
    assert_eq!(
        oak_chest_boat[0].parts,
        BOAT_CHEST_TEXTURED_PARTS.as_slice()
    );

    let bamboo_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, false);
    assert_eq!(bamboo_raft[0].model_layer, MODEL_LAYER_BAMBOO_RAFT);
    assert_eq!(bamboo_raft[0].texture, BOAT_BAMBOO_TEXTURE_REF);
    assert_eq!(bamboo_raft[0].parts, RAFT_TEXTURED_PARTS.as_slice());

    let bamboo_chest_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, true);
    assert_eq!(
        bamboo_chest_raft[0].model_layer,
        MODEL_LAYER_BAMBOO_CHEST_RAFT
    );
    assert_eq!(bamboo_chest_raft[0].texture, CHEST_BOAT_BAMBOO_TEXTURE_REF);
    assert_eq!(
        bamboo_chest_raft[0].parts,
        RAFT_CHEST_TEXTURED_PARTS.as_slice()
    );
}

#[test]
fn boat_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_OAK_BOAT, "minecraft:boat/oak#main");
    assert_eq!(MODEL_LAYER_OAK_CHEST_BOAT, "minecraft:chest_boat/oak#main");
    assert_eq!(MODEL_LAYER_BAMBOO_RAFT, "minecraft:boat/bamboo#main");
    assert_eq!(
        MODEL_LAYER_BAMBOO_CHEST_RAFT,
        "minecraft:chest_boat/bamboo#main"
    );
    assert_eq!(BOAT_TEXTURED_PARTS.len(), 7);
    assert_eq!(BOAT_CHEST_TEXTURED_PARTS.len(), 10);
    assert_eq!(RAFT_TEXTURED_PARTS.len(), 3);
    assert_eq!(RAFT_CHEST_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        BOAT_TEXTURED_BOTTOM[0],
        TexturedModelCubeDesc {
            min: [-14.0, -9.0, -3.0],
            size: [28.0, 16.0, 3.0],
            uv_size: [28.0, 16.0, 3.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        BOAT_TEXTURED_RIGHT_SIDE[0],
        TexturedModelCubeDesc {
            min: [-14.0, -7.0, -1.0],
            size: [28.0, 6.0, 2.0],
            uv_size: [28.0, 6.0, 2.0],
            tex: [0.0, 35.0],
            mirror: false,
        }
    );
    assert_eq!(BOAT_TEXTURED_LEFT_SIDE[0].tex, [0.0, 43.0]);
    assert_eq!(
        BOAT_TEXTURED_LEFT_PADDLE[1],
        TexturedModelCubeDesc {
            min: [-1.001, -3.0, 8.0],
            size: [1.0, 6.0, 7.0],
            uv_size: [1.0, 6.0, 7.0],
            tex: [62.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        RAFT_TEXTURED_BOTTOM[1],
        TexturedModelCubeDesc {
            min: [-14.0, -9.0, -8.0],
            size: [28.0, 16.0, 4.0],
            uv_size: [28.0, 16.0, 4.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(RAFT_TEXTURED_LEFT_PADDLE[0].tex, [0.0, 24.0]);
    assert_eq!(RAFT_TEXTURED_RIGHT_PADDLE[0].tex, [40.0, 24.0]);
    assert_eq!(
        BOAT_TEXTURED_CHEST_BOTTOM[0],
        TexturedModelCubeDesc {
            min: [0.0, 0.0, 0.0],
            size: [12.0, 8.0, 12.0],
            uv_size: [12.0, 8.0, 12.0],
            tex: [0.0, 76.0],
            mirror: false,
        }
    );
    assert_eq!(BOAT_TEXTURED_CHEST_LID[0].tex, [0.0, 59.0]);
    assert_eq!(BOAT_TEXTURED_CHEST_LOCK[0].tex, [0.0, 59.0]);
    assert_eq!(BOAT_CHEST_TEXTURED_PARTS[7].pose, BOAT_CHEST_PARTS[0].pose);
    assert_eq!(RAFT_CHEST_TEXTURED_PARTS[3].pose, RAFT_CHEST_PARTS[0].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_boat_png_slots() {
    let images = boat_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 1920);
    assert_eq!(layout.entries.len(), 20);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/boat/acacia.png"
    );
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/chest_boat/acacia.png"
    );
    assert_eq!(
        layout.entries[14].texture.path,
        "textures/entity/boat/oak.png"
    );
    assert_eq!(
        layout.entries[19].texture.path,
        "textures/entity/chest_boat/spruce.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 1920.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 1920.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 192.0 / 1920.0]);
    assert_close2(layout.entries[14].uv.min, [0.0, 1344.0 / 1920.0]);
    assert_close2(layout.entries[14].uv.max, [1.0, 1408.0 / 1920.0]);

    let acacia_chest_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(
        &rgba[acacia_chest_first_pixel..acacia_chest_first_pixel + 4],
        &[1; 4]
    );
    let oak_first_pixel = rgba_offset(layout.width, 1344, 0, "test").unwrap();
    assert_eq!(&rgba[oak_first_pixel..oak_first_pixel + 4], &[14; 4]);
    let spruce_chest_first_pixel = rgba_offset(layout.width, 1792, 0, "test").unwrap();
    assert_eq!(
        &rgba[spruce_chest_first_pixel..spruce_chest_first_pixel + 4],
        &[19; 4]
    );
}

#[test]
fn boat_textured_mesh_uses_vanilla_uvs_tints_and_root_transform() {
    let images = [
        BOAT_OAK_TEXTURE_REF,
        CHEST_BOAT_OAK_TEXTURE_REF,
        BOAT_BAMBOO_TEXTURE_REF,
        CHEST_BOAT_BAMBOO_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::boat(201, [0.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, false),
            EntityModelInstance::boat(202, [3.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true),
            EntityModelInstance::boat(203, [6.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, false),
            EntityModelInstance::boat(204, [9.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, true),
        ],
        &atlas,
    );

    assert_eq!(atlas.width, 128);
    assert_eq!(atlas.height, 384);
    assert_eq!(mesh.cutout_faces, 216);
    assert_eq!(mesh.vertices.len(), 864);
    assert_eq!(mesh.indices.len(), 1296);
    assert_close2(mesh.vertices[0].uv, [31.0 / 128.0, 0.0]);
    assert_close2(mesh.vertices[216].uv, [31.0 / 128.0, 64.0 / 384.0]);
    assert_close2(mesh.vertices[504].uv, [32.0 / 128.0, 192.0 / 384.0]);
    assert_close2(mesh.vertices[648].uv, [32.0 / 128.0, 256.0 / 384.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert!(max[0] - min[0] > 9.0);
    assert!(max[2] - min[2] > 1.0);
}

#[test]
fn vehicle_and_placeholder_models_emit_sane_bounds() {
    let cases = [
        EntityModelInstance::new(1, EntityModelKind::Minecart, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::new(
            2,
            EntityModelKind::Boat {
                family: BoatModelFamily::Oak,
                chest: true,
            },
            [3.0, 64.0, 0.0],
            0.0,
        ),
        EntityModelInstance::placeholder(
            3,
            [6.0, 64.0, 0.0],
            0.0,
            "todo_test_bounds",
            1.0,
            2.0,
            0.5,
        ),
    ];

    for instance in cases {
        let mesh = entity_model_mesh(&[instance]);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn entity_model_kind_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .model_key(),
        "chicken_temperate"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: false
        }
        .model_key(),
        "pig_cold"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: true
        }
        .model_key(),
        "pig_warm_baby"
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "humanoid_zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ArmorStand {
            small: true,
            show_arms: true,
            show_base_plate: false,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        }
        .model_key(),
        "armor_stand_small"
    );
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.model_key(),
        "zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
        .model_key(),
        "husk"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
        .model_key(),
        "husk_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
        .model_key(),
        "drowned"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
        .model_key(),
        "drowned_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
        .model_key(),
        "zombie_villager"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
        .model_key(),
        "zombie_villager_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
        .model_key(),
        "piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
        .model_key(),
        "piglin_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
        .model_key(),
        "piglin_brute"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
        .model_key(),
        "zombified_piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
        .model_key(),
        "zombified_piglin_baby"
    );
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .model_key(),
        "stray"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .model_key(),
        "parched"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .model_key(),
        "wither_skeleton"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
        .model_key(),
        "bogged"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
        .model_key(),
        "cow_warm"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
        .model_key(),
        "cow_cold_baby"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
        }
        .model_key(),
        "sheep_baby"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.model_key(),
        "villager_baby"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelInstance::wolf(0, [0.0, 0.0, 0.0], 0.0, true)
            .kind
            .model_key(),
        "wolf_baby"
    );
    assert_eq!(
        EntityModelKind::Horse { baby: true }.model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "donkey"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "donkey_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "mule"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "mule_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
        .model_key(),
        "skeleton_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
        .model_key(),
        "skeleton_horse_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
        .model_key(),
        "zombie_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "zombie_horse_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
        .model_key(),
        "camel"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
        .model_key(),
        "camel_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: true
        }
        .model_key(),
        "camel_husk"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: true
        }
        .model_key(),
        "llama_creamy"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::White,
            baby: true,
            has_chest: false
        }
        .model_key(),
        "llama_white_baby"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "trader_llama_brown"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "trader_llama_gray_baby"
    );
    assert_eq!(
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true
        }
        .model_key(),
        "goat_baby"
    );
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
        .model_key(),
        "evoker"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
        .model_key(),
        "illusioner"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
        .model_key(),
        "pillager"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
        .model_key(),
        "vindicator"
    );
    assert_eq!(
        EntityModelKind::Placeholder {
            name: "todo_test_bounds",
            bounds: EntityModelBounds {
                width: 1.0,
                height: 1.0,
                depth: 1.0
            }
        }
        .model_key(),
        "todo_test_bounds"
    );
}

#[test]
fn sanitize_entity_model_instances_drops_non_finite_instances() {
    assert_eq!(
        sanitize_entity_model_instances(vec![
            EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false),
            EntityModelInstance::chicken(2, [0.0, f32::NAN, 0.0], 0.0, false),
            EntityModelInstance::chicken(3, [0.0, 0.0, 0.0], f32::INFINITY, false),
        ]),
        vec![EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false)]
    );
}

#[test]
fn entity_model_vertex_layout_matches_shader_inputs() {
    let layout = entity_model_vertex_layout();

    assert_eq!(
        layout.array_stride,
        std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress
    );
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES.len(), 2);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[0].shader_location, 0);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[1].shader_location, 1);
}

fn mesh_extents(mesh: &EntityModelMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn textured_mesh_extents(mesh: &EntityModelTexturedMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.iter().copied().zip(expected.iter().copied()) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn wolf_texture_images() -> Vec<EntityModelTextureImage> {
    wolf_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn pig_texture_images() -> Vec<EntityModelTextureImage> {
    pig_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn cow_texture_images() -> Vec<EntityModelTextureImage> {
    cow_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn player_texture_images() -> Vec<EntityModelTextureImage> {
    player_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn creeper_texture_images() -> Vec<EntityModelTextureImage> {
    creeper_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn spider_texture_images() -> Vec<EntityModelTextureImage> {
    spider_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_same_geometry(actual: &EntityModelMesh, expected: &EntityModelMesh) {
    assert_eq!(actual.opaque_faces, expected.opaque_faces);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.vertices.len(), expected.vertices.len());
    for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
        assert_eq!(actual.position, expected.position);
    }
}

fn assert_part(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert!(part.children.is_empty());
}

fn assert_part_tree(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
    children: &[ModelPartDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert_eq!(part.children, children);
}
