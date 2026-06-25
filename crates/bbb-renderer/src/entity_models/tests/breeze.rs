use super::*;

#[test]
fn breeze_geometry_matches_vanilla_26_1_base_body_layer() {
    // Vanilla `BreezeModel.createBaseMesh` retained to `head` + `rods` (atlas 32×32). The head is
    // the `texOffs(4, 24)` 10×3×4 jaw plate plus the `texOffs(0, 0)` 8×8×8 head cube.
    assert_eq!(BREEZE_HEAD[0].min, [-5.0, -5.0, -4.2]);
    assert_eq!(BREEZE_HEAD[0].size, [10.0, 3.0, 4.0]);
    assert_eq!(BREEZE_HEAD[1].size, [8.0, 8.0, 8.0]);
    assert_eq!(BREEZE_HEAD_POSE.offset, [0.0, 4.0, 0.0]);

    // The three rods share the `texOffs(0, 17)` 2×8×2 box at distinct compound bind rotations.
    assert_eq!(BREEZE_ROD[0].size, [2.0, 8.0, 2.0]);
    assert_eq!(BREEZE_RODS_POSE.offset, [0.0, 8.0, 0.0]);
    assert_eq!(BREEZE_ROD_1_POSE.rotation, [-2.7489, -1.0472, 3.1416]);
    assert_eq!(BREEZE_ROD_2_POSE.rotation, [-2.7489, 1.0472, 3.1416]);
    assert_eq!(BREEZE_ROD_3_POSE.rotation, [0.3927, 0.0, 0.0]);

    // Each unified cube also carries the vanilla `BreezeModel.createBaseMesh` texOffs UV (atlas
    // 32×32); no `CubeDeformation`, so each `uv_size` matches the box `size`.
    assert_eq!(BREEZE_HEAD[0].tex, [4.0, 24.0]);
    assert_eq!(BREEZE_HEAD[0].uv_size, [10.0, 3.0, 4.0]);
    assert_eq!(BREEZE_HEAD[1].tex, [0.0, 0.0]);
    assert_eq!(BREEZE_ROD[0].tex, [0.0, 17.0]);
    assert!(!BREEZE_ROD[0].mirror);
}

#[test]
fn breeze_idle_animation_matches_vanilla_definition() {
    // Vanilla `BreezeAnimation.IDLE` is a 2.0s looping animation; the base body layer uses the
    // `head` (CATMULLROM position) and `rods` (LINEAR rotation + position) bones.
    assert_eq!(BREEZE_IDLE.length_seconds, 2.0);
    assert!(BREEZE_IDLE.looping);
    assert_eq!(BREEZE_IDLE.bones.len(), 2);

    // The head bobs `0 → posVec(0, 1, 0) → 0` (y negated) on a CATMULLROM spline; sampled at the
    // mid keyframe it reaches `-1`.
    let (head_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "head", 1.0, 1.0);
    assert!((head_pos[1] - -1.0).abs() < 1.0e-6);

    // The rods spin a full `1080° = 6π` of yaw over the 2s cycle (LINEAR); halfway is `3π`.
    let (_, rods_rot) = sample_bone_offsets(&BREEZE_IDLE, "rods", 1.0, 1.0);
    assert!((rods_rot[1] - 3.0 * std::f32::consts::PI).abs() < 1.0e-5);
}

