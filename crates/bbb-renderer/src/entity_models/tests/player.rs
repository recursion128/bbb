use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};
use crate::player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage};
use glam::{Mat4, Vec3};

/// The wide-player limb rest poses, for the desc-level arm-swing/bob reference-formula tests (the
/// player now builds a named tree, so it has no `*_PARTS` desc const). Right arm `x = -5`, left arm
/// `x = +5`, right leg `x = -1.9` — the vanilla `PlayerModel.createMesh` offsets.
const PLAYER_FIXTURE_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

#[test]
fn player_model_parts_match_vanilla_26_1_body_layers() {
    // The player builds a named-children tree (each base part nests one skin overlay child:
    // `head` -> `hat`, `body` -> `jacket`, the arms -> `sleeve`, the legs -> `pants`), so the head
    // look resolves the `head` child by name and the visibility toggles resolve the overlays by name.
    // The geometry is asserted on the per-part unified cube consts (colored tint + textured
    // uv/tex/mirror); the wide and slim layouts differ only in the arm/sleeve widths.
    assert_eq!(PLAYER_HEAD[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(PLAYER_HAT[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_BODY[0].size, [8.0, 12.0, 4.0]);
    assert_eq!(PLAYER_JACKET[0].size, [8.5, 12.5, 4.5]);
    assert_eq!(PLAYER_JACKET[0].uv_size, [8.0, 12.0, 4.0]);
    // Wide arms/sleeves are 4 wide; slim arms/sleeves are 3 wide.
    assert_eq!(PLAYER_WIDE_RIGHT_ARM[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].size, [4.5, 12.5, 4.5]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].uv_size, [4.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].size, [3.5, 12.5, 4.5]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(
        PLAYER_RIGHT_PANTS[0],
        ModelCube::new(
            [-2.25, -0.25, -2.25],
            [4.5, 12.5, 4.5],
            PLAYER_BLUE,
            [4.0, 12.0, 4.0],
            [0.0, 32.0],
            false,
        )
    );
    assert_eq!(PLAYER_LEFT_PANTS[0].size, [4.5, 12.5, 4.5]);
}

#[test]
fn player_ears_model_matches_vanilla_deadmau5_layer_geometry() {
    // Vanilla `PlayerEarsModel.createEarsLayer`: `PlayerModel.createMesh(...).clearRecursively()`
    // keeps only the head and adds two inflated ear cubes under it.
    assert_eq!(MODEL_LAYER_PLAYER_EARS, "minecraft:player#ears");
    assert_eq!(
        PLAYER_EAR[0],
        ModelCube::new(
            [-4.0, -7.0, -2.0],
            [8.0, 8.0, 3.0],
            PLAYER_BLUE,
            [6.0, 6.0, 1.0],
            [24.0, 0.0],
            false,
        )
    );

    let model = PlayerEarsModel::new();
    assert!(model
        .root()
        .try_descendant_attach_transform(&["head", "left_ear"])
        .is_some());
    assert!(model
        .root()
        .try_descendant_attach_transform(&["head", "right_ear"])
        .is_some());
    assert!(model
        .root()
        .try_descendant_attach_transform(&["body"])
        .is_none());
}

#[test]
fn player_mesh_uses_vanilla_body_layer_geometry_and_avatar_scale() {
    let wide = entity_model_mesh(&[EntityModelInstance::player(
        155,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let slim = entity_model_mesh(&[EntityModelInstance::player(
        156,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.opaque_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PLAYER_BLUE, 0.78)));
    }

    let (wide_min, wide_max) = mesh_extents(&wide);
    let (slim_min, slim_max) = mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_texture_refs_match_vanilla_default_assets() {
    let cases = [
        (
            false,
            "player",
            EntityModelTextureRef {
                path: "textures/entity/player/wide/steve.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "player_slim",
            EntityModelTextureRef {
                path: "textures/entity/player/slim/steve.png",
                size: [64, 64],
            },
        ),
    ];

    for (slim, model_key, texture) in cases {
        let kind = EntityModelKind::Player {
            skin: EntityPlayerSkin::default_for_model(slim),
            parts: PLAYER_MODEL_PARTS_ALL_VISIBLE,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
    let profiled = EntityModelKind::Player {
        skin: EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex),
        parts: PLAYER_MODEL_PARTS_ALL_VISIBLE,
    };
    assert_eq!(profiled.model_key(), "player_slim");
    assert_eq!(
        profiled.vanilla_texture_ref(),
        Some(PLAYER_SLIM_ALEX_TEXTURE_REF)
    );
}

#[test]
fn player_model_part_visibility_masks_match_vanilla_player_model_part_bits() {
    assert_eq!(PlayerModelPartVisibility::CAPE_MASK, 1 << 0);
    assert_eq!(PlayerModelPartVisibility::JACKET_MASK, 1 << 1);
    assert_eq!(PlayerModelPartVisibility::LEFT_SLEEVE_MASK, 1 << 2);
    assert_eq!(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK, 1 << 3);
    assert_eq!(PlayerModelPartVisibility::LEFT_PANTS_MASK, 1 << 4);
    assert_eq!(PlayerModelPartVisibility::RIGHT_PANTS_MASK, 1 << 5);
    assert_eq!(PlayerModelPartVisibility::HAT_MASK, 1 << 6);
    assert_eq!(PlayerModelPartVisibility::ALL_MASK, 0x7f);
    assert_eq!(
        PLAYER_MODEL_PARTS_ALL_VISIBLE.vanilla_mask(),
        PlayerModelPartVisibility::ALL_MASK
    );
    assert_eq!(PLAYER_MODEL_PARTS_ALL_HIDDEN.vanilla_mask(), 0);

    let mask = PlayerModelPartVisibility::HAT_MASK
        | PlayerModelPartVisibility::JACKET_MASK
        | PlayerModelPartVisibility::LEFT_SLEEVE_MASK
        | PlayerModelPartVisibility::RIGHT_PANTS_MASK;
    let parts = PlayerModelPartVisibility::from_vanilla_mask(mask);
    assert!(parts.hat);
    assert!(parts.jacket);
    assert!(parts.left_sleeve);
    assert!(!parts.right_sleeve);
    assert!(!parts.left_pants);
    assert!(parts.right_pants);
    assert!(!parts.cape);
    assert_eq!(parts.vanilla_mask(), mask);
}

#[test]
fn player_textured_layer_passes_match_vanilla_avatar_renderer_model_layers() {
    let wide = player_textured_layer_passes(false, PLAYER_MODEL_PARTS_ALL_VISIBLE);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(wide[0].model_layer, MODEL_LAYER_PLAYER);
    assert_eq!(wide[0].texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    // The unified `PlayerModel` tree drives the geometry, so the layer-pass parts are vestigial; the
    // part visibility still rides the pass to the renderer.
    assert_eq!(
        wide[0].visibility,
        EntityModelLayerVisibility::PlayerParts(PLAYER_MODEL_PARTS_ALL_VISIBLE)
    );
    assert_eq!(wide[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wide[0].order, wide[0].submit_sequence), (0, 0));

    let slim_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    let slim = player_textured_layer_passes(true, slim_parts);
    assert_eq!(slim.len(), 1);
    assert_eq!(slim[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(slim[0].model_layer, MODEL_LAYER_PLAYER_SLIM);
    assert_eq!(slim[0].texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(
        slim[0].visibility,
        EntityModelLayerVisibility::PlayerParts(slim_parts)
    );
    assert_eq!(slim[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slim[0].order, slim[0].submit_sequence), (0, 0));
}

#[test]
fn player_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PLAYER, "minecraft:player#main");
    assert_eq!(MODEL_LAYER_PLAYER_SLIM, "minecraft:player_slim#main");
    // The player UVs are now carried on the unified cubes' `.tex` field; the overlay cubes keep the
    // base box as uv_size.
    assert_eq!(PLAYER_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(PLAYER_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].tex, [32.0, 0.0]);
    assert_eq!(PLAYER_HAT[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PLAYER_HAT[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(PLAYER_BODY[0].tex, [16.0, 16.0]);
    assert_eq!(PLAYER_JACKET[0].tex, [16.0, 32.0]);
    assert_eq!(PLAYER_JACKET[0].uv_size, [8.0, 12.0, 4.0]);
    assert_eq!(PLAYER_JACKET[0].size, [8.5, 12.5, 4.5]);
    assert_eq!(PLAYER_WIDE_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert_eq!(PLAYER_WIDE_LEFT_ARM[0].tex, [32.0, 48.0]);
    assert_eq!(PLAYER_WIDE_RIGHT_SLEEVE[0].tex, [40.0, 32.0]);
    assert_eq!(PLAYER_WIDE_LEFT_SLEEVE[0].tex, [48.0, 48.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_LEFT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_RIGHT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_LEFT_SLEEVE[0].uv_size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert_eq!(PLAYER_LEFT_LEG[0].tex, [16.0, 48.0]);
    assert_eq!(PLAYER_RIGHT_PANTS[0].tex, [0.0, 32.0]);
    assert_eq!(PLAYER_LEFT_PANTS[0].tex, [0.0, 48.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_player_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();

    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        player_entity_texture_refs()
            .iter()
            .map(|texture| texture.path)
            .collect::<Vec<_>>()
    );
    assert_eq!(layout.entries.len(), 18);
    assert!(rgba.len() >= 18 * 64 * 64 * 4);
}

#[test]
fn player_textured_mesh_uses_vanilla_uvs_tints_and_avatar_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let wide_instance = EntityModelInstance::player(901, [0.0, 64.0, 0.0], 0.0, false)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let slim_instance = EntityModelInstance::player(902, [0.0, 64.0, 0.0], 0.0, true)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.6)
        .with_has_red_overlay(true);
    let wide_meshes = entity_model_textured_meshes(&[wide_instance], &atlas);
    let slim_meshes = entity_model_textured_meshes(&[slim_instance], &atlas);
    assert_player_submissions_match_vanilla(&wide_meshes, wide_instance);
    assert_player_submissions_match_vanilla(&slim_meshes, slim_instance);
    let wide = &wide_meshes.cutout;
    let slim = &slim_meshes.cutout;

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.cutout_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
    assert_close2(wide.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(slim.vertices[0].uv, [16.0 / 64.0, 0.5]);

    let (wide_min, wide_max) = textured_mesh_extents(&wide);
    let (slim_min, slim_max) = textured_mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_textured_submission_records_vanilla_render_type_texture_tint_transform_and_order() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_parts(
        903,
        [3.0, 64.0, -2.0],
        25.0,
        true,
        PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
        ),
    );
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, player_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}

#[test]
fn player_deadmau5_ears_layer_records_vanilla_submission_and_geometry() {
    // Vanilla `Deadmau5EarsLayer`: when `showExtraEars && !isInvisible`, submits
    // `PlayerEarsModel` with the player's body skin, `entitySolid`, `getOverlayCoords(state, 0.0)`,
    // and default order 0 before the cape layer.
    let ears_pass = player_extra_ears_layer_pass_with_texture(PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(ears_pass.kind, EntityModelLayerKind::PlayerExtraEars);
    assert_eq!(ears_pass.model_layer, MODEL_LAYER_PLAYER_EARS);
    assert_eq!(
        ears_pass.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(ears_pass.render_type.vanilla_name(), "entitySolid");
    assert_eq!(ears_pass.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(ears_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((ears_pass.order, ears_pass.submit_sequence), (0, 1));

    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        56,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_show_extra_ears(true)
    .with_head_look(25.0, -15.0)
    .with_is_crouching(true)
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.submit_sequence == 0)
        .expect("player body submission");
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(body_submit.transform, player_model_root_transform(instance));
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));

    let ears_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.submit_sequence == 1)
        .expect("deadmau5 ears submission");
    assert_eq!(
        ears_submit.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(ears_submit.render_type.vanilla_name(), "entitySolid");
    assert_eq!(ears_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(ears_submit.dynamic_player_skin, None);
    assert_eq!(ears_submit.dynamic_player_texture, None);
    assert_eq!(ears_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(ears_submit.transform, player_model_root_transform(instance));
    assert_eq!(ears_submit.light, body_submit.light);
    assert_eq!(ears_submit.overlay, [0.0, body_submit.overlay[1]]);
    assert_ne!(ears_submit.overlay, body_submit.overlay);
    assert_eq!((ears_submit.order, ears_submit.submit_sequence), (0, 1));

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(meshes.cutout_cull.vertices.len(), 48);
    let ears_vertices = &meshes.cutout_cull.vertices;
    assert_eq!(ears_vertices.len(), 48);
    assert!(ears_vertices.iter().all(|vertex| {
        vertex.tint == ears_submit.tint
            && vertex.light == ears_submit.light
            && vertex.overlay == ears_submit.overlay
    }));
}

#[test]
fn ready_dynamic_player_skin_deadmau5_ears_use_dynamic_skin_atlas_submission() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 7702,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        rgba: vec![0xdd; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        57,
        [1.0, 65.0, -3.0],
        15.0,
        EntityPlayerSkin::Dynamic(dynamic_skin),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_show_extra_ears(true)
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_has_red_overlay(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_skins(&[instance], &atlas, Some(&dynamic_atlas));

    let ears_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.submit_sequence == 1)
        .expect("dynamic skin ears submission");
    assert_eq!(
        ears_submit.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(ears_submit.render_type.vanilla_name(), "entitySolid");
    assert_eq!(ears_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(ears_submit.dynamic_player_skin, Some(dynamic_skin));
    assert_eq!(ears_submit.dynamic_player_texture, None);
    assert_eq!(ears_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(ears_submit.transform, player_model_root_transform(instance));
    assert_eq!(ears_submit.light, instance.render_state.shader_light());
    assert_eq!(
        ears_submit.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );
    assert_eq!((ears_submit.order, ears_submit.submit_sequence), (0, 1));
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_cutout.vertices.len(), 288);
    assert_eq!(meshes.dynamic_player_skin_cutout_cull.vertices.len(), 48);
}

#[test]
fn player_deadmau5_ears_layer_is_suppressed_when_player_is_invisible() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        58,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_show_extra_ears(true)
    .with_invisible(true)
    .with_invisible_to_player(false);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.render_type != EntityModelLayerRenderType::EntitySolid));
    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.submit_sequence != 1));
}

#[test]
fn player_spin_attack_effect_records_vanilla_submission_and_geometry() {
    // Vanilla `AvatarRenderer` registers `SpinAttackEffectLayer` after WingsLayer/ParrotOnShoulder:
    // while `isAutoSpinAttack` is set it submits `SpinAttackEffectModel` with the riptide trident
    // texture, default `EntityModel` render type (`entityCutout`), no overlay, and default order 0.
    let pass = player_spin_attack_effect_layer_pass();
    assert_eq!(pass.kind, EntityModelLayerKind::PlayerSpinAttackEffect);
    assert_eq!(pass.model_layer, MODEL_LAYER_PLAYER_SPIN_ATTACK);
    assert_eq!(pass.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(pass.texture, TRIDENT_RIPTIDE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 7));

    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_riptide_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        54,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_age_in_ticks(2.0)
    .with_auto_spin_age_ticks(Some(2.0))
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF)
        .expect("player body submission");
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(body_submit.transform, player_model_root_transform(instance));
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));

    let spin_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == TRIDENT_RIPTIDE_TEXTURE_REF)
        .expect("riptide spin attack submission");
    assert_eq!(
        spin_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(spin_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(spin_submit.dynamic_player_skin, None);
    assert_eq!(spin_submit.dynamic_player_texture, None);
    assert_eq!(spin_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(spin_submit.transform, player_model_root_transform(instance));
    assert_eq!(spin_submit.light, body_submit.light);
    assert_eq!(spin_submit.overlay, [0.0, 10.0]);
    assert_ne!(spin_submit.overlay, body_submit.overlay);
    assert_eq!((spin_submit.order, spin_submit.submit_sequence), (0, 7));

    let riptide_entry = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == TRIDENT_RIPTIDE_TEXTURE_REF)
        .expect("riptide atlas entry");
    let riptide_vertices: Vec<_> = meshes
        .cutout
        .vertices
        .iter()
        .filter(|vertex| {
            vertex.uv[0] >= riptide_entry.uv.min[0]
                && vertex.uv[0] <= riptide_entry.uv.max[0]
                && vertex.uv[1] >= riptide_entry.uv.min[1]
                && vertex.uv[1] <= riptide_entry.uv.max[1]
        })
        .collect();
    assert_eq!(riptide_vertices.len(), 48);
    assert!(riptide_vertices.iter().all(|vertex| {
        vertex.tint == spin_submit.tint
            && vertex.light == spin_submit.light
            && vertex.overlay == spin_submit.overlay
    }));
    assert_eq!(meshes.cutout.vertices.len(), 336);
}

#[test]
fn player_spin_attack_submission_survives_missing_texture_atlas_entry() {
    // Missing stitched riptide texture data must suppress only the folded `SpinAttackEffectModel`
    // vertices; vanilla still builds the submit node for `trident_riptide.png` while spinning.
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == TRIDENT_RIPTIDE_TEXTURE_REF));
    let instance = EntityModelInstance::player_with_skin(
        55,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_age_in_ticks(2.0)
    .with_auto_spin_age_ticks(Some(2.0))
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF)
        .expect("player body submission");
    let spin_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == TRIDENT_RIPTIDE_TEXTURE_REF)
        .expect("riptide spin attack submission");
    assert_eq!(
        spin_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(spin_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(spin_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(spin_submit.transform, player_model_root_transform(instance));
    assert_eq!(spin_submit.light, body_submit.light);
    assert_eq!(spin_submit.overlay, [0.0, 10.0]);
    assert_eq!((spin_submit.order, spin_submit.submit_sequence), (0, 7));
    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == body_submit.overlay));
}

#[test]
fn player_parrot_on_shoulder_layer_records_vanilla_submissions_and_geometry() {
    // Vanilla `AvatarRenderer` registers `ParrotOnShoulderLayer` after `WingsLayer` and before
    // `SpinAttackEffectLayer`. The layer submits left then right shoulder parrots with
    // `ParrotRenderer.getVariantTexture`, `entityCutout`, player light/outline, and NO_OVERLAY.
    let left_pass = player_parrot_on_shoulder_layer_pass(ParrotModelVariant::Blue, true);
    assert_eq!(
        left_pass.kind,
        EntityModelLayerKind::PlayerLeftShoulderParrot
    );
    assert_eq!(left_pass.model_layer, MODEL_LAYER_PARROT);
    assert_eq!(
        left_pass.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(left_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(
        left_pass.texture,
        parrot_texture_ref(ParrotModelVariant::Blue)
    );
    assert_eq!(left_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((left_pass.order, left_pass.submit_sequence), (0, 5));

    let right_pass = player_parrot_on_shoulder_layer_pass(ParrotModelVariant::Gray, false);
    assert_eq!(
        right_pass.kind,
        EntityModelLayerKind::PlayerRightShoulderParrot
    );
    assert_eq!(
        right_pass.texture,
        parrot_texture_ref(ParrotModelVariant::Gray)
    );
    assert_eq!((right_pass.order, right_pass.submit_sequence), (0, 6));

    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_parrot_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        56,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_head_look(18.0, -12.0)
    .with_walk_animation(2.0, 0.7)
    .with_age_in_ticks(3.5)
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_player_left_shoulder_parrot(Some(ParrotModelVariant::Blue))
    .with_player_right_shoulder_parrot(Some(ParrotModelVariant::Gray));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF)
        .expect("player body submission");
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());

    let left_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == parrot_texture_ref(ParrotModelVariant::Blue))
        .expect("left shoulder parrot submission");
    assert_eq!(
        left_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(left_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(left_submit.dynamic_player_skin, None);
    assert_eq!(left_submit.dynamic_player_texture, None);
    assert_eq!(left_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        left_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::new(0.4, -1.5, 0.0))
    );
    assert_eq!(left_submit.light, body_submit.light);
    assert_eq!(left_submit.overlay, [0.0, 10.0]);
    assert_ne!(left_submit.overlay, body_submit.overlay);
    assert_eq!((left_submit.order, left_submit.submit_sequence), (0, 5));

    let right_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == parrot_texture_ref(ParrotModelVariant::Gray))
        .expect("right shoulder parrot submission");
    assert_eq!(
        right_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(right_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(right_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        right_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::new(-0.4, -1.5, 0.0))
    );
    assert_eq!(right_submit.light, body_submit.light);
    assert_eq!(right_submit.overlay, [0.0, 10.0]);
    assert_eq!((right_submit.order, right_submit.submit_sequence), (0, 6));

    for (variant, submit) in [
        (ParrotModelVariant::Blue, left_submit),
        (ParrotModelVariant::Gray, right_submit),
    ] {
        let entry = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == parrot_texture_ref(variant))
            .expect("parrot atlas entry");
        let parrot_vertices: Vec<_> = meshes
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
        assert_eq!(parrot_vertices.len(), 264);
        assert!(parrot_vertices.iter().all(|vertex| {
            vertex.tint == submit.tint
                && vertex.light == submit.light
                && vertex.overlay == submit.overlay
        }));
    }
    assert_eq!(meshes.cutout.vertices.len(), 288 + 264 * 2);
}

#[test]
fn player_parrot_on_shoulder_layer_uses_crouching_y_offset() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_parrot_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        57,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_is_crouching(true)
    .with_player_left_shoulder_parrot(Some(ParrotModelVariant::Green));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    let submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == parrot_texture_ref(ParrotModelVariant::Green))
        .expect("left shoulder parrot submission");
    assert_eq!(
        submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::new(0.4, -1.3, 0.0))
    );
    assert_eq!((submit.order, submit.submit_sequence), (0, 5));
}

#[test]
fn player_shoulder_parrot_submission_survives_missing_texture_atlas_entry() {
    // Missing stitched parrot texture data suppresses only the folded parrot geometry; vanilla still
    // records the submit node with texture/render type/order metadata.
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == parrot_texture_ref(ParrotModelVariant::YellowBlue)));
    let instance = EntityModelInstance::player_with_skin(
        58,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_player_left_shoulder_parrot(Some(ParrotModelVariant::YellowBlue));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    let shoulder_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == parrot_texture_ref(ParrotModelVariant::YellowBlue))
        .expect("left shoulder parrot submission");
    assert_eq!(
        shoulder_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(shoulder_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(shoulder_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        shoulder_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::new(0.4, -1.5, 0.0))
    );
    assert_eq!(
        (shoulder_submit.order, shoulder_submit.submit_sequence),
        (0, 5)
    );
    assert_eq!(meshes.cutout.vertices.len(), 288);
}

#[test]
fn ready_dynamic_player_skin_body_uses_dynamic_cutout_atlas_submission() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 7701,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let instance = EntityModelInstance::player_with_skin(
        904,
        [1.0, 65.0, -3.0],
        15.0,
        EntityPlayerSkin::Dynamic(dynamic_skin),
        PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
        ),
    );
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let meshes =
        entity_model_textured_meshes_with_dynamic_skins(&[instance], &atlas, Some(&dynamic_atlas));

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_skin, Some(dynamic_skin));
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, player_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_cutout.cutout_faces, 48);
    assert_eq!(meshes.dynamic_player_skin_cutout.vertices.len(), 192);
}

#[test]
fn first_person_player_arm_submits_selected_arm_as_entity_translucent() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let light = [5.0 / 15.0, 11.0 / 15.0];
    let bare_arm = FirstPersonPlayerArm {
        left: false,
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        sleeve_visible: false,
        transform,
        light,
    };
    let sleeved_arm = FirstPersonPlayerArm {
        sleeve_visible: true,
        ..bare_arm
    };

    let bare = first_person_player_arm_textured_meshes(&[bare_arm], &atlas, None);
    let sleeved = first_person_player_arm_textured_meshes(&[sleeved_arm], &atlas, None);

    assert_eq!(bare.submissions.len(), 1);
    let submit = bare.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_skin, None);
    assert_eq!(submit.transform, transform);
    assert_eq!(submit.light, light);
    assert_eq!(submit.overlay, ENTITY_VERTEX_NO_OVERLAY);
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert!(bare.cutout.vertices.is_empty());
    assert_eq!(bare.translucent.vertices.len(), 24);
    assert_eq!(bare.translucent.indices.len(), 36);
    assert_eq!(sleeved.translucent.vertices.len(), 48);
    assert_eq!(sleeved.translucent.indices.len(), 72);
    assert_ne!(bare.translucent.vertices, sleeved.translucent.vertices);
}

/// End-to-end GPU proof of the player-arm content of [`crate::render`]'s `first_person_item_pass`:
/// bakes the real first-person right-arm translucent mesh (`first_person_player_arm_textured_meshes`,
/// the exact geometry the pass draws) in front of a fixed perspective first-person camera, renders it
/// through the real [`create_entity_model_translucent_pipeline`] + entity texture atlas + lightmap (the
/// pass's `entity_model_translucent_pipeline` path), reads the framebuffer back, and asserts the arm's
/// projected centroid shows the arm's texture colour while a far corner stays background — the
/// remaining first-person sentinel state (2). Skips (no assertion) when no GPU adapter is available.
#[test]
fn first_person_player_arm_renders_visible_pixels() {
    use wgpu::util::DeviceExt;

    use crate::camera::{CameraPose, CameraUniform, LightmapEnvironment};
    use crate::entity_models::gpu::create_entity_model_translucent_pipeline;
    use crate::gpu::{create_camera_buffer, create_depth_target, create_terrain_bind_group_layout};
    use crate::lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu,
        create_lightmap_sample_bind_group_layout,
    };

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;
    // Non-sRGB target so the readback bytes are the shader's linear output verbatim. `320 * 4 = 1280`
    // is a multiple of 256, so the texture-to-buffer copy needs no row padding.
    const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let Some(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
    else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("bbb-first-person-arm-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    )) else {
        return;
    };

    // Fixed first-person camera looking straight down +Z (yaw = pitch = 0); `CameraUniform::from_pose`
    // is the exact perspective `view_proj` the first-person pass uploads, and `camera_world =
    // view.inverse()` mirrors `first_person_camera_world_transform` in bbb-native.
    let aspect = WIDTH as f32 / HEIGHT as f32;
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let camera_uniform = CameraUniform::from_pose(pose, aspect);
    let view_proj = camera_uniform.view_proj();
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    let camera_world = Mat4::look_at_rh(eye, eye + Vec3::Z, Vec3::Y).inverse();

    // Entity texture atlas filled with opaque green in the WideSteve skin slot, so the arm's textured
    // faces sample an unambiguous, non-background colour (the shipped `steve` test images are a
    // transparent 0-fill, which would render nothing). The atlas layout drives the real arm mesh baker.
    let green = |texture: EntityModelTextureRef| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        let mut rgba = vec![0u8; len];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 255, 0, 255]);
        }
        EntityModelTextureImage::new(texture, rgba)
    };
    let images = [
        green(PLAYER_WIDE_STEVE_TEXTURE_REF),
        green(PLAYER_SLIM_STEVE_TEXTURE_REF),
    ];
    let (atlas_layout, atlas_rgba) =
        build_entity_model_texture_atlas(&images).expect("entity atlas");

    // Probe the real right-arm mesh at identity to find its model-space centroid, then seat that
    // centroid a fixed offset in front of the camera (`camera_world * T(view_offset)`, scaled down): the
    // arm anchor is derived from the mesh, not guessed. `first_person_player_arm_textured_meshes` is the
    // exact production baker the renderer feeds into the first-person pass.
    let arm = FirstPersonPlayerArm {
        left: false,
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        sleeve_visible: false,
        transform: Mat4::IDENTITY,
        light: [1.0, 1.0],
    };
    let probe = first_person_player_arm_textured_meshes(&[arm], &atlas_layout, None);
    assert!(
        !probe.translucent.vertices.is_empty(),
        "arm mesh has geometry"
    );
    let mut centroid = Vec3::ZERO;
    for vertex in &probe.translucent.vertices {
        centroid += Vec3::from_array(vertex.position);
    }
    centroid /= probe.translucent.vertices.len() as f32;

    // Seat the centroid at `VIEW_OFFSET` (right + slightly up + forward) in camera-local space, scaled
    // so the ~12-unit arm becomes a fraction of a metre — small enough that a far corner stays clear.
    const SCALE: f32 = 0.03;
    let view_offset = Vec3::new(0.15, -0.10, -0.75);
    let placed_transform = camera_world
        * Mat4::from_translation(view_offset)
        * Mat4::from_scale(Vec3::splat(SCALE))
        * Mat4::from_translation(-centroid);
    let placed_arm = FirstPersonPlayerArm {
        transform: placed_transform,
        ..arm
    };
    let mesh =
        first_person_player_arm_textured_meshes(&[placed_arm], &atlas_layout, None).translucent;

    // Anchor: the centroid maps to `camera_world * VIEW_OFFSET`; project it through the same `view_proj`
    // the shader uses, then apply wgpu's viewport transform (origin top-left, NDC y up so y is flipped).
    let clip = view_proj * (placed_transform * centroid.extend(1.0));
    let ndc = clip.truncate() / clip.w;
    let anchor_px = ((ndc.x * 0.5 + 0.5) * WIDTH as f32).round() as u32;
    let anchor_py = ((0.5 - ndc.y * 0.5) * HEIGHT as f32).round() as u32;
    assert!(
        anchor_px < WIDTH && anchor_py < HEIGHT,
        "arm centroid projects on-screen, got ({anchor_px},{anchor_py})"
    );

    // Upload the green atlas as the entity texture atlas bound at group 0 (camera @0, texture @1,
    // sampler @2 — the terrain-style layout the entity pipeline shares).
    let bind_group_layout = create_terrain_bind_group_layout(&device);
    let camera_buffer = create_camera_buffer(&device);
    queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fp-arm-test-atlas"),
        size: wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.width * 4),
            rows_per_image: Some(atlas_layout.height),
        },
        wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("fp-arm-test-sampler"),
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fp-arm-test-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas_sampler),
            },
        ],
    });
    let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
    let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
    let lightmap = create_lightmap_gpu(
        &device,
        &queue,
        &lightmap_bind_group_layout,
        &lightmap_sample_bind_group_layout,
        LightmapEnvironment::default(),
    );
    let pipeline = create_entity_model_translucent_pipeline(
        &device,
        COLOR_FORMAT,
        &bind_group_layout,
        &lightmap_sample_bind_group_layout,
    );

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("fp-arm-test-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("fp-arm-test-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fp-arm-test-color"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_depth_target(&device, WIDTH, HEIGHT);
    let bytes_per_row = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fp-arm-test-readback"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fp-arm-test-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Blue background — distinct from the green arm.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &color_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let o = (y * bytes_per_row + x * 4) as usize;
        [data[o], data[o + 1], data[o + 2], data[o + 3]]
    };
    let anchor_pixel = pixel(anchor_px, anchor_py);
    let corner_pixel = pixel(0, 0);

    // (2) arm visible: the arm centroid shows the green arm texture (green dominant, not the blue
    // background); the top-left corner stays blue background.
    assert!(
        anchor_pixel[1] > 64
            && anchor_pixel[1] > anchor_pixel[2]
            && anchor_pixel[0] < anchor_pixel[1],
        "arm centroid should show the green player arm, got {anchor_pixel:?}"
    );
    assert!(
        corner_pixel[2] > 128 && corner_pixel[1] < 128,
        "corner should stay background, got {corner_pixel:?}"
    );

    drop(data);
    readback.unmap();
}

