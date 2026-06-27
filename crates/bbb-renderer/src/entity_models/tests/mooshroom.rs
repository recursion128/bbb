use super::*;

#[test]
fn mooshroom_mesh_matches_the_temperate_cow_body() {
    // Vanilla `MushroomCowRenderer` renders the mooshroom with the shared `CowModel` body
    // (`ModelLayers.MOOSHROOM` bakes to the same temperate `cowBodyLayer` as `ModelLayers.COW`), so the
    // mooshroom emits the exact temperate-cow mesh — the real cow body, not the generic quadruped.
    let mooshroom = entity_model_mesh(&[EntityModelInstance::mooshroom(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let cow = entity_model_mesh(&[EntityModelInstance::cow(700, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(mooshroom.vertices, cow.vertices);
    assert_eq!(mooshroom.indices, cow.indices);

    // The temperate adult cow body is ten cubes → 60 faces / 240 vertices / 360 indices, COW_BROWN.
    assert_eq!(mooshroom.opaque_faces, 60);
    assert_eq!(mooshroom.vertices.len(), 240);
    assert_eq!(mooshroom.indices.len(), 360);
    assert!(mooshroom
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COW_BROWN, 1.0)));
}

#[test]
fn baby_mooshroom_mesh_matches_the_baby_cow_body() {
    // The baby mooshroom uses `BabyCowModel.createBodyLayer()` (`ModelLayers.MOOSHROOM_BABY`), the same
    // body as the baby cow, so its mesh matches the baby cow's and is smaller than the adult.
    let baby = entity_model_mesh(&[EntityModelInstance::mooshroom(
        701,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let baby_cow = entity_model_mesh(&[EntityModelInstance::cow(701, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(baby.vertices, baby_cow.vertices);

    let adult = entity_model_mesh(&[EntityModelInstance::mooshroom(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_height = adult_max[1] - adult_min[1];
    let baby_height = baby_max[1] - baby_min[1];
    assert!(
        baby_height < adult_height,
        "baby mooshroom height {baby_height} should be smaller than adult {adult_height}"
    );
}

#[test]
fn mooshroom_colored_runtime_skips_the_texture_backed_mooshroom() {
    // The mooshroom now binds the mooshroom recolor over the shared cow geometry, so — like the cow it
    // reuses — it renders on the textured path and the colored runtime path skips it (the full path
    // still emits the colored fallback). The block-mushroom layer and red/brown variants stay deferred.
    let mooshroom_instances = [EntityModelInstance::mooshroom(
        703,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )];
    assert!(!entity_model_mesh(&mooshroom_instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&mooshroom_instances)
        .vertices
        .is_empty());

    let cow_colored = entity_model_colored_runtime_mesh(&[EntityModelInstance::cow(
        703,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert!(
        cow_colored.vertices.is_empty(),
        "the textured cow is likewise skipped on the colored runtime path"
    );
}

#[test]
fn mooshroom_exposes_stable_model_keys() {
    // The model_key (mesh geometry) is variant-agnostic: red and brown share the `CowModel` body.
    for variant in [MooshroomVariant::Red, MooshroomVariant::Brown] {
        assert_eq!(
            EntityModelKind::Mooshroom {
                baby: false,
                variant,
            }
            .model_key(),
            "mooshroom"
        );
        assert_eq!(
            EntityModelKind::Mooshroom {
                baby: true,
                variant,
            }
            .model_key(),
            "mooshroom_baby"
        );
    }
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
        assert_eq!(passes[0].kind, EntityModelLayerKind::MooshroomBase);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    }
    let texture = |baby, variant| {
        EntityModelKind::Mooshroom { baby, variant }
            .vanilla_texture_ref()
            .unwrap()
            .path
    };
    assert_eq!(
        texture(false, MooshroomVariant::Red),
        "textures/entity/cow/mooshroom_red.png"
    );
    assert_eq!(
        texture(true, MooshroomVariant::Red),
        "textures/entity/cow/mooshroom_red_baby.png"
    );
    assert_eq!(
        texture(false, MooshroomVariant::Brown),
        "textures/entity/cow/mooshroom_brown.png"
    );
    assert_eq!(
        texture(true, MooshroomVariant::Brown),
        "textures/entity/cow/mooshroom_brown_baby.png"
    );
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_BROWN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_BROWN_BABY_TEXTURE_REF));
    assert_eq!(
        mooshroom_entity_texture_refs(),
        &[
            MOOSHROOM_TEXTURE_REF,
            MOOSHROOM_BABY_TEXTURE_REF,
            MOOSHROOM_BROWN_TEXTURE_REF,
            MOOSHROOM_BROWN_BABY_TEXTURE_REF,
        ]
    );

    // The atlas carries both the mooshroom recolor and the cow textures so the cow comparison emit
    // below can resolve its own texture.
    let images: Vec<EntityModelTextureImage> = mooshroom_entity_texture_refs()
        .iter()
        .chain(cow_entity_texture_refs())
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
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
        let mooshroom = entity_model_textured_meshes(&[instance], &atlas);
        assert!(mooshroom.translucent.vertices.is_empty());
        assert!(mooshroom.eyes.vertices.is_empty());
        assert_eq!(mooshroom.submissions.len(), 1);
        let submit = mooshroom.submissions[0];
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, expected_texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, instance.render_state.overlay_coords());
        assert_ne!(submit.overlay, [0.0, 10.0]);

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
                && vertex.light == submit.light
                && vertex.overlay == submit.overlay));
    }
}
