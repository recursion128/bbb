use super::*;

#[test]
fn scale_is_inert_at_one() {
    // The default scale (1.0) leaves the model untouched: the mesh is byte-identical
    // to the resting render, so a normally-sized entity is unaffected.
    let base = EntityModelInstance::cow(940, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let unit = entity_model_mesh(&[base.with_scale(1.0)]);

    assert_eq!(resting.vertices, unit.vertices);
}

#[test]
fn scale_resizes_the_model_uniformly_about_the_entity_position() {
    // Vanilla LivingEntityRenderer.submit applies state.scale as a uniform
    // poseStack.scale before setupRotations, so the whole model grows about the
    // entity position: every axis span scales by the factor.
    let base = EntityModelInstance::cow(941, [0.0, 64.0, 0.0], 0.0, false);
    let unit = entity_model_mesh(&[base.with_scale(1.0)]);
    let doubled = entity_model_mesh(&[base.with_scale(2.0)]);

    let (unit_min, unit_max) = mesh_extents(&unit);
    let (doubled_min, doubled_max) = mesh_extents(&doubled);
    for axis in 0..3 {
        let unit_span = unit_max[axis] - unit_min[axis];
        let doubled_span = doubled_max[axis] - doubled_min[axis];
        assert!(
            (doubled_span - 2.0 * unit_span).abs() < 1e-3,
            "axis {axis}: doubling the scale doubles the span"
        );
    }
}

#[test]
fn scale_applies_to_the_upside_down_branch() {
    // The uniform scale is applied before setupRotations, so it enlarges the model
    // in the upside-down branch too. (The vanilla `(bbHeight + 0.1) / entityScale`
    // divisor keeps the world-space lift at bbHeight + 0.1 across scales, so for the
    // default scale of 1.0 the existing upside-down tests render byte-identically.)
    let base = EntityModelInstance::cow(942, [0.0, 64.0, 0.0], 0.0, false)
        .with_upside_down_height(Some(1.4));
    let unit = entity_model_mesh(&[base.with_scale(1.0)]);
    let doubled = entity_model_mesh(&[base.with_scale(2.0)]);

    let (unit_min, unit_max) = mesh_extents(&unit);
    let (doubled_min, doubled_max) = mesh_extents(&doubled);
    // The horizontal footprint of the flipped model doubles with the scale.
    assert!(
        ((doubled_max[0] - doubled_min[0]) - 2.0 * (unit_max[0] - unit_min[0])).abs() < 1e-3,
        "the scale enlarges the upside-down model uniformly"
    );
}