#[test]
fn first_person_ready_dynamic_player_arm_uses_dynamic_skin_translucent_bucket() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 8812,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        rgba: vec![0x7f; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let arm = FirstPersonPlayerArm {
        left: true,
        skin: EntityPlayerSkin::Dynamic(dynamic_skin),
        sleeve_visible: true,
        transform: Mat4::IDENTITY,
        light: [1.0, 1.0],
    };

    let meshes = first_person_player_arm_textured_meshes(&[arm], &atlas, Some(&dynamic_atlas));

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_skin, Some(dynamic_skin));
    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_translucent.vertices.len(), 48);
    assert_eq!(meshes.dynamic_player_skin_translucent.indices.len(), 72);
}

#[test]
fn dynamic_player_texture_atlas_stitches_variable_profile_textures() {
    let cape = dynamic_player_texture_image(20, [4, 2], 10);
    let elytra = dynamic_player_texture_image(10, [2, 3], 40);

    let (layout, rgba) = build_dynamic_player_texture_atlas(&[cape.clone(), elytra.clone()])
        .expect("dynamic player texture atlas");

    assert_eq!(layout.width, 4);
    assert_eq!(layout.height, 5);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(layout.entries[0].handle, cape.handle);
    assert_eq!(layout.entries[0].size, cape.size);
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 2.0 / 5.0]);
    assert_eq!(layout.entries[1].handle, elytra.handle);
    assert_eq!(layout.entries[1].size, elytra.size);
    assert_close2(layout.entries[1].uv.min, [0.0, 2.0 / 5.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);

    assert_eq!(atlas_pixel(&rgba, layout.width, 3, 1), [13, 3, 1, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 1, 2), [41, 1, 0, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 2, 2), [0, 0, 0, 0]);
}

