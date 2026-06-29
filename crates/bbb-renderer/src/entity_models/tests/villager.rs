use super::*;

use crate::entity_models::colored::villager_adult_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. The hat/jacket deformed cubes inflate their geometry but keep the
    // base box as `uv_size`.
    assert_eq!(
        ADULT_VILLAGER_HAT[0],
        ModelCube::new(
            [-4.51, -10.51, -4.51],
            [9.02, 11.02, 9.02],
            VILLAGER_ROBE,
            [8.0, 10.0, 8.0],
            [32.0, 0.0],
            false,
        )
    );
    assert_eq!(
        ADULT_VILLAGER_JACKET[0],
        ModelCube::new(
            [-4.5, -0.5, -3.5],
            [9.0, 21.0, 7.0],
            VILLAGER_ROBE,
            [8.0, 20.0, 6.0],
            [0.0, 38.0],
            false,
        )
    );
    assert_eq!(ADULT_VILLAGER_HEAD[0].size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_VILLAGER_BODY[0].size, [8.0, 12.0, 6.0]);
    assert_eq!(ADULT_VILLAGER_ARMS[1].tex, [44.0, 22.0]);
    assert!(ADULT_VILLAGER_ARMS[1].mirror);
    assert_eq!(ADULT_VILLAGER_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert!(!ADULT_VILLAGER_RIGHT_LEG[0].mirror);
    assert!(ADULT_VILLAGER_LEFT_LEG[0].mirror);
}

#[test]
fn villager_adult_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875003, 64.00094, -0.46875006]);
    assert_close3(max, [0.46875003, 66.02301, 0.46875003]);

    let wandering_trader_mesh = entity_model_mesh(&[EntityModelInstance::wandering_trader(
        141,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(wandering_trader_mesh.opaque_faces, mesh.opaque_faces);
    assert_eq!(wandering_trader_mesh.vertices, mesh.vertices);
    assert_eq!(wandering_trader_mesh.indices, mesh.indices);
}

#[test]
fn villager_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_VILLAGER_RIGHT_HAND,
        [
            ModelCube::new(
                [-1.0, -2.4925, -1.8401],
                [2.0, 4.0, 2.0],
                VILLAGER_ROBE,
                [2.0, 4.0, 2.0],
                [36.0, 15.0],
                false,
            ),
            ModelCube::new(
                [5.0, -2.4925, -1.8401],
                [2.0, 4.0, 2.0],
                VILLAGER_ROBE,
                [2.0, 4.0, 2.0],
                [16.0, 15.0],
                false,
            ),
        ]
    );
    assert_eq!(
        BABY_VILLAGER_BB_MAIN[0],
        ModelCube::new(
            [-2.7, -8.2, -1.7],
            [4.4, 6.4, 3.4],
            VILLAGER_ROBE,
            [4.0, 6.0, 3.0],
            [16.0, 21.0],
            false,
        )
    );
    // The baby `createBodyModel` lists the parts in a different order (arms container, legs, head,
    // body, bb_main); the unified tree preserves that render order under the vanilla child names. The
    // baby head/body/leg cubes carry the textured UV sources.
    assert_eq!(BABY_VILLAGER_HEAD[0].size, [8.0, 8.0, 7.0]);
    assert_eq!(BABY_VILLAGER_BODY[0].size, [4.0, 5.0, 3.0]);
    assert_eq!(BABY_VILLAGER_RIGHT_LEG[0].tex, [8.0, 23.0]);
    assert_eq!(BABY_VILLAGER_LEFT_LEG[0].tex, [0.0, 23.0]);
    assert_eq!(
        BABY_VILLAGER_HAT[0],
        ModelCube::new(
            [-4.3, -4.3, -3.8],
            [8.6, 8.6, 7.6],
            VILLAGER_ROBE,
            [8.0, 8.0, 7.0],
            [0.0, 30.0],
            false,
        )
    );
}

#[test]
fn villager_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(max, [0.43750003, 65.01975, 0.37500003]);
}

#[test]
fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Villager { baby: false }.model_key(),
        "villager"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/wandering_trader/wandering_trader.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        villager_entity_texture_refs(),
        [
            EntityModelTextureRef {
                path: "textures/entity/villager/villager.png",
                size: [64, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/villager/villager_baby.png",
                size: [64, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/wandering_trader/wandering_trader.png",
                size: [64, 64],
            },
        ]
    );
    assert!(entity_model_texture_refs().contains(&VILLAGER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&VILLAGER_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WANDERING_TRADER_TEXTURE_REF));
}

