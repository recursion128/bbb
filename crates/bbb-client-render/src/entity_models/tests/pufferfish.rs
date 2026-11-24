use super::*;

#[test]
fn pufferfish_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&PUFFERFISH_TEXTURE_REF));
}

#[test]
fn pufferfish_fin_wiggle_matches_vanilla_setup_anim() {
    // Vanilla: right fin zRot = -0.2 + 0.4 * sin(ageInTicks * 0.2); left fin is the negation.
    for age in [0.0f32, 7.5, 31.4, 100.0] {
        let want = -0.2 + 0.4 * (age * 0.2).sin();
        assert!(
            (pufferfish_right_fin_z_rot(age) - want).abs() < 1e-7,
            "age {age}"
        );
    }
    // The fin pose sets zRot absolutely over the zeroed rest pose.
    let base = layer_pose(ModelLayer::PUFFERFISH_SMALL, "right_fin");
    let posed = pufferfish_fin_pose(base, 0.37);
    assert_eq!(posed.offset, base.offset);
    assert_eq!(posed.rotation, [0.0, 0.0, 0.37]);
}

#[test]
fn pufferfish_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&pufferfish_texture_images()).unwrap();
    let instance = EntityModelInstance::pufferfish(107, [0.0, 64.0, 0.0], 0.0, 2).with(|s| {
        s.light_coords = (5_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 78);
    assert_eq!(mesh.vertices.len(), 312);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(mesh);
    assert_close3(min, [-0.37250832, 64.1435, -0.29419422]);
    assert_close3(max, [0.37250835, 64.7685, 0.29419422]);
}

fn pufferfish_texture_images() -> Vec<EntityModelTextureImage> {
    [PUFFERFISH_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
