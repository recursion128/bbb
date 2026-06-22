use super::*;

use std::f32::consts::PI;

#[test]
fn dolphin_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `DolphinModel.createBodyLayer` (atlas 64×64): the `body` is the 8×7×13 root child.
    assert_eq!(DOLPHIN_BODY[0].min, [-4.0, -7.0, 0.0]);
    assert_eq!(DOLPHIN_BODY[0].size, [8.0, 7.0, 13.0]);
    assert_eq!(DOLPHIN_BODY_POSE.offset, [0.0, 22.0, -5.0]);

    // The side fins share one 1×4×7 box; the tail/tail-fin/head/nose match vanilla.
    assert_eq!(DOLPHIN_SIDE_FIN[0].size, [1.0, 4.0, 7.0]);
    assert_eq!(DOLPHIN_TAIL[0].size, [4.0, 5.0, 11.0]);
    assert_eq!(DOLPHIN_TAIL_FIN[0].size, [10.0, 1.0, 6.0]);
    assert_eq!(DOLPHIN_HEAD[0].size, [8.0, 7.0, 6.0]);
    assert_eq!(DOLPHIN_NOSE[0].size, [2.0, 2.0, 4.0]);

    // The fins' compound bind rotations and the tail's bind pitch.
    assert_eq!(
        DOLPHIN_LEFT_FIN_POSE.rotation,
        [PI / 3.0, 0.0, 2.0 * PI / 3.0]
    );
    assert_eq!(
        DOLPHIN_RIGHT_FIN_POSE.rotation,
        [PI / 3.0, 0.0, -2.0 * PI / 3.0]
    );
    assert_eq!(
        DOLPHIN_TAIL_POSE.rotation,
        [DOLPHIN_TAIL_BIND_X_ROT, 0.0, 0.0]
    );
}

#[test]
fn dolphin_wave_matches_vanilla_setup_anim() {
    // `DolphinModel.setupAnim` swim wave term `cos(ageInTicks · 0.3)` (1 at `t=0`).
    assert!((dolphin_wave(0.0) - 1.0).abs() < 1.0e-6);
    assert!((dolphin_wave(10.0) - (3.0_f32).cos()).abs() < 1.0e-6);
}

#[test]
fn dolphin_mesh_uses_vanilla_body_layer_geometry() {
    // Body, back fin, two side fins, tail, tail fin, head, nose → 8 cubes / 48 faces / 192
    // vertices.
    let adult = entity_model_mesh(&[EntityModelInstance::dolphin(
        970,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 48);
    assert_eq!(adult.vertices.len(), 192);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DOLPHIN_GRAY, 1.0)));

    // The baby is the same geometry scaled by 0.5, so it occupies a smaller world-space extent but
    // keeps the same cube count.
    let baby = entity_model_mesh(&[EntityModelInstance::dolphin(
        971,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.vertices.len(), 192);
    assert_ne!(adult.vertices, baby.vertices, "the baby is scaled down");
}

#[test]
fn dolphin_tail_waves_only_while_moving() {
    // At rest the tail holds its bind pitch and the model is static across ages.
    let still = EntityModelInstance::dolphin(972, [0.0, 64.0, 0.0], 0.0, false);
    let still_early = entity_model_mesh(&[still]);
    let still_later = entity_model_mesh(&[still.with_age_in_ticks(5.0)]);
    assert_eq!(
        still_early.vertices, still_later.vertices,
        "a still dolphin does not animate"
    );

    // While moving, the swim tail/body wave re-poses the mesh as the age advances.
    let moving = still.with_is_moving(true);
    let moving_early = entity_model_mesh(&[moving]);
    let moving_later = entity_model_mesh(&[moving.with_age_in_ticks(5.0)]);
    assert_ne!(
        moving_early.vertices, moving_later.vertices,
        "a moving dolphin waves its tail"
    );
    assert_ne!(still_early.vertices, moving_early.vertices);
}

#[test]
fn dolphin_body_steers_by_look() {
    // `body.xRot = state.xRot`, `body.yRot = state.yRot`: the body tracks the look pitch/yaw.
    let base = EntityModelInstance::dolphin(973, [0.0, 64.0, 0.0], 0.0, false);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -25.0)]);
    let forward = entity_model_mesh(&[base]);
    assert_ne!(forward.vertices, looking.vertices);
}

#[test]
fn dolphin_texture_ref_matches_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Dolphin { baby: false }.model_key(),
        "dolphin"
    );
    assert_eq!(
        EntityModelKind::Dolphin { baby: true }.model_key(),
        "dolphin_baby"
    );
    assert_eq!(
        EntityModelKind::Dolphin { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/dolphin/dolphin.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Dolphin { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/dolphin/dolphin_baby.png",
            size: [64, 64],
        })
    );
}
