use super::*;

#[test]
fn dolphin_wave_matches_vanilla_setup_anim() {
    // `DolphinModel.setupAnim` swim wave term `cos(ageInTicks · 0.3)` (1 at `t=0`).
    assert!((dolphin_wave(0.0) - 1.0).abs() < 1.0e-6);
    assert!((dolphin_wave(10.0) - (3.0_f32).cos()).abs() < 1.0e-6);
}

#[test]
fn dolphin_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&dolphin_texture_images()).unwrap();

    let base = EntityModelInstance::dolphin(980, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);

    // Eight cubes → 48 faces / 192 vertices, nothing on the translucent or eyes passes, white tint.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 48);
    assert_eq!(meshes.cutout.vertices.len(), 192);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.render_state.shader_light()
            && vertex.overlay == base.render_state.overlay_coords()));

    // Vanilla `BabyDolphinModel` bakes its own layer.
    let baby = EntityModelInstance::dolphin(981, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_eq!(baby_meshes.cutout.vertices.len(), 192);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby.render_state.overlay_coords()));
    assert_ne!(meshes.cutout.vertices, baby_meshes.cutout.vertices);

    // A still dolphin is static; a moving one waves its tail with age.
    let still_later = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 5.0)], &atlas);
    assert_eq!(meshes.cutout.vertices, still_later.cutout.vertices);
    let moving = base.with(|s| s.kind.dolphin_mut().is_moving = true);
    let moving_early = entity_model_textured_meshes(&[moving], &atlas);
    let moving_later =
        entity_model_textured_meshes(&[moving.with(|s| s.age_in_ticks = 5.0)], &atlas);
    assert_ne!(moving_early.cutout.vertices, moving_later.cutout.vertices);
}

fn dolphin_texture_images() -> Vec<EntityModelTextureImage> {
    [DOLPHIN_TEXTURE_REF, DOLPHIN_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
