use super::*;

#[test]
fn enderman_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ENDERMAN_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_HAT[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_ARM[0],
        ModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_LEG[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );

    assert_eq!(ENDERMAN_PARTS.len(), 6);
    assert_part_tree(
        &ENDERMAN_PARTS[0],
        [0.0, -13.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HEAD.as_slice(),
        ENDERMAN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ENDERMAN_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HAT.as_slice(),
    );
    assert_part(
        &ENDERMAN_PARTS[1],
        [0.0, -14.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_BODY.as_slice(),
    );

    let limb_specs = [
        ([-5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([-2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
        ([2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in ENDERMAN_PARTS[2..].iter().zip(limb_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn enderman_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::enderman(141, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 63.9385, -0.25]);
    assert_close3(max, [0.375, 66.8135, 0.25]);
}

#[test]
fn enderman_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(
        EntityModelKind::Enderman.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/enderman/enderman.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        enderman_entity_texture_refs(),
        [
            EntityModelTextureRef {
                path: "textures/entity/enderman/enderman.png",
                size: [64, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/enderman/enderman_eyes.png",
                size: [64, 32],
            },
        ]
    );
    assert_eq!(
        EntityModelKind::Enderman.vanilla_layer_texture_refs(),
        &[ENDERMAN_EYES_TEXTURE_REF]
    );
}

#[test]
fn enderman_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let passes = enderman_textured_layer_passes();
    assert_eq!(passes.len(), 2);

    assert_eq!(passes[0].kind, EntityModelLayerKind::EndermanBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_ENDERMAN);
    assert_eq!(passes[0].texture, ENDERMAN_TEXTURE_REF);
    assert_eq!(passes[0].parts, ENDERMAN_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );

    assert_eq!(passes[1].kind, EntityModelLayerKind::EndermanEyes);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].model_layer, MODEL_LAYER_ENDERMAN);
    assert_eq!(passes[1].texture, ENDERMAN_EYES_TEXTURE_REF);
    assert_eq!(passes[1].parts, ENDERMAN_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[1].collector_order, passes[1].submit_sequence),
        (1, 1)
    );
}

#[test]
fn enderman_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_ENDERMAN, "minecraft:enderman#main");
    assert_eq!(
        ENDERMAN_EYES_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/enderman/enderman_eyes.png",
            size: [64, 32],
        }
    );
    assert_eq!(ENDERMAN_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        ENDERMAN_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [32.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_RIGHT_ARM[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            uv_size: [2.0, 30.0, 2.0],
            tex: [56.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_LEFT_ARM[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            uv_size: [2.0, 30.0, 2.0],
            tex: [56.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(ENDERMAN_TEXTURED_RIGHT_LEG[0].mirror, false);
    assert_eq!(ENDERMAN_TEXTURED_LEFT_LEG[0].mirror, true);
    assert_eq!(ENDERMAN_TEXTURED_PARTS[0].pose, ENDERMAN_PARTS[0].pose);
    assert_eq!(
        ENDERMAN_TEXTURED_PARTS[0].children,
        ENDERMAN_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(ENDERMAN_TEXTURED_PARTS[5].pose, ENDERMAN_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_enderman_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 64);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/enderman/enderman.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/enderman/enderman_eyes.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let eyes_start = rgba_offset(layout.width, 32, 0, "enderman eyes atlas row").unwrap();
    assert_eq!(&rgba[eyes_start..eyes_start + 4], &[1; 4]);
}

#[test]
fn enderman_textured_mesh_uses_parent_geometry_for_base_and_eyes_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();

    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::enderman(142, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );

    assert_eq!(meshes.cutout.cutout_faces, 42);
    assert_eq!(meshes.cutout.vertices.len(), 168);
    assert_eq!(meshes.cutout.indices.len(), 252);
    assert_eq!(meshes.eyes.cutout_faces, 42);
    assert_eq!(meshes.eyes.vertices.len(), 168);
    assert_eq!(meshes.eyes.indices.len(), 252);
    assert_close2(meshes.cutout.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(meshes.eyes.vertices[0].uv, [16.0 / 64.0, 0.5]);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        textured_mesh_extents(&meshes.eyes),
        textured_mesh_extents(&meshes.cutout)
    );
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-0.375, 63.9385, -0.25]);
    assert_close3(max, [0.375, 66.8135, 0.25]);
}

#[test]
fn enderman_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(143, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn enderman_leg_swing_pose_halves_and_clamps_the_humanoid_swing() {
    // Vanilla EndermanModel.setupAnim: super.setupAnim sets leg.xRot =
    // cos(pos * 0.6662 [+ π]) * 1.4 * speed, then the enderman halves it (*= 0.5) and
    // clamps it to [-0.4, 0.4]. ENDERMAN_PARTS lists the right leg at index 4 (x = -2,
    // in phase) and the left at index 5 (x = +2, out of phase).
    // At pos = 0, speed = 1: raw = cos(0) * 1.4 * 0.5 = 0.7, clamped to 0.4.
    let right = enderman_leg_swing_pose(ENDERMAN_PARTS[4].pose, 0.0, 1.0);
    let left = enderman_leg_swing_pose(ENDERMAN_PARTS[5].pose, 0.0, 1.0);
    assert!(
        (right.rotation[0] - 0.4).abs() < 1e-6,
        "right leg clamps to +0.4: {}",
        right.rotation[0]
    );
    assert!(
        (left.rotation[0] + 0.4).abs() < 1e-6,
        "left leg clamps to -0.4: {}",
        left.rotation[0]
    );

    // A low speed stays inside the clamp window, showing the bare halving:
    // cos(0) * 1.4 * 0.3 * 0.5 = 0.21.
    let right_slow = enderman_leg_swing_pose(ENDERMAN_PARTS[4].pose, 0.0, 0.3);
    assert!(
        (right_slow.rotation[0] - 1.4 * 0.3 * 0.5).abs() < 1e-6,
        "unclamped half amplitude: {}",
        right_slow.rotation[0]
    );
    // A general (pos, speed) within the window: cos(pos * 0.6662) * 1.4 * speed * 0.5.
    let phase = 2.0_f32 * 0.6662;
    let right_general = enderman_leg_swing_pose(ENDERMAN_PARTS[4].pose, 2.0, 0.3);
    assert!((right_general.rotation[0] - phase.cos() * 1.4 * 0.3 * 0.5).abs() < 1e-6);
}

#[test]
fn enderman_arm_swing_pose_halves_and_clamps_the_humanoid_swing() {
    // Vanilla EndermanModel.setupAnim: super.setupAnim sets arm.xRot =
    // cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5 (amplitude 1.0), then the enderman
    // halves it (*= 0.5) and clamps it to [-0.4, 0.4], exactly as it does the legs.
    // ENDERMAN_PARTS lists the right arm at index 2 (x = -5, the out-of-phase + π side)
    // and the left at index 3 (x = +5, in phase). The combined amplitude is
    // 2.0 * 0.5 * 0.5 = 0.5, so unclamped arm.xRot = cos(angle) * speed * 0.5.
    // At pos = 0, speed = 1: right raw = cos(π) * 0.5 = -0.5, clamped to -0.4; left raw
    // = cos(0) * 0.5 = +0.5, clamped to +0.4.
    let right = enderman_arm_swing_pose(ENDERMAN_PARTS[2].pose, 0.0, 1.0);
    let left = enderman_arm_swing_pose(ENDERMAN_PARTS[3].pose, 0.0, 1.0);
    assert!(
        (right.rotation[0] + 0.4).abs() < 1e-6,
        "right arm clamps to -0.4: {}",
        right.rotation[0]
    );
    assert!(
        (left.rotation[0] - 0.4).abs() < 1e-6,
        "left arm clamps to +0.4: {}",
        left.rotation[0]
    );

    // A low speed stays inside the clamp window, showing the bare halving:
    // cos(π) * 1 * 0.5 * 0.3 = -0.15 (right), the opposite phase to the same-side leg.
    let right_slow = enderman_arm_swing_pose(ENDERMAN_PARTS[2].pose, 0.0, 0.3);
    assert!(
        (right_slow.rotation[0] + 0.3 * 0.5).abs() < 1e-6,
        "unclamped half amplitude, out of phase: {}",
        right_slow.rotation[0]
    );
    // A general (pos, speed) within the window: cos(pos * 0.6662 + π) * 2.0 * speed * 0.5
    // * 0.5 for the right arm; the arm's + π phase is the leg's negation.
    let phase = 2.0_f32 * 0.6662;
    let right_general = enderman_arm_swing_pose(ENDERMAN_PARTS[2].pose, 2.0, 0.3);
    assert!(
        (right_general.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * 0.3 * 0.5 * 0.5)
            .abs()
            < 1e-6
    );
    // The arm and same-side leg counter-swing: the right arm (+ π) is the negation of
    // the right leg (in phase) at the same half amplitude.
    let right_leg = enderman_leg_swing_pose(ENDERMAN_PARTS[4].pose, 2.0, 0.3);
    assert!((right_general.rotation[0] + right_leg.rotation[0] * (1.0 / 1.4)).abs() < 1e-6);
}

#[test]
fn enderman_swings_its_legs_when_walking() {
    // `EndermanModel extends HumanoidModel`; its legs swing the inherited swing,
    // halved and clamped. A standing enderman is inert; a walking one lifts its feet
    // and splays its legs along Z. The arm halve/clamp is covered separately by
    // `enderman_swings_its_arms_when_walking`; the carried-block and creepy poses are
    // deferred. Colored path here, textured below.
    let base = EntityModelInstance::enderman(260, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking enderman's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking enderman's legs should splay along Z"
    );
}

#[test]
fn enderman_textured_mesh_swings_legs_when_walking() {
    // The real enderman render path (texture-backed) swings the same halved/clamped
    // legs. A standing enderman is byte-identical however far the swing has advanced;
    // a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(261, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing enderman is inert"
    );
    assert_eq!(
        resting.vertices.len(),
        walking.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.vertices, walking.vertices,
        "a walking enderman differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting);
    let (walk_min, walk_max) = textured_mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking enderman's feet should lift off the ground"
    );
}

#[test]
fn enderman_swings_its_arms_when_walking() {
    // The enderman applies the inherited HumanoidModel arm swing, halved and clamped to
    // [-0.4, 0.4], to its long arms. In the body layer the parts emit head(0)+hat(1)+
    // body(2), then right_arm(3), left_arm(4), right_leg(5), left_leg(6) as 24-vertex
    // blocks, so the arms occupy vertices [72, 120) and the legs [120, 168). A standing
    // enderman is inert; a walking one swings both its arms and legs while the head and
    // body stay put. Colored path here, textured below.
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let base = EntityModelInstance::enderman(262, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        rest.vertices[0..72],
        walking.vertices[0..72],
        "head and body never swing"
    );
    assert_ne!(
        rest.vertices[72..120],
        walking.vertices[72..120],
        "arms swing"
    );
    assert_ne!(
        rest.vertices[120..168],
        walking.vertices[120..168],
        "legs swing"
    );
    let rest_arm_z = z_extent(&rest.vertices[72..120]);
    let walk_arm_z = z_extent(&walking.vertices[72..120]);
    assert!(
        walk_arm_z > rest_arm_z + 0.1,
        "a forward/back arm swing deepens the arm Z footprint: {rest_arm_z} -> {walk_arm_z}"
    );
}

#[test]
fn enderman_textured_mesh_swings_arms_when_walking() {
    // The texture-backed enderman base layer runs the same halved/clamped arm swing,
    // emitting the parts in the same order, so the arms occupy textured vertices
    // [72, 120). A standing enderman is byte-identical; a walking one swings its arms.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(263, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);
    assert_eq!(
        resting.vertices[0..72],
        walking.vertices[0..72],
        "head and body never swing"
    );
    assert_ne!(
        resting.vertices[72..120],
        walking.vertices[72..120],
        "arms swing"
    );
    let rest_arm_z = z_extent(&resting.vertices[72..120]);
    let walk_arm_z = z_extent(&walking.vertices[72..120]);
    assert!(
        walk_arm_z > rest_arm_z + 0.1,
        "the textured arms splay along Z when walking: {rest_arm_z} -> {walk_arm_z}"
    );
}

#[test]
fn enderman_carried_arm_pose_matches_vanilla_setup_anim() {
    // Vanilla EndermanModel.setupAnim carried-block branch *sets* both arms to xRot = -0.5
    // (overriding the swing and its clamp) with zRot = +0.05 on the right arm (part offset
    // x < 0) and -0.05 on the left; yRot and the bind offset are preserved. ENDERMAN_PARTS
    // lists the right arm at index 2 (x = -5) and the left at index 3 (x = +5).
    let right = enderman_carried_arm_pose(ENDERMAN_PARTS[2].pose);
    let left = enderman_carried_arm_pose(ENDERMAN_PARTS[3].pose);
    assert_eq!(right.offset, ENDERMAN_PARTS[2].pose.offset);
    assert_eq!(left.offset, ENDERMAN_PARTS[3].pose.offset);
    assert_eq!(right.rotation, [-0.5, 0.0, 0.05]);
    assert_eq!(left.rotation, [-0.5, 0.0, -0.05]);
}

#[test]
fn enderman_creepy_hat_child_raises_to_match_head_drop() {
    // Vanilla EndermanModel.setupAnim isCreepy branch raises the hat child y += 5 while the
    // emit drops the head y -= 5, so the outer head layer keeps its world position as the
    // inner head opens downward. The creepy hat child differs from the rest hat only by the
    // +5 y offset and is otherwise the same cube (colored and textured).
    assert_eq!(
        ENDERMAN_HEAD_CHILDREN_CREEPY[0].pose.offset,
        [0.0, 5.0, 0.0]
    );
    assert_eq!(
        ENDERMAN_HEAD_CHILDREN_CREEPY[0].pose.rotation,
        [0.0, 0.0, 0.0]
    );
    assert_eq!(
        ENDERMAN_HEAD_CHILDREN_CREEPY[0].cubes,
        ENDERMAN_HEAD_CHILDREN[0].cubes
    );
    assert_eq!(
        ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY[0].pose.offset,
        [0.0, 5.0, 0.0]
    );
    assert_eq!(
        ENDERMAN_TEXTURED_HEAD_CHILDREN_CREEPY[0].cubes,
        ENDERMAN_TEXTURED_HEAD_CHILDREN[0].cubes
    );
}

#[test]
fn enderman_holds_its_arms_out_when_carrying_a_block() {
    // Carrying a block sets both arms to xRot = -0.5 (held out front), overriding the swing
    // and leaving the head, body and legs untouched. The colored parts emit head(0)+hat(1)+
    // body(2) as vertices [0, 72), then the arms [72, 120) and legs [120, 168). Colored path
    // here, textured below.
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let base = EntityModelInstance::enderman(264, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let carrying = entity_model_mesh(&[base.with_enderman_carrying(true)]);
    assert_eq!(
        rest.vertices[0..72],
        carrying.vertices[0..72],
        "the head and body do not move to carry a block"
    );
    assert_eq!(
        rest.vertices[120..168],
        carrying.vertices[120..168],
        "the legs do not move to carry a block"
    );
    assert_ne!(
        rest.vertices[72..120],
        carrying.vertices[72..120],
        "both arms swing out to carry the block"
    );
    let rest_arm_z = z_extent(&rest.vertices[72..120]);
    let carry_arm_z = z_extent(&carrying.vertices[72..120]);
    assert!(
        carry_arm_z > rest_arm_z + 0.3,
        "the held-out arms reach forward along Z: {rest_arm_z} -> {carry_arm_z}"
    );
}

#[test]
fn enderman_textured_mesh_holds_its_arms_out_when_carrying() {
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(265, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let carrying = entity_model_textured_mesh(&[base.with_enderman_carrying(true)], &atlas);
    assert_eq!(
        resting.vertices[0..72],
        carrying.vertices[0..72],
        "the head and body stay put"
    );
    assert_ne!(
        resting.vertices[72..120],
        carrying.vertices[72..120],
        "the textured arms are held out to carry the block"
    );
    let rest_arm_z = z_extent(&resting.vertices[72..120]);
    let carry_arm_z = z_extent(&carrying.vertices[72..120]);
    assert!(
        carry_arm_z > rest_arm_z + 0.3,
        "the held-out arms reach forward along Z: {rest_arm_z} -> {carry_arm_z}"
    );
}

#[test]
fn enderman_drops_its_head_when_creepy() {
    // The creepy stare drops the inner head y -= 5 while the hat child rises y += 5, so the
    // hat holds its world position while the head opens downward. Only the head cube moves;
    // the hat, body, arms and legs are byte-identical. The colored head occupies vertices
    // [0, 24), the hat [24, 48). Colored path here, textured below.
    let y_centroid = |verts: &[EntityModelVertex]| -> f32 {
        verts.iter().map(|vertex| vertex.position[1]).sum::<f32>() / verts.len() as f32
    };
    let base = EntityModelInstance::enderman(266, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let creepy = entity_model_mesh(&[base.with_enderman_creepy(true)]);
    assert_ne!(
        rest.vertices[0..24],
        creepy.vertices[0..24],
        "the inner head drops"
    );
    assert_eq!(
        rest.vertices[24..48],
        creepy.vertices[24..48],
        "the hat holds its world position (the +5 raise cancels the head's -5 drop)"
    );
    assert_eq!(
        rest.vertices[48..168],
        creepy.vertices[48..168],
        "the body, arms and legs do not move"
    );
    // 5 model pixels at the 1/16 entity-model scale = 0.3125 world units; the (-1, -1, 1)
    // flip lifts the dropped inner head in world Y.
    let shift = y_centroid(&creepy.vertices[0..24]) - y_centroid(&rest.vertices[0..24]);
    assert!(
        (shift.abs() - 0.3125).abs() < 1.0e-3,
        "the inner head shifts 5px (0.3125 world) in Y: {shift}"
    );
}

#[test]
fn enderman_textured_mesh_drops_its_head_when_creepy() {
    // The texture-backed enderman runs the same creepy head/hat shift. Only the inner head
    // moves; the hat stays put and the rest of the body is byte-identical.
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(267, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let creepy = entity_model_textured_mesh(&[base.with_enderman_creepy(true)], &atlas);
    assert_ne!(
        resting.vertices[0..24],
        creepy.vertices[0..24],
        "the inner head drops"
    );
    assert_eq!(
        resting.vertices[24..48],
        creepy.vertices[24..48],
        "the hat holds its world position"
    );
    assert_eq!(
        resting.vertices[48..168],
        creepy.vertices[48..168],
        "the body, arms and legs do not move"
    );
}

fn enderman_texture_images() -> Vec<EntityModelTextureImage> {
    enderman_entity_texture_refs()
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
