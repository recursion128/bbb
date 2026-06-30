use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn strider_adult_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultStriderModel.createBodyLayer` (atlas 64×128). Each unified cube carries both the
    // colored geometry/tint and the textured `uv_size` / `texOffs` / `mirror`.
    assert_eq!(
        STRIDER_BODY[0],
        ModelCube::new(
            [-8.0, -6.0, -8.0],
            [16.0, 14.0, 16.0],
            STRIDER_MAROON,
            [16.0, 14.0, 16.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(STRIDER_RIGHT_LEG[0].min, [-2.0, 0.0, -2.0]);
    assert_eq!(STRIDER_RIGHT_LEG[0].size, [4.0, 16.0, 4.0]);
    assert_eq!(STRIDER_RIGHT_LEG[0].tex, [0.0, 32.0]);
    assert_eq!(STRIDER_LEFT_LEG[0].size, [4.0, 16.0, 4.0]);
    assert_eq!(STRIDER_LEFT_LEG[0].tex, [0.0, 55.0]);

    // Bristles are zero-thickness 12×0×16 planes; the right ones' box starts at -12 (mirrored), the
    // left ones at 0. Each bristle carries its own `texOffs(16, 33/49/65)`.
    assert_eq!(STRIDER_RIGHT_TOP_BRISTLE[0].min, [-12.0, 0.0, 0.0]);
    assert_eq!(STRIDER_RIGHT_TOP_BRISTLE[0].size, [12.0, 0.0, 16.0]);
    assert_eq!(STRIDER_RIGHT_TOP_BRISTLE[0].tex, [16.0, 33.0]);
    assert!(STRIDER_RIGHT_TOP_BRISTLE[0].mirror);
    assert_eq!(STRIDER_RIGHT_MIDDLE_BRISTLE[0].tex, [16.0, 49.0]);
    assert_eq!(STRIDER_RIGHT_BOTTOM_BRISTLE[0].tex, [16.0, 65.0]);
    assert_eq!(STRIDER_LEFT_TOP_BRISTLE[0].min, [0.0, 0.0, 0.0]);
    assert_eq!(STRIDER_LEFT_TOP_BRISTLE[0].size, [12.0, 0.0, 16.0]);
    assert_eq!(STRIDER_LEFT_TOP_BRISTLE[0].tex, [16.0, 33.0]);
    assert!(!STRIDER_LEFT_TOP_BRISTLE[0].mirror);
    assert_eq!(STRIDER_LEFT_MIDDLE_BRISTLE[0].tex, [16.0, 49.0]);
    assert_eq!(STRIDER_LEFT_BOTTOM_BRISTLE[0].tex, [16.0, 65.0]);

    // Offsets and bristle rest rolls.
    assert_eq!(STRIDER_BODY_BASE_Y, 2.0);
    assert_eq!(STRIDER_LEG_BASE_Y, 8.0);
    assert_eq!(STRIDER_RIGHT_LEG_X, -4.0);
    assert_eq!(STRIDER_LEFT_LEG_X, 4.0);
    assert_eq!(STRIDER_RIGHT_TOP_BRISTLE_POSE.offset, [-8.0, -5.0, -8.0]);
    assert_eq!(STRIDER_RIGHT_MIDDLE_BRISTLE_POSE.offset, [-8.0, -1.0, -8.0]);
    assert_eq!(STRIDER_RIGHT_BOTTOM_BRISTLE_POSE.offset, [-8.0, 4.0, -8.0]);
    assert_eq!(STRIDER_LEFT_TOP_BRISTLE_POSE.offset, [8.0, -6.0, -8.0]);
    assert_eq!(STRIDER_LEFT_MIDDLE_BRISTLE_POSE.offset, [8.0, -2.0, -8.0]);
    assert_eq!(STRIDER_LEFT_BOTTOM_BRISTLE_POSE.offset, [8.0, 3.0, -8.0]);
    assert!((STRIDER_RIGHT_TOP_BRISTLE_POSE.rotation[2] - -0.872_664_63).abs() < 1.0e-6);
    assert!((STRIDER_RIGHT_MIDDLE_BRISTLE_POSE.rotation[2] - -1.134_464).abs() < 1.0e-6);
    assert!((STRIDER_RIGHT_BOTTOM_BRISTLE_POSE.rotation[2] - -1.221_730_5).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_TOP_BRISTLE_POSE.rotation[2] - 0.872_664_63).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_MIDDLE_BRISTLE_POSE.rotation[2] - 1.134_464).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_BOTTOM_BRISTLE_POSE.rotation[2] - 1.221_730_5).abs() < 1.0e-6);
}

#[test]
fn strider_baby_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyStriderModel.createBodyLayer` (atlas 32×32).
    assert_eq!(STRIDER_BABY_BODY[0].min, [-3.5, -3.75, -4.0]);
    assert_eq!(STRIDER_BABY_BODY[0].size, [7.0, 7.0, 8.0]);
    assert_eq!(STRIDER_BABY_BODY[0].tex, [0.0, 0.0]);
    assert_eq!(STRIDER_BABY_RIGHT_LEG[0].size, [2.0, 4.0, 2.0]);
    assert_eq!(STRIDER_BABY_RIGHT_LEG[0].tex, [0.0, 24.0]);
    assert_eq!(STRIDER_BABY_LEFT_LEG[0].size, [2.0, 4.0, 2.0]);
    assert_eq!(STRIDER_BABY_LEFT_LEG[0].tex, [8.0, 24.0]);

    // Baby bristles are zero-thickness 7×3×0 planes that flap on `xRot` (no rest roll); each carries
    // its own `texOffs(0, 15/18/21)`.
    assert_eq!(STRIDER_BABY_FRONT_BRISTLE[0].min, [-3.5, -2.5, 0.0]);
    assert_eq!(STRIDER_BABY_FRONT_BRISTLE[0].size, [7.0, 3.0, 0.0]);
    assert_eq!(STRIDER_BABY_FRONT_BRISTLE[0].tex, [0.0, 15.0]);
    assert_eq!(STRIDER_BABY_MIDDLE_BRISTLE[0].tex, [0.0, 18.0]);
    assert_eq!(STRIDER_BABY_BACK_BRISTLE[0].tex, [0.0, 21.0]);

    assert_eq!(STRIDER_BABY_BODY_BASE_Y, 17.25);
    assert_eq!(STRIDER_BABY_LEG_BASE_Y, 20.0);
    assert_eq!(STRIDER_BABY_RIGHT_LEG_X, -1.5);
    assert_eq!(STRIDER_BABY_LEFT_LEG_X, 1.5);
    assert_eq!(STRIDER_BABY_FRONT_BRISTLE_POSE.offset, [0.0, -4.25, -2.0]);
    assert_eq!(STRIDER_BABY_MIDDLE_BRISTLE_POSE.offset, [0.0, -4.25, 0.0]);
    assert_eq!(STRIDER_BABY_BACK_BRISTLE_POSE.offset, [0.0, -4.25, 2.0]);
}

