use super::*;
use crate::DynamicEntityTextureImage;
use std::collections::BTreeMap;

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

// Build an atlas covering the zombie base texture plus the iron equipment-asset textures and armor
// glint texture, enough to render an iron-clad zombie.
fn iron_armor_atlas() -> EntityModelTextureAtlasLayout {
    let mut refs: Vec<EntityModelTextureRef> =
        [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF].to_vec();
    refs.push(ARMOR_IRON_HUMANOID_TEXTURE_REF);
    refs.push(ARMOR_IRON_LEGGINGS_TEXTURE_REF);
    refs.push(ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
    refs.push(ENCHANTED_GLINT_ARMOR_TEXTURE_REF);
    build_atlas_for_refs(&refs)
}

fn armor_stand_iron_armor_atlas() -> EntityModelTextureAtlasLayout {
    let mut refs: Vec<EntityModelTextureRef> = [ARMOR_STAND_TEXTURE_REF].to_vec();
    refs.push(ARMOR_IRON_HUMANOID_TEXTURE_REF);
    refs.push(ARMOR_IRON_LEGGINGS_TEXTURE_REF);
    refs.push(ENCHANTED_GLINT_ARMOR_TEXTURE_REF);
    build_atlas_for_refs(&refs)
}

fn build_atlas_for_refs(refs: &[EntityModelTextureRef]) -> EntityModelTextureAtlasLayout {
    let images: Vec<EntityModelTextureImage> = refs
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn owned_equipment_entry(
    texture: EntityModelOwnedTextureRef,
    min: [f32; 2],
    max: [f32; 2],
) -> EntityModelOwnedTextureAtlasEntry {
    EntityModelOwnedTextureAtlasEntry {
        texture,
        uv: EntityModelUvRect { min, max },
    }
}

#[test]
fn armor_slot_part_subsets_match_vanilla_retain_exact_parts() {
    // Vanilla `HumanoidModel.ADULT_ARMOR_PARTS_PER_SLOT`.
    assert_eq!(HumanoidArmorSlot::Head.part_names(), &["head"]);
    assert_eq!(
        HumanoidArmorSlot::Chest.part_names(),
        &["body", "right_arm", "left_arm"]
    );
    assert_eq!(
        HumanoidArmorSlot::Legs.part_names(),
        &["body", "right_leg", "left_leg"]
    );
    assert_eq!(
        HumanoidArmorSlot::Feet.part_names(),
        &["right_leg", "left_leg"]
    );
    // Vanilla `HumanoidModel.BABY_ARMOR_PARTS_PER_SLOT` keeps the waist static and nests baby feet
    // under empty leg parents, so only the humanoid direct children copy animated pose data.
    assert_eq!(HumanoidArmorSlot::Head.baby_pose_part_names(), &["head"]);
    assert_eq!(
        HumanoidArmorSlot::Chest.baby_pose_part_names(),
        &["body", "right_arm", "left_arm"]
    );
    assert_eq!(
        HumanoidArmorSlot::Legs.baby_pose_part_names(),
        &["right_leg", "left_leg"]
    );
    assert_eq!(
        HumanoidArmorSlot::Feet.baby_pose_part_names(),
        &["right_leg", "left_leg"]
    );
}

#[test]
fn leather_armor_tints_with_default_undyed_color_others_white() {
    // Vanilla `EquipmentLayerRenderer.getColorForLayer`: leather (the only dyeable humanoid material)
    // tints by `DyedItemColor.LEATHER_COLOR` (0xA06540) when undyed; non-dyeable materials render white.
    let leather = [
        0xA0 as f32 / 255.0,
        0x65 as f32 / 255.0,
        0x40 as f32 / 255.0,
        1.0,
    ];
    // The rendered leather chestplate carries the brown tint; iron stays white.
    let mut refs: Vec<EntityModelTextureRef> =
        [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF].to_vec();
    refs.push(ARMOR_LEATHER_HUMANOID_TEXTURE_REF);
    let images: Vec<EntityModelTextureImage> = refs
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let armored = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(76, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.chest_equipment_asset = Some(test_leather_equipment_asset_id())),
        ],
        &atlas,
    );
    assert!(armored
        .armor_cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint == leather));
}

