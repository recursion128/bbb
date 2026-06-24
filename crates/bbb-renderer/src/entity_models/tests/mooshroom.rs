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
    assert_eq!(
        EntityModelKind::Mooshroom { baby: false }.model_key(),
        "mooshroom"
    );
    assert_eq!(
        EntityModelKind::Mooshroom { baby: true }.model_key(),
        "mooshroom_baby"
    );
}

#[test]
fn mooshroom_textured_render_reuses_cow_geometry_with_the_mooshroom_recolor() {
    assert_eq!(
        mooshroom_textured_layer_passes(false)[0].texture,
        MOOSHROOM_TEXTURE_REF
    );
    assert_eq!(
        mooshroom_textured_layer_passes(true)[0].texture,
        MOOSHROOM_BABY_TEXTURE_REF
    );
    assert_eq!(
        EntityModelKind::Mooshroom { baby: false }.vanilla_texture_ref(),
        Some(MOOSHROOM_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Mooshroom { baby: true }.vanilla_texture_ref(),
        Some(MOOSHROOM_BABY_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MOOSHROOM_BABY_TEXTURE_REF));
    assert_eq!(
        mooshroom_entity_texture_refs(),
        &[MOOSHROOM_TEXTURE_REF, MOOSHROOM_BABY_TEXTURE_REF]
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
    for baby in [false, true] {
        let mooshroom = entity_model_textured_mesh(
            &[EntityModelInstance::mooshroom(
                900,
                [0.0, 64.0, 0.0],
                0.0,
                baby,
            )],
            &atlas,
        );
        // The mooshroom reuses the cow tree, so it emits the same textured geometry the cow does.
        let cow = entity_model_textured_mesh(
            &[EntityModelInstance::cow(901, [0.0, 64.0, 0.0], 0.0, baby)],
            &atlas,
        );
        assert!(
            !mooshroom.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert_eq!(mooshroom.vertices.len(), cow.vertices.len());
        assert!(mooshroom
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}
