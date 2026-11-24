use super::*;

#[test]
fn tadpole_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = tadpole_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, TADPOLE_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    assert!(entity_model_texture_refs().contains(&TADPOLE_TEXTURE_REF));
}

#[test]
fn tadpole_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = [TADPOLE_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.in_water = true;
        s.light_coords = (7_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_tadpole_base_submission(&meshes, base);

    // body box = 6 faces; the textured path emits all 6 faces of the 0-width tail box (the 4
    // degenerate side quads have zero area) → 12 cutout faces.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 12);
    assert_eq!(meshes.cutout.vertices.len(), 48);

    // The body remains
    // stable while the tail sways with age, and the beached tail uses the larger amplitude.
    let swaying = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 5.0)], &atlas);
    assert_tadpole_base_submission(&swaying, base.with(|s| s.age_in_ticks = 5.0));
    assert_eq!(
        meshes.cutout.vertices[24..],
        swaying.cutout.vertices[24..],
        "the textured body stays put"
    );
    assert_ne!(
        meshes.cutout.vertices[..24],
        swaying.cutout.vertices[..24],
        "the textured tail fin sways with age"
    );

    let beached = entity_model_textured_meshes(
        &[base.with(|s| {
            s.in_water = false;
            s.age_in_ticks = 5.0;
        })],
        &atlas,
    );
    assert_tadpole_base_submission(
        &beached,
        base.with(|s| {
            s.in_water = false;
            s.age_in_ticks = 5.0;
        }),
    );
    assert_ne!(beached.cutout.vertices[..24], swaying.cutout.vertices[..24]);
}

#[test]
fn tadpole_tail_sway_matches_vanilla_setup_anim() {
    // `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, amplitude 1.0 in water / 1.5 out (a
    // beached tadpole thrashes harder). At age 0 the sway is zero regardless of amplitude.
    assert_eq!(tadpole_tail_yrot(0.0, true), 0.0);
    assert_eq!(tadpole_tail_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.3 * age).sin();
    assert!((tadpole_tail_yrot(age, true) - (-0.25 * s)).abs() < 1.0e-6);
    assert!((tadpole_tail_yrot(age, false) - (-1.5 * 0.25 * s)).abs() < 1.0e-6);
    assert!(tadpole_tail_yrot(age, false).abs() > tadpole_tail_yrot(age, true).abs());
}

fn assert_tadpole_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
