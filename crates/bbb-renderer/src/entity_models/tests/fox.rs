use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn fox_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultFoxModel.createBodyLayer` (atlas 48×32): six root parts — head (with ears + snout),
    // body (with tail), and four legs.
    assert_eq!(FOX_PARTS.len(), 6);

    // `head` (offset (-1, 16.5, -3)): the 8×6×6 skull parenting the two 2×2×1 ears and the 4×2×3 snout.
    let head = &FOX_PARTS[0];
    assert_eq!(head.pose.offset, [-1.0, 16.5, -3.0]);
    assert_eq!(head.cubes[0].min, [-3.0, -2.0, -5.0]);
    assert_eq!(head.cubes[0].size, [8.0, 6.0, 6.0]);
    assert_eq!(head.children.len(), 3);
    // The ears and snout all sit at the head origin (PartPose.ZERO).
    assert_eq!(head.children[0].pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(head.children[0].cubes[0].min, [-3.0, -4.0, -4.0]);
    assert_eq!(head.children[1].cubes[0].min, [3.0, -4.0, -4.0]);
    assert_eq!(head.children[2].cubes[0].min, [-1.0, 2.01, -8.0]);
    assert_eq!(head.children[2].cubes[0].size, [4.0, 2.0, 3.0]);

    // `body` (offset (0, 16, -6), pitched π/2): the 6×11×6 trunk parenting the tail.
    let body = &FOX_PARTS[1];
    assert_eq!(body.pose.offset, [0.0, 16.0, -6.0]);
    assert_eq!(body.pose.rotation, [std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
    assert_eq!(body.cubes[0].min, [-3.0, 3.999, -3.5]);
    assert_eq!(body.cubes[0].size, [6.0, 11.0, 6.0]);
    assert_eq!(body.children.len(), 1);
    // `tail` (offset (-4, 15, -1), pitched -0.05235988): the 4×9×5 brush.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [-4.0, 15.0, -1.0]);
    assert_eq!(tail.pose.rotation, [-0.05235988, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].min, [2.0, 0.0, -1.0]);
    assert_eq!(tail.cubes[0].size, [4.0, 9.0, 5.0]);

    // The four legs share one fudge-inflated box (built off-center at +2 X); hind at z=7, front at z=0,
    // the right pair at pivot x=-5, the left pair at pivot x=-1.
    assert_eq!(FOX_PARTS[2].pose.offset, [-5.0, 17.5, 7.0]);
    assert_eq!(FOX_PARTS[3].pose.offset, [-1.0, 17.5, 7.0]);
    assert_eq!(FOX_PARTS[4].pose.offset, [-5.0, 17.5, 0.0]);
    assert_eq!(FOX_PARTS[5].pose.offset, [-1.0, 17.5, 0.0]);
    for leg in &FOX_PARTS[2..6] {
        assert_eq!(leg.cubes[0].min, [1.999, 0.499, -1.001]);
        assert_eq!(leg.cubes[0].size, [2.002, 6.002, 2.002]);
    }

    // Ten cubes (head, two ears, snout, body, tail, four legs).
    assert_eq!(count_cubes(&FOX_PARTS), 10);
}

#[test]
fn fox_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices, one orange tint.
    let fox = entity_model_mesh(&[EntityModelInstance::fox(400, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(fox.opaque_faces, 60);
    assert_eq!(fox.vertices.len(), 240);
    assert_eq!(fox.indices.len(), 360);
    assert!(fox
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FOX_ORANGE, 1.0)));
}

#[test]
fn fox_mesh_matches_on_both_render_paths() {
    // The fox is a colored-only entity, so the texture-skipping colored runtime path emits the exact
    // same mesh as the full path (unlike the wolf proxy it replaced).
    let instances = [EntityModelInstance::fox(401, [0.0, 64.0, 0.0], 0.0, false)];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn fox_head_look_turns_only_the_head() {
    // Vanilla `FoxModel.setupAnim` sets `head.xRot/yRot` from the look while standing. The head is the
    // first root part (skull + ears + snout, four cubes → vertices `[0, 96)`); the body, tail, and legs
    // `[96, 240)` hold.
    let rest = EntityModelInstance::fox(402, [0.0, 64.0, 0.0], 0.0, false);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head (skull, ears, snout) turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body, tail, and legs stay put"
    );

    // Both yaw and pitch move the head.
    let yaw_only = entity_model_mesh(&[rest.with_head_look(35.0, 0.0)]);
    let pitch_only = entity_model_mesh(&[rest.with_head_look(0.0, -25.0)]);
    assert_ne!(rest_mesh.vertices[..96], yaw_only.vertices[..96]);
    assert_ne!(rest_mesh.vertices[..96], pitch_only.vertices[..96]);
}

#[test]
fn baby_fox_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyFoxModel.createBodyLayer` (atlas 32×32): six root parts — head (ears + snout baked
    // in as cubes), four legs, then body (with tail). Flatter than the adult and the body has no pitch.
    assert_eq!(BABY_FOX_PARTS.len(), 6);

    let head = &BABY_FOX_PARTS[0];
    assert_eq!(head.pose.offset, [0.0, 18.125, 0.125]);
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].min, [-3.0, -2.125, -5.125]);
    assert_eq!(head.cubes[0].size, [6.0, 5.0, 5.0]);
    assert!(head.children.is_empty());

    // Legs 1..=4 (right-hind / left-hind / right-front / left-front), the 2×2×2 box.
    assert_eq!(BABY_FOX_PARTS[1].pose.offset, [-1.5, 22.0, 4.0]);
    assert_eq!(BABY_FOX_PARTS[4].pose.offset, [1.5, 22.0, 0.0]);
    assert_eq!(BABY_FOX_PARTS[1].cubes[0].size, [2.0, 2.0, 2.0]);

    // `body` (5, no pitch) parenting the tail.
    let body = &BABY_FOX_PARTS[5];
    assert_eq!(body.pose.offset, [0.0, 20.0, 2.0]);
    assert_eq!(body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(body.cubes[0].size, [5.0, 4.0, 6.0]);
    assert_eq!(body.children.len(), 1);
    assert_eq!(body.children[0].pose.offset, [0.0, -0.5, 3.0]);
    assert_eq!(body.children[0].cubes[0].size, [3.0, 3.0, 6.0]);

    // Ten cubes (head 4, four legs, body, tail).
    assert_eq!(count_cubes(&BABY_FOX_PARTS), 10);
}

