use super::*;
use crate::entity_models::colored::GIANT_SCALE;

// Build an atlas covering the zombie base texture plus the iron equipment-asset textures (humanoid +
// leggings), enough to render an iron-clad zombie.
fn iron_armor_atlas() -> EntityModelTextureAtlasLayout {
    let mut refs: Vec<EntityModelTextureRef> = zombie_entity_texture_refs().to_vec();
    refs.push(ARMOR_IRON_HUMANOID_TEXTURE_REF);
    refs.push(ARMOR_IRON_LEGGINGS_TEXTURE_REF);
    refs.push(ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
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

#[test]
fn armor_slot_textures_match_vanilla_layer_types() {
    // Vanilla `HumanoidArmorLayer.usesInnerModel`: only the LEGS slot reads the `humanoid_leggings`
    // texture; the head / chest / feet slots read the `humanoid` texture.
    assert_eq!(
        armor_slot_texture_for_layer(EntityArmorMaterial::Iron, HumanoidArmorSlot::Head, false),
        Some(ARMOR_IRON_HUMANOID_TEXTURE_REF)
    );
    assert_eq!(
        armor_slot_texture_for_layer(EntityArmorMaterial::Iron, HumanoidArmorSlot::Chest, false),
        Some(ARMOR_IRON_HUMANOID_TEXTURE_REF)
    );
    assert_eq!(
        armor_slot_texture_for_layer(EntityArmorMaterial::Iron, HumanoidArmorSlot::Feet, false),
        Some(ARMOR_IRON_HUMANOID_TEXTURE_REF)
    );
    assert_eq!(
        armor_slot_texture_for_layer(EntityArmorMaterial::Iron, HumanoidArmorSlot::Legs, false),
        Some(ARMOR_IRON_LEGGINGS_TEXTURE_REF)
    );
    assert_eq!(
        armor_slot_texture_for_layer(EntityArmorMaterial::Diamond, HumanoidArmorSlot::Legs, false),
        Some(ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF)
    );
    // Vanilla `HumanoidArmorLayer` switches every non-armor-stand baby slot to
    // `EquipmentClientInfo.LayerType.HUMANOID_BABY`, so baby leggings do not read the adult
    // `humanoid_leggings` texture.
    for slot in [
        HumanoidArmorSlot::Head,
        HumanoidArmorSlot::Chest,
        HumanoidArmorSlot::Legs,
        HumanoidArmorSlot::Feet,
    ] {
        assert_eq!(
            armor_slot_texture_for_layer(EntityArmorMaterial::Iron, slot, true),
            Some(ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF)
        );
    }
    assert_eq!(
        armor_slot_texture_for_layer(
            EntityArmorMaterial::ArmadilloScute,
            HumanoidArmorSlot::Chest,
            false
        ),
        None
    );
    // Every equipment texture is stitched into the shared atlas.
    assert!(entity_model_texture_refs().contains(&ARMOR_IRON_HUMANOID_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF));
}

#[test]
fn humanoid_armor_layer_pass_records_vanilla_model_layer_metadata() {
    let chest = humanoid_armor_layer_pass(
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE,
        HumanoidArmorSlot::Chest,
        EntityArmorMaterial::Iron,
        ARMOR_IRON_HUMANOID_TEXTURE_REF,
        None,
        1,
    );
    assert_eq!(chest.kind, EntityModelLayerKind::HumanoidArmor);
    assert_eq!(chest.model_layer, "minecraft:zombie#chestplate");
    assert_eq!(
        chest.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(chest.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(chest.texture, ARMOR_IRON_HUMANOID_TEXTURE_REF);
    assert_eq!(chest.visibility, EntityModelLayerVisibility::All);
    assert_eq!(chest.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((chest.order, chest.submit_sequence), (1, 1));

    let baby_legs = humanoid_armor_layer_pass(
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY,
        HumanoidArmorSlot::Legs,
        EntityArmorMaterial::Iron,
        ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF,
        None,
        2,
    );
    assert_eq!(baby_legs.model_layer, "minecraft:zombie_baby#leggings");
    assert_eq!(baby_legs.texture, ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
    assert_eq!((baby_legs.order, baby_legs.submit_sequence), (1, 2));

    let dyed_helmet = humanoid_armor_layer_pass(
        HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM,
        HumanoidArmorSlot::Head,
        EntityArmorMaterial::Leather,
        ARMOR_LEATHER_HUMANOID_TEXTURE_REF,
        Some(0x003F_6CDA),
        4,
    );
    assert_eq!(dyed_helmet.model_layer, "minecraft:player_slim#helmet");
    assert_eq!(dyed_helmet.texture, ARMOR_LEATHER_HUMANOID_TEXTURE_REF);
    assert_eq!(
        dyed_helmet.tint,
        [
            0x3F as f32 / 255.0,
            0x6C as f32 / 255.0,
            0xDA as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!((dyed_helmet.order, dyed_helmet.submit_sequence), (1, 4));
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
    assert!(HumanoidArmorSlot::Legs.uses_inner_model());
    assert!(!HumanoidArmorSlot::Chest.uses_inner_model());

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
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Leather, None),
        leather
    );
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Iron, None),
        [1.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Diamond, None),
        [1.0, 1.0, 1.0, 1.0]
    );

    // The rendered leather chestplate carries the brown tint; iron stays white.
    let mut refs: Vec<EntityModelTextureRef> = zombie_entity_texture_refs().to_vec();
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
                .with_chest_armor(Some(EntityArmorMaterial::Leather)),
        ],
        &atlas,
    );
    assert!(armored
        .cutout
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
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Leather, Some(dye)),
        dyed
    );
    // The incoming alpha byte is discarded (vanilla `ARGB.opaque`): 0xFF000000 | rgb is irrelevant.
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Leather, Some(0xFF00_0000 | dye)),
        dyed
    );
    // Non-dyeable materials always render white regardless of any stray dye (vanilla returns -1).
    assert_eq!(
        armor_layer_tint(EntityArmorMaterial::Iron, Some(dye)),
        [1.0, 1.0, 1.0, 1.0]
    );

    // The rendered dyed leather chestplate carries the custom tint, not the default brown.
    let mut refs: Vec<EntityModelTextureRef> = zombie_entity_texture_refs().to_vec();
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
            EntityModelInstance::zombie(77, [0.0, 64.0, 0.0], 0.0, false)
                .with_chest_armor(Some(EntityArmorMaterial::Leather))
                .with_chest_armor_dye(Some(dye)),
        ],
        &atlas,
    );
    assert!(armored
        .cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint == dyed));
    assert!(!armored.cutout.vertices.iter().any(|vertex| vertex.tint
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
    let armored_instance = EntityModelInstance::zombie(71, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_chest_armor(Some(EntityArmorMaterial::Iron))
        .with_legs_armor(Some(EntityArmorMaterial::Iron))
        .with_feet_armor(Some(EntityArmorMaterial::Iron));
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);
    assert_eq!(
        armored.cutout.vertices.len() - bare_cutout,
        240,
        "four iron armor pieces add 10 cubes of cutout geometry"
    );
    assert_eq!(armored.submissions.len(), 5);
    assert_eq!(
        armored.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        armored.submissions[0].render_type.vanilla_name(),
        "entityCutout"
    );
    assert_eq!(armored.submissions[0].texture, ZOMBIE_TEXTURE_REF);
    assert_eq!(armored.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            armored.submissions[0].order,
            armored.submissions[0].submit_sequence
        ),
        (0, 0)
    );
    assert_eq!(
        armored.submissions[0].light,
        armored_instance.render_state.shader_light()
    );
    assert_eq!(
        armored.submissions[0].overlay,
        armored_instance.render_state.overlay_coords()
    );
    let expected_transform = entity_model_root_transform(armored_instance);
    assert_eq!(armored.submissions[0].transform, expected_transform);
    for (submit, texture, sequence) in [
        (armored.submissions[1], ARMOR_IRON_HUMANOID_TEXTURE_REF, 1),
        (armored.submissions[2], ARMOR_IRON_LEGGINGS_TEXTURE_REF, 2),
        (armored.submissions[3], ARMOR_IRON_HUMANOID_TEXTURE_REF, 3),
        (armored.submissions[4], ARMOR_IRON_HUMANOID_TEXTURE_REF, 4),
    ] {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
        assert_eq!(submit.transform, expected_transform);
        assert_eq!(submit.light, armored_instance.render_state.shader_light());
        assert_eq!(submit.overlay, [0.0, 10.0]);
    }

    // Folded cutout geometry is appended in the same order as the explicit vanilla submissions:
    // base, chest, legs, feet, head.
    let mut vertex_start = 0;
    for (submit, vertex_count) in [
        (armored.submissions[0], bare_cutout),
        (armored.submissions[1], 72),
        (armored.submissions[2], 72),
        (armored.submissions[3], 48),
        (armored.submissions[4], 48),
    ] {
        let vertex_end = vertex_start + vertex_count;
        assert!(armored.cutout.vertices[vertex_start..vertex_end]
            .iter()
            .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
        vertex_start = vertex_end;
    }
    assert_eq!(vertex_start, armored.cutout.vertices.len());

    // The armor is inflated (`CubeDeformation 1.0` / `0.5`), so it floats just outside the body: the
    // armored cutout reaches wider in X than the bare body.
    let bare_x_max = bare
        .cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    let armored_x_max = armored
        .cutout
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
fn humanoid_armor_submissions_survive_missing_texture_atlas_entries() {
    // Vanilla `HumanoidArmorLayer.submit` calls the four armor slots in chest,
    // legs, feet, head order. Missing stitched equipment textures suppress only
    // folded armor geometry; the armor submissions still carry the vanilla metadata.
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

    let bare_instance = EntityModelInstance::zombie(95, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let armored_instance = bare_instance
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_chest_armor(Some(EntityArmorMaterial::Iron))
        .with_legs_armor(Some(EntityArmorMaterial::Iron))
        .with_feet_armor(Some(EntityArmorMaterial::Iron));

    let bare = entity_model_textured_meshes(&[bare_instance], &atlas);
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);

    assert_eq!(
        armored.cutout.vertices.len(),
        bare.cutout.vertices.len(),
        "missing armor atlas entries must not fold stale armor geometry"
    );
    assert_eq!(armored.submissions.len(), 5);
    let body_submit = armored.submissions[0];
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.texture, ZOMBIE_TEXTURE_REF);
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        body_submit.transform,
        entity_model_root_transform(armored_instance)
    );
    assert_eq!(
        body_submit.light,
        armored_instance.render_state.shader_light()
    );
    assert_eq!(
        body_submit.overlay,
        armored_instance.render_state.overlay_coords()
    );
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));
    assert!(armored.cutout.vertices.iter().all(|vertex| {
        vertex.light == body_submit.light && vertex.overlay == body_submit.overlay
    }));

    for (submit, texture, sequence) in [
        (armored.submissions[1], ARMOR_IRON_HUMANOID_TEXTURE_REF, 1),
        (armored.submissions[2], ARMOR_IRON_LEGGINGS_TEXTURE_REF, 2),
        (armored.submissions[3], ARMOR_IRON_HUMANOID_TEXTURE_REF, 3),
        (armored.submissions[4], ARMOR_IRON_HUMANOID_TEXTURE_REF, 4),
    ] {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, body_submit.transform);
        assert_eq!(submit.light, body_submit.light);
        assert_eq!(submit.overlay, [0.0, 10.0]);
        assert_ne!(submit.overlay, body_submit.overlay);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
    }
}

