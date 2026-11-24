use super::*;

fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_fixture(instances, atlas)
}

fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    skins: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
        instances, atlas, skins, textures,
    )
}

use crate::DynamicEntityTextureImage;

#[test]
fn generic_dynamic_entity_texture_uses_shared_atlas_and_static_fallback() {
    let (static_atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let texture = EntityDynamicTexture { handle: 8_001 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: texture.handle,
        size: [7, 5],
        standalone_sampling: None,
        rgba: vec![0x7f; 7 * 5 * 4],
    }])
    .unwrap()
    .0;

    let instance = EntityModelInstance {
        dynamic_texture: Some(texture),
        ..EntityModelInstance::pig(
            8_001,
            [0.0, 0.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        )
    };
    let ready = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(ready.cutout.vertices.is_empty());
    assert!(!ready.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(ready
        .dynamic_player_texture_cutout
        .vertices
        .iter()
        .all(|vertex| {
            vertex.standalone_local_uv == [0.0; 2]
                && vertex.standalone_pixel_origin == [0.0; 2]
                && vertex.standalone_size_flags == [0.0; 4]
        }));

    let pending =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);
    assert!(!pending.cutout.vertices.is_empty());
    assert!(pending.dynamic_player_texture_cutout.vertices.is_empty());
}

#[test]
fn pig_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = pig_textured_layer_passes(PigModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(
        temperate[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(temperate[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(temperate[0].texture, PIG_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(temperate[0].order, 0);

    let warm_baby = pig_textured_layer_passes(PigModelVariant::Warm, true);
    assert_eq!(warm_baby[0].texture, PIG_WARM_BABY_TEXTURE_REF);

    let cold_adult = pig_textured_layer_passes(PigModelVariant::Cold, false);
    assert_eq!(cold_adult[0].texture, PIG_COLD_TEXTURE_REF);
}

#[test]
fn entity_texture_atlas_stitches_official_pig_png_slots() {
    let images = pig_texture_images();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    let textures = &texture_refs_matching(|path| path.starts_with("textures/entity/pig/"));
    assert_texture_atlas_layout_matches_refs(&layout, textures);
    let warm_baby_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 3), 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 4), 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn pig_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let instances = [
        EntityModelInstance::pig(
            501,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        )
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        }),
        EntityModelInstance::pig(502, [1.0, 64.0, 0.0], 0.0, PigModelVariant::Cold, false).with(
            |s| {
                s.light_coords = (5_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            },
        ),
        EntityModelInstance::pig(503, [2.0, 64.0, 0.0], 0.0, PigModelVariant::Warm, true).with(
            |s| {
                s.light_coords = (5_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            },
        ),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_pig_only_uses_cutout_buckets(&meshes);

    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.vertices[168].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.vertices[360].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instances[0].render_state.shader_light()
        && vertex.overlay == instances[0].render_state.overlay_coords()));
    assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(mesh);
    assert!(max[0] - min[0] > 2.0);
    assert_close3([min[1], max[1], max[2] - min[2]], [64.001, 65.001, 1.5]);
}

#[test]
fn pig_saddle_layer_renders_for_adults_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        PIG_TEMPERATE_TEXTURE_REF,
        PIG_SADDLE_TEXTURE_REF,
        PIG_TEMPERATE_BABY_TEXTURE_REF,
    ]))
    .unwrap();
    let adult = EntityModelInstance::pig(
        521,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult
        .with(|s| s.kind.pig_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id()));
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_pig_only_uses_cutout_buckets(&bare_meshes);

    assert_pig_only_uses_cutout_buckets(&saddled_meshes);

    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    let saddle_mesh = &saddled_meshes.armor_cutout;

    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddle_mesh.cutout_faces, 42);
    assert_eq!(saddle_mesh.vertices.len(), 168);
    assert_close2(saddle_mesh.vertices[0].uv, [16.0 / 64.0, 64.0 / 160.0]);
    let (bare_min, bare_max) = textured_mesh_extents(bare);
    let (saddle_min, saddle_max) = textured_mesh_extents(saddle_mesh);
    assert!(saddle_min[0] < bare_min[0]);
    assert!(saddle_max[0] > bare_max[0]);

    let baby_instance =
        EntityModelInstance::pig(523, [0.0, 64.0, 0.0], 0.0, PigModelVariant::Temperate, true)
            .with(|s| {
                s.kind.pig_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
                s.light_coords = (4_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_pig_only_uses_cutout_buckets(&baby_meshes);

    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 42);
    assert_eq!(baby.vertices.len(), 168);
}

#[test]
fn pig_saddle_missing_texture_preserves_other_geometry() {
    // Vanilla `PigRenderer` routes the saddle item through `SimpleEquipmentLayer`, which submits
    // `PIG_SADDLE` equipment before texture atlas data is resolved; missing texture data suppresses
    // only the folded saddle geometry.
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let adult = EntityModelInstance::pig(
        524,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult
        .with(|s| s.kind.pig_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id()));
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);

    assert_pig_only_uses_cutout_buckets(&bare_meshes);
    assert_pig_only_uses_cutout_buckets(&saddled_meshes);

    assert_eq!(
        saddled_meshes.cutout.cutout_faces,
        bare_meshes.cutout.cutout_faces
    );
    assert_eq!(saddled_meshes.cutout.vertices, bare_meshes.cutout.vertices);
    assert!(saddled_meshes.armor_cutout.vertices.is_empty());
}

