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

fn assert_elytra_vertices_have_vanilla_metadata(
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
fn non_player_humanoid_wings_layer_uses_static_equipment_texture_submission() {
    // Vanilla `HumanoidMobRenderer` adds `WingsLayer` to humanoid mobs. The profile
    // elytra/cape override is only for `AvatarRenderState`, so a zombie with a
    // WINGS asset still uses the equipment texture even when `use_player_texture` is true.
    let pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF, false, 2);
    assert_eq!(pass.kind, EntityModelLayerKind::Wings);
    assert_eq!(pass.model_layer, MODEL_LAYER_ELYTRA);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 2));

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
    let instance = EntityModelInstance::zombie(81, [1.0, 64.0, -2.0], 25.0, false)
        .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
            texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
            use_player_texture: true,
        }))
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_player_elytra_texture(Some(profile_elytra))
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("zombie elytra wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.dynamic_player_texture, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        entity_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, instance.render_state.shader_light());
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 2));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&meshes, &atlas, instance);
}

#[test]
fn zombie_custom_head_wings_and_armor_follow_vanilla_layer_order() {
    // Vanilla `HumanoidMobRenderer` registers CustomHeadLayer -> WingsLayer -> ItemInHandLayer,
    // and zombie-family subclasses append HumanoidArmorLayer afterwards. Armor still uses
    // `EquipmentLayerRenderer.order(1)`, while the skull and wings stay in order 0 after the base.
    let atlas = atlas_for(&[
        ZOMBIE_TEXTURE_REF,
        SKELETON_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        ARMOR_IRON_HUMANOID_TEXTURE_REF,
    ]);
    let instance = EntityModelInstance::zombie(185, [1.0, 64.0, -2.0], 25.0, false)
        .with_custom_head_skull(Some(EntityCustomHeadSkull::Skeleton))
        .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
            texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
            use_player_texture: true,
        }))
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_head_armor(Some(EntityArmorMaterial::Iron))
        .with_head_equipment_asset(Some(test_iron_equipment_asset_id()));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ZOMBIE_TEXTURE_REF)
        .expect("zombie body");
    assert_eq!((body.order, body.submit_sequence), (0, 0));
    let skull = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == SKELETON_TEXTURE_REF)
        .expect("custom head skull");
    assert_eq!((skull.order, skull.submit_sequence), (0, 1));
    let wings = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("wings");
    assert_eq!((wings.order, wings.submit_sequence), (0, 2));
    let armor = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ARMOR_IRON_HUMANOID_TEXTURE_REF)
        .expect("head armor");
    assert_eq!((armor.order, armor.submit_sequence), (1, 4));
}

#[test]
fn non_player_humanoid_wings_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `WingsLayer` still submits the WINGS equipment layer through
    // `EquipmentLayerRenderer.renderLayers(..., order = 0)`; a missing stitched
    // texture may suppress folded geometry, but must not erase the submission metadata.
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
    let instance = EntityModelInstance::zombie(86, [1.0, 64.0, -2.0], 25.0, false)
        .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
            texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
            use_player_texture: true,
        }))
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_player_elytra_texture(Some(profile_elytra))
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ZOMBIE_TEXTURE_REF)
        .expect("zombie body submission");
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.dynamic_player_texture, None);
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(body_submit.transform, entity_model_root_transform(instance));
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));

    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("zombie elytra wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.dynamic_player_texture, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        entity_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, instance.render_state.shader_light());
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_ne!(wings_submit.overlay, body_submit.overlay);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 2));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == body_submit.overlay));
}