#[test]
fn armored_giant_uses_vanilla_giant_armor_layer() {
    let atlas = iron_armor_atlas();
    let base = EntityModelInstance::giant(94, [0.0, 64.0, 0.0], 0.0);
    let armored = base
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_chest_armor(Some(EntityArmorMaterial::Iron))
        .with_legs_armor(Some(EntityArmorMaterial::Iron))
        .with_feet_armor(Some(EntityArmorMaterial::Iron));

    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    let armored_meshes = entity_model_textured_meshes(&[armored], &atlas);

    assert_eq!(
        armored_meshes.cutout.vertices.len() - bare_meshes.cutout.vertices.len(),
        240,
        "GiantMobRenderer's HumanoidArmorLayer adds the standard 10 armor cubes"
    );
    assert_eq!(armored_meshes.submissions.len(), 5);
    let expected_transform = mesh_transformer_scaled_model_root_transform(base, GIANT_SCALE);
    assert_eq!(
        armored_meshes.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(armored_meshes.submissions[0].texture, ZOMBIE_TEXTURE_REF);
    assert_eq!(armored_meshes.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            armored_meshes.submissions[0].order,
            armored_meshes.submissions[0].submit_sequence
        ),
        (0, 0)
    );
    assert_eq!(armored_meshes.submissions[0].transform, expected_transform);

    for (submit, texture, sequence) in [
        (
            armored_meshes.submissions[1],
            ARMOR_IRON_HUMANOID_TEXTURE_REF,
            1,
        ),
        (
            armored_meshes.submissions[2],
            ARMOR_IRON_LEGGINGS_TEXTURE_REF,
            2,
        ),
        (
            armored_meshes.submissions[3],
            ARMOR_IRON_HUMANOID_TEXTURE_REF,
            3,
        ),
        (
            armored_meshes.submissions[4],
            ARMOR_IRON_HUMANOID_TEXTURE_REF,
            4,
        ),
    ] {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
        assert_eq!(submit.transform, expected_transform);
    }
}

