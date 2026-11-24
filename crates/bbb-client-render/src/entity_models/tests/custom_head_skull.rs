use super::*;
use crate::player_skin::DynamicPlayerSkinImage;

fn atlas_with(texture: EntityModelTextureRef) -> EntityModelTextureAtlasLayout {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    build_entity_model_texture_atlas(&[EntityModelTextureImage::new(texture, vec![0; len])])
        .unwrap()
        .0
}

fn atlas_with_many(textures: &[EntityModelTextureRef]) -> EntityModelTextureAtlasLayout {
    let images: Vec<_> = textures
        .iter()
        .copied()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![0; len])
        })
        .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn assert_skull_layer_pass(
    skull: EntityCustomHeadSkull,
    texture: EntityModelTextureRef,
    render_type: EntityModelLayerRenderType,
) {
    let pass = custom_head_skull_layer_pass(skull, texture);
    assert_eq!(pass.render_type, render_type);
    let expected_render_type_name = match render_type {
        EntityModelLayerRenderType::EntityCutoutZOffset => "entityCutoutZOffset",
        EntityModelLayerRenderType::EntityTranslucent => "entityTranslucent",
        _ => render_type.vanilla_name(),
    };
    assert_eq!(pass.render_type.vanilla_name(), expected_render_type_name);
    assert_eq!(pass.texture, texture);
    assert_eq!(pass.visibility, EntityModelLayerVisibility::All);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);
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
        assert_skull_layer_pass(
            skull,
            texture,
            EntityModelLayerRenderType::EntityCutoutZOffset,
        );
        let atlas = atlas_with(texture);
        let instance = EntityModelInstance::player_with_parts(
            910,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
        .with(|s| {
            s.custom_head_skull = Some(skull);
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);

        // `ZOMBIE_HEAD` is the humanoid head layer, which keeps the inflated hat cube.
        let cubes = if skull == EntityCustomHeadSkull::Zombie {
            2
        } else {
            1
        };
        assert_eq!(meshes.cutout_z_offset.cutout_faces, 6 * cubes, "{skull:?}");
        assert_eq!(
            meshes.cutout_z_offset.vertices.len(),
            24 * cubes,
            "{skull:?}"
        );
        assert_eq!(
            meshes.cutout_z_offset.indices.len(),
            36 * cubes,
            "{skull:?}"
        );
        assert!(meshes.translucent.vertices.is_empty(), "{skull:?}");
        assert!(meshes
            .cutout_z_offset
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]));
        assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    }
}

#[test]
fn custom_head_static_skull_missing_texture_atlas_entry_suppresses_geometry() {
    let atlas = atlas_with(PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Skeleton,
        SKELETON_TEXTURE_REF,
        EntityModelLayerRenderType::EntityCutoutZOffset,
    );
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == SKELETON_TEXTURE_REF));
    let instance = EntityModelInstance::player_with_skin(
        917,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton);
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != [0.0, 10.0]));
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout_z_offset.vertices.is_empty());
}

#[test]
fn invisible_illusioner_custom_head_skull_submits_for_each_clone() {
    let atlas = atlas_with_many(&[ILLUSIONER_TEXTURE_REF, SKELETON_TEXTURE_REF]);
    let offsets = [
        [1.25, 0.0, -0.75],
        [-1.0, 1.0, 0.5],
        [0.5, 0.0, 1.0],
        [-1.5, 0.0, -1.25],
    ];
    let instance =
        EntityModelInstance::illager(68, [4.0, 64.0, -2.0], 30.0, IllagerModelFamily::Illusioner)
            .with_invisible(true)
            .with(|s| {
                s.age_in_ticks = 8.0;
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton);
                s.kind.illager_mut().clone_offsets = offsets;
            });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 240 * 4);
    assert_eq!(meshes.cutout_z_offset.vertices.len(), 24 * 4);
    assert!(meshes.translucent.vertices.is_empty());

    let first_offset = Vec3::from_array(expected_illusioner_clone_offset(instance, 0));
    for index in 1..4 {
        let delta =
            Vec3::from_array(expected_illusioner_clone_offset(instance, index)) - first_offset;
        for (first, clone) in meshes.cutout_z_offset.vertices[..24]
            .iter()
            .zip(&meshes.cutout_z_offset.vertices[index * 24..(index + 1) * 24])
        {
            assert_close3(
                clone.position,
                (Vec3::from_array(first.position) + delta).to_array(),
            );
        }
    }
}

