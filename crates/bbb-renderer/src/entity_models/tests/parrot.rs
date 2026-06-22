use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn parrot_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ParrotModel.createBodyLayer` (atlas 32×32): seven sibling root parts — body, tail,
    // two wings, head (parenting head2, the two beak halves, and the crest feather), two legs.
    assert_eq!(PARROT_PARTS.len(), 7);

    // `body` (3×6×3) pitched by 0.4937 rad.
    let body = &PARROT_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 16.5, -3.0]);
    assert_eq!(body.pose.rotation, [0.4937, 0.0, 0.0]);
    assert_eq!(body.cubes[0].size, [3.0, 6.0, 3.0]);

    // `tail` (3×4×1) pitched by 1.015 rad.
    let tail = &PARROT_PARTS[1];
    assert_eq!(tail.pose.offset, [0.0, 21.07, 1.16]);
    assert_eq!(tail.pose.rotation, [1.015, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [3.0, 4.0, 1.0]);

    // The two 1×5×3 wings: mirrored pivots, both flipped yRot = -π.
    let left_wing = &PARROT_PARTS[2];
    let right_wing = &PARROT_PARTS[3];
    assert_eq!(left_wing.pose.offset, [1.5, 16.94, -2.76]);
    assert_eq!(
        left_wing.pose.rotation,
        [-0.6981, -std::f32::consts::PI, 0.0]
    );
    assert_eq!(right_wing.pose.offset, [-1.5, 16.94, -2.76]);
    assert_eq!(right_wing.cubes[0].size, [1.0, 5.0, 3.0]);

    // `head` (2×3×2) at offset (0, 15.69, -2.76), parenting four cubes.
    let head = &PARROT_PARTS[4];
    assert_eq!(head.pose.offset, [0.0, 15.69, -2.76]);
    assert_eq!(head.cubes[0].size, [2.0, 3.0, 2.0]);
    assert_eq!(head.children.len(), 4);
    // head2 2×1×4, beak1 / beak2 1×2×1, the crest feather 0×5×4 pitched by -0.2214 rad.
    assert_eq!(head.children[0].cubes[0].size, [2.0, 1.0, 4.0]);
    assert_eq!(head.children[1].cubes[0].size, [1.0, 2.0, 1.0]);
    assert_eq!(head.children[3].pose.rotation, [-0.2214, 0.0, 0.0]);
    assert_eq!(head.children[3].cubes[0].size, [0.0, 5.0, 4.0]);

    // The two 1×2×1 legs at the mirrored pivots, both pitched by -0.0299 rad.
    let left_leg = &PARROT_PARTS[5];
    let right_leg = &PARROT_PARTS[6];
    assert_eq!(left_leg.pose.offset, [1.0, 22.0, -1.05]);
    assert_eq!(left_leg.pose.rotation, [-0.0299, 0.0, 0.0]);
    assert_eq!(right_leg.pose.offset, [-1.0, 22.0, -1.05]);

    // Eleven cubes total.
    assert_eq!(count_cubes(&PARROT_PARTS), 11);
}

#[test]
fn parrot_mesh_uses_vanilla_body_layer_geometry() {
    // The body carries the body tint; the two beak halves carry the beak tint.
    let parrot = entity_model_mesh(&[EntityModelInstance::parrot(980, [0.0, 64.0, 0.0], 0.0)]);
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BODY, 1.0)));
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BEAK, 1.0)));
}
