use super::*;

use std::f32::consts::PI;

#[test]
fn guardian_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `GuardianModel.createBodyLayer` (atlas 64×64). The `head` part carries five body
    // cubes: the main 12×12×16 box, two 2×12×12 side plates, and the bottom/top 12×2×12 plates.
    assert_eq!(GUARDIAN_HEAD.len(), 5);
    assert_eq!(GUARDIAN_HEAD[0].min, [-6.0, 10.0, -8.0]);
    assert_eq!(GUARDIAN_HEAD[0].size, [12.0, 12.0, 16.0]);
    assert_eq!(GUARDIAN_HEAD[1].min, [-8.0, 10.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[1].size, [2.0, 12.0, 12.0]);
    assert_eq!(GUARDIAN_HEAD[2].min, [6.0, 10.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[3].min, [-6.0, 8.0, -6.0]);
    assert_eq!(GUARDIAN_HEAD[3].size, [12.0, 2.0, 12.0]);
    assert_eq!(GUARDIAN_HEAD[4].min, [-6.0, 22.0, -6.0]);

    // The shared spike box `addBox(-1, -4.5, -1, 2, 9, 2)`.
    assert_eq!(GUARDIAN_SPIKE[0].min, [-1.0, -4.5, -1.0]);
    assert_eq!(GUARDIAN_SPIKE[0].size, [2.0, 9.0, 2.0]);

    // The eye `addBox(-1, 15, 0, 2, 2, 1)` at `offset(0, 0, -8.25)`.
    assert_eq!(GUARDIAN_EYE_CUBE[0].min, [-1.0, 15.0, 0.0]);
    assert_eq!(GUARDIAN_EYE_CUBE[0].size, [2.0, 2.0, 1.0]);
    assert_eq!(GUARDIAN_EYE_POSE.offset, [0.0, 0.0, -8.25]);

    // The three-segment tail: tail0 (ZERO), tail1 at (-1.5, 0.5, 14), tail2 (two cubes) at
    // (0.5, 0.5, 6).
    assert_eq!(GUARDIAN_TAIL0[0].min, [-2.0, 14.0, 7.0]);
    assert_eq!(GUARDIAN_TAIL0[0].size, [4.0, 4.0, 8.0]);
    assert_eq!(GUARDIAN_TAIL1[0].size, [3.0, 3.0, 7.0]);
    assert_eq!(GUARDIAN_TAIL1_POSE.offset, [-1.5, 0.5, 14.0]);
    assert_eq!(GUARDIAN_TAIL2.len(), 2);
    assert_eq!(GUARDIAN_TAIL2[0].size, [2.0, 2.0, 6.0]);
    assert_eq!(GUARDIAN_TAIL2[1].min, [1.0, 10.5, 3.0]);
    assert_eq!(GUARDIAN_TAIL2[1].size, [1.0, 9.0, 9.0]);
    assert_eq!(GUARDIAN_TAIL2_POSE.offset, [0.5, 0.5, 6.0]);

    // The `SPIKE_*` tables transcribed verbatim from `GuardianModel`.
    assert_eq!(
        GUARDIAN_SPIKE_X_ROT,
        [1.75, 0.25, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.25, 0.75, 0.0, 0.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Y_ROT,
        [0.0, 0.0, 0.0, 0.0, 0.25, 1.75, 1.25, 0.75, 0.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Z_ROT,
        [0.0, 0.0, 0.25, 1.75, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.75, 1.25]
    );
    assert_eq!(
        GUARDIAN_SPIKE_X,
        [0.0, 0.0, 8.0, -8.0, -8.0, 8.0, 8.0, -8.0, 0.0, 0.0, 8.0, -8.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Y,
        [-8.0, -8.0, -8.0, -8.0, 0.0, 0.0, 0.0, 0.0, 8.0, 8.0, 8.0, 8.0]
    );
    assert_eq!(
        GUARDIAN_SPIKE_Z,
        [8.0, -8.0, 0.0, 0.0, -8.0, -8.0, 8.0, 8.0, 8.0, -8.0, 0.0, 0.0]
    );
}

#[test]
fn guardian_spike_bind_pose_matches_vanilla_get_spike() {
    // Vanilla `createBodyLayer` places spike `i` at `getSpike{X,Y,Z}(i, 0, 0)` with rotation
    // `PI * SPIKE_{X,Y,Z}_ROT[i]`, where `getSpikeOffset(i, 0, 0) = 1 + cos(i) * 0.01` and the
    // Y base adds 16.
    for i in 0..12 {
        let factor = 1.0 + (i as f32).cos() * 0.01;
        let pose = guardian_spike_bind_pose(i);
        assert!((pose.offset[0] - GUARDIAN_SPIKE_X[i] * factor).abs() < 1.0e-6);
        assert!((pose.offset[1] - (16.0 + GUARDIAN_SPIKE_Y[i] * factor)).abs() < 1.0e-6);
        assert!((pose.offset[2] - GUARDIAN_SPIKE_Z[i] * factor).abs() < 1.0e-6);
        assert!((pose.rotation[0] - PI * GUARDIAN_SPIKE_X_ROT[i]).abs() < 1.0e-6);
        assert!((pose.rotation[1] - PI * GUARDIAN_SPIKE_Y_ROT[i]).abs() < 1.0e-6);
        assert!((pose.rotation[2] - PI * GUARDIAN_SPIKE_Z_ROT[i]).abs() < 1.0e-6);
    }
}

#[test]
fn guardian_mesh_uses_vanilla_body_layer_geometry() {
    // 5 head cubes + 12 spikes + 1 eye + 3 tail cubes (1 + 1 + 2) = 22 cubes → 132 faces /
    // 528 vertices.
    let guardian = entity_model_mesh(&[EntityModelInstance::guardian(
        990,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(guardian.opaque_faces, 132);
    assert_eq!(guardian.vertices.len(), 528);
    assert_eq!(guardian.indices.len(), 792);
    // The body uses the guardian body color; the eye uses its own pink tint.
    assert!(guardian
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GUARDIAN_BODY, 1.0)));
    assert!(guardian
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GUARDIAN_EYE, 1.0)));
}

#[test]
fn elder_guardian_is_the_guardian_mesh_scaled_up() {
    // The elder guardian is the same 22-cube mesh scaled 2.35× by `ELDER_GUARDIAN_SCALE`, so it
    // keeps the cube count but occupies a larger world-space extent.
    assert_eq!(GUARDIAN_ELDER_SCALE, 2.35);
    let guardian = entity_model_mesh(&[EntityModelInstance::guardian(
        991,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let elder = entity_model_mesh(&[EntityModelInstance::guardian(
        992,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(guardian.vertices.len(), elder.vertices.len());
    assert_ne!(guardian.vertices, elder.vertices, "the elder is scaled up");

    let (g_min, g_max) = mesh_extents(&guardian);
    let (e_min, e_max) = mesh_extents(&elder);
    let guardian_width = g_max[0] - g_min[0];
    let elder_width = e_max[0] - e_min[0];
    assert!(
        elder_width > guardian_width * 2.0,
        "the elder guardian is ~2.35× the guardian's size ({guardian_width} vs {elder_width})"
    );
}
