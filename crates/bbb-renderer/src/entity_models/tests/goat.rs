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

    assert_eq!(
        goat_entity_texture_refs(),
        &[
            EntityModelTextureRef {
                path: "textures/entity/goat/goat.png",
                size: [64, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/goat/goat_baby.png",
                size: [64, 64],
            },
        ]
    );
    assert!(entity_model_texture_refs().contains(&GOAT_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GOAT_BABY_TEXTURE_REF));
}

#[test]
fn goat_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = goat_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::GoatBase);
    assert_eq!(adult[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(adult[0].model_layer, MODEL_LAYER_GOAT);
    assert_eq!(adult[0].texture, GOAT_TEXTURE_REF);
    assert_eq!(adult[0].parts, ADULT_GOAT_TEXTURED_PARTS.as_slice());
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].collector_order, adult[0].submit_sequence), (0, 0));

    let baby = goat_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::GoatBase);
    assert_eq!(baby[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_GOAT_BABY);
    assert_eq!(baby[0].texture, GOAT_BABY_TEXTURE_REF);
    assert_eq!(baby[0].parts, BABY_GOAT_TEXTURED_PARTS.as_slice());
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].collector_order, baby[0].submit_sequence), (0, 0));
}

#[test]
fn goat_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_GOAT, "minecraft:goat#main");
    assert_eq!(MODEL_LAYER_GOAT_BABY, "minecraft:goat_baby#main");
    assert_eq!(
        ADULT_GOAT_TEXTURED_HEAD[1],
        TexturedModelCubeDesc {
            min: [2.0, -11.0, -10.0],
            size: [3.0, 2.0, 1.0],
            uv_size: [3.0, 2.0, 1.0],
            tex: [2.0, 61.0],
            mirror: true,
        }
    );
    assert_eq!(
        ADULT_GOAT_TEXTURED_HEAD[2],
        TexturedModelCubeDesc {
            min: [-0.5, -3.0, -14.0],
            size: [0.0, 7.0, 5.0],
            uv_size: [0.0, 7.0, 5.0],
            tex: [23.0, 52.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_GOAT_TEXTURED_LEFT_HORN[0].tex, [12.0, 55.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_RIGHT_HORN[0].tex, [12.0, 55.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_NOSE[0].tex, [34.0, 46.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_LEFT_HIND_LEG[0].tex, [36.0, 29.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_RIGHT_HIND_LEG[0].tex, [49.0, 29.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_LEFT_FRONT_LEG[0].tex, [49.0, 2.0]);
    assert_eq!(ADULT_GOAT_TEXTURED_RIGHT_FRONT_LEG[0].tex, [35.0, 2.0]);
    assert_eq!(
        ADULT_GOAT_TEXTURED_PARTS[ADULT_GOAT_HEAD_INDEX].children,
        ADULT_GOAT_TEXTURED_HEAD_CHILDREN.as_slice()
    );

    assert_eq!(BABY_GOAT_TEXTURED_LEFT_HIND_LEG[0].tex, [29.0, 12.0]);
    assert_eq!(BABY_GOAT_TEXTURED_RIGHT_HIND_LEG[0].tex, [21.0, 12.0]);
    assert_eq!(BABY_GOAT_TEXTURED_RIGHT_FRONT_LEG[0].tex, [21.0, 5.0]);
    assert_eq!(BABY_GOAT_TEXTURED_LEFT_FRONT_LEG[0].tex, [29.0, 5.0]);
    assert_eq!(BABY_GOAT_TEXTURED_RIGHT_HORN[0].tex, [24.0, 0.0]);
    assert!(BABY_GOAT_TEXTURED_RIGHT_HORN[0].mirror);
    assert!(BABY_GOAT_TEXTURED_LEFT_HORN[0].mirror);
    assert_eq!(BABY_GOAT_TEXTURED_RIGHT_EAR[0].tex, [0.0, 12.0]);
    assert!(BABY_GOAT_TEXTURED_RIGHT_EAR[0].mirror);
    assert!(!BABY_GOAT_TEXTURED_LEFT_EAR[0].mirror);
    assert_eq!(BABY_GOAT_TEXTURED_HEAD_MAIN[0].tex, [0.0, 0.0]);
    assert_eq!(
        BABY_GOAT_TEXTURED_PARTS[BABY_GOAT_HEAD_INDEX].children,
        BABY_GOAT_TEXTURED_HEAD_CHILDREN.as_slice()
    );
}

#[test]
fn entity_texture_atlas_stitches_official_goat_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/goat/goat.png",
            "textures/entity/goat/goat_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "goat baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
}

#[test]
fn goat_textured_mesh_uses_vanilla_uvs_tints_and_horn_visibility() {
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    let adult = EntityModelInstance::goat(401, [0.0, 64.0, 0.0], 0.0, false, true, true);
    let adult_mesh = entity_model_textured_mesh(&[adult], &atlas);
    assert_eq!(adult_mesh.cutout_faces, 72);
    assert_eq!(adult_mesh.vertices.len(), 288);
    assert_eq!(adult_mesh.indices.len(), 432);
    assert_close2(adult_mesh.vertices[0].uv, [6.0 / 64.0, 61.0 / 128.0]);
    assert_eq!(adult_mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    let (adult_textured_min, adult_textured_max) = textured_mesh_extents(&adult_mesh);
    let (adult_colored_min, adult_colored_max) = mesh_extents(&entity_model_mesh(&[adult]));
    assert_close3(adult_textured_min, adult_colored_min);
    assert_close3(adult_textured_max, adult_colored_max);

    let adult_left_horn_only = entity_model_textured_mesh(
        &[EntityModelInstance::goat(
            402,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
        )],
        &atlas,
    );
    assert_eq!(adult_left_horn_only.cutout_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);

    let adult_no_horns = entity_model_textured_mesh(
        &[EntityModelInstance::goat(
            403,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
        )],
        &atlas,
    );
    assert_eq!(adult_no_horns.cutout_faces, 60);
    assert_eq!(adult_no_horns.vertices.len(), 240);

    let baby = EntityModelInstance::goat(404, [0.0, 64.0, 0.0], 0.0, true, true, true);
    let baby_mesh = entity_model_textured_mesh(&[baby], &atlas);
    assert_eq!(baby_mesh.cutout_faces, 72);
    assert_close2(baby_mesh.vertices[0].uv, [33.0 / 64.0, 76.0 / 128.0]);
    let (baby_textured_min, baby_textured_max) = textured_mesh_extents(&baby_mesh);
    let (baby_colored_min, baby_colored_max) = mesh_extents(&entity_model_mesh(&[baby]));
    assert_close3(baby_textured_min, baby_colored_min);
    assert_close3(baby_textured_max, baby_colored_max);

    let baby_no_horns = entity_model_textured_mesh(
        &[EntityModelInstance::goat(
            405,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
            false,
        )],
        &atlas,
    );
    assert_eq!(baby_no_horns.cutout_faces, 60);
    assert!(baby_no_horns
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn goat_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    for base in [
        EntityModelInstance::goat(440, [0.0, 64.0, 0.0], 0.0, false, true, true),
        EntityModelInstance::goat(441, [0.0, 64.0, 0.0], 0.0, true, true, true),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

#[test]
fn goat_swings_its_legs_when_walking() {
    // Vanilla `GoatModel extends QuadrupedModel`: `setupAnim` runs `super.setupAnim`
    // (the diagonal `QuadrupedModel` leg swing) before the horn visibility and the
    // ramming head tilt, so the four legs swing. A standing goat is inert; a walking
    // adult lifts its feet and splays its legs along Z; the baby's short legs swing
    // too but the motion stays inside its bounding box, so only the adult asserts the
    // extent change. The ramming head tilt is deferred. Colored path.
    for (name, base, adult_size) in [
        (
            "goat_adult",
            EntityModelInstance::goat(450, [0.0, 64.0, 0.0], 0.0, false, true, true),
            true,
        ),
        (
            "goat_baby",
            EntityModelInstance::goat(451, [0.0, 64.0, 0.0], 0.0, true, true, true),
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        if adult_size {
            let (rest_min, rest_max) = mesh_extents(&rest);
            let (walk_min, walk_max) = mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking goat's feet should lift off the ground"
            );
            assert!(
                (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
                "{name}: a walking goat's legs should splay along Z"
            );
        }
    }
}

#[test]
fn goat_textured_mesh_swings_legs_when_walking() {
    // The real goat render path (texture-backed) swings the same `QuadrupedModel`
    // legs on the shared visibility-filtered part array. A standing goat is
    // byte-identical however far the swing position has advanced; a walking adult
    // lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&goat_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "goat_adult",
            EntityModelInstance::goat(452, [0.0, 64.0, 0.0], 0.0, false, true, true),
            true,
        ),
        (
            "goat_baby",
            EntityModelInstance::goat(453, [0.0, 64.0, 0.0], 0.0, true, true, true),
            false,
        ),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing goat is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking goat differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking goat's feet should lift off the ground"
            );
        }
    }
}

fn goat_texture_images() -> Vec<EntityModelTextureImage> {
    goat_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
