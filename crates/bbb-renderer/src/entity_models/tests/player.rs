use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

/// The wide-player limb rest poses, for the desc-level arm-swing/bob reference-formula tests (the
/// player now builds a named tree, so it has no `*_PARTS` desc const). Right arm `x = -5`, left arm
/// `x = +5`, right leg `x = -1.9` — the vanilla `PlayerModel.createMesh` offsets.
const PLAYER_FIXTURE_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

#[test]
fn player_model_parts_match_vanilla_26_1_body_layers() {
    // The player builds a named-children tree (each base part nests one skin overlay child:
    // `head` -> `hat`, `body` -> `jacket`, the arms -> `sleeve`, the legs -> `pants`), so the head
    // look resolves the `head` child by name and the visibility toggles resolve the overlays by name.
    // The geometry is asserted on the per-part unified cube consts (colored tint + textured
    // uv/tex/mirror); the wide and slim layouts differ only in the arm/sleeve widths.
    assert_eq!(PLAYER_HEAD[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(PLAYER_HAT[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_BODY[0].size, [8.0, 12.0, 4.0]);
    assert_eq!(PLAYER_JACKET[0].size, [8.5, 12.5, 4.5]);
    assert_eq!(PLAYER_JACKET[0].uv_size, [8.0, 12.0, 4.0]);
    // Wide arms/sleeves are 4 wide; slim arms/sleeves are 3 wide.
    assert_eq!(PLAYER_WIDE_RIGHT_ARM[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].size, [4.5, 12.5, 4.5]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].uv_size, [4.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].size, [3.5, 12.5, 4.5]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(
        PLAYER_RIGHT_PANTS[0],
        ModelCube::new(
            [-2.25, -0.25, -2.25],
            [4.5, 12.5, 4.5],
            PLAYER_BLUE,
            [4.0, 12.0, 4.0],
            [0.0, 32.0],
            false,
        )
    );
    assert_eq!(PLAYER_LEFT_PANTS[0].size, [4.5, 12.5, 4.5]);
}

#[test]
fn player_mesh_uses_vanilla_body_layer_geometry_and_avatar_scale() {
    let wide = entity_model_mesh(&[EntityModelInstance::player(
        155,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let slim = entity_model_mesh(&[EntityModelInstance::player(
        156,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.opaque_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PLAYER_BLUE, 0.78)));
    }

    let (wide_min, wide_max) = mesh_extents(&wide);
    let (slim_min, slim_max) = mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_texture_refs_match_vanilla_default_assets() {
    let cases = [
        (
            false,
            "player",
            EntityModelTextureRef {
                path: "textures/entity/player/wide/steve.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "player_slim",
            EntityModelTextureRef {
                path: "textures/entity/player/slim/steve.png",
                size: [64, 64],
            },
        ),
    ];

    for (slim, model_key, texture) in cases {
        let kind = EntityModelKind::Player {
            slim,
            parts: PLAYER_MODEL_PARTS_ALL_VISIBLE,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn player_model_part_visibility_masks_match_vanilla_player_model_part_bits() {
    assert_eq!(PlayerModelPartVisibility::CAPE_MASK, 1 << 0);
    assert_eq!(PlayerModelPartVisibility::JACKET_MASK, 1 << 1);
    assert_eq!(PlayerModelPartVisibility::LEFT_SLEEVE_MASK, 1 << 2);
    assert_eq!(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK, 1 << 3);
    assert_eq!(PlayerModelPartVisibility::LEFT_PANTS_MASK, 1 << 4);
    assert_eq!(PlayerModelPartVisibility::RIGHT_PANTS_MASK, 1 << 5);
    assert_eq!(PlayerModelPartVisibility::HAT_MASK, 1 << 6);
    assert_eq!(PlayerModelPartVisibility::ALL_MASK, 0x7f);
    assert_eq!(
        PLAYER_MODEL_PARTS_ALL_VISIBLE.vanilla_mask(),
        PlayerModelPartVisibility::ALL_MASK
    );
    assert_eq!(PLAYER_MODEL_PARTS_ALL_HIDDEN.vanilla_mask(), 0);

    let mask = PlayerModelPartVisibility::HAT_MASK
        | PlayerModelPartVisibility::JACKET_MASK
        | PlayerModelPartVisibility::LEFT_SLEEVE_MASK
        | PlayerModelPartVisibility::RIGHT_PANTS_MASK;
    let parts = PlayerModelPartVisibility::from_vanilla_mask(mask);
    assert!(parts.hat);
    assert!(parts.jacket);
    assert!(parts.left_sleeve);
    assert!(!parts.right_sleeve);
    assert!(!parts.left_pants);
    assert!(parts.right_pants);
    assert!(!parts.cape);
    assert_eq!(parts.vanilla_mask(), mask);
}

#[test]
fn player_textured_layer_passes_match_vanilla_avatar_renderer_model_layers() {
    let wide = player_textured_layer_passes(false, PLAYER_MODEL_PARTS_ALL_VISIBLE);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(wide[0].model_layer, MODEL_LAYER_PLAYER);
    assert_eq!(wide[0].texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    // The unified `PlayerModel` tree drives the geometry, so the layer-pass parts are vestigial; the
    // part visibility still rides the pass to the renderer.
    assert_eq!(
        wide[0].visibility,
        EntityModelLayerVisibility::PlayerParts(PLAYER_MODEL_PARTS_ALL_VISIBLE)
    );
    assert_eq!(wide[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wide[0].collector_order, wide[0].submit_sequence), (0, 0));

    let slim_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    let slim = player_textured_layer_passes(true, slim_parts);
    assert_eq!(slim.len(), 1);
    assert_eq!(slim[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(slim[0].model_layer, MODEL_LAYER_PLAYER_SLIM);
    assert_eq!(slim[0].texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(
        slim[0].visibility,
        EntityModelLayerVisibility::PlayerParts(slim_parts)
    );
    assert_eq!(slim[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slim[0].collector_order, slim[0].submit_sequence), (0, 0));
}

#[test]
fn player_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PLAYER, "minecraft:player#main");
    assert_eq!(MODEL_LAYER_PLAYER_SLIM, "minecraft:player_slim#main");
    // The player UVs are now carried on the unified cubes' `.tex` field; the overlay cubes keep the
    // base box as uv_size.
    assert_eq!(PLAYER_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(PLAYER_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].tex, [32.0, 0.0]);
    assert_eq!(PLAYER_HAT[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(PLAYER_BODY[0].tex, [16.0, 16.0]);
    assert_eq!(PLAYER_JACKET[0].tex, [16.0, 32.0]);
    assert_eq!(PLAYER_JACKET[0].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(PLAYER_JACKET[0].size, [8.5, 12.5, 4.5]);
    assert_eq!(PLAYER_WIDE_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert_eq!(PLAYER_WIDE_LEFT_ARM[0].tex, [32.0, 48.0]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].tex, [40.0, 32.0]);
    assert_eq!(PLAYER_WIDE_LEFT_SLEEVE[0].tex, [48.0, 48.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_LEFT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_LEFT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert_eq!(PLAYER_LEFT_LEG[0].tex, [16.0, 48.0]);
    assert_eq!(PLAYER_RIGHT_PANTS[0].tex, [0.0, 32.0]);
    assert_eq!(PLAYER_LEFT_PANTS[0].tex, [0.0, 48.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_player_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/player/wide/steve.png",
            "textures/entity/player/slim/steve.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    let slim_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(&rgba[0..4], &[0; 4]);
    assert_eq!(&rgba[slim_first_pixel..slim_first_pixel + 4], &[1; 4]);
}

#[test]
fn player_textured_mesh_uses_vanilla_uvs_tints_and_avatar_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let wide = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            901,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    let slim = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            902,
            [0.0, 64.0, 0.0],
            0.0,
            true,
        )],
        &atlas,
    );

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.cutout_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
    assert_close2(wide.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(slim.vertices[0].uv, [16.0 / 64.0, 0.5]);

    let (wide_min, wide_max) = textured_mesh_extents(&wide);
    let (slim_min, slim_max) = textured_mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_textured_mesh_applies_vanilla_model_part_visibility_to_overlay_parts() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let hidden = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            903,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_HIDDEN,
        )],
        &atlas,
    );
    assert_eq!(hidden.cutout_faces, 36);
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.indices.len(), 216);

    let partial_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let partial = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            904,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            partial_parts,
        )],
        &atlas,
    );
    assert_eq!(partial.cutout_faces, 48);
    assert_eq!(partial.vertices.len(), 192);
    assert_eq!(partial.indices.len(), 288);
    assert!(partial
        .vertices
        .iter()
        .any(|vertex| vertex.uv[1] >= 32.0 / 64.0));
}

#[test]
fn player_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(903, [0.0, 64.0, 0.0], 0.0, slim);
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);

        // Head look turns the head part (index 0, shared across all passes)
        // without changing the vertex count.
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "slim={slim}");
        assert_ne!(yawed.vertices, pitched.vertices, "slim={slim}");
    }
}

#[test]
fn player_attack_swing_twists_body_and_whacks_the_swinging_arm() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` (WHACK), at attackTime `t` with head pitch `p`:
    // the body twists `yRot = sin(sqrt(t) · 2π) · 0.2`, the arm anchors swing around it, and the
    // attacking arm whacks down. `setupAttackAnimation` runs last, so it accumulates onto the idle pose.
    let t = 0.4_f32;
    let pitch = -10.0_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let base =
        EntityModelInstance::player(900, [0.0, 64.0, 0.0], 0.0, false).with_head_look(0.0, pitch);

    // Baseline: a non-swinging player keeps the body untwisted and the arms on the idle pose.
    let mut resting = PlayerModel::new(false);
    resting.prepare(&base);
    let resting_right_xrot = resting.root_mut().child_mut("right_arm").pose.rotation[0];
    let resting_left_xrot = resting.root_mut().child_mut("left_arm").pose.rotation[0];
    assert_eq!(resting.root_mut().child_mut("body").pose.rotation[1], 0.0);

    // Main-hand swing twists the body and whacks the RIGHT arm.
    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_attack_anim(t));
    assert!(
        (main.root_mut().child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body twists with the swing"
    );
    let right = main.root_mut().child_mut("right_arm");
    // The arm anchor swings around the twisting body (vanilla `rightArm.x/z`), overwriting the bind.
    assert!((right.pose.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!((right.pose.offset[2] - body_yrot.sin() * 5.0).abs() < 1e-6);
    // The whack drives the right arm well forward of its idle pitch.
    assert!(
        right.pose.rotation[0] < resting_right_xrot - 0.8,
        "the main hand whacks down: {} vs resting {resting_right_xrot}",
        right.pose.rotation[0]
    );
    // The off (left) arm is not whacked — only the shared body twist adds to its xRot.
    let main_left_xrot = main.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (main_left_xrot - (resting_left_xrot + body_yrot)).abs() < 1e-6,
        "the idle off arm only picks up the body twist on xRot"
    );

    // Off-hand swing negates the body twist and whacks the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_attack_anim(t).with_attack_arm_off_hand(true));
    assert!(
        (off.root_mut().child_mut("body").pose.rotation[1] - (-body_yrot)).abs() < 1e-6,
        "the off-hand swing negates the body twist"
    );
    assert!(
        off.root_mut().child_mut("left_arm").pose.rotation[0] < resting_left_xrot - 0.8,
        "the off hand whacks the left arm"
    );
    // The idle right arm in an off-hand swing is not whacked on its pitch (only yRot picks up the twist).
    assert!(
        (off.root_mut().child_mut("right_arm").pose.rotation[0] - resting_right_xrot).abs() < 1e-6,
        "the idle main arm keeps its pitch during an off-hand swing"
    );
}

#[test]
fn player_with_a_spear_lunges_instead_of_whacking() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` `STAB` (`SpearAnimations.thirdPersonAttackHand`): the
    // shared body twist + arm anchors run, but the attacking arm LUNGES on its pitch
    // (`xRot += (90·prepare − 120·attack + 30·retract)·π/180`) instead of whacking, and the body-twist
    // contributions on the arm rotations are undone (the off arm keeps its resting pitch).
    let t = 0.1_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let progress = |t: f32, a: f32, b: f32| ((t - a) / (b - a)).clamp(0.0, 1.0);
    let in_out_sine = |x: f32| -((PI * x).cos() - 1.0) / 2.0;
    let in_out_expo = |x: f32| {
        if x < 0.5 {
            if x == 0.0 {
                0.0
            } else {
                2.0_f32.powf(20.0 * x - 10.0) / 2.0
            }
        } else if x == 1.0 {
            1.0
        } else {
            (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
        }
    };
    let stab = {
        let prepare = in_out_sine(progress(t, 0.0, 0.05));
        let attack = progress(t, 0.05, 0.2).powi(2);
        let retract = in_out_expo(progress(t, 0.4, 1.0));
        (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians()
    };

    let base =
        EntityModelInstance::player(905, [0.0, 64.0, 0.0], 0.0, false).with_head_look(0.0, -10.0);
    let mut resting = PlayerModel::new(false);
    resting.prepare(&base);
    let resting_right_xrot = resting.root_mut().child_mut("right_arm").pose.rotation[0];
    let resting_left_xrot = resting.root_mut().child_mut("left_arm").pose.rotation[0];

    // A spear-swing twists the body the same way the whack does, but lunges the right arm.
    let spear = base.with_attack_anim(t).with_main_hand_swing_is_stab(true);
    let mut model = PlayerModel::new(false);
    model.prepare(&spear);
    assert!(
        (model.root_mut().child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body still twists with the swing"
    );
    let right = model.root_mut().child_mut("right_arm");
    let right_offset0 = right.pose.offset[0];
    let right_xrot = right.pose.rotation[0];
    assert!((right_offset0 - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!(
        (right_xrot - (resting_right_xrot + stab)).abs() < 1e-6,
        "the main arm lunges by the stab term: {right_xrot} vs {}",
        resting_right_xrot + stab
    );

    // Unlike the whack, the stab undoes the prologue's body-twist add on the off arm's pitch — it stays
    // at its resting pitch.
    let left_xrot = model.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (left_xrot - resting_left_xrot).abs() < 1e-6,
        "the off arm keeps its resting pitch during a stab (no body-twist add)"
    );

    // The same swing time with a non-spear (WHACK) drives a visibly different attacking-arm pitch.
    let mut whack = PlayerModel::new(false);
    whack.prepare(&base.with_attack_anim(t));
    let whack_right_xrot = whack.root_mut().child_mut("right_arm").pose.rotation[0];
    assert!(
        (whack_right_xrot - right_xrot).abs() > 0.3,
        "the spear lunge differs from the whack chop: stab {right_xrot} vs whack {whack_right_xrot}"
    );
}

#[test]
fn player_using_a_spyglass_raises_it_to_the_eye() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `SPYGLASS`: the holding arm raises along the
    // head look — `xRot = clamp(head.xRot − 1.9198622 − (crouch?π/12), −2.4, 3.3)`, `yRot = head.yRot ∓
    // π/12` — and skips the idle bob (`zRot` back to bind). Applied before crouch, so a crouching player's
    // `arm.xRot += 0.4` still lands on top.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base =
        EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, false).with_head_look(yaw, pitch);

    // Main-hand spyglass raises the RIGHT arm and removes its bob roll.
    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_using_spyglass(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the right arm pitches up to the eye: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 12.0)).abs() < 1e-6,
        "the right arm yaws −π/12 off the head: {}",
        right.rotation[1]
    );
    assert_eq!(
        right.rotation[2], 0.0,
        "the spyglass arm skips the idle bob"
    );

    // Off-hand spyglass raises the LEFT arm with the mirrored yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_using_spyglass(true)
            .with_use_item_off_hand(true),
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the left arm pitches up to the eye: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 12.0)).abs() < 1e-6,
        "the left arm yaws +π/12 off the head"
    );

    // Crouching adds the crouch term to the clamp AND the crouch block's +0.4 on top (pose runs first).
    let mut crouch = PlayerModel::new(false);
    crouch.prepare(
        &base
            .with_player_using_spyglass(true)
            .with_is_crouching(true),
    );
    let crouch_right_x = crouch.root_mut().child_mut("right_arm").pose.rotation[0];
    let expected = (pitch_rad - 1.9198622 - PI / 12.0).clamp(-2.4, 3.3) + 0.4;
    assert!(
        (crouch_right_x - expected).abs() < 1e-6,
        "a crouching spyglass arm clamps with the crouch term, then gains +0.4: {crouch_right_x} vs {expected}"
    );
}

#[test]
fn player_tooting_a_goat_horn_raises_it_to_the_mouth() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `TOOT_HORN`: the holding arm raises the horn —
    // `xRot = clamp(head.xRot, −1.2, 1.2) − 1.4835298`, `yRot = head.yRot ∓ π/6`. Unlike the spyglass it
    // keeps the idle bob, so the arm roll (`zRot`) is left non-zero.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base =
        EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, false).with_head_look(yaw, pitch);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_tooting_horn(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad.clamp(-1.2, 1.2) - 1.4835298)).abs() < 1e-6,
        "the right arm raises the horn to the mouth: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 6.0)).abs() < 1e-6,
        "the right arm yaws −π/6 off the head: {}",
        right.rotation[1]
    );
    assert_ne!(
        right.rotation[2], 0.0,
        "the horn arm keeps the idle bob roll (unlike the spyglass)"
    );

    // Off-hand horn raises the LEFT arm with the mirrored yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_tooting_horn(true)
            .with_use_item_off_hand(true),
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 6.0)).abs() < 1e-6,
        "the left arm yaws +π/6 off the head"
    );
}

