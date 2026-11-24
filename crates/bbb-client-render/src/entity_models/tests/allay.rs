use super::*;

#[test]
fn allay_setup_anim_constants_and_curves_match_vanilla() {
    // `flyingFactor = min(walkAnimationSpeed / 0.3, 1)`.
    assert!((allay_flying_factor(0.0) - 0.0).abs() < 1.0e-6);
    assert!((allay_flying_factor(0.15) - 0.5).abs() < 1.0e-6);
    assert!((allay_flying_factor(0.3) - 1.0).abs() < 1.0e-6);
    assert!(
        (allay_flying_factor(0.6) - 1.0).abs() < 1.0e-6,
        "clamped to 1"
    );

    // `flapAmount = cos(ageInTicks·20° + walkAnimationPos)·π·0.15 + walkAnimationSpeed`.
    assert!(
        (allay_wing_flap_amount(0.0, 0.0, 0.0) - std::f32::consts::PI * 0.15).abs() < 1.0e-6,
        "at age 0 the flap is the peak amplitude"
    );
    let (age, pos, speed) = (9.0_f32, 0.3_f32, 0.1_f32);
    let expected_flap =
        (age * 20.0_f32.to_radians() + pos).cos() * std::f32::consts::PI * 0.15 + speed;
    assert!((allay_wing_flap_amount(age, pos, speed) - expected_flap).abs() < 1.0e-6);

    // `wing.xRot = 0.43633232·(1 - flyingFactor)`, `body.xRot = flyingFactor·π/4`.
    assert!((allay_wing_rest_x_rot(0.0) - 0.436_332_32).abs() < 1.0e-6);
    assert!(
        (allay_wing_rest_x_rot(0.3) - 0.0).abs() < 1.0e-6,
        "flat while flying"
    );
    assert!((allay_body_x_rot(0.0) - 0.0).abs() < 1.0e-6);
    assert!((allay_body_x_rot(0.3) - std::f32::consts::FRAC_PI_4).abs() < 1.0e-6);

    // `root.y = 23.5 + cos(ageInTicks·9°)·0.25·(1 - flyingFactor)`; idle peak at age 0 is 23.75,
    // and the bob vanishes once flying.
    assert!((allay_root_y(0.0, 0.0) - 23.75).abs() < 1.0e-6);
    assert!((allay_root_y(0.0, 0.3) - 23.5).abs() < 1.0e-6);
    let expected_root = 23.5 + (age * 9.0_f32.to_radians()).cos() * 0.25;
    assert!((allay_root_y(age, 0.0) - expected_root).abs() < 1.0e-6);

    // `armIdleBobAmount = 0.43633232 - cos(idleBobSpeed + 3π/2)·π·0.075·(1 - flyingFactor)`;
    // at age 0 `cos(3π/2) = 0` so the arms rest exactly on `0.43633232`.
    assert!((allay_arm_idle_bob_amount(0.0, 0.0) - 0.436_332_32).abs() < 1.0e-6);
    let expected_arm = 0.436_332_32
        - (age * 9.0_f32.to_radians() + std::f32::consts::PI * 1.5).cos()
            * std::f32::consts::PI
            * 0.075;
    assert!((allay_arm_idle_bob_amount(age, 0.0) - expected_arm).abs() < 1.0e-6);
    // While flying the arm bob collapses to the rest angle.
    assert!((allay_arm_idle_bob_amount(age, 0.3) - 0.436_332_32).abs() < 1.0e-6);

    // `holdingAnimationProgress` scales the idle arm bob out and raises/turns both arms inward:
    // `armFlyingRotX = holding * lerp(flyingFactor, -π/3, -1.134464)` and
    // `arm.yRot = ±0.27925268 * holding`.
    assert!((ALLAY_HELD_ITEM_ARM_Y_ROT - 0.279_252_68).abs() < 1.0e-6);
    assert!(
        (allay_arm_z_rot_amount(age, 0.0, 1.0) - ALLAY_REST_ANGLE).abs() < 1.0e-6,
        "full holding progress suppresses the idle bob amplitude"
    );
    assert!(
        (allay_arm_holding_x_rot(0.0, 1.0) + std::f32::consts::FRAC_PI_3).abs() < 1.0e-6,
        "idle allays raise arms to -π/3"
    );
    assert!(
        (allay_arm_holding_x_rot(0.3, 1.0) - ALLAY_MAX_HAND_HOLDING_ITEM_X_ROT).abs() < 1.0e-6,
        "flying allays use the deeper held-item arm pitch"
    );
}

#[test]
fn allay_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&allay_texture_images()).unwrap();
    // Vanilla `AllayModel` constructs with `RenderTypes::entityTranslucent`. The backend folds that
    // into the translucent mesh, but the submission must keep the vanilla render type, texture, tint,
    // transform, and default collector order.
    let base = EntityModelInstance::allay(850, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (15_u32 << 4) | (8_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);

    // Seven cubes → 42 faces / 168 vertices, with nothing on the cutout or eyes passes.
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 42);
    assert_eq!(meshes.translucent.vertices.len(), 168);
    assert_eq!(meshes.translucent.indices.len(), 252);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.render_state.shader_light()
            && vertex.overlay == base.render_state.overlay_coords()));
    assert_eq!(base.render_state.shader_light(), [1.0, 8.0 / 15.0]);
    assert_ne!(base.render_state.overlay_coords(), [0.0, 10.0]);

    // The head re-poses with the projected look yaw/pitch.
    let looking = entity_model_textured_meshes(
        &[base.with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))],
        &atlas,
    );
    assert_ne!(meshes.translucent.vertices, looking.translucent.vertices);

    // The wings flap and the arms bob as the age advances.
    let flapping = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, flapping.translucent.vertices);

    // The flying pose (walkAnimationSpeed ≥ 0.3) differs from the idle pose.
    let flying = entity_model_textured_meshes(
        &[base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.3))],
        &atlas,
    );
    assert_ne!(meshes.translucent.vertices, flying.translucent.vertices);
}

