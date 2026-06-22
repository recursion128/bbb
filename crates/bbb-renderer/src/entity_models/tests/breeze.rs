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
}