#[test]
fn dynamic_player_texture_atlas_rejects_bad_profile_texture_dimensions() {
    let err = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: 88,
        size: [2, 2],
        rgba: vec![0xff; 15],
    }])
    .unwrap_err();

    assert!(err.to_string().contains("expected 16 for 2x2"));
}

#[test]
fn dynamic_player_texture_atlas_rejects_duplicate_profile_texture_handles() {
    let err = build_dynamic_player_texture_atlas(&[
        dynamic_player_texture_image(88, [2, 2], 10),
        dynamic_player_texture_image(88, [1, 3], 20),
    ])
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("duplicate dynamic player texture handle 88"));
}

#[test]
fn dynamic_player_texture_atlas_rejects_zero_sized_profile_textures() {
    let err = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: 89,
        size: [0, 2],
        rgba: Vec::new(),
    }])
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("dynamic player texture 89 has zero-sized dimensions"));
}

#[test]
fn ready_dynamic_player_texture_submission_uses_dynamic_texture_atlas_bucket() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_texture = EntityDynamicPlayerTexture {
        handle: 7001,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: dynamic_texture.handle,
        size: [64, 64],
        rgba: vec![0xaa; 64 * 64 * 4],
    }])
    .unwrap()
    .0;

    let meshes = dynamic_player_texture_test_meshes(
        EntityModelLayerRenderType::EntityCutout,
        dynamic_texture,
        &static_atlas,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(submit.dynamic_player_skin, None);
    assert_eq!(submit.dynamic_player_texture, Some(dynamic_texture));
    assert_eq!(submit.tint, [0.25, 0.5, 0.75, 1.0]);
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert_eq!(meshes.dynamic_player_texture_cutout.cutout_faces, 72);
    assert_eq!(meshes.dynamic_player_texture_cutout.vertices.len(), 288);
}

