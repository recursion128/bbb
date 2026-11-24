use super::*;

#[test]
fn warden_idle_and_head_pose_match_vanilla_setup_anim() {
    let age = 50.0_f32;
    let s = age * 0.1;

    // `animateIdlePose` body roll: xRot += 0.025·cos(s), zRot += 0.025·sin(s), yRot untouched.
    let body_bind = layer_pose(ModelLayer::WARDEN, "body");
    let body = warden_idle_body_pose(body_bind, age);
    assert!((body.rotation[0] - (body_bind.rotation[0] + 0.025 * s.cos())).abs() < 1.0e-6);
    assert_eq!(body.rotation[1], body_bind.rotation[1]);
    assert!((body.rotation[2] - (body_bind.rotation[2] + 0.025 * s.sin())).abs() < 1.0e-6);
    assert_eq!(body.offset, body_bind.offset);

    // The head pose: the look sets xRot/yRot, then the idle roll adds xRot += 0.06·sin(s) and
    // zRot += 0.06·cos(s).
    let head_bind = layer_pose(ModelLayer::WARDEN, "head");
    let head = warden_head_pose(head_bind, 40.0, -30.0, age);
    assert!((head.rotation[0] - ((-30.0_f32).to_radians() + 0.06 * s.sin())).abs() < 1.0e-6);
    assert!((head.rotation[1] - 40.0_f32.to_radians()).abs() < 1.0e-6);
    assert!((head.rotation[2] - (head_bind.rotation[2] + 0.06 * s.cos())).abs() < 1.0e-6);
    assert_eq!(head.offset, head_bind.offset);
}

#[test]
fn warden_walk_pose_matches_vanilla_animate_walk() {
    use std::f32::consts::{FRAC_PI_2, PI};
    // Vanilla `WardenModel.animateWalk(animationPos, animationSpeed)`:
    //   speedModifier        = min(0.5, 3·animationSpeed)
    //   adjustedPos          = animationPos·0.8662
    //   speedModifierWithMin = min(0.35, speedModifier)
    //   head.zRot += 0.3·sin(adjustedPos)·speedModifier
    //   head.xRot += 1.2·cos(adjustedPos + π/2)·speedModifierWithMin
    //   body.zRot  = 0.1·sin(adjustedPos)·speedModifier
    //   body.xRot  = 1.0·cos(adjustedPos)·speedModifierWithMin
    //   leftLeg.xRot  = 1.0·cos(adjustedPos)·speedModifier
    //   rightLeg.xRot = 1.0·cos(adjustedPos + π)·speedModifier
    //   leftArm.xRot  = -(0.8·cos(adjustedPos)·speedModifier)
    //   rightArm.xRot = -(0.8·sin(adjustedPos)·speedModifier)
    // `animationSpeed = 0.2` makes `3·0.2 = 0.6` clamp to `speedModifier = 0.5`, which in turn clamps
    // to `speedModifierWithMin = 0.35`, exercising both `min`s.
    let walk_pos = 3.0_f32;
    let walk_speed = 0.2_f32;
    let speed = (3.0 * walk_speed).min(0.5);
    let speed_with_min = speed.min(0.35);
    let adjusted = walk_pos * 0.8662;
    let cos = adjusted.cos();
    let sin = adjusted.sin();
    let pose = warden_walk_pose(walk_pos, walk_speed);
    assert!((pose.head_x_rot - 1.2 * (adjusted + FRAC_PI_2).cos() * speed_with_min).abs() < 1.0e-6);
    assert!((pose.head_z_rot - 0.3 * sin * speed).abs() < 1.0e-6);
    assert!((pose.body_x_rot - cos * speed_with_min).abs() < 1.0e-6);
    assert!((pose.body_z_rot - 0.1 * sin * speed).abs() < 1.0e-6);
    assert!((pose.left_leg_x_rot - cos * speed).abs() < 1.0e-6);
    assert!((pose.right_leg_x_rot - (adjusted + PI).cos() * speed).abs() < 1.0e-6);
    assert!((pose.left_arm_x_rot - -(0.8 * cos * speed)).abs() < 1.0e-6);
    assert!((pose.right_arm_x_rot - -(0.8 * sin * speed)).abs() < 1.0e-6);

    // A standing warden (`animationSpeed = 0`) zeroes every term, so it adds nothing on top of the
    // look/idle pose.
    let still = warden_walk_pose(7.5, 0.0);
    assert_eq!(
        [
            still.head_x_rot,
            still.head_z_rot,
            still.body_x_rot,
            still.body_z_rot,
            still.left_leg_x_rot,
            still.right_leg_x_rot,
            still.left_arm_x_rot,
            still.right_arm_x_rot
        ],
        [0.0; 8]
    );
}

