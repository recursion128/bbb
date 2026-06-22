use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn leash_knot_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `LeashKnotModel.createBodyLayer` (atlas 32×32): a single `knot` part at ZERO with one
    // 6×8×6 box.
    assert_eq!(LEASH_KNOT_PARTS.len(), 1);
    let knot = &LEASH_KNOT_PARTS[0];
    assert_eq!(knot.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(knot.cubes[0].min, [-3.0, -8.0, -3.0]);
    assert_eq!(knot.cubes[0].size, [6.0, 8.0, 6.0]);
    assert_eq!(count_cubes(&LEASH_KNOT_PARTS), 1);
}

#[test]
fn leash_knot_mesh_uses_vanilla_body_layer_geometry() {
    // 1 cube → 6 faces / 24 vertices / 36 indices, carrying the knot tint.
    let knot = entity_model_mesh(&[EntityModelInstance::leash_knot(760, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(knot.opaque_faces, 6);
    assert_eq!(knot.vertices.len(), 24);
    assert_eq!(knot.indices.len(), 36);
    assert!(knot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LEASH_KNOT_COLOR, 1.0)));
}