fn expected_illusioner_clone_offset(instance: EntityModelInstance, index: usize) -> [f32; 3] {
    let offset = instance.render_state.kind.illager().clone_offsets[index];
    let i = index as f32;
    let age = instance.render_state.age_in_ticks;
    [
        offset[0] + (i + age * 0.5).cos() * 0.025,
        offset[1] + (i + age * 0.75).cos() * 0.0125,
        offset[2] + (i + age * 0.7).cos() * 0.025,
    ]
}

#[test]
fn marker_hidden_glowing_armor_stand_keeps_custom_head_skull_without_base_geometry() {
    // Vanilla runs `CustomHeadLayer` even when a marker armor stand's hidden/glowing base has no
    // render type, so the skull branch still submits `SkullBlockRenderer.submitSkull`.
    let atlas = atlas_with_many(&[ARMOR_STAND_TEXTURE_REF, SKELETON_TEXTURE_REF]);
    let instance = EntityModelInstance::armor_stand_with_marker(
        918,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton);
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true)
    .with_outline_color(0xff33_77cc);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout_z_offset.vertices.len(), 24);
    assert!(meshes
        .cutout_z_offset
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.outline.vertices.len(), 24);
    assert!(meshes
        .outline
        .vertices
        .iter()
        .all(|vertex| vertex.tint == super::super::geometry::argb_to_tint(0xff33_77cc)));
}