#[test]
fn warden_tendril_x_rot_matches_vanilla_animate_tendrils() {
    // Vanilla `WardenModel.animateTendrils`:
    //   tendrilXRot = tendrilAnimation · (float)(cos(ageInTicks · 2.25) · π · 0.1)
    // The `cos·π·0.1` factor is evaluated in double precision before the `(float)` cast.
    let age = 7.0_f32;
    let tendril = 0.6_f32;
    let expected = tendril * ((age as f64 * 2.25).cos() * std::f64::consts::PI * 0.1) as f32;
    assert!((warden_tendril_x_rot(tendril, age) - expected).abs() < 1.0e-7);
    // A resting warden (`tendrilAnimation = 0`) adds no sway, whatever the age.
    assert_eq!(warden_tendril_x_rot(0.0, age), 0.0);
}

#[test]
fn warden_textured_render_matches_vanilla_renderer() {
    // The warden binds its base body texture (warden.png, atlas 128×128, whole model), then the five
    // `WardenEmissiveLayer`s as entityTranslucentEmissive passes, each over its own
    // `retainExactParts` subset: the always-on bioluminescent overlay (head/arms/legs), the two
    // pulsating-spots overlays (body/legs), the tendril overlay (the two tendrils, reusing the base
    // texture at the lerped `tendrilAnimation` alpha), and the heart overlay (body only,
    // warden_heart.png at the lerped `heartAnimation` alpha).
    let passes = warden_textured_layer_passes(0.0, 1.0, 0.7);
    assert_eq!(passes.len(), 6);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, WARDEN_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[1].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[1].texture, WARDEN_BIOLUMINESCENT_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[1].order, 1);
    assert_eq!(
        passes[1].visibility,
        EntityModelLayerVisibility::RetainedParts(&[
            "head",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg"
        ])
    );
    assert_eq!(
        passes[2].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[2].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[2].texture, WARDEN_PULSATING_SPOTS_1_TEXTURE_REF);
    assert_eq!(passes[2].order, 1);
    assert_eq!(
        passes[2].visibility,
        EntityModelLayerVisibility::RetainedParts(&[
            "body",
            "head",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg"
        ])
    );
    assert_eq!(
        passes[3].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[3].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[3].texture, WARDEN_PULSATING_SPOTS_2_TEXTURE_REF);
    assert_eq!(passes[3].order, 1);
    assert_eq!(passes[3].visibility, passes[2].visibility);
    // The tendril overlay reuses warden.png over the two tendril planes at `tendrilAnimation` (1.0 here).
    assert_eq!(
        passes[4].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[4].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[4].texture, WARDEN_TEXTURE_REF);
    assert_eq!(passes[4].tint[3], 1.0);
    assert_eq!(passes[4].order, 1);
    assert_eq!(
        passes[4].visibility,
        EntityModelLayerVisibility::RetainedParts(&["left_tendril", "right_tendril"])
    );
    // The heart overlay binds warden_heart.png over the body only at `heartAnimation` (0.7 here).
    assert_eq!(
        passes[5].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[5].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[5].texture, WARDEN_HEART_TEXTURE_REF);
    assert_eq!(passes[5].tint[3], 0.7);
    assert_eq!(passes[5].order, 1);
    assert_eq!(
        passes[5].visibility,
        EntityModelLayerVisibility::RetainedParts(&["body"])
    );

    // The pulsating alpha is `max(0, cos(ageInTicks · 0.045 + phase) · 0.25)`. At age 0 the first set
    // is at its peak 0.25 while the π-offset second set is clamped to 0; the two alternate over time.
    assert!((passes[2].tint[3] - 0.25).abs() < 1.0e-6);
    assert_eq!(passes[3].tint[3], 0.0);
    assert!((warden_pulsating_spots_alpha(0.0, 0.0) - 0.25).abs() < 1.0e-6);
    assert_eq!(warden_pulsating_spots_alpha(0.0, std::f32::consts::PI), 0.0);
    let half_period = std::f32::consts::PI / 0.045;
    assert_eq!(warden_pulsating_spots_alpha(half_period, 0.0), 0.0);
    assert!(
        (warden_pulsating_spots_alpha(half_period, std::f32::consts::PI) - 0.25).abs() < 1.0e-6
    );

    for texture in [
        WARDEN_TEXTURE_REF,
        WARDEN_BIOLUMINESCENT_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
        WARDEN_HEART_TEXTURE_REF,
    ] {
        assert!(entity_model_texture_refs().contains(&texture));
    }

    let images: Vec<EntityModelTextureImage> = [
        WARDEN_TEXTURE_REF,
        WARDEN_BIOLUMINESCENT_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
        WARDEN_HEART_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let default_instance = EntityModelInstance::warden(920, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[default_instance], &atlas);

    // The base cutout pass draws the whole body (10 cubes → 240 vertices), every cube at the neutral
    // tint; the emissive overlays route to the entityTranslucentEmissive mesh, not here.
    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == default_instance.render_state.shader_light()
            && vertex.overlay == default_instance.render_state.overlay_coords()));
    // Vanilla skips 0-alpha emissive layers. At age 0 with no heart/tendril pulse, only the
    // bioluminescent layer (5 cubes) and the first pulsating-spots layer (3 cubes) submit.
    assert_eq!(meshes.translucent_emissive.vertices.len(), 8 * 24);
    assert!(meshes.translucent_emissive.vertices.iter().all(|vertex| {
        vertex.light == default_instance.render_state.shader_light()
            && vertex.overlay == [0.0, default_instance.render_state.overlay_coords()[1]]
    }));

    let animated_instance = EntityModelInstance::warden(921, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.age_in_ticks = half_period;
        s.kind.warden_mut().tendril_animation = 1.0;
        s.kind.warden_mut().heart_animation = 0.7;
    });
    let animated_meshes = entity_model_textured_meshes(&[animated_instance], &atlas);

    // Half a pulsating-spots period flips the active spots layer; the non-zero tendril and heart
    // pulses add their retained tendril (2 cubes) and body-heart (1 cube) submissions.
    assert_eq!(animated_meshes.translucent_emissive.vertices.len(), 11 * 24);
    assert!(animated_meshes
        .translucent_emissive
        .vertices
        .iter()
        .all(|vertex| {
            vertex.light == animated_instance.render_state.shader_light()
                && vertex.overlay == [0.0, animated_instance.render_state.overlay_coords()[1]]
        }));
}

#[test]
fn warden_bioluminescent_missing_texture_preserves_other_geometry() {
    let images: Vec<EntityModelTextureImage> = [
        WARDEN_TEXTURE_REF,
        WARDEN_BIOLUMINESCENT_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
        WARDEN_HEART_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .filter_map(|(index, texture)| {
        if *texture == WARDEN_BIOLUMINESCENT_TEXTURE_REF {
            return None;
        }
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        Some(EntityModelTextureImage::new(
            *texture,
            vec![index as u8; len],
        ))
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::warden(922, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (2_u32 << 4) | (14_u32 << 20);
        s.white_overlay_progress = 0.7;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert_eq!(
        meshes.translucent_emissive.vertices.len(),
        3 * 24,
        "missing bioluminescent texture suppresses only its five retained cubes"
    );
    assert!(meshes
        .translucent_emissive
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 0.25]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]));
}
