use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn end_crystal_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EndCrystalModel.createBodyLayer` (atlas 64×32): the base slab at ZERO plus the
    // concentric glass stack at offset (0, 24, 0) — outer (unscaled), inner (`withScale(0.875)`),
    // and the core (cumulative `0.875 · 0.765625`).
    assert_eq!(END_CRYSTAL_PARTS.len(), 4);

    // `base` (12×4×12) at the model origin.
    let base = &END_CRYSTAL_PARTS[0];
    assert_eq!(base.pose.offset, [0.0, 0.0, 0.0]);
    assert_eq!(base.cubes[0].min, [-6.0, 0.0, -6.0]);
    assert_eq!(base.cubes[0].size, [12.0, 4.0, 12.0]);

    // `outer_glass`: the unscaled 8×8×8 cube at (0, 24, 0).
    let outer = &END_CRYSTAL_PARTS[1];
    assert_eq!(outer.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(outer.cubes[0].size, [8.0, 8.0, 8.0]);

    // `inner_glass`: the 8×8×8 cube baked at `withScale(0.875)` → a centred 7×7×7 box.
    let inner = &END_CRYSTAL_PARTS[2];
    assert_eq!(inner.pose.offset, [0.0, 24.0, 0.0]);
    let inner_scale = 0.875_f32;
    assert!((inner.cubes[0].size[0] - 8.0 * inner_scale).abs() < 1.0e-6);
    assert_eq!(inner.cubes[0].size, [7.0, 7.0, 7.0]);
    assert_eq!(inner.cubes[0].min, [-3.5, -3.5, -3.5]);

    // `cube`: the core 8×8×8 cube baked at the cumulative `0.875 · 0.765625` scale.
    let core = &END_CRYSTAL_PARTS[3];
    assert_eq!(core.pose.offset, [0.0, 24.0, 0.0]);
    let core_scale = 0.875_f32 * 0.765625_f32;
    assert!((core.cubes[0].size[0] - 8.0 * core_scale).abs() < 1.0e-6);
    assert!((core.cubes[0].min[0] - (-4.0 * core_scale)).abs() < 1.0e-6);
    assert_eq!(core.cubes[0].size, [5.359375, 5.359375, 5.359375]);

    // The three glass boxes share the same centre and shrink monotonically.
    assert!(outer.cubes[0].size[0] > inner.cubes[0].size[0]);
    assert!(inner.cubes[0].size[0] > core.cubes[0].size[0]);

    // Four cubes total.
    assert_eq!(count_cubes(&END_CRYSTAL_PARTS), 4);
}

#[test]
fn end_crystal_mesh_uses_vanilla_body_layer_geometry() {
    // 4 cubes → 24 faces / 96 vertices / 144 indices; the glass, core, and base carry their tints.
    let crystal =
        entity_model_mesh(&[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(crystal.opaque_faces, 24);
    assert_eq!(crystal.vertices.len(), 96);
    assert_eq!(crystal.indices.len(), 144);
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_GLASS, 1.0)));
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_CORE, 1.0)));
    assert!(crystal
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(END_CRYSTAL_BASE, 1.0)));
}
