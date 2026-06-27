use super::*;

use crate::entity_models::colored::GIANT_SCALE;

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

#[test]
fn giant_textured_render_reuses_the_zombie_texture_scaled_six_times() {
    // Vanilla `GiantMobRenderer` binds the plain zombie texture on the scaled humanoid, so the giant's
    // textured path reuses the zombie pass — same geometry, 6× the size.
    assert_eq!(
        EntityModelKind::Giant.vanilla_texture_ref(),
        Some(ZOMBIE_TEXTURE_REF)
    );
    let base_pass = zombie_textured_layer_passes(false)[0];
    assert_eq!(base_pass.model_layer, MODEL_LAYER_ZOMBIE);
    assert_eq!(base_pass.texture, ZOMBIE_TEXTURE_REF);

    let images: Vec<EntityModelTextureImage> = zombie_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let giant_instance = EntityModelInstance::giant(594, [0.0, 64.0, 0.0], 0.0);
    let zombie_instance = EntityModelInstance::zombie(595, [0.0, 64.0, 0.0], 0.0, false);
    let giant_meshes = entity_model_textured_meshes(&[giant_instance], &atlas);
    let zombie_meshes = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert!(giant_meshes.translucent.vertices.is_empty());
    assert!(giant_meshes.eyes.vertices.is_empty());
    assert_eq!(giant_meshes.submissions.len(), 1);
    let giant_submit = giant_meshes.submissions[0];
    assert_eq!(giant_submit.texture, ZOMBIE_TEXTURE_REF);
    assert_eq!(
        giant_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(giant_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(giant_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        giant_submit.transform,
        mesh_transformer_scaled_model_root_transform(giant_instance, GIANT_SCALE)
    );
    assert_eq!((giant_submit.order, giant_submit.submit_sequence), (0, 0));
    let giant = &giant_meshes.cutout;
    let zombie = &zombie_meshes.cutout;

    assert!(!giant.vertices.is_empty());
    assert_eq!(giant.vertices.len(), zombie.vertices.len());
    assert!(giant
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The giant span is 6× the zombie span on the textured path too.
    let (giant_min, giant_max) = textured_mesh_extents(giant);
    let (zombie_min, zombie_max) = textured_mesh_extents(zombie);
    for axis in 0..3 {
        let giant_span = giant_max[axis] - giant_min[axis];
        let zombie_span = zombie_max[axis] - zombie_min[axis];
        assert!(
            (giant_span - zombie_span * 6.0).abs() < 1.0e-3 * giant_span.max(1.0),
            "axis {axis}: giant textured span {giant_span} should be 6× the zombie span {zombie_span}"
        );
    }
}
