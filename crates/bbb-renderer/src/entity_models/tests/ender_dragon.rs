use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn ender_dragon_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EnderDragonModel.createBodyLayer` (atlas 256×256): head (+jaw), five neck and twelve
    // tail spine segments, and the body (+wings +legs) — 19 root parts.
    assert_eq!(ENDER_DRAGON_PARTS.len(), 19);

    // `head` (offset (0, 20, -62)): six cubes, parenting the jaw.
    let head = &ENDER_DRAGON_PARTS[0];
    assert_eq!(head.pose.offset, [0.0, 20.0, -62.0]);
    assert_eq!(head.cubes.len(), 6);
    assert_eq!(head.cubes[1].size, [16.0, 16.0, 16.0]);
    assert_eq!(head.children.len(), 1);
    assert_eq!(head.children[0].pose.offset, [0.0, 4.0, -8.0]);
    assert_eq!(head.children[0].cubes[0].size, [12.0, 4.0, 16.0]);

    // The five neck segments at `offset(0, 20, -12 - i·10)`, each the 2-cube spine.
    for i in 0..5 {
        let neck = &ENDER_DRAGON_PARTS[1 + i];
        assert_eq!(neck.pose.offset, [0.0, 20.0, -12.0 - i as f32 * 10.0]);
        assert_eq!(neck.cubes.len(), 2);
        assert_eq!(neck.cubes[0].size, [10.0, 10.0, 10.0]);
    }

    // The twelve tail segments at `offset(0, 10, 60 + i·10)`, each the 2-cube spine.
    for i in 0..12 {
        let tail = &ENDER_DRAGON_PARTS[6 + i];
        assert_eq!(tail.pose.offset, [0.0, 10.0, 60.0 + i as f32 * 10.0]);
        assert_eq!(tail.cubes.len(), 2);
    }

    // `body` (offset (0, 3, 8)): four cubes, parenting two wings and four legs.
    let body = &ENDER_DRAGON_PARTS[18];
    assert_eq!(body.pose.offset, [0.0, 3.0, 8.0]);
    assert_eq!(body.cubes[0].size, [24.0, 24.0, 64.0]);
    assert_eq!(body.children.len(), 6);

    // `left_wing` (offset (12, 2, -6)): the bone plus the membrane, parenting the wing tip.
    let left_wing = &body.children[0];
    assert_eq!(left_wing.pose.offset, [12.0, 2.0, -6.0]);
    assert_eq!(left_wing.cubes[0].size, [56.0, 8.0, 8.0]);
    assert_eq!(left_wing.cubes[1].size, [56.0, 0.0, 56.0]);
    assert_eq!(left_wing.children[0].pose.offset, [56.0, 0.0, 0.0]);

    // A front leg is a three-segment chain (leg → tip → foot) with the vanilla bind rotations.
    let left_front_leg = &body.children[1];
    assert_eq!(left_front_leg.pose.offset, [12.0, 17.0, -6.0]);
    assert_eq!(left_front_leg.pose.rotation, [1.3, 0.0, 0.0]);
    let leg_tip = &left_front_leg.children[0];
    assert_eq!(leg_tip.pose.rotation, [-0.5, 0.0, 0.0]);
    let foot = &leg_tip.children[0];
    assert_eq!(foot.pose.rotation, [0.75, 0.0, 0.0]);
    assert_eq!(foot.cubes[0].size, [8.0, 4.0, 16.0]);

    // The right wing extends -X (vanilla's mirror is true geometry).
    let right_wing = &body.children[3];
    assert_eq!(right_wing.pose.offset, [-12.0, 2.0, -6.0]);
    assert_eq!(right_wing.cubes[0].min, [-56.0, -4.0, -4.0]);

    // Sixty-five cubes total.
    assert_eq!(count_cubes(&ENDER_DRAGON_PARTS), 65);
}

#[test]
fn ender_dragon_mesh_uses_vanilla_body_layer_geometry() {
    // 65 cubes → 390 faces / 1560 vertices / 2340 indices; the body is dark and the wing membranes
    // carry their lighter tint.
    let dragon = entity_model_mesh(&[EntityModelInstance::ender_dragon(
        430,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(dragon.opaque_faces, 390);
    assert_eq!(dragon.vertices.len(), 1560);
    assert_eq!(dragon.indices.len(), 2340);
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_BODY, 1.0)));
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_MEMBRANE, 1.0)));
}
