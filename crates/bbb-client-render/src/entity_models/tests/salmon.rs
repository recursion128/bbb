use super::*;

#[test]
fn salmon_swim_multipliers_match_vanilla() {
    // `SalmonModel.setupAnim` / `SalmonRenderer.setupRotations`: a swimming salmon uses
    // `(1.0, 1.0)`; a beached salmon thrashes harder and faster `(1.3, 1.7)`.
    assert_eq!(salmon_swim_multipliers(true), (1.0, 1.0));
    assert_eq!(salmon_swim_multipliers(false), (1.3, 1.7));
}

#[test]
fn salmon_body_back_sway_matches_vanilla_setup_anim() {
    // `bodyBack.yRot = -amplitude * 0.25 * sin(angle * 0.6 * ageInTicks)`. At age 0 the
    // sway is zero regardless of amplitude.
    assert_eq!(salmon_body_back_yrot(0.0, true), 0.0);
    assert_eq!(salmon_body_back_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (1.0 * 0.6 * age).sin();
    assert!((salmon_body_back_yrot(age, true) - (-0.25 * s)).abs() < 1.0e-6);
    let s_out = (1.7 * 0.6 * age).sin();
    assert!((salmon_body_back_yrot(age, false) - (-1.3 * 0.25 * s_out)).abs() < 1.0e-6);
}

#[test]
fn salmon_from_vanilla_variant_id_clamps() {
    // `Salmon.Variant`: SMALL(0) / MEDIUM(1, the default) / LARGE(2, any other id).
    assert_eq!(SalmonModelSize::from_vanilla_id(0), SalmonModelSize::Small);
    assert_eq!(SalmonModelSize::from_vanilla_id(1), SalmonModelSize::Medium);
    assert_eq!(SalmonModelSize::from_vanilla_id(2), SalmonModelSize::Large);
    assert_eq!(SalmonModelSize::from_vanilla_id(99), SalmonModelSize::Large);
}

#[test]
fn salmon_textured_layer_passes_match_vanilla_renderer() {
    // `SalmonRenderer` renders a single cutout base layer; only the bound model layer
    // differs per size (`ModelLayers.SALMON` / `SALMON_SMALL` / `SALMON_LARGE`).
    let passes = salmon_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, SALMON_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn salmon_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&salmon_texture_images()).unwrap();
    // Eight cubes → 192 textured vertices on the cutout pass.
    let base = EntityModelInstance::salmon(720, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Medium)
        .with(|s| {
            s.in_water = true;
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let still = entity_model_textured_meshes(&[base], &atlas);
    assert_salmon_base_submission(&still, base);
    assert_ne!(base.render_state.overlay_coords(), [0.0, 10.0]);
    assert!(still.translucent.vertices.is_empty());
    assert!(still.eyes.vertices.is_empty());
    assert_eq!(still.cutout.vertices.len(), 192);

    // The back-body sway / body wiggle reorient the mesh as the age advances.
    let swimming_instance = base.with(|s| s.age_in_ticks = 7.0);
    let swimming = entity_model_textured_meshes(&[swimming_instance], &atlas);
    assert_salmon_base_submission(&swimming, swimming_instance);
    assert_eq!(still.cutout.vertices.len(), swimming.cutout.vertices.len());
    assert_ne!(still.cutout.vertices, swimming.cutout.vertices);

    // A beached salmon flops onto its side.
    let beached_instance = base.with(|s| s.in_water = false);
    let beached = entity_model_textured_meshes(&[beached_instance], &atlas);
    assert_salmon_base_submission(&beached, beached_instance);
    assert_ne!(still.cutout.vertices, beached.cutout.vertices);

    // The size variants scale the mesh.
    let small_instance =
        EntityModelInstance::salmon(721, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Small).with(|s| {
            s.in_water = true;
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let small = entity_model_textured_meshes(&[small_instance], &atlas);
    assert_salmon_base_submission(&small, small_instance);
    assert_eq!(small.cutout.vertices.len(), still.cutout.vertices.len());
    assert_ne!(
        small.cutout.vertices, still.cutout.vertices,
        "the small salmon is scaled"
    );
}

fn salmon_texture_images() -> Vec<EntityModelTextureImage> {
    [SALMON_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_salmon_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
