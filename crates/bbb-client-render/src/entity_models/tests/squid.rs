use super::*;

#[test]
fn squid_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let cases = [
        (false, false, SQUID_TEXTURE_REF),
        (false, true, SQUID_BABY_TEXTURE_REF),
        (true, false, GLOW_SQUID_TEXTURE_REF),
        (true, true, GLOW_SQUID_BABY_TEXTURE_REF),
    ];
    for (glow, baby, texture) in cases {
        let passes = squid_textured_layer_passes(glow, baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
    }
}

#[test]
fn squid_textured_mesh_uses_vanilla_geometry_and_variant_texture() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    let squid_instance =
        EntityModelInstance::squid(810, [0.0, 64.0, 0.0], 0.0, false, false).with(|s| {
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let squid_meshes = entity_model_textured_meshes(&[squid_instance], &atlas);
    assert_squid_submission(&squid_meshes, squid_instance);
    assert_ne!(squid_instance.render_state.overlay_coords(), [0.0, 10.0]);
    assert!(squid_meshes.translucent.vertices.is_empty());
    assert!(squid_meshes.eyes.vertices.is_empty());
    // Nine cubes (body + eight tentacles) → 216 textured vertices, all on the cutout pass.
    let squid = &squid_meshes.cutout;
    assert_eq!(squid.vertices.len(), 216);

    // The glow squid reuses the geometry at the same positions but samples a different
    // texture (glow_squid.png), so the vertex UVs differ from the plain squid.
    let glow_instance =
        EntityModelInstance::squid(811, [0.0, 64.0, 0.0], 0.0, true, false).with(|s| {
            s.light_coords = (15_u32 << 4) | (8_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let glow_meshes = entity_model_textured_meshes(&[glow_instance], &atlas);
    assert_squid_submission(&glow_meshes, glow_instance);
    let glow = &glow_meshes.cutout;
    assert_eq!(glow.vertices.len(), squid.vertices.len());
    assert_eq!(
        glow.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        squid
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        glow.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
        squid.vertices.iter().map(|v| v.uv).collect::<Vec<_>>()
    );

    // The baby uses `BabySquidModel`: an 8..24 px tall mesh on the 32x32 baby texture.
    let baby_instance =
        EntityModelInstance::squid(812, [0.0, 64.0, 0.0], 0.0, false, true).with(|s| {
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_squid_submission(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.vertices.len(), 216);
    let (baby_min, baby_max) = textured_mesh_extents(baby);
    assert!(
        ((baby_max[1] - baby_min[1]) - 1.0).abs() < 1.0e-3,
        "baby squid height {}",
        baby_max[1] - baby_min[1]
    );
}

#[test]
fn squid_textured_mesh_swims_its_tentacles() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    let base = EntityModelInstance::squid(813, [0.0, 64.0, 0.0], 0.0, false, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let rest = entity_model_textured_meshes(&[base], &atlas);
    assert_squid_submission(&rest, base);
    let swept_instance = base.with(|s| s.kind.squid_mut().tentacle_angle = 0.8);
    let swept = entity_model_textured_meshes(&[swept_instance], &atlas);
    assert_squid_submission(&swept, swept_instance);
    assert_eq!(rest.cutout.vertices.len(), swept.cutout.vertices.len());
    assert_ne!(rest.cutout.vertices, swept.cutout.vertices);
}

fn squid_texture_images() -> Vec<EntityModelTextureImage> {
    [
        SQUID_TEXTURE_REF,
        SQUID_BABY_TEXTURE_REF,
        GLOW_SQUID_TEXTURE_REF,
        GLOW_SQUID_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn assert_squid_submission(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
