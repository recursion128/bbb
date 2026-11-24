use super::*;

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

use crate::DynamicPlayerTextureImage;
use std::collections::BTreeMap;

fn atlas_for(textures: &[EntityModelTextureRef]) -> EntityModelTextureAtlasLayout {
    let images: Vec<_> = textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn owned_wings_entry(
    texture: EntityModelOwnedTextureRef,
    min: [f32; 2],
    max: [f32; 2],
) -> EntityModelOwnedTextureAtlasEntry {
    EntityModelOwnedTextureAtlasEntry {
        texture,
        uv: EntityModelUvRect { min, max },
    }
}

fn elytra_vertex_positions(
    meshes: &EntityModelTexturedMeshes,
    atlas: &EntityModelTextureAtlasLayout,
) -> Vec<[f32; 3]> {
    let entry = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("elytra atlas entry");
    meshes
        .armor_cutout
        .vertices
        .iter()
        .filter(|vertex| {
            vertex.uv[0] >= entry.uv.min[0]
                && vertex.uv[0] <= entry.uv.max[0]
                && vertex.uv[1] >= entry.uv.min[1]
                && vertex.uv[1] <= entry.uv.max[1]
        })
        .map(|vertex| vertex.position)
        .collect()
}

fn y_extent(positions: &[[f32; 3]]) -> f32 {
    let (min_y, max_y) = positions
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min_y, max_y), pos| {
            (min_y.min(pos[1]), max_y.max(pos[1]))
        });
    max_y - min_y
}

fn assert_elytra_vertices_have_vanilla_attributes(
    meshes: &EntityModelTexturedMeshes,
    atlas: &EntityModelTextureAtlasLayout,
    instance: EntityModelInstance,
) {
    let entry = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("elytra atlas entry");
    let vertices: Vec<_> = meshes
        .armor_cutout
        .vertices
        .iter()
        .filter(|vertex| {
            vertex.uv[0] >= entry.uv.min[0]
                && vertex.uv[0] <= entry.uv.max[0]
                && vertex.uv[1] >= entry.uv.min[1]
                && vertex.uv[1] <= entry.uv.max[1]
        })
        .collect();
    assert_eq!(vertices.len(), 48);
    assert!(vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
}

#[test]
fn non_player_humanoid_wings_layer_uses_static_equipment_texture() {
    // Vanilla `HumanoidMobRenderer` adds `WingsLayer` to humanoid mobs. The profile
    // elytra/cape override is only for `AvatarRenderState`, so a zombie with a
    // WINGS asset still uses the equipment texture even when `use_player_texture` is true.
    let pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 911,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_elytra.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x77; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::zombie(81, [1.0, 64.0, -2.0], 25.0, false).with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&meshes, &atlas, instance);
}

#[test]
fn non_player_humanoid_wings_missing_texture_atlas_entry_suppresses_geometry() {
    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF]);
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF));
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 912,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_elytra.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x44; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::zombie(86, [1.0, 64.0, -2.0], 25.0, false).with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.armor_cutout.vertices.is_empty());
}