#[test]
fn dynamic_player_texture_submission_falls_back_to_static_atlas_when_not_uploaded() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_texture = EntityDynamicPlayerTexture {
        handle: 7002,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };

    let meshes = dynamic_player_texture_test_meshes(
        EntityModelLayerRenderType::EntityTranslucent,
        dynamic_texture,
        &static_atlas,
        None,
    );

    assert_eq!(meshes.submissions.len(), 1);
    assert_eq!(
        meshes.submissions[0].dynamic_player_texture,
        Some(dynamic_texture)
    );
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 72);
    assert_eq!(meshes.translucent.vertices.len(), 288);
}

#[test]
fn player_cape_layer_uses_dynamic_profile_texture_atlas_submission() {
    let pass = player_cape_layer_pass();
    assert_eq!(pass.kind, EntityModelLayerKind::PlayerCape);
    assert_eq!(pass.model_layer, MODEL_LAYER_PLAYER_CAPE);
    assert_eq!(pass.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(pass.render_type.vanilla_name(), "entitySolid");
    assert_eq!(pass.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 2));

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7101,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        rgba: vec![0x66; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        45,
        [1.0, 65.0, -2.0],
        30.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_player_cape_texture(Some(cape_texture))
    .with_player_cape_flap(4.0)
    .with_player_cape_lean(10.0)
    .with_player_cape_lean2(-6.0);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    let body_submit = meshes.submissions[0];
    assert_eq!(body_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    let cape_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_player_texture == Some(cape_texture))
        .expect("cape submission");
    assert_eq!(
        cape_submit.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(cape_submit.render_type.vanilla_name(), "entitySolid");
    assert_eq!(cape_submit.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(cape_submit.dynamic_player_skin, None);
    assert_eq!(cape_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(cape_submit.transform, player_model_root_transform(instance));
    assert_eq!(cape_submit.light, body_submit.light);
    assert_eq!(cape_submit.overlay, [0.0, 10.0]);
    assert_ne!(cape_submit.overlay, body_submit.overlay);
    assert_eq!((cape_submit.order, cape_submit.submit_sequence), (0, 2));
    assert_eq!(meshes.dynamic_player_texture_cutout_cull.cutout_faces, 6);
    assert_eq!(meshes.dynamic_player_texture_cutout_cull.vertices.len(), 24);
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
}

#[test]
fn player_cape_layer_waits_for_dynamic_profile_texture_upload() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7102,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let instance = EntityModelInstance::player_with_skin(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_player_cape_texture(Some(cape_texture));

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(cape_texture)));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
}

#[test]
fn player_cape_layer_is_suppressed_when_player_is_invisible() {
    // Vanilla `CapeLayer.submit` gates on `!state.isInvisible`, even when the player is self-visible
    // through the force-transparent base-body path.
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7105,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        rgba: vec![0x99; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        49,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_player_cape_texture(Some(cape_texture))
    .with_invisible(true)
    .with_invisible_to_player(false);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(cape_texture)));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
}

#[test]
fn player_cape_layer_is_suppressed_by_wings_chest_equipment() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7103,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        rgba: vec![0x77; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        47,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_player_cape_texture(Some(cape_texture))
    .with_chest_equipment_has_wings(true)
    .with_chest_equipment_has_humanoid(true);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(cape_texture)));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
}

#[test]
fn player_cape_layer_offsets_for_humanoid_chest_equipment() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7104,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        rgba: vec![0x88; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        48,
        [3.0, 66.0, -4.0],
        25.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((4_u32 << 4) | (10_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_player_cape_texture(Some(cape_texture))
    .with_chest_equipment_has_humanoid(true);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    let cape_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_player_texture == Some(cape_texture))
        .expect("cape submission");
    assert_eq!(
        cape_submit.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(cape_submit.render_type.vanilla_name(), "entitySolid");
    assert_eq!(cape_submit.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(cape_submit.dynamic_player_skin, None);
    assert_eq!(cape_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        cape_submit.transform,
        player_model_root_transform(instance)
            * Mat4::from_translation(Vec3::new(0.0, -0.053125, 0.06875))
    );
    assert_eq!((cape_submit.order, cape_submit.submit_sequence), (0, 2));
    assert_eq!(cape_submit.light, instance.render_state.shader_light());
    assert_eq!(cape_submit.overlay, [0.0, 10.0]);
    assert_ne!(cape_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!(meshes.dynamic_player_texture_cutout_cull.vertices.len(), 24);
    assert!(meshes
        .dynamic_player_texture_cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == cape_submit.light
            && vertex.overlay == cape_submit.overlay));
}

#[test]
fn player_wings_layer_uses_static_equipment_texture_submission() {
    let pass = wings_layer_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF, false, 4);
    assert_eq!(pass.kind, EntityModelLayerKind::Wings);
    assert_eq!(pass.model_layer, MODEL_LAYER_ELYTRA);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 4));

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_and_elytra_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        49,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_equipment_has_wings(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    let body_submit = meshes.submissions[0];
    assert_eq!(body_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("static elytra wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.dynamic_player_texture, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, body_submit.light);
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_ne!(wings_submit.overlay, body_submit.overlay);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 4));
    assert_eq!(meshes.dynamic_player_texture_cutout.vertices.len(), 0);
    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(meshes.armor_cutout.vertices.len(), 48);
}

#[test]
fn player_wings_layer_static_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `WingsLayer` falls back to the equipment WINGS texture when the
    // avatar has no profile elytra/cape override. A missing stitched texture
    // must suppress only folded geometry, not the submission boundary.
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!static_atlas
        .entries
        .iter()
        .any(|entry| entry.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF));
    let instance = EntityModelInstance::player_with_skin(
        53,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((7_u32 << 4) | (9_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_equipment_has_wings(true);

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    let body_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == PLAYER_WIDE_STEVE_TEXTURE_REF)
        .expect("player body submission");
    assert_eq!(
        body_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(body_submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(body_submit.dynamic_player_texture, None);
    assert_eq!(body_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(body_submit.transform, player_model_root_transform(instance));
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((body_submit.order, body_submit.submit_sequence), (0, 0));

    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF)
        .expect("static elytra wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.dynamic_player_texture, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, body_submit.light);
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_ne!(wings_submit.overlay, body_submit.overlay);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 4));
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == body_submit.overlay));
}

#[test]
fn player_wings_layer_prefers_ready_profile_elytra_texture_over_cape() {
    let pass = wings_layer_pass(PLAYER_PROFILE_ELYTRA_TEXTURE_REF, false, 4);
    assert_eq!(pass.kind, EntityModelLayerKind::Wings);
    assert_eq!(pass.model_layer, MODEL_LAYER_ELYTRA);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 4));

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 7201,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let profile_cape = EntityDynamicPlayerTexture {
        handle: 7202,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_elytra.handle,
        size: [64, 32],
        rgba: vec![0x99; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        50,
        [1.0, 64.0, -1.0],
        10.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((4_u32 << 4) | (12_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_equipment_has_wings(true)
    .with_player_elytra_texture(Some(profile_elytra))
    .with_player_cape_texture(Some(profile_cape));

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    let body_submit = meshes.submissions[0];
    assert_eq!(body_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_player_texture == Some(profile_elytra))
        .expect("profile elytra wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.texture, PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!(wings_submit.dynamic_player_skin, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, body_submit.light);
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_ne!(wings_submit.overlay, body_submit.overlay);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 4));
    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(profile_cape)));
    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert!(
        meshes
            .dynamic_player_texture_armor_cutout
            .vertices
            .iter()
            .all(|vertex| vertex.light == wings_submit.light
                && vertex.overlay == wings_submit.overlay)
    );
}

#[test]
fn player_wings_layer_uses_ready_profile_cape_texture_when_elytra_is_absent() {
    let pass = wings_layer_pass(PLAYER_PROFILE_CAPE_TEXTURE_REF, false, 4);
    assert_eq!(pass.kind, EntityModelLayerKind::Wings);
    assert_eq!(pass.model_layer, MODEL_LAYER_ELYTRA);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((pass.order, pass.submit_sequence), (0, 4));

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let profile_cape = EntityDynamicPlayerTexture {
        handle: 7203,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_cape.handle,
        size: [64, 32],
        rgba: vec![0x55; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        51,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_light_coords((6_u32 << 4) | (10_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_equipment_has_wings(true)
    .with_player_cape_texture(Some(profile_cape));

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    let body_submit = meshes.submissions[0];
    assert_eq!(body_submit.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(body_submit.light, instance.render_state.shader_light());
    assert_eq!(body_submit.overlay, instance.render_state.overlay_coords());
    let wings_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.dynamic_player_texture == Some(profile_cape))
        .expect("profile cape wings submission");
    assert_eq!(
        wings_submit.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(wings_submit.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(wings_submit.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(wings_submit.dynamic_player_skin, None);
    assert_eq!(wings_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        wings_submit.transform,
        player_model_root_transform(instance) * Mat4::from_translation(Vec3::Z * 0.125)
    );
    assert_eq!(wings_submit.light, body_submit.light);
    assert_eq!(wings_submit.overlay, [0.0, 10.0]);
    assert_ne!(wings_submit.overlay, body_submit.overlay);
    assert_eq!((wings_submit.order, wings_submit.submit_sequence), (0, 4));
    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert!(
        meshes
            .dynamic_player_texture_armor_cutout
            .vertices
            .iter()
            .all(|vertex| vertex.light == wings_submit.light
                && vertex.overlay == wings_submit.overlay)
    );
}

#[test]
fn player_wings_layer_waits_for_profile_texture_upload() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_and_elytra_texture_images()).unwrap();
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 7204,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let instance = EntityModelInstance::player_with_skin(
        52,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_chest_wings_layer(Some(EntityEquipmentLayerTexture {
        texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
        use_player_texture: true,
    }))
    .with_chest_equipment_has_wings(true)
    .with_player_elytra_texture(Some(profile_elytra));

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.dynamic_player_texture != Some(profile_elytra)));
    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.texture != ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF));
    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_armor_cutout
        .vertices
        .is_empty());
}

#[test]
fn player_textured_mesh_applies_vanilla_model_part_visibility_to_overlay_parts() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let hidden_instance = EntityModelInstance::player_with_parts(
        903,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_HIDDEN,
    );
    let hidden_meshes = entity_model_textured_meshes(&[hidden_instance], &atlas);
    assert_player_submissions_match_vanilla(&hidden_meshes, hidden_instance);
    let hidden = &hidden_meshes.cutout;
    assert_eq!(hidden.cutout_faces, 36);
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.indices.len(), 216);

    let partial_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let partial_instance =
        EntityModelInstance::player_with_parts(904, [0.0, 64.0, 0.0], 0.0, true, partial_parts);
    let partial_meshes = entity_model_textured_meshes(&[partial_instance], &atlas);
    assert_player_submissions_match_vanilla(&partial_meshes, partial_instance);
    let partial = &partial_meshes.cutout;
    assert_eq!(partial.cutout_faces, 48);
    assert_eq!(partial.vertices.len(), 192);
    assert_eq!(partial.indices.len(), 288);
    assert!(partial
        .vertices
        .iter()
        .any(|vertex| vertex.uv[1] >= 32.0 / 64.0));
}

