use super::*;
use crate::entity_models::model_layers::ravager_model;

#[test]
fn ravager_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&RAVAGER_TEXTURE_REF));
}

#[test]
fn ravager_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = ravager_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, RAVAGER_TEXTURE_REF);
    // The vestigial `parts` slice is nulled; emit builds `RavagerModel::new()` and renders its tree.
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(ModelLayer::RAVAGER, ModelLayer::RAVAGER);
    assert_eq!(RAVAGER_TEXTURE_REF.size, [128, 128]);
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
    let instance = EntityModelInstance::ravager(109, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_ravager_submission_matches_vanilla(&meshes);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);
    let neck = ModelLayer::RAVAGER
        .bake()
        .part_vertex_range("neck")
        .unwrap();
    assert_close2(mesh.vertices[neck.start].uv, [96.0 / 128.0, 73.0 / 128.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}

#[test]
fn ravager_textured_mesh_turns_nested_head_not_neck_or_body() {
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let base = EntityModelInstance::ravager(110, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (50.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_ravager_submission_matches_vanilla(&resting);
    assert_ravager_submission_matches_vanilla(&yawed);
    assert_ravager_submission_matches_vanilla(&pitched);

    // The vanilla look turns the nested head only; the neck cube and the body/legs stay put.
    let model = ModelLayer::RAVAGER.bake();
    let neck = model.part_vertex_range("neck").unwrap();
    let head = model.part_vertex_range("head").unwrap();
    let neck_cube = neck.start..head.start;
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_eq!(
        resting.cutout.vertices[neck_cube.clone()],
        yawed.cutout.vertices[neck_cube]
    );
    assert_ne!(
        resting.cutout.vertices[head.clone()],
        yawed.cutout.vertices[head.clone()]
    );
    assert_ne!(
        yawed.cutout.vertices[head.clone()],
        pitched.cutout.vertices[head]
    );
    assert_eq!(
        parts_vertices(&resting.cutout.vertices, &model, RAVAGER_BODY_AND_LEGS),
        parts_vertices(&yawed.cutout.vertices, &model, RAVAGER_BODY_AND_LEGS)
    );
}

#[test]
fn ravager_textured_mesh_swings_legs_not_neck_or_head() {
    // A walking ravager (head at rest) changes only the body/leg region; the neck and head
    // stay put. A standing ravager is byte-identical however far the swing has advanced.
    let model = ModelLayer::RAVAGER.bake();
    let neck = model.part_vertex_range("neck").unwrap();
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let base = EntityModelInstance::ravager(281, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_ravager_submission_matches_vanilla(&resting);
    assert_ravager_submission_matches_vanilla(&still);
    assert_ravager_submission_matches_vanilla(&walking);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing ravager is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_eq!(
        resting.cutout.vertices[neck.clone()],
        walking.cutout.vertices[neck],
        "the neck and head stay put while walking"
    );
    assert_ne!(
        parts_vertices(&resting.cutout.vertices, &model, RAVAGER_BODY_AND_LEGS),
        parts_vertices(&walking.cutout.vertices, &model, RAVAGER_BODY_AND_LEGS),
        "the body/leg region swings"
    );
}

const RAVAGER_BODY_AND_LEGS: &[&str] = &[
    "body",
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

fn assert_ravager_submission_matches_vanilla(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn ravager_leg_swing_pose_matches_vanilla_formula() {
    // Vanilla RavagerModel.setupAnim: legRot = 0.4 * walkAnimationSpeed;
    // rightHindLeg.xRot = cos(pos * 0.6662) * legRot;
    // leftHindLeg.xRot  = cos(pos * 0.6662 + π) * legRot;
    // rightFrontLeg.xRot = cos(pos * 0.6662 + π) * legRot;
    // leftFrontLeg.xRot  = cos(pos * 0.6662) * legRot.
    // This is the QuadrupedModel diagonal phase (in phase when x*z < 0) but with a 0.4
    // amplitude rather than the usual 1.4. The right hind leg sits at x = -8, z = 18
    // (x*z < 0 -> in phase), the left hind at x = 8 (out of phase); the front legs are at
    // z = -5 so their phases flip.
    let right_hind_pose = PartPose {
        offset: [-8.0, -13.0, 18.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_hind_pose = PartPose {
        offset: [8.0, -13.0, 18.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_front_pose = PartPose {
        offset: [-8.0, -13.0, -5.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_front_pose = PartPose {
        offset: [8.0, -13.0, -5.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_hind = ravager_leg_swing_pose(right_hind_pose, 0.0, 1.0);
    let left_hind = ravager_leg_swing_pose(left_hind_pose, 0.0, 1.0);
    let right_front = ravager_leg_swing_pose(right_front_pose, 0.0, 1.0);
    let left_front = ravager_leg_swing_pose(left_front_pose, 0.0, 1.0);
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
    let right_hind = ravager_leg_swing_pose(right_hind_pose, 1.5, 0.5);
    let left_hind = ravager_leg_swing_pose(left_hind_pose, 1.5, 0.5);
    assert!((right_hind.rotation[0] - (1.5_f32 * 0.6662).cos() * 0.4 * 0.5).abs() < 1e-6);
    assert!(
        (left_hind.rotation[0] - (1.5_f32 * 0.6662 + std::f32::consts::PI).cos() * 0.4 * 0.5).abs()
            < 1e-6
    );

    // The swing only sets xRot; the resting offset and yRot/zRot are preserved.
    assert_eq!(right_hind.offset, right_hind_pose.offset);
    assert_eq!(right_hind.rotation[1], right_hind_pose.rotation[1]);
    assert_eq!(right_hind.rotation[2], right_hind_pose.rotation[2]);
}

fn ravager_texture_images() -> Vec<EntityModelTextureImage> {
    [RAVAGER_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

// Vanilla `Mth.triangleWave` (replicated for the test expectations).
fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

fn ravager_neck_and_mouth(
    model: &mut crate::entity_models::model_layer::BakedRoot,
) -> (PartPose, PartPose) {
    let neck = model.child_mut("neck").pose;
    let mouth = model
        .child_mut("neck")
        .child_mut("head")
        .child_mut("mouth")
        .pose;
    (neck, mouth)
}

#[test]
fn ravager_combat_poses_drive_the_neck_and_mouth() {
    use std::f32::consts::PI;

    let base = EntityModelInstance::ravager(230, [0.0, 64.0, 0.0], 0.0);

    // Resting: the neck sits at its bind offset and the jaw cracks open π·0.01 (vanilla always poses it).
    let mut resting = ravager_model(&base);
    let (neck, mouth) = ravager_neck_and_mouth(&mut resting);
    assert_eq!(neck.offset, [0.0, -7.0, 5.5]);
    assert_eq!(neck.rotation[0], 0.0);
    assert!(
        (mouth.rotation[0] - PI * 0.01).abs() < 1.0e-6,
        "resting jaw: {}",
        mouth.rotation[0]
    );

    // Attacking: the neck lunges forward (`z = -6.5 + headPos`) and the jaw snaps open.
    let attack = 8.0_f32;
    let mut attacking =
        ravager_model(&base.with(|s| s.kind.ravager_mut().attack_ticks_remaining = attack));
    let (neck, mouth) = ravager_neck_and_mouth(&mut attacking);
    let scaled = (1.0 + triangle_wave(attack, 10.0)) * 0.5;
    let head_pos = scaled * scaled * scaled * 12.0;
    assert!(
        (neck.offset[2] - (-6.5 + head_pos)).abs() < 1.0e-6,
        "neck lunges: {}",
        neck.offset[2]
    );
    // attack > 5 → mouth.xRot = sin((-4 + tick)/4)·π·0.4.
    assert!(
        (mouth.rotation[0] - ((-4.0 + attack) / 4.0).sin() * PI * 0.4).abs() < 1.0e-6,
        "bite jaw: {}",
        mouth.rotation[0]
    );

    // Stunned: the neck tilts (xRot = 0.21991149) and side-shakes (neck.x != 0); jaw opens π·0.05.
    let stun = 30.0_f32;
    let mut stunned =
        ravager_model(&base.with(|s| s.kind.ravager_mut().stunned_ticks_remaining = stun));
    let (neck, mouth) = ravager_neck_and_mouth(&mut stunned);
    assert!(
        (neck.rotation[0] - 0.21991149).abs() < 1.0e-6,
        "stun tilt: {}",
        neck.rotation[0]
    );
    assert!(
        (neck.offset[0] - (f64::from(stun) / 40.0 * 10.0).sin() as f32 * 3.0).abs() < 1.0e-6,
        "stun shake: {}",
        neck.offset[0]
    );
    assert!(
        (mouth.rotation[0] - PI * 0.05).abs() < 1.0e-6,
        "stun jaw: {}",
        mouth.rotation[0]
    );

    // Roaring (not stunned/attacking): the jaw gapes π/2·sin(roar·π/4).
    let roar = 0.5_f32;
    let mut roaring = ravager_model(&base.with(|s| s.kind.ravager_mut().roar_animation = roar));
    let (_, mouth) = ravager_neck_and_mouth(&mut roaring);
    assert!(
        (mouth.rotation[0] - (PI / 2.0) * (roar * PI * 0.25).sin()).abs() < 1.0e-6,
        "roar jaw: {}",
        mouth.rotation[0]
    );

    // The combat pose visibly changes the rendered mesh vs a resting ravager.
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        entity_model_mesh(&[base.with(|s| s.kind.ravager_mut().attack_ticks_remaining = attack)])
            .vertices,
        "the attack lunge re-poses the neck/mouth"
    );
}