#[test]
fn custom_dyed_leather_tints_by_dye_color_and_non_leather_ignores_it() {
    // Vanilla `DyedItemColor.getOrDefault` → `getColorForLayer`: a custom-dyed leather piece tints by
    // its `dyed_color` component (here 0x3F6CDA), forced opaque. The low 24 bits become the RGB tint.
    let dye = 0x003F_6CDA;
    let dyed = [
        0x3F as f32 / 255.0,
        0x6C as f32 / 255.0,
        0xDA as f32 / 255.0,
        1.0,
    ];
    // The rendered dyed leather chestplate carries the custom tint, not the default brown.
    let mut refs: Vec<EntityModelTextureRef> =
        [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF].to_vec();
    refs.push(ARMOR_LEATHER_HUMANOID_TEXTURE_REF);
    let images: Vec<EntityModelTextureImage> = refs
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let armored = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(77, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
                s.chest_equipment_asset = Some(test_leather_equipment_asset_id());
                s.chest_armor_dye = Some(dye);
            }),
        ],
        &atlas,
    );
    assert!(armored
        .armor_cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint == dyed));
    assert!(!armored
        .armor_cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint
            == [
                0xA0 as f32 / 255.0,
                0x65 as f32 / 255.0,
                0x40 as f32 / 255.0,
                1.0
            ]));
}

#[test]
fn armored_zombie_emits_inflated_armor_pieces() {
    let atlas = iron_armor_atlas();

    // A bare zombie wears no armor: its cutout is just the body.
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(
            70,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    let bare_cutout = bare.cutout.vertices.len();

    // A fully iron-clad adult zombie adds the four armor pieces into the cutout pass: helmet
    // (head + hat = 2 cubes), chestplate (body + 2 arms = 3), leggings (body + 2 legs = 3), boots
    // (2 legs = 2) — 10 cubes → 240 vertices.
    let armored_instance =
        EntityModelInstance::zombie(71, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.light_coords = (4_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
            s.head_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
            s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        });
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);
    assert_eq!(
        armored.armor_cutout.vertices.len(),
        240,
        "four iron armor pieces add 10 cubes of armorCutoutNoCull geometry"
    );
    assert_eq!(armored.cutout.vertices.len(), bare_cutout);

    assert!(armored.cutout.vertices.iter().all(|vertex| {
        vertex.light == armored_instance.render_state.shader_light()
            && vertex.overlay == armored_instance.render_state.overlay_coords()
    }));

    // Armor geometry is appended in the same order as the explicit vanilla armor submissions:
    // chest, legs, feet, head.
    let mut vertex_start = 0;
    for vertex_count in [72, 72, 48, 48] {
        let vertex_end = vertex_start + vertex_count;
        assert!(armored.armor_cutout.vertices[vertex_start..vertex_end]
            .iter()
            .all(
                |vertex| vertex.light == armored_instance.render_state.shader_light()
                    && vertex.overlay == [0.0, 10.0]
            ));
        vertex_start = vertex_end;
    }
    assert_eq!(vertex_start, armored.armor_cutout.vertices.len());

    // The armor is inflated (`CubeDeformation 1.0` / `0.5`), so it floats just outside the body: the
    // armored cutout reaches wider in X than the bare body.
    let bare_x_max = bare
        .cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    let armored_x_max = armored
        .armor_cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    assert!(
        armored_x_max > bare_x_max,
        "inflated armor extends beyond the body ({armored_x_max} vs {bare_x_max})"
    );
}