#[test]
fn player_brushing_lowers_the_arm_to_the_block() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `BRUSH`: the holding arm lowers — `xRot =
    // arm.xRot · 0.5 − π/5`, `yRot = 0`. At rest (age 0, not walking) the arm pitch is `0`, so the brush
    // pose lands the arm at exactly `−π/5`.
    let base =
        EntityModelInstance::player(922, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_brushing(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the right arm lowers to −π/5: {}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the brush pose zeroes the arm yaw");

    // A non-brushing player keeps its (much higher) idle arm pitch — the brush visibly lowers the arm.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI / 5.0 + 0.3,
        "an idle player does not lower the arm to the block"
    );

    // Off-hand brushing lowers the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_brushing(true).with_use_item_off_hand(true));
    assert!(
        (off.root_mut().child_mut("left_arm").pose.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the off hand lowers the left arm"
    );
}

#[test]
fn player_holding_an_item_lowers_the_main_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` (`HumanoidModel.poseRightArm` ITEM case): a player
    // holding a plain main-hand item lowers/halves the arm — `xRot = arm.xRot · 0.5 − π/10`, `yRot = 0`. At
    // rest (age 0, not walking) the arm pitch is `0`, so the ITEM pose lands the right arm at exactly `−π/10`.
    let base =
        EntityModelInstance::player(930, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut held = PlayerModel::new(false);
    held.prepare(&base.with_player_main_hand_item_pose(true));
    let right = held.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 10.0)).abs() < 1e-6,
        "the right arm lowers to −π/10: {}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the ITEM pose zeroes the arm yaw");

    // An empty-handed player keeps its (much higher) idle arm pitch — holding an item visibly lowers it.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI / 10.0 + 0.3,
        "an empty-handed player does not lower the arm"
    );

    // The main-hand ITEM pose is right-arm only; the left arm is untouched.
    let mut held_left = PlayerModel::new(false);
    held_left.prepare(&base.with_player_main_hand_item_pose(true));
    let left = held_left.root_mut().child_mut("left_arm").pose;
    let mut bare_left = PlayerModel::new(false);
    bare_left.prepare(&base);
    assert_eq!(
        left.rotation,
        bare_left.root_mut().child_mut("left_arm").pose.rotation,
        "the main-hand ITEM pose leaves the off (left) arm alone"
    );
}

#[test]
fn player_holding_an_off_hand_item_lowers_the_left_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` (`HumanoidModel.poseLeftArm` ITEM
    // case): a player holding a plain off-hand item lowers/halves the OFF (left) arm — `xRot = arm.xRot ·
    // 0.5 − π/10`, `yRot = 0`. At rest the arm pitch is `0`, so the left arm lands at exactly `−π/10`.
    let base =
        EntityModelInstance::player(940, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut held = PlayerModel::new(false);
    held.prepare(&base.with_player_off_hand_item_pose(true));
    let left = held.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 10.0)).abs() < 1e-6,
        "the left arm lowers to −π/10: {}",
        left.rotation[0]
    );
    assert_eq!(
        left.rotation[1], 0.0,
        "the off-hand ITEM pose zeroes the arm yaw"
    );

    // The off-hand ITEM pose is left-arm only; the right (main) arm is untouched.
    let right = held.root_mut().child_mut("right_arm").pose;
    let mut bare = PlayerModel::new(false);
    bare.prepare(&base);
    assert_eq!(
        right.rotation,
        bare.root_mut().child_mut("right_arm").pose.rotation,
        "the off-hand ITEM pose leaves the main (right) arm alone"
    );
}

