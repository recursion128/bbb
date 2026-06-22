use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn trident_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `TridentModel.createLayer` (atlas 32×32): the `pole` shaft parents the `base`
    // crossguard and the three spikes, all at ZERO.
    assert_eq!(TRIDENT_PARTS.len(), 1);

    let pole = &TRIDENT_PARTS[0];
    assert_eq!(pole.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(pole.cubes[0].min, [-0.5, 2.0, -0.5]);
    assert_eq!(pole.cubes[0].size, [1.0, 25.0, 1.0]);
    assert_eq!(pole.children.len(), 4);

    // `base` (3×2×1 crossguard).
    let base = &pole.children[0];
    assert_eq!(base.cubes[0].min, [-1.5, 0.0, -0.5]);
    assert_eq!(base.cubes[0].size, [3.0, 2.0, 1.0]);

    // The three 1×4×1 spikes at their X origins.
    let left = &pole.children[1];
    let middle = &pole.children[2];
    let right = &pole.children[3];
    assert_eq!(left.cubes[0].min, [-2.5, -3.0, -0.5]);
    assert_eq!(middle.cubes[0].min, [-0.5, -4.0, -0.5]);
    assert_eq!(right.cubes[0].min, [1.5, -3.0, -0.5]);
    assert_eq!(left.cubes[0].size, [1.0, 4.0, 1.0]);

    // Five cubes total.
    assert_eq!(count_cubes(&TRIDENT_PARTS), 5);
}

#[test]
fn trident_mesh_uses_vanilla_body_layer_geometry() {
    // 5 cubes → 30 faces / 120 vertices / 180 indices; the pole and the spikes carry their tints.
    let trident = entity_model_mesh(&[EntityModelInstance::trident(1350, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(trident.opaque_faces, 30);
    assert_eq!(trident.vertices.len(), 120);
    assert_eq!(trident.indices.len(), 180);
    assert!(trident
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TRIDENT_POLE, 1.0)));
    assert!(trident
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TRIDENT_SPIKE, 1.0)));
}