#[test]
fn pig_saddle_foil_and_trim_follow_the_base_equipment_layer() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        PIG_TEMPERATE_TEXTURE_REF,
        PIG_TEMPERATE_BABY_TEXTURE_REF,
        PIG_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 52_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let trimmed_saddle = |entity_id, baby| {
        EntityModelInstance::pig(
            entity_id,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            baby,
        )
        .with(|s| {
            s.kind.pig_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
            s.kind.pig_mut().saddle_foil = true;
            s.kind.pig_mut().saddle_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            });
        })
    };

    let adult = trimmed_saddle(525, false);
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[adult],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.armor_cutout.vertices.len(), 168);
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 168);
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 168);

    let baby = trimmed_saddle(526, true);
    let baby_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[baby],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(baby_meshes.armor_cutout.vertices.is_empty());
    assert!(baby_meshes.armor_entity_glint.vertices.is_empty());
    assert!(baby_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .is_empty());
}

#[test]
fn pig_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let base = EntityModelInstance::pig(
        504,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (55.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -25.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_pig_only_uses_cutout_buckets(&resting);
    assert_pig_only_uses_cutout_buckets(&yawed);
    assert_pig_only_uses_cutout_buckets(&pitched);

    // Head look turns the textured head part without adding or dropping vertices.
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(resting.cutout.vertices, pitched.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn pig_textured_mesh_swings_its_legs_when_walking() {
    // The real pig render path (textured) consumes the projected limb swing through
    // the shared QuadrupedModel leg rotation: a standing pig is inert, a walking
    // pig keeps its vertex count but lifts its feet, for every variant and the baby.
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    for (variant, baby) in [
        (PigModelVariant::Temperate, false),
        (PigModelVariant::Warm, false),
        (PigModelVariant::Cold, false),
        (PigModelVariant::Temperate, true),
    ] {
        let base = EntityModelInstance::pig(511, [0.0, 64.0, 0.0], 0.0, variant, baby);
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_pig_only_uses_cutout_buckets(&resting);
        assert_pig_only_uses_cutout_buckets(&still);
        assert_pig_only_uses_cutout_buckets(&walking);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{variant:?} baby={baby}: a standing pig is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{variant:?} baby={baby}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{variant:?} baby={baby}: a walking pig differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
        let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{variant:?} baby={baby}: a walking pig's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{variant:?} baby={baby}: a walking pig's legs should splay along Z"
        );
    }
}

fn assert_pig_only_uses_cutout_buckets(meshes: &EntityModelTexturedMeshes) {
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

fn pig_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&texture_refs_matching(|path| {
        path.starts_with("textures/entity/pig/")
    }))
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
