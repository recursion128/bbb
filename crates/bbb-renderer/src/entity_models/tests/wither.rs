use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn wither_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` (atlas 64×64): six sibling
    // root parts — shoulders, ribcage (spine + three ribs), tail, center head, two side heads.
    assert_eq!(WITHER_PARTS.len(), 6);

    // `shoulders` (20×3×3) at ZERO.
    let shoulders = &WITHER_PARTS[0];
    assert_eq!(shoulders.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(shoulders.cubes[0].min, [-10.0, 3.9, -0.5]);
    assert_eq!(shoulders.cubes[0].size, [20.0, 3.0, 3.0]);

    // `ribcage` (offset (-2, 6.9, -0.5), pitched 0.20420352 rad): the spine plus three rib bars.
    let ribcage = &WITHER_PARTS[1];
    assert_eq!(ribcage.pose.offset, [-2.0, 6.9, -0.5]);
    assert_eq!(ribcage.pose.rotation, [0.204_203_52, 0.0, 0.0]);
    assert_eq!(ribcage.cubes.len(), 4);
    assert_eq!(ribcage.cubes[0].size, [3.0, 10.0, 3.0]);
    assert_eq!(ribcage.cubes[1].min, [-4.0, 1.5, 0.5]);
    assert_eq!(ribcage.cubes[2].min, [-4.0, 4.0, 0.5]);
    assert_eq!(ribcage.cubes[3].min, [-4.0, 6.5, 0.5]);
    assert_eq!(ribcage.cubes[1].size, [11.0, 2.0, 2.0]);

    // `tail` (3×6×3) at the bind position derived from the ribcage bind pitch.
    let tail = &WITHER_PARTS[2];
    let ribcage_bind_xrot = 0.20420352_f32;
    let expected_tail_y = 6.9 + ribcage_bind_xrot.cos() * 10.0;
    let expected_tail_z = -0.5 + ribcage_bind_xrot.sin() * 10.0;
    assert!((tail.pose.offset[1] - expected_tail_y).abs() < 1.0e-4);
    assert!((tail.pose.offset[2] - expected_tail_z).abs() < 1.0e-4);
    assert_eq!(tail.pose.rotation, [0.832_522_03, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [3.0, 6.0, 3.0]);

    // `center_head` (8×8×8) at ZERO; the two 6×6×6 side heads at their pivots.
    let center_head = &WITHER_PARTS[3];
    assert_eq!(center_head.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(center_head.cubes[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(WITHER_PARTS[4].pose.offset, [-8.0, 4.0, 0.0]);
    assert_eq!(WITHER_PARTS[5].pose.offset, [10.0, 4.0, 0.0]);
    assert_eq!(WITHER_PARTS[4].cubes[0].size, [6.0, 6.0, 6.0]);

    // Nine cubes total.
    assert_eq!(count_cubes(&WITHER_PARTS), 9);
}

#[test]
fn wither_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices; the body carries the body tint and the three
    // heads carry the head tint.
    let wither = entity_model_mesh(&[EntityModelInstance::wither(1450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(wither.opaque_faces, 54);
    assert_eq!(wither.vertices.len(), 216);
    assert_eq!(wither.indices.len(), 324);
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_BODY, 1.0)));
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_HEAD, 1.0)));
}
