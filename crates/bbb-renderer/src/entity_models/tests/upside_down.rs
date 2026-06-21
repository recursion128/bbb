use super::*;

#[test]
fn upside_down_is_inert_until_named() {
    // No upside-down height (None) leaves the model upright: the mesh is
    // byte-identical to the resting render, so the Dinnerbone/Grumm branch never
    // fires for a normally-named entity.
    let base = EntityModelInstance::cow(920, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let normal = entity_model_mesh(&[base.with_upside_down_height(None)]);

    assert_eq!(resting.vertices, normal.vertices);
}

#[test]
fn upside_down_flips_the_model_vertically() {
    // Vanilla LivingEntityRenderer.setupRotations upside-down branch:
    //   translate(0, (bbHeight + 0.1) / entityScale, 0) then Axis.ZP.rotationDegrees(180).
    // The Rz(180) mirrors the model about the post-yaw origin (X and Y negated), so
    // the upright top maps to the bottom while the rigid rotation preserves every
    // axis span; the lift only shifts the whole model.
    let base = EntityModelInstance::cow(921, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let flipped = entity_model_mesh(&[base.with_upside_down_height(Some(1.4))]);

    assert_eq!(resting.vertices.len(), flipped.vertices.len());
    assert_ne!(resting.vertices, flipped.vertices);

    // A 180-degree Z rotation plus a translate is rigid: the X/Y spans are unchanged.
    let (resting_min, resting_max) = mesh_extents(&resting);
    let (flipped_min, flipped_max) = mesh_extents(&flipped);
    let span = |min: [f32; 3], max: [f32; 3], axis: usize| max[axis] - min[axis];
    assert!(
        (span(flipped_min, flipped_max, 1) - span(resting_min, resting_max, 1)).abs() < 1e-3,
        "the upside-down flip preserves the Y span (it is a rigid rotation, not a squash)"
    );
    assert!(
        (span(flipped_min, flipped_max, 0) - span(resting_min, resting_max, 0)).abs() < 1e-3,
        "the upside-down flip preserves the X span"
    );

    // The vertical order inverts: the vertex that was highest at rest sits below the
    // vertex that was lowest at rest once flipped (the uniform lift shifts both
    // equally, so their relative order proves the mirror).
    let top_idx = (0..resting.vertices.len())
        .max_by(|&a, &b| {
            resting.vertices[a].position[1].total_cmp(&resting.vertices[b].position[1])
        })
        .unwrap();
    let bottom_idx = (0..resting.vertices.len())
        .min_by(|&a, &b| {
            resting.vertices[a].position[1].total_cmp(&resting.vertices[b].position[1])
        })
        .unwrap();
    assert!(
        flipped.vertices[top_idx].position[1] < flipped.vertices[bottom_idx].position[1],
        "the resting top vertex hangs below the resting bottom vertex when flipped"
    );
}

#[test]
fn death_and_auto_spin_take_precedence_over_upside_down() {
    // Vanilla setupRotations else-if chain: death > auto-spin > sleeping > upside
    // down, so a dying or spinning Dinnerbone ignores the upside-down branch.
    let base = EntityModelInstance::cow(922, [0.0, 64.0, 0.0], 0.0, false);
    let upside_only = entity_model_mesh(&[base.with_upside_down_height(Some(1.4))]);

    let death_only = entity_model_mesh(&[base.with_death_time(20.0)]);
    let death_and_upside = entity_model_mesh(&[base
        .with_death_time(20.0)
        .with_upside_down_height(Some(1.4))]);
    assert_eq!(death_only.vertices, death_and_upside.vertices);

    let spin_only = entity_model_mesh(&[base.with_auto_spin_age_ticks(Some(5.0))]);
    let spin_and_upside = entity_model_mesh(&[base
        .with_auto_spin_age_ticks(Some(5.0))
        .with_upside_down_height(Some(1.4))]);
    assert_eq!(spin_only.vertices, spin_and_upside.vertices);

    // The upside-down pose is genuinely distinct from both higher-precedence poses.
    assert_ne!(upside_only.vertices, death_only.vertices);
    assert_ne!(upside_only.vertices, spin_only.vertices);
}
