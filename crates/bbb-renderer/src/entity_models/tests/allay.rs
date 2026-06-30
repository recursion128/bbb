use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn allay_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AllayModel.createBodyLayer` (atlas 32×32). Head is a plain 5³ box. Each unified cube
    // carries both the colored geometry/tint and the textured `uv_size` / `texOffs` / `mirror`.
    assert_eq!(
        ALLAY_HEAD[0],
        ModelCube::new(
            [-2.5, -5.0, -2.5],
            [5.0, 5.0, 5.0],
            ALLAY_BLUE,
            [5.0, 5.0, 5.0],
            [0.0, 0.0],
            false,
        )
    );

    // Body: the plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
    // `CubeDeformation(-0.2)` (min +0.2, size -0.4); the inset cube keeps the 3×5×2 base uv_size.
    assert_eq!(ALLAY_BODY.len(), 2);
    assert_eq!(ALLAY_BODY[0].min, [-1.5, 0.0, -1.0]);
    assert_eq!(ALLAY_BODY[0].size, [3.0, 4.0, 2.0]);
    assert_eq!(ALLAY_BODY[0].tex, [0.0, 10.0]);
    assert_eq!(ALLAY_BODY[0].uv_size, [3.0, 4.0, 2.0]);
    assert_eq!(ALLAY_BODY[1].min, [-1.3, 0.2, -0.8]);
    assert_eq!(ALLAY_BODY[1].size, [2.6, 4.6, 1.6]);
    assert_eq!(ALLAY_BODY[1].tex, [0.0, 16.0]);
    assert_eq!(ALLAY_BODY[1].uv_size, [3.0, 5.0, 2.0]);

    // Arms: 1×4×2 boxes inset by `CubeDeformation(-0.01)` (min +0.01, size -0.02), uv_size keeps the
    // 1×4×2 base box. The right and left arms differ in their box origin (`-0.75` vs `-0.25`) and
    // `texOffs(23, 0)` / `texOffs(23, 6)`.
    assert_eq!(ALLAY_RIGHT_ARM[0].min, [-0.74, -0.49, -0.99]);
    assert_eq!(ALLAY_RIGHT_ARM[0].size, [0.98, 3.98, 1.98]);
    assert_eq!(ALLAY_RIGHT_ARM[0].uv_size, [1.0, 4.0, 2.0]);
    assert_eq!(ALLAY_RIGHT_ARM[0].tex, [23.0, 0.0]);
    assert_eq!(ALLAY_LEFT_ARM[0].min, [-0.24, -0.49, -0.99]);
    assert_eq!(ALLAY_LEFT_ARM[0].size, [0.98, 3.98, 1.98]);
    assert_eq!(ALLAY_LEFT_ARM[0].uv_size, [1.0, 4.0, 2.0]);
    assert_eq!(ALLAY_LEFT_ARM[0].tex, [23.0, 6.0]);

    // Wings: zero-thickness 0×5×8 planes whose box starts at y=1, both `texOffs(16, 14)` with NO
    // mirror (unlike the vex).
    assert_eq!(ALLAY_WING[0].min, [0.0, 1.0, 0.0]);
    assert_eq!(ALLAY_WING[0].size, [0.0, 5.0, 8.0]);
    assert_eq!(ALLAY_WING[0].uv_size, [0.0, 5.0, 8.0]);
    assert_eq!(ALLAY_WING[0].tex, [16.0, 14.0]);
    assert!(!ALLAY_WING[0].mirror);

    // Part offsets: the model root sits at +23.5, head at -3.99, body at -4.0, arms ±1.75,
    // wings ±0.5 and forward 0.6.
    assert_eq!(ALLAY_ROOT_BASE_Y, 23.5);
    assert_eq!(ALLAY_HEAD_POSE.offset, [0.0, -3.99, 0.0]);
    assert_eq!(ALLAY_BODY_POSE.offset, [0.0, -4.0, 0.0]);
    assert_eq!(ALLAY_RIGHT_ARM_POSE.offset, [-1.75, 0.5, 0.0]);
    assert_eq!(ALLAY_LEFT_ARM_POSE.offset, [1.75, 0.5, 0.0]);
    assert_eq!(ALLAY_RIGHT_WING_POSE.offset, [-0.5, 0.0, 0.6]);
    assert_eq!(ALLAY_LEFT_WING_POSE.offset, [0.5, 0.0, 0.6]);
}

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
fn allay_mesh_uses_vanilla_body_layer_geometry() {
    // Seven cubes (head, two body boxes, two arms, two wings) → 42 faces / 168 vertices.
    let allay = entity_model_mesh(&[EntityModelInstance::allay(800, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(allay.opaque_faces, 42);
    assert_eq!(allay.vertices.len(), 168);
    assert_eq!(allay.indices.len(), 252);
    assert!(allay
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ALLAY_BLUE, 1.0)));
}

#[test]
fn allay_head_tracks_look_angles() {
    // The head re-poses with the projected look yaw/pitch; everything else is unchanged.
    let base = EntityModelInstance::allay(801, [0.0, 64.0, 0.0], 0.0);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -25.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the head tracks the look"
    );
}

#[test]
fn allay_wings_and_arms_animate_with_age() {
    // A still allay (age 0) differs from one advanced in age: the wings flap and the arms bob.
    let base = EntityModelInstance::allay(802, [0.0, 64.0, 0.0], 0.0);
    let still = entity_model_mesh(&[base]);
    let flapping = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), flapping.vertices.len());
    assert_ne!(still.vertices, flapping.vertices, "the wings flap with age");
}

#[test]
fn allay_holding_item_progress_poses_both_arms() {
    // Vanilla `AllayModel.setupAnim` uses `holdingAnimationProgress` to pitch both
    // arms up, turn them inward, and reduce the empty-hand idle arm bob. Same age
    // and walk state, so only the held-item arm blend changes the mesh.
    let base = EntityModelInstance::allay(805, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(9.0);
    let empty_handed = entity_model_mesh(&[base]);
    let holding = entity_model_mesh(&[base.with_allay_holding_item_progress(1.0)]);
    assert_eq!(empty_handed.vertices.len(), holding.vertices.len());
    assert_ne!(
        empty_handed.vertices, holding.vertices,
        "holdingAnimationProgress raises and turns the allay arms"
    );
}

#[test]
fn allay_flying_pose_differs_from_idle() {
    // An idle allay (walkAnimationSpeed 0) bobs vertically and holds its wings/arms; a flying
    // allay (walkAnimationSpeed ≥ 0.3) tilts its body, flattens its wings, and stops bobbing.
    let idle = EntityModelInstance::allay(803, [0.0, 64.0, 0.0], 0.0);
    let flying = idle.with_walk_animation(0.0, 0.3);
    let idle_mesh = entity_model_mesh(&[idle]);
    let flying_mesh = entity_model_mesh(&[flying]);
    assert_eq!(idle_mesh.vertices.len(), flying_mesh.vertices.len());
    assert_ne!(
        idle_mesh.vertices, flying_mesh.vertices,
        "the flying pose tilts the body and flattens the wings"
    );
}

#[test]
fn allay_dance_pose_differs_from_idle_and_spins() {
    // A dancing allay (`DATA_DANCING`) replaces the head-look with the dance head tilt + body sway
    // (`AllayModel.setupAnim` dance branch); entering the spin sub-window whirls the whole root by
    // `4π·spinningProgress`. Same age for all three so only the dance state differs.
    let base = EntityModelInstance::allay(804, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(7.0);
    let idle = entity_model_mesh(&[base]);

    // Swaying (dancing, not yet spinning): the body sways and the head tilts away from the idle pose.
    let swaying = entity_model_mesh(&[base.with_allay_dancing(true)]);
    assert_eq!(idle.vertices.len(), swaying.vertices.len());
    assert_ne!(
        idle.vertices, swaying.vertices,
        "the dance sways the body and tilts the head"
    );

    // Spinning: the same dance frame inside the spin sub-window whirls the root by `4π·0.25 = π`.
    let spinning = entity_model_mesh(&[base
        .with_allay_dancing(true)
        .with_allay_spinning(true)
        .with_allay_spinning_progress(0.25)]);
    assert_eq!(swaying.vertices.len(), spinning.vertices.len());
    assert_ne!(
        swaying.vertices, spinning.vertices,
        "the spin whirls the whole model"
    );
}

#[test]
fn allay_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Allay;
    assert_eq!(kind.model_key(), "allay");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/allay/allay.png",
            size: [32, 32],
        })
    );
}

#[test]
fn allay_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&allay_texture_images()).unwrap();
    // Vanilla `AllayModel` constructs with `RenderTypes::entityTranslucent`. The backend folds that
    // into the translucent mesh, but the submission must keep the vanilla render type, texture, tint,
    // transform, and default collector order.
    let base = EntityModelInstance::allay(850, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((15_u32 << 4) | (8_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(submit.texture, ALLAY_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(base));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.light, base.render_state.shader_light());
    assert_eq!(submit.overlay, base.render_state.overlay_coords());

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
    let looking = entity_model_textured_meshes(&[base.with_head_look(40.0, -25.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, looking.translucent.vertices);

    // The wings flap and the arms bob as the age advances.
    let flapping = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, flapping.translucent.vertices);

    // The flying pose (walkAnimationSpeed ≥ 0.3) differs from the idle pose.
    let flying = entity_model_textured_meshes(&[base.with_walk_animation(0.0, 0.3)], &atlas);
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
    );

    assert_eq!(
        meshes
            .submissions
            .iter()
            .map(|submit| (submit.render_type, submit.order, submit.submit_sequence))
            .collect::<Vec<_>>(),
        vec![
            (EntityModelLayerRenderType::EntityTranslucent, 0, 0),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 0),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 0),
            (EntityModelLayerRenderType::BreezeWind, 1, 1),
            (EntityModelLayerRenderType::EntityTranslucentEmissive, 1, 2,),
            (EntityModelLayerRenderType::EntityTranslucent, 0, 0),
        ],
        "vanilla submit order stays recorded before GPU draw-plan sorting"
    );

    let draws = &meshes.sorted_translucent_draws;
    assert_eq!(draws.len(), 5);
    assert!(draws
        .iter()
        .all(|draw| draw.atlas == EntityModelTexturedDrawAtlas::Static));
    assert_eq!(
        draws
            .iter()
            .map(|draw| (draw.render_type, draw.order, draw.insertion_index))
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
    assert!(draws[0].distance_sq > draws[1].distance_sq);
    assert!(draws[1].distance_sq > draws[2].distance_sq);
    assert!((draws[2].distance_sq - draws[3].distance_sq).abs() < 1.0e-5);
    assert!(
        draws[4].distance_sq > draws[2].distance_sq,
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
                    draw.order,
                    draw.insertion_index,
                    "textured",
                ),
                EntityModelTranslucentDrawRange::Scroll(draw) =>
                    (draw.render_type, draw.order, draw.insertion_index, "scroll",),
                EntityModelTranslucentDrawRange::AdditiveScroll(draw) => (
                    draw.render_type,
                    draw.order,
                    draw.insertion_index,
                    "additive_scroll",
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
            (EntityModelLayerRenderType::BreezeWind, 1, 3, "scroll"),
            (
                EntityModelLayerRenderType::EntityTranslucentEmissive,
                1,
                4,
                "textured",
            ),
        ],
        "main translucent draw plan preserves BreezeRenderer's order(1) WindLayer before EyesLayer"
    );
}

fn allay_texture_images() -> Vec<EntityModelTextureImage> {
    allay_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn allay_and_breeze_texture_images() -> Vec<EntityModelTextureImage> {
    allay_entity_texture_refs()
        .iter()
        .chain(breeze_entity_texture_refs())
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