#[test]
fn strider_setup_anim_curves_match_vanilla() {
    // `animationSpeed = min(walkAnimationSpeed, 0.25)`.
    assert!((strider_animation_speed(0.1) - 0.1).abs() < 1.0e-6);
    assert!(
        (strider_animation_speed(0.8) - 0.25).abs() < 1.0e-6,
        "clamped"
    );

    let (pos, speed, age) = (3.0_f32, 0.2_f32, 11.0_f32);

    // `body.zRot = 0.1·sin(pos·1.5)·4·speed`.
    let expected_body_z = 0.1 * (pos * 1.5).sin() * 4.0 * speed;
    assert!((strider_body_z_rot(pos, speed) - expected_body_z).abs() < 1.0e-6);

    // Leg swing/roll: left uses phase 0, right uses phase π.
    let expected_left_x = (pos * 0.75).sin() * 2.0 * speed;
    let expected_right_x = (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_x_rot(pos, speed, false) - expected_left_x).abs() < 1.0e-6);
    assert!((strider_leg_x_rot(pos, speed, true) - expected_right_x).abs() < 1.0e-6);
    let expected_left_z = (std::f32::consts::PI / 18.0) * (pos * 0.75).cos() * speed;
    assert!((strider_leg_z_rot(pos, speed, false) - expected_left_z).abs() < 1.0e-6);

    // Body bob (adult mul 2, baby mul 1).
    let expected_adult_body_y = 2.0 - 2.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(2.0, 2.0, pos, speed) - expected_adult_body_y).abs() < 1.0e-6);
    let expected_baby_body_y = 17.25 - 1.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(17.25, 1.0, pos, speed) - expected_baby_body_y).abs() < 1.0e-6);

    // Leg lift: right uses phase 0, left uses phase π (opposite of the swing).
    let expected_right_y = 8.0 + 2.0 * (pos * 0.75).sin() * 2.0 * speed;
    let expected_left_y = 8.0 + 2.0 * (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_y(8.0, pos, speed, true) - expected_right_y).abs() < 1.0e-6);
    assert!((strider_leg_y(8.0, pos, speed, false) - expected_left_y).abs() < 1.0e-6);

    // Bristle flow + per-bristle ripple.
    let flow = (pos * 1.5 + std::f32::consts::PI).cos() * speed;
    assert!((strider_bristle_flow(pos, speed) - flow).abs() < 1.0e-6);
    assert!(
        (strider_bristle_top_flow(flow, age) - (flow * 0.6 + 0.1 * (age * 0.4).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_middle_flow(flow, age) - (flow * 1.2 + 0.1 * (age * 0.2).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_bottom_flow(flow, age) - (flow * 1.3 + 0.05 * (age * -0.4).sin())).abs()
            < 1.0e-6
    );
}

#[test]
fn strider_adult_mesh_uses_vanilla_body_layer_geometry() {
    // Nine cubes (body, two legs, six bristles) → 54 faces / 216 vertices.
    let strider = entity_model_mesh(&[EntityModelInstance::strider(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    assert_eq!(strider.opaque_faces, 54);
    assert_eq!(strider.vertices.len(), 216);
    assert_eq!(strider.indices.len(), 324);
    assert!(strider
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(STRIDER_MAROON, 1.0)));
    assert!(strider
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(STRIDER_LEG, 1.0)));
}

#[test]
fn strider_baby_mesh_uses_vanilla_body_layer_geometry() {
    // Six cubes (body, two legs, three bristles) → 36 faces / 144 vertices.
    let baby = entity_model_mesh(&[EntityModelInstance::strider(
        701,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )]);
    assert_eq!(baby.opaque_faces, 36);
    assert_eq!(baby.vertices.len(), 144);
    assert_eq!(baby.indices.len(), 216);

    // The baby is a different (smaller) model than the adult, not a scaled copy.
    let adult = entity_model_mesh(&[EntityModelInstance::strider(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    assert_ne!(baby.vertices.len(), adult.vertices.len());
}

#[test]
fn strider_body_tracks_look_angles() {
    // The body tracks the projected look yaw/pitch.
    let base = EntityModelInstance::strider(703, [0.0, 64.0, 0.0], 0.0, false, false);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -25.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the body tracks the look"
    );

    let ridden = entity_model_mesh(&[base.with_head_look(40.0, -25.0).with_strider_ridden(true)]);
    assert_eq!(
        forward.vertices, ridden.vertices,
        "ridden striders zero the body pitch/yaw instead of tracking the look"
    );
}

#[test]
fn strider_walk_animates_legs_body_and_bristles() {
    // A standing strider differs from a walking one: the legs swing/lift, the body sways/bobs,
    // and the bristles flow.
    let base = EntityModelInstance::strider(704, [0.0, 64.0, 0.0], 0.0, false, false);
    let standing = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(3.0, 0.2)]);
    assert_eq!(standing.vertices.len(), walking.vertices.len());
    assert_ne!(standing.vertices, walking.vertices, "the walk animates");

    // The bristles also ripple on `ageInTicks` even when standing still.
    let idle_rippled =
        entity_model_mesh(&[base.with_age_in_ticks(13.0).with_walk_animation(3.0, 0.2)]);
    assert_ne!(
        walking.vertices, idle_rippled.vertices,
        "bristles ripple with age"
    );
}

#[test]
fn strider_texture_refs_match_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Strider {
            baby: false,
            cold: false,
        }
        .model_key(),
        "strider"
    );
    assert_eq!(
        EntityModelKind::Strider {
            baby: true,
            cold: false,
        }
        .model_key(),
        "strider_baby"
    );
    assert_eq!(
        EntityModelKind::Strider {
            baby: false,
            cold: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/strider/strider.png",
            size: [64, 128],
        })
    );
    assert_eq!(
        EntityModelKind::Strider {
            baby: true,
            cold: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/strider/strider_baby.png",
            size: [32, 32],
        })
    );
    // A suffocating (cold) strider swaps to the `strider_cold` texture × age
    // (`StriderRenderer.getTextureLocation`).
    assert_eq!(
        EntityModelKind::Strider {
            baby: false,
            cold: true,
        }
        .vanilla_texture_ref(),
        Some(STRIDER_COLD_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Strider {
            baby: true,
            cold: true,
        }
        .vanilla_texture_ref(),
        Some(STRIDER_COLD_BABY_TEXTURE_REF)
    );
    assert_eq!(strider_texture_ref(false, true), STRIDER_COLD_TEXTURE_REF);
    assert_eq!(
        strider_texture_ref(true, true),
        STRIDER_COLD_BABY_TEXTURE_REF
    );
    assert_eq!(
        STRIDER_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/strider_saddle/saddle.png",
            size: [64, 128],
        }
    );
    assert!(entity_model_texture_refs().contains(&STRIDER_COLD_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRIDER_COLD_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRIDER_SADDLE_TEXTURE_REF));
    assert_eq!(strider_entity_texture_refs().len(), 4);
}

