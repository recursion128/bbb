use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn warden_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WardenModel.createBodyLayer` (atlas 128×128): the mesh root holds one `bone` part
    // at `offset(0, 24, 0)` parenting the body and the two legs.
    assert_eq!(WARDEN_PARTS.len(), 1);
    let bone = &WARDEN_PARTS[0];
    assert_eq!(bone.pose.offset, [0.0, 24.0, 0.0]);
    assert!(bone.cubes.is_empty());
    assert_eq!(bone.children.len(), 3);

    // `body` (offset (0, -21, 0)): one 18×21×11 box parenting two ribcages, the head, and arms.
    let body = &bone.children[0];
    assert_eq!(body.pose.offset, [0.0, -21.0, 0.0]);
    assert_eq!(body.cubes.len(), 1);
    assert_eq!(body.cubes[0].min, [-9.0, -13.0, -4.0]);
    assert_eq!(body.cubes[0].size, [18.0, 21.0, 11.0]);
    assert_eq!(body.children.len(), 5);

    // The two 9×21×0 ribcage planes.
    let right_ribcage = &body.children[0];
    let left_ribcage = &body.children[1];
    assert_eq!(right_ribcage.pose.offset, [-7.0, -2.0, -4.0]);
    assert_eq!(right_ribcage.cubes[0].min, [-2.0, -11.0, -0.1]);
    assert_eq!(left_ribcage.pose.offset, [7.0, -2.0, -4.0]);
    assert_eq!(left_ribcage.cubes[0].min, [-7.0, -11.0, -0.1]);
    assert_eq!(right_ribcage.cubes[0].size, [9.0, 21.0, 0.0]);

    // `head` (16×16×10) parents the two 16×16×0 tendril planes.
    let head = &body.children[2];
    assert_eq!(head.pose.offset, [0.0, -13.0, 0.0]);
    assert_eq!(head.cubes[0].size, [16.0, 16.0, 10.0]);
    assert_eq!(head.children.len(), 2);
    assert_eq!(head.children[0].pose.offset, [-8.0, -12.0, 0.0]);
    assert_eq!(head.children[0].cubes[0].min, [-16.0, -13.0, 0.0]);
    assert_eq!(head.children[1].cubes[0].min, [0.0, -13.0, 0.0]);
    assert_eq!(head.children[1].cubes[0].size, [16.0, 16.0, 0.0]);

    // The two 8×28×8 arms.
    let right_arm = &body.children[3];
    let left_arm = &body.children[4];
    assert_eq!(right_arm.pose.offset, [-13.0, -13.0, 1.0]);
    assert_eq!(left_arm.pose.offset, [13.0, -13.0, 1.0]);
    assert_eq!(right_arm.cubes[0].size, [8.0, 28.0, 8.0]);

    // The two 6×13×6 legs (differing only in X origin).
    let right_leg = &bone.children[1];
    let left_leg = &bone.children[2];
    assert_eq!(right_leg.pose.offset, [-5.9, -13.0, 0.0]);
    assert_eq!(right_leg.cubes[0].min, [-3.1, 0.0, -3.0]);
    assert_eq!(left_leg.pose.offset, [5.9, -13.0, 0.0]);
    assert_eq!(left_leg.cubes[0].min, [-2.9, 0.0, -3.0]);

    // Ten cubes total.
    assert_eq!(count_cubes(&WARDEN_PARTS), 10);
}

#[test]
fn warden_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices; the tendrils carry their own cyan tint.
    let warden = entity_model_mesh(&[EntityModelInstance::warden(920, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(warden.opaque_faces, 60);
    assert_eq!(warden.vertices.len(), 240);
    assert_eq!(warden.indices.len(), 360);
    assert!(warden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WARDEN_BODY, 1.0)));
    assert!(warden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WARDEN_TENDRIL, 1.0)));
}