#[test]
fn translucent_draw_plan_sorts_order_then_camera_distance_then_insertion() {
    let (atlas, _) = build_entity_model_texture_atlas(&allay_and_breeze_texture_images()).unwrap();
    let near_a = EntityModelInstance::allay(851, [0.0, 64.0, 2.0], 0.0);
    let near_b = EntityModelInstance::allay(852, [0.0, 64.0, -2.0], 0.0);
    let breeze = EntityModelInstance::breeze(853, [0.0, 64.0, 8.0], 0.0);
    let far = EntityModelInstance::allay(854, [0.0, 64.0, 12.0], 0.0);

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[near_a, near_b, breeze, far],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    let draws = meshes
        .sorted_main_translucent_draws
        .iter()
        .filter_map(|draw| match draw {
            EntityModelTranslucentDrawRange::Textured(draw) => Some(draw),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(draws.len(), 5);
    assert!(draws
        .iter()
        .all(|draw| draw.atlas == EntityModelTexturedDrawAtlas::Static));
    assert_eq!(
        draws
            .iter()
            .map(|draw| (
                draw.render_type,
                draw.sort_key.order,
                draw.sort_key.insertion_index
            ))
            .collect::<Vec<_>>(),
        vec![
            (EntityModelLayerRenderType::EntityTranslucent, 0, 5),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 2),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 0),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 1),
            (
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                1,
                4,
            ),
        ],
        "draw ranges follow SubmitNodeCollector.order first, then far-to-near distance, then stable insertion"
    );
    assert!(draws[0].sort_key.distance_sq > draws[1].sort_key.distance_sq);
    assert!(draws[1].sort_key.distance_sq > draws[2].sort_key.distance_sq);
    assert!((draws[2].sort_key.distance_sq - draws[3].sort_key.distance_sq).abs() < 1.0e-5);
    assert!(
        draws[4].sort_key.distance_sq > draws[2].sort_key.distance_sq,
        "order(1) emissive overlay remains after all order(0) translucent model draws even when farther"
    );

    let main_draws = &meshes.sorted_main_translucent_draws;
    assert_eq!(main_draws.len(), 6);
    assert_eq!(
        main_draws
            .iter()
            .map(|draw| match draw {
                EntityModelTranslucentDrawRange::Textured(draw) => (
                    draw.render_type,
                    draw.sort_key.order,
                    draw.sort_key.insertion_index,
                    "textured",
                ),
                EntityModelTranslucentDrawRange::Geometry(draw) => (
                    draw.render_type,
                    draw.sort_key.order,
                    draw.sort_key.insertion_index,
                    "geometry",
                ),
            })
            .collect::<Vec<_>>(),
        vec![
            (
                EntityModelLayerRenderType::EntityTranslucent,
                0,
                5,
                "textured"
            ),
            (
                EntityModelLayerRenderType::EntityTranslucent,
                0,
                2,
                "textured"
            ),
            (
                EntityModelLayerRenderType::EntityTranslucent,
                0,
                0,
                "textured"
            ),
            (
                EntityModelLayerRenderType::EntityTranslucent,
                0,
                1,
                "textured"
            ),
            (EntityModelLayerRenderType::BreezeWind, 1, 3, "geometry"),
            (
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                1,
                4,
                "textured",
            ),
        ],
        "main translucent draw plan preserves BreezeRenderer's order(1) WindLayer before EyesLayer"
    );

    let near_breeze = EntityModelInstance::breeze(855, [0.0, 64.0, 2.0], 0.0);
    let winds = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[near_breeze, breeze],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );
    let draws = winds
        .sorted_main_translucent_draws
        .iter()
        .filter_map(|draw| {
            if let EntityModelTranslucentDrawRange::Geometry(draw) = draw {
                Some(draw)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(draws.len(), 2);
    for (draw, instance) in draws.into_iter().zip([breeze, near_breeze]) {
        let single = entity_model_textured_meshes(&[instance], &atlas);
        let start = draw.index_start as usize;
        let end = start + draw.index_count as usize;
        assert!(winds.scroll.indices[start..end]
            .iter()
            .map(|index| &winds.scroll.vertices[*index as usize])
            .eq(single
                .scroll
                .indices
                .iter()
                .map(|index| &single.scroll.vertices[*index as usize])));
    }
}

fn allay_texture_images() -> Vec<EntityModelTextureImage> {
    [ALLAY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn allay_and_breeze_texture_images() -> Vec<EntityModelTextureImage> {
    [ALLAY_TEXTURE_REF]
        .iter()
        .chain(&[
            BREEZE_TEXTURE_REF,
            BREEZE_EYES_TEXTURE_REF,
            BREEZE_WIND_TEXTURE_REF,
        ])
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
