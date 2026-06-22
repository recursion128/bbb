use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn tadpole_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `TadpoleModel.createBodyLayer` (atlas 16×16): two sibling root parts — a 3×2×3 body
    // box at offset (0, 22, -3) and a 0×2×7 tail fin plane at offset (0, 22, 0).
    assert_eq!(TADPOLE_PARTS.len(), 2);

    let body = &TADPOLE_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 22.0, -3.0]);
    assert!(body.children.is_empty());
    assert_eq!(body.cubes[0].min, [-1.5, -1.0, 0.0]);
    assert_eq!(body.cubes[0].size, [3.0, 2.0, 3.0]);

    let tail = &TADPOLE_PARTS[1];
    assert_eq!(tail.pose.offset, [0.0, 22.0, 0.0]);
    assert_eq!(tail.cubes[0].min, [0.0, -1.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [0.0, 2.0, 7.0]);

    // Two cubes total.
    assert_eq!(count_cubes(&TADPOLE_PARTS), 2);
}

#[test]
fn tadpole_mesh_uses_vanilla_body_layer_geometry() {
    // The body box contributes 6 faces; the tail is a zero-width plane (front/back quads only). The
    // body carries the body tint and the tail carries its own fin tint.
    let tadpole = entity_model_mesh(&[EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0)]);
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_BODY, 1.0)));
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_TAIL, 1.0)));
}