#[test]
fn player_raising_a_shield_tucks_the_arm_along_the_head_look() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseBlockingArm` `BLOCK`: the raising arm tucks the shield forward —
    // `xRot = arm.xRot · 0.5 − 0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)`,
    // `yRot = (right ? −π/6 : π/6) + clamp(head.yRot, −π/6, π/6)`. With a head look of `(yaw 10°, pitch −10°)`
    // both clamps pass through, and at rest (age 0) the arm pitch is `0`.
    let yaw = 10.0_f32.to_radians();
    let pitch = (-10.0_f32).to_radians();
    let base =
        EntityModelInstance::player(950, [0.0, 64.0, 0.0], 0.0, false).with_head_look(10.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_blocking(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-0.9424779 + pitch)).abs() < 1e-5,
        "the right arm pitch tucks to −0.9424779 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-PI / 6.0 + yaw)).abs() < 1e-5,
        "the right arm yaws −π/6 + head.yRot: {}",
        right.rotation[1]
    );

    // An idle (non-blocking) player keeps its much higher arm pitch — the shield visibly tucks the arm down.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -0.9424779 + pitch + 0.3,
        "an idle player does not tuck the arm"
    );

    // Off-hand blocking tucks the LEFT arm instead, with the +π/6 base yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_blocking(true).with_use_item_off_hand(true));
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (PI / 6.0 + yaw)).abs() < 1e-5,
        "the off hand yaws +π/6 + head.yRot: {}",
        left.rotation[1]
    );
}

#[test]
fn player_charging_a_trident_raises_the_arm_overhead() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `THROW_TRIDENT`: the holding arm raises the trident
    // straight overhead — `xRot = arm.xRot · 0.5 − π`, `yRot = 0`. At rest (age 0) the arm pitch is `0`, so
    // it lands at exactly `−π`.
    let base =
        EntityModelInstance::player(960, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_throwing_trident(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI)).abs() < 1e-6,
        "the right arm raises to −π: {}",
        right.rotation[0]
    );
    assert_eq!(
        right.rotation[1], 0.0,
        "the THROW_TRIDENT pose zeroes the arm yaw"
    );

    // An idle player keeps its much higher arm pitch — charging the throw visibly raises the arm overhead.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI + 0.5,
        "an idle player does not raise the arm overhead"
    );

    // Off-hand charging raises the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_throwing_trident(true)
            .with_use_item_off_hand(true),
    );
    assert!(
        (off.root_mut().child_mut("left_arm").pose.rotation[0] - (-PI)).abs() < 1e-6,
        "the off hand raises the left arm overhead"
    );
}

#[test]
fn player_swings_its_legs_when_walking() {
    // `PlayerModel extends HumanoidModel` and its `setupAnim` only toggles part
    // visibility before `super.setupAnim`, so a remote player inherits the
    // `HumanoidModel` legs unchanged (legs at [4, 5], the right leg in phase). A
    // standing player is inert; a walking one lifts its feet and splays its legs
    // along Z, for both the wide and slim arm models. This is the colored path.
    for slim in [false, true] {
        let base = EntityModelInstance::player(910, [0.0, 64.0, 0.0], 0.0, slim);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "slim={slim}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "slim={slim}: walking differs"
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "slim={slim}: a walking player's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.1,
            "slim={slim}: a walking player's legs should splay along Z"
        );
    }
}

#[test]
fn player_textured_mesh_swings_legs_when_walking() {
    // The real player render path (texture-backed) swings the inherited
    // `HumanoidModel` legs (and the pants children that ride them) on the shared
    // visibility-filtered part array. A standing player is byte-identical however
    // far the swing position has advanced; a walking one lifts its feet. Checked
    // with all model parts visible (pants present) and with the pants hidden.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let no_pants = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::ALL_MASK
            & !PlayerModelPartVisibility::LEFT_PANTS_MASK
            & !PlayerModelPartVisibility::RIGHT_PANTS_MASK,
    );
    for slim in [false, true] {
        for (label, base) in [
            (
                "all_parts",
                EntityModelInstance::player(911, [0.0, 64.0, 0.0], 0.0, slim),
            ),
            (
                "no_pants",
                EntityModelInstance::player_with_parts(912, [0.0, 64.0, 0.0], 0.0, slim, no_pants),
            ),
        ] {
            let resting = entity_model_textured_mesh(&[base], &atlas);
            let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
            let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

            assert_eq!(
                resting.vertices, still.vertices,
                "slim={slim} {label}: a standing player is inert"
            );
            assert_eq!(
                resting.vertices.len(),
                walking.vertices.len(),
                "slim={slim} {label}: leg swing keeps the vertex count"
            );
            assert_ne!(
                resting.vertices, walking.vertices,
                "slim={slim} {label}: a walking player differs"
            );

            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
                "slim={slim} {label}: a walking player's feet should lift off the ground"
            );
        }
    }
}

#[test]
fn player_swings_its_arms_when_walking() {
    // `PlayerModel` inherits the `HumanoidModel` arm swing unchanged. With all overlays
    // visible the colored mesh emits head+body (verts 0..96), the two arms with their
    // sleeves (96..192), then the two legs with their pants (192..288). A standing player
    // is inert; a walking one moves the arms (and legs) while the head and body stay put
    // (no head look here). The held-item/attack arm poses and the idle bob are deferred.
    for slim in [false, true] {
        let base = EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, slim);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(rest.vertices.len(), 288, "slim={slim}");
        assert_eq!(rest.vertices.len(), walking.vertices.len(), "slim={slim}");
        assert_eq!(
            rest.vertices[0..96],
            walking.vertices[0..96],
            "slim={slim}: head and body stay put while walking"
        );
        assert_ne!(
            rest.vertices[96..192],
            walking.vertices[96..192],
            "slim={slim}: the arms swing"
        );
    }
}

#[test]
fn player_textured_mesh_swings_its_arms_when_walking() {
    // The real (texture-backed) player render path swings the arms on the same shared
    // visibility-filtered part array as the legs; the sleeve children ride the arm parts.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);
        assert_eq!(
            resting.vertices, still.vertices,
            "slim={slim}: inert when standing"
        );
        assert_eq!(resting.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            resting.vertices[0..96],
            walking.vertices[0..96],
            "slim={slim}: head and body stay put"
        );
        assert_ne!(
            resting.vertices[96..192],
            walking.vertices[96..192],
            "slim={slim}: the arms swing"
        );
    }
}

#[test]
fn humanoid_arm_swing_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim: rightArm.xRot = cos(pos*0.6662 + π)*2.0*speed*0.5,
    // leftArm.xRot = cos(pos*0.6662)*2.0*speed*0.5 (amplitude 1.0). The right arm
    // (offset x < 0) is the out-of-phase one, opposite the same-side leg. Only xRot
    // moves. The right arm rests at x = -5, the left at x = +5.
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    let right = humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, speed);
    let left = humanoid_arm_swing_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, pos, speed);
    assert!(
        (right.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "right arm out of phase"
    );
    assert!(
        (left.rotation[0] - phase.cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "left arm in phase"
    );
    // The arm swing is the opposite phase to the same-side leg (right arm uses +π, the
    // right leg uses none) and a shorter amplitude (1.0 vs 1.4).
    let right_leg = humanoid_leg_swing_pose(PLAYER_FIXTURE_RIGHT_LEG_POSE, pos, speed);
    assert!(
        (right.rotation[0] + right_leg.rotation[0] / 1.4).abs() < 1e-6,
        "right arm is the negated, scaled right-leg swing"
    );
    // Only xRot changes.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    assert_eq!(right.rotation[2], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2]);
    // At rest (speed 0) the arms hold their body-layer pose.
    assert_eq!(
        humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, 0.0),
        PLAYER_FIXTURE_RIGHT_ARM_POSE
    );
}

#[test]
fn humanoid_arm_bob_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim applies AnimationUtils.bobModelPart to both arms every
    // frame — bobModelPart(rightArm, age, 1.0), bobModelPart(leftArm, age, -1.0):
    //   arm.zRot += scale * (cos(age * 0.09)  * 0.05 + 0.05)
    //   arm.xRot += scale * (sin(age * 0.067) * 0.05)
    // The right arm rests at x = -5 (bob scale +1), the left at x = +5 (scale -1); the bob
    // accumulates onto the arm's rest pose.
    let age = 27.3_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let right = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, age);
    let left = humanoid_arm_bob_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, age);
    assert!(
        (right.rotation[0] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0] + bob_x)).abs() < 1e-6,
        "right arm bob xRot"
    );
    assert!(
        (right.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + bob_z)).abs() < 1e-6,
        "right arm bob zRot"
    );
    // The left arm uses the opposite sign (scale -1).
    assert!(
        (left.rotation[0] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[0] - bob_x)).abs() < 1e-6,
        "left arm bob xRot mirrored"
    );
    assert!(
        (left.rotation[2] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[2] - bob_z)).abs() < 1e-6,
        "left arm bob zRot mirrored"
    );
    // The bob preserves the offset and yRot.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    // The xRot term vanishes at age 0 (sin 0 = 0) but the zRot baseline does not
    // (cos 0 = 1 gives ±0.1), so the arms never sit at the bare rest pose.
    let at_zero = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, 0.0);
    assert!((at_zero.rotation[0] - PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0]).abs() < 1e-6);
    assert!((at_zero.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + 0.1)).abs() < 1e-6);
}

#[test]
fn player_arms_idle_bob_as_age_advances_even_when_standing() {
    // The HumanoidModel idle arm bob advances every frame regardless of walking, so a
    // standing player's arms move with ageInTicks while the head, body, and legs stay put
    // (no walk swing, no head look). The colored mesh emits head+body (0..96), the two arms
    // with sleeves (96..192), then the two legs with pants (192..288).
    for slim in [false, true] {
        let base = EntityModelInstance::player(930, [0.0, 64.0, 0.0], 0.0, slim);
        let early = entity_model_mesh(&[base]);
        let later = entity_model_mesh(&[base.with_age_in_ticks(27.3)]);
        assert_eq!(early.vertices.len(), 288, "slim={slim}");
        assert_eq!(early.vertices.len(), later.vertices.len(), "slim={slim}");
        assert_eq!(
            early.vertices[0..96],
            later.vertices[0..96],
            "slim={slim}: head and body do not bob"
        );
        assert_ne!(
            early.vertices[96..192],
            later.vertices[96..192],
            "slim={slim}: the arms idle-bob with ageInTicks"
        );
        assert_eq!(
            early.vertices[192..288],
            later.vertices[192..288],
            "slim={slim}: the legs do not bob"
        );
    }
}

#[test]
fn player_textured_arms_idle_bob_as_age_advances() {
    // The texture-backed player path applies the same idle bob on the shared
    // visibility-filtered part array, so a standing player's arms bob with ageInTicks while
    // the head, body, and legs are byte-identical across ages.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(931, [0.0, 64.0, 0.0], 0.0, slim);
        let early = entity_model_textured_mesh(&[base], &atlas);
        let later = entity_model_textured_mesh(&[base.with_age_in_ticks(27.3)], &atlas);
        assert_eq!(early.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            early.vertices[0..96],
            later.vertices[0..96],
            "slim={slim}: head and body do not bob"
        );
        assert_ne!(
            early.vertices[96..192],
            later.vertices[96..192],
            "slim={slim}: the arms idle-bob with ageInTicks"
        );
        assert_eq!(
            early.vertices[192..288],
            later.vertices[192..288],
            "slim={slim}: the legs do not bob"
        );
    }
}

#[test]
fn player_crouches_when_sneaking() {
    // Vanilla `HumanoidModel.setupAnim` crouch (`isCrouching`): the body leans forward
    // (`xRot = 0.5`) and drops (`y += 3.2`), the head drops (`y += 4.2`), the arms tilt
    // (`xRot += 0.4`, `y += 3.2`) and the legs tuck back (`z += 4`). A sneaking player is
    // shorter and leans forward. Colored path here, textured below.
    for slim in [false, true] {
        let base = EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, slim);
        let standing = entity_model_mesh(&[base]);
        let crouching = entity_model_mesh(&[base.with_is_crouching(true)]);
        assert_eq!(standing.vertices.len(), crouching.vertices.len());
        assert_ne!(
            standing.vertices, crouching.vertices,
            "slim={slim}: a sneaking player poses differently"
        );

        let (stand_min, stand_max) = mesh_extents(&standing);
        let (crouch_min, crouch_max) = mesh_extents(&crouching);
        assert!(
            (crouch_max[1] - crouch_min[1]) < (stand_max[1] - stand_min[1]) - 0.1,
            "slim={slim}: a sneaking player is shorter"
        );
        assert!(
            (crouch_max[2] - crouch_min[2]) > (stand_max[2] - stand_min[2]) + 0.1,
            "slim={slim}: a sneaking player leans forward, deepening its Z footprint"
        );

        // A standing player is unaffected by the flag default.
        assert_eq!(
            standing.vertices,
            entity_model_mesh(&[base.with_is_crouching(false)]).vertices,
            "slim={slim}: a standing player is unchanged"
        );
    }
}

#[test]
fn player_textured_mesh_crouches_when_sneaking() {
    // The real player render path (texture-backed) applies the same crouch to the shared
    // visibility-filtered part array (the hat/jacket/sleeve/pants children ride the shifted
    // parts).
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let standing = entity_model_textured_mesh(&[base], &atlas);
        let crouching = entity_model_textured_mesh(&[base.with_is_crouching(true)], &atlas);
        assert_eq!(standing.vertices.len(), crouching.vertices.len());
        assert_ne!(
            standing.vertices, crouching.vertices,
            "slim={slim}: a sneaking player poses differently"
        );

        let (stand_min, stand_max) = textured_mesh_extents(&standing);
        let (crouch_min, crouch_max) = textured_mesh_extents(&crouching);
        assert!(
            (crouch_max[1] - crouch_min[1]) < (stand_max[1] - stand_min[1]) - 0.1,
            "slim={slim}: a sneaking player is shorter"
        );
        assert!(
            (crouch_max[2] - crouch_min[2]) > (stand_max[2] - stand_min[2]) + 0.1,
            "slim={slim}: a sneaking player leans forward"
        );
    }
}

fn player_texture_images() -> Vec<EntityModelTextureImage> {
    player_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
