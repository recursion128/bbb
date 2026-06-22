use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn shulker_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ShulkerModel.createBodyLayer` (atlas 64×64): three sibling root parts — the 16×12×16
    // lid and the 16×8×16 base (both at offset (0, 24, 0)), and the 6×6×6 head at offset (0, 12, 0).
    assert_eq!(SHULKER_PARTS.len(), 3);

    let lid = &SHULKER_PARTS[0];
    assert_eq!(lid.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(lid.cubes[0].min, [-8.0, -16.0, -8.0]);
    assert_eq!(lid.cubes[0].size, [16.0, 12.0, 16.0]);

    let base = &SHULKER_PARTS[1];
    assert_eq!(base.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(base.cubes[0].min, [-8.0, -8.0, -8.0]);
    assert_eq!(base.cubes[0].size, [16.0, 8.0, 16.0]);

    let head = &SHULKER_PARTS[2];
    assert_eq!(head.pose.offset, [0.0, 12.0, 0.0]);
    assert_eq!(head.cubes[0].min, [-3.0, 0.0, -3.0]);
    assert_eq!(head.cubes[0].size, [6.0, 6.0, 6.0]);

    // Three cubes total.
    assert_eq!(count_cubes(&SHULKER_PARTS), 3);
}

#[test]
fn shulker_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the shell carries the shell tint and the head
    // carries its own yellow tint.
    let shulker = entity_model_mesh(&[EntityModelInstance::shulker(1120, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(shulker.opaque_faces, 18);
    assert_eq!(shulker.vertices.len(), 72);
    assert_eq!(shulker.indices.len(), 108);
    assert!(shulker
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SHULKER_SHELL, 1.0)));
    assert!(shulker
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SHULKER_HEAD, 1.0)));
}