#[test]
fn player_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(903, [0.0, 64.0, 0.0], 0.0, slim);
        let yawed_instance = base.with_head_look(45.0, 0.0);
        let pitched_instance = base.with_head_look(0.0, -20.0);
        let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
        let yawed_meshes = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched_meshes = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_player_submissions_match_vanilla(&resting_meshes, base);
        assert_player_submissions_match_vanilla(&yawed_meshes, yawed_instance);
        assert_player_submissions_match_vanilla(&pitched_meshes, pitched_instance);
        let resting = &resting_meshes.cutout;
        let yawed = &yawed_meshes.cutout;
        let pitched = &pitched_meshes.cutout;

        // Head look turns the head part (index 0, shared across all passes)
        // without changing the vertex count.
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "slim={slim}");
        assert_ne!(yawed.vertices, pitched.vertices, "slim={slim}");
    }
}

#[test]
fn player_attack_swing_twists_body_and_whacks_the_swinging_arm() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` (WHACK), at attackTime `t` with head pitch `p`:
    // the body twists `yRot = sin(sqrt(t) · 2π) · 0.2`, the arm anchors swing around it, and the
    // attacking arm whacks down. `setupAttackAnimation` runs last, so it accumulates onto the idle pose.
    let t = 0.4_f32;
    let pitch = -10.0_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let base =
        EntityModelInstance::player(900, [0.0, 64.0, 0.0], 0.0, false).with_head_look(0.0, pitch);

    // Baseline: a non-swinging player keeps the body untwisted and the arms on the idle pose.
    let mut resting = PlayerModel::new(false);
    resting.prepare(&base);
    let resting_right_xrot = resting.root_mut().child_mut("right_arm").pose.rotation[0];
    let resting_left_xrot = resting.root_mut().child_mut("left_arm").pose.rotation[0];
    assert_eq!(resting.root_mut().child_mut("body").pose.rotation[1], 0.0);

    // Main-hand swing twists the body and whacks the RIGHT arm.
    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_attack_anim(t));
    assert!(
        (main.root_mut().child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body twists with the swing"
    );
    let right = main.root_mut().child_mut("right_arm");
    // The arm anchor swings around the twisting body (vanilla `rightArm.x/z`), overwriting the bind.
    assert!((right.pose.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!((right.pose.offset[2] - body_yrot.sin() * 5.0).abs() < 1e-6);
    // The whack drives the right arm well forward of its idle pitch.
    assert!(
        right.pose.rotation[0] < resting_right_xrot - 0.8,
        "the main hand whacks down: {} vs resting {resting_right_xrot}",
        right.pose.rotation[0]
    );
    // The off (left) arm is not whacked — only the shared body twist adds to its xRot.
    let main_left_xrot = main.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (main_left_xrot - (resting_left_xrot + body_yrot)).abs() < 1e-6,
        "the idle off arm only picks up the body twist on xRot"
    );

    // Off-hand swing negates the body twist and whacks the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_attack_anim(t).with_attack_arm_off_hand(true));
    assert!(
        (off.root_mut().child_mut("body").pose.rotation[1] - (-body_yrot)).abs() < 1e-6,
        "the off-hand swing negates the body twist"
    );
    assert!(
        off.root_mut().child_mut("left_arm").pose.rotation[0] < resting_left_xrot - 0.8,
        "the off hand whacks the left arm"
    );
    // The idle right arm in an off-hand swing is not whacked on its pitch (only yRot picks up the twist).
    assert!(
        (off.root_mut().child_mut("right_arm").pose.rotation[0] - resting_right_xrot).abs() < 1e-6,
        "the idle main arm keeps its pitch during an off-hand swing"
    );
}

#[test]
fn player_none_swing_keeps_attack_prologue_without_whack() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` still runs its shared body/arm-anchor prologue for
    // `SwingAnimationType.NONE`, then breaks before the WHACK arm chop.
    let t = 0.4_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let base =
        EntityModelInstance::player(904, [0.0, 64.0, 0.0], 0.0, false).with_head_look(0.0, -10.0);

    let mut resting = PlayerModel::new(false);
    resting.prepare(&base);
    let resting_right = resting.root_mut().child_mut("right_arm").pose;
    let resting_left = resting.root_mut().child_mut("left_arm").pose;

    let mut none = PlayerModel::new(false);
    none.prepare(&base.with_attack_anim(t).with_main_hand_swing_is_none(true));
    let body = none.root_mut().child_mut("body").pose;
    let right = none.root_mut().child_mut("right_arm").pose;
    let left = none.root_mut().child_mut("left_arm").pose;

    assert!((body.rotation[1] - body_yrot).abs() < 1.0e-6);
    assert!((right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1.0e-6);
    assert!((right.offset[2] - body_yrot.sin() * 5.0).abs() < 1.0e-6);
    assert!((right.rotation[0] - resting_right.rotation[0]).abs() < 1.0e-6);
    assert!((right.rotation[1] - (resting_right.rotation[1] + body_yrot)).abs() < 1.0e-6);
    assert!((right.rotation[2] - resting_right.rotation[2]).abs() < 1.0e-6);
    assert!((left.rotation[0] - (resting_left.rotation[0] + body_yrot)).abs() < 1.0e-6);

    let mut whack = PlayerModel::new(false);
    whack.prepare(&base.with_attack_anim(t));
    assert!(
        whack.root_mut().child_mut("right_arm").pose.rotation[0] < right.rotation[0] - 0.8,
        "WHACK should chop the attack arm while NONE keeps the prologue-only pitch"
    );
}

#[test]
fn player_with_a_spear_lunges_instead_of_whacking() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` `STAB` (`SpearAnimations.thirdPersonAttackHand`): the
    // shared body twist + arm anchors run, but the attacking arm LUNGES on its pitch
    // (`xRot += (90·prepare − 120·attack + 30·retract)·π/180`) instead of whacking, and the body-twist
    // contributions on the arm rotations are undone (the off arm keeps its resting pitch).
    let t = 0.1_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let progress = |t: f32, a: f32, b: f32| ((t - a) / (b - a)).clamp(0.0, 1.0);
    let in_out_sine = |x: f32| -((PI * x).cos() - 1.0) / 2.0;
    let in_out_expo = |x: f32| {
        if x < 0.5 {
            if x == 0.0 {
                0.0
            } else {
                2.0_f32.powf(20.0 * x - 10.0) / 2.0
            }
        } else if x == 1.0 {
            1.0
        } else {
            (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
        }
    };
    let stab = {
        let prepare = in_out_sine(progress(t, 0.0, 0.05));
        let attack = progress(t, 0.05, 0.2).powi(2);
        let retract = in_out_expo(progress(t, 0.4, 1.0));
        (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians()
    };

    let base =
        EntityModelInstance::player(905, [0.0, 64.0, 0.0], 0.0, false).with_head_look(0.0, -10.0);
    let mut resting = PlayerModel::new(false);
    resting.prepare(&base);
    let resting_left_xrot = resting.root_mut().child_mut("left_arm").pose.rotation[0];

    let spear_base = base.with_player_main_hand_spear_pose(true);
    let mut spear_held = PlayerModel::new(false);
    spear_held.prepare(&spear_base);
    let spear_base_right_xrot = spear_held.root_mut().child_mut("right_arm").pose.rotation[0];

    // A spear-swing first points the held spear along the head look, twists the body the same way the whack
    // does, then lunges the right arm from that SPEAR base pose.
    let spear = spear_base
        .with_attack_anim(t)
        .with_main_hand_swing_is_stab(true);
    let mut model = PlayerModel::new(false);
    model.prepare(&spear);
    assert!(
        (model.root_mut().child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body still twists with the swing"
    );
    let right = model.root_mut().child_mut("right_arm");
    let right_offset0 = right.pose.offset[0];
    let right_xrot = right.pose.rotation[0];
    assert!((right_offset0 - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!(
        (right_xrot - (spear_base_right_xrot + stab)).abs() < 1e-6,
        "the main arm lunges from the held-spear pose: {right_xrot} vs {}",
        spear_base_right_xrot + stab
    );

    // Unlike the whack, the stab undoes the prologue's body-twist add on the off arm's pitch — it stays
    // at its resting pitch.
    let left_xrot = model.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (left_xrot - resting_left_xrot).abs() < 1e-6,
        "the off arm keeps its resting pitch during a stab (no body-twist add)"
    );

    // The same swing time with a non-spear (WHACK) drives a visibly different attacking-arm pitch.
    let mut whack = PlayerModel::new(false);
    whack.prepare(&base.with_attack_anim(t));
    let whack_right_xrot = whack.root_mut().child_mut("right_arm").pose.rotation[0];
    assert!(
        (whack_right_xrot - right_xrot).abs() > 0.3,
        "the spear lunge differs from the whack chop: stab {right_xrot} vs whack {whack_right_xrot}"
    );
}

#[test]
fn player_holding_a_spear_points_the_arm_along_head_look() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` for a held spear even when the player is not using
    // it. `SpearAnimations.thirdPersonHandUse` still applies the head-look base pose, but with
    // `ticksUsingItem <= 0` it skips all kinetic sway.
    let yaw = 25.0_f32;
    let pitch = -15.0_f32;
    let base =
        EntityModelInstance::player(906, [0.0, 64.0, 0.0], 0.0, false).with_head_look(yaw, pitch);
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    let idle_right_zrot = idle.root_mut().child_mut("right_arm").pose.rotation[2];
    let expected_x = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_main_hand_spear_pose(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    let expected_right_y = (-0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    assert!((right.rotation[0] - expected_x).abs() < 1e-6);
    assert!((right.rotation[1] - expected_right_y).abs() < 1e-6);
    assert_eq!(
        right.rotation[2], idle_right_zrot,
        "held spear keeps only the folded-in idle bob roll, with no kinetic roll sway"
    );

    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_off_hand_spear_pose(true));
    let left = off.root_mut().child_mut("left_arm").pose;
    let expected_left_y = (0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    assert!((left.rotation[0] - expected_x).abs() < 1e-6);
    assert!((left.rotation[1] - expected_left_y).abs() < 1e-6);
}

#[test]
fn player_using_a_spear_applies_kinetic_hand_sway() {
    // Vanilla `SpearAnimations.thirdPersonHandUse`: the SPEAR arm pose points along the head look first,
    // then kinetic weapon timing adds y/z sway and the raise/lower/back pitch terms from `ticksUsingItem`.
    let kinetic = SpearKineticWeapon {
        delay_ticks: 12.0,
        dismount_duration_ticks: 50.0,
        knockback_duration_ticks: 135.0,
        damage_duration_ticks: 225.0,
        forward_movement: 0.38,
    };
    let ticks = 60.0;
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let base = EntityModelInstance::player(906, [0.0, 64.0, 0.0], 0.0, false)
        .with_head_look(yaw, pitch)
        .with_player_using_spear(Some(kinetic));

    let mut steady = PlayerModel::new(false);
    steady.prepare(&base);
    let steady_right = steady.root_mut().child_mut("right_arm").pose;
    let expected_y = (-0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    let expected_x = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    assert!((steady_right.rotation[0] - expected_x).abs() < 1e-6);
    assert!((steady_right.rotation[1] - expected_y).abs() < 1e-6);

    let mut swaying = PlayerModel::new(false);
    swaying.prepare(&base.with_crossbow_charge_ticks(ticks));
    let swaying_right = swaying.root_mut().child_mut("right_arm").pose;
    let params = kinetic.use_params(ticks);
    let x_delta = (-40.0 * params.raise_progress_start
        + 30.0 * params.raise_progress_middle
        + -20.0 * params.raise_progress_end
        + 20.0 * params.lower_progress
        + 10.0 * params.raise_back_progress
        + 0.6 * params.sway_scale_slow * params.sway_intensity)
        .to_radians();
    let y_delta = -params.sway_scale_fast.to_radians() * params.sway_intensity;
    let z_delta = -params.sway_scale_slow.to_radians() * params.sway_intensity * 0.5;
    assert!((swaying_right.rotation[0] - (steady_right.rotation[0] + x_delta)).abs() < 1e-6);
    assert!((swaying_right.rotation[1] - (steady_right.rotation[1] + y_delta)).abs() < 1e-6);
    assert!((swaying_right.rotation[2] - (steady_right.rotation[2] + z_delta)).abs() < 1e-6);

    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_use_item_off_hand(true)
            .with_crossbow_charge_ticks(ticks),
    );
    let off_left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (off_left.rotation[1] - (0.1 + yaw.to_radians() - y_delta)).abs() < 1e-6,
        "off-hand spear mirrors the kinetic yaw sway"
    );
}

