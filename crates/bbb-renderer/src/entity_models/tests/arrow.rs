use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn arrow_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ArrowModel.createBodyLayer` (atlas 32×32): the `back` arrowhead plane plus the two
    // crossed fletching planes.
    assert_eq!(ARROW_PARTS.len(), 3);

    // `back`: the 0×5×5 plane at offset (-11, 0, 0), pitched π/4, with `withScale(0.8)` baked → 0×4×4.
    let back = &ARROW_PARTS[0];
    assert_eq!(back.pose.offset, [-11.0, 0.0, 0.0]);
    assert_eq!(back.pose.rotation, [std::f32::consts::FRAC_PI_4, 0.0, 0.0]);
    assert!((back.cubes[0].size[1] - 5.0 * 0.8).abs() < 1.0e-6);
    assert_eq!(back.cubes[0].min, [0.0, -2.0, -2.0]);
    assert_eq!(back.cubes[0].size, [0.0, 4.0, 4.0]);

    // `cross_1` / `cross_2`: the shared 16×4×0 plane at pitches π/4 and 3π/4.
    let cross_1 = &ARROW_PARTS[1];
    let cross_2 = &ARROW_PARTS[2];
    assert_eq!(cross_1.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(
        cross_1.pose.rotation,
        [std::f32::consts::FRAC_PI_4, 0.0, 0.0]
    );
    assert_eq!(cross_1.cubes[0].min, [-12.0, -2.0, 0.0]);
    assert_eq!(cross_1.cubes[0].size, [16.0, 4.0, 0.0]);
    assert_eq!(
        cross_2.pose.rotation,
        [3.0 * std::f32::consts::FRAC_PI_4, 0.0, 0.0]
    );
    assert_eq!(cross_2.cubes[0].size, [16.0, 4.0, 0.0]);

    // Three planes total.
    assert_eq!(count_cubes(&ARROW_PARTS), 3);
}

#[test]
fn arrow_mesh_uses_vanilla_body_layer_geometry() {
    // 3 planes → 18 faces / 72 vertices / 108 indices; the shaft cross and the head carry their tints.
    let arrow = entity_model_mesh(&[EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(arrow.opaque_faces, 18);
    assert_eq!(arrow.vertices.len(), 72);
    assert_eq!(arrow.indices.len(), 108);
    assert!(arrow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARROW_SHAFT, 1.0)));
    assert!(arrow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARROW_HEAD, 1.0)));
}
