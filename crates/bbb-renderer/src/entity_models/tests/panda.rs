use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn panda_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `PandaModel.createBodyLayer` (atlas 64×64): six flat root parts in `QuadrupedModel`
    // order — head, body, and four legs.
    assert_eq!(PANDA_PARTS.len(), 6);

    // `head` (offset (0, 11.5, -17)): the 13×10×9 skull, the 7×5×2 muzzle, and the two 5×4×1 ears.
    let head = &PANDA_PARTS[0];
    assert_eq!(head.pose.offset, [0.0, 11.5, -17.0]);
    assert_eq!(head.pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].min, [-6.5, -5.0, -4.0]);
    assert_eq!(head.cubes[0].size, [13.0, 10.0, 9.0]);
    assert_eq!(head.cubes[1].min, [-3.5, 0.0, -6.0]);
    assert_eq!(head.cubes[1].size, [7.0, 5.0, 2.0]);
    assert_eq!(head.cubes[2].min, [3.5, -8.0, -1.0]);
    assert_eq!(head.cubes[3].min, [-8.5, -8.0, -1.0]);
    assert_eq!(head.cubes[2].size, [5.0, 4.0, 1.0]);
    assert!(head.children.is_empty());

    // `body` (offset (0, 10, 0), pitched π/2): the 19×26×13 trunk.
    let body = &PANDA_PARTS[1];
    assert_eq!(body.pose.offset, [0.0, 10.0, 0.0]);
    assert_eq!(body.pose.rotation, [std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
    assert_eq!(body.cubes[0].min, [-9.5, -13.0, -6.5]);
    assert_eq!(body.cubes[0].size, [19.0, 26.0, 13.0]);

    // The four legs share one 6×9×6 box; right-hind/left-hind at z=9, right-front/left-front at z=-9.
    assert_eq!(PANDA_PARTS[2].pose.offset, [-5.5, 15.0, 9.0]);
    assert_eq!(PANDA_PARTS[3].pose.offset, [5.5, 15.0, 9.0]);
    assert_eq!(PANDA_PARTS[4].pose.offset, [-5.5, 15.0, -9.0]);
    assert_eq!(PANDA_PARTS[5].pose.offset, [5.5, 15.0, -9.0]);
    for leg in &PANDA_PARTS[2..6] {
        assert_eq!(leg.cubes[0].min, [-3.0, 0.0, -3.0]);
        assert_eq!(leg.cubes[0].size, [6.0, 9.0, 6.0]);
    }

    // Nine cubes (head 4, body 1, four legs 1 each).
    assert_eq!(count_cubes(&PANDA_PARTS), 9);
}

#[test]
fn panda_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices, two tones: white body/head/muzzle, black
    // ears/legs (the per-face directional shading varies the brightness).
    let panda = entity_model_mesh(&[EntityModelInstance::panda(
        600,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(panda.opaque_faces, 54);
    assert_eq!(panda.vertices.len(), 216);
    assert_eq!(panda.indices.len(), 324);
    assert!(panda
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PANDA_WHITE, 1.0)));
    assert!(panda
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PANDA_BLACK, 1.0)));
}

