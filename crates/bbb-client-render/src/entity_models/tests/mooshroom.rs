use super::*;

#[test]
fn mooshroom_variants_match_vanilla_ids() {
    assert_eq!(MooshroomVariant::from_vanilla_id(-1), MooshroomVariant::Red);
    assert_eq!(
        MooshroomVariant::from_vanilla_id(2),
        MooshroomVariant::Brown
    );
    assert_eq!(MooshroomVariant::Red.serialized_name(), "red");
    assert_eq!(MooshroomVariant::Brown.serialized_name(), "brown");
}

#[test]
fn mooshroom_textured_render_reuses_cow_geometry_with_the_mooshroom_recolor() {
    // Vanilla `MushroomCowRenderer.getTextureLocation`: red/brown × adult/baby.
    for (baby, variant, texture) in [
        (false, MooshroomVariant::Red, MOOSHROOM_TEXTURE_REF),
        (true, MooshroomVariant::Red, MOOSHROOM_BABY_TEXTURE_REF),
        (false, MooshroomVariant::Brown, MOOSHROOM_BROWN_TEXTURE_REF),
        (
            true,
            MooshroomVariant::Brown,
            MOOSHROOM_BROWN_BABY_TEXTURE_REF,
        ),
    ] {
        let passes = mooshroom_textured_layer_passes(baby, variant);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
    }
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_BROWN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_BROWN_BABY_TEXTURE_REF));

    // The atlas carries both the mooshroom recolor and the cow textures so the cow comparison emit
    // below can resolve its own texture.
    let images: Vec<EntityModelTextureImage> = [
        MOOSHROOM_TEXTURE_REF,
        MOOSHROOM_BABY_TEXTURE_REF,
        MOOSHROOM_BROWN_TEXTURE_REF,
        MOOSHROOM_BROWN_BABY_TEXTURE_REF,
    ]
    .iter()
    .chain(&texture_refs_matching(|path| {
        path.starts_with("textures/entity/cow/cow_")
    }))
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for (baby, variant, expected_texture) in [
        (false, MooshroomVariant::Red, MOOSHROOM_TEXTURE_REF),
        (true, MooshroomVariant::Red, MOOSHROOM_BABY_TEXTURE_REF),
        (false, MooshroomVariant::Brown, MOOSHROOM_BROWN_TEXTURE_REF),
        (
            true,
            MooshroomVariant::Brown,
            MOOSHROOM_BROWN_BABY_TEXTURE_REF,
        ),
    ] {
        let instance = EntityModelInstance::new(
            900,
            EntityModelKind::Mooshroom { baby, variant },
            [0.0, 64.0, 0.0],
            0.0,
        )
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let mooshroom = entity_model_textured_meshes(&[instance], &atlas);
        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == expected_texture)
            .unwrap()
            .uv;
        assert!(mooshroom.cutout.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
        assert!(mooshroom.translucent.vertices.is_empty());
        assert!(mooshroom.eyes.vertices.is_empty());

        // The mooshroom reuses the cow tree, so it emits the same textured geometry the cow does.
        let cow = entity_model_textured_meshes(
            &[EntityModelInstance::cow(900, [0.0, 64.0, 0.0], 0.0, baby)],
            &atlas,
        );
        assert!(
            !mooshroom.cutout.vertices.is_empty(),
            "baby={baby} variant={variant:?} emits textured geometry"
        );
        assert_eq!(mooshroom.cutout.vertices.len(), cow.cutout.vertices.len());
        assert!(mooshroom
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
    }
}
