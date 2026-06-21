use super::*;

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
    assert_eq!(
        ravager_entity_texture_refs(),
        [EntityModelTextureRef {
            path: "textures/entity/illager/ravager.png",
            size: [128, 128],
        }]
    );
    assert!(entity_model_texture_refs().contains(&RAVAGER_TEXTURE_REF));
}

#[test]
fn ravager_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = ravager_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::RavagerBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_RAVAGER);
    assert_eq!(passes[0].texture, RAVAGER_TEXTURE_REF);
    assert_eq!(passes[0].parts, RAVAGER_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn ravager_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_RAVAGER, "minecraft:ravager#main");
    assert_eq!(RAVAGER_TEXTURE_REF.size, [128, 128]);
    assert_eq!(RAVAGER_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        RAVAGER_TEXTURED_NECK[0],
        TexturedModelCubeDesc {
            min: [-5.0, -1.0, -18.0],
            size: [10.0, 10.0, 18.0],
            uv_size: [10.0, 10.0, 18.0],
            tex: [68.0, 73.0],
            mirror: false,
        }
    );
    assert_eq!(
        RAVAGER_TEXTURED_HEAD,
        [
            TexturedModelCubeDesc {
                min: [-8.0, -20.0, -14.0],
                size: [16.0, 20.0, 16.0],
                uv_size: [16.0, 20.0, 16.0],
                tex: [0.0, 0.0],
                mirror: false,
            },
            TexturedModelCubeDesc {
                min: [-2.0, -6.0, -18.0],
                size: [4.0, 8.0, 4.0],
                uv_size: [4.0, 8.0, 4.0],
                tex: [0.0, 0.0],
                mirror: false,
            },
        ]
    );
    assert_eq!(
        RAVAGER_TEXTURED_LEFT_HORN[0],
        TexturedModelCubeDesc {
            min: [0.0, -14.0, -2.0],
            size: [2.0, 14.0, 4.0],
            uv_size: [2.0, 14.0, 4.0],
            tex: [74.0, 55.0],
            mirror: true,
        }
    );
    assert_eq!(
        RAVAGER_TEXTURED_BODY,
        [
            TexturedModelCubeDesc {
                min: [-7.0, -10.0, -7.0],
                size: [14.0, 16.0, 20.0],
                uv_size: [14.0, 16.0, 20.0],
                tex: [0.0, 55.0],
                mirror: false,
            },
            TexturedModelCubeDesc {
                min: [-6.0, 6.0, -7.0],
                size: [12.0, 13.0, 18.0],
                uv_size: [12.0, 13.0, 18.0],
                tex: [0.0, 91.0],
                mirror: false,
            },
        ]
    );
    assert_eq!(RAVAGER_TEXTURED_PARTS[0].pose, RAVAGER_PARTS[0].pose);
    assert_eq!(
        RAVAGER_TEXTURED_PARTS[0].children,
        RAVAGER_TEXTURED_NECK_CHILDREN.as_slice()
    );
    assert_eq!(
        RAVAGER_TEXTURED_NECK_CHILDREN[0].children,
        RAVAGER_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(RAVAGER_TEXTURED_PARTS[5].pose, RAVAGER_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_ravager_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 128);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/illager/ravager.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn ravager_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let instance = EntityModelInstance::ravager(109, [0.0, 64.0, 0.0], 0.0);
    let mesh = entity_model_textured_mesh(&[instance], &atlas);

    assert_eq!(mesh.cutout_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);
    assert_close2(mesh.vertices[0].uv, [96.0 / 128.0, 73.0 / 128.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let colored = entity_model_mesh(&[instance]);
    let (expected_min, expected_max) = mesh_extents(&colored);
    let (actual_min, actual_max) = textured_mesh_extents(&mesh);
    assert_close3(actual_min, expected_min);
    assert_close3(actual_max, expected_max);
}

#[test]
fn ravager_textured_mesh_turns_nested_head_not_neck_or_body() {
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let base = EntityModelInstance::ravager(110, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(50.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);

    // Emit order matches the colored path: neck cube (verts 0..24), head + horn/
    // mouth children (24..144), then body and legs (144..). The vanilla look turns
    // the nested head only; the neck cube and the body/legs stay put.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_eq!(resting.vertices[0..24], yawed.vertices[0..24]);
    assert_ne!(resting.vertices[24..144], yawed.vertices[24..144]);
    assert_ne!(yawed.vertices[24..144], pitched.vertices[24..144]);
    assert_eq!(resting.vertices[144..], yawed.vertices[144..]);
}

#[test]
fn ravager_swings_its_legs_when_walking() {
    // Vanilla `RavagerModel.setupAnim` swings the four legs `cos(pos * 0.6662 [+ π]) *
    // 0.4 * speed` (the `QuadrupedModel` phase with a shorter 0.4 amplitude, legs at
    // [2, 3, 4, 5]). A standing ravager is inert; a walking one lifts its feet and
    // splays its legs along Z. The neck/mouth attack/stun/roar poses are deferred.
    let base = EntityModelInstance::ravager(280, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    // The 0.4 amplitude is small, so the overall height barely changes; assert the two
    // direct signals instead: the lowest point (the feet) rises off the ground, and the
    // legs splay along Z (hind legs back, front legs forward).
    assert!(
        walk_min[1] > rest_min[1] + 0.02,
        "a walking ravager's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking ravager's legs should splay along Z"
    );
}

#[test]
fn ravager_textured_mesh_swings_legs_not_neck_or_head() {
    // The real ravager render path (texture-backed) swings the same legs while the
    // nested-head neck handling is unaffected. Emit order: neck cube 0..24, head +
    // children 24..144, body and legs 144.. . A walking ravager (head at rest) changes
    // only the body/leg region; the neck and head stay put. A standing ravager is
    // byte-identical however far the swing has advanced.
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let base = EntityModelInstance::ravager(281, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing ravager is inert"
    );
    assert_eq!(
        resting.vertices.len(),
        walking.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_eq!(
        resting.vertices[0..144],
        walking.vertices[0..144],
        "the neck and head stay put while walking"
    );
    assert_ne!(
        resting.vertices[144..],
        walking.vertices[144..],
        "the body/leg region swings"
    );
}

#[test]
fn ravager_leg_swing_pose_matches_vanilla_formula() {
    // Vanilla RavagerModel.setupAnim: legRot = 0.4 * walkAnimationSpeed;
    // rightHindLeg.xRot = cos(pos * 0.6662) * legRot;
    // leftHindLeg.xRot  = cos(pos * 0.6662 + π) * legRot;
    // rightFrontLeg.xRot = cos(pos * 0.6662 + π) * legRot;
    // leftFrontLeg.xRot  = cos(pos * 0.6662) * legRot.
    // This is the QuadrupedModel diagonal phase (in phase when x*z < 0) but with a 0.4
    // amplitude rather than the usual 1.4. RAVAGER_PARTS lists right hind at index 2
    // (x = -8, z = 18 -> x*z < 0 -> in phase) and left hind at index 3 (x = 8 -> out of
    // phase); front legs are at z = -5 so their phases flip.
    let right_hind = ravager_leg_swing_pose(RAVAGER_PARTS[2].pose, 0.0, 1.0);
    let left_hind = ravager_leg_swing_pose(RAVAGER_PARTS[3].pose, 0.0, 1.0);
    let right_front = ravager_leg_swing_pose(RAVAGER_PARTS[4].pose, 0.0, 1.0);
    let left_front = ravager_leg_swing_pose(RAVAGER_PARTS[5].pose, 0.0, 1.0);
    assert!(
        (right_hind.rotation[0] - 0.4).abs() < 1e-6,
        "right hind in phase at amplitude 0.4: {}",
        right_hind.rotation[0]
    );
    assert!(
        (left_hind.rotation[0] + 0.4).abs() < 1e-6,
        "left hind out of phase at amplitude 0.4: {}",
        left_hind.rotation[0]
    );
    // Diagonal pairs: right front matches left hind, left front matches right hind.
    assert!((right_front.rotation[0] - left_hind.rotation[0]).abs() < 1e-6);
    assert!((left_front.rotation[0] - right_hind.rotation[0]).abs() < 1e-6);

    // A general (pos, speed) reproduces cos(pos * 0.6662 [+ π]) * 0.4 * speed.
    let right_hind = ravager_leg_swing_pose(RAVAGER_PARTS[2].pose, 1.5, 0.5);
    let left_hind = ravager_leg_swing_pose(RAVAGER_PARTS[3].pose, 1.5, 0.5);
    assert!((right_hind.rotation[0] - (1.5_f32 * 0.6662).cos() * 0.4 * 0.5).abs() < 1e-6);
    assert!(
        (left_hind.rotation[0] - (1.5_f32 * 0.6662 + std::f32::consts::PI).cos() * 0.4 * 0.5).abs()
            < 1e-6
    );

    // The swing only sets xRot; the resting offset and yRot/zRot are preserved.
    assert_eq!(right_hind.offset, RAVAGER_PARTS[2].pose.offset);
    assert_eq!(right_hind.rotation[1], RAVAGER_PARTS[2].pose.rotation[1]);
    assert_eq!(right_hind.rotation[2], RAVAGER_PARTS[2].pose.rotation[2]);
}

fn ravager_texture_images() -> Vec<EntityModelTextureImage> {
    ravager_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
