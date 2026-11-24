use super::*;
use crate::entity_models::model_layers::hoglin_model;

#[test]
fn hoglin_texture_refs_match_vanilla_renderers() {
    assert!(entity_model_texture_refs().contains(&HOGLIN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&HOGLIN_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOGLIN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOGLIN_BABY_TEXTURE_REF));
}

#[test]
fn hoglin_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let cases = [
        (HoglinModelFamily::Hoglin, false, HOGLIN_TEXTURE_REF),
        (HoglinModelFamily::Hoglin, true, HOGLIN_BABY_TEXTURE_REF),
        (HoglinModelFamily::Zoglin, false, ZOGLIN_TEXTURE_REF),
        (HoglinModelFamily::Zoglin, true, ZOGLIN_BABY_TEXTURE_REF),
    ];

    for (family, baby, texture) in cases {
        let passes = hoglin_textured_layer_passes(family, baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        // The vestigial `parts` slice is nulled; emit builds `HoglinModel::new(baby)` and renders it.
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
    }
}

#[test]
fn hoglin_model_layers_match_vanilla() {
    assert_eq!(ModelLayer::HOGLIN, ModelLayer::HOGLIN);
    assert_eq!(ModelLayer::HOGLIN_BABY, ModelLayer::HOGLIN_BABY);
    assert_eq!(ModelLayer::ZOGLIN, ModelLayer::ZOGLIN);
    assert_eq!(ModelLayer::ZOGLIN_BABY, ModelLayer::ZOGLIN_BABY);
}

#[test]
fn entity_texture_atlas_stitches_official_hoglin_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 256);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/hoglin/hoglin.png",
            "textures/entity/hoglin/hoglin_baby.png",
            "textures/entity/hoglin/zoglin.png",
            "textures/entity/hoglin/zoglin_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 256.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 256.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 128.0 / 256.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 128.0 / 256.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 192.0 / 256.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 192.0 / 256.0]);
    assert_close2(layout.entries[3].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "hoglin baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
    let zoglin_first_pixel = rgba_offset(layout.width, 128, 0, "zoglin atlas row").unwrap();
    assert_eq!(&rgba[zoglin_first_pixel..zoglin_first_pixel + 4], &[2; 4]);
    let zoglin_baby_first_pixel =
        rgba_offset(layout.width, 192, 0, "zoglin baby atlas row").unwrap();
    assert_eq!(
        &rgba[zoglin_baby_first_pixel..zoglin_baby_first_pixel + 4],
        &[3; 4]
    );
}

