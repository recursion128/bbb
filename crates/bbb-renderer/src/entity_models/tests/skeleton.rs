use super::*;

#[test]
fn skeleton_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SKELETON_HAT[0],
        ModelCubeDesc {
            min: [-4.5, -8.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: SKELETON_BONE,
        }
    );
    assert_eq!(SKELETON_PARTS.len(), 6);
    assert_eq!(SKELETON_PARTS[0].pose, PART_POSE_ZERO);
    assert_eq!(SKELETON_PARTS[0].cubes, SKELETON_HEAD.as_slice());
    assert_eq!(
        SKELETON_PARTS[0].children,
        SKELETON_HEAD_CHILDREN.as_slice()
    );
    assert_part(
        &SKELETON_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SKELETON_BODY.as_slice(),
    );
    assert_part(
        &SKELETON_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        SKELETON_ARM.as_slice(),
    );
    assert_part(
        &SKELETON_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        SKELETON_ARM.as_slice(),
    );
    assert_part(
        &SKELETON_PARTS[4],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SKELETON_LEG.as_slice(),
    );
    assert_part(
        &SKELETON_PARTS[5],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        SKELETON_LEG.as_slice(),
    );
}

#[test]
fn skeleton_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::skeleton(115, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.28125]);
    assert_close3(max, [0.375, 66.03225, 0.28125]);
}

#[test]
fn skeleton_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::Skeleton.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/skeleton.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/stray.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/parched.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/wither_skeleton.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: false }
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/skeleton/bogged.png",
            size: [64, 32],
        })
    );
}

#[test]
fn skeleton_variant_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        PARCHED_BODY,
        [
            ModelCubeDesc {
                min: [-4.0, 0.0, -2.0],
                size: [8.0, 12.0, 4.0],
                color: PARCHED_BONE,
            },
            ModelCubeDesc {
                min: [-4.0, 10.0, -2.0],
                size: [8.0, 1.0, 4.0],
                color: PARCHED_BONE,
            },
            ModelCubeDesc {
                min: [-4.025, -0.025, -2.025],
                size: [8.05, 12.05, 4.05],
                color: PARCHED_BONE,
            },
        ]
    );
    assert_eq!(
        PARCHED_HEAD[1],
        ModelCubeDesc {
            min: [-4.2, -8.2, -4.2],
            size: [8.4, 8.4, 8.4],
            color: PARCHED_BONE,
        }
    );

    assert_eq!(PARCHED_PARTS.len(), 6);
    assert_part_tree(
        &PARCHED_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_HEAD.as_slice(),
        PARCHED_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &PARCHED_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_EMPTY_HAT.as_slice(),
    );
    assert_part(
        &PARCHED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_BODY.as_slice(),
    );
    assert_part(
        &PARCHED_PARTS[2],
        [-5.5, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &PARCHED_PARTS[3],
        [5.5, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_LEFT_ARM.as_slice(),
    );
    assert_part(
        &PARCHED_PARTS[4],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_LEG.as_slice(),
    );
    assert_part(
        &PARCHED_PARTS[5],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PARCHED_LEG.as_slice(),
    );

    assert_eq!(
        BOGGED_RED_MUSHROOM_PLANE[0],
        ModelCubeDesc {
            min: [-3.0, -3.0, 0.0],
            size: [6.0, 4.0, 0.0],
            color: BOGGED_RED_MUSHROOM_COLOR,
        }
    );
    assert_eq!(BOGGED_PARTS.len(), 6);
    assert_part_tree(
        &BOGGED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        BOGGED_HEAD.as_slice(),
        BOGGED_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BOGGED_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        BOGGED_HAT.as_slice(),
    );
    assert_part_tree(
        &BOGGED_HEAD_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BOGGED_MUSHROOM_CHILDREN.as_slice(),
    );
    assert_part(
        &BOGGED_MUSHROOM_CHILDREN[0],
        [3.0, -8.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        BOGGED_RED_MUSHROOM_PLANE.as_slice(),
    );
    assert_part(
        &BOGGED_MUSHROOM_CHILDREN[1],
        [3.0, -8.0, 3.0],
        [0.0, std::f32::consts::FRAC_PI_4 * 3.0, 0.0],
        BOGGED_RED_MUSHROOM_PLANE.as_slice(),
    );
    assert_part(
        &BOGGED_MUSHROOM_CHILDREN[2],
        [-3.0, -8.0, -3.0],
        [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        BOGGED_BROWN_MUSHROOM_PLANE.as_slice(),
    );
    assert_part(
        &BOGGED_MUSHROOM_CHILDREN[5],
        [-2.0, -1.0, 4.0],
        [
            -std::f32::consts::FRAC_PI_2,
            0.0,
            std::f32::consts::FRAC_PI_4 * 3.0,
        ],
        BOGGED_BROWN_TOP_MUSHROOM_PLANE.as_slice(),
    );
    assert_part_tree(
        &BOGGED_SHEARED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        BOGGED_HEAD.as_slice(),
        BOGGED_HAT_CHILDREN.as_slice(),
    );
}

#[test]
fn skeleton_variant_meshes_use_vanilla_body_layer_geometry() {
    let skeleton = entity_model_mesh(&[EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0)]);
    let stray = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        128,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Stray,
    )]);
    assert_eq!(stray.vertices, skeleton.vertices);
    assert_eq!(stray.indices, skeleton.indices);

    let wither = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        146,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::WitherSkeleton,
    )]);
    assert_eq!(wither.opaque_faces, 42);
    assert_eq!(wither.vertices.len(), 168);
    assert_eq!(wither.indices.len(), 252);
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_SKELETON_DARK, 0.78)));
    let (wither_min, wither_max) = mesh_extents(&wither);
    assert_close3(wither_min, [-0.45000002, 64.0012, -0.33750004]);
    assert_close3(wither_max, [0.45000002, 66.4387, 0.33750004]);

    let parched = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        97,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    )]);
    assert_eq!(parched.opaque_faces, 78);
    assert_eq!(parched.vertices.len(), 312);
    assert_eq!(parched.indices.len(), 468);
    let (parched_min, parched_max) = mesh_extents(&parched);
    assert_close3(parched_min, [-0.440625, 64.001, -0.26250002]);
    assert_close3(parched_max, [0.440625, 66.0135, 0.26250002]);

    let bogged = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        16,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: false },
    )]);
    assert_eq!(bogged.opaque_faces, 78);
    assert_eq!(bogged.vertices.len(), 312);
    assert_eq!(bogged.indices.len(), 468);
    assert!(bogged
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BOGGED_RED_MUSHROOM_COLOR, 0.78)));
    let (bogged_min, bogged_max) = mesh_extents(&bogged);
    assert_close3(bogged_min, [-0.375, 64.001, -0.5]);
    assert_close3(bogged_max, [0.375, 66.1885, 0.32008255]);

    let sheared_bogged = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
        17,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: true },
    )]);
    assert_eq!(sheared_bogged.opaque_faces, 42);
    assert_eq!(sheared_bogged.vertices.len(), 168);
    assert_eq!(sheared_bogged.indices.len(), 252);
    assert_same_geometry(&sheared_bogged, &skeleton);
}
