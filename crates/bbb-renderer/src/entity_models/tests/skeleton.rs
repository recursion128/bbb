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
fn skeleton_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let base = skeleton_textured_layer_passes(None);
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].kind, EntityModelLayerKind::SkeletonBase);
    assert_eq!(base[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(base[0].model_layer, MODEL_LAYER_SKELETON);
    assert_eq!(base[0].texture, SKELETON_TEXTURE_REF);
    assert_eq!(base[0].parts, SKELETON_TEXTURED_PARTS.as_slice());
    assert_eq!(base[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(base[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((base[0].collector_order, base[0].submit_sequence), (0, 0));

    let stray = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Stray));
    assert_eq!(stray.len(), 1);
    assert_eq!(stray[0].model_layer, MODEL_LAYER_STRAY);
    assert_eq!(stray[0].texture, STRAY_TEXTURE_REF);
    assert_eq!(stray[0].parts, SKELETON_TEXTURED_PARTS.as_slice());

    let parched = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Parched));
    assert_eq!(parched.len(), 1);
    assert_eq!(parched[0].model_layer, MODEL_LAYER_PARCHED);
    assert_eq!(parched[0].texture, PARCHED_TEXTURE_REF);
    assert_eq!(parched[0].parts, PARCHED_TEXTURED_PARTS.as_slice());

    let wither = skeleton_textured_layer_passes(Some(SkeletonModelFamily::WitherSkeleton));
    assert_eq!(wither.len(), 1);
    assert_eq!(wither[0].model_layer, MODEL_LAYER_WITHER_SKELETON);
    assert_eq!(wither[0].texture, WITHER_SKELETON_TEXTURE_REF);
    assert_eq!(wither[0].parts, SKELETON_TEXTURED_PARTS.as_slice());

    let bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: false }));
    assert_eq!(bogged.len(), 1);
    assert_eq!(bogged[0].model_layer, MODEL_LAYER_BOGGED);
    assert_eq!(bogged[0].texture, BOGGED_TEXTURE_REF);
    assert_eq!(bogged[0].parts, BOGGED_TEXTURED_PARTS.as_slice());

    let sheared_bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: true }));
    assert_eq!(sheared_bogged.len(), 1);
    assert_eq!(
        sheared_bogged[0].parts,
        BOGGED_SHEARED_TEXTURED_PARTS.as_slice()
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
fn skeleton_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SKELETON, "minecraft:skeleton#main");
    assert_eq!(MODEL_LAYER_STRAY, "minecraft:stray#main");
    assert_eq!(MODEL_LAYER_PARCHED, "minecraft:parched#main");
    assert_eq!(
        MODEL_LAYER_WITHER_SKELETON,
        "minecraft:wither_skeleton#main"
    );
    assert_eq!(MODEL_LAYER_BOGGED, "minecraft:bogged#main");
    assert_eq!(
        SKELETON_TEXTURED_RIGHT_ARM[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 12.0, 2.0],
            uv_size: [2.0, 12.0, 2.0],
            tex: [40.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(SKELETON_TEXTURED_LEFT_ARM[0].tex, [40.0, 16.0]);
    assert!(SKELETON_TEXTURED_LEFT_ARM[0].mirror);
    assert_eq!(SKELETON_TEXTURED_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert!(SKELETON_TEXTURED_LEFT_LEG[0].mirror);
    assert_eq!(SKELETON_TEXTURED_PARTS[0].pose, SKELETON_PARTS[0].pose);
    assert_eq!(
        SKELETON_TEXTURED_PARTS[0].children,
        SKELETON_TEXTURED_HEAD_CHILDREN.as_slice()
    );

    assert_eq!(PARCHED_TEXTURED_BODY[1].tex, [28.0, 0.0]);
    assert_eq!(PARCHED_TEXTURED_BODY[2].tex, [16.0, 48.0]);
    assert_eq!(PARCHED_TEXTURED_BODY[2].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(PARCHED_TEXTURED_HEAD[1].tex, [0.0, 32.0]);
    assert_eq!(PARCHED_TEXTURED_RIGHT_ARM[1].tex, [42.0, 33.0]);
    assert_eq!(PARCHED_TEXTURED_LEFT_ARM[0].tex, [56.0, 16.0]);
    assert_eq!(PARCHED_TEXTURED_LEFT_ARM[1].tex, [40.0, 48.0]);
    assert_eq!(PARCHED_TEXTURED_RIGHT_LEG[1].tex, [0.0, 49.0]);
    assert_eq!(PARCHED_TEXTURED_LEFT_LEG[1].tex, [4.0, 49.0]);

    assert_eq!(BOGGED_TEXTURED_RED_MUSHROOM_PLANE[0].tex, [50.0, 16.0]);
    assert_eq!(BOGGED_TEXTURED_BROWN_MUSHROOM_PLANE[0].tex, [50.0, 22.0]);
    assert_eq!(
        BOGGED_TEXTURED_BROWN_TOP_MUSHROOM_PLANE[0].tex,
        [50.0, 28.0]
    );
    assert_eq!(
        BOGGED_TEXTURED_PARTS[0].children,
        BOGGED_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(
        BOGGED_SHEARED_TEXTURED_PARTS[0].children,
        BOGGED_TEXTURED_HAT_CHILDREN.as_slice()
    );
}

#[test]
fn entity_texture_atlas_stitches_official_skeleton_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 192);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/skeleton/skeleton.png",
            "textures/entity/skeleton/stray.png",
            "textures/entity/skeleton/parched.png",
            "textures/entity/skeleton/wither_skeleton.png",
            "textures/entity/skeleton/bogged.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 32.0 / 192.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 64.0 / 192.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 128.0 / 192.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 160.0 / 192.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 1.0]);
    assert!(entity_model_texture_refs().contains(&SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRAY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&PARCHED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BOGGED_TEXTURE_REF));
    let parched_first_pixel = rgba_offset(layout.width, 64, 0, "parched atlas row").unwrap();
    assert_eq!(&rgba[parched_first_pixel..parched_first_pixel + 4], &[2; 4]);
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

#[test]
fn skeleton_textured_mesh_uses_vanilla_uvs_tints_and_variant_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    let skeleton = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(skeleton.cutout_faces, 42);
    assert_eq!(skeleton.vertices.len(), 168);
    assert_eq!(skeleton.indices.len(), 252);
    assert_close2(skeleton.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(skeleton
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&skeleton);
    assert_close3(min, [-0.375, 64.001, -0.28125]);
    assert_close3(max, [0.375, 66.03225, 0.28125]);

    let stray = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            128,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Stray,
        )],
        &atlas,
    );
    assert_eq!(stray.vertices.len(), 168);
    assert_close2(stray.vertices[0].uv, [16.0 / 64.0, 32.0 / 192.0]);
    assert_eq!(
        textured_mesh_extents(&stray),
        textured_mesh_extents(&skeleton)
    );

    let wither = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            146,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::WitherSkeleton,
        )],
        &atlas,
    );
    assert_eq!(wither.cutout_faces, 42);
    assert_close2(wither.vertices[0].uv, [16.0 / 64.0, 128.0 / 192.0]);
    let (wither_min, wither_max) = textured_mesh_extents(&wither);
    assert_close3(wither_min, [-0.45000002, 64.0012, -0.33750004]);
    assert_close3(wither_max, [0.45000002, 66.4387, 0.33750004]);

    let parched = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            97,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Parched,
        )],
        &atlas,
    );
    assert_eq!(parched.cutout_faces, 78);
    assert_eq!(parched.vertices.len(), 312);
    assert_close2(parched.vertices[0].uv, [28.0 / 64.0, 80.0 / 192.0]);
    let (parched_min, parched_max) = textured_mesh_extents(&parched);
    assert_close3(parched_min, [-0.440625, 64.001, -0.26250002]);
    assert_close3(parched_max, [0.440625, 66.0135, 0.26250002]);

    let bogged = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            16,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: false },
        )],
        &atlas,
    );
    assert_eq!(bogged.cutout_faces, 78);
    assert_eq!(bogged.vertices.len(), 312);
    assert_close2(bogged.vertices[48].uv, [56.0 / 64.0, 176.0 / 192.0]);
    let (bogged_min, bogged_max) = textured_mesh_extents(&bogged);
    assert_close3(bogged_min, [-0.375, 64.001, -0.5]);
    assert_close3(bogged_max, [0.375, 66.1885, 0.32008255]);

    let sheared_bogged = entity_model_textured_mesh(
        &[EntityModelInstance::skeleton_variant(
            17,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: true },
        )],
        &atlas,
    );
    assert_eq!(sheared_bogged.cutout_faces, 42);
    assert_eq!(sheared_bogged.vertices.len(), 168);
    assert_eq!(
        textured_mesh_extents(&sheared_bogged),
        textured_mesh_extents(&skeleton)
    );
}

fn skeleton_texture_images() -> Vec<EntityModelTextureImage> {
    skeleton_entity_texture_refs()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, texture)| EntityModelTextureImage {
            texture,
            rgba: vec![
                u8::try_from(index).unwrap();
                usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap()
            ],
        })
        .collect()
}
