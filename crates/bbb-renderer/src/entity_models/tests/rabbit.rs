use super::*;

#[test]
fn adult_rabbit_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultRabbitModel.createBodyLayer` (atlas 64×64): the mesh root holds the `body`
    // (pitched -0.3927, carrying the tail, head, and the cubeless `frontlegs` pivot) and the
    // cubeless `backlegs` pivot (carrying the two hind legs, each parenting a haunch).

    // `body` (offset (0, 23, 4), rot -0.3927): the 8×6×10 torso.
    assert_eq!(RABBIT_BODY_POSE.offset, [0.0, 23.0, 4.0]);
    assert_eq!(RABBIT_BODY_POSE.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(RABBIT_BODY_CUBES.len(), 1);
    assert_eq!(RABBIT_BODY_CUBES[0].min, [-4.0, -6.0, -9.0]);
    assert_eq!(RABBIT_BODY_CUBES[0].size, [8.0, 6.0, 10.0]);

    // `tail` (offset (0, -4.9916, 0.0125)): the 4×4×4 puff.
    assert_eq!(RABBIT_TAIL_POSE.offset, [0.0, -4.9916, 0.0125]);
    assert_eq!(RABBIT_TAIL_CUBES[0].size, [4.0, 4.0, 4.0]);

    // `head` (offset (0, -5.2929, -8.1213), rot 0.3927): the 5×5×5 skull parenting the two ears.
    assert_eq!(RABBIT_HEAD_POSE.offset, [0.0, -5.2929, -8.1213]);
    assert_eq!(RABBIT_HEAD_POSE.rotation, [0.3927, 0.0, 0.0]);
    assert_eq!(RABBIT_HEAD_CUBES[0].min, [-2.5, -3.0, -4.0]);
    assert_eq!(RABBIT_HEAD_CUBES[0].size, [5.0, 5.0, 5.0]);
    // The two 2×5×1 ears share their box geometry, differing only in the pivot X sign (and, on the
    // textured path, in their per-side texOffs — so the box is now a per-side cube const).
    assert_eq!(RABBIT_RIGHT_EAR_POSE.offset, [1.5, -3.7071, -0.8787]);
    assert_eq!(RABBIT_LEFT_EAR_POSE.offset, [-1.5, -3.7071, -0.8787]);
    assert_eq!(RABBIT_RIGHT_EAR_CUBES[0].min, [-1.0, -4.2929, -0.1213]);
    assert_eq!(RABBIT_RIGHT_EAR_CUBES[0].size, [2.0, 5.0, 1.0]);
    assert_eq!(RABBIT_LEFT_EAR_CUBES[0].min, [-1.0, -4.2929, -0.1213]);
    assert_eq!(RABBIT_LEFT_EAR_CUBES[0].size, [2.0, 5.0, 1.0]);

    // `frontlegs` (offset (0, -1.5349, -6.3108)): a cubeless pivot parenting the two front legs.
    assert_eq!(RABBIT_FRONTLEGS_POSE.offset, [0.0, -1.5349, -6.3108]);
    // Both 2×4×2 legs share the 0.3927 pitch; the right leg's box is nudged -0.9 on X.
    assert_eq!(RABBIT_RIGHT_FRONT_LEG_POSE.offset, [-2.0, 1.9239, 0.3827]);
    assert_eq!(RABBIT_RIGHT_FRONT_LEG_POSE.rotation, [0.3927, 0.0, 0.0]);
    assert_eq!(RABBIT_RIGHT_FRONT_LEG_CUBES[0].min, [-0.9, -1.0, -0.9]);
    assert_eq!(RABBIT_RIGHT_FRONT_LEG_CUBES[0].size, [2.0, 4.0, 2.0]);
    assert_eq!(RABBIT_LEFT_FRONT_LEG_POSE.offset, [2.0, 1.9239, 0.4827]);
    assert_eq!(RABBIT_LEFT_FRONT_LEG_CUBES[0].min, [-1.0, -1.0, -1.0]);

    // `backlegs` (offset (0, 23, 4)): a cubeless pivot parenting the two hind legs.
    assert_eq!(RABBIT_BACKLEGS_POSE.offset, [0.0, 23.0, 4.0]);

    // Each hind leg is a cubeless pivot; its haunch carries the only cube, yawed ±0.3927.
    assert_eq!(RABBIT_RIGHT_HIND_LEG_POSE.offset, [-3.0, 0.5, 0.0]);
    assert_eq!(RABBIT_RIGHT_HAUNCH_POSE.offset, [0.0, -0.5, 0.0]);
    assert_eq!(RABBIT_RIGHT_HAUNCH_POSE.rotation, [0.0, 0.3927, 0.0]);
    assert_eq!(RABBIT_RIGHT_HAUNCH_CUBES[0].min, [-1.0, 0.0, -5.0]);
    assert_eq!(RABBIT_RIGHT_HAUNCH_CUBES[0].size, [2.0, 1.0, 6.0]);
    assert_eq!(RABBIT_LEFT_HIND_LEG_POSE.offset, [3.0, 0.5, 0.0]);
    assert_eq!(RABBIT_LEFT_HAUNCH_POSE.rotation, [0.0, -0.3927, 0.0]);
}

#[test]
fn rabbit_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices, all in the one rabbit brown tint (the
    // per-face directional shading varies the brightness, so the unshaded face carries the tint).
    let rabbit = entity_model_mesh(&[EntityModelInstance::rabbit(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
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
    let instances = [EntityModelInstance::rabbit(
        701,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )];
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
    let rest = EntityModelInstance::rabbit(702, [0.0, 64.0, 0.0], 0.0, false);
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
fn baby_rabbit_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyRabbitModel.createBodyLayer` (atlas 32×32): a deeper `_r1`-nested layout. The
    // cubeless `body` pivot parents `body_r1` / `tail` / `head` / `frontlegs`; the head is `body`'s
    // THIRD child (unlike the adult's second).
    assert_eq!(BABY_RABBIT_BODY_POSE.offset, [0.0, 23.0, 1.6]);

    // `body_r1` (pitched -0.5236): the 4×3×6 trunk.
    assert_eq!(BABY_RABBIT_BODY_R1_POSE.rotation, [-0.5236, 0.0, 0.0]);
    assert_eq!(BABY_RABBIT_BODY_CUBES[0].size, [4.0, 3.0, 6.0]);

    // `tail` (cubeless) parents the pitched `tail_r1`.
    assert_eq!(BABY_RABBIT_TAIL_R1_POSE.rotation, [-0.5236, 0.0, 0.0]);
    assert_eq!(BABY_RABBIT_TAIL_CUBES[0].size, [3.0, 3.0, 3.0]);

    // `head`: the 5×4×4 skull parenting the two 2×4×1 ears.
    assert_eq!(BABY_RABBIT_HEAD_POSE.offset, [0.0, -5.0, -2.6]);
    assert_eq!(BABY_RABBIT_HEAD_CUBES[0].size, [5.0, 4.0, 4.0]);
    assert_eq!(BABY_RABBIT_RIGHT_EAR_POSE.offset, [-1.5, -3.5, -0.5]);
    assert_eq!(BABY_RABBIT_RIGHT_EAR_CUBES[0].size, [2.0, 4.0, 1.0]);

    // `frontlegs` (cubeless) → each front leg (cubeless, pitched 0.3927) → its `_r1` cube.
    assert_eq!(BABY_RABBIT_LEFT_FRONT_LEG_POSE.offset, [1.0, 1.0, -0.5]);
    assert_eq!(BABY_RABBIT_LEFT_FRONT_LEG_POSE.rotation, [0.3927, 0.0, 0.0]);
    assert_eq!(BABY_RABBIT_FRONT_LEG_R1_POSE.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(BABY_RABBIT_LEFT_FRONT_LEG_CUBES[0].size, [1.0, 3.0, 1.0]);

    // `backlegs` (cubeless) → each hind leg (cubeless, yawed π) → its yawed haunch.
    assert_eq!(BABY_RABBIT_BACKLEGS_POSE.offset, [0.0, 23.0, 2.0]);
    assert_eq!(BABY_RABBIT_LEFT_HIND_LEG_POSE.rotation, [0.0, 3.1416, 0.0]);
    assert_eq!(BABY_RABBIT_LEFT_HAUNCH_POSE.rotation, [0.0, -0.7854, 0.0]);
    assert_eq!(BABY_RABBIT_LEFT_HAUNCH_CUBES[0].size, [2.0, 1.0, 3.0]);
}