#[test]
fn player_using_a_spyglass_raises_it_to_the_eye() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `SPYGLASS`: the holding arm raises along the
    // head look — `xRot = clamp(head.xRot − 1.9198622 − (crouch?π/12), −2.4, 3.3)`, `yRot = head.yRot ∓
    // π/12` — and skips the idle bob (`zRot` back to bind). Applied before crouch, so a crouching player's
    // `arm.xRot += 0.4` still lands on top.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base =
        EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, false).with_head_look(yaw, pitch);

    // Main-hand spyglass raises the RIGHT arm and removes its bob roll.
    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_using_spyglass(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the right arm pitches up to the eye: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 12.0)).abs() < 1e-6,
        "the right arm yaws −π/12 off the head: {}",
        right.rotation[1]
    );
    assert_eq!(
        right.rotation[2], 0.0,
        "the spyglass arm skips the idle bob"
    );

    // Off-hand spyglass raises the LEFT arm with the mirrored yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_using_spyglass(true)
            .with_use_item_off_hand(true),
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the left arm pitches up to the eye: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 12.0)).abs() < 1e-6,
        "the left arm yaws +π/12 off the head"
    );

    // Crouching adds the crouch term to the clamp AND the crouch block's +0.4 on top (pose runs first).
    let mut crouch = PlayerModel::new(false);
    crouch.prepare(
        &base
            .with_player_using_spyglass(true)
            .with_is_crouching(true),
    );
    let crouch_right_x = crouch.root_mut().child_mut("right_arm").pose.rotation[0];
    let expected = (pitch_rad - 1.9198622 - PI / 12.0).clamp(-2.4, 3.3) + 0.4;
    assert!(
        (crouch_right_x - expected).abs() < 1e-6,
        "a crouching spyglass arm clamps with the crouch term, then gains +0.4: {crouch_right_x} vs {expected}"
    );
}

#[test]
fn player_tooting_a_goat_horn_raises_it_to_the_mouth() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `TOOT_HORN`: the holding arm raises the horn —
    // `xRot = clamp(head.xRot, −1.2, 1.2) − 1.4835298`, `yRot = head.yRot ∓ π/6`. Unlike the spyglass it
    // keeps the idle bob, so the arm roll (`zRot`) is left non-zero.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base =
        EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, false).with_head_look(yaw, pitch);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_tooting_horn(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad.clamp(-1.2, 1.2) - 1.4835298)).abs() < 1e-6,
        "the right arm raises the horn to the mouth: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 6.0)).abs() < 1e-6,
        "the right arm yaws −π/6 off the head: {}",
        right.rotation[1]
    );
    assert_ne!(
        right.rotation[2], 0.0,
        "the horn arm keeps the idle bob roll (unlike the spyglass)"
    );

    // Off-hand horn raises the LEFT arm with the mirrored yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_tooting_horn(true)
            .with_use_item_off_hand(true),
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 6.0)).abs() < 1e-6,
        "the left arm yaws +π/6 off the head"
    );
}

#[test]
fn player_brushing_lowers_the_arm_to_the_block() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `BRUSH`: the holding arm lowers — `xRot =
    // arm.xRot · 0.5 − π/5`, `yRot = 0`. At rest (age 0, not walking) the arm pitch is `0`, so the brush
    // pose lands the arm at exactly `−π/5`.
    let base =
        EntityModelInstance::player(922, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_brushing(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the right arm lowers to −π/5: {}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the brush pose zeroes the arm yaw");

    // A non-brushing player keeps its (much higher) idle arm pitch — the brush visibly lowers the arm.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI / 5.0 + 0.3,
        "an idle player does not lower the arm to the block"
    );

    // Off-hand brushing lowers the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_brushing(true).with_use_item_off_hand(true));
    assert!(
        (off.root_mut().child_mut("left_arm").pose.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the off hand lowers the left arm"
    );
}

#[test]
fn player_holding_an_item_lowers_the_main_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` (`HumanoidModel.poseRightArm` ITEM case): a player
    // holding a plain main-hand item lowers/halves the arm — `xRot = arm.xRot · 0.5 − π/10`, `yRot = 0`. At
    // rest (age 0, not walking) the arm pitch is `0`, so the ITEM pose lands the right arm at exactly `−π/10`.
    let base =
        EntityModelInstance::player(930, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut held = PlayerModel::new(false);
    held.prepare(&base.with_player_main_hand_item_pose(true));
    let right = held.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 10.0)).abs() < 1e-6,
        "the right arm lowers to −π/10: {}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the ITEM pose zeroes the arm yaw");

    // An empty-handed player keeps its (much higher) idle arm pitch — holding an item visibly lowers it.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI / 10.0 + 0.3,
        "an empty-handed player does not lower the arm"
    );

    // The main-hand ITEM pose is right-arm only; the left arm is untouched.
    let mut held_left = PlayerModel::new(false);
    held_left.prepare(&base.with_player_main_hand_item_pose(true));
    let left = held_left.root_mut().child_mut("left_arm").pose;
    let mut bare_left = PlayerModel::new(false);
    bare_left.prepare(&base);
    assert_eq!(
        left.rotation,
        bare_left.root_mut().child_mut("left_arm").pose.rotation,
        "the main-hand ITEM pose leaves the off (left) arm alone"
    );
}

#[test]
fn player_holding_an_off_hand_item_lowers_the_left_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` (`HumanoidModel.poseLeftArm` ITEM
    // case): a player holding a plain off-hand item lowers/halves the OFF (left) arm — `xRot = arm.xRot ·
    // 0.5 − π/10`, `yRot = 0`. At rest the arm pitch is `0`, so the left arm lands at exactly `−π/10`.
    let base =
        EntityModelInstance::player(940, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut held = PlayerModel::new(false);
    held.prepare(&base.with_player_off_hand_item_pose(true));
    let left = held.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 10.0)).abs() < 1e-6,
        "the left arm lowers to −π/10: {}",
        left.rotation[0]
    );
    assert_eq!(
        left.rotation[1], 0.0,
        "the off-hand ITEM pose zeroes the arm yaw"
    );

    // The off-hand ITEM pose is left-arm only; the right (main) arm is untouched.
    let right = held.root_mut().child_mut("right_arm").pose;
    let mut bare = PlayerModel::new(false);
    bare.prepare(&base);
    assert_eq!(
        right.rotation,
        bare.root_mut().child_mut("right_arm").pose.rotation,
        "the off-hand ITEM pose leaves the main (right) arm alone"
    );
}

#[test]
fn player_raising_a_shield_tucks_the_arm_along_the_head_look() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseBlockingArm` `BLOCK`: the raising arm tucks the shield forward —
    // `xRot = arm.xRot · 0.5 − 0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)`,
    // `yRot = (right ? −π/6 : π/6) + clamp(head.yRot, −π/6, π/6)`. With a head look of `(yaw 10°, pitch −10°)`
    // both clamps pass through, and at rest (age 0) the arm pitch is `0`.
    let yaw = 10.0_f32.to_radians();
    let pitch = (-10.0_f32).to_radians();
    let base =
        EntityModelInstance::player(950, [0.0, 64.0, 0.0], 0.0, false).with_head_look(10.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_blocking(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-0.9424779 + pitch)).abs() < 1e-5,
        "the right arm pitch tucks to −0.9424779 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-PI / 6.0 + yaw)).abs() < 1e-5,
        "the right arm yaws −π/6 + head.yRot: {}",
        right.rotation[1]
    );

    // An idle (non-blocking) player keeps its much higher arm pitch — the shield visibly tucks the arm down.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -0.9424779 + pitch + 0.3,
        "an idle player does not tuck the arm"
    );

    // Off-hand blocking tucks the LEFT arm instead, with the +π/6 base yaw.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_blocking(true).with_use_item_off_hand(true));
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (PI / 6.0 + yaw)).abs() < 1e-5,
        "the off hand yaws +π/6 + head.yRot: {}",
        left.rotation[1]
    );
}

#[test]
fn player_charging_a_trident_raises_the_arm_overhead() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `THROW_TRIDENT`: the holding arm raises the trident
    // straight overhead — `xRot = arm.xRot · 0.5 − π`, `yRot = 0`. At rest (age 0) the arm pitch is `0`, so
    // it lands at exactly `−π`.
    let base =
        EntityModelInstance::player(960, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut main = PlayerModel::new(false);
    main.prepare(&base.with_player_throwing_trident(true));
    let right = main.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI)).abs() < 1e-6,
        "the right arm raises to −π: {}",
        right.rotation[0]
    );
    assert_eq!(
        right.rotation[1], 0.0,
        "the THROW_TRIDENT pose zeroes the arm yaw"
    );

    // An idle player keeps its much higher arm pitch — charging the throw visibly raises the arm overhead.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI + 0.5,
        "an idle player does not raise the arm overhead"
    );

    // Off-hand charging raises the LEFT arm instead.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_throwing_trident(true)
            .with_use_item_off_hand(true),
    );
    assert!(
        (off.root_mut().child_mut("left_arm").pose.rotation[0] - (-PI)).abs() < 1e-6,
        "the off hand raises the left arm overhead"
    );
}

#[test]
fn player_drawing_a_bow_raises_both_arms_along_the_head_look() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm` `BOW_AND_ARROW`: both arms raise along the head look —
    // `rightArm.xRot = leftArm.xRot = −π/2 + head.xRot`, `rightArm.yRot = −0.1 + head.yRot`,
    // `leftArm.yRot = 0.1 + head.yRot + 0.4`. These are SET, overwriting the walk pitch/yaw.
    let yaw = 15.0_f32.to_radians();
    let pitch = (-20.0_f32).to_radians();
    let base = EntityModelInstance::player(970, [0.0, 64.0, 0.0], 0.0, false)
        .with_head_look(15.0, -20.0)
        .with_walk_animation(0.0, 1.0);

    let mut drawing = PlayerModel::new(false);
    drawing.prepare(&base.with_player_drawing_bow(true));
    let right = drawing.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the right arm raises to −π/2 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.1 + yaw)).abs() < 1e-5,
        "the right arm yaws −0.1 + head.yRot: {}",
        right.rotation[1]
    );
    let left = drawing.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the left arm raises to −π/2 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.1 + yaw + 0.4)).abs() < 1e-5,
        "the left arm yaws 0.1 + head.yRot + 0.4: {}",
        left.rotation[1]
    );

    // Vanilla `HumanoidModel.poseLeftArm` mirrors the brace offset for off-hand bow draw.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_drawing_bow(true)
            .with_use_item_off_hand(true),
    );
    let right = off.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the off-hand bow still raises the right arm to −π/2 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.1 + yaw - 0.4)).abs() < 1e-5,
        "the off-hand bow yaws the right arm −0.1 + head.yRot − 0.4: {}",
        right.rotation[1]
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the off-hand bow raises the left arm to −π/2 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.1 + yaw)).abs() < 1e-5,
        "the off-hand bow yaws the left arm 0.1 + head.yRot: {}",
        left.rotation[1]
    );

    // A non-drawing walking player keeps its swinging (much lower) arm pitch — the bow visibly raises both.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[0] > -PI / 2.0 + pitch + 0.5,
        "a non-drawing player does not raise the arm to the bow stance"
    );
}