#[test]
fn villager_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let adult = villager_textured_layer_passes(false);
    let baby = villager_textured_layer_passes(true);
    let trader = wandering_trader_textured_layer_passes();

    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::VillagerBase);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].model_layer, MODEL_LAYER_VILLAGER);
    assert_eq!(adult[0].texture, VILLAGER_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].order, adult[0].submit_sequence), (0, 0));

    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::VillagerBase);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].model_layer, MODEL_LAYER_VILLAGER_BABY);
    assert_eq!(baby[0].texture, VILLAGER_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].order, baby[0].submit_sequence), (0, 0));

    let farmer =
        VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Farmer, 4);
    let overlays = villager_data_textured_layer_passes(false, farmer);
    assert_eq!(overlays.len(), 3);
    assert_eq!(overlays[0].kind, EntityModelLayerKind::VillagerType);
    assert_eq!(overlays[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(overlays[0].model_layer, MODEL_LAYER_VILLAGER_NO_HAT);
    assert_eq!(
        overlays[0].texture,
        villager_type_texture_ref(VillagerModelType::Snow, false)
    );
    assert_eq!((overlays[0].order, overlays[0].submit_sequence), (1, 1));
    assert_eq!(overlays[1].kind, EntityModelLayerKind::VillagerProfession);
    assert_eq!(overlays[1].model_layer, MODEL_LAYER_VILLAGER);
    assert_eq!(
        overlays[1].texture,
        villager_profession_texture_ref(VillagerModelProfession::Farmer).unwrap()
    );
    assert_eq!((overlays[1].order, overlays[1].submit_sequence), (2, 2));
    assert_eq!(overlays[2].kind, EntityModelLayerKind::VillagerLevel);
    assert_eq!(overlays[2].model_layer, MODEL_LAYER_VILLAGER);
    assert_eq!(overlays[2].texture, villager_level_texture_ref(4));
    assert_eq!((overlays[2].order, overlays[2].submit_sequence), (3, 3));

    let baby_overlays = villager_data_textured_layer_passes(true, farmer);
    assert_eq!(baby_overlays.len(), 1);
    assert_eq!(baby_overlays[0].kind, EntityModelLayerKind::VillagerType);
    assert_eq!(
        baby_overlays[0].model_layer,
        MODEL_LAYER_VILLAGER_BABY_NO_HAT
    );
    assert_eq!(
        baby_overlays[0].texture,
        villager_type_texture_ref(VillagerModelType::Snow, true)
    );
    assert_eq!(
        (baby_overlays[0].order, baby_overlays[0].submit_sequence),
        (1, 1)
    );

    assert_eq!(trader.len(), 1);
    assert_eq!(trader[0].kind, EntityModelLayerKind::WanderingTraderBase);
    assert_eq!(
        trader[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    // Vanilla `WanderingTraderRenderer` bakes only `ModelLayers.WANDERING_TRADER`
    // and does not register a baby model/texture pair.
    assert_eq!(trader[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(trader[0].model_layer, MODEL_LAYER_WANDERING_TRADER);
    assert_eq!(trader[0].texture, WANDERING_TRADER_TEXTURE_REF);
    assert_eq!(trader[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(trader[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((trader[0].order, trader[0].submit_sequence), (0, 0));
}

#[test]
fn villager_textured_model_parts_match_vanilla_tex_offs_sources() {
    assert_eq!(MODEL_LAYER_VILLAGER, "minecraft:villager#main");
    assert_eq!(MODEL_LAYER_VILLAGER_BABY, "minecraft:villager_baby#main");
    assert_eq!(
        MODEL_LAYER_WANDERING_TRADER,
        "minecraft:wandering_trader#main"
    );
    assert_eq!(VILLAGER_TEXTURE_REF.size, [64, 64]);
    assert_eq!(VILLAGER_BABY_TEXTURE_REF.size, [64, 64]);
    assert_eq!(WANDERING_TRADER_TEXTURE_REF.size, [64, 64]);

    // The unified cubes carry the textured UV sources (`uv_size`/`texOffs`/`mirror`) merged into the
    // colored geometry.
    assert_eq!(ADULT_VILLAGER_HAT[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(ADULT_VILLAGER_HAT[0].tex, [32.0, 0.0]);
    assert_eq!(ADULT_VILLAGER_JACKET[0].uv_size, [8.0, 20.0, 6.0]);
    assert_eq!(ADULT_VILLAGER_JACKET[0].tex, [0.0, 38.0]);
    assert_eq!(ADULT_VILLAGER_ARMS[1].tex, [44.0, 22.0]);
    assert!(ADULT_VILLAGER_ARMS[1].mirror);
    assert!(ADULT_VILLAGER_LEFT_LEG[0].mirror);

    assert_eq!(
        BABY_VILLAGER_RIGHT_HAND,
        [
            ModelCube::new(
                [-1.0, -2.4925, -1.8401],
                [2.0, 4.0, 2.0],
                VILLAGER_ROBE,
                [2.0, 4.0, 2.0],
                [36.0, 15.0],
                false,
            ),
            ModelCube::new(
                [5.0, -2.4925, -1.8401],
                [2.0, 4.0, 2.0],
                VILLAGER_ROBE,
                [2.0, 4.0, 2.0],
                [16.0, 15.0],
                false,
            ),
        ]
    );
    assert_eq!(BABY_VILLAGER_RIGHT_LEG[0].tex, [8.0, 23.0]);
    assert_eq!(BABY_VILLAGER_LEFT_LEG[0].tex, [0.0, 23.0]);
    assert_eq!(BABY_VILLAGER_HAT[0].uv_size, [8.0, 8.0, 7.0]);
    assert_eq!(BABY_VILLAGER_HAT[0].tex, [0.0, 30.0]);
    assert_eq!(BABY_VILLAGER_BB_MAIN[0].uv_size, [4.0, 6.0, 3.0]);
    assert_eq!(BABY_VILLAGER_BB_MAIN[0].tex, [16.0, 21.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_villager_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/villager/villager.png",
            "textures/entity/villager/villager_baby.png",
            "textures/entity/wandering_trader/wandering_trader.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let third_slot = rgba_offset(layout.width, 128, 0, "villager test atlas").unwrap();
    assert_eq!(&rgba[third_slot..third_slot + 4], &[2; 4]);
}

#[test]
fn villager_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();
    let adult = villager_submission_probe(EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    ));
    let baby = villager_submission_probe(EntityModelInstance::villager(
        140,
        [2.0, 64.0, 0.0],
        0.0,
        true,
    ));
    let trader = villager_submission_probe(EntityModelInstance::wandering_trader(
        141,
        [4.0, 64.0, 0.0],
        0.0,
    ));

    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_villager_submissions_match_vanilla(&adult_meshes, adult);
    assert_eq!(adult_meshes.cutout.cutout_faces, 66);
    assert_eq!(adult_meshes.cutout.vertices.len(), 264);
    assert_eq!(adult_meshes.cutout.indices.len(), 396);
    assert_close2(adult_meshes.cutout.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(adult_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let adult_colored = entity_model_mesh(&[adult]);
    let (adult_expected_min, adult_expected_max) = mesh_extents(&adult_colored);
    let (adult_actual_min, adult_actual_max) = textured_mesh_extents(&adult_meshes.cutout);
    assert_close3(adult_actual_min, adult_expected_min);
    assert_close3(adult_actual_max, adult_expected_max);

    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_submissions_match_vanilla(&baby_meshes, baby);
    assert_eq!(baby_meshes.cutout.cutout_faces, 66);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    assert_eq!(baby_meshes.cutout.indices.len(), 396);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let baby_colored = entity_model_mesh(&[baby]);
    let (baby_expected_min, baby_expected_max) = mesh_extents(&baby_colored);
    let (baby_actual_min, baby_actual_max) = textured_mesh_extents(&baby_meshes.cutout);
    assert_close3(baby_actual_min, baby_expected_min);
    assert_close3(baby_actual_max, baby_expected_max);

    let trader_meshes = entity_model_textured_meshes(&[trader], &atlas);
    assert_villager_submissions_match_vanilla(&trader_meshes, trader);
    assert_eq!(
        trader_meshes.cutout.cutout_faces,
        adult_meshes.cutout.cutout_faces
    );
    assert_eq!(
        trader_meshes.cutout.vertices.len(),
        adult_meshes.cutout.vertices.len()
    );
    assert_eq!(
        trader_meshes.cutout.indices.len(),
        adult_meshes.cutout.indices.len()
    );
    assert_close2(
        trader_meshes.cutout.vertices[0].uv,
        [16.0 / 64.0, 2.0 / 3.0],
    );
    let trader_colored = entity_model_mesh(&[trader]);
    let (trader_expected_min, trader_expected_max) = mesh_extents(&trader_colored);
    let (trader_actual_min, trader_actual_max) = textured_mesh_extents(&trader_meshes.cutout);
    assert_close3(trader_actual_min, trader_expected_min);
    assert_close3(trader_actual_max, trader_expected_max);
}

#[test]
fn villager_profession_layers_render_type_profession_and_level_overlays() {
    let textures = [
        VILLAGER_TEXTURE_REF,
        VILLAGER_TYPE_TEXTURE_REFS[3],
        VILLAGER_PROFESSION_TEXTURE_REFS[9],
        VILLAGER_LEVEL_TEXTURE_REFS[4],
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&textures)).unwrap();
    let mason = villager_submission_probe(
        EntityModelInstance::villager(401, [0.0, 64.0, 0.0], 0.0, false).with_villager_model_data(
            VillagerModelData::new(
                VillagerModelType::Savanna,
                VillagerModelProfession::Mason,
                9,
            ),
        ),
    );
    let meshes = entity_model_textured_meshes(&[mason], &atlas);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 264);
    assert_eq!(mesh.vertices.len(), 1056);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(mesh.vertices[264].uv, [16.0 / 64.0, 1.0 / 4.0]);
    assert_close2(mesh.vertices[528].uv, [16.0 / 64.0, 2.0 / 4.0]);
    assert_close2(mesh.vertices[792].uv, [16.0 / 64.0, 3.0 / 4.0]);

    assert_eq!(meshes.submissions.len(), 4);
    assert_villager_submissions_match_vanilla(&meshes, mason);
    let body_overlay = mason.render_state.overlay_coords();
    let layer_overlay = [0.0, body_overlay[1]];
    assert_ne!(body_overlay, layer_overlay);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == body_overlay));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == layer_overlay));
    let expected_transform = villager_adult_model_root_transform(mason);
    for (submit, texture, order, sequence) in [
        (meshes.submissions[0], VILLAGER_TEXTURE_REF, 0, 0),
        (meshes.submissions[1], VILLAGER_TYPE_TEXTURE_REFS[3], 1, 1),
        (
            meshes.submissions[2],
            VILLAGER_PROFESSION_TEXTURE_REFS[9],
            2,
            2,
        ),
        (meshes.submissions[3], VILLAGER_LEVEL_TEXTURE_REFS[4], 3, 3),
    ] {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
        assert_eq!(submit.transform, expected_transform);
    }
}

#[test]
fn villager_profession_submissions_survive_missing_texture_atlas_entries() {
    // Vanilla `VillagerProfessionLayer.submit` records type/profession/level submits at orders
    // 1/2/3 after the base body; missing atlas data suppresses only folded overlay geometry.
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        VILLAGER_TEXTURE_REF,
        VILLAGER_BABY_TEXTURE_REF,
    ]))
    .unwrap();

    let adult = villager_submission_probe(
        EntityModelInstance::villager(406, [0.0, 64.0, 0.0], 0.0, false).with_villager_model_data(
            VillagerModelData::new(
                VillagerModelType::Savanna,
                VillagerModelProfession::Mason,
                9,
            ),
        ),
    );
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_villager_folded_meshes_are_cutout_only(&adult_meshes);
    assert_eq!(adult_meshes.submissions.len(), 4);
    assert_eq!(adult_meshes.cutout.vertices.len(), 264);
    let adult_base_overlay = adult.render_state.overlay_coords();
    let adult_layer_overlay = [0.0, adult_base_overlay[1]];
    let adult_transform = villager_adult_model_root_transform(adult);
    for (submit, texture, order, sequence, overlay) in [
        (
            adult_meshes.submissions[0],
            VILLAGER_TEXTURE_REF,
            0,
            0,
            adult_base_overlay,
        ),
        (
            adult_meshes.submissions[1],
            villager_type_texture_ref(VillagerModelType::Savanna, false),
            1,
            1,
            adult_layer_overlay,
        ),
        (
            adult_meshes.submissions[2],
            villager_profession_texture_ref(VillagerModelProfession::Mason).unwrap(),
            2,
            2,
            adult_layer_overlay,
        ),
        (
            adult_meshes.submissions[3],
            villager_level_texture_ref(9),
            3,
            3,
            adult_layer_overlay,
        ),
    ] {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, adult_transform);
        assert_eq!(submit.light, adult.render_state.shader_light());
        assert_eq!(submit.overlay, overlay);
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
    }
    assert!(adult_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult_base_overlay
    }));
    assert!(adult_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != adult_layer_overlay));

    let baby = villager_submission_probe(
        EntityModelInstance::villager(407, [2.0, 64.0, 0.0], 0.0, true).with_villager_model_data(
            VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::Mason, 5),
        ),
    );
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_folded_meshes_are_cutout_only(&baby_meshes);
    assert_eq!(baby_meshes.submissions.len(), 2);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    let baby_base_overlay = baby.render_state.overlay_coords();
    let baby_layer_overlay = [0.0, baby_base_overlay[1]];
    for (submit, texture, order, sequence, overlay) in [
        (
            baby_meshes.submissions[0],
            VILLAGER_BABY_TEXTURE_REF,
            0,
            0,
            baby_base_overlay,
        ),
        (
            baby_meshes.submissions[1],
            villager_type_texture_ref(VillagerModelType::Plains, true),
            1,
            1,
            baby_layer_overlay,
        ),
    ] {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(baby));
        assert_eq!(submit.light, baby.render_state.shader_light());
        assert_eq!(submit.overlay, overlay);
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
    }
    assert!(baby_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby_base_overlay
    }));
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != baby_layer_overlay));
}

