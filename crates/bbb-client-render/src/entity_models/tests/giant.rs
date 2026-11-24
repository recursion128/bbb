use super::*;

#[test]
fn giant_textured_render_reuses_the_zombie_texture_scaled_six_times() {
    // Vanilla `GiantMobRenderer` binds the plain zombie texture on the scaled humanoid, so the giant's
    // textured path reuses the zombie pass — same geometry, 6× the size.
    let base_pass = zombie_textured_layer_passes(false)[0];
    assert_eq!(base_pass.texture, ZOMBIE_TEXTURE_REF);

    let images: Vec<EntityModelTextureImage> = [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let giant_instance = EntityModelInstance::giant(594, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let zombie_instance = EntityModelInstance::zombie(595, [0.0, 64.0, 0.0], 0.0, false);
    let giant_meshes = entity_model_textured_meshes(&[giant_instance], &atlas);
    let zombie_meshes = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert!(giant_meshes.translucent.vertices.is_empty());
    assert!(giant_meshes.eyes.vertices.is_empty());

    let giant = &giant_meshes.cutout;
    let zombie = &zombie_meshes.cutout;

    assert!(!giant.vertices.is_empty());
    assert_eq!(giant.vertices.len(), zombie.vertices.len());
    assert!(giant
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(giant.vertices.iter().all(|vertex| {
        vertex.light == giant_instance.render_state.shader_light()
            && vertex.overlay == giant_instance.render_state.overlay_coords()
    }));

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