#[test]
fn hidden_zombie_keeps_custom_head_skull_without_base_geometry() {
    // Vanilla `CustomHeadLayer` has no `state.isInvisible` gate, so the skull branch still submits
    // when an invisible humanoid mob's base body render type is null.
    let atlas = atlas_with_many(&[ZOMBIE_TEXTURE_REF, SKELETON_TEXTURE_REF]);
    let instance = EntityModelInstance::zombie(919, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| {
            s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton);
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
        .with_invisible(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout_z_offset.vertices.len(), 24);
    assert!(meshes
        .cutout_z_offset
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
}

#[test]
fn custom_head_skull_layer_renders_profileless_player_head_with_default_skin() {
    let atlas = atlas_with(PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve,
        )),
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        EntityModelLayerRenderType::EntityCutoutZOffset,
    );
    let instance = EntityModelInstance::player_with_parts(
        910,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve,
        )));
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout_z_offset.cutout_faces, 12);
    assert_eq!(meshes.cutout_z_offset.vertices.len(), 48);
    assert_eq!(meshes.cutout_z_offset.indices.len(), 72);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .cutout_z_offset
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
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
    let slim_instance = EntityModelInstance::player_with_parts(
        914,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex),
        ));
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let wide_instance = EntityModelInstance::player_with_parts(
        915,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve),
        ));
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 42,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_instance = EntityModelInstance::player_with_parts(
        916,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            dynamic_skin,
        )));
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let slim_meshes = entity_model_textured_meshes(&[slim_instance], &atlas);
    let wide_meshes = entity_model_textured_meshes(&[wide_instance], &atlas);
    let dynamic_meshes = entity_model_textured_meshes(&[dynamic_instance], &atlas);

    assert_skull_layer_pass(
        EntityCustomHeadSkull::Player(EntityPlayerSkin::ProfiledDefault(
            EntityDefaultPlayerSkin::SlimAlex,
        )),
        PLAYER_SLIM_ALEX_TEXTURE_REF,
        EntityModelLayerRenderType::EntityTranslucent,
    );
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Player(EntityPlayerSkin::ProfiledDefault(
            EntityDefaultPlayerSkin::WideSteve,
        )),
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        EntityModelLayerRenderType::EntityTranslucent,
    );
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(dynamic_skin)),
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        EntityModelLayerRenderType::EntityTranslucent,
    );

    assert_ne!(dynamic_instance.render_state.overlay_coords(), [0.0, 10.0]);

    assert!(slim_meshes.cutout.vertices.is_empty());
    assert!(wide_meshes.cutout.vertices.is_empty());
    assert!(dynamic_meshes.cutout.vertices.is_empty());
    assert_eq!(slim_meshes.translucent.cutout_faces, 12);
    assert_eq!(wide_meshes.translucent.cutout_faces, 12);
    assert_eq!(dynamic_meshes.translucent.cutout_faces, 12);
    assert_eq!(
        slim_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
        wide_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        slim_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>(),
        wide_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        dynamic_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>(),
        wide_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>()
    );
    assert!(slim_meshes
        .translucent
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == slim_instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
    assert!(wide_meshes
        .translucent
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == wide_instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
    assert!(dynamic_meshes
        .translucent
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == dynamic_instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
}

#[test]
fn custom_head_ready_dynamic_player_skin_renders_from_dynamic_skin_atlas() {
    // Vanilla `PlayerSkinRenderCache.getOrDefault(profile).renderType()` returns the resolved
    // player skin as `SkullBlockRenderer.getPlayerSkinRenderType`, which is `entityTranslucent`.
    let static_atlas =
        atlas_with_many(&[PLAYER_SLIM_ALEX_TEXTURE_REF, PLAYER_WIDE_STEVE_TEXTURE_REF]);
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 42,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let instance = EntityModelInstance::player_with_parts(
        916,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            dynamic_skin,
        )));
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let rgba = (0..usize::try_from(64 * 64 * 4).unwrap())
        .map(|index| index as u8)
        .collect::<Vec<_>>();
    let (dynamic_atlas, dynamic_rgba) =
        build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
            handle: dynamic_skin.handle,
            standalone_sampling: None,
            rgba: rgba.clone(),
        }])
        .unwrap();

    let fallback_meshes = entity_model_textured_meshes(&[instance], &static_atlas);
    let dynamic_meshes = entity_model_textured_meshes_with_dynamic_skins(
        &[instance],
        &static_atlas,
        Some(&dynamic_atlas),
    );

    assert_eq!(dynamic_rgba, rgba);
    assert_eq!(dynamic_atlas.width, 64);
    assert_eq!(dynamic_atlas.height, 64);
    assert_eq!(dynamic_atlas.entries[0].handle, dynamic_skin.handle);
    assert_eq!(dynamic_atlas.entries[0].uv.min, [0.0, 0.0]);
    assert_eq!(dynamic_atlas.entries[0].uv.max, [1.0, 1.0]);

    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);

    assert!(dynamic_meshes.translucent.vertices.is_empty());
    assert_eq!(
        dynamic_meshes.dynamic_player_skin_translucent.cutout_faces,
        12
    );
    assert_eq!(
        dynamic_meshes
            .dynamic_player_skin_translucent
            .vertices
            .len(),
        48
    );
    assert_eq!(
        dynamic_meshes.dynamic_player_skin_translucent.indices.len(),
        72
    );
    assert_eq!(
        dynamic_meshes
            .dynamic_player_skin_translucent
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
        fallback_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        dynamic_meshes
            .dynamic_player_skin_translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>(),
        fallback_meshes
            .translucent
            .vertices
            .iter()
            .map(|vertex| vertex.uv)
            .collect::<Vec<_>>()
    );
    assert!(dynamic_meshes
        .dynamic_player_skin_translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
            && (0.0..=1.0).contains(&vertex.uv[0])
            && (0.0..=1.0).contains(&vertex.uv[1])));
}