#[test]
fn single_armor_slot_emits_only_its_pieces() {
    let atlas = iron_armor_atlas();
    let bare = entity_model_textured_meshes(
        &[EntityModelInstance::zombie(
            72,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    // Just a helmet: head + hat = 2 cubes → 48 vertices.
    let helmet = entity_model_textured_meshes(
        &[
            EntityModelInstance::zombie(73, [0.0, 64.0, 0.0], 0.0, false)
                .with_head_armor(Some(EntityArmorMaterial::Iron)),
        ],
        &atlas,
    );
    assert_eq!(
        helmet.cutout.vertices.len() - bare.cutout.vertices.len(),
        48
    );
}

#[test]
fn standard_humanoid_wearers_all_drape_armor() {
    let atlas = iron_armor_atlas();
    let full_iron = |instance: EntityModelInstance| {
        instance
            .with_head_armor(Some(EntityArmorMaterial::Iron))
            .with_chest_armor(Some(EntityArmorMaterial::Iron))
            .with_legs_armor(Some(EntityArmorMaterial::Iron))
            .with_feet_armor(Some(EntityArmorMaterial::Iron))
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
            armored.cutout.vertices.len() - bare.cutout.vertices.len(),
            240,
            "{:?} drapes the four armor pieces",
            instance.kind
        );
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
    assert_eq!(
        armored.cutout.vertices.len() - bare.cutout.vertices.len(),
        216
    );
}

#[test]
fn skeleton_family_armor_submissions_match_vanilla_armor_layer() {
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
        let armored = instance.with_chest_armor(Some(EntityArmorMaterial::Iron));
        let meshes = entity_model_textured_meshes(&[armored], &atlas);
        let armor_submissions: Vec<_> = meshes
            .submissions
            .iter()
            .filter(|submit| submit.render_type == EntityModelLayerRenderType::ArmorCutoutNoCull)
            .collect();
        assert_eq!(
            armor_submissions.len(),
            1,
            "{:?} should emit exactly the chest armor submit",
            instance.kind
        );
        let submit = armor_submissions[0];
        assert_eq!(submit.texture, ARMOR_IRON_HUMANOID_TEXTURE_REF);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (1, 1));
        let expected_transform = match instance.kind {
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => wither_skeleton_model_root_transform(instance),
            _ => entity_model_root_transform(instance),
        };
        assert_eq!(submit.transform, expected_transform);
    }
}

#[test]
fn piglin_family_drapes_armor_at_wider_deformation() {
    let atlas = iron_armor_atlas();
    let full_iron = |instance: EntityModelInstance| {
        instance
            .with_head_armor(Some(EntityArmorMaterial::Iron))
            .with_chest_armor(Some(EntityArmorMaterial::Iron))
            .with_legs_armor(Some(EntityArmorMaterial::Iron))
            .with_feet_armor(Some(EntityArmorMaterial::Iron))
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
            armored.cutout.vertices.len() - bare_meshes.cutout.vertices.len(),
            240,
            "{family:?} drapes the four armor pieces"
        );
    }

    // The piglin armor is grown by OUTER 1.02 (vanilla `LayerDefinitions` piglin armor) vs the standard
    // 1.0, so a rest-posed piglin's armor reaches very slightly wider in X than a same-posed zombie's
    // (0.02 model units through the shared root transform). Both are idle, so only the deformation
    // differs.
    let max_armor_x = |instance: EntityModelInstance| {
        entity_model_textured_meshes(&[full_iron(instance)], &atlas)
            .cutout
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
    for (family, base_texture) in [
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
            armored.cutout.vertices.len() - bare.cutout.vertices.len(),
            216,
            "{family:?} baby armor retains nine baby cubes"
        );
        assert_eq!(armored.submissions.len(), 5);
        assert_eq!(armored.submissions[0].texture, base_texture);
        assert_eq!(
            (
                armored.submissions[0].order,
                armored.submissions[0].submit_sequence
            ),
            (0, 0)
        );
        let expected_transform = entity_model_root_transform(baby);
        for (submit, sequence) in armored.submissions[1..].iter().zip(1..) {
            assert_eq!(
                submit.render_type,
                EntityModelLayerRenderType::ArmorCutoutNoCull
            );
            assert_eq!(submit.texture, ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
            assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
            assert_eq!(submit.transform, expected_transform);
        }
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
        &[EntityModelInstance::zombie(75, [0.0, 64.0, 0.0], 0.0, true)
            .with_head_armor(Some(EntityArmorMaterial::Iron))
            .with_chest_armor(Some(EntityArmorMaterial::Iron))
            .with_legs_armor(Some(EntityArmorMaterial::Iron))
            .with_feet_armor(Some(EntityArmorMaterial::Iron))],
        &atlas,
    );
    assert_eq!(
        armored.cutout.vertices.len() - bare.cutout.vertices.len(),
        216,
        "baby humanoid armor keeps nine retained cubes"
    );
    assert_eq!(armored.submissions.len(), 5);
    assert_eq!(armored.submissions[0].texture, ZOMBIE_BABY_TEXTURE_REF);
    assert_eq!(
        armored.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        (
            armored.submissions[0].order,
            armored.submissions[0].submit_sequence
        ),
        (0, 0)
    );

    let expected_transform =
        entity_model_root_transform(EntityModelInstance::zombie(75, [0.0, 64.0, 0.0], 0.0, true));
    for (submit, sequence) in armored.submissions[1..].iter().zip(1..) {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.texture, ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
        assert_eq!(submit.transform, expected_transform);
    }
}

#[test]
fn baby_humanoid_armor_submissions_survive_missing_texture_atlas_entry() {
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

    let bare_instance = EntityModelInstance::zombie(97, [0.0, 64.0, 0.0], 0.0, true)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let armored_instance = bare_instance
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_chest_armor(Some(EntityArmorMaterial::Iron))
        .with_legs_armor(Some(EntityArmorMaterial::Iron))
        .with_feet_armor(Some(EntityArmorMaterial::Iron));

    let bare = entity_model_textured_meshes(&[bare_instance], &atlas);
    let armored = entity_model_textured_meshes(&[armored_instance], &atlas);

    assert_eq!(
        armored.cutout.vertices.len(),
        bare.cutout.vertices.len(),
        "missing baby armor atlas entry must not fold stale armor geometry"
    );
    assert_eq!(armored.submissions.len(), 5);
    let body_submit = armored.submissions[0];
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.texture, ZOMBIE_BABY_TEXTURE_REF);
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        body_submit.transform,
        entity_model_root_transform(armored_instance)
    );
    assert_eq!(
        body_submit.light,
        armored_instance.render_state.shader_light()
    );
    assert_eq!(
        body_submit.overlay,
        armored_instance.render_state.overlay_coords()
    );
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));
    assert!(armored.cutout.vertices.iter().all(|vertex| {
        vertex.light == body_submit.light && vertex.overlay == body_submit.overlay
    }));

    for (submit, sequence) in armored.submissions[1..].iter().copied().zip(1..) {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
        assert_eq!(submit.texture, ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, body_submit.transform);
        assert_eq!(submit.light, body_submit.light);
        assert_eq!(submit.overlay, [0.0, 10.0]);
        assert_ne!(submit.overlay, body_submit.overlay);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
    }
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
    .with_villager_model_data(VillagerModelData::new(
        VillagerModelType::Plains,
        VillagerModelProfession::None,
        1,
    ));
    let armored = base
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_chest_armor(Some(EntityArmorMaterial::Iron))
        .with_legs_armor(Some(EntityArmorMaterial::Iron))
        .with_feet_armor(Some(EntityArmorMaterial::Iron));

    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    let armored_meshes = entity_model_textured_meshes(&[armored], &atlas);
    assert_eq!(
        armored_meshes.cutout.vertices.len() - bare_meshes.cutout.vertices.len(),
        216,
        "baby zombie-villager armor keeps the nine-cube baby humanoid armor topology"
    );
    assert_eq!(armored_meshes.submissions.len(), 6);
    let expected_transform = entity_model_root_transform(base);
    assert_eq!(
        armored_meshes.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        armored_meshes.submissions[0].texture,
        ZOMBIE_VILLAGER_BABY_TEXTURE_REF
    );
    assert_eq!(armored_meshes.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            armored_meshes.submissions[0].order,
            armored_meshes.submissions[0].submit_sequence
        ),
        (0, 0)
    );
    assert_eq!(armored_meshes.submissions[0].transform, expected_transform);

    for (submit, sequence) in armored_meshes.submissions[1..5].iter().zip(1..) {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::ArmorCutoutNoCull
        );
        assert_eq!(submit.texture, ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (1, sequence));
        assert_eq!(submit.transform, expected_transform);
    }

    let type_submit = armored_meshes.submissions[5];
    assert_eq!(
        type_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        type_submit.texture,
        ZOMBIE_VILLAGER_BABY_TYPE_TEXTURE_REFS[2]
    );
    assert_eq!(type_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (type_submit.order, type_submit.submit_sequence),
        (1, 1),
        "VillagerProfessionLayer submits the baby type overlay with vanilla order(1) after armor"
    );
    assert_eq!(type_submit.transform, expected_transform);
}