#[test]
fn foiled_humanoid_armor_emits_vanilla_armor_entity_glint() {
    let atlas = iron_armor_atlas();
    let armored = EntityModelInstance::zombie(171, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_armor_foil = true;
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[armored], &atlas);

    assert!(!meshes.armor_entity_glint.vertices.is_empty());
    assert!(meshes.armor_entity_glint.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == armored.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
    assert!(meshes.entity_glint.vertices.is_empty());
}

#[test]
fn adult_humanoid_armor_trim_uses_runtime_atlas_after_foil() {
    let atlas = iron_armor_atlas();
    let trim_texture = EntityDynamicTexture { handle: 91_001 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let armored = EntityModelInstance::zombie(91_001, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_armor_foil = true;
        s.chest_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[armored],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(
        meshes.dynamic_player_texture_armor_trim.vertices.len(),
        72,
        "the trim reuses the posed chestplate body and arm cubes"
    );
    assert!(meshes
        .dynamic_player_texture_armor_trim_decal
        .vertices
        .is_empty());
}

#[test]
fn baby_humanoid_armor_suppresses_trim_layer() {
    let atlas = iron_armor_atlas();
    let trim_texture = EntityDynamicTexture { handle: 91_002 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let armored = EntityModelInstance::zombie(91_002, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[armored],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes.dynamic_player_texture_armor_trim.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_armor_trim_decal
        .vertices
        .is_empty());
}

#[test]
fn small_armor_stand_keeps_decal_armor_trim_on_adult_texture_layer() {
    let atlas = armor_stand_iron_armor_atlas();
    let trim_texture = EntityDynamicTexture { handle: 91_003 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let armored = EntityModelInstance::armor_stand(
        91_003,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: true,
        });
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[armored],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(
        meshes
            .dynamic_player_texture_armor_trim_decal
            .vertices
            .len(),
        72
    );
    assert!(meshes.dynamic_player_texture_armor_trim.vertices.is_empty());
}

#[test]
fn hidden_foiled_humanoid_armor_keeps_glint_without_base_geometry() {
    let atlas = iron_armor_atlas();
    let hidden = EntityModelInstance::zombie(172, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| {
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_armor_foil = true;
        })
        .with_invisible(true)
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });

    let meshes = entity_model_textured_meshes(&[hidden], &atlas);

    assert!(!meshes.armor_entity_glint.vertices.is_empty());
    assert!(meshes.armor_entity_glint.vertices.iter().all(|vertex| {
        vertex.tint == [1.0; 4]
            && vertex.light == hidden.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
    assert!(meshes.entity_glint.vertices.is_empty());
}

#[test]
fn hidden_zombie_keeps_humanoid_armor_without_base_geometry() {
    // Vanilla `LivingEntityRenderer.submit` still invokes `HumanoidArmorLayer` when the invisible
    // base render type is null; `HumanoidArmorLayer` itself has no `state.isInvisible` gate.
    let atlas = iron_armor_atlas();
    let hidden = EntityModelInstance::zombie(72, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| {
            s.head_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
            s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
        .with_invisible(true);

    let meshes = entity_model_textured_meshes(&[hidden], &atlas);

    assert_iron_armor_vertices(&meshes, hidden);
    assert_eq!(
        meshes.armor_cutout.vertices.len(),
        240,
        "hidden base has no geometry, but vanilla still runs HumanoidArmorLayer"
    );
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn hidden_player_keeps_humanoid_armor_without_base_geometry() {
    // Player renderers use the same ungated `HumanoidArmorLayer`; the hidden base must not suppress
    // player armor geometry.
    let atlas = iron_armor_atlas();
    let hidden = EntityModelInstance::player_with_skin(
        74,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true);

    let meshes = entity_model_textured_meshes(&[hidden], &atlas);

    assert_iron_armor_vertices(&meshes, hidden);
    assert_eq!(meshes.armor_cutout.vertices.len(), 240);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn invisible_glowing_zombie_keeps_humanoid_armor_geometry() {
    let atlas = iron_armor_atlas();
    let hidden_glowing = EntityModelInstance::zombie(73, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| {
            s.head_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
            s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
        .with_invisible(true)
        .with_outline_color(0xff11_44cc);

    let meshes = entity_model_textured_meshes(&[hidden_glowing], &atlas);

    assert_iron_armor_vertices(&meshes, hidden_glowing);
    assert_eq!(
        meshes.armor_cutout.vertices.len(),
        240,
        "outline-only base folds into the outline bucket, while armorCutoutNoCull equipment uses the armor bucket"
    );
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(!meshes.outline.vertices.is_empty());
    assert!(meshes
        .outline
        .vertices
        .iter()
        .all(|vertex| vertex.tint == super::super::geometry::argb_to_tint(0xff11_44cc)));
}

#[test]
fn full_armor_stand_armor_has_vanilla_vertex_attributes() {
    let atlas = armor_stand_iron_armor_atlas();
    let armored_instance = EntityModelInstance::armor_stand(
        472,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes(&[armored_instance], &atlas);

    assert_iron_armor_vertices(&meshes, armored_instance);
    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert_eq!(meshes.armor_cutout.vertices.len(), 240);
}

#[test]
fn invisible_glowing_armor_stand_keeps_armor_geometry() {
    let atlas = armor_stand_iron_armor_atlas();
    let hidden_glowing = EntityModelInstance::armor_stand(
        473,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true)
    .with_outline_color(0xff66_aa33);

    let meshes = entity_model_textured_meshes(&[hidden_glowing], &atlas);

    assert_iron_armor_vertices(&meshes, hidden_glowing);
    assert_eq!(
        meshes.armor_cutout.vertices.len(),
        240,
        "outline-only base folds into the outline bucket, while armorCutoutNoCull equipment uses the armor bucket"
    );
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(!meshes.outline.vertices.is_empty());
    assert!(meshes
        .outline
        .vertices
        .iter()
        .all(|vertex| vertex.tint == super::super::geometry::argb_to_tint(0xff66_aa33)));
}

#[test]
fn marker_invisible_glowing_armor_stand_keeps_armor_without_base_geometry() {
    let atlas = armor_stand_iron_armor_atlas();
    let marker_hidden_glowing = EntityModelInstance::armor_stand_with_marker(
        474,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with_invisible(true)
    .with_outline_color(0xff22_88cc);

    let meshes = entity_model_textured_meshes(&[marker_hidden_glowing], &atlas);

    assert_iron_armor_vertices(&meshes, marker_hidden_glowing);
    assert_eq!(
        meshes.armor_cutout.vertices.len(),
        240,
        "marker hidden base has no geometry, but vanilla still runs HumanoidArmorLayer"
    );
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(!meshes.outline.vertices.is_empty());
    assert!(meshes
        .outline
        .vertices
        .iter()
        .all(|vertex| vertex.tint == super::super::geometry::argb_to_tint(0xff22_88cc)));
}

#[test]
fn small_armor_stand_armor_uses_small_model_scale_with_adult_textures() {
    let atlas = armor_stand_iron_armor_atlas();
    let full = EntityModelInstance::armor_stand(
        475,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });
    let small = EntityModelInstance::armor_stand(
        476,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let full_meshes = entity_model_textured_meshes(&[full], &atlas);
    let small_meshes = entity_model_textured_meshes(&[small], &atlas);
    assert_iron_armor_vertices(&full_meshes, full);
    assert_iron_armor_vertices(&small_meshes, small);

    let full_positions = armor_stand_armor_vertex_positions(&full_meshes, &atlas);
    let small_positions = armor_stand_armor_vertex_positions(&small_meshes, &atlas);
    assert_eq!(full_positions.len(), 240);
    assert_eq!(small_positions.len(), 240);
    assert!(
        y_extent(&small_positions) < y_extent(&full_positions) * 0.8,
        "ARMOR_STAND_SMALL_ARMOR applies HumanoidModel.BABY_TRANSFORMER while keeping adult equipment textures"
    );
}

fn armor_stand_armor_vertex_positions(
    meshes: &EntityModelTexturedMeshes,
    atlas: &EntityModelTextureAtlasLayout,
) -> Vec<[f32; 3]> {
    let armor_entries: Vec<_> = atlas
        .entries
        .iter()
        .filter(|entry| {
            entry.texture == ARMOR_IRON_HUMANOID_TEXTURE_REF
                || entry.texture == ARMOR_IRON_LEGGINGS_TEXTURE_REF
        })
        .collect();
    meshes
        .armor_cutout
        .vertices
        .iter()
        .filter(|vertex| {
            armor_entries.iter().any(|entry| {
                vertex.uv[0] >= entry.uv.min[0]
                    && vertex.uv[0] <= entry.uv.max[0]
                    && vertex.uv[1] >= entry.uv.min[1]
                    && vertex.uv[1] <= entry.uv.max[1]
            })
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

#[test]
fn humanoid_armor_missing_texture_atlas_entries_suppress_geometry() {
    let len = usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let (atlas, _) = build_entity_model_texture_atlas(&[EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0x21; len],
    )])
    .unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == ARMOR_IRON_HUMANOID_TEXTURE_REF
            || entry.texture == ARMOR_IRON_LEGGINGS_TEXTURE_REF));

    let bare_instance = EntityModelInstance::zombie(95, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let armored_instance = bare_instance.with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let bare = entity_model_textured_meshes(&[bare_instance], &atlas);
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);

    assert_eq!(
        armored.cutout.vertices.len(),
        bare.cutout.vertices.len(),
        "missing armor atlas entries must not fold stale armor geometry"
    );
    assert!(armored.armor_cutout.vertices.is_empty());

    assert!(armored.cutout.vertices.iter().all(|vertex| {
        vertex.light == armored_instance.render_state.shader_light()
            && vertex.overlay == armored_instance.render_state.overlay_coords()
    }));
}

#[test]
fn armored_giant_uses_vanilla_giant_armor_layer() {
    let atlas = iron_armor_atlas();
    let base = EntityModelInstance::giant(94, [0.0, 64.0, 0.0], 0.0);
    let armored = base.with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    let armored_meshes = entity_model_textured_meshes(&[armored], &atlas);

    assert_eq!(
        armored_meshes.armor_cutout.vertices.len(),
        240,
        "GiantMobRenderer's HumanoidArmorLayer adds the standard 10 armor cubes"
    );
    assert_eq!(
        armored_meshes.cutout.vertices.len(),
        bare_meshes.cutout.vertices.len()
    );
}

#[test]
fn single_armor_slot_emits_only_its_pieces() {
    let atlas = iron_armor_atlas();
    // Just a helmet: head + hat = 2 cubes → 48 vertices.
    let helmet = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(73, [0.0, 64.0, 0.0], 0.0, false)
                .with(|s| s.head_equipment_asset = Some(test_iron_equipment_asset_id())),
        ],
        &atlas,
    );
    assert_eq!(helmet.armor_cutout.vertices.len(), 48);
}

#[test]
fn standard_humanoid_wearers_all_drape_armor() {
    let atlas = iron_armor_atlas();
    let full_iron = |instance: EntityModelInstance| {
        instance.with(|s| {
            s.head_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
            s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        })
    };
    // Each standard-`HumanoidModel` armor wearer (zombie family, skeleton family, player) drapes the
    // same 10-cube / 240-vertex armor set; the armor delta is independent of the base body texture.
    let wearers = [
        EntityModelInstance::skeleton(80, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::zombie_variant(
            81,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        ),
        EntityModelInstance::zombie_variant(
            82,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Husk,
            false,
        ),
        EntityModelInstance::player(83, [0.0, 64.0, 0.0], 0.0, false),
    ];
    for instance in wearers {
        let bare = entity_model_textured_meshes(&[instance], &atlas);
        let armored = entity_model_textured_meshes(&[full_iron(instance)], &atlas);
        assert_eq!(
            armored.armor_cutout.vertices.len(),
            240,
            "{:?} drapes the four armor pieces",
            instance.kind
        );
        assert_eq!(armored.cutout.vertices.len(), bare.cutout.vertices.len());
    }

    // A baby husk uses the standard baby humanoid armor mesh: helmet (1 cube), chestplate (body +
    // 2 arms), leggings (waist + 2 legs), boots (2 feet) — 9 cubes -> 216 vertices.
    let baby_husk = EntityModelInstance::zombie_variant(
        84,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        true,
    );
    let bare = entity_model_textured_meshes(&[baby_husk], &atlas);
    let armored = entity_model_textured_meshes(&[full_iron(baby_husk)], &atlas);
    assert_eq!(armored.armor_cutout.vertices.len(), 216);
    assert_eq!(armored.cutout.vertices.len(), bare.cutout.vertices.len());
}

#[test]
fn skeleton_family_armor_emits_vanilla_vertex_attributes() {
    let atlas = iron_armor_atlas();
    let cases = [
        EntityModelInstance::skeleton(86, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::skeleton_variant(
            87,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Stray,
        ),
        EntityModelInstance::skeleton_variant(
            88,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Parched,
        ),
        EntityModelInstance::skeleton_variant(
            89,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::WitherSkeleton,
        ),
        EntityModelInstance::skeleton_variant(
            90,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: false },
        ),
        EntityModelInstance::skeleton_variant(
            91,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Bogged { sheared: true },
        ),
    ];

    for instance in cases {
        let armored =
            instance.with(|s| s.chest_equipment_asset = Some(test_iron_equipment_asset_id()));
        let meshes = entity_model_textured_meshes(&[armored], &atlas);
        assert_iron_armor_vertices(&meshes, armored);
    }
}

#[test]
fn piglin_family_drapes_armor_at_wider_deformation() {
    let atlas = iron_armor_atlas();
    let full_iron = |instance: EntityModelInstance| {
        instance.with(|s| {
            s.head_equipment_asset = Some(test_iron_equipment_asset_id());
            s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
            s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
            s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
        })
    };
    // Every adult piglin-family wearer (piglin, piglin brute, zombified piglin) drapes the same
    // 10-cube / 240-vertex armor set (vanilla `AbstractPiglinModel.createArmorMeshSet`).
    for family in [
        PiglinModelFamily::Piglin,
        PiglinModelFamily::PiglinBrute,
        PiglinModelFamily::ZombifiedPiglin,
    ] {
        let bare = EntityModelInstance::piglin(90, [0.0, 64.0, 0.0], 0.0, family, false);
        let bare_meshes = entity_model_textured_meshes(&[bare], &atlas);
        let armored = entity_model_textured_meshes(&[full_iron(bare)], &atlas);
        assert_eq!(
            armored.armor_cutout.vertices.len(),
            240,
            "{family:?} drapes the four armor pieces"
        );
        assert_eq!(
            armored.cutout.vertices.len(),
            bare_meshes.cutout.vertices.len()
        );
    }

    // The piglin armor is grown by OUTER 1.02 (vanilla `LayerDefinitions` piglin armor) vs the standard
    // 1.0, so a rest-posed piglin's armor reaches very slightly wider in X than a same-posed zombie's
    // (0.02 model units through the shared root transform). Both are idle, so only the deformation
    // differs.
    let max_armor_x = |instance: EntityModelInstance| {
        entity_model_textured_meshes(&[full_iron(instance)], &atlas)
            .armor_cutout
            .vertices
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::MIN, f32::max)
    };
    let zombie_x = max_armor_x(EntityModelInstance::zombie(
        91,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    ));
    let piglin_x = max_armor_x(EntityModelInstance::piglin(
        92,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        false,
    ));
    assert!(
        piglin_x > zombie_x,
        "piglin armor (OUTER 1.02) extends wider than zombie armor (OUTER 1.0): {piglin_x} vs {zombie_x}"
    );

    // Baby piglin-family armor uses vanilla `AbstractPiglinModel.createBabyArmorMeshSet`: the same
    // 9-cube baby retained topology as standard baby armor, but with uniform 0.7 deformation and the
    // `(0.5, -0.5, 0)` arm offset.
    for (family, _base_texture) in [
        (PiglinModelFamily::Piglin, PIGLIN_BABY_TEXTURE_REF),
        (
            PiglinModelFamily::ZombifiedPiglin,
            ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
        ),
    ] {
        let baby = EntityModelInstance::piglin(93, [0.0, 64.0, 0.0], 0.0, family, true);
        let bare = entity_model_textured_meshes(&[baby], &atlas);
        let armored = entity_model_textured_meshes(&[full_iron(baby)], &atlas);
        assert_eq!(
            armored.armor_cutout.vertices.len(),
            216,
            "{family:?} baby armor retains nine baby cubes"
        );
        assert_eq!(armored.cutout.vertices.len(), bare.cutout.vertices.len());
    }

    let baby_zombie_x = max_armor_x(EntityModelInstance::zombie(95, [0.0, 64.0, 0.0], 0.0, true));
    let baby_piglin_x = max_armor_x(EntityModelInstance::piglin(
        96,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        true,
    ));
    assert!(
        baby_piglin_x > baby_zombie_x,
        "piglin baby armor's 0.7 deformation / arm offset extends wider than standard baby armor: {baby_piglin_x} vs {baby_zombie_x}"
    );
}

#[test]
fn baby_zombie_armor_uses_humanoid_baby_layer() {
    let atlas = iron_armor_atlas();
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(74, [0.0, 64.0, 0.0], 0.0, true)],
        &atlas,
    );
    let armored = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(75, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
                s.head_equipment_asset = Some(test_iron_equipment_asset_id());
                s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
                s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
                s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
            }),
        ],
        &atlas,
    );
    assert_eq!(
        armored.armor_cutout.vertices.len(),
        216,
        "baby humanoid armor keeps nine retained cubes"
    );
    assert_eq!(armored.cutout.vertices.len(), bare.cutout.vertices.len());
}

#[test]
fn baby_humanoid_armor_missing_texture_atlas_entry_suppresses_geometry() {
    // Vanilla `HumanoidArmorLayer` switches every non-armor-stand baby slot to
    // `HUMANOID_BABY`, so all four iron armor pieces read the same baby texture.
    let len =
        usize::try_from(ZOMBIE_BABY_TEXTURE_REF.size[0] * ZOMBIE_BABY_TEXTURE_REF.size[1] * 4)
            .unwrap();
    let (atlas, _) = build_entity_model_texture_atlas(&[EntityModelTextureImage::new(
        ZOMBIE_BABY_TEXTURE_REF,
        vec![0x37; len],
    )])
    .unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF));

    let bare_instance = EntityModelInstance::zombie(97, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let armored_instance = bare_instance.with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let bare = entity_model_textured_meshes(&[bare_instance], &atlas);
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);

    assert_eq!(
        armored.cutout.vertices.len(),
        bare.cutout.vertices.len(),
        "missing baby armor atlas entry must not fold stale armor geometry"
    );
    assert!(armored.armor_cutout.vertices.is_empty());

    assert!(armored.cutout.vertices.iter().all(|vertex| {
        vertex.light == armored_instance.render_state.shader_light()
            && vertex.overlay == armored_instance.render_state.overlay_coords()
    }));
}

#[test]
fn baby_zombie_villager_armor_uses_zombie_villager_baby_armor_set() {
    // Vanilla `ZombieVillagerRenderer` registers a dedicated
    // `ModelLayers.ZOMBIE_VILLAGER_BABY_ARMOR`, but `LayerDefinitions` builds it through inherited
    // `HumanoidModel.createBabyArmorMeshSet(..., PartPose.ZERO)`. The layer therefore uses the same
    // standard baby humanoid armor topology and `HUMANOID_BABY` equipment texture, posed from the
    // `BabyZombieVillagerModel` host.
    let textures = [
        ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
        ZOMBIE_VILLAGER_BABY_TYPE_TEXTURE_REFS[2],
        ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF,
    ];
    let images: Vec<EntityModelTextureImage> = textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::zombie_variant(
        76,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        true,
    )
    .with(|s| {
        s.kind.villager_mut().model_data =
            VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::None, 1)
    });
    let armored = base.with(|s| {
        s.head_equipment_asset = Some(test_iron_equipment_asset_id());
        s.chest_equipment_asset = Some(test_iron_equipment_asset_id());
        s.legs_equipment_asset = Some(test_iron_equipment_asset_id());
        s.feet_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    let armored_meshes = entity_model_textured_meshes(&[armored], &atlas);
    assert_eq!(
        armored_meshes.armor_cutout.vertices.len(),
        216,
        "baby zombie-villager armor keeps the nine-cube baby humanoid armor topology"
    );
    assert_eq!(
        armored_meshes.cutout.vertices.len(),
        bare_meshes.cutout.vertices.len()
    );
}

#[test]
fn equipment_asset_humanoid_layers_preserve_dye_foil_trim_and_declared_layers() {
    let generation = 7;
    let skipped_texture =
        EntityModelOwnedTextureRef::new("example:equipment/skipped.png", [64, 32]);
    let tinted_texture = EntityModelOwnedTextureRef::new("example:equipment/tinted.png", [64, 32]);
    let plain_texture = EntityModelOwnedTextureRef::new("example:equipment/plain.png", [64, 32]);
    let owned_entries = vec![
        owned_equipment_entry(skipped_texture.clone(), [0.0, 0.0], [0.25, 1.0]),
        owned_equipment_entry(tinted_texture.clone(), [0.25, 0.0], [0.5, 1.0]),
        owned_equipment_entry(plain_texture.clone(), [0.5, 0.0], [0.75, 1.0]),
    ];
    let catalog = EntityEquipmentAssetCatalog::new(
        generation,
        vec![EntityEquipmentAsset {
            layers: BTreeMap::from([(
                EntityEquipmentLayerType::Humanoid,
                vec![
                    EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(skipped_texture),
                        dyeable: Some(EntityEquipmentDyeable {
                            color_when_undyed: None,
                        }),
                        use_player_texture: false,
                    },
                    EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(tinted_texture),
                        dyeable: Some(EntityEquipmentDyeable {
                            color_when_undyed: Some(0x12_34_56),
                        }),
                        use_player_texture: false,
                    },
                    EntityEquipmentAssetLayer {
                        texture: EntityEquipmentAssetLayerTexture::Owned(plain_texture),
                        dyeable: None,
                        use_player_texture: false,
                    },
                ],
            )]),
        }],
    );
    let resolved = catalog.resolve(&owned_entries).unwrap();

    let mut atlas_refs = [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF].to_vec();
    atlas_refs.push(ENCHANTED_GLINT_ARMOR_TEXTURE_REF);
    let atlas = build_atlas_for_refs(&atlas_refs);
    let trim_texture = EntityDynamicTexture { handle: 92_001 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x77; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::zombie(920, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.chest_equipment_asset = Some(EntityEquipmentAssetId::new(generation, 0));
        s.chest_armor_foil = true;
        s.chest_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
    });
    let meshes = entity_model_textured_meshes_with_equipment_for_camera(
        &[instance],
        &atlas,
        &owned_entries,
        &resolved,
        None,
        Some(&dynamic_atlas),
        None,
        None,
    );

    assert_eq!(meshes.armor_cutout.vertices.len(), 144);
    let tint = [
        0x12 as f32 / 255.0,
        0x34 as f32 / 255.0,
        0x56 as f32 / 255.0,
        1.0,
    ];
    for (vertices, uv, expected_tint) in [
        (
            &meshes.armor_cutout.vertices[..72],
            owned_entries[1].uv,
            tint,
        ),
        (
            &meshes.armor_cutout.vertices[72..],
            owned_entries[2].uv,
            [1.0; 4],
        ),
    ] {
        assert!(vertices.iter().all(|vertex| vertex.tint == expected_tint
            && vertex.uv[0] >= uv.min[0]
            && vertex.uv[0] <= uv.max[0]
            && vertex.uv[1] >= uv.min[1]
            && vertex.uv[1] <= uv.max[1]));
    }
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 72);
    assert!(meshes
        .armor_entity_glint
        .vertices
        .iter()
        .all(|vertex| vertex.tint == tint));
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 72);
}