#[test]
fn villager_profession_layers_apply_no_hat_and_baby_rules() {
    let textures = [
        VILLAGER_TEXTURE_REF,
        VILLAGER_BABY_TEXTURE_REF,
        VILLAGER_TYPE_TEXTURE_REFS[0],
        VILLAGER_BABY_TYPE_TEXTURE_REFS[2],
        VILLAGER_PROFESSION_TEXTURE_REFS[4],
        VILLAGER_LEVEL_TEXTURE_REFS[0],
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&textures)).unwrap();

    let desert_farmer = EntityModelInstance::villager(402, [0.0, 64.0, 0.0], 0.0, false)
        .with_villager_model_data(VillagerModelData::new(
            VillagerModelType::Desert,
            VillagerModelProfession::Farmer,
            1,
        ));
    let meshes = entity_model_textured_meshes(&[desert_farmer], &atlas);
    assert_villager_submissions_match_vanilla(&meshes, desert_farmer);
    let mesh = &meshes.cutout;
    // Base 66 faces + type layer without hat/rim 54 + profession 66 + level 66.
    assert_eq!(mesh.cutout_faces, 252);
    assert_eq!(mesh.vertices.len(), 1008);

    let baby =
        EntityModelInstance::villager(403, [2.0, 64.0, 0.0], 0.0, true).with_villager_model_data(
            VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::Mason, 5),
        );
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_submissions_match_vanilla(&baby_meshes, baby);
    // Vanilla skips profession and level overlays for babies even when data carries a profession.
    assert_eq!(baby_meshes.cutout.cutout_faces, 132);
    assert_eq!(baby_meshes.cutout.vertices.len(), 528);
}

