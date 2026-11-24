use super::*;
use crate::entity_models::model_layers::armadillo_model;

#[test]
fn armadillo_clamps_the_head_look_to_vanilla_bounds() {
    // Vanilla `ArmadilloModel.setupAnim` (not hiding) clamps the look: pitch (`xRot`) to [-22.5, 25]
    // and yaw (`yRot`) to [-32.5, 32.5] degrees before assigning `head.xRot/yRot`.
    assert_eq!(armadillo_clamped_head_look(50.0, 40.0), (32.5, 25.0));
    assert_eq!(armadillo_clamped_head_look(-50.0, -40.0), (-32.5, -22.5));
    assert_eq!(armadillo_clamped_head_look(10.0, 5.0), (10.0, 5.0));
}

#[test]
fn armadillo_setup_selects_age_specific_shell_and_ear_channels() {
    let adult = armadillo_model(
        false,
        true,
        &EntityModelInstance::armadillo(812, [0.0, 64.0, 0.0], 0.0, false, true)
            .with(|s| s.kind.armadillo_mut().roll_up_seconds = 0.4583),
    );
    let adult_cube = adult.try_child("cube").expect("adult shell cube");
    assert!(
        (adult_cube.pose.offset[1] - (layer_pose(ModelLayer::ARMADILLO, "cube").offset[1] - 0.6))
            .abs()
            < 1.0e-4
    );
    assert!((adult_cube.pose.rotation[0] - 5.0_f32.to_radians()).abs() < 1.0e-4);
    let baby = armadillo_model(
        true,
        true,
        &EntityModelInstance::armadillo(813, [0.0, 64.0, 0.0], 0.0, true, true)
            .with(|s| s.kind.armadillo_mut().roll_out_seconds = 0.15),
    );
    let baby_left_ear = baby
        .try_child("body")
        .and_then(|body| body.try_child("head"))
        .and_then(|head| head.try_child("head_cube"))
        .and_then(|head_cube| head_cube.try_child("left_ear"))
        .expect("baby left ear");
    assert!(
        (baby_left_ear.pose.offset[1]
            - (layer_pose(ModelLayer::ARMADILLO_BABY, "left_ear").offset[1] + 1.4129))
            .abs()
            < 1.0e-4
    );
    assert!(
        (baby_left_ear.pose.offset[2]
            - (layer_pose(ModelLayer::ARMADILLO_BABY, "left_ear").offset[2] + 0.0617))
            .abs()
            < 1.0e-4
    );
}

#[test]
fn armadillo_rolling_re_poses_off_the_bind_pose_before_the_ball_takes_over() {
    // During ROLLING's first ~5 ticks the armadillo is NOT yet hiding (`shouldHideInShell` flips at
    // tick 5), so it renders the full rest tree with the roll-up keyframe ADDED onto the walk/bind
    // pose — the body curls in. A rolling armadillo therefore re-poses off the bind pose without
    // changing the cube count (10 cubes / 240 vertices).
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let rolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        83,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with(|s| s.kind.armadillo_mut().roll_up_seconds = 0.2)]);
    assert_eq!(
        rest.vertices.len(),
        rolling.vertices.len(),
        "the roll-up re-poses parts, it does not add or hide cubes"
    );
    assert_ne!(
        rest.vertices, rolling.vertices,
        "the rolling armadillo curls off the bind pose before the ball takes over"
    );
    // Advancing the roll-up's elapsed seconds advances the curl.
    let rolling_later = entity_model_mesh(&[EntityModelInstance::armadillo(
        84,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with(|s| s.kind.armadillo_mut().roll_up_seconds = 0.4)]);
    assert_ne!(
        rolling.vertices, rolling_later.vertices,
        "the curl-in advances as the roll-up elapsed seconds climb"
    );
}

#[test]
fn armadillo_unrolling_re_poses_off_the_bind_pose() {
    // Once UNROLLING un-hides (inStateTicks >= 26) the rest tree shows again with the roll-out
    // keyframe ADDED, so the body un-curls off the bind pose (same cube count).
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        85,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let unrolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        86,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with(|s| s.kind.armadillo_mut().roll_out_seconds = 1.35)]);
    assert_eq!(rest.vertices.len(), unrolling.vertices.len());
    assert_ne!(
        rest.vertices, unrolling.vertices,
        "the unrolling armadillo un-curls off the bind pose"
    );
}

#[test]
fn armadillo_hidden_applies_visible_roll_keyframes() {
    // While `isHidingInShell` (the `rolled_up` tree), vanilla still applies roll-up / roll-out
    // keyframes after the visibility swap. bbb's rolled tree omits the hidden body/tail/hind legs,
    // while the visible head, front legs, and shell `cube` receive their exact channels.
    let hidden = entity_model_mesh(&[EntityModelInstance::armadillo(
        87,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    let hidden_rolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        88,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )
    .with(|s| s.kind.armadillo_mut().roll_up_seconds = 0.2)]);
    // Six cubes → 36 faces / 144 vertices: the shell ball, head, ears, two front legs.
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.vertices.len(), hidden_rolling.vertices.len());
    assert_ne!(
        hidden.vertices, hidden_rolling.vertices,
        "hidden armadillo roll keyframes move the visible head/front legs"
    );
}

#[test]
fn armadillo_textured_render_matches_vanilla_renderer() {
    // The adult and baby armadillo share the UV layout but bind their own 64×64 textures.
    for (baby, texture) in [
        (false, ARMADILLO_TEXTURE_REF),
        (true, ARMADILLO_BABY_TEXTURE_REF),
    ] {
        let passes = armadillo_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
    }
    assert!(entity_model_texture_refs().contains(&ARMADILLO_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ARMADILLO_BABY_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [ARMADILLO_TEXTURE_REF, ARMADILLO_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // Both ages, both rolled/unrolled, emit textured geometry through a vanilla-shaped submission
    // before folding into the cutout mesh.
    for baby in [false, true] {
        for rolled_up in [false, true] {
            let instance =
                EntityModelInstance::armadillo(980, [0.0, 64.0, 0.0], 15.0, baby, rolled_up).with(
                    |s| {
                        s.kind.armadillo_mut().peek_seconds = if rolled_up { 2.15 } else { 0.5 };
                        s.light_coords = (5_u32 << 4) | (10_u32 << 20);
                        s.white_overlay_progress = 0.8;
                        s.has_red_overlay = true;
                    },
                );
            let meshes = entity_model_textured_meshes(&[instance], &atlas);
            assert!(
                !meshes.cutout.vertices.is_empty(),
                "baby={baby} rolled_up={rolled_up} emits textured geometry"
            );
            assert!(meshes
                .cutout
                .vertices
                .iter()
                .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                    && vertex.light == instance.render_state.shader_light()
                    && vertex.overlay == instance.render_state.overlay_coords()));
        }
    }
}