#[test]
fn custom_head_skull_layer_renders_piglin_head_with_specialized_geometry() {
    let atlas = atlas_with(PIGLIN_TEXTURE_REF);
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Piglin,
        PIGLIN_TEXTURE_REF,
        EntityModelLayerRenderType::EntityCutoutZOffset,
    );
    let instance = EntityModelInstance::player_with_parts(
        914,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Piglin));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // Vanilla `PiglinHeadModel.createHeadModel` reuses `PiglinModel.addHead`: four head cubes and
    // two ear cubes, each rendered as a normal cutout cube.
    assert_eq!(meshes.cutout_z_offset.cutout_faces, 36);
    assert_eq!(meshes.cutout_z_offset.vertices.len(), 144);
    assert_eq!(meshes.cutout_z_offset.indices.len(), 216);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .cutout_z_offset
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
    .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Piglin));

    let first_instance = base.with(|s| s.worn_head_animation_pos = 0.0);
    let later_instance = base.with(|s| s.worn_head_animation_pos = 7.0);
    let first = entity_model_textured_meshes(&[first_instance], &atlas);
    let later = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_eq!(
        first.cutout_z_offset.cutout_faces,
        later.cutout_z_offset.cutout_faces
    );
    assert_eq!(
        first.cutout_z_offset.vertices.len(),
        later.cutout_z_offset.vertices.len()
    );
    assert_ne!(
        first.cutout_z_offset.vertices, later.cutout_z_offset.vertices,
        "PiglinHeadModel drives its ear zRot from SkullModelBase.State.animationPos"
    );
}

#[test]
fn custom_head_skull_layer_renders_dragon_head_with_specialized_geometry() {
    let atlas = atlas_with(ENDER_DRAGON_TEXTURE_REF);
    assert_skull_layer_pass(
        EntityCustomHeadSkull::Dragon,
        ENDER_DRAGON_TEXTURE_REF,
        EntityModelLayerRenderType::EntityCutoutZOffset,
    );
    let instance = EntityModelInstance::player_with_parts(
        916,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Dragon));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // Vanilla `DragonHeadModel.createHeadLayer`: six head cubes plus one jaw cube.
    assert_eq!(meshes.cutout_z_offset.cutout_faces, 42);
    assert_eq!(meshes.cutout_z_offset.vertices.len(), 168);
    assert_eq!(meshes.cutout_z_offset.indices.len(), 252);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .cutout_z_offset
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
    .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Dragon));

    let first_instance = base.with(|s| s.worn_head_animation_pos = 0.0);
    let later_instance = base.with(|s| s.worn_head_animation_pos = 2.5);
    let first = entity_model_textured_meshes(&[first_instance], &atlas);
    let later = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_eq!(
        first.cutout_z_offset.cutout_faces,
        later.cutout_z_offset.cutout_faces
    );
    assert_eq!(
        first.cutout_z_offset.vertices.len(),
        later.cutout_z_offset.vertices.len()
    );
    assert_ne!(
        first.cutout_z_offset.vertices, later.cutout_z_offset.vertices,
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
    .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton));

    let looking_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (35.0, -20.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let looking = entity_model_textured_meshes(&[looking_instance], &atlas);

    assert_eq!(
        resting.cutout_z_offset.vertices.len(),
        looking.cutout_z_offset.vertices.len()
    );
    assert_ne!(
        resting.cutout_z_offset.vertices, looking.cutout_z_offset.vertices,
        "CustomHeadLayer walks through the posed host head before rendering the skull"
    );
}

#[test]
fn custom_head_skull_layer_requires_a_custom_head_host_model() {
    let atlas = atlas_with_many(&[CREEPER_TEXTURE_REF, SKELETON_TEXTURE_REF]);
    let instance = EntityModelInstance::new(912, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.custom_head_skull = Some(EntityCustomHeadSkull::Skeleton));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}