#[test]
fn baby_fox_mesh_is_more_compact_than_the_adult() {
    // The baby uses a smaller body layer, so its mesh is geometrically more compact than the adult's
    // (both 10 cubes → 240 vertices). Head is part 0 in both layouts, so the head look isolates it.
    let adult = entity_model_mesh(&[EntityModelInstance::fox(410, [0.0, 64.0, 0.0], 0.0, false)]);
    let baby = entity_model_mesh(&[EntityModelInstance::fox(411, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(baby.vertices.len(), 240);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FOX_ORANGE, 1.0)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_span = adult_max[2] - adult_min[2];
    let baby_span = baby_max[2] - baby_min[2];
    assert!(
        baby_span < adult_span,
        "baby z-span {baby_span} should be smaller than adult {adult_span}"
    );

    // The baby head (part 0, vertices [0, 96)) turns with the look; the rest holds.
    let baby_rest = EntityModelInstance::fox(412, [0.0, 64.0, 0.0], 0.0, true);
    let baby_rest_mesh = entity_model_mesh(&[baby_rest]);
    let baby_looked_mesh = entity_model_mesh(&[baby_rest.with_head_look(35.0, -25.0)]);
    assert_ne!(
        baby_rest_mesh.vertices[..96],
        baby_looked_mesh.vertices[..96]
    );
    assert_eq!(
        baby_rest_mesh.vertices[96..],
        baby_looked_mesh.vertices[96..]
    );
}

#[test]
fn fox_exposes_stable_model_keys() {
    assert_eq!(EntityModelKind::Fox { baby: false }.model_key(), "fox");
    assert_eq!(EntityModelKind::Fox { baby: true }.model_key(), "fox_baby");
}