#[test]
fn villager_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();

    // Adult villager/wandering trader head is part 0: head look turns it.
    let adult = EntityModelInstance::villager(142, [0.0, 64.0, 0.0], 0.0, false);
    let adult_yawed_instance = adult.with_head_look(45.0, 0.0);
    let adult_pitched_instance = adult.with_head_look(0.0, -20.0);
    let adult_resting = entity_model_textured_meshes(&[adult], &atlas);
    let adult_yawed = entity_model_textured_meshes(&[adult_yawed_instance], &atlas);
    let adult_pitched = entity_model_textured_meshes(&[adult_pitched_instance], &atlas);
    assert_villager_submissions_match_vanilla(&adult_resting, adult);
    assert_villager_submissions_match_vanilla(&adult_yawed, adult_yawed_instance);
    assert_villager_submissions_match_vanilla(&adult_pitched, adult_pitched_instance);
    assert_eq!(
        adult_resting.cutout.vertices.len(),
        adult_yawed.cutout.vertices.len()
    );
    assert_ne!(adult_resting.cutout.vertices, adult_yawed.cutout.vertices);
    assert_ne!(adult_yawed.cutout.vertices, adult_pitched.cutout.vertices);

    let trader = EntityModelInstance::wandering_trader(143, [0.0, 64.0, 0.0], 0.0);
    let trader_looking_instance = trader.with_head_look(45.0, -20.0);
    let trader_resting = entity_model_textured_meshes(&[trader], &atlas);
    let trader_looking = entity_model_textured_meshes(&[trader_looking_instance], &atlas);
    assert_villager_submissions_match_vanilla(&trader_resting, trader);
    assert_villager_submissions_match_vanilla(&trader_looking, trader_looking_instance);
    assert_ne!(
        trader_resting.cutout.vertices,
        trader_looking.cutout.vertices
    );

    // Baby villager lists an empty arms container then legs first (head at index
    // 3); head look turns the head and leaves the leading leg cube untouched.
    let baby = EntityModelInstance::villager(144, [0.0, 64.0, 0.0], 0.0, true);
    let baby_looking_instance = baby.with_head_look(45.0, -20.0);
    let baby_resting = entity_model_textured_meshes(&[baby], &atlas);
    let baby_looking = entity_model_textured_meshes(&[baby_looking_instance], &atlas);
    assert_villager_submissions_match_vanilla(&baby_resting, baby);
    assert_villager_submissions_match_vanilla(&baby_looking, baby_looking_instance);
    assert_ne!(baby_resting.cutout.vertices, baby_looking.cutout.vertices);
    assert_eq!(
        baby_resting.cutout.vertices[0..24],
        baby_looking.cutout.vertices[0..24]
    );
}