#[test]
fn strider_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&strider_texture_images()).unwrap();

    // `StriderModel` inherits `EntityModel`'s default `RenderTypes::entityCutout`. The backend folds
    // adult/baby into the cutout mesh, but the submissions keep the vanilla texture, render type,
    // tint, transform, and default collector order.
    let adult = EntityModelInstance::strider(750, [0.0, 64.0, 0.0], 0.0, false, false)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, STRIDER_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(adult));
    assert_eq!(submit.light, adult.render_state.shader_light());
    assert_eq!(submit.overlay, adult.render_state.overlay_coords());
    assert_ne!(submit.overlay, [0.0, 10.0]);
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));

    // Nine cubes → 54 faces / 216 vertices, with nothing on the translucent or eyes passes.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 54);
    assert_eq!(meshes.cutout.vertices.len(), 216);
    assert_eq!(meshes.cutout.indices.len(), 324);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));

    // Baby is the smaller model: six cubes → 36 faces / 144 vertices.
    let baby = EntityModelInstance::strider(751, [0.0, 64.0, 0.0], 0.0, true, false)
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.submissions.len(), 1);
    let baby_submit = baby_meshes.submissions[0];
    assert_eq!(
        baby_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby_submit.texture, STRIDER_BABY_TEXTURE_REF);
    assert_eq!(baby_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby_submit.transform, entity_model_root_transform(baby));
    assert_eq!(baby_submit.light, baby.render_state.shader_light());
    assert_eq!(baby_submit.overlay, baby.render_state.overlay_coords());
    assert_ne!(baby_submit.overlay, [0.0, 10.0]);
    assert_eq!((baby_submit.order, baby_submit.submit_sequence), (0, 0));
    assert_eq!(baby_meshes.cutout.vertices.len(), 144);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby_submit.light && vertex.overlay == baby_submit.overlay));
    assert!(baby_meshes.armor_cutout.vertices.is_empty());

    // The body tracks the look, and the walk + age animate the legs/body/bristles.
    let looking = entity_model_textured_meshes(&[adult.with_head_look(40.0, -25.0)], &atlas);
    assert_ne!(meshes.cutout.vertices, looking.cutout.vertices);
    let ridden = entity_model_textured_meshes(
        &[adult.with_head_look(40.0, -25.0).with_strider_ridden(true)],
        &atlas,
    );
    assert_eq!(
        meshes.cutout.vertices, ridden.cutout.vertices,
        "ridden striders zero the body pitch/yaw in the textured model"
    );
    let walking = entity_model_textured_meshes(&[adult.with_walk_animation(3.0, 0.2)], &atlas);
    assert_ne!(meshes.cutout.vertices, walking.cutout.vertices);
    let rippled = entity_model_textured_meshes(
        &[adult.with_age_in_ticks(13.0).with_walk_animation(3.0, 0.2)],
        &atlas,
    );
    assert_ne!(walking.cutout.vertices, rippled.cutout.vertices);
}