#[test]
fn small_armor_stand_wings_layer_uses_baby_elytra_model() {
    // Vanilla `ArmorStand.isBaby()` returns `isSmall()`, and `WingsLayer` selects
    // `ModelLayers.ELYTRA_BABY` when `state.isBaby`.
    let adult_pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF, false, 1);
    let baby_pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF, true, 1);
    assert_eq!(adult_pass.model_layer, MODEL_LAYER_ELYTRA);
    assert_eq!(baby_pass.model_layer, MODEL_LAYER_ELYTRA_BABY);
    assert_eq!(baby_pass.kind, EntityModelLayerKind::Wings);
    assert_eq!(
        baby_pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!((baby_pass.order, baby_pass.submit_sequence), (0, 1));

    let atlas = atlas_for(&[ARMOR_STAND_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let adult = EntityModelInstance::armor_stand(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);
    let small = EntityModelInstance::armor_stand(
        83,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_light_coords((6_u32 << 4) | (10_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);

    let adult_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[adult], &atlas, None, None);
    let small_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[small], &atlas, None, None);
    let adult_positions = elytra_vertex_positions(&adult_meshes, &atlas);
    let small_positions = elytra_vertex_positions(&small_meshes, &atlas);

    assert_eq!(adult_positions.len(), 48);
    assert_eq!(small_positions.len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&adult_meshes, &atlas, adult);
    assert_elytra_vertices_have_vanilla_metadata(&small_meshes, &atlas, small);
    assert!(
        (y_extent(&adult_positions) * 0.5 - y_extent(&small_positions)).abs() < 1.0e-5,
        "small armor stand elytra should use the half-scale baby layer"
    );
}

#[test]
fn armor_stand_wings_precede_custom_head_in_vanilla_layer_order() {
    // Vanilla `ArmorStandRenderer` registers HumanoidArmorLayer, ItemInHandLayer, WingsLayer, then
    // CustomHeadLayer. The two modeled order-0 layers therefore submit wings before the skull.
    let atlas = atlas_for(&[
        ARMOR_STAND_TEXTURE_REF,
        SKELETON_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        ARMOR_IRON_HUMANOID_TEXTURE_REF,
    ]);
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let instance = EntityModelInstance::armor_stand(
        186,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Skeleton))
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_head_armor(Some(EntityArmorMaterial::Iron))
    .with_head_equipment_asset(Some(test_iron_equipment_asset_id()));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ARMOR_STAND_TEXTURE_REF)
        .expect("armor stand body");
    assert_eq!((body.order, body.submit_sequence), (0, 0));
    let wings = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("wings");
    assert_eq!((wings.order, wings.submit_sequence), (0, 1));
    let skull = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == SKELETON_TEXTURE_REF)
        .expect("custom head skull");
    assert_eq!((skull.order, skull.submit_sequence), (0, 2));
    let armor = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ARMOR_IRON_HUMANOID_TEXTURE_REF)
        .expect("head armor");
    assert_eq!((armor.order, armor.submit_sequence), (1, 4));
}

#[test]
fn player_custom_head_slot_sits_between_cape_and_wings() {
    // Vanilla `AvatarRenderer` registers HumanoidArmorLayer, item/arrow layers, ears, cape,
    // CustomHeadLayer, then WingsLayer. The modeled texture-backed order-0 sequence keeps the
    // skull immediately before wings, while armor remains in order 1.
    let atlas = atlas_for(&[
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        SKELETON_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        ARMOR_IRON_HUMANOID_TEXTURE_REF,
    ]);
    let instance = EntityModelInstance::player_with_skin(
        187,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Skeleton))
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_head_armor(Some(EntityArmorMaterial::Iron))
    .with_head_equipment_asset(Some(test_iron_equipment_asset_id()));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF)
        .expect("player body");
    assert_eq!((body.order, body.submit_sequence), (0, 0));
    let skull = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == SKELETON_TEXTURE_REF)
        .expect("custom head skull");
    assert_eq!((skull.order, skull.submit_sequence), (0, 4));
    let wings = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("wings");
    assert_eq!((wings.order, wings.submit_sequence), (0, 5));
    let armor = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ARMOR_IRON_HUMANOID_TEXTURE_REF)
        .expect("head armor");
    assert_eq!((armor.order, armor.submit_sequence), (1, 4));
}

