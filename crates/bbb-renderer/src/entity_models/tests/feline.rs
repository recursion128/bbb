use super::*;
use crate::entity_models::model::EntityModel;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn feline_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultFelineModel.createBodyMesh(NONE)` (atlas 64×32): eight flat root parts — head,
    // body, two tail segments, and four legs.
    assert_eq!(FELINE_PARTS.len(), 8);

    // `head` (offset (0, 15, -9)): the 5×4×5 skull, the 3×2×2 nose, and the two 1×1×2 ears.
    let head = &FELINE_PARTS[0];
    assert_eq!(head.pose.offset, [0.0, 15.0, -9.0]);
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].min, [-2.5, -2.0, -3.0]);
    assert_eq!(head.cubes[0].size, [5.0, 4.0, 5.0]);
    assert_eq!(head.cubes[1].min, [-1.5, -0.001, -4.0]);
    assert_eq!(head.cubes[1].size, [3.0, 2.0, 2.0]);
    assert_eq!(head.cubes[2].min, [-2.0, -3.0, 0.0]);
    assert_eq!(head.cubes[3].min, [1.0, -3.0, 0.0]);

    // `body` (offset (0, 12, -10), pitched π/2): the 4×16×6 trunk.
    let body = &FELINE_PARTS[1];
    assert_eq!(body.pose.offset, [0.0, 12.0, -10.0]);
    assert_eq!(body.pose.rotation, [std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
    assert_eq!(body.cubes[0].min, [-2.0, 3.0, -8.0]);
    assert_eq!(body.cubes[0].size, [4.0, 16.0, 6.0]);

    // `tail1` (offset (0, 15, 8), pitched 0.9): the upper 1×8×1 segment.
    let tail1 = &FELINE_PARTS[2];
    assert_eq!(tail1.pose.offset, [0.0, 15.0, 8.0]);
    assert_eq!(tail1.pose.rotation, [0.9, 0.0, 0.0]);
    assert_eq!(tail1.cubes[0].size, [1.0, 8.0, 1.0]);

    // `tail2` (offset (0, 20, 14)): the lower segment, deflated by the vanilla `CubeDeformation(-0.02)`.
    let tail2 = &FELINE_PARTS[3];
    assert_eq!(tail2.pose.offset, [0.0, 20.0, 14.0]);
    assert_eq!(tail2.pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(tail2.cubes[0].min, [-0.48, 0.02, 0.02]);
    assert_eq!(tail2.cubes[0].size, [0.96, 7.96, 0.96]);

    // The four legs: hind (2×6×2) at z=5, front (2×10×2) at z=-5, mirrored on X.
    assert_eq!(FELINE_PARTS[4].pose.offset, [1.1, 18.0, 5.0]);
    assert_eq!(FELINE_PARTS[5].pose.offset, [-1.1, 18.0, 5.0]);
    assert_eq!(FELINE_PARTS[4].cubes[0].size, [2.0, 6.0, 2.0]);
    assert_eq!(FELINE_PARTS[6].pose.offset, [1.2, 14.1, -5.0]);
    assert_eq!(FELINE_PARTS[7].pose.offset, [-1.2, 14.1, -5.0]);
    assert_eq!(FELINE_PARTS[6].cubes[0].size, [2.0, 10.0, 2.0]);

    // Eleven cubes (head 4, body 1, two tail segments, four legs).
    assert_eq!(count_cubes(&FELINE_PARTS), 11);
}

#[test]
fn feline_mesh_uses_vanilla_body_layer_geometry() {
    // 11 cubes → 66 faces / 264 vertices / 396 indices, one tan tint.
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        500,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(ocelot.opaque_faces, 66);
    assert_eq!(ocelot.vertices.len(), 264);
    assert_eq!(ocelot.indices.len(), 396);
    assert!(ocelot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FELINE_TAN, 1.0)));
}

#[test]
fn feline_mesh_matches_on_both_render_paths() {
    // The feline is a colored-only entity, so the texture-skipping colored runtime path emits the
    // exact same mesh as the full path (unlike the wolf proxy it replaced).
    let instances = [EntityModelInstance::feline(
        501,
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
fn feline_head_look_turns_only_the_head() {
    // Vanilla `AdultFelineModel.setupAnim` sets `head.xRot/yRot` from the look angles. The head is the
    // first root part (four cubes → vertices `[0, 96)`); the body, tail, and legs `[96, 264)` hold (the
    // standing tail droop is applied identically at both, so it does not differ).
    let rest = EntityModelInstance::feline(502, [0.0, 64.0, 0.0], 0.0, false);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body, tail, and legs stay put"
    );
}

#[test]
fn feline_standing_drops_the_lower_tail() {
    // Vanilla `AdultFelineModel.setupAnim` sets `tail2.xRot = 1.7278761` while not sitting (the base the
    // deferred walk wobble adds onto), a real change from the `0` bind rotation; the bind-0.9 `tail1`
    // is left alone at rest.
    let mut model = FelineModel::new();
    model.prepare(&EntityModelInstance::feline(
        503,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    ));
    assert!((model.root_mut().child_at_mut(3).pose.rotation[0] - 1.7278761).abs() < 1.0e-6);
    assert_eq!(model.root_mut().child_at_mut(2).pose.rotation[0], 0.9);
}

#[test]
fn cat_mesh_is_the_ocelot_mesh_scaled_down() {
    // Vanilla `AdultCatModel.CAT_TRANSFORMER = MeshTransformer.scaling(0.8)`: the cat is the same mesh
    // as the ocelot, scaled 0.8 (so the same vertex count but a more compact mesh).
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        504,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let cat = entity_model_mesh(&[EntityModelInstance::feline(
        505,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(ocelot.vertices.len(), cat.vertices.len());
    let (ocelot_min, ocelot_max) = mesh_extents(&ocelot);
    let (cat_min, cat_max) = mesh_extents(&cat);
    let ocelot_span = ocelot_max[1] - ocelot_min[1];
    let cat_span = cat_max[1] - cat_min[1];
    assert!(
        cat_span < ocelot_span,
        "cat y-span {cat_span} should be smaller than ocelot {ocelot_span}"
    );
}

#[test]
fn feline_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Feline { cat: true }.model_key(),
        "feline_cat"
    );
    assert_eq!(
        EntityModelKind::Feline { cat: false }.model_key(),
        "feline_ocelot"
    );
}