#[test]
fn small_armor_stand_wings_layer_uses_baby_elytra_model() {
    // Vanilla `ArmorStand.isBaby()` returns `isSmall()`, and `WingsLayer` selects
    // `ModelLayers.ELYTRA_BABY` when `state.isBaby`.
    let baby_pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(
        baby_pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(baby_pass.order, 0);

    let atlas = atlas_for(&[ARMOR_STAND_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let adult = EntityModelInstance::armor_stand(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let small = EntityModelInstance::armor_stand(
        83,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let adult_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[adult], &atlas, None, None);
    let small_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[small], &atlas, None, None);
    let adult_positions = elytra_vertex_positions(&adult_meshes, &atlas);
    let small_positions = elytra_vertex_positions(&small_meshes, &atlas);

    assert_eq!(adult_positions.len(), 48);
    assert_eq!(small_positions.len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&adult_meshes, &atlas, adult);
    assert_elytra_vertices_have_vanilla_attributes(&small_meshes, &atlas, small);
    assert!(
        (y_extent(&adult_positions) * 0.5 - y_extent(&small_positions)).abs() < 1.0e-5,
        "small armor stand elytra should use the half-scale baby layer"
    );
}

#[test]
fn marker_hidden_glowing_armor_stand_keeps_wings_layer_without_base_geometry() {
    // Vanilla `ArmorStandRenderer.getRenderType` returns null for hidden marker bases even when
    // glowing, but `LivingEntityRenderer.submit` still runs `WingsLayer` afterwards.
    let atlas = atlas_for(&[ARMOR_STAND_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let instance = EntityModelInstance::armor_stand_with_marker(
        87,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true)
    .with_outline_color(0xff55_aa11);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
}

#[test]
fn hidden_zombie_keeps_wings_layer_without_base_geometry() {
    // Vanilla `WingsLayer` has no `state.isInvisible` gate, so an invisible humanoid mob keeps the
    // WINGS equipment layer even when the base body render type is null.
    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let instance = EntityModelInstance::zombie(88, [1.0, 64.0, -2.0], 25.0, false)
        .with(|s| {
            s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
        .with_invisible(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
}

#[test]
fn hidden_player_keeps_wings_layer_without_base_geometry() {
    // Player `WingsLayer` is also ungated by `state.isInvisible`.
    let atlas = atlas_for(&[
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
    ]);
    let instance = EntityModelInstance::player_with_skin(
        89,
        [1.0, 64.0, -2.0],
        25.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
}

#[test]
fn baby_zombie_wings_layer_uses_baby_elytra_model() {
    // Vanilla `WingsLayer` is attached by `HumanoidMobRenderer`, and its model
    // choice keys off `HumanoidRenderState.isBaby`.
    let atlas = atlas_for(&[
        ZOMBIE_TEXTURE_REF,
        ZOMBIE_BABY_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
    ]);
    let adult = EntityModelInstance::zombie(84, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby = EntityModelInstance::zombie(85, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let adult_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[adult], &atlas, None, None);
    let baby_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[baby], &atlas, None, None);
    let adult_positions = elytra_vertex_positions(&adult_meshes, &atlas);
    let baby_positions = elytra_vertex_positions(&baby_meshes, &atlas);

    assert_eq!(adult_positions.len(), 48);
    assert_eq!(baby_positions.len(), 48);
    assert_elytra_vertices_have_vanilla_attributes(&adult_meshes, &atlas, adult);
    assert_elytra_vertices_have_vanilla_attributes(&baby_meshes, &atlas, baby);
    assert!(
        (y_extent(&adult_positions) * 0.5 - y_extent(&baby_positions)).abs() < 1.0e-5,
        "baby zombie elytra should use the half-scale baby layer"
    );
}

#[test]
fn wings_trim_follows_foil_for_static_and_profile_textures() {
    let atlas = atlas_for(&[
        ZOMBIE_TEXTURE_REF,
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]);
    let trim_texture = EntityDynamicTexture { handle: 91_501 };
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 91_502,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicPlayerTextureImage {
            handle: trim_texture.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0xff; 64 * 32 * 4],
        },
        DynamicPlayerTextureImage {
            handle: profile_elytra.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x77; 64 * 32 * 4],
        },
    ])
    .unwrap()
    .0;
    let trim = Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    });

    let zombie = EntityModelInstance::zombie(86, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.chest_wings_foil = true;
        s.chest_wings_trim = trim;
    });
    let zombie_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[zombie],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(zombie_meshes.armor_entity_glint.vertices.len(), 48);
    assert_eq!(
        zombie_meshes
            .dynamic_player_texture_armor_trim
            .vertices
            .len(),
        48
    );

    let player = EntityModelInstance::player_with_skin(
        87,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.chest_wings_foil = true;
        s.chest_wings_trim = trim;
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
    });
    let player_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[player],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(player_meshes.armor_entity_glint.vertices.len(), 48);
    assert_eq!(
        player_meshes
            .dynamic_player_texture_armor_trim
            .vertices
            .len(),
        48
    );
}

#[test]
fn baby_wings_trim_uses_baby_model_and_survives_missing_base_texture() {
    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF]);
    let trim_texture = EntityDynamicTexture { handle: 91_503 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let trimmed = |entity_id, baby| {
        EntityModelInstance::zombie(entity_id, [0.0, 64.0, 0.0], 0.0, baby).with(|s| {
            s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
            s.chest_wings_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            });
        })
    };

    let adult = trimmed(88, false);
    let baby = trimmed(89, true);
    let adult_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[adult],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let baby_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[baby],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    for meshes in [&adult_meshes, &baby_meshes] {
        assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 48);
        assert!(meshes.armor_cutout.vertices.is_empty());
    }

    let adult_positions: Vec<_> = adult_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .iter()
        .map(|vertex| vertex.position)
        .collect();
    let baby_positions: Vec<_> = baby_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .iter()
        .map(|vertex| vertex.position)
        .collect();
    assert!(
        (y_extent(&adult_positions) * 0.5 - y_extent(&baby_positions)).abs() < 1.0e-5,
        "WINGS trims are retained on the half-scale baby elytra model"
    );
}