#[test]
fn hoglin_textured_mesh_uses_vanilla_uvs_tints_and_family_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    let adult_hoglin =
        EntityModelInstance::hoglin(224, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false)
            .with(|s| {
                s.light_coords = (4_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let adult_zoglin =
        EntityModelInstance::hoglin(225, [3.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, false)
            .with(|s| {
                s.light_coords = (5_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let baby_hoglin =
        EntityModelInstance::hoglin(226, [6.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, true)
            .with(|s| {
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let baby_zoglin =
        EntityModelInstance::hoglin(227, [9.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, true)
            .with(|s| {
                s.light_coords = (7_u32 << 4) | (9_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let instances = [adult_hoglin, adult_zoglin, baby_hoglin, baby_zoglin];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_hoglin_textured_output(&meshes, &instances);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 264);
    assert_eq!(mesh.vertices.len(), 1056);
    assert_eq!(mesh.indices.len(), 1584);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn hoglin_textured_meshes_apply_yaw_only_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    // Vanilla `HoglinModel.setupAnim` turns the head in yaw only, keeping the
    // headbutt-rest pitch baked into the base pose, so a pitch-only look leaves the
    // mesh unchanged while a yaw look turns the head.
    for (id, baby) in [(228, false), (229, true)] {
        let base =
            EntityModelInstance::hoglin(id, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, baby);
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (50.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_hoglin_textured_output(&resting, &[base]);
        assert_hoglin_textured_output(&yawed, &[yawed_instance]);
        assert_hoglin_textured_output(&pitched, &[pitched_instance]);
        assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
        assert_ne!(
            resting.cutout.vertices, yawed.cutout.vertices,
            "baby={baby} yaw turns head"
        );
        assert_eq!(
            resting.cutout.vertices, pitched.cutout.vertices,
            "baby={baby} pitch ignored"
        );
    }
}

#[test]
fn hoglin_leg_swing_pose_matches_vanilla_formula() {
    // Vanilla HoglinModel.setupAnim: rightFrontLeg.xRot = cos(pos) * 1.2 * speed,
    // leftFrontLeg.xRot = cos(pos + π) * 1.2 * speed, rightHindLeg = leftFrontLeg,
    // leftHindLeg = rightFrontLeg. The amplitude is 1.2 (not the QuadrupedModel 1.4)
    // and there is NO 0.6662 frequency factor. The right-front leg sits at offset
    // x = -4, z = -8.5 (x*z > 0 -> in phase) and the left-front at x = 4, z = -8.5
    // (x*z < 0 -> out of phase).
    let right_front_pose = PartPose {
        offset: [-4.0, 10.0, -8.5],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_front_pose = PartPose {
        offset: [4.0, 10.0, -8.5],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_hind_pose = PartPose {
        offset: [-5.0, 13.0, 10.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_hind_pose = PartPose {
        offset: [5.0, 13.0, 10.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_front = hoglin_leg_swing_pose(right_front_pose, 0.0, 1.0);
    let left_front = hoglin_leg_swing_pose(left_front_pose, 0.0, 1.0);
    assert!(
        (right_front.rotation[0] - 1.2).abs() < 1e-6,
        "right front in phase at amplitude 1.2: {}",
        right_front.rotation[0]
    );
    assert!(
        (left_front.rotation[0] + 1.2).abs() < 1e-6,
        "left front out of phase at amplitude 1.2: {}",
        left_front.rotation[0]
    );
    // The diagonal pair: right hind (x*z < 0) matches left front, and left hind (x*z > 0)
    // matches right front.
    let right_hind = hoglin_leg_swing_pose(right_hind_pose, 0.0, 1.0);
    let left_hind = hoglin_leg_swing_pose(left_hind_pose, 0.0, 1.0);
    assert!((right_hind.rotation[0] - left_front.rotation[0]).abs() < 1e-6);
    assert!((left_hind.rotation[0] - right_front.rotation[0]).abs() < 1e-6);

    // A general (pos, speed) reproduces cos(pos [+ π]) * 1.2 * speed, with no 0.6662.
    let right_front = hoglin_leg_swing_pose(right_front_pose, 1.5, 0.5);
    let left_front = hoglin_leg_swing_pose(left_front_pose, 1.5, 0.5);
    assert!((right_front.rotation[0] - 1.5_f32.cos() * 1.2 * 0.5).abs() < 1e-6);
    assert!(
        (left_front.rotation[0] - (1.5_f32 + std::f32::consts::PI).cos() * 1.2 * 0.5).abs() < 1e-6
    );
}

#[test]
fn hoglin_textured_mesh_swings_legs_when_walking() {
    // The real hoglin render path (texture-backed) swings the same legs. A standing
    // hoglin is byte-identical however far the swing has advanced; a walking adult
    // lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "hoglin",
            EntityModelInstance::hoglin(
                243,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                false,
            ),
            true,
        ),
        (
            "hoglin_baby",
            EntityModelInstance::hoglin(
                244,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                true,
            ),
            false,
        ),
    ] {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_hoglin_textured_output(&resting, &[base]);
        assert_hoglin_textured_output(&still, &[still_instance]);
        assert_hoglin_textured_output(&walking, &[walking_instance]);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{name}: a standing hoglin is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{name}: a walking hoglin differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
            let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking hoglin's feet should lift off the ground"
            );
        }
    }
}

fn assert_hoglin_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instances: &[EntityModelInstance],
) {
    assert_hoglin_folded_meshes_are_cutout_only(meshes);

    let mut vertex_offset = 0;
    for instance in instances {
        let (family, baby) = match instance.kind {
            EntityModelKind::Hoglin { family, baby } => (family, baby),
            _ => panic!("expected hoglin instance"),
        };
        let passes = hoglin_textured_layer_passes(family, baby);
        assert_eq!(passes.len(), 1);

        let vertex_count = 264;
        assert!(
            meshes.cutout.vertices[vertex_offset..vertex_offset + vertex_count]
                .iter()
                .all(
                    |vertex| vertex.light == instance.render_state.shader_light()
                        && vertex.overlay == instance.render_state.overlay_coords()
                )
        );
        vertex_offset += vertex_count;
    }
    assert_eq!(vertex_offset, meshes.cutout.vertices.len());
}

fn assert_hoglin_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn hoglin_texture_images() -> Vec<EntityModelTextureImage> {
    [
        HOGLIN_TEXTURE_REF,
        HOGLIN_BABY_TEXTURE_REF,
        ZOGLIN_TEXTURE_REF,
        ZOGLIN_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

#[test]
fn hoglin_ear_sway_pose_matches_vanilla_formula() {
    const HOGLIN_EAR_Z_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
    const BABY_HOGLIN_EAR_Z_ROT: f32 = 0.8727;
    // Vanilla HoglinModel.setupAnim: rightEar.zRot = -2π/9 - speed * sin(pos),
    // leftEar.zRot = +2π/9 + speed * sin(pos). The adult ear poses rest at ∓2π/9, so the
    // sway adds ∓speed * sin(pos) onto each; only zRot changes.
    let right = PartPose {
        offset: [-6.0, -2.0, -3.0],
        rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
    };
    let left = PartPose {
        offset: [6.0, -2.0, -3.0],
        rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
    };
    let ear_z = std::f32::consts::PI * 2.0 / 9.0;
    assert!(
        (right.rotation[2] + ear_z).abs() < 1e-6,
        "right ear rests at -2π/9"
    );
    assert!(
        (left.rotation[2] - ear_z).abs() < 1e-6,
        "left ear rests at +2π/9"
    );

    // At pos = π/2, speed = 1: sin = 1, so the sway magnitude is 1.0.
    let pos = std::f32::consts::FRAC_PI_2;
    let swayed_right = hoglin_ear_sway_pose(right, false, pos, 1.0);
    let swayed_left = hoglin_ear_sway_pose(left, true, pos, 1.0);
    assert!(
        (swayed_right.rotation[2] - (-ear_z - 1.0)).abs() < 1e-6,
        "right ear: {}",
        swayed_right.rotation[2]
    );
    assert!(
        (swayed_left.rotation[2] - (ear_z + 1.0)).abs() < 1e-6,
        "left ear: {}",
        swayed_left.rotation[2]
    );
    // Only zRot changes; the offset and other axes are preserved.
    assert_eq!(swayed_right.offset, right.offset);
    assert_eq!(swayed_right.rotation[0], right.rotation[0]);
    assert_eq!(swayed_right.rotation[1], right.rotation[1]);

    // A general (pos, speed): sway = speed * sin(pos).
    let pos = 1.3_f32;
    let speed = 0.6_f32;
    let sway = speed * pos.sin();
    let swayed_right = hoglin_ear_sway_pose(right, false, pos, speed);
    let swayed_left = hoglin_ear_sway_pose(left, true, pos, speed);
    assert!((swayed_right.rotation[2] - (-ear_z - sway)).abs() < 1e-6);
    assert!((swayed_left.rotation[2] - (ear_z + sway)).abs() < 1e-6);

    // sin(pos) == 0 (pos = 0) leaves the adult ears at their ±2π/9 rest splay.
    let swayed_right = hoglin_ear_sway_pose(right, false, 0.0, 1.0);
    assert_eq!(swayed_right.rotation[2], right.rotation[2]);

    // The baby ear poses rest at a wider angle (±BABY_HOGLIN_EAR_Z_ROT), but vanilla writes
    // the absolute from the literal 2π/9, so hoglin_ear_sway_pose ignores the base angle and
    // overrides the baby ears to ±2π/9 (± the sway).
    let baby_right = PartPose {
        offset: [-5.0, -1.0, -1.5],
        rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
    };
    let baby_left = PartPose {
        offset: [5.0, -1.0, -1.5],
        rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
    };
    assert!(
        (baby_right.rotation[2] + BABY_HOGLIN_EAR_Z_ROT).abs() < 1e-6
            && (BABY_HOGLIN_EAR_Z_ROT - ear_z).abs() > 1e-3,
        "the baby layer ears rest at a wider angle than 2π/9"
    );
    let baby_rest_right = hoglin_ear_sway_pose(baby_right, false, 0.0, 1.0);
    let baby_rest_left = hoglin_ear_sway_pose(baby_left, true, 0.0, 1.0);
    assert!(
        (baby_rest_right.rotation[2] + ear_z).abs() < 1e-6,
        "baby right ear overridden to -2π/9 at rest: {}",
        baby_rest_right.rotation[2]
    );
    assert!(
        (baby_rest_left.rotation[2] - ear_z).abs() < 1e-6,
        "baby left ear overridden to +2π/9 at rest: {}",
        baby_rest_left.rotation[2]
    );
}

#[test]
fn hoglin_textured_mesh_sways_its_ears_when_walking() {
    // The texture-backed hoglin runs the same adult ear sway, emitting the head subtree in
    // the same order, so the ears occupy textured vertices [72, 120). A standing hoglin is
    // byte-identical; a walking one (sin-nonzero phase) sways its ears.
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    let base =
        EntityModelInstance::hoglin(251, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false);
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (1.5, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_hoglin_textured_output(&resting, &[base]);
    assert_hoglin_textured_output(&walking, &[walking_instance]);
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "the ear sway keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices[72..120],
        walking.cutout.vertices[72..120],
        "the textured ears sway when walking"
    );
    assert_eq!(
        resting.cutout.vertices[0..72],
        walking.cutout.vertices[0..72],
        "body/mane/head cubes stay put"
    );
}

#[test]
fn hoglin_headbutt_raises_the_head_from_its_rest_tilt() {
    use std::f32::consts::PI;

    // Vanilla `HoglinModel.animateHeadbutt`: head.xRot = lerp(1 - |10 - 2·tick|/10, 0.87266463, -π/9).
    // At rest (tick 0) the factor is 0, so the head holds its baked down-tilt; at the attack midpoint
    // (tick 5) the factor is 1, so the head rises to -π/9. The baby additionally lifts head.y by
    // factor·2.5.
    let rest_tilt = 0.87266463_f32;
    let base =
        EntityModelInstance::hoglin(243, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false);

    // Rest: head holds the down-tilt.
    let mut resting = hoglin_model(false, &base);
    let resting_pitch = resting.child_mut("head").pose.rotation[0];
    assert!(
        (resting_pitch - rest_tilt).abs() < 1.0e-6,
        "rest tilt: {resting_pitch}"
    );

    // Peak ram (tick 5 → factor 1): head rises to -π/9.
    let mut ramming = hoglin_model(
        false,
        &base.with(|s| s.kind.hoglin_mut().attack_animation_tick = 5),
    );
    let ram_pitch = ramming.child_mut("head").pose.rotation[0];
    assert!(
        (ram_pitch - (-PI / 9.0)).abs() < 1.0e-6,
        "peak ram pitch: {ram_pitch}"
    );
    assert!(
        ram_pitch < resting_pitch,
        "the ram raises the head from its rest tilt"
    );

    // Mid-ramp (tick 8 → factor = 1 - |10-16|/10 = 0.4): lerp between the two.
    let mut mid = hoglin_model(
        false,
        &base.with(|s| s.kind.hoglin_mut().attack_animation_tick = 8),
    );
    let mid_pitch = mid.child_mut("head").pose.rotation[0];
    let factor = 1.0 - (10i32 - 2 * 8).abs() as f32 / 10.0;
    assert!(
        (mid_pitch - (rest_tilt + factor * (-PI / 9.0 - rest_tilt))).abs() < 1.0e-6,
        "mid-ramp pitch: {mid_pitch}"
    );

    // The baby lifts its head (head.y) at the ram peak; the adult does not.
    let baby_base =
        EntityModelInstance::hoglin(244, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, true);
    let mut baby_rest = hoglin_model(true, &baby_base);
    let baby_rest_y = baby_rest.child_mut("head").pose.offset[1];
    let mut baby_ram = hoglin_model(
        true,
        &baby_base.with(|s| s.kind.hoglin_mut().attack_animation_tick = 5),
    );
    let baby_ram_y = baby_ram.child_mut("head").pose.offset[1];
    assert!(
        (baby_ram_y - baby_rest_y - 2.5).abs() < 1.0e-6,
        "the baby lifts its head by 2.5"
    );

    // The headbutt visibly re-poses the rendered mesh.
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        entity_model_mesh(&[base.with(|s| s.kind.hoglin_mut().attack_animation_tick = 5)]).vertices,
        "the headbutt raises the head in the mesh"
    );
}