#[test]
fn marker_hidden_glowing_armor_stand_keeps_wings_layer_without_base_submission() {
    // Vanilla `ArmorStandRenderer.getRenderType` returns null for hidden marker bases even when
    // glowing, but `LivingEntityRenderer.submit` still runs `WingsLayer` afterwards.
    let atlas = atlas_for(&[ARMOR_STAND_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
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
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_invisible(true)
    .with_outline_color(0xff55_aa11);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(meshes.submissions.len(), 1);
    assert!(!meshes
        .submissions
        .iter()
        .any(|submit| submit.texture == ARMOR_STAND_TEXTURE_REF));
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(submit.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_texture, None);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        submit.transform,
        entity_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert_eq!(submit.outline_color, 0xff55_aa11);
    assert_eq!((submit.order, submit.submit_sequence), (0, 1));
    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn hidden_zombie_keeps_wings_layer_without_base_submission() {
    // Vanilla `WingsLayer` has no `state.isInvisible` gate, so an invisible humanoid mob keeps the
    // WINGS equipment layer even when the base body render type is null.
    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let instance = EntityModelInstance::zombie(88, [1.0, 64.0, -2.0], 25.0, false)
        .with_chest_wings_layer(layer)
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_light_coords((7_u32 << 4) | (9_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_invisible(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(meshes.submissions.len(), 1);
    assert!(!meshes
        .submissions
        .iter()
        .any(|submit| submit.texture == ZOMBIE_TEXTURE_REF));
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(submit.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_texture, None);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        submit.transform,
        entity_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert_eq!(submit.outline_color, 0);
    assert_eq!((submit.order, submit.submit_sequence), (0, 2));
    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn hidden_player_keeps_wings_layer_without_base_submission() {
    // Player `WingsLayer` is also ungated by `state.isInvisible`; hidden players keep the elytra
    // equipment submission with the player-specific submit sequence.
    let atlas = atlas_for(&[
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
    ]);
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let instance = EntityModelInstance::player_with_skin(
        89,
        [1.0, 64.0, -2.0],
        25.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_invisible(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &atlas, None, None);

    assert_eq!(meshes.submissions.len(), 1);
    assert!(!meshes
        .submissions
        .iter()
        .any(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF));
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(submit.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_texture, None);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert_eq!(submit.outline_color, 0);
    assert_eq!((submit.order, submit.submit_sequence), (0, 5));
    assert_eq!(elytra_vertex_positions(&meshes, &atlas).len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&meshes, &atlas, instance);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
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
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let adult = EntityModelInstance::zombie(84, [0.0, 64.0, 0.0], 0.0, false)
        .with_chest_wings_layer(layer)
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let baby = EntityModelInstance::zombie(85, [0.0, 64.0, 0.0], 0.0, true)
        .with_chest_wings_layer(layer)
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);

    let adult_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[adult], &atlas, None, None);
    let baby_meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[baby], &atlas, None, None);
    let adult_positions = elytra_vertex_positions(&adult_meshes, &atlas);
    let baby_positions = elytra_vertex_positions(&baby_meshes, &atlas);

    assert_eq!(adult_positions.len(), 48);
    assert_eq!(baby_positions.len(), 48);
    assert_elytra_vertices_have_vanilla_metadata(&adult_meshes, &atlas, adult);
    assert_elytra_vertices_have_vanilla_metadata(&baby_meshes, &atlas, baby);
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
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let trim = Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    });

    let zombie = EntityModelInstance::zombie(86, [0.0, 64.0, 0.0], 0.0, false)
        .with_chest_wings_layer(layer)
        .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
        .with_chest_equipment_has_wings(true)
        .with_chest_wings_foil(true)
        .with_chest_wings_trim(trim);
    let zombie_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[zombie],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let base = zombie_meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("static wings base");
    assert_eq!((base.order, base.submit_sequence), (0, 2));
    let foil = zombie_meshes
        .submissions
        .iter()
        .find(|submit| submit.render_type == EntityModelLayerRenderType::ArmorEntityGlint)
        .expect("wings foil");
    assert_eq!((foil.order, foil.submit_sequence), (1, 2));
    let trim_submit = zombie_meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_texture == Some(trim_texture))
        .expect("wings trim");
    assert_eq!(
        trim_submit.render_type,
        EntityModelLayerRenderType::ArmorTrimCutoutNoCull
    );
    assert_eq!(trim_submit.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!((trim_submit.order, trim_submit.submit_sequence), (2, 2));
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
    .with_chest_wings_layer(layer)
    .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
    .with_chest_equipment_has_wings(true)
    .with_chest_wings_foil(true)
    .with_chest_wings_trim(trim)
    .with_player_elytra_texture(Some(profile_elytra));
    let player_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[player],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let profile_base = player_meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_player_texture == Some(profile_elytra))
        .expect("profile wings base");
    assert_eq!((profile_base.order, profile_base.submit_sequence), (0, 5));
    let profile_trim = player_meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_texture == Some(trim_texture))
        .expect("profile wings trim");
    assert_eq!(profile_trim.texture, PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!((profile_trim.order, profile_trim.submit_sequence), (2, 5));
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
    let layer = Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    });
    let trimmed = |entity_id, baby| {
        EntityModelInstance::zombie(entity_id, [0.0, 64.0, 0.0], 0.0, baby)
            .with_chest_wings_layer(layer)
            .with_chest_wings_equipment_asset(Some(test_wings_equipment_asset_id()))
            .with_chest_equipment_has_wings(true)
            .with_chest_wings_trim(Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }))
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
        let base = meshes
            .submissions
            .iter()
            .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
            .expect("missing-atlas wings base submission");
        assert_eq!((base.order, base.submit_sequence), (0, 2));
        let trim_submit = meshes
            .submissions
            .iter()
            .find(|submit| submit.dynamic_texture == Some(trim_texture))
            .expect("wings trim submission");
        assert_eq!((trim_submit.order, trim_submit.submit_sequence), (1, 2));
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
fn equipment_asset_wings_use_per_layer_player_override_and_submission_first_owned_lookup() {
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
    .with_chest_wings_equipment_asset(Some(EntityEquipmentAssetId::new(generation, 0)))
    .with_chest_equipment_has_wings(true)
    .with_chest_wings_foil(true)
    .with_chest_wings_trim(Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    }))
    .with_player_cape_texture(Some(profile_cape))
    .with_player_elytra_texture(Some(profile_elytra));
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
    assert_eq!(meshes.submissions.len(), 5);
    let profile = meshes.submissions[1];
    assert_eq!(profile.dynamic_player_texture, Some(profile_elytra));
    assert_eq!(profile.texture, PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!(profile.owned_texture, None);
    assert_eq!((profile.order, profile.submit_sequence), (0, 5));
    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(profile_cape)));

    let foil = meshes.submissions[2];
    assert_eq!(
        foil.render_type,
        EntityModelLayerRenderType::ArmorEntityGlint
    );
    assert_eq!((foil.order, foil.submit_sequence), (1, 5));

    let overlay = meshes.submissions[3];
    assert_eq!(
        overlay.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(overlay.dynamic_player_texture, None);
    assert_eq!(overlay.owned_texture, Some(1));
    assert_eq!((overlay.order, overlay.submit_sequence), (2, 5));

    let trim = meshes.submissions[4];
    assert_eq!(trim.dynamic_texture, Some(trim_texture));
    assert_eq!((trim.order, trim.submit_sequence), (3, 5));
    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert_eq!(meshes.armor_cutout.vertices.len(), 48);
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 48);
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 48);

    let missing_overlay = render(&owned_entries[..1]);
    assert_eq!(
        missing_overlay.submissions[3].owned_texture,
        Some(1),
        "the declared layer submission survives while its owned atlas entry is unavailable"
    );
    assert!(missing_overlay.armor_cutout.vertices.is_empty());
    assert_eq!(
        missing_overlay
            .dynamic_player_texture_armor_cutout
            .vertices
            .len(),
        48
    );
}
