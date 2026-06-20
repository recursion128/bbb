use super::*;

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
