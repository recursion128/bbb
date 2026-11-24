use super::*;

#[test]
fn fox_textured_render_matches_vanilla_renderer() {
    // `FoxRenderer.getTextureLocation`: the {red, snow} × {adult, baby} × {idle, sleeping} matrix.
    let variants = [FoxModelVariant::Red, FoxModelVariant::Snow];
    for variant in variants {
        for baby in [false, true] {
            for sleeping in [false, true] {
                let texture = fox_texture_ref(variant, baby, sleeping);
                let passes = fox_textured_layer_passes(variant, baby, sleeping);
                assert_eq!(passes.len(), 1);
                assert_eq!(
                    passes[0].render_type,
                    EntityModelLayerRenderType::EntityCutout
                );
                assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
                assert_eq!(passes[0].texture, texture);
                assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
                assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
                assert_eq!(passes[0].order, 0);
                assert!(entity_model_texture_refs().contains(&texture));
            }
        }
    }
    assert_eq!(FoxModelVariant::from_id(-1), FoxModelVariant::Red);
    assert_eq!(FoxModelVariant::from_id(2), FoxModelVariant::Red);
    assert_eq!(
        variants.map(FoxModelVariant::serialized_name),
        ["red", "snow"]
    );

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/fox/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        for sleeping in [false, true] {
            let instance =
                EntityModelInstance::fox(900, [0.0, 64.0, 0.0], 0.0, baby, FoxModelVariant::Snow)
                    .with(|s| {
                        s.kind.fox_mut().is_sleeping = sleeping;
                        s.light_coords = (3_u32 << 4) | (15_u32 << 20);
                        s.white_overlay_progress = 0.8;
                        s.has_red_overlay = true;
                    });
            let meshes = entity_model_textured_meshes(&[instance], &atlas);
            assert!(meshes.translucent.vertices.is_empty());
            assert!(meshes.eyes.vertices.is_empty());

            let mesh = &meshes.cutout;

            assert!(
                !mesh.vertices.is_empty(),
                "baby={baby} sleeping={sleeping} emits textured geometry"
            );
            assert!(mesh
                .vertices
                .iter()
                .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                    && vertex.light == instance.render_state.shader_light()
                    && vertex.overlay == instance.render_state.overlay_coords()));
        }
    }

    let pouncing =
        EntityModelInstance::fox(901, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Snow)
            .with(|s| s.kind.fox_mut().is_pouncing = true)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 30.0));
    let meshes = entity_model_textured_meshes(&[pouncing], &atlas);
    let grounded = entity_model_textured_meshes(
        &[pouncing.with(|s| s.kind.fox_mut().is_pouncing = false)],
        &atlas,
    );
    assert!(!meshes.cutout.vertices.is_empty());
    assert_ne!(meshes.cutout.vertices, grounded.cutout.vertices);
}