#[test]
fn panda_mesh_matches_on_both_render_paths() {
    // The panda is a colored-only entity, so the texture-skipping colored runtime path emits the
    // exact same mesh as the full path (unlike the cow proxy it replaced).
    let instances = [EntityModelInstance::panda(
        601,
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
fn panda_head_look_turns_only_the_head() {
    // Vanilla `QuadrupedModel.setupAnim` sets `head.xRot/yRot` from the look angles. The head is the
    // first root part (four cubes → vertices `[0, 96)`); the body and four legs `[96, 216)` hold.
    let rest = EntityModelInstance::panda(602, [0.0, 64.0, 0.0], 0.0, false);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head (skull, muzzle, and ears) turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body and legs stay put"
    );

    // Both yaw and pitch move the head.
    let yaw_only = entity_model_mesh(&[rest.with_head_look(35.0, 0.0)]);
    let pitch_only = entity_model_mesh(&[rest.with_head_look(0.0, -25.0)]);
    assert_ne!(rest_mesh.vertices[..96], yaw_only.vertices[..96]);
    assert_ne!(rest_mesh.vertices[..96], pitch_only.vertices[..96]);
}

#[test]
fn panda_walk_swings_only_the_legs() {
    // Vanilla `QuadrupedModel.setupAnim` swings the four legs off the walk cycle (a no-op at rest).
    // The legs are the last four root parts (vertices `[120, 216)`); the head and body `[0, 120)` hold.
    let still = EntityModelInstance::panda(603, [0.0, 64.0, 0.0], 0.0, false);
    let walking = still.with_walk_animation(6.0, 1.0);
    let still_mesh = entity_model_mesh(&[still]);
    let walking_mesh = entity_model_mesh(&[walking]);
    assert_eq!(still_mesh.vertices.len(), walking_mesh.vertices.len());
    assert_eq!(
        still_mesh.vertices[..120],
        walking_mesh.vertices[..120],
        "the head and body stay put while walking"
    );
    assert_ne!(
        still_mesh.vertices[120..],
        walking_mesh.vertices[120..],
        "the four legs swing off the walk cycle"
    );

    // A standing panda (walk speed 0) collapses the swing to the bind pose.
    let zero_speed =
        entity_model_mesh(&[
            EntityModelInstance::panda(604, [0.0, 64.0, 0.0], 0.0, false)
                .with_walk_animation(6.0, 0.0),
        ]);
    assert_eq!(still_mesh.vertices, zero_speed.vertices);
}

#[test]
fn baby_panda_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyPandaModel.createBodyLayer` (atlas 64×64): the `QuadrupedModel` baby convention lists
    // the body FIRST then the head, and the baby body carries no π/2 pitch.
    assert_eq!(BABY_PANDA_PARTS.len(), 6);

    // `body` (0, no pitch): the 9×7×11 trunk.
    let body = &BABY_PANDA_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 18.5, 2.5]);
    assert_eq!(body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(body.cubes[0].size, [9.0, 7.0, 11.0]);

    // `head` (1): the 7×6×5 skull, the 4×2×1 muzzle, and the two 3×3×1 ears.
    let head = &BABY_PANDA_PARTS[1];
    assert_eq!(head.pose.offset, [0.0, 19.0, -3.0]);
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].size, [7.0, 6.0, 5.0]);
    assert_eq!(head.cubes[2].min, [-4.5, -4.0, -3.5]);
    assert_eq!(head.cubes[3].min, [1.5, -4.0, -3.5]);

    // The four legs (2..=5), the 3×2×3 box.
    assert_eq!(BABY_PANDA_PARTS[2].pose.offset, [-3.0, 22.0, 6.5]);
    assert_eq!(BABY_PANDA_PARTS[5].pose.offset, [3.0, 22.0, -1.5]);
    assert_eq!(BABY_PANDA_PARTS[2].cubes[0].size, [3.0, 2.0, 3.0]);

    // Nine cubes (head 4, body 1, four legs).
    assert_eq!(count_cubes(&BABY_PANDA_PARTS), 9);
}

#[test]
fn baby_panda_head_is_part_one_and_turns_with_the_look() {
    // The baby layout lists the body first (vertices `[0, 24)`) then the head (four cubes,
    // `[24, 120)`), then the four legs. The head look turns the head; the body and legs hold.
    let rest = EntityModelInstance::panda(610, [0.0, 64.0, 0.0], 0.0, true);
    let baby = entity_model_mesh(&[rest]);
    assert_eq!(baby.vertices.len(), 216);
    let looked = entity_model_mesh(&[rest.with_head_look(35.0, -25.0)]);
    assert_eq!(baby.vertices[..24], looked.vertices[..24], "the body holds");
    assert_ne!(
        baby.vertices[24..120],
        looked.vertices[24..120],
        "the head turns"
    );
    assert_eq!(
        baby.vertices[120..],
        looked.vertices[120..],
        "the legs hold at rest"
    );

    // The baby is more compact than the adult (smaller body layer).
    let adult = entity_model_mesh(&[EntityModelInstance::panda(
        611,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!((baby_max[1] - baby_min[1]) < (adult_max[1] - adult_min[1]));
}

#[test]
fn panda_exposes_stable_model_keys() {
    assert_eq!(EntityModelKind::Panda { baby: false }.model_key(), "panda");
    assert_eq!(
        EntityModelKind::Panda { baby: true }.model_key(),
        "panda_baby"
    );
}
