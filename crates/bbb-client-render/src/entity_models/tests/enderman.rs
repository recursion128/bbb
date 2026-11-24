use super::*;

use crate::entity_models::transforms::enderman_creepy_render_offset;

// The bind poses of the enderman's two arms / two legs, mirrored from the model file so the
// pose-math tests can exercise `enderman_*_swing_pose` without the deleted `ENDERMAN_PARTS` consts.
const RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, -12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, -5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, -5.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

#[test]
fn enderman_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let passes = enderman_textured_layer_passes();
    assert_eq!(passes.len(), 2);

    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, ENDERMAN_TEXTURE_REF);
    // The vestigial `parts` slices are nulled; both passes read the unified `EndermanModel` tree.
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[1].texture, ENDERMAN_EYES_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[1].order, 1);

    assert_eq!(ModelLayer::ENDERMAN, ModelLayer::ENDERMAN);
    assert_eq!(
        ENDERMAN_EYES_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/enderman/enderman_eyes.png",
            size: [64, 32],
        }
    );
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
    let instance = EntityModelInstance::enderman(142, [0.0, 64.0, 0.0], 0.0);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_enderman_textured_output(&meshes, instance);

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
    // The arms' always-on idle bob splays the X extent to `±0.5494`.
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-0.5493963, 63.9385, -0.25]);
    assert_close3(max, [0.5493963, 66.8135, 0.25]);
}