#[test]
fn breeze_mesh_uses_vanilla_base_body_geometry() {
    // Head (two cubes) plus three rods → 5 cubes / 30 faces / 120 vertices.
    let breeze = entity_model_mesh(&[EntityModelInstance::breeze(950, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(breeze.opaque_faces, 30);
    assert_eq!(breeze.vertices.len(), 120);
    assert!(breeze
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BREEZE_SLATE, 1.0)));
}

#[test]
fn breeze_idle_animates_and_loops() {
    // The looping IDLE re-poses the mesh as the age advances within the 2.0s (40-tick) cycle.
    let base = EntityModelInstance::breeze(951, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(early.vertices, later.vertices, "the idle animates with age");

    // The animation loops every 2.0s = 40 ticks, so age 0 and age 40 sample the same phase.
    let one_cycle = entity_model_mesh(&[base.with_age_in_ticks(40.0)]);
    assert_eq!(
        early.vertices, one_cycle.vertices,
        "the idle loops every 40 ticks"
    );
}

#[test]
fn breeze_action_animations_match_vanilla_definitions() {
    let total_keyframes = |def: &AnimationDefinition| -> usize {
        def.bones
            .iter()
            .flat_map(|bone| bone.channels.iter())
            .map(|channel| channel.keyframes.len())
            .sum()
    };

    // Vanilla `BreezeAnimation.SHOOT`: 1.125s, NOT looping, 6 bones (body/head/rods + 3 wind), 43
    // keyframes. The head pitches `-12.5°` at the draw (t=0.25).
    assert_eq!(BREEZE_SHOOT.length_seconds, 1.125);
    assert!(!BREEZE_SHOOT.looping);
    assert_eq!(BREEZE_SHOOT.bones.len(), 6);
    assert_eq!(total_keyframes(&BREEZE_SHOOT), 43);
    let (_, head_rot) = sample_bone_offsets(&BREEZE_SHOOT, "head", 0.25, 1.0);
    assert!((head_rot[0] - (-12.5_f32).to_radians()).abs() < 1.0e-5);

    // Vanilla `BreezeAnimation.JUMP` / `INHALE`: 0.5s / 2.0s, NOT looping, 7 bones each, 26 keyframes.
    assert_eq!(BREEZE_JUMP.length_seconds, 0.5);
    assert!(!BREEZE_JUMP.looping);
    assert_eq!(BREEZE_JUMP.bones.len(), 7);
    assert_eq!(total_keyframes(&BREEZE_JUMP), 26);
    assert_eq!(BREEZE_INHALE.length_seconds, 2.0);
    assert!(!BREEZE_INHALE.looping);
    assert_eq!(BREEZE_INHALE.bones.len(), 7);
    assert_eq!(total_keyframes(&BREEZE_INHALE), 26);

    // Vanilla `SLIDE` / `SLIDE_BACK`: 0.2s / 0.1s, NOT looping, 3 bones, 6 keyframes. The `body` pivot
    // slides back `posVec(0, 0, -6)` (z) over SLIDE and returns to the neutral pose over SLIDE_BACK.
    assert_eq!(BREEZE_SLIDE.length_seconds, 0.2);
    assert!(!BREEZE_SLIDE.looping);
    assert_eq!(BREEZE_SLIDE.bones.len(), 3);
    assert_eq!(total_keyframes(&BREEZE_SLIDE), 6);
    let (slide_body, _) = sample_bone_offsets(&BREEZE_SLIDE, "body", 0.2, 1.0);
    assert!((slide_body[2] - -6.0).abs() < 1.0e-5);
    assert_eq!(BREEZE_SLIDE_BACK.length_seconds, 0.1);
    assert!(!BREEZE_SLIDE_BACK.looping);
    assert_eq!(BREEZE_SLIDE_BACK.bones.len(), 3);
    assert_eq!(total_keyframes(&BREEZE_SLIDE_BACK), 6);
    let (back_body, _) = sample_bone_offsets(&BREEZE_SLIDE_BACK, "body", 0.1, 1.0);
    assert!(
        back_body[2].abs() < 1.0e-5,
        "slideBack ends at the neutral pose"
    );
}

#[test]
fn breeze_actions_re_pose_the_body_model() {
    // Each pose-driven action, applied over its projected elapsed seconds, re-poses the base body
    // layer's bones (body/head/rods) off the idle pose; the `-1.0` stopped sentinel leaves the idle
    // pose untouched. The body model has 120 vertices (head [0, 48), rods [48, 120)).
    let base = EntityModelInstance::breeze(952, [0.0, 64.0, 0.0], 0.0);
    let idle = entity_model_mesh(&[base]);

    for (label, instance) in [
        ("shoot", base.with_breeze_shoot_seconds(0.25)),
        ("slide", base.with_breeze_slide_seconds(0.1)),
        ("slide_back", base.with_breeze_slide_back_seconds(0.05)),
        ("inhale", base.with_breeze_inhale_seconds(1.0)),
        ("long_jump", base.with_breeze_long_jump_seconds(0.25)),
    ] {
        let posed = entity_model_mesh(&[instance]);
        assert_eq!(idle.vertices.len(), posed.vertices.len());
        assert_ne!(
            idle.vertices, posed.vertices,
            "the {label} action re-poses the body model"
        );
    }

    // The `-1.0` sentinel applies no action — the mesh stays at the idle pose.
    assert_eq!(
        idle.vertices,
        entity_model_mesh(&[base
            .with_breeze_shoot_seconds(-1.0)
            .with_breeze_slide_seconds(-1.0)
            .with_breeze_slide_back_seconds(-1.0)
            .with_breeze_inhale_seconds(-1.0)
            .with_breeze_long_jump_seconds(-1.0)])
        .vertices,
        "the stopped sentinels leave the idle pose"
    );
}

#[test]
fn breeze_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Breeze;
    assert_eq!(kind.model_key(), "breeze");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/breeze/breeze.png",
            size: [32, 32],
        })
    );
    assert_eq!(
        breeze_entity_texture_refs(),
        &[EntityModelTextureRef {
            path: "textures/entity/breeze/breeze.png",
            size: [32, 32],
        }]
    );
}

#[test]
fn breeze_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&breeze_texture_images()).unwrap();

    // The breeze base body draws into the translucent mesh (vanilla `RenderTypes::entityTranslucent`).
    // Head (two cubes) plus three rods → 5 cubes / 30 faces / 120 vertices, nothing on the cutout
    // or eyes passes, white tint.
    let base = EntityModelInstance::breeze(960, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 30);
    assert_eq!(meshes.translucent.vertices.len(), 120);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The looping IDLE re-poses the mesh with age and loops every 40 ticks.
    let later = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, later.translucent.vertices);
    let one_cycle = entity_model_textured_meshes(&[base.with_age_in_ticks(40.0)], &atlas);
    assert_eq!(meshes.translucent.vertices, one_cycle.translucent.vertices);
}

fn breeze_texture_images() -> Vec<EntityModelTextureImage> {
    breeze_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