#[test]
fn villager_unhappy_head_pose_matches_vanilla_setup_anim() {
    // Vanilla `VillagerModel.setupAnim`: head look sets yRot/xRot first; the unhappy branch preserves
    // yRot, sets `head.xRot = 0.4`, and rolls by `0.3 * sin(0.45 * ageInTicks)`.
    let age = 7.0_f32;
    let base = EntityModelInstance::villager(145, [0.0, 64.0, 0.0], 0.0, false)
        .with_head_look(35.0, -20.0)
        .with_age_in_ticks(age);

    let mut content = VillagerModel::new(false);
    content.prepare(&base);
    let content_head = content.root_mut().child_mut("head").pose.rotation;
    assert!(
        (content_head[0] - (-20.0_f32).to_radians()).abs() < 1.0e-6,
        "content villager keeps the look pitch: {}",
        content_head[0]
    );
    assert!(
        (content_head[1] - 35.0_f32.to_radians()).abs() < 1.0e-6,
        "content villager keeps the look yaw: {}",
        content_head[1]
    );
    assert_eq!(content_head[2], 0.0);

    let mut unhappy = VillagerModel::new(false);
    unhappy.prepare(&base.with_villager_unhappy(true));
    let unhappy_head = unhappy.root_mut().child_mut("head").pose.rotation;
    let shake = 0.3 * (0.45 * age).sin();
    assert!(
        (unhappy_head[0] - 0.4).abs() < 1.0e-6,
        "unhappy branch fixes the head pitch: {}",
        unhappy_head[0]
    );
    assert!(
        (unhappy_head[1] - 35.0_f32.to_radians()).abs() < 1.0e-6,
        "unhappy branch preserves the look yaw: {}",
        unhappy_head[1]
    );
    assert!(
        (unhappy_head[2] - shake).abs() < 1.0e-6,
        "unhappy branch rolls the head: {} vs {shake}",
        unhappy_head[2]
    );

    let mut baby = VillagerModel::new(true);
    baby.prepare(&base.with_villager_unhappy(true));
    let baby_head = baby.root_mut().child_mut("head").pose.rotation;
    assert!(
        (baby_head[0] - 0.4).abs() < 1.0e-6,
        "baby villagers use the same unhappy pitch: {}",
        baby_head[0]
    );
    assert!(
        (baby_head[2] - shake).abs() < 1.0e-6,
        "baby villagers use the same unhappy roll: {}",
        baby_head[2]
    );
}

