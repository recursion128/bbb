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
    // Vanilla `BreezeAnimation.IDLE` is a 2.0s looping animation over four bones: the base body
    // layer's `head` (CATMULLROM position) and `rods` (LINEAR rotation + position), plus the wind
    // layer's `wind_top` / `wind_mid` LINEAR position sways.
    assert_eq!(BREEZE_IDLE.length_seconds, 2.0);
    assert!(BREEZE_IDLE.looping);
    assert_eq!(BREEZE_IDLE.bones.len(), 4);

    // The head bobs `0 → posVec(0, 1, 0) → 0` (y negated) on a CATMULLROM spline; sampled at the
    // mid keyframe it reaches `-1`.
    let (head_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "head", 1.0, 1.0);
    assert!((head_pos[1] - -1.0).abs() < 1.0e-6);

    // The rods spin a full `1080° = 6π` of yaw over the 2s cycle (LINEAR); halfway is `3π`.
    let (_, rods_rot) = sample_bone_offsets(&BREEZE_IDLE, "rods", 1.0, 1.0);
    assert!((rods_rot[1] - 3.0 * std::f32::consts::PI).abs() < 1.0e-5);

    // The wind pivots sway on LINEAR position splines (`pos_vec` negates only Y, so X/Z pass
    // through). `wind_top` starts at `posVec(0.5, 0, 0)`; at its `t = 0.75` keyframe it reaches
    // `posVec(-0.5, 0, -0.5)`.
    let (wind_top_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "wind_top", 0.75, 1.0);
    assert!((wind_top_pos[0] - -0.5).abs() < 1.0e-5);
    assert!((wind_top_pos[2] - -0.5).abs() < 1.0e-5);
    // `wind_mid` starts at `posVec(0.5, 0, -0.5)`; at `t = 1.0` it reaches `posVec(-0.5, 0, 0.5)`.
    let (wind_mid_pos, _) = sample_bone_offsets(&BREEZE_IDLE, "wind_mid", 1.0, 1.0);
    assert!((wind_mid_pos[0] - -0.5).abs() < 1.0e-5);
    assert!((wind_mid_pos[2] - 0.5).abs() < 1.0e-5);
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
    // Vanilla `BreezeEyesLayer`'s always-on emissive glow adds the `breeze_eyes.png` eyes overlay, and
    // `BreezeWindLayer` adds the 128×128 `breeze_wind.png` swirling wind body.
    assert!(entity_model_texture_refs().contains(&BREEZE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BREEZE_EYES_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BREEZE_WIND_TEXTURE_REF));
    assert_eq!(
        breeze_entity_texture_refs(),
        &[
            EntityModelTextureRef {
                path: "textures/entity/breeze/breeze.png",
                size: [32, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/breeze/breeze_eyes.png",
                size: [32, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/breeze/breeze_wind.png",
                size: [128, 128],
            },
        ]
    );
}

#[test]
fn breeze_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&breeze_texture_images()).unwrap();

    // The breeze base body draws into the translucent mesh (vanilla `RenderTypes::entityTranslucent`).
    // Head (two cubes) plus three rods → 5 cubes / 30 faces / 120 vertices, nothing on the cutout
    // pass, white tint.
    let base = EntityModelInstance::breeze(960, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == BREEZE_TEXTURE_REF)
        .expect("breeze emits a base body submit");
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(body_submit.order, 0);
    assert_eq!(body_submit.submit_sequence, 0);
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 30);
    assert_eq!(meshes.translucent.vertices.len(), 120);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // Vanilla `BreezeEyesLayer`'s always-on emissive glow re-renders the same head+rods geometry into
    // the eyes mesh with `breeze_eyes.png` (transparent except the head's eye UVs).
    let eyes_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == BREEZE_EYES_TEXTURE_REF)
        .expect("breeze emits a breezeEyes submit");
    assert_eq!(eyes_submit.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes_submit.order, 1);
    assert_eq!(eyes_submit.submit_sequence, 2);
    assert_eq!(eyes_submit.transform, entity_model_root_transform(base));
    assert_eq!(meshes.eyes.cutout_faces, 30);
    assert_eq!(meshes.eyes.vertices.len(), 120);

    // The looping IDLE re-poses both meshes with age and loops every 40 ticks.
    let later = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, later.translucent.vertices);
    assert_ne!(meshes.eyes.vertices, later.eyes.vertices);
    let one_cycle = entity_model_textured_meshes(&[base.with_age_in_ticks(40.0)], &atlas);
    assert_eq!(meshes.translucent.vertices, one_cycle.translucent.vertices);
    assert_eq!(meshes.eyes.vertices, one_cycle.eyes.vertices);
}

