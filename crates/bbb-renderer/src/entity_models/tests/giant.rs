use super::*;

#[test]
fn giant_mesh_is_the_zombie_humanoid_scaled_six_times() {
    // Vanilla `GiantZombieModel` is the standard humanoid (zombie) body layer baked through
    // `humanoidBodyLayer.apply(MeshTransformer.scaling(6.0))`, so the giant renders the exact zombie
    // geometry at 6× the size. Build both at the same rest pose and confirm same topology, 6× span.
    let giant = entity_model_mesh(&[EntityModelInstance::giant(590, [0.0, 64.0, 0.0], 0.0)]);
    let zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        591,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    // Same humanoid geometry → identical vertex / face / index counts.
    assert_eq!(giant.vertices.len(), zombie.vertices.len());
    assert_eq!(giant.opaque_faces, zombie.opaque_faces);
    assert_eq!(giant.indices.len(), zombie.indices.len());
    assert!(giant.vertices.len() > 0);

    // The giant span is 6× the zombie span on every axis (the `MeshTransformer.scaling(6.0)`).
    let (giant_min, giant_max) = mesh_extents(&giant);
    let (zombie_min, zombie_max) = mesh_extents(&zombie);
    for axis in 0..3 {
        let giant_span = giant_max[axis] - giant_min[axis];
        let zombie_span = zombie_max[axis] - zombie_min[axis];
        assert!(
            (giant_span - zombie_span * 6.0).abs() < 1.0e-3 * giant_span.max(1.0),
            "axis {axis}: giant span {giant_span} should be 6× the zombie span {zombie_span}"
        );
    }
}

#[test]
fn giant_shares_the_zombie_body_tints() {
    // The giant reuses the zombie body parts verbatim, so every colour present in the zombie mesh is
    // present in the giant mesh.
    let giant = entity_model_mesh(&[EntityModelInstance::giant(592, [0.0, 64.0, 0.0], 0.0)]);
    let zombie = entity_model_mesh(&[EntityModelInstance::zombie(
        593,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    for vertex in &zombie.vertices {
        assert!(
            giant
                .vertices
                .iter()
                .any(|other| other.color == vertex.color),
            "giant mesh is missing a zombie body colour"
        );
    }
}
