use super::*;

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