#[test]
fn baby_rabbit_mesh_and_head_look() {
    // The baby has the same pre-order cube layout as the adult (body/tail `[0, 48)`, head + ears
    // `[48, 120)`, legs + haunches `[120, 216)`), so the head look isolates the head subtree, and the
    // baby mesh is more compact than the adult.
    let rest = EntityModelInstance::rabbit(710, [0.0, 64.0, 0.0], 0.0, true);
    let baby = entity_model_mesh(&[rest]);
    assert_eq!(baby.vertices.len(), 216);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(RABBIT_BROWN, 1.0)));

    let looked = entity_model_mesh(&[rest.with_head_look(35.0, -25.0)]);
    assert_eq!(baby.vertices[..48], looked.vertices[..48]);
    assert_ne!(baby.vertices[48..120], looked.vertices[48..120]);
    assert_eq!(baby.vertices[120..], looked.vertices[120..]);

    let adult = entity_model_mesh(&[EntityModelInstance::rabbit(
        711,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!((baby_max[2] - baby_min[2]) < (adult_max[2] - adult_min[2]));
}

#[test]
fn rabbit_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Rabbit { baby: false }.model_key(),
        "rabbit"
    );
    assert_eq!(
        EntityModelKind::Rabbit { baby: true }.model_key(),
        "rabbit_baby"
    );
}