#[test]
fn enderman_eyes_missing_texture_preserves_other_geometry() {
    let len =
        usize::try_from(ENDERMAN_TEXTURE_REF.size[0] * ENDERMAN_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ENDERMAN_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::enderman(142, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
}

#[test]
fn enderman_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(143, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_enderman_textured_output(&resting, base);
    assert_enderman_textured_output(&yawed, yawed_instance);
    assert_enderman_textured_output(&pitched, pitched_instance);
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn enderman_leg_swing_pose_halves_and_clamps_the_humanoid_swing() {
    // Vanilla EndermanModel.setupAnim: super.setupAnim sets leg.xRot =
    // cos(pos * 0.6662 [+ π]) * 1.4 * speed, then the enderman halves it (*= 0.5) and
    // clamps it to [-0.4, 0.4]. The right leg sits at x = -2 (in phase), the left at x = +2
    // (out of phase). At pos = 0, speed = 1: raw = cos(0) * 1.4 * 0.5 = 0.7, clamped to 0.4.
    let right = enderman_leg_swing_pose(RIGHT_LEG_POSE, 0.0, 1.0);
    let left = enderman_leg_swing_pose(LEFT_LEG_POSE, 0.0, 1.0);
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
    let right_slow = enderman_leg_swing_pose(RIGHT_LEG_POSE, 0.0, 0.3);
    assert!(
        (right_slow.rotation[0] - 1.4 * 0.3 * 0.5).abs() < 1e-6,
        "unclamped half amplitude: {}",
        right_slow.rotation[0]
    );
    // A general (pos, speed) within the window: cos(pos * 0.6662) * 1.4 * speed * 0.5.
    let phase = 2.0_f32 * 0.6662;
    let right_general = enderman_leg_swing_pose(RIGHT_LEG_POSE, 2.0, 0.3);
    assert!((right_general.rotation[0] - phase.cos() * 1.4 * 0.3 * 0.5).abs() < 1e-6);
}

#[test]
fn enderman_arm_swing_pose_halves_and_clamps_the_humanoid_swing() {
    // Vanilla EndermanModel.setupAnim: super.setupAnim sets arm.xRot =
    // cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5 (amplitude 1.0) plus the idle bob, then
    // the enderman halves the xRot (*= 0.5) and clamps it to [-0.4, 0.4], exactly as it
    // does the legs. At ageInTicks = 0 the bob's xRot term (sin(0) * 0.05) is zero, so the
    // xRot here is the bare swing. The right arm sits at x = -5 (the out-of-phase + π side)
    // and the left at x = +5 (in phase). The combined amplitude is 2.0 * 0.5 * 0.5 = 0.5,
    // so unclamped arm.xRot = cos(angle) * speed * 0.5. At pos = 0, speed = 1: right raw =
    // cos(π) * 0.5 = -0.5, clamped to -0.4; left raw = cos(0) * 0.5 = +0.5, clamped to +0.4.
    let right = enderman_arm_swing_pose(RIGHT_ARM_POSE, 0.0, 1.0, 0.0);
    let left = enderman_arm_swing_pose(LEFT_ARM_POSE, 0.0, 1.0, 0.0);
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
    let right_slow = enderman_arm_swing_pose(RIGHT_ARM_POSE, 0.0, 0.3, 0.0);
    assert!(
        (right_slow.rotation[0] + 0.3 * 0.5).abs() < 1e-6,
        "unclamped half amplitude, out of phase: {}",
        right_slow.rotation[0]
    );
    // A general (pos, speed) within the window: cos(pos * 0.6662 + π) * 2.0 * speed * 0.5
    // * 0.5 for the right arm; the arm's + π phase is the leg's negation.
    let phase = 2.0_f32 * 0.6662;
    let right_general = enderman_arm_swing_pose(RIGHT_ARM_POSE, 2.0, 0.3, 0.0);
    assert!(
        (right_general.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * 0.3 * 0.5 * 0.5)
            .abs()
            < 1e-6
    );
    // The arm and same-side leg counter-swing: the right arm (+ π) is the negation of
    // the right leg (in phase) at the same half amplitude.
    let right_leg = enderman_leg_swing_pose(RIGHT_LEG_POSE, 2.0, 0.3);
    assert!((right_general.rotation[0] + right_leg.rotation[0] * (1.0 / 1.4)).abs() < 1e-6);
}

#[test]
fn enderman_arm_pose_composes_the_idle_bob_before_the_halve_and_clamp() {
    // Vanilla EndermanModel.setupAnim's super.setupAnim applies the always-on idle bob
    // (`AnimationUtils.bobModelPart`) to both arms before the enderman halves/clamps xRot.
    // The halve/clamp touches only xRot, so the bob's zRot survives untouched (the arms
    // splay) while its xRot contribution is halved along with the swing.
    let age = 13.0_f32;
    // At rest (speed 0) the swing is zero, isolating the bob. The right arm (offset x < 0)
    // bobs with scale +1: zRot = cos(age * 0.09) * 0.05 + 0.05, xRot = sin(age * 0.067) *
    // 0.05 — and the enderman halves+clamps that xRot.
    let right = enderman_arm_swing_pose(RIGHT_ARM_POSE, 0.0, 0.0, age);
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let bob_x = (age * 0.067).sin() * 0.05;
    assert!(
        (right.rotation[2] - bob_z).abs() < 1e-6,
        "the bob's zRot survives the clamp: {} vs {}",
        right.rotation[2],
        bob_z
    );
    assert!(
        (right.rotation[0] - (bob_x * 0.5).clamp(-0.4, 0.4)).abs() < 1e-6,
        "the bob's xRot is halved and clamped: {}",
        right.rotation[0]
    );
    // The left arm (offset x >= 0) bobs with the opposite sign (vanilla scale -1.0).
    let left = enderman_arm_swing_pose(LEFT_ARM_POSE, 0.0, 0.0, age);
    assert!(
        (left.rotation[2] + bob_z).abs() < 1e-6,
        "the left arm splays the opposite way: {}",
        left.rotation[2]
    );
}

#[test]
fn enderman_textured_mesh_swings_legs_when_walking() {
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(261, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_enderman_textured_output(&resting, base);
    assert_enderman_textured_output(&still, still_instance);
    assert_enderman_textured_output(&walking, walking_instance);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing enderman is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices, walking.cutout.vertices,
        "a walking enderman differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
    let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking enderman's feet should lift off the ground"
    );
}

#[test]
fn enderman_textured_mesh_swings_arms_when_walking() {
    // A standing enderman is byte-identical; a walking one swings its arms.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    const STILL: &[&str] = &["head", "body"];
    const ARMS: &[&str] = &["right_arm", "left_arm"];
    let model = ModelLayer::ENDERMAN.bake();
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(263, [0.0, 64.0, 0.0], 0.0);
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_enderman_textured_output(&resting, base);
    assert_enderman_textured_output(&walking, walking_instance);
    assert_eq!(
        parts_vertices(&resting.cutout.vertices, &model, STILL),
        parts_vertices(&walking.cutout.vertices, &model, STILL),
        "head and body never swing"
    );
    assert_ne!(
        parts_vertices(&resting.cutout.vertices, &model, ARMS),
        parts_vertices(&walking.cutout.vertices, &model, ARMS),
        "arms swing"
    );
    let rest_arm_z = z_extent(&parts_vertices(&resting.cutout.vertices, &model, ARMS));
    let walk_arm_z = z_extent(&parts_vertices(&walking.cutout.vertices, &model, ARMS));
    assert!(
        walk_arm_z > rest_arm_z + 0.1,
        "the textured arms splay along Z when walking: {rest_arm_z} -> {walk_arm_z}"
    );
}

#[test]
fn enderman_carried_arm_pose_matches_vanilla_setup_anim() {
    // Vanilla EndermanModel.setupAnim carried-block branch *sets* both arms to xRot = -0.5
    // (overriding the swing and its clamp) with zRot = +0.05 on the right arm (part offset
    // x < 0) and -0.05 on the left; yRot and the bind offset are preserved. The right arm
    // sits at x = -5 and the left at x = +5.
    let right = enderman_carried_arm_pose(RIGHT_ARM_POSE);
    let left = enderman_carried_arm_pose(LEFT_ARM_POSE);
    assert_eq!(right.offset, RIGHT_ARM_POSE.offset);
    assert_eq!(left.offset, LEFT_ARM_POSE.offset);
    assert_eq!(right.rotation, [-0.5, 0.0, 0.05]);
    assert_eq!(left.rotation, [-0.5, 0.0, -0.05]);
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
    const STILL: &[&str] = &["head", "body"];
    const ARMS: &[&str] = &["right_arm", "left_arm"];
    let model = ModelLayer::ENDERMAN.bake();
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base = EntityModelInstance::enderman(265, [0.0, 64.0, 0.0], 0.0);
    let carrying_instance = base.with(|s| s.kind.enderman_mut().carrying = true);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let carrying = entity_model_textured_meshes(&[carrying_instance], &atlas);
    assert_enderman_textured_output(&resting, base);
    assert_enderman_textured_output(&carrying, carrying_instance);
    assert_eq!(
        parts_vertices(&resting.cutout.vertices, &model, STILL),
        parts_vertices(&carrying.cutout.vertices, &model, STILL),
        "the head and body stay put"
    );
    assert_ne!(
        parts_vertices(&resting.cutout.vertices, &model, ARMS),
        parts_vertices(&carrying.cutout.vertices, &model, ARMS),
        "the textured arms are held out to carry the block"
    );
    let rest_arm_z = z_extent(&parts_vertices(&resting.cutout.vertices, &model, ARMS));
    let carry_arm_z = z_extent(&parts_vertices(&carrying.cutout.vertices, &model, ARMS));
    assert!(
        carry_arm_z > rest_arm_z + 0.3,
        "the held-out arms reach forward along Z: {rest_arm_z} -> {carry_arm_z}"
    );
}

#[test]
fn enderman_creepy_render_offset_matches_vanilla_axes_scale_and_seed() {
    // Vanilla `EndermanRenderer.getRenderOffset`: when `state.isCreepy`, add two
    // gaussian samples on X/Z with amplitude `0.02 * state.scale`; Y stays at the
    // inherited offset. The renderer RNG seed is not protocol-visible, so bbb uses
    // a deterministic Java-LCG-shaped seed from entity id + age while preserving the
    // vanilla axis and scale semantics.
    let calm =
        EntityModelInstance::enderman(266, [0.0, 64.0, 0.0], 0.0).with(|s| s.age_in_ticks = 7.0);
    let creepy = calm.with(|s| s.kind.enderman_mut().creepy = true);
    let non_enderman = EntityModelInstance::chicken(266, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.kind.enderman_mut().creepy = true;
        s.age_in_ticks = 7.0;
    });

    assert_eq!(enderman_creepy_render_offset(&calm), [0.0; 3]);
    assert_eq!(enderman_creepy_render_offset(&non_enderman), [0.0; 3]);

    let offset = enderman_creepy_render_offset(&creepy);
    assert_eq!(offset[1], 0.0);
    assert!(
        offset[0] != 0.0 || offset[2] != 0.0,
        "creepy endermen should jitter on the X/Z plane"
    );
    assert_eq!(enderman_creepy_render_offset(&creepy), offset);

    let scaled = enderman_creepy_render_offset(&creepy.with(|s| s.scale = 2.0));
    assert!((scaled[0] - offset[0] * 2.0).abs() < 1.0e-6);
    assert_eq!(scaled[1], 0.0);
    assert!((scaled[2] - offset[2] * 2.0).abs() < 1.0e-6);
    assert_ne!(
        enderman_creepy_render_offset(&creepy.with(|s| s.age_in_ticks = 8.0)),
        offset,
        "changing age changes the deterministic renderer-local RNG seed"
    );
}

#[test]
fn enderman_textured_mesh_drops_its_head_when_creepy() {
    // The texture-backed enderman runs the same root jitter and creepy head/hat
    // shift. After subtracting the root jitter from the folded mesh, only the
    // inner head moves; the hat stays put and the rest of the body is
    // byte-identical.
    let model = ModelLayer::ENDERMAN.bake();
    const REST: &[&str] = &["body", "right_arm", "left_arm", "right_leg", "left_leg"];
    let head = model.part_vertex_range("head").unwrap();
    let hat = model.part_vertex_range("hat").unwrap();
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let base =
        EntityModelInstance::enderman(268, [0.0, 64.0, 0.0], 0.0).with(|s| s.age_in_ticks = 7.0);
    let creepy_instance = base.with(|s| s.kind.enderman_mut().creepy = true);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let creepy = entity_model_textured_meshes(&[creepy_instance], &atlas);
    assert_enderman_textured_output(&resting, base);
    assert_enderman_textured_output(&creepy, creepy_instance);

    let creepy_offset = enderman_creepy_render_offset(&creepy_instance);
    assert_ne!(creepy_offset, [0.0; 3]);
    let creepy_cutout_without_root_offset =
        textured_vertices_without_render_offset(&creepy.cutout.vertices, creepy_offset);
    assert_ne!(
        resting.cutout.vertices[head.start..hat.start],
        creepy_cutout_without_root_offset[head.start..hat.start],
        "the inner head drops"
    );
    assert_textured_vertices_close(
        &resting.cutout.vertices[hat.clone()],
        &creepy_cutout_without_root_offset[hat],
    );
    assert_textured_vertices_close(
        &parts_vertices(&resting.cutout.vertices, &model, REST),
        &parts_vertices(&creepy_cutout_without_root_offset, &model, REST),
    );
}

#[test]
fn enderman_eyes_submission_uses_no_overlay_metadata() {
    // Vanilla `EyesLayer.submit` forwards the entity light but always uses
    // `OverlayTexture.NO_OVERLAY`, even when the base body is hurt or white-flashing.
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();
    let light_coords = (6_u32 << 4) | (10_u32 << 20);
    let instance = EntityModelInstance::enderman(268, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = light_coords;
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_enderman_textured_output(&meshes, instance);
}

fn assert_enderman_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_enderman_folded_meshes_are_base_and_eyes_only(meshes);

    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

fn textured_vertices_without_render_offset(
    vertices: &[EntityModelTexturedVertex],
    offset: [f32; 3],
) -> Vec<EntityModelTexturedVertex> {
    vertices
        .iter()
        .copied()
        .map(|mut vertex| {
            vertex.position[0] -= offset[0];
            vertex.position[1] -= offset[1];
            vertex.position[2] -= offset[2];
            vertex
        })
        .collect()
}

fn assert_textured_vertices_close(
    expected: &[EntityModelTexturedVertex],
    actual: &[EntityModelTexturedVertex],
) {
    assert_eq!(expected.len(), actual.len());
    for (expected, actual) in expected.iter().zip(actual) {
        assert_vertex_position_close(actual.position, expected.position);
        assert_eq!(actual.uv, expected.uv);
        assert_eq!(actual.tint, expected.tint);
        assert_eq!(actual.light, expected.light);
        assert_eq!(actual.overlay, expected.overlay);
        assert_eq!(actual.normal, expected.normal);
    }
}

fn assert_vertex_position_close(actual: [f32; 3], expected: [f32; 3]) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1.0e-5,
            "axis {axis}: expected {expected:?}, got {actual:?}"
        );
    }
}

fn assert_enderman_folded_meshes_are_base_and_eyes_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
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

fn enderman_texture_images() -> Vec<EntityModelTextureImage> {
    [ENDERMAN_TEXTURE_REF, ENDERMAN_EYES_TEXTURE_REF]
        .iter()
        .copied()
        .enumerate()
        .map(|(index, texture)| {
            EntityModelTextureImage::new(
                texture,
                vec![
                    u8::try_from(index).unwrap();
                    usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap()
                ],
            )
        })
        .collect()
}
