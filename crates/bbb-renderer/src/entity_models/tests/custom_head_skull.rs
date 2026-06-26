use super::*;

fn atlas_with(texture: EntityModelTextureRef) -> EntityModelTextureAtlasLayout {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    build_entity_model_texture_atlas(&[EntityModelTextureImage::new(texture, vec![0; len])])
        .unwrap()
        .0
}

#[test]
fn custom_head_skull_model_uses_vanilla_mob_head_layer_geometry() {
    // Vanilla `SkullModel.createMobHeadLayer` (atlas 64x32): one `head` part at ZERO with
    // `addBox(-4, -8, -4, 8, 8, 8)` and `texOffs(0, 0)`.
    assert_eq!(CUSTOM_HEAD_SKULL_CUBE.min, [-4.0, -8.0, -4.0]);
    assert_eq!(CUSTOM_HEAD_SKULL_CUBE.size, [8.0, 8.0, 8.0]);
    assert_eq!(CUSTOM_HEAD_SKULL_CUBE.uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(CUSTOM_HEAD_SKULL_CUBE.tex, [0.0, 0.0]);
    assert!(!CUSTOM_HEAD_SKULL_CUBE.mirror);
}

#[test]
fn custom_head_skull_layer_renders_static_mob_heads_with_matching_textures() {
    for (skull, texture) in [
        (EntityCustomHeadSkull::Skeleton, SKELETON_TEXTURE_REF),
        (
            EntityCustomHeadSkull::WitherSkeleton,
            WITHER_SKELETON_TEXTURE_REF,
        ),
        (EntityCustomHeadSkull::Zombie, ZOMBIE_TEXTURE_REF),
        (EntityCustomHeadSkull::Creeper, CREEPER_TEXTURE_REF),
    ] {
        let atlas = atlas_with(texture);
        let mesh = entity_model_textured_mesh(
            &[EntityModelInstance::player_with_parts(
                910,
                [0.0, 64.0, 0.0],
                0.0,
                false,
                PLAYER_MODEL_PARTS_ALL_VISIBLE,
            )
            .with_custom_head_skull(Some(skull))],
            &atlas,
        );

        assert_eq!(mesh.cutout_faces, 6, "{skull:?}");
        assert_eq!(mesh.vertices.len(), 24, "{skull:?}");
        assert_eq!(mesh.indices.len(), 36, "{skull:?}");
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}

#[test]
fn custom_head_skull_layer_follows_host_head_pose() {
    let atlas = atlas_with(SKELETON_TEXTURE_REF);
    let base = EntityModelInstance::new(
        911,
        EntityModelKind::Zombie { baby: false },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Skeleton));

    let resting = entity_model_textured_mesh(&[base], &atlas);
    let looking = entity_model_textured_mesh(&[base.with_head_look(35.0, -20.0)], &atlas);

    assert_eq!(resting.vertices.len(), looking.vertices.len());
    assert_ne!(
        resting.vertices, looking.vertices,
        "CustomHeadLayer walks through the posed host head before rendering the skull"
    );
}

#[test]
fn custom_head_skull_layer_requires_a_custom_head_host_model() {
    let atlas = atlas_with(SKELETON_TEXTURE_REF);
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::new(912, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
                .with_custom_head_skull(Some(EntityCustomHeadSkull::Skeleton)),
        ],
        &atlas,
    );

    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}