#[test]
fn equipment_asset_selects_leggings_and_baby_layers_and_suppresses_baby_trim() {
    let generation = 8;
    let humanoid = EntityModelOwnedTextureRef::new("example:equipment/humanoid.png", [64, 32]);
    let leggings = EntityModelOwnedTextureRef::new("example:equipment/leggings.png", [64, 32]);
    let baby = EntityModelOwnedTextureRef::new("example:equipment/baby.png", [64, 64]);
    let owned_entries = vec![
        owned_equipment_entry(humanoid.clone(), [0.0, 0.0], [0.25, 1.0]),
        owned_equipment_entry(leggings.clone(), [0.25, 0.0], [0.5, 1.0]),
        owned_equipment_entry(baby.clone(), [0.5, 0.0], [1.0, 1.0]),
    ];
    let layer = |texture| {
        vec![EntityEquipmentAssetLayer {
            texture: EntityEquipmentAssetLayerTexture::Owned(texture),
            dyeable: None,
            use_player_texture: false,
        }]
    };
    let catalog = EntityEquipmentAssetCatalog::new(
        generation,
        vec![EntityEquipmentAsset {
            layers: BTreeMap::from([
                (EntityEquipmentLayerType::Humanoid, layer(humanoid)),
                (EntityEquipmentLayerType::HumanoidLeggings, layer(leggings)),
                (EntityEquipmentLayerType::HumanoidBaby, layer(baby)),
            ]),
        }],
    );
    let resolved = catalog.resolve(&owned_entries).unwrap();
    let mut atlas_refs = [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF].to_vec();
    atlas_refs.extend_from_slice(&[ARMOR_STAND_TEXTURE_REF]);
    let atlas = build_atlas_for_refs(&atlas_refs);
    let trim_texture = EntityDynamicTexture { handle: 92_002 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0x55; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let asset_id = Some(EntityEquipmentAssetId::new(generation, 0));
    let render = |instance| {
        entity_model_textured_meshes_with_equipment_for_camera(
            &[instance],
            &atlas,
            &owned_entries,
            &resolved,
            None,
            Some(&dynamic_atlas),
            None,
            None,
        )
    };

    let chest = render(
        EntityModelInstance::zombie(921, [0.0, 64.0, 0.0], 0.0, false)
            .with(|s| s.chest_equipment_asset = asset_id),
    );

    let legs = render(
        EntityModelInstance::zombie(922, [0.0, 64.0, 0.0], 0.0, false)
            .with(|s| s.legs_equipment_asset = asset_id),
    );

    let baby = render(
        EntityModelInstance::zombie(923, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
            s.chest_equipment_asset = asset_id;
            s.chest_armor_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: true,
            });
        }),
    );

    assert!(baby
        .dynamic_player_texture_armor_trim_decal
        .vertices
        .is_empty());

    let small_armor_stand = render(
        EntityModelInstance::armor_stand(
            924,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            true,
            true,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )
        .with(|s| {
            s.chest_equipment_asset = asset_id;
            s.chest_armor_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: true,
            });
        }),
    );

    assert!(!small_armor_stand
        .dynamic_player_texture_armor_trim_decal
        .vertices
        .is_empty());
    for (mesh, index) in [(&chest, 0), (&legs, 1), (&baby, 2), (&small_armor_stand, 0)] {
        let uv = owned_entries[index].uv;
        assert!(!mesh.armor_cutout.vertices.is_empty());
        assert!(mesh.armor_cutout.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
    }
}

fn assert_iron_armor_vertices(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert!(!meshes.armor_cutout.vertices.is_empty());
    assert!(meshes.armor_cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0; 4]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
}
