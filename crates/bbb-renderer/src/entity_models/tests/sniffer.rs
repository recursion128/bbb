use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn sniffer_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `SnifferModel.createBodyLayer` (atlas 192×192): the mesh root holds one `bone` part
    // at `offset(0, 5, 0)` parenting the body and the six legs.
    assert_eq!(SNIFFER_PARTS.len(), 1);
    let bone = &SNIFFER_PARTS[0];
    assert_eq!(bone.pose.offset, [0.0, 5.0, 0.0]);
    assert!(bone.cubes.is_empty());
    assert_eq!(bone.children.len(), 7);

    // `body`: the 25×29×40 trunk, the inner block inflated by `CubeDeformation(0.5)`
    // (`min -= 0.5`, `size += 1`), and the 25×0×40 belly plane.
    let body = &bone.children[0];
    assert_eq!(body.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(body.cubes.len(), 3);
    assert_eq!(body.cubes[0].min, [-12.5, -14.0, -20.0]);
    assert_eq!(body.cubes[0].size, [25.0, 29.0, 40.0]);
    // The deformed inner block: base min[-12.5,-14,-20] size[25,24,40] grown by 0.5 on each face.
    assert_eq!(body.cubes[1].min, [-13.0, -14.5, -20.5]);
    assert_eq!(body.cubes[1].size, [26.0, 25.0, 41.0]);
    assert_eq!(body.cubes[2].size, [25.0, 0.0, 40.0]);
    assert_eq!(body.children.len(), 1);

    // `head` (offset (0, 6.5, -19.48)) parents two ears, the nose, and the lower beak.
    let head = &body.children[0];
    assert_eq!(head.pose.offset, [0.0, 6.5, -19.48]);
    assert_eq!(head.cubes.len(), 2);
    assert_eq!(head.cubes[0].size, [13.0, 18.0, 11.0]);
    assert_eq!(head.children.len(), 4);
    assert_eq!(head.children[0].pose.offset, [6.51, -7.5, -4.51]);
    assert_eq!(head.children[0].cubes[0].size, [1.0, 19.0, 7.0]);
    // The nose pad and lower beak.
    assert_eq!(head.children[2].pose.offset, [0.0, -4.5, -11.5]);
    assert_eq!(head.children[2].cubes[0].size, [13.0, 2.0, 9.0]);
    assert_eq!(head.children[3].cubes[0].size, [13.0, 12.0, 9.0]);

    // The six legs share one 7×10×8 box at the standard three pairs of offsets.
    for (i, expected) in [
        [-7.5, 10.0, -15.0],
        [-7.5, 10.0, 0.0],
        [-7.5, 10.0, 15.0],
        [7.5, 10.0, -15.0],
        [7.5, 10.0, 0.0],
        [7.5, 10.0, 15.0],
    ]
    .into_iter()
    .enumerate()
    {
        let leg = &bone.children[i + 1];
        assert_eq!(leg.pose.offset, expected);
        assert_eq!(leg.cubes[0].size, [7.0, 10.0, 8.0]);
    }

    // Fifteen cubes total.
    assert_eq!(count_cubes(&SNIFFER_PARTS), 15);
}

#[test]
fn sniffer_mesh_uses_vanilla_body_layer_geometry() {
    // 15 cubes → 90 faces / 360 vertices / 540 indices; the nose carries its own pink tint.
    let sniffer = entity_model_mesh(&[EntityModelInstance::sniffer(930, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(sniffer.opaque_faces, 90);
    assert_eq!(sniffer.vertices.len(), 360);
    assert_eq!(sniffer.indices.len(), 540);
    assert!(sniffer
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SNIFFER_BROWN, 1.0)));
    assert!(sniffer
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SNIFFER_NOSE, 1.0)));
}

#[test]
fn sniffer_head_follows_look_angles() {
    // Vanilla `SnifferModel.setupAnim` sets `head.xRot/yRot` from the plain look. The head is nested
    // bone → body → head ([`SNIFFER_HEAD_PART_PATH`]); the emit order is body (3 cubes → [0, 72)),
    // then the head subtree (head's 2 cubes + the ear/nose/beak children's 4 = 6 cubes → [72, 216)),
    // then the six legs ([216, 360)). A non-zero look turns only the head subtree (the ears, nose,
    // and beak ride with it); the body and legs stay at bind.
    assert_eq!(SNIFFER_HEAD_PART_PATH, &[0, 0, 0]);
    let base = EntityModelInstance::sniffer(931, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_eq!(
        rest.vertices[..72],
        looking.vertices[..72],
        "the body stays at bind"
    );
    assert_ne!(
        rest.vertices[72..216],
        looking.vertices[72..216],
        "the head, ears, nose, and beak turn with the look"
    );
    assert_eq!(
        rest.vertices[216..],
        looking.vertices[216..],
        "the six legs stay at bind"
    );
}
