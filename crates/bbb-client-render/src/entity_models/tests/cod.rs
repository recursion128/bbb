use super::*;

#[test]
fn cod_tail_fin_sway_matches_vanilla_setup_anim() {
    // `tailFin.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, amplitude 1.0 in water
    // / 1.5 out. At age 0 the sway is zero regardless of amplitude.
    assert_eq!(cod_tail_fin_yrot(0.0, true), 0.0);
    assert_eq!(cod_tail_fin_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.6 * age).sin();
    assert!((cod_tail_fin_yrot(age, true) - (-0.45 * s)).abs() < 1.0e-6);
    assert!((cod_tail_fin_yrot(age, false) - (-1.5 * 0.45 * s)).abs() < 1.0e-6);
    // The beached cod thrashes harder (1.5x amplitude).
    assert!(cod_tail_fin_yrot(age, false).abs() > cod_tail_fin_yrot(age, true).abs());
}

#[test]
fn cod_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&cod_texture_images()).unwrap();
    // Vanilla `CodModel` calls `EntityModel(root)`, so the base submit uses the default
    // `entityCutout` render type. The backend folds it into the cutout mesh, but the submission
    // keeps the vanilla texture, render type, tint, transform, light, overlay, and default
    // collector order.
    let base = EntityModelInstance::cod(910, [0.0, 64.0, 0.0], 0.0).with(|s| s.in_water = true);
    let still_meshes = entity_model_textured_meshes(&[base], &atlas);

    // Seven cubes → 168 textured vertices on the cutout pass.
    assert!(still_meshes.translucent.vertices.is_empty());
    assert!(still_meshes.eyes.vertices.is_empty());
    assert_eq!(still_meshes.cutout.vertices.len(), 168);

    // The tail sway / body wiggle reorient the mesh as the age advances.
    let swimming = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 7.0)], &atlas);

    assert_eq!(
        still_meshes.cutout.vertices.len(),
        swimming.cutout.vertices.len()
    );
    assert_ne!(still_meshes.cutout.vertices, swimming.cutout.vertices);

    // A beached cod flops onto its side.
    let beached_instance = base.with(|s| s.in_water = false);
    let beached = entity_model_textured_meshes(&[beached_instance], &atlas);

    assert_ne!(still_meshes.cutout.vertices, beached.cutout.vertices);
}

#[test]
fn cod_submission_preserves_light_and_overlay_metadata() {
    let (atlas, _) = build_entity_model_texture_atlas(&cod_texture_images()).unwrap();
    // Vanilla `LivingEntityRenderer.submit` forwards `state.lightCoords` and
    // `getOverlayCoords(state, whiteOverlayProgress)` into `submitModel`; keep that metadata at the
    // submission boundary instead of only filling the folded mesh vertices.
    let light_coords = (7_u32 << 4) | (12_u32 << 20);
    let instance = EntityModelInstance::cod(911, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.in_water = true;
        s.light_coords = light_coords;
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == [7.0_f32 / 15.0, 12.0_f32 / 15.0]
            && vertex.overlay == [12.0, 3.0]));
}

fn cod_texture_images() -> Vec<EntityModelTextureImage> {
    [COD_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
