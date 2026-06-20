use super::*;

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
