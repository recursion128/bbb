use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn nautilus_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `NautilusModel.createBodyMesh` (atlas 128×128): one cubeless `root` pivot parenting the
    // shell and the body (the body parenting the three mouths).
    assert_eq!(NAUTILUS_PARTS.len(), 1);
    let root = &NAUTILUS_PARTS[0];
    assert_eq!(root.pose.offset, [0.0, 29.0, -6.0]);
    assert!(root.cubes.is_empty());
    assert_eq!(root.children.len(), 2);

    // `shell` (offset (0, -13, 5)): the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 fin plane.
    let shell = &root.children[0];
    assert_eq!(shell.pose.offset, [0.0, -13.0, 5.0]);
    assert_eq!(shell.cubes.len(), 3);
    assert_eq!(shell.cubes[0].min, [-7.0, -10.0, -7.0]);
    assert_eq!(shell.cubes[0].size, [14.0, 10.0, 16.0]);
    assert_eq!(shell.cubes[1].size, [14.0, 8.0, 20.0]);
    assert_eq!(shell.cubes[2].size, [14.0, 8.0, 0.0]);

    // `body` (offset (0, -8.5, 12.3)): the 10×8×14 trunk and a 10×8×0 fin plane, parenting the mouths.
    let body = &root.children[1];
    assert_eq!(body.pose.offset, [0.0, -8.5, 12.3]);
    assert_eq!(body.cubes.len(), 2);
    assert_eq!(body.cubes[0].min, [-5.0, -4.51, -3.0]);
    assert_eq!(body.cubes[0].size, [10.0, 8.0, 14.0]);
    assert_eq!(body.children.len(), 3);

    // The three mouths; upper/lower deflated by the vanilla `CubeDeformation(-0.001)`, inner undeformed.
    assert_eq!(body.children[0].pose.offset, [0.0, -2.51, 7.0]);
    assert_eq!(body.children[0].cubes[0].min, [-4.999, -1.999, 0.001]);
    assert_eq!(body.children[0].cubes[0].size, [9.998, 3.998, 3.998]);
    assert_eq!(body.children[1].pose.offset, [0.0, -0.51, 7.5]);
    assert_eq!(body.children[1].cubes[0].min, [-3.0, -2.0, -0.5]);
    assert_eq!(body.children[1].cubes[0].size, [6.0, 4.0, 4.0]);
    assert_eq!(body.children[2].pose.offset, [0.0, 1.49, 7.0]);
    assert_eq!(body.children[2].cubes[0].min, [-4.999, -1.979, 0.001]);

    // Eight cubes (shell 3, body 2, three mouths).
    assert_eq!(count_cubes(&NAUTILUS_PARTS), 8);
}

#[test]
fn nautilus_mesh_uses_vanilla_body_layer_geometry() {
    // 8 cubes → 48 faces / 192 vertices / 288 indices, two tones: tan shell, pale body/mouths.
    let nautilus = entity_model_mesh(&[EntityModelInstance::nautilus(300, [0.0, 64.0, 0.0], 0.0)]);
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
    let instances = [EntityModelInstance::nautilus(301, [0.0, 64.0, 0.0], 0.0)];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn nautilus_body_look_turns_the_body_and_mouths_not_the_shell() {
    // Vanilla `NautilusModel.applyBodyRotation` steers the `body` (not the shell) by the look. The shell
    // is the first child (3 cubes → vertices `[0, 72)`); the body and its three mouths `[72, 192)` turn.
    let rest = EntityModelInstance::nautilus(302, [0.0, 64.0, 0.0], 0.0);
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
            EntityModelInstance::nautilus(303, [0.0, 64.0, 0.0], 0.0).with_head_look(50.0, 50.0)
        ]);
    let clamped_90 =
        entity_model_mesh(&[
            EntityModelInstance::nautilus(304, [0.0, 64.0, 0.0], 0.0).with_head_look(90.0, 90.0)
        ]);
    assert_eq!(
        clamped_50.vertices, clamped_90.vertices,
        "looks past ±10° clamp to the same body rotation"
    );
    let within_5 = entity_model_mesh(&[
        EntityModelInstance::nautilus(305, [0.0, 64.0, 0.0], 0.0).with_head_look(5.0, 5.0)
    ]);
    assert_ne!(
        within_5.vertices, clamped_50.vertices,
        "a look inside ±10° turns the body less than the clamp"
    );
}

#[test]
fn nautilus_exposes_stable_model_key() {
    assert_eq!(EntityModelKind::Nautilus.model_key(), "nautilus");
}
