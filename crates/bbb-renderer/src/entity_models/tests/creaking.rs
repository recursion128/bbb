use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn creaking_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `CreakingModel.createBodyLayer` (atlas 64×64): the mesh root holds one `root` part
    // at `offset(0, 24, 0)` parenting the `upper_body` pivot and the two legs.
    assert_eq!(CREAKING_PARTS.len(), 1);
    let root = &CREAKING_PARTS[0];
    assert_eq!(root.pose.offset, [0.0, 24.0, 0.0]);
    assert!(root.cubes.is_empty());
    assert_eq!(root.children.len(), 3);

    // `upper_body` (empty pivot at (-1, -19, 0)) parents head / body / right_arm / left_arm.
    let upper_body = &root.children[0];
    assert_eq!(upper_body.pose.offset, [-1.0, -19.0, 0.0]);
    assert!(upper_body.cubes.is_empty());
    assert_eq!(upper_body.children.len(), 4);

    // `head`: the 6×10×6 skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes.
    let head = &upper_body.children[0];
    assert_eq!(head.pose.offset, [-3.0, -11.0, 0.0]);
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].min, [-3.0, -10.0, -3.0]);
    assert_eq!(head.cubes[0].size, [6.0, 10.0, 6.0]);
    assert_eq!(head.cubes[2].size, [9.0, 14.0, 0.0]);
    assert_eq!(head.cubes[3].min, [-12.0, -14.0, 0.0]);

    // `body`: the 6×13×5 trunk plus the 6×7×5 block.
    let body = &upper_body.children[1];
    assert_eq!(body.pose.offset, [0.0, -7.0, 1.0]);
    assert_eq!(body.cubes[0].size, [6.0, 13.0, 5.0]);
    assert_eq!(body.cubes[1].min, [-6.0, -4.0, -3.0]);

    // The asymmetric arms: right is a 3×21×3 limb + hand, left a 3×16×3 limb + two blocks.
    let right_arm = &upper_body.children[2];
    let left_arm = &upper_body.children[3];
    assert_eq!(right_arm.pose.offset, [-7.0, -9.5, 1.5]);
    assert_eq!(right_arm.cubes[0].size, [3.0, 21.0, 3.0]);
    assert_eq!(left_arm.pose.offset, [6.0, -9.0, 0.5]);
    assert_eq!(left_arm.cubes.len(), 3);

    // The legs (each with a 5×0×9 foot plane); the right leg has an extra 3×3×3 hip block.
    let left_leg = &root.children[1];
    let right_leg = &root.children[2];
    assert_eq!(left_leg.pose.offset, [1.5, -16.0, 0.5]);
    assert_eq!(left_leg.cubes[1].size, [5.0, 0.0, 9.0]);
    assert_eq!(right_leg.pose.offset, [-1.0, -17.5, 0.5]);
    assert_eq!(right_leg.cubes.len(), 3);
    assert_eq!(right_leg.cubes[2].size, [3.0, 3.0, 3.0]);

    // Sixteen cubes total.
    assert_eq!(count_cubes(&CREAKING_PARTS), 16);
}

#[test]
fn creaking_mesh_uses_vanilla_body_layer_geometry() {
    // 16 cubes → 96 faces / 384 vertices / 576 indices, all in the bark tint.
    let creaking = entity_model_mesh(&[EntityModelInstance::creaking(940, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(creaking.opaque_faces, 96);
    assert_eq!(creaking.vertices.len(), 384);
    assert_eq!(creaking.indices.len(), 576);
    assert!(creaking
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CREAKING_BARK, 1.0)));
}

#[test]
fn creaking_head_follows_look_angles() {
    // Vanilla `CreakingModel.setupAnim` sets `head.xRot/yRot` from the plain look. The head is
    // nested root → upper_body → head ([`CREAKING_HEAD_PART_PATH`]) and is emitted first — its four
    // cubes (skull, brow, two antler planes) are vertices [0, 96). A non-zero look re-poses only the
    // head subtree; the body, the two arms, and the two legs stay at bind.
    assert_eq!(CREAKING_HEAD_PART_PATH, &[0, 0, 0]);
    let base = EntityModelInstance::creaking(941, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_ne!(
        rest.vertices[..96],
        looking.vertices[..96],
        "the head turns with the look"
    );
    assert_eq!(
        rest.vertices[96..],
        looking.vertices[96..],
        "the body, arms, and legs stay at bind"
    );
}
