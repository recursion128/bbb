use super::*;
use crate::entity_models::model::EntityModel;

#[test]
fn feline_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultFelineModel.createBodyMesh(NONE)` (atlas 64×32): eight flat named root parts —
    // head, body, two tail segments, and four legs.

    // `head` (offset (0, 15, -9)): the 5×4×5 skull, the 3×2×2 nose, and the two 1×1×2 ears.
    assert_eq!(FELINE_HEAD_POSE.offset, [0.0, 15.0, -9.0]);
    assert_eq!(FELINE_HEAD_CUBES.len(), 4);
    assert_eq!(FELINE_HEAD_CUBES[0].min, [-2.5, -2.0, -3.0]);
    assert_eq!(FELINE_HEAD_CUBES[0].size, [5.0, 4.0, 5.0]);
    assert_eq!(FELINE_HEAD_CUBES[1].min, [-1.5, -0.001, -4.0]);
    assert_eq!(FELINE_HEAD_CUBES[1].size, [3.0, 2.0, 2.0]);
    assert_eq!(FELINE_HEAD_CUBES[2].min, [-2.0, -3.0, 0.0]);
    assert_eq!(FELINE_HEAD_CUBES[3].min, [1.0, -3.0, 0.0]);

    // `body` (offset (0, 12, -10), pitched π/2): the 4×16×6 trunk.
    assert_eq!(FELINE_BODY_POSE.offset, [0.0, 12.0, -10.0]);
    assert_eq!(
        FELINE_BODY_POSE.rotation,
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0]
    );
    assert_eq!(FELINE_BODY_CUBES[0].min, [-2.0, 3.0, -8.0]);
    assert_eq!(FELINE_BODY_CUBES[0].size, [4.0, 16.0, 6.0]);

    // `tail1` (offset (0, 15, 8), pitched 0.9): the upper 1×8×1 segment.
    assert_eq!(FELINE_TAIL1_POSE.offset, [0.0, 15.0, 8.0]);
    assert_eq!(FELINE_TAIL1_POSE.rotation, [0.9, 0.0, 0.0]);
    assert_eq!(FELINE_TAIL1_CUBES[0].size, [1.0, 8.0, 1.0]);

    // `tail2` (offset (0, 20, 14)): the lower segment, deflated by the vanilla `CubeDeformation(-0.02)`.
    assert_eq!(FELINE_TAIL2_POSE.offset, [0.0, 20.0, 14.0]);
    assert_eq!(FELINE_TAIL2_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(FELINE_TAIL2_CUBES[0].min, [-0.48, 0.02, 0.02]);
    assert_eq!(FELINE_TAIL2_CUBES[0].size, [0.96, 7.96, 0.96]);

    // The four legs: hind (2×6×2) at z=5, front (2×10×2) at z=-5, mirrored on X.
    assert_eq!(FELINE_LEFT_HIND_LEG_POSE.offset, [1.1, 18.0, 5.0]);
    assert_eq!(FELINE_RIGHT_HIND_LEG_POSE.offset, [-1.1, 18.0, 5.0]);
    assert_eq!(FELINE_HIND_LEG_CUBES[0].size, [2.0, 6.0, 2.0]);
    assert_eq!(FELINE_LEFT_FRONT_LEG_POSE.offset, [1.2, 14.1, -5.0]);
    assert_eq!(FELINE_RIGHT_FRONT_LEG_POSE.offset, [-1.2, 14.1, -5.0]);
    assert_eq!(FELINE_FRONT_LEG_CUBES[0].size, [2.0, 10.0, 2.0]);
}

#[test]
fn baby_feline_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyFelineModel.createBodyMesh` (atlas 32×32): eight flat named root parts in a
    // flatter, all-upright layout — head, three legs, body, the fourth leg, then the two tail segments.

    // `head` (offset (0, 20, -3.125)): the 5×4×4 skull, two 1×1×2 ears, and a 3×2×1 nose.
    assert_eq!(BABY_FELINE_HEAD_POSE.offset, [0.0, 20.0, -3.125]);
    assert_eq!(BABY_FELINE_HEAD_CUBES.len(), 4);
    assert_eq!(BABY_FELINE_HEAD_CUBES[0].min, [-2.5, -3.0, -2.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[0].size, [5.0, 4.0, 4.0]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[1].min, [-2.0, -4.0, -0.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[2].min, [1.0, -4.0, -0.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[3].min, [-1.5, -1.0, -3.875]);
    assert_eq!(BABY_FELINE_HEAD_CUBES[3].size, [3.0, 2.0, 1.0]);

    // The four 1×2×2 legs (shared box): left/right front at z=-1.5, left/right hind at z=2.5.
    assert_eq!(BABY_FELINE_LEFT_FRONT_LEG_POSE.offset, [1.0, 22.0, -1.5]);
    assert_eq!(BABY_FELINE_RIGHT_FRONT_LEG_POSE.offset, [-1.0, 22.0, -1.5]);
    assert_eq!(BABY_FELINE_LEFT_HIND_LEG_POSE.offset, [1.0, 22.0, 2.5]);
    assert_eq!(BABY_FELINE_RIGHT_HIND_LEG_POSE.offset, [-1.0, 22.0, 2.5]);
    assert_eq!(BABY_FELINE_LEG_CUBES[0].min, [-0.5, 0.0, -1.0]);
    assert_eq!(BABY_FELINE_LEG_CUBES[0].size, [1.0, 2.0, 2.0]);

    // `body` (offset (0, 20.5, 0.5)): the 4×3×7 trunk, upright (no pitch).
    assert_eq!(BABY_FELINE_BODY_POSE.offset, [0.0, 20.5, 0.5]);
    assert_eq!(BABY_FELINE_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(BABY_FELINE_BODY_CUBES[0].min, [-2.0, -1.5, -3.5]);
    assert_eq!(BABY_FELINE_BODY_CUBES[0].size, [4.0, 3.0, 7.0]);

    // `tail1` (offset (0, 19.107, 3.9151), pitched -0.567232): the single 1×1×5 segment.
    assert_eq!(BABY_FELINE_TAIL1_POSE.offset, [0.0, 19.107, 3.9151]);
    assert_eq!(BABY_FELINE_TAIL1_POSE.rotation, [-0.567232, 0.0, 0.0]);
    assert_eq!(BABY_FELINE_TAIL1_CUBES[0].size, [1.0, 1.0, 5.0]);

    // `tail2` (PartPose.ZERO): a cubeless pivot.
    assert_eq!(BABY_FELINE_TAIL2_POSE.offset, [0.0, 0.0, 0.0]);
}

#[test]
fn feline_mesh_uses_vanilla_body_layer_geometry() {
    // 11 cubes → 66 faces / 264 vertices / 396 indices, one tan tint.
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        500,
        [0.0, 64.0, 0.0],
        0.0,
        false,
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
fn baby_feline_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices, one tan tint.
    let baby = entity_model_mesh(&[EntityModelInstance::feline(
        506,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FELINE_TAN, 1.0)));
}

#[test]
fn feline_mesh_matches_on_both_render_paths() {
    // The feline is a colored-only entity, so the texture-skipping colored runtime path emits the
    // exact same mesh as the full path (unlike the wolf proxy it replaced).
    let instances = [
        EntityModelInstance::feline(501, [0.0, 64.0, 0.0], 0.0, false, false),
        EntityModelInstance::feline(507, [4.0, 64.0, 0.0], 0.0, true, true),
    ];
    let full = entity_model_mesh(&instances);
    let colored = entity_model_colored_runtime_mesh(&instances);
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);
}

#[test]
fn baby_cat_and_baby_ocelot_share_the_same_mesh() {
    // Vanilla `CAT_BABY` and `OCELOT_BABY` both use the unscaled `felineBabyBodyLayer`, so the two
    // babies render the identical mesh regardless of the `cat` flag.
    let cat = entity_model_mesh(&[EntityModelInstance::feline(
        508,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
    )]);
    let ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        508,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    assert_eq!(cat.vertices, ocelot.vertices);
}

#[test]
fn feline_head_look_turns_only_the_head() {
    // Vanilla `AdultFelineModel.setupAnim` sets `head.xRot/yRot` from the look angles. The head is the
    // first root part (four cubes → vertices `[0, 96)`); the body, tail, and legs `[96, 264)` hold (the
    // standing tail droop is applied identically at both, so it does not differ).
    let rest = EntityModelInstance::feline(502, [0.0, 64.0, 0.0], 0.0, false, false);
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
fn baby_feline_head_look_turns_only_the_head() {
    // The baby head is also the first root part (four cubes → vertices `[0, 96)`). Everything below it
    // holds — the baby's `tail2` droop is a no-op (it is cubeless), so the rest of the mesh is rigid.
    let rest = EntityModelInstance::feline(509, [0.0, 64.0, 0.0], 0.0, false, true);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the legs, body, and tail stay put"
    );
}

#[test]
fn feline_standing_drops_the_lower_tail() {
    // Vanilla `AdultFelineModel.setupAnim` sets `tail2.xRot = 1.7278761` while not sitting (the base the
    // deferred walk wobble adds onto), a real change from the `0` bind rotation; the bind-0.9 `tail1`
    // is left alone at rest.
    let mut model = FelineModel::new(false);
    model.prepare(&EntityModelInstance::feline(
        503,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    ));
    assert!((model.root_mut().child_mut("tail2").pose.rotation[0] - 1.7278761).abs() < 1.0e-6);
    assert_eq!(model.root_mut().child_mut("tail1").pose.rotation[0], 0.9);
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
        false,
    )]);
    let cat = entity_model_mesh(&[EntityModelInstance::feline(
        505,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
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
fn baby_feline_is_not_scaled_like_the_adult_cat() {
    // Unlike the adult cat, the baby cat does not get the 0.8 `CAT_TRANSFORMER`: the baby cat and baby
    // ocelot share the unscaled `felineBabyBodyLayer`, so their meshes are identical despite the flag.
    let baby_cat = entity_model_mesh(&[EntityModelInstance::feline(
        510,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
    )]);
    let baby_ocelot = entity_model_mesh(&[EntityModelInstance::feline(
        510,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    let (cat_min, cat_max) = mesh_extents(&baby_cat);
    let (ocelot_min, ocelot_max) = mesh_extents(&baby_ocelot);
    assert_eq!(cat_min, ocelot_min);
    assert_eq!(cat_max, ocelot_max);
}

#[test]
fn feline_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Feline {
            cat: true,
            baby: false
        }
        .model_key(),
        "feline_cat"
    );
    assert_eq!(
        EntityModelKind::Feline {
            cat: false,
            baby: false
        }
        .model_key(),
        "feline_ocelot"
    );
    assert_eq!(
        EntityModelKind::Feline {
            cat: true,
            baby: true
        }
        .model_key(),
        "feline_cat_baby"
    );
    assert_eq!(
        EntityModelKind::Feline {
            cat: false,
            baby: true
        }
        .model_key(),
        "feline_ocelot_baby"
    );
}