#[test]
fn equipment_asset_wings_use_per_layer_player_override_and_owned_texture_lookup() {
    let generation = 9;
    let override_fallback =
        EntityModelOwnedTextureRef::new("example:equipment/wings_profile_fallback.png", [64, 32]);
    let second_layer =
        EntityModelOwnedTextureRef::new("example:equipment/wings_overlay.png", [64, 32]);
    let owned_entries = vec![
        owned_wings_entry(override_fallback.clone(), [0.0, 0.0], [0.5, 1.0]),
        owned_wings_entry(second_layer.clone(), [0.5, 0.0], [1.0, 1.0]),
    ];
    let catalog = EntityEquipmentAssetCatalog::new(
        generation,
        vec![EntityEquipmentAsset {
            layers: BTreeMap::from([(
                EntityEquipmentLayerType::Wings,
                vec![
                    EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(override_fallback),
                        dyeable: None,
                        use_player_texture: true,
                    },
                    EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(second_layer),
                        dyeable: None,
                        use_player_texture: false,
                    },
                ],
            )]),
        }],
    );
    let resolved = catalog.resolve(&owned_entries).unwrap();
    let atlas = atlas_for(&[
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]);
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 93_001,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let profile_cape = EntityDynamicPlayerTexture {
        handle: 93_002,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let trim_texture = EntityDynamicTexture { handle: 93_003 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicPlayerTextureImage {
            handle: profile_elytra.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x11; 64 * 32 * 4],
        },
        DynamicPlayerTextureImage {
            handle: profile_cape.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x22; 64 * 32 * 4],
        },
        DynamicPlayerTextureImage {
            handle: trim_texture.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x33; 64 * 32 * 4],
        },
    ])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        930,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(EntityEquipmentAssetId::new(generation, 0));
        s.chest_wings_foil = true;
        s.chest_wings_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
        s.kind.player_mut().cape_texture = Some(profile_cape);
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
    });
    let render = |entries: &[EntityModelOwnedTextureAtlasEntry]| {
        entity_model_textured_meshes_with_equipment_for_camera(
            &[instance],
            &atlas,
            entries,
            &resolved,
            None,
            Some(&dynamic_atlas),
            None,
            None,
        )
    };

    let meshes = render(&owned_entries);

    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert_eq!(meshes.armor_cutout.vertices.len(), 48);
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 48);
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 48);
    for (mesh, handle) in [
        (
            &meshes.dynamic_player_texture_armor_cutout,
            profile_elytra.handle,
        ),
        (
            &meshes.dynamic_player_texture_armor_trim,
            trim_texture.handle,
        ),
    ] {
        let uv = dynamic_atlas
            .entries
            .iter()
            .find(|entry| entry.handle == handle)
            .unwrap()
            .uv;
        assert!(mesh.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
    }

    let missing_overlay = render(&owned_entries[..1]);

    assert!(missing_overlay.armor_cutout.vertices.is_empty());
    assert_eq!(
        missing_overlay
            .dynamic_player_texture_armor_cutout
            .vertices
            .len(),
        48
    );
}
