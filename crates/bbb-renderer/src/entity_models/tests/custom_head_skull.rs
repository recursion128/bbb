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
fn custom_head_player_skull_model_adds_the_humanoid_hat_geometry() {
    // Vanilla `SkullModel.createHumanoidHeadLayer` adds a `hat` child inflated by
    // `CubeDeformation(0.25)` at `texOffs(32, 0)` on the 64x64 player skin.
    assert_eq!(CUSTOM_HEAD_PLAYER_HAT_CUBE.min, [-4.25, -8.25, -4.25]);
    assert_eq!(CUSTOM_HEAD_PLAYER_HAT_CUBE.size, [8.5, 8.5, 8.5]);
    assert_eq!(CUSTOM_HEAD_PLAYER_HAT_CUBE.uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(CUSTOM_HEAD_PLAYER_HAT_CUBE.tex, [32.0, 0.0]);
    assert!(!CUSTOM_HEAD_PLAYER_HAT_CUBE.mirror);
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
fn custom_head_skull_layer_renders_profileless_player_head_with_default_skin() {
    let atlas = atlas_with(PLAYER_SLIM_STEVE_TEXTURE_REF);
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            913,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Player(
            EntityDefaultPlayerSkin::SlimSteve,
        )))],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 12);
    assert_eq!(mesh.vertices.len(), 48);
    assert_eq!(mesh.indices.len(), 72);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn custom_head_skull_layer_uses_profile_default_player_skin_texture() {
    let atlas = build_entity_model_texture_atlas(&[
        EntityModelTextureImage::new(
            PLAYER_SLIM_ALEX_TEXTURE_REF,
            vec![0; usize::try_from(64 * 64 * 4).unwrap()],
        ),
        EntityModelTextureImage::new(
            PLAYER_WIDE_STEVE_TEXTURE_REF,
            vec![0; usize::try_from(64 * 64 * 4).unwrap()],
        ),
    ])
    .unwrap()
    .0;
    let slim = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            914,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Player(
            EntityDefaultPlayerSkin::SlimAlex,
        )))],
        &atlas,
    );
    let wide = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            915,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Player(
            EntityDefaultPlayerSkin::WideSteve,
        )))],
        &atlas,
    );

    assert_eq!(slim.cutout_faces, 12);
    assert_eq!(wide.cutout_faces, 12);
    assert_eq!(
        slim.vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
        wide.vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        slim.vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>(),
        wide.vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>()
    );
}

#[test]
fn custom_head_skull_layer_renders_piglin_head_with_specialized_geometry() {
    let atlas = atlas_with(PIGLIN_TEXTURE_REF);
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            914,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Piglin))],
        &atlas,
    );

    // Vanilla `PiglinHeadModel.createHeadModel` reuses `PiglinModel.addHead`: four head cubes and
    // two ear cubes, each rendered as a normal cutout cube.
    assert_eq!(mesh.cutout_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn custom_head_piglin_skull_animates_ears_from_worn_head_animation_pos() {
    let atlas = atlas_with(PIGLIN_TEXTURE_REF);
    let base = EntityModelInstance::player_with_parts(
        915,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Piglin));

    let first = entity_model_textured_mesh(&[base.with_worn_head_animation_pos(0.0)], &atlas);
    let later = entity_model_textured_mesh(&[base.with_worn_head_animation_pos(7.0)], &atlas);

    assert_eq!(first.cutout_faces, later.cutout_faces);
    assert_eq!(first.vertices.len(), later.vertices.len());
    assert_ne!(
        first.vertices, later.vertices,
        "PiglinHeadModel drives its ear zRot from SkullModelBase.State.animationPos"
    );
}

#[test]
fn custom_head_dragon_skull_model_uses_vanilla_head_layer_pose() {
    // Vanilla `DragonHeadModel.createHeadLayer`: `PartPose.offset(0, -7.986666, 0).scaled(0.75)`.
    assert_eq!(CUSTOM_HEAD_DRAGON_HEAD_POSE.offset, [0.0, -7.986666, 0.0]);
    assert_eq!(CUSTOM_HEAD_DRAGON_HEAD_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(CUSTOM_HEAD_DRAGON_HEAD_SCALE, [0.75, 0.75, 0.75]);
}

#[test]
fn custom_head_skull_layer_renders_dragon_head_with_specialized_geometry() {
    let atlas = atlas_with(ENDER_DRAGON_TEXTURE_REF);
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            916,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Dragon))],
        &atlas,
    );

    // Vanilla `DragonHeadModel.createHeadLayer`: six head cubes plus one jaw cube.
    assert_eq!(mesh.cutout_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn custom_head_dragon_skull_animates_jaw_from_worn_head_animation_pos() {
    let atlas = atlas_with(ENDER_DRAGON_TEXTURE_REF);
    let base = EntityModelInstance::player_with_parts(
        917,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Dragon));

    let first = entity_model_textured_mesh(&[base.with_worn_head_animation_pos(0.0)], &atlas);
    let later = entity_model_textured_mesh(&[base.with_worn_head_animation_pos(2.5)], &atlas);

    assert_eq!(first.cutout_faces, later.cutout_faces);
    assert_eq!(first.vertices.len(), later.vertices.len());
    assert_ne!(
        first.vertices, later.vertices,
        "DragonHeadModel drives its jaw xRot from SkullModelBase.State.animationPos"
    );
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
