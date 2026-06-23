use super::*;

#[test]
fn nautilus_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `NautilusModel.createBodyMesh` (atlas 128×128): one cubeless `root` pivot parenting the
    // `shell` and the `body` (the body parenting the three mouths).
    assert_eq!(NAUTILUS_ROOT_POSE.offset, [0.0, 29.0, -6.0]);

    // `shell` (offset (0, -13, 5)): the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 fin plane.
    assert_eq!(NAUTILUS_SHELL_POSE.offset, [0.0, -13.0, 5.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES.len(), 3);
    assert_eq!(NAUTILUS_SHELL_CUBES[0].min, [-7.0, -10.0, -7.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[0].size, [14.0, 10.0, 16.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[1].size, [14.0, 8.0, 20.0]);
    assert_eq!(NAUTILUS_SHELL_CUBES[2].size, [14.0, 8.0, 0.0]);

    // `body` (offset (0, -8.5, 12.3)): the 10×8×14 trunk and a 10×8×0 fin plane, parenting the mouths.
    assert_eq!(NAUTILUS_BODY_POSE.offset, [0.0, -8.5, 12.3]);
    assert_eq!(NAUTILUS_BODY_CUBES.len(), 2);
    assert_eq!(NAUTILUS_BODY_CUBES[0].min, [-5.0, -4.51, -3.0]);
    assert_eq!(NAUTILUS_BODY_CUBES[0].size, [10.0, 8.0, 14.0]);

    // The three mouths; upper/lower deflated by the vanilla `CubeDeformation(-0.001)`, inner undeformed.
    assert_eq!(NAUTILUS_UPPER_MOUTH_POSE.offset, [0.0, -2.51, 7.0]);
    assert_eq!(NAUTILUS_UPPER_MOUTH_CUBES[0].min, [-4.999, -1.999, 0.001]);
    assert_eq!(NAUTILUS_UPPER_MOUTH_CUBES[0].size, [9.998, 3.998, 3.998]);
    assert_eq!(NAUTILUS_INNER_MOUTH_POSE.offset, [0.0, -0.51, 7.5]);
    assert_eq!(NAUTILUS_INNER_MOUTH_CUBES[0].min, [-3.0, -2.0, -0.5]);
    assert_eq!(NAUTILUS_INNER_MOUTH_CUBES[0].size, [6.0, 4.0, 4.0]);
    assert_eq!(NAUTILUS_LOWER_MOUTH_POSE.offset, [0.0, 1.49, 7.0]);
    assert_eq!(NAUTILUS_LOWER_MOUTH_CUBES[0].min, [-4.999, -1.979, 0.001]);
}

#[test]
fn nautilus_mesh_uses_vanilla_body_layer_geometry() {
    // 8 cubes → 48 faces / 192 vertices / 288 indices, two tones: tan shell, pale body/mouths.
    let nautilus = entity_model_mesh(&[EntityModelInstance::nautilus(
        300,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(nautilus.opaque_faces, 48);
    assert_eq!(nautilus.vertices.len(), 192);
    assert_eq!(nautilus.indices.len(), 288);
    assert!(nautilus
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_SHELL, 1.0)));
    assert!(nautilus
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_BODY, 1.0)));
}

#[test]
fn nautilus_mesh_matches_on_both_render_paths() {
    // The nautilus is a colored-only entity, so the texture-skipping colored runtime path emits the
    // exact same mesh as the full path (unlike the horse proxy it replaced).
    let instances = [
        EntityModelInstance::nautilus(301, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::nautilus(311, [4.0, 64.0, 0.0], 0.0, true),
    ];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn nautilus_body_look_turns_the_body_and_mouths_not_the_shell() {
    // Vanilla `NautilusModel.applyBodyRotation` steers the `body` (not the shell) by the look. The shell
    // is the first child (3 cubes → vertices `[0, 72)`); the body and its three mouths `[72, 192)` turn.
    let rest = EntityModelInstance::nautilus(302, [0.0, 64.0, 0.0], 0.0, false);
    let looked = rest.with_head_look(8.0, -8.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..72],
        looked_mesh.vertices[..72],
        "the shell stays put"
    );
    assert_ne!(
        rest_mesh.vertices[72..],
        looked_mesh.vertices[72..],
        "the body and its mouths turn with the look"
    );
}

#[test]
fn nautilus_body_look_is_clamped_to_ten_degrees() {
    // Vanilla `applyBodyRotation` clamps the look to ±10° before steering the body, so two looks past
    // the clamp render identically while a look inside the clamp differs.
    let clamped_50 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(303, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(50.0, 50.0),
        ]);
    let clamped_90 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(304, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(90.0, 90.0),
        ]);
    assert_eq!(
        clamped_50.vertices, clamped_90.vertices,
        "looks past ±10° clamp to the same body rotation"
    );
    let within_5 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(305, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_look(5.0, 5.0),
        ]);
    assert_ne!(
        within_5.vertices, clamped_50.vertices,
        "a look inside ±10° turns the body less than the clamp"
    );
}