#[test]
fn strider_saddle_layer_renders_for_adults_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        STRIDER_TEXTURE_REF,
        STRIDER_BABY_TEXTURE_REF,
        STRIDER_COLD_TEXTURE_REF,
        STRIDER_COLD_BABY_TEXTURE_REF,
        STRIDER_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let adult = EntityModelInstance::strider(752, [0.0, 64.0, 0.0], 0.0, false, false)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with_strider_saddle(true);
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    let saddle_mesh = &saddled_meshes.armor_cutout;
    assert_eq!(saddled_meshes.submissions.len(), 2);
    let base_submit = saddled_meshes.submissions[0];
    assert_eq!(
        base_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(base_submit.texture, STRIDER_TEXTURE_REF);
    assert_eq!(base_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        base_submit.transform,
        entity_model_root_transform(saddled_instance)
    );
    assert_eq!(base_submit.light, adult.render_state.shader_light());
    assert_eq!(base_submit.overlay, adult.render_state.overlay_coords());
    assert_ne!(base_submit.overlay, [0.0, 10.0]);
    assert_eq!((base_submit.order, base_submit.submit_sequence), (0, 0));
    let saddle_submit = saddled_meshes.submissions[1];
    assert_eq!(
        saddle_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(saddle_submit.texture, STRIDER_SADDLE_TEXTURE_REF);
    assert_eq!(saddle_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((saddle_submit.order, saddle_submit.submit_sequence), (0, 1));
    assert_eq!(
        saddle_submit.transform,
        entity_model_root_transform(saddled_instance)
    );
    assert_eq!(saddle_submit.light, adult.render_state.shader_light());
    assert_eq!(saddle_submit.overlay, [0.0, 10.0]);
    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddle_mesh.cutout_faces, 54);
    assert_eq!(saddle_mesh.vertices.len(), 216);
    assert!(saddled.vertices[..216]
        .iter()
        .all(|vertex| vertex.light == base_submit.light && vertex.overlay == base_submit.overlay));
    assert!(saddle_mesh.vertices.iter().all(
        |vertex| vertex.light == saddle_submit.light && vertex.overlay == saddle_submit.overlay
    ));

    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == STRIDER_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_saddle_vertex = saddle_mesh.vertices[0].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby = EntityModelInstance::strider(753, [0.0, 64.0, 0.0], 0.0, true, false)
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_strider_saddle(true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.submissions.len(), 1);
    let baby_submit = baby_meshes.submissions[0];
    assert_eq!(
        baby_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby_submit.texture, STRIDER_BABY_TEXTURE_REF);
    assert_eq!(baby_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby_submit.transform, entity_model_root_transform(baby));
    assert_eq!(baby_submit.light, baby.render_state.shader_light());
    assert_eq!(baby_submit.overlay, baby.render_state.overlay_coords());
    assert_eq!((baby_submit.order, baby_submit.submit_sequence), (0, 0));
    assert_eq!(
        baby_meshes.cutout.cutout_faces, 36,
        "vanilla supplies no baby model for the strider saddle layer"
    );
    assert_eq!(baby_meshes.cutout.vertices.len(), 144);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby_submit.light && vertex.overlay == baby_submit.overlay));
}

#[test]
fn strider_saddle_submission_survives_missing_saddle_texture_atlas_entry() {
    // Vanilla `StriderRenderer` records `SimpleEquipmentLayer(STRIDER_SADDLE)` with the default
    // collector order, after the base body, and forces `OverlayTexture.NO_OVERLAY`.
    let images = texture_images(&[STRIDER_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::strider(754, [0.0, 64.0, 0.0], 0.0, false, false)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_strider_saddle(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, STRIDER_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_eq!(
        meshes.cutout.vertices.len(),
        216,
        "missing strider_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));

    let saddle = meshes.submissions[1];
    assert_eq!(
        saddle.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(saddle.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(saddle.texture, STRIDER_SADDLE_TEXTURE_REF);
    assert_eq!(saddle.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(saddle.transform, base.transform);
    assert_eq!((saddle.order, saddle.submit_sequence), (0, 1));
    assert_eq!(saddle.light, base.light);
    assert_eq!(saddle.overlay, [0.0, 10.0]);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn strider_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(strider_entity_texture_refs())
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