#[test]
fn player_charging_a_crossbow_braces_and_draws_the_string() {
    use std::f32::consts::FRAC_PI_2;

    // Vanilla `AnimationUtils.animateCrossbowCharge` (right-handed): the right arm braces the crossbow
    // (`yRot = −0.8`, `xRot = −0.97079635`) while the left arm pulls the string back, lerping its `xRot`
    // `−0.97079635 → −π/2` and `yRot` `0.4 → 0.85` over `ticksUsingItem / 25`. At half draw (12.5 / 25 =
    // 0.5) the left arm is halfway through that lerp. Reuses the same helper as the pillager/piglin.
    const HOLD_X: f32 = -0.97079635;
    let base = EntityModelInstance::player(980, [0.0, 64.0, 0.0], 0.0, false)
        .with_head_look(15.0, -10.0)
        .with_crossbow_charge_ticks(12.5);

    let mut drawing = PlayerModel::new(false);
    drawing.prepare(&base.with_player_charging_crossbow(true));
    let right = drawing.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[1] - (-0.8)).abs() < 1e-6,
        "the right arm braces at yRot −0.8: {}",
        right.rotation[1]
    );
    assert!(
        (right.rotation[0] - HOLD_X).abs() < 1e-6,
        "the right arm braces at xRot −0.97079635: {}",
        right.rotation[0]
    );
    let left = drawing.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (0.4 + (0.85 - 0.4) * 0.5)).abs() < 1e-6,
        "the left arm yaws halfway through the draw: {}",
        left.rotation[1]
    );
    assert!(
        (left.rotation[0] - (HOLD_X + (-FRAC_PI_2 - HOLD_X) * 0.5)).abs() < 1e-6,
        "the left arm pitches halfway through the string pull: {}",
        left.rotation[0]
    );

    // Vanilla `holdingInRightArm = false` mirrors the brace into the left arm and makes the right arm pull.
    let mut off = PlayerModel::new(false);
    off.prepare(
        &base
            .with_player_charging_crossbow(true)
            .with_use_item_off_hand(true),
    );
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - 0.8).abs() < 1e-6,
        "the off-hand crossbow braces the left arm at yRot 0.8: {}",
        left.rotation[1]
    );
    assert!(
        (left.rotation[0] - HOLD_X).abs() < 1e-6,
        "the off-hand crossbow braces the left arm at xRot −0.97079635: {}",
        left.rotation[0]
    );
    let right = off.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[1] - (-(0.4 + (0.85 - 0.4) * 0.5))).abs() < 1e-6,
        "the off-hand crossbow yaws the right arm halfway through the mirrored draw: {}",
        right.rotation[1]
    );
    assert!(
        (right.rotation[0] - (HOLD_X + (-FRAC_PI_2 - HOLD_X) * 0.5)).abs() < 1e-6,
        "the off-hand crossbow pitches the right arm halfway through the string pull: {}",
        right.rotation[0]
    );

    // A non-charging player does not brace — the right arm keeps its (much higher) idle pitch.
    let mut idle = PlayerModel::new(false);
    idle.prepare(&base);
    assert!(
        idle.root_mut().child_mut("right_arm").pose.rotation[1] > -0.8 + 0.3,
        "an idle player does not brace the crossbow"
    );
}

#[test]
fn player_holding_a_charged_crossbow_levels_it_along_the_head_look() {
    use std::f32::consts::FRAC_PI_2;

    // Vanilla `AnimationUtils.animateCrossbowHold` (right-handed): the right arm levels the crossbow
    // (`xRot = −π/2 + head.xRot + 0.1`, `yRot = −0.3 + head.yRot`) while the left reaches the trigger
    // (`xRot = −1.5 + head.xRot`, `yRot = 0.6 + head.yRot`). Reuses the same helper as the pillager.
    let yaw = 20.0_f32.to_radians();
    let pitch = (-10.0_f32).to_radians();
    let base =
        EntityModelInstance::player(990, [0.0, 64.0, 0.0], 0.0, false).with_head_look(20.0, -10.0);

    let mut holding = PlayerModel::new(false);
    holding.prepare(&base.with_player_crossbow_hold(true));
    let right = holding.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6,
        "the right arm levels at −π/2 + head.xRot + 0.1: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.3 + yaw)).abs() < 1e-6,
        "the right arm yaws −0.3 + head.yRot: {}",
        right.rotation[1]
    );
    let left = holding.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-1.5 + pitch)).abs() < 1e-6,
        "the left arm reaches the trigger at −1.5 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.6 + yaw)).abs() < 1e-6,
        "the left arm yaws 0.6 + head.yRot: {}",
        left.rotation[1]
    );

    // Vanilla `AvatarRenderer.getArmPose(avatar, arm)` forces a non-empty off hand to ITEM when the
    // main-hand pose is the two-handed `CROSSBOW_HOLD`. `HumanoidModel.setupAnim` applies that off-hand pose
    // first and then overwrites both arms with the main-hand hold, so an off-hand SPEAR must not survive.
    let mut holding_over_spear = PlayerModel::new(false);
    holding_over_spear.prepare(
        &base
            .with_player_off_hand_spear_pose(true)
            .with_player_crossbow_hold(true),
    );
    let right = holding_over_spear.root_mut().child_mut("right_arm").pose;
    assert!((right.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6);
    assert!((right.rotation[1] - (-0.3 + yaw)).abs() < 1e-6);
    let left = holding_over_spear.root_mut().child_mut("left_arm").pose;
    assert!((left.rotation[0] - (-1.5 + pitch)).abs() < 1e-6);
    assert!((left.rotation[1] - (0.6 + yaw)).abs() < 1e-6);

    // Vanilla `holdingInRightArm = false` mirrors `CROSSBOW_HOLD` for a charged off-hand crossbow.
    let mut off = PlayerModel::new(false);
    off.prepare(&base.with_player_crossbow_hold_off_hand(true));
    let left = off.root_mut().child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6,
        "the off-hand crossbow levels the left arm at −π/2 + head.xRot + 0.1: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.3 + yaw)).abs() < 1e-6,
        "the off-hand crossbow yaws the left arm 0.3 + head.yRot: {}",
        left.rotation[1]
    );
    let right = off.root_mut().child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-1.5 + pitch)).abs() < 1e-6,
        "the off-hand crossbow reaches the right arm to −1.5 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.6 + yaw)).abs() < 1e-6,
        "the off-hand crossbow yaws the right arm −0.6 + head.yRot: {}",
        right.rotation[1]
    );
}

#[test]
fn player_swings_its_legs_when_walking() {
    // `PlayerModel extends HumanoidModel` and its `setupAnim` only toggles part
    // visibility before `super.setupAnim`, so a remote player inherits the
    // `HumanoidModel` legs unchanged (legs at [4, 5], the right leg in phase). A
    // standing player is inert; a walking one lifts its feet and splays its legs
    // along Z, for both the wide and slim arm models. This is the colored path.
    for slim in [false, true] {
        let base = EntityModelInstance::player(910, [0.0, 64.0, 0.0], 0.0, slim);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "slim={slim}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "slim={slim}: walking differs"
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "slim={slim}: a walking player's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.1,
            "slim={slim}: a walking player's legs should splay along Z"
        );
    }
}

#[test]
fn player_textured_mesh_swings_legs_when_walking() {
    // The real player render path (texture-backed) swings the inherited
    // `HumanoidModel` legs (and the pants children that ride them) on the shared
    // visibility-filtered part array. A standing player is byte-identical however
    // far the swing position has advanced; a walking one lifts its feet. Checked
    // with all model parts visible (pants present) and with the pants hidden.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let no_pants = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::ALL_MASK
            & !PlayerModelPartVisibility::LEFT_PANTS_MASK
            & !PlayerModelPartVisibility::RIGHT_PANTS_MASK,
    );
    for slim in [false, true] {
        for (label, base) in [
            (
                "all_parts",
                EntityModelInstance::player(911, [0.0, 64.0, 0.0], 0.0, slim),
            ),
            (
                "no_pants",
                EntityModelInstance::player_with_parts(912, [0.0, 64.0, 0.0], 0.0, slim, no_pants),
            ),
        ] {
            let still_instance = base.with_walk_animation(2.5, 0.0);
            let walking_instance = base.with_walk_animation(0.0, 1.0);
            let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
            let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
            let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
            assert_player_submissions_match_vanilla(&resting_meshes, base);
            assert_player_submissions_match_vanilla(&still_meshes, still_instance);
            assert_player_submissions_match_vanilla(&walking_meshes, walking_instance);
            let resting = &resting_meshes.cutout;
            let still = &still_meshes.cutout;
            let walking = &walking_meshes.cutout;

            assert_eq!(
                resting.vertices, still.vertices,
                "slim={slim} {label}: a standing player is inert"
            );
            assert_eq!(
                resting.vertices.len(),
                walking.vertices.len(),
                "slim={slim} {label}: leg swing keeps the vertex count"
            );
            assert_ne!(
                resting.vertices, walking.vertices,
                "slim={slim} {label}: a walking player differs"
            );

            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
                "slim={slim} {label}: a walking player's feet should lift off the ground"
            );
        }
    }
}

#[test]
fn player_swings_its_arms_when_walking() {
    // `PlayerModel` inherits the `HumanoidModel` arm swing unchanged. With all overlays
    // visible the colored mesh emits head+body (verts 0..96), the two arms with their
    // sleeves (96..192), then the two legs with their pants (192..288). A standing player
    // is inert; a walking one moves the arms (and legs) while the head and body stay put
    // (no head look here). The held-item/attack arm poses and the idle bob are deferred.
    for slim in [false, true] {
        let base = EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, slim);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(rest.vertices.len(), 288, "slim={slim}");
        assert_eq!(rest.vertices.len(), walking.vertices.len(), "slim={slim}");
        assert_eq!(
            rest.vertices[0..96],
            walking.vertices[0..96],
            "slim={slim}: head and body stay put while walking"
        );
        assert_ne!(
            rest.vertices[96..192],
            walking.vertices[96..192],
            "slim={slim}: the arms swing"
        );
    }
}

