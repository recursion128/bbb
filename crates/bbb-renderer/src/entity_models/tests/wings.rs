use super::*;

use crate::DynamicPlayerTextureImage;

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
        .cutout
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
        .cutout
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
    let atlas = atlas_for(&[ZOMBIE_TEXTURE_REF, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF]);
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 911,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_elytra.handle,
        size: [64, 32],
        rgba: vec![0x77; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::zombie(81, [1.0, 64.0, -2.0], 25.0, false)
        .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
            texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
            use_player_texture: true,
        }))
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
fn small_armor_stand_wings_layer_uses_baby_elytra_model() {
    // Vanilla `ArmorStand.isBaby()` returns `isSmall()`, and `WingsLayer` selects
    // `ModelLayers.ELYTRA_BABY` when `state.isBaby`.
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
        .with_chest_equipment_has_wings(true)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let baby = EntityModelInstance::zombie(85, [0.0, 64.0, 0.0], 0.0, true)
        .with_chest_wings_layer(layer)
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