#[test]
fn baby_nautilus_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `NautilusModel.createBabyBodyLayer` (atlas 64×64): the same cubeless `root → shell + body
    // → three mouths` structure as the adult, in smaller hatchling proportions.
    assert_eq!(BABY_NAUTILUS_ROOT_POSE.offset, [-0.5, 28.0, -0.5]);

    // `shell` (offset (3, -8, -2)): the 7×4×7 dome, the 7×4×9 whorl, and a 7×4×0 fin plane.
    assert_eq!(BABY_NAUTILUS_SHELL_POSE.offset, [3.0, -8.0, -2.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES.len(), 3);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[0].min, [-6.0, -4.0, -1.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[0].size, [7.0, 4.0, 7.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[1].size, [7.0, 4.0, 9.0]);
    assert_eq!(BABY_NAUTILUS_SHELL_CUBES[2].size, [7.0, 4.0, 0.0]);

    // `body` (offset (0.5, -5, 3)): the 5×4×7 trunk and a 5×4×0 fin plane, parenting the mouths.
    assert_eq!(BABY_NAUTILUS_BODY_POSE.offset, [0.5, -5.0, 3.0]);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES.len(), 2);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES[0].min, [-2.5, -3.01, -1.0]);
    assert_eq!(BABY_NAUTILUS_BODY_CUBES[0].size, [5.0, 4.0, 7.0]);

    // The three mouths; upper/lower deflated by the vanilla `CubeDeformation(-0.001)`, inner undeformed.
    assert_eq!(BABY_NAUTILUS_UPPER_MOUTH_POSE.offset, [0.0, -2.01, 3.9]);
    assert_eq!(
        BABY_NAUTILUS_UPPER_MOUTH_CUBES[0].min,
        [-2.499, -0.999, 0.001]
    );
    assert_eq!(
        BABY_NAUTILUS_UPPER_MOUTH_CUBES[0].size,
        [4.998, 1.998, 1.998]
    );
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_POSE.offset, [0.0, -1.01, 4.9]);
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_CUBES[0].min, [-1.5, -1.0, -1.0]);
    assert_eq!(BABY_NAUTILUS_INNER_MOUTH_CUBES[0].size, [3.0, 2.0, 2.0]);
    assert_eq!(BABY_NAUTILUS_LOWER_MOUTH_POSE.offset, [0.0, -0.01, 3.9]);
    assert_eq!(
        BABY_NAUTILUS_LOWER_MOUTH_CUBES[0].min,
        [-2.499, -0.999, 0.001]
    );
}

#[test]
fn baby_nautilus_mesh_uses_vanilla_body_layer_geometry() {
    // 8 cubes → 48 faces / 192 vertices / 288 indices, two tones: tan shell, pale body/mouths.
    let baby = entity_model_mesh(&[EntityModelInstance::nautilus(
        310,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 48);
    assert_eq!(baby.vertices.len(), 192);
    assert_eq!(baby.indices.len(), 288);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_SHELL, 1.0)));
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(NAUTILUS_BODY, 1.0)));
}

#[test]
fn baby_nautilus_is_smaller_than_the_adult() {
    // The hatchling `createBabyBodyLayer` is a compacted version of the adult mesh, so its bounds are
    // tighter on every axis (the shell shrinks from 14-wide to 7-wide, etc.).
    let adult = entity_model_mesh(&[EntityModelInstance::nautilus(
        312,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let baby = entity_model_mesh(&[EntityModelInstance::nautilus(
        312,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    for axis in 0..3 {
        let adult_span = adult_max[axis] - adult_min[axis];
        let baby_span = baby_max[axis] - baby_min[axis];
        assert!(
            baby_span < adult_span,
            "baby axis {axis} span {baby_span} should be smaller than adult {adult_span}"
        );
    }
}

#[test]
fn baby_nautilus_body_look_turns_the_body_and_mouths_not_the_shell() {
    // The baby shares the adult's hierarchy (shell is the first child, 3 cubes → vertices `[0, 72)`), so
    // the clamped body look turns the body and its mouths `[72, 192)` while the shell holds.
    let rest = EntityModelInstance::nautilus(313, [0.0, 64.0, 0.0], 0.0, true);
    let looked = rest.with_head_look(8.0, -8.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(
        rest_mesh.vertices[..72],
        looked_mesh.vertices[..72],
        "the shell stays put"
    );
    assert_ne!(
        rest_mesh.vertices[72..],
        looked_mesh.vertices[72..],
        "the body and its mouths turn with the look"
    );
}

#[test]
fn nautilus_exposes_stable_model_key() {
    assert_eq!(
        EntityModelKind::Nautilus { baby: false }.model_key(),
        "nautilus"
    );
    assert_eq!(
        EntityModelKind::Nautilus { baby: true }.model_key(),
        "nautilus_baby"
    );
}
