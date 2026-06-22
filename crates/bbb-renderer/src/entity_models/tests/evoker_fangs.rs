use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn evoker_fangs_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EvokerFangsModel.createBodyLayer` (atlas 64×32): the base block at offset (-5, 24,
    // -5) parents the two jaws (a shared 4×14×8 box) at their closed-jaw bind rotations.
    assert_eq!(EVOKER_FANGS_PARTS.len(), 1);

    let base = &EVOKER_FANGS_PARTS[0];
    assert_eq!(base.pose.offset, [-5.0, 24.0, -5.0]);
    assert_eq!(base.cubes[0].min, [0.0, 0.0, 0.0]);
    assert_eq!(base.cubes[0].size, [10.0, 12.0, 10.0]);
    assert_eq!(base.children.len(), 2);

    // `upper_jaw` at offset (6.5, 0, 1), closed-jaw `zRot = 0.65π = 2.042035`.
    let upper = &base.children[0];
    assert_eq!(upper.pose.offset, [6.5, 0.0, 1.0]);
    assert_eq!(upper.pose.rotation, [0.0, 0.0, 2.042035]);
    let closed_upper_zrot = std::f32::consts::PI - 0.35 * std::f32::consts::PI;
    assert!((upper.pose.rotation[2] - closed_upper_zrot).abs() < 1.0e-4);
    assert_eq!(upper.cubes[0].size, [4.0, 14.0, 8.0]);

    // `lower_jaw` at offset (3.5, 0, 9), `yRot = π` and closed-jaw `zRot = 1.35π = 4.2411504`.
    let lower = &base.children[1];
    assert_eq!(lower.pose.offset, [3.5, 0.0, 9.0]);
    assert_eq!(lower.pose.rotation, [0.0, std::f32::consts::PI, 4.2411504]);
    let closed_lower_zrot = std::f32::consts::PI + 0.35 * std::f32::consts::PI;
    assert!((lower.pose.rotation[2] - closed_lower_zrot).abs() < 1.0e-4);

    // Three cubes total.
    assert_eq!(count_cubes(&EVOKER_FANGS_PARTS), 3);
}

#[test]
fn evoker_fangs_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the base and jaws carry their tints.
    let fangs = entity_model_mesh(&[EntityModelInstance::evoker_fangs(
        470,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(fangs.opaque_faces, 18);
    assert_eq!(fangs.vertices.len(), 72);
    assert_eq!(fangs.indices.len(), 108);
    assert!(fangs
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(EVOKER_FANGS_BASE, 1.0)));
    assert!(fangs
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(EVOKER_FANGS_JAW, 1.0)));
}