#[test]
fn player_textured_mesh_swings_its_arms_when_walking() {
    // The real (texture-backed) player render path swings the arms on the same shared
    // visibility-filtered part array as the legs; the sleeve children ride the arm parts.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
        let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_player_submissions_match_vanilla(&resting_meshes, base);
        assert_player_submissions_match_vanilla(&still_meshes, still_instance);
        assert_player_submissions_match_vanilla(&walking_meshes, walking_instance);
        let resting = &resting_meshes.cutout;
        let still = &still_meshes.cutout;
        let walking = &walking_meshes.cutout;
        assert_eq!(
            resting.vertices, still.vertices,
            "slim={slim}: inert when standing"
        );
        assert_eq!(resting.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            resting.vertices[0..96],
            walking.vertices[0..96],
            "slim={slim}: head and body stay put"
        );
        assert_ne!(
            resting.vertices[96..192],
            walking.vertices[96..192],
            "slim={slim}: the arms swing"
        );
    }
}

#[test]
fn humanoid_arm_swing_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim: rightArm.xRot = cos(pos*0.6662 + π)*2.0*speed*0.5,
    // leftArm.xRot = cos(pos*0.6662)*2.0*speed*0.5 (amplitude 1.0). The right arm
    // (offset x < 0) is the out-of-phase one, opposite the same-side leg. Only xRot
    // moves. The right arm rests at x = -5, the left at x = +5.
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    let right = humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, speed);
    let left = humanoid_arm_swing_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, pos, speed);
    assert!(
        (right.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "right arm out of phase"
    );
    assert!(
        (left.rotation[0] - phase.cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "left arm in phase"
    );
    // The arm swing is the opposite phase to the same-side leg (right arm uses +π, the
    // right leg uses none) and a shorter amplitude (1.0 vs 1.4).
    let right_leg = humanoid_leg_swing_pose(PLAYER_FIXTURE_RIGHT_LEG_POSE, pos, speed);
    assert!(
        (right.rotation[0] + right_leg.rotation[0] / 1.4).abs() < 1e-6,
        "right arm is the negated, scaled right-leg swing"
    );
    // Only xRot changes.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    assert_eq!(right.rotation[2], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2]);
    // At rest (speed 0) the arms hold their body-layer pose.
    assert_eq!(
        humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, 0.0),
        PLAYER_FIXTURE_RIGHT_ARM_POSE
    );
}

#[test]
fn humanoid_arm_bob_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim applies AnimationUtils.bobModelPart to both arms every
    // frame — bobModelPart(rightArm, age, 1.0), bobModelPart(leftArm, age, -1.0):
    //   arm.zRot += scale * (cos(age * 0.09)  * 0.05 + 0.05)
    //   arm.xRot += scale * (sin(age * 0.067) * 0.05)
    // The right arm rests at x = -5 (bob scale +1), the left at x = +5 (scale -1); the bob
    // accumulates onto the arm's rest pose.
    let age = 27.3_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let right = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, age);
    let left = humanoid_arm_bob_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, age);
    assert!(
        (right.rotation[0] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0] + bob_x)).abs() < 1e-6,
        "right arm bob xRot"
    );
    assert!(
        (right.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + bob_z)).abs() < 1e-6,
        "right arm bob zRot"
    );
    // The left arm uses the opposite sign (scale -1).
    assert!(
        (left.rotation[0] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[0] - bob_x)).abs() < 1e-6,
        "left arm bob xRot mirrored"
    );
    assert!(
        (left.rotation[2] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[2] - bob_z)).abs() < 1e-6,
        "left arm bob zRot mirrored"
    );
    // The bob preserves the offset and yRot.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    // The xRot term vanishes at age 0 (sin 0 = 0) but the zRot baseline does not
    // (cos 0 = 1 gives ±0.1), so the arms never sit at the bare rest pose.
    let at_zero = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, 0.0);
    assert!((at_zero.rotation[0] - PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0]).abs() < 1e-6);
    assert!((at_zero.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + 0.1)).abs() < 1e-6);
}

#[test]
fn player_arms_idle_bob_as_age_advances_even_when_standing() {
    // The HumanoidModel idle arm bob advances every frame regardless of walking, so a
    // standing player's arms move with ageInTicks while the head, body, and legs stay put
    // (no walk swing, no head look). The colored mesh emits head+body (0..96), the two arms
    // with sleeves (96..192), then the two legs with pants (192..288).
    for slim in [false, true] {
        let base = EntityModelInstance::player(930, [0.0, 64.0, 0.0], 0.0, slim);
        let early = entity_model_mesh(&[base]);
        let later = entity_model_mesh(&[base.with_age_in_ticks(27.3)]);
        assert_eq!(early.vertices.len(), 288, "slim={slim}");
        assert_eq!(early.vertices.len(), later.vertices.len(), "slim={slim}");
        assert_eq!(
            early.vertices[0..96],
            later.vertices[0..96],
            "slim={slim}: head and body do not bob"
        );
        assert_ne!(
            early.vertices[96..192],
            later.vertices[96..192],
            "slim={slim}: the arms idle-bob with ageInTicks"
        );
        assert_eq!(
            early.vertices[192..288],
            later.vertices[192..288],
            "slim={slim}: the legs do not bob"
        );
    }
}

#[test]
fn player_textured_arms_idle_bob_as_age_advances() {
    // The texture-backed player path applies the same idle bob on the shared
    // visibility-filtered part array, so a standing player's arms bob with ageInTicks while
    // the head, body, and legs are byte-identical across ages.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(931, [0.0, 64.0, 0.0], 0.0, slim);
        let later_instance = base.with_age_in_ticks(27.3);
        let early_meshes = entity_model_textured_meshes(&[base], &atlas);
        let later_meshes = entity_model_textured_meshes(&[later_instance], &atlas);
        assert_player_submissions_match_vanilla(&early_meshes, base);
        assert_player_submissions_match_vanilla(&later_meshes, later_instance);
        let early = &early_meshes.cutout;
        let later = &later_meshes.cutout;
        assert_eq!(early.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            early.vertices[0..96],
            later.vertices[0..96],
            "slim={slim}: head and body do not bob"
        );
        assert_ne!(
            early.vertices[96..192],
            later.vertices[96..192],
            "slim={slim}: the arms idle-bob with ageInTicks"
        );
        assert_eq!(
            early.vertices[192..288],
            later.vertices[192..288],
            "slim={slim}: the legs do not bob"
        );
    }
}

#[test]
fn player_crouches_when_sneaking() {
    // Vanilla `HumanoidModel.setupAnim` crouch (`isCrouching`): the body leans forward
    // (`xRot = 0.5`) and drops (`y += 3.2`), the head drops (`y += 4.2`), the arms tilt
    // (`xRot += 0.4`, `y += 3.2`) and the legs tuck back (`z += 4`). A sneaking player is
    // shorter and leans forward. Colored path here, textured below.
    for slim in [false, true] {
        let base = EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, slim);
        let standing = entity_model_mesh(&[base]);
        let crouching = entity_model_mesh(&[base.with_is_crouching(true)]);
        assert_eq!(standing.vertices.len(), crouching.vertices.len());
        assert_ne!(
            standing.vertices, crouching.vertices,
            "slim={slim}: a sneaking player poses differently"
        );

        let (stand_min, stand_max) = mesh_extents(&standing);
        let (crouch_min, crouch_max) = mesh_extents(&crouching);
        assert!(
            (crouch_max[1] - crouch_min[1]) < (stand_max[1] - stand_min[1]) - 0.1,
            "slim={slim}: a sneaking player is shorter"
        );
        assert!(
            (crouch_max[2] - crouch_min[2]) > (stand_max[2] - stand_min[2]) + 0.1,
            "slim={slim}: a sneaking player leans forward, deepening its Z footprint"
        );

        // A standing player is unaffected by the flag default.
        assert_eq!(
            standing.vertices,
            entity_model_mesh(&[base.with_is_crouching(false)]).vertices,
            "slim={slim}: a standing player is unchanged"
        );
    }
}

#[test]
fn player_textured_mesh_crouches_when_sneaking() {
    // The real player render path (texture-backed) applies the same crouch to the shared
    // visibility-filtered part array (the hat/jacket/sleeve/pants children ride the shifted
    // parts).
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let crouching_instance = base.with_is_crouching(true);
        let standing_meshes = entity_model_textured_meshes(&[base], &atlas);
        let crouching_meshes = entity_model_textured_meshes(&[crouching_instance], &atlas);
        assert_player_submissions_match_vanilla(&standing_meshes, base);
        assert_player_submissions_match_vanilla(&crouching_meshes, crouching_instance);
        let standing = &standing_meshes.cutout;
        let crouching = &crouching_meshes.cutout;
        assert_eq!(standing.vertices.len(), crouching.vertices.len());
        assert_ne!(
            standing.vertices, crouching.vertices,
            "slim={slim}: a sneaking player poses differently"
        );

        let (stand_min, stand_max) = textured_mesh_extents(&standing);
        let (crouch_min, crouch_max) = textured_mesh_extents(&crouching);
        assert!(
            (crouch_max[1] - crouch_min[1]) < (stand_max[1] - stand_min[1]) - 0.1,
            "slim={slim}: a sneaking player is shorter"
        );
        assert!(
            (crouch_max[2] - crouch_min[2]) > (stand_max[2] - stand_min[2]) + 0.1,
            "slim={slim}: a sneaking player leans forward"
        );
    }
}

fn player_texture_images() -> Vec<EntityModelTextureImage> {
    player_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn steve_player_texture_images() -> Vec<EntityModelTextureImage> {
    [PLAYER_WIDE_STEVE_TEXTURE_REF, PLAYER_SLIM_STEVE_TEXTURE_REF]
        .into_iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![index as u8; len])
        })
        .collect()
}

fn steve_and_elytra_texture_images() -> Vec<EntityModelTextureImage> {
    [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn steve_and_riptide_texture_images() -> Vec<EntityModelTextureImage> {
    [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn steve_and_parrot_texture_images() -> Vec<EntityModelTextureImage> {
    [PLAYER_WIDE_STEVE_TEXTURE_REF, PLAYER_SLIM_STEVE_TEXTURE_REF]
        .into_iter()
        .chain(parrot_entity_texture_refs().iter().copied())
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_player_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    let EntityModelKind::Player { skin, parts } = instance.kind else {
        panic!("expected player instance");
    };
    assert_player_folded_meshes_are_static_cutout_only(meshes);
    let passes = player_textured_layer_passes(skin.is_slim(), parts);
    assert_eq!(meshes.submissions.len(), passes.len());
    assert_eq!(passes.len(), 1);
    let submit = meshes.submissions[0];
    let pass = passes[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.render_type, pass.render_type);
    assert_eq!(submit.texture, pass.texture);
    assert_eq!(submit.tint, pass.tint);
    assert_eq!(submit.transform, player_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert_eq!(
        (submit.order, submit.submit_sequence),
        (pass.order, pass.submit_sequence)
    );
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}

fn assert_player_folded_meshes_are_static_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn dynamic_player_texture_image(
    handle: u64,
    size: [u32; 2],
    base: u8,
) -> DynamicPlayerTextureImage {
    let mut rgba = Vec::new();
    for y in 0..size[1] {
        for x in 0..size[0] {
            rgba.extend_from_slice(&[base + x as u8, x as u8, y as u8, 255]);
        }
    }
    DynamicPlayerTextureImage { handle, size, rgba }
}

fn atlas_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = rgba_offset(width, y, x, "test atlas pixel").unwrap();
    [
        rgba[offset],
        rgba[offset + 1],
        rgba[offset + 2],
        rgba[offset + 3],
    ]
}
