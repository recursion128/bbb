use super::*;

#[test]
fn rabbit_textured_render_matches_vanilla_renderer() {
    // `RabbitRenderer.getTextureLocation`: the seven `Rabbit.Variant` colours × {adult, baby}, with
    // the `Toast` named-rabbit override (which ignores the variant).
    let variants = [
        RabbitModelVariant::Brown,
        RabbitModelVariant::White,
        RabbitModelVariant::Black,
        RabbitModelVariant::WhiteSplotched,
        RabbitModelVariant::Gold,
        RabbitModelVariant::Salt,
        RabbitModelVariant::Evil,
    ];
    for variant in variants {
        for baby in [false, true] {
            for toast in [false, true] {
                let texture = rabbit_texture_ref(variant, baby, toast);
                let passes = rabbit_textured_layer_passes(variant, baby, toast);
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
    // The Toast override resolves to the same texture regardless of the colour variant.
    assert_eq!(
        rabbit_texture_ref(RabbitModelVariant::Evil, false, true),
        rabbit_texture_ref(RabbitModelVariant::Gold, false, true)
    );

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/rabbit/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let instance = EntityModelInstance::rabbit(
            900,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
            RabbitModelVariant::Gold,
            false,
        )
        .with(|s| {
            s.light_coords = (2_u32 << 4) | (14_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
    }
}
