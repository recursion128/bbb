use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn adult_rabbit_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultRabbitModel.createBodyLayer` (atlas 64×64): the mesh root holds the `body`
    // (pitched -0.3927, carrying the tail, head, and the cubeless `frontlegs` pivot) and the
    // cubeless `backlegs` pivot (carrying the two hind legs, each parenting a haunch).
    assert_eq!(ADULT_RABBIT_PARTS.len(), 2);

    // `body` (offset (0, 23, 4), rot -0.3927): the 8×6×10 torso.
    let body = &ADULT_RABBIT_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 23.0, 4.0]);
    assert_eq!(body.pose.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(body.cubes.len(), 1);
    assert_eq!(body.cubes[0].min, [-4.0, -6.0, -9.0]);
    assert_eq!(body.cubes[0].size, [8.0, 6.0, 10.0]);
    assert_eq!(body.children.len(), 3);

    // `tail` (offset (0, -4.9916, 0.0125)): the 4×4×4 puff.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [0.0, -4.9916, 0.0125]);
    assert_eq!(tail.cubes[0].size, [4.0, 4.0, 4.0]);

    // `head` (offset (0, -5.2929, -8.1213), rot 0.3927): the 5×5×5 skull parenting the two ears.
    let head = &body.children[1];
    assert_eq!(head.pose.offset, [0.0, -5.2929, -8.1213]);
    assert_eq!(head.pose.rotation, [0.3927, 0.0, 0.0]);
    assert_eq!(head.cubes[0].min, [-2.5, -3.0, -4.0]);
    assert_eq!(head.cubes[0].size, [5.0, 5.0, 5.0]);
    assert_eq!(head.children.len(), 2);
    // The two 2×5×1 ears share their box, differing only in the pivot X sign.
    assert_eq!(head.children[0].pose.offset, [1.5, -3.7071, -0.8787]);
    assert_eq!(head.children[1].pose.offset, [-1.5, -3.7071, -0.8787]);
    assert_eq!(head.children[0].cubes[0].min, [-1.0, -4.2929, -0.1213]);
    assert_eq!(head.children[0].cubes[0].size, [2.0, 5.0, 1.0]);

    // `frontlegs` (offset (0, -1.5349, -6.3108)): a cubeless pivot parenting the two front legs.
    let front_legs = &body.children[2];
    assert_eq!(front_legs.pose.offset, [0.0, -1.5349, -6.3108]);
    assert!(front_legs.cubes.is_empty());
    assert_eq!(front_legs.children.len(), 2);
    // Both 2×4×2 legs share the 0.3927 pitch; the right leg's box is nudged -0.9 on X.
    assert_eq!(front_legs.children[0].pose.offset, [-2.0, 1.9239, 0.3827]);
    assert_eq!(front_legs.children[0].pose.rotation, [0.3927, 0.0, 0.0]);
    assert_eq!(front_legs.children[0].cubes[0].min, [-0.9, -1.0, -0.9]);
    assert_eq!(front_legs.children[0].cubes[0].size, [2.0, 4.0, 2.0]);
    assert_eq!(front_legs.children[1].pose.offset, [2.0, 1.9239, 0.4827]);
    assert_eq!(front_legs.children[1].cubes[0].min, [-1.0, -1.0, -1.0]);

    // `backlegs` (offset (0, 23, 4)): a cubeless pivot parenting the two hind legs.
    let back_legs = &ADULT_RABBIT_PARTS[1];
    assert_eq!(back_legs.pose.offset, [0.0, 23.0, 4.0]);
    assert!(back_legs.cubes.is_empty());
    assert_eq!(back_legs.children.len(), 2);

    // Each hind leg is a cubeless pivot; its haunch carries the only cube, yawed ±0.3927.
    let right_hind = &back_legs.children[0];
    assert_eq!(right_hind.pose.offset, [-3.0, 0.5, 0.0]);
    assert!(right_hind.cubes.is_empty());
    let right_haunch = &right_hind.children[0];
    assert_eq!(right_haunch.pose.offset, [0.0, -0.5, 0.0]);
    assert_eq!(right_haunch.pose.rotation, [0.0, 0.3927, 0.0]);
    assert_eq!(right_haunch.cubes[0].min, [-1.0, 0.0, -5.0]);
    assert_eq!(right_haunch.cubes[0].size, [2.0, 1.0, 6.0]);
    let left_hind = &back_legs.children[1];
    assert_eq!(left_hind.pose.offset, [3.0, 0.5, 0.0]);
    assert_eq!(left_hind.children[0].pose.rotation, [0.0, -0.3927, 0.0]);

    // Nine cubes (body, tail, head, two ears, two front legs, two haunches).
    assert_eq!(count_cubes(&ADULT_RABBIT_PARTS), 9);
}

#[test]
fn rabbit_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices, all in the one rabbit brown tint (the
    // per-face directional shading varies the brightness, so the unshaded face carries the tint).
    let rabbit = entity_model_mesh(&[EntityModelInstance::rabbit(700, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(rabbit.opaque_faces, 54);
    assert_eq!(rabbit.vertices.len(), 216);
    assert_eq!(rabbit.indices.len(), 324);
    assert!(rabbit
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(RABBIT_BROWN, 1.0)));
}

#[test]
fn rabbit_mesh_matches_on_both_render_paths() {
    // The rabbit is a colored-only entity, so the texture-skipping colored runtime path emits the
    // exact same mesh as the full path (unlike the texture-backed wolf proxy it replaced).
    let instances = [EntityModelInstance::rabbit(701, [0.0, 64.0, 0.0], 0.0)];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn rabbit_head_look_turns_only_the_head_subtree() {
    // Vanilla `RabbitModel.setupAnim` sets `head.yRot/xRot` from the look angles (overwriting the
    // head's baked 0.3927 pitch, since vanilla assigns rather than adds). The head is `body`'s
    // second child, so only the head and its two ears turn. Pre-order emit: body/tail `[0, 48)`,
    // the head plus its two ears `[48, 120)`, then the front legs and haunches `[120, 216)`.
    let rest = EntityModelInstance::rabbit(702, [0.0, 64.0, 0.0], 0.0);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..48],
        looked_mesh.vertices[..48],
        "the body and tail stay put"
    );
    assert_ne!(
        rest_mesh.vertices[48..120],
        looked_mesh.vertices[48..120],
        "the head and its two ears turn"
    );
    assert_eq!(
        rest_mesh.vertices[120..],
        looked_mesh.vertices[120..],
        "the front legs and haunches stay put"
    );

    // Both the yaw and the pitch move the head (vanilla sets `head.yRot` and `head.xRot`).
    let yaw_only = entity_model_mesh(&[rest.with_head_look(35.0, 0.0)]);
    let pitch_only = entity_model_mesh(&[rest.with_head_look(0.0, -25.0)]);
    assert_ne!(rest_mesh.vertices[48..120], yaw_only.vertices[48..120]);
    assert_ne!(rest_mesh.vertices[48..120], pitch_only.vertices[48..120]);
}

#[test]
fn rabbit_exposes_stable_model_key() {
    assert_eq!(EntityModelKind::Rabbit.model_key(), "rabbit");
}
