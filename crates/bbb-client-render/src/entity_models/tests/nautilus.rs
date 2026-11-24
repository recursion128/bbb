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
fn nautilus_textured_render_matches_vanilla_renderer() {
    let adult_pass = nautilus_textured_layer_passes(false)[0];
    assert_eq!(adult_pass.texture, NAUTILUS_TEXTURE_REF);
    assert_eq!(
        adult_pass.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult_pass.visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult_pass.order, 0);

    let baby_pass = nautilus_textured_layer_passes(true)[0];
    assert_eq!(baby_pass.texture, NAUTILUS_BABY_TEXTURE_REF);
    assert_eq!(baby_pass.order, 0);
    assert!(entity_model_texture_refs().contains(&NAUTILUS_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&NAUTILUS_BABY_TEXTURE_REF));
    assert_eq!(
        NAUTILUS_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/nautilus_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert_eq!(
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/nautilus_body/iron.png",
            size: [128, 128],
        }
    );
    assert!(entity_model_texture_refs().contains(&NAUTILUS_SADDLE_TEXTURE_REF));
    for texture in [
        NAUTILUS_BODY_COPPER_TEXTURE_REF,
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        NAUTILUS_BODY_GOLD_TEXTURE_REF,
        NAUTILUS_BODY_DIAMOND_TEXTURE_REF,
        NAUTILUS_BODY_NETHERITE_TEXTURE_REF,
    ] {
        assert!(entity_model_texture_refs().contains(&texture));
    }

    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF, NAUTILUS_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let instance = EntityModelInstance::nautilus(900, [0.0, 64.0, 0.0], 0.0, baby).with(|s| {
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert_nautilus_folded_meshes_are_cutout_only(&meshes);
        assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
        let mesh = &meshes.cutout;
        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(mesh.vertices.iter().all(|vertex| vertex.light
            == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()));
    }
}

#[test]
fn nautilus_saddle_layer_renders_for_adult_living_and_zombie_only() {
    let images: Vec<EntityModelTextureImage> = [
        NAUTILUS_TEXTURE_REF,
        NAUTILUS_BABY_TEXTURE_REF,
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
        NAUTILUS_SADDLE_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == NAUTILUS_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;

    let adult = EntityModelInstance::nautilus(920, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with(|s| {
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&bare_meshes);
    assert_nautilus_folded_meshes_are_cutout_only(&saddled_meshes);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    let saddle_mesh = &saddled_meshes.armor_cutout;

    // Saddle layer: one inflated shell cube plus the adult body/mouth subtree = 6 cubes.
    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddle_mesh.cutout_faces, 36);
    assert_eq!(saddle_mesh.vertices.len(), 144);
    assert_eq!(
        saddle_mesh.vertices[0].light,
        adult.render_state.shader_light()
    );
    assert_eq!(saddle_mesh.vertices[0].overlay, [0.0, 10.0]);
    let first_saddle_vertex = saddle_mesh.vertices[0].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby = EntityModelInstance::nautilus(921, [0.0, 64.0, 0.0], 0.0, true);
    let baby_saddled_instance = baby.with(|s| {
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let baby_bare = entity_model_textured_meshes(&[baby], &atlas);
    let baby_saddled = entity_model_textured_meshes(&[baby_saddled_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&baby_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&baby_saddled);
    assert_eq!(
        baby_saddled.cutout.vertices.len(),
        baby_bare.cutout.vertices.len(),
        "vanilla supplies no baby nautilus saddle model"
    );
    assert!(baby_saddled.armor_cutout.vertices.is_empty());

    let zombie = EntityModelInstance::new(
        922,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let zombie_saddled_instance = zombie.with(|s| {
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let zombie_bare = entity_model_textured_meshes(&[zombie], &atlas);
    let zombie_saddled = entity_model_textured_meshes(&[zombie_saddled_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&zombie_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&zombie_saddled);
    assert_eq!(zombie_saddled.armor_cutout.cutout_faces, 36);
    assert_eq!(zombie_saddled.armor_cutout.vertices.len(), 144);
    assert_eq!(
        zombie_saddled.cutout.vertices.len(),
        zombie_bare.cutout.vertices.len()
    );

    let warm = EntityModelInstance::new(
        923,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_saddled_instance = warm.with(|s| {
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let warm_bare = entity_model_textured_meshes(&[warm], &atlas);
    let warm_saddled = entity_model_textured_meshes(&[warm_saddled_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&warm_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&warm_saddled);
    assert_eq!(warm_saddled.armor_cutout.cutout_faces, 36);
    assert_eq!(warm_saddled.armor_cutout.vertices.len(), 144);
    assert_eq!(
        warm_saddled.cutout.vertices.len(),
        warm_bare.cutout.vertices.len()
    );
}

#[test]
fn nautilus_saddle_missing_saddle_texture_texture_preserves_other_geometry() {
    // Vanilla `NautilusRenderer` records `SimpleEquipmentLayer(NAUTILUS_SADDLE)` after the base body
    // at the default collector order, forcing `OverlayTexture.NO_OVERLAY`.
    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::nautilus(924, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(
        meshes.cutout.vertices.len(),
        192,
        "missing nautilus_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));

    assert_nautilus_folded_meshes_are_cutout_only(&meshes);
}

#[test]
fn nautilus_body_armor_layer_renders_for_adult_living_and_zombie_only() {
    let images: Vec<EntityModelTextureImage> = [
        NAUTILUS_TEXTURE_REF,
        NAUTILUS_BABY_TEXTURE_REF,
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
        NAUTILUS_SADDLE_TEXTURE_REF,
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        NAUTILUS_BODY_NETHERITE_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 35) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let iron_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == NAUTILUS_BODY_IRON_TEXTURE_REF)
        .unwrap()
        .uv;

    let adult = EntityModelInstance::nautilus(930, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let armored_instance = adult.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id())
    });
    let armored_meshes = entity_model_textured_meshes(&[armored_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&bare_meshes);
    assert_nautilus_folded_meshes_are_cutout_only(&armored_meshes);
    let bare = &bare_meshes.cutout;
    let armored = &armored_meshes.cutout;
    let armor_mesh = &armored_meshes.armor_cutout;

    // Armor layer: adult shell/body/mouth subtree = 8 cubes.
    assert_eq!(armored.cutout_faces, bare.cutout_faces);
    assert_eq!(armored.vertices.len(), bare.vertices.len());
    assert_eq!(armor_mesh.cutout_faces, 48);
    assert_eq!(armor_mesh.vertices.len(), 192);
    assert_eq!(
        armor_mesh.vertices[0].light,
        adult.render_state.shader_light()
    );
    assert_eq!(armor_mesh.vertices[0].overlay, [0.0, 10.0]);
    let first_armor_vertex = armor_mesh.vertices[0].uv;
    assert!(first_armor_vertex[0] >= iron_uv.min[0]);
    assert!(first_armor_vertex[0] <= iron_uv.max[0]);
    assert!(first_armor_vertex[1] >= iron_uv.min[1]);
    assert!(first_armor_vertex[1] <= iron_uv.max[1]);

    let armored_saddled_instance = adult.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let armored_saddled_meshes = entity_model_textured_meshes(&[armored_saddled_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&armored_saddled_meshes);

    assert_eq!(
        armored_saddled_meshes.armor_cutout.vertices.len() - armor_mesh.vertices.len(),
        144
    );
    assert!(armored_saddled_meshes
        .cutout
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == armored_saddled_instance.render_state.shader_light()
                && vertex.overlay == armored_saddled_instance.render_state.overlay_coords()
        ));
    assert!(
        armored_saddled_meshes.armor_cutout.vertices[..armor_mesh.vertices.len()]
            .iter()
            .all(
                |vertex| vertex.light == armored_saddled_instance.render_state.shader_light()
                    && vertex.overlay == [0.0, 10.0]
            )
    );
    assert!(
        armored_saddled_meshes.armor_cutout.vertices[armor_mesh.vertices.len()..]
            .iter()
            .all(
                |vertex| vertex.light == armored_saddled_instance.render_state.shader_light()
                    && vertex.overlay == [0.0, 10.0]
            )
    );

    let invalid_material_instance = adult.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_equipment_asset_id(u32::MAX))
    });
    let invalid_material = entity_model_textured_meshes(&[invalid_material_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&invalid_material);
    assert_eq!(
        invalid_material.cutout.vertices.len(),
        bare.vertices.len(),
        "vanilla 26.1 has no chainmail nautilus_body equipment texture"
    );
    assert!(invalid_material.armor_cutout.vertices.is_empty());

    let baby = EntityModelInstance::nautilus(931, [0.0, 64.0, 0.0], 0.0, true);
    let baby_armored_instance = baby.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id())
    });
    let baby_bare = entity_model_textured_meshes(&[baby], &atlas);
    let baby_armored = entity_model_textured_meshes(&[baby_armored_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&baby_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&baby_armored);
    assert_eq!(
        baby_armored.cutout.vertices.len(),
        baby_bare.cutout.vertices.len(),
        "vanilla supplies no baby nautilus armor model"
    );
    assert!(baby_armored.armor_cutout.vertices.is_empty());

    let zombie = EntityModelInstance::new(
        932,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let zombie_armored_instance = zombie.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_netherite_equipment_asset_id())
    });
    let zombie_bare = entity_model_textured_meshes(&[zombie], &atlas);
    let zombie_armored = entity_model_textured_meshes(&[zombie_armored_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&zombie_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&zombie_armored);
    assert_eq!(zombie_armored.armor_cutout.cutout_faces, 48);
    assert_eq!(zombie_armored.armor_cutout.vertices.len(), 192);
    assert_eq!(
        zombie_armored.cutout.vertices.len(),
        zombie_bare.cutout.vertices.len()
    );

    let warm = EntityModelInstance::new(
        933,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_armored_instance = warm.with(|s| {
        s.kind.nautilus_mut().body_item_present = true;
        s.kind.nautilus_mut().body_equipment_asset = Some(test_netherite_equipment_asset_id());
    });
    let warm_bare = entity_model_textured_meshes(&[warm], &atlas);
    let warm_armored = entity_model_textured_meshes(&[warm_armored_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&warm_bare);
    assert_nautilus_folded_meshes_are_cutout_only(&warm_armored);
    assert_eq!(
        warm_bare.cutout.cutout_faces - zombie_bare.cutout.cutout_faces,
        48,
        "warm zombie nautilus normally adds the coral cluster"
    );
    assert_eq!(
        warm_armored.cutout.cutout_faces, zombie_armored.cutout.cutout_faces,
        "ZombieNautilusCoralModel hides corals while body armor is present"
    );
    assert_eq!(
        warm_armored.cutout.vertices.len(),
        zombie_armored.cutout.vertices.len()
    );
    assert_eq!(
        warm_armored.armor_cutout.vertices.len(),
        zombie_armored.armor_cutout.vertices.len()
    );
}

#[test]
fn zombie_nautilus_corals_follow_body_item_presence_not_asset_classification() {
    let texture = ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF;
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    let (atlas, _) =
        build_entity_model_texture_atlas(&[EntityModelTextureImage::new(texture, vec![0x7f; len])])
            .unwrap();
    let warm = EntityModelInstance::new(
        9330,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let classified_only = warm.with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id())
    });
    let nonempty_unclassified = warm.with(|s| s.kind.nautilus_mut().body_item_present = true);
    let bare = entity_model_textured_meshes(&[warm], &atlas);
    let classified_only = entity_model_textured_meshes(&[classified_only], &atlas);
    let nonempty_unclassified = entity_model_textured_meshes(&[nonempty_unclassified], &atlas);

    assert_eq!(
        classified_only.cutout.vertices.len(),
        bare.cutout.vertices.len()
    );
    assert_eq!(
        bare.cutout.vertices.len() - nonempty_unclassified.cutout.vertices.len(),
        48 * 4,
        "every non-empty BODY stack hides the 48-face coral subtree"
    );
}

#[test]
fn nautilus_body_and_saddle_trims_keep_layer_and_collector_order() {
    let images: Vec<EntityModelTextureImage> = [
        NAUTILUS_TEXTURE_REF,
        NAUTILUS_BABY_TEXTURE_REF,
        NAUTILUS_BODY_IRON_TEXTURE_REF,
        NAUTILUS_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 31) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let body_trim_texture = EntityDynamicTexture { handle: 91_301 };
    let saddle_trim_texture = EntityDynamicTexture { handle: 91_302 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicEntityTextureImage {
            handle: body_trim_texture.handle,
            size: [128, 128],
            standalone_sampling: None,
            rgba: vec![0xff; 128 * 128 * 4],
        },
        DynamicEntityTextureImage {
            handle: saddle_trim_texture.handle,
            size: [64, 64],
            standalone_sampling: None,
            rgba: vec![0xff; 64 * 64 * 4],
        },
    ])
    .unwrap()
    .0;
    let adult = EntityModelInstance::nautilus(934, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
        s.kind.nautilus_mut().body_armor_foil = true;
        s.kind.nautilus_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: body_trim_texture,
            decal: false,
        });
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        s.kind.nautilus_mut().saddle_foil = true;
        s.kind.nautilus_mut().saddle_trim = Some(EntityArmorTrim {
            texture: saddle_trim_texture,
            decal: false,
        });
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[adult],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.armor_entity_glint.vertices.len(), 192 + 144);
    assert_eq!(
        meshes.dynamic_player_texture_armor_trim.vertices.len(),
        192 + 144,
        "each trim reuses its posed adult nautilus equipment model"
    );

    let baby = EntityModelInstance::nautilus(935, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
        s.kind.nautilus_mut().body_armor_foil = true;
        s.kind.nautilus_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: body_trim_texture,
            decal: false,
        });
        s.kind.nautilus_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        s.kind.nautilus_mut().saddle_foil = true;
        s.kind.nautilus_mut().saddle_trim = Some(EntityArmorTrim {
            texture: saddle_trim_texture,
            decal: false,
        });
    });
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
fn nautilus_body_armor_missing_armor_texture_texture_preserves_other_geometry() {
    // Vanilla `NautilusRenderer` records `SimpleEquipmentLayer(NAUTILUS_BODY)` before the saddle
    // layer, at the default collector order, and forces `OverlayTexture.NO_OVERLAY`.
    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 35) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::nautilus(934, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.nautilus_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(
        meshes.cutout.vertices.len(),
        192,
        "missing nautilus_body/iron.png suppresses only folded armor geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));

    assert_nautilus_folded_meshes_are_cutout_only(&meshes);
}

#[test]
fn zombie_nautilus_uses_its_own_texture_over_the_shared_adult_body() {
    // Vanilla `ZombieNautilusRenderer` `NORMAL` variant: the shared adult `NautilusModel` body
    // (`ModelLayers.ZOMBIE_NAUTILUS` bakes to `NautilusModel.createBodyLayer()`) textured by
    // `zombie_nautilus.png` — same geometry as the living adult nautilus, only the texture differs.
    let passes = zombie_nautilus_textured_layer_passes(ZombieNautilusModelVariant::Temperate);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].texture, ZOMBIE_NAUTILUS_TEXTURE_REF);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert!(entity_model_texture_refs().contains(&ZOMBIE_NAUTILUS_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [NAUTILUS_TEXTURE_REF, ZOMBIE_NAUTILUS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 30) as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let living_instance = EntityModelInstance::nautilus(900, [0.0, 64.0, 0.0], 0.0, false);
    let zombie_instance = EntityModelInstance::new(
        901,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let living = entity_model_textured_meshes(&[living_instance], &atlas);
    let zombie = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&living);
    assert_nautilus_folded_meshes_are_cutout_only(&zombie);
    assert!(!zombie.cutout.vertices.is_empty());
    assert!(zombie
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // Same shared adult-nautilus geometry: identical vertex positions...
    assert_eq!(living.cutout.vertices.len(), zombie.cutout.vertices.len());
    assert!(living
        .cutout
        .vertices
        .iter()
        .zip(&zombie.cutout.vertices)
        .all(|(a, b)| a.position == b.position));
    // ...but the zombie samples a different atlas region (its own texture).
    assert!(living
        .cutout
        .vertices
        .iter()
        .zip(&zombie.cutout.vertices)
        .any(|(a, b)| a.uv != b.uv));
}

#[test]
fn zombie_nautilus_warm_variant_adds_the_coral_cluster() {
    // Vanilla `ZombieNautilusRenderer` `WARM` variant: the `ZombieNautilusCoralModel` — the adult body
    // PLUS the `corals` subtree — over `zombie_nautilus_coral.png`. The corals are eight textured-only
    // cross-plane cubes (yellow ×2, pink + pink_second, blue ×2, red ×2), so the WARM mesh carries
    // exactly 8×6 = 48 more faces than the plain (`Temperate`) zombie nautilus body.
    let coral_pass = zombie_nautilus_textured_layer_passes(ZombieNautilusModelVariant::Warm)[0];
    assert_eq!(coral_pass.texture, ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF);
    assert_eq!(coral_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(coral_pass.order, 0);
    assert!(entity_model_texture_refs().contains(&ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [
        ZOMBIE_NAUTILUS_TEXTURE_REF,
        ZOMBIE_NAUTILUS_CORAL_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![(index * 30) as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let plain_instance = EntityModelInstance::new(
        902,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let warm_instance = EntityModelInstance::new(
        903,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let plain = entity_model_textured_meshes(&[plain_instance], &atlas);
    let warm = entity_model_textured_meshes(&[warm_instance], &atlas);
    assert_nautilus_folded_meshes_are_cutout_only(&plain);
    assert_nautilus_folded_meshes_are_cutout_only(&warm);
    assert!(!warm.cutout.vertices.is_empty());
    assert!(warm
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // The WARM mesh is the plain body plus the eight coral cross-plane cubes (48 faces, 192 vertices).
    assert_eq!(warm.cutout.cutout_faces, plain.cutout.cutout_faces + 48);
    assert_eq!(
        warm.cutout.vertices.len(),
        plain.cutout.vertices.len() + 48 * 4
    );
}

fn assert_nautilus_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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