#[test]
fn villager_family_swings_its_legs_when_walking() {
    // `VillagerModel.setupAnim` swings the legs `cos(pos * 0.6662 [+ π]) * 1.4 *
    // speed * 0.5` (half the `HumanoidModel` amplitude). A standing villager is inert
    // and a walking one differs, for the adult and baby layers and the wandering
    // trader (which reuses the adult layer). The adult-size legs (12 tall) also lift
    // the feet and splay along Z; the baby's short 3-px legs swing too but the motion
    // stays inside the larger head/body bounding box, so only the adult-size models
    // assert the extent change. The combined `arms` part defers.
    let instances: [(&str, EntityModelInstance, bool); 3] = [
        (
            "villager_adult",
            EntityModelInstance::villager(300, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            "villager_baby",
            EntityModelInstance::villager(301, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
        (
            "wandering_trader",
            EntityModelInstance::wandering_trader(302, [0.0, 64.0, 0.0], 0.0),
            true,
        ),
    ];
    for (name, base, adult_size) in instances {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        if adult_size {
            let (rest_min, rest_max) = mesh_extents(&rest);
            let (walk_min, walk_max) = mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking villager's feet should lift off the ground"
            );
            assert!(
                (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
                "{name}: a walking villager's legs should splay along Z"
            );
        }
    }
}

#[test]
fn villager_family_textured_mesh_swings_legs_when_walking() {
    // The real villager render path (texture-backed) swings the same half-amplitude
    // legs. A standing villager is byte-identical however far the swing position has
    // advanced; a walking one differs, for the adult/baby layers and trader. The
    // adult-size models also lift their feet (the baby's short legs stay inside the
    // head/body box, as in the colored test).
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();
    let instances: [(&str, EntityModelInstance, bool); 3] = [
        (
            "villager_adult",
            EntityModelInstance::villager(303, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            "villager_baby",
            EntityModelInstance::villager(304, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
        (
            "wandering_trader",
            EntityModelInstance::wandering_trader(305, [0.0, 64.0, 0.0], 0.0),
            true,
        ),
    ];
    for (name, base, adult_size) in instances {
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_villager_submissions_match_vanilla(&resting, base);
        assert_villager_submissions_match_vanilla(&still, still_instance);
        assert_villager_submissions_match_vanilla(&walking, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{name}: a standing villager is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{name}: a walking villager differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
            let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking villager's feet should lift off the ground"
            );
        }
    }
}

fn assert_villager_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_villager_folded_meshes_are_cutout_only(meshes);
    let (base_passes, transform, data) = match instance.kind {
        EntityModelKind::Villager { baby } => {
            let transform = if baby {
                entity_model_root_transform(instance)
            } else {
                villager_adult_model_root_transform(instance)
            };
            (
                villager_textured_layer_passes(baby),
                transform,
                Some((baby, instance.render_state.villager_model_data)),
            )
        }
        EntityModelKind::WanderingTrader => (
            wandering_trader_textured_layer_passes(),
            villager_adult_model_root_transform(instance),
            None,
        ),
        _ => panic!("expected villager family instance"),
    };

    let mut expected = Vec::new();
    expected.extend(
        base_passes
            .iter()
            .map(|pass| (pass.texture, pass.order, pass.submit_sequence, false)),
    );
    if let Some((baby, data)) = data {
        expected.extend(
            villager_data_textured_layer_passes(baby, data)
                .iter()
                .map(|pass| (pass.texture, pass.order, pass.submit_sequence, true)),
        );
    }

    assert_eq!(meshes.submissions.len(), expected.len());
    let base_overlay = instance.render_state.overlay_coords();
    let zero_white_overlay = [0.0, base_overlay[1]];
    let has_zero_white_overlay = expected.iter().any(|expected| expected.3);
    for (submit, (texture, order, sequence, zero_white_overlay_submit)) in
        meshes.submissions.iter().zip(expected)
    {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, texture);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, transform);
        assert_eq!((submit.order, submit.submit_sequence), (order, sequence));
        assert_eq!(submit.light, instance.render_state.shader_light());
        let expected_overlay = if zero_white_overlay_submit {
            zero_white_overlay
        } else {
            base_overlay
        };
        assert_eq!(submit.overlay, expected_overlay);
        if zero_white_overlay_submit && expected_overlay != base_overlay {
            assert_ne!(submit.overlay, base_overlay);
        }
    }
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
    if has_zero_white_overlay && zero_white_overlay != base_overlay {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == base_overlay));
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.overlay == base_overlay || vertex.overlay == zero_white_overlay
        }));
    } else {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == base_overlay));
    }
}

fn villager_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance
        .with_light_coords((8_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
}

fn assert_villager_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn villager_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(villager_entity_texture_refs())
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