#[test]
fn breeze_wind_geometry_matches_vanilla_26_1_wind_layer() {
    // Vanilla `BreezeModel.createWindLayer` (atlas 128×128) retains the `wind_body` pivot → the
    // `wind_bottom` → `wind_mid` → `wind_top` shell chain. Each tier nests three concentric shells of
    // decreasing radius; `wind_bottom` is the single `texOffs(1, 83)` 5×7×5 box.
    assert_eq!(BREEZE_WIND_BOTTOM[0].min, [-2.5, -7.0, -2.5]);
    assert_eq!(BREEZE_WIND_BOTTOM[0].size, [5.0, 7.0, 5.0]);
    assert_eq!(BREEZE_WIND_BOTTOM[0].tex, [1.0, 83.0]);

    // `wind_mid` is the `texOffs(74, 28)` 12×6×12 outer shell + `texOffs(78, 32)` 8×6×8 + `texOffs(49,
    // 71)` 5×6×5; `wind_top` is the `texOffs(0, 0)` 18×8×18 outer shell + `texOffs(6, 6)` 12×8×12 +
    // `texOffs(105, 57)` 5×8×5.
    assert_eq!(BREEZE_WIND_MID.len(), 3);
    assert_eq!(BREEZE_WIND_MID[0].size, [12.0, 6.0, 12.0]);
    assert_eq!(BREEZE_WIND_MID[0].tex, [74.0, 28.0]);
    assert_eq!(BREEZE_WIND_TOP.len(), 3);
    assert_eq!(BREEZE_WIND_TOP[0].min, [-9.0, -8.0, -9.0]);
    assert_eq!(BREEZE_WIND_TOP[0].size, [18.0, 8.0, 18.0]);
    assert_eq!(BREEZE_WIND_TOP[0].tex, [0.0, 0.0]);

    // Each tier nests under the previous on its `PartPose.offset` (the pivots ride a vertical column).
    assert_eq!(BREEZE_WIND_BOTTOM_POSE.offset, [0.0, 24.0, 0.0]);
    assert_eq!(BREEZE_WIND_MID_POSE.offset, [0.0, -7.0, 0.0]);
    assert_eq!(BREEZE_WIND_TOP_POSE.offset, [0.0, -6.0, 0.0]);
}

#[test]
fn breeze_wind_body_folds_into_scrolling_overlay() {
    let (atlas, _) = build_entity_model_texture_atlas(&breeze_texture_images()).unwrap();

    // Vanilla `BreezeWindLayer` renders the separate `wind_body` shell chain with the translucent,
    // U-scrolling `breezeWind` render type. The wind body's 7 cubes (`wind_bottom` 1 + `wind_mid` 3 +
    // `wind_top` 3) → 42 faces / 168 vertices fold into the scroll mesh; nothing on the additive swirl.
    let base = EntityModelInstance::breeze(970, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((2_u32 << 4) | (14_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == BREEZE_TEXTURE_REF)
        .expect("breeze emits a base body submit");
    assert_eq!(body_submit.light, base.render_state.shader_light());
    assert_eq!(body_submit.overlay, base.render_state.overlay_coords());
    let wind_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.render_type == EntityModelLayerRenderType::BreezeWind)
        .expect("breeze emits a breezeWind layer submit");
    assert_eq!(wind_submit.texture, BREEZE_WIND_TEXTURE_REF);
    assert_eq!(wind_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(wind_submit.order, 1);
    assert_eq!(wind_submit.submit_sequence, 1);
    assert_eq!(wind_submit.transform, entity_model_root_transform(base));
    assert_eq!(wind_submit.light, body_submit.light);
    assert_eq!(wind_submit.overlay, [0.0, 10.0]);
    assert_ne!(wind_submit.overlay, body_submit.overlay);
    assert_eq!(meshes.scroll.vertices.len(), 168);
    assert_eq!(meshes.scroll.indices.len(), 42 * 6);
    assert!(meshes.scroll_additive.vertices.is_empty());

    // The wind body sways with the looping IDLE and its U coordinate scrolls by `ageInTicks · 0.02`;
    // age 7 differs from age 0 in both pose and scroll.
    let later = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.scroll.vertices, later.scroll.vertices);

    // The IDLE pose loops every 40 ticks, but the U-scroll keeps advancing (`40 · 0.02 = 0.8 ≠ 0`), so
    // the wind body at age 40 carries the same pose phase yet a different scrolled local UV.
    let one_cycle = entity_model_textured_meshes(&[base.with_age_in_ticks(40.0)], &atlas);
    assert_ne!(
        meshes.scroll.vertices, one_cycle.scroll.vertices,
        "the wind body's U-scroll advances past the looped pose"
    );

    // Each pose-driven action also re-poses the wind shell chain (the `wind_*` channels of SHOOT etc.).
    let shooting = entity_model_textured_meshes(&[base.with_breeze_shoot_seconds(0.25)], &atlas);
    assert_ne!(
        meshes.scroll.vertices, shooting.scroll.vertices,
        "the shoot action re-poses the wind body"
    );
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
