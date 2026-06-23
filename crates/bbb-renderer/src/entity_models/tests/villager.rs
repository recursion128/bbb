use super::*;

use crate::entity_models::model::ModelCube;

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
    assert_eq!(adult[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(adult[0].model_layer, MODEL_LAYER_VILLAGER);
    assert_eq!(adult[0].texture, VILLAGER_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].collector_order, adult[0].submit_sequence), (0, 0));

    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::VillagerBase);
    assert_eq!(baby[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_VILLAGER_BABY);
    assert_eq!(baby[0].texture, VILLAGER_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].collector_order, baby[0].submit_sequence), (0, 0));

    assert_eq!(trader.len(), 1);
    assert_eq!(trader[0].kind, EntityModelLayerKind::WanderingTraderBase);
    assert_eq!(trader[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(trader[0].model_layer, MODEL_LAYER_WANDERING_TRADER);
    assert_eq!(trader[0].texture, WANDERING_TRADER_TEXTURE_REF);
    assert_eq!(trader[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(trader[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (trader[0].collector_order, trader[0].submit_sequence),
        (0, 0)
    );
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
    let adult = EntityModelInstance::villager(139, [0.0, 64.0, 0.0], 0.0, false);
    let baby = EntityModelInstance::villager(140, [2.0, 64.0, 0.0], 0.0, true);
    let trader = EntityModelInstance::wandering_trader(141, [4.0, 64.0, 0.0], 0.0);

    let adult_mesh = entity_model_textured_mesh(&[adult], &atlas);
    assert_eq!(adult_mesh.cutout_faces, 66);
    assert_eq!(adult_mesh.vertices.len(), 264);
    assert_eq!(adult_mesh.indices.len(), 396);
    assert_close2(adult_mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(adult_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let adult_colored = entity_model_mesh(&[adult]);
    let (adult_expected_min, adult_expected_max) = mesh_extents(&adult_colored);
    let (adult_actual_min, adult_actual_max) = textured_mesh_extents(&adult_mesh);
    assert_close3(adult_actual_min, adult_expected_min);
    assert_close3(adult_actual_max, adult_expected_max);

    let baby_mesh = entity_model_textured_mesh(&[baby], &atlas);
    assert_eq!(baby_mesh.cutout_faces, 66);
    assert_eq!(baby_mesh.vertices.len(), 264);
    assert_eq!(baby_mesh.indices.len(), 396);
    assert!(baby_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let baby_colored = entity_model_mesh(&[baby]);
    let (baby_expected_min, baby_expected_max) = mesh_extents(&baby_colored);
    let (baby_actual_min, baby_actual_max) = textured_mesh_extents(&baby_mesh);
    assert_close3(baby_actual_min, baby_expected_min);
    assert_close3(baby_actual_max, baby_expected_max);

    let trader_mesh = entity_model_textured_mesh(&[trader], &atlas);
    assert_eq!(trader_mesh.cutout_faces, adult_mesh.cutout_faces);
    assert_eq!(trader_mesh.vertices.len(), adult_mesh.vertices.len());
    assert_eq!(trader_mesh.indices.len(), adult_mesh.indices.len());
    assert_close2(trader_mesh.vertices[0].uv, [16.0 / 64.0, 2.0 / 3.0]);
    let trader_colored = entity_model_mesh(&[trader]);
    let (trader_expected_min, trader_expected_max) = mesh_extents(&trader_colored);
    let (trader_actual_min, trader_actual_max) = textured_mesh_extents(&trader_mesh);
    assert_close3(trader_actual_min, trader_expected_min);
    assert_close3(trader_actual_max, trader_expected_max);
}

#[test]
fn villager_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();

    // Adult villager/wandering trader head is part 0: head look turns it.
    let adult = EntityModelInstance::villager(142, [0.0, 64.0, 0.0], 0.0, false);
    let adult_resting = entity_model_textured_mesh(&[adult], &atlas);
    let adult_yawed = entity_model_textured_mesh(&[adult.with_head_look(45.0, 0.0)], &atlas);
    let adult_pitched = entity_model_textured_mesh(&[adult.with_head_look(0.0, -20.0)], &atlas);
    assert_eq!(adult_resting.vertices.len(), adult_yawed.vertices.len());
    assert_ne!(adult_resting.vertices, adult_yawed.vertices);
    assert_ne!(adult_yawed.vertices, adult_pitched.vertices);

    let trader = EntityModelInstance::wandering_trader(143, [0.0, 64.0, 0.0], 0.0);
    let trader_resting = entity_model_textured_mesh(&[trader], &atlas);
    let trader_looking = entity_model_textured_mesh(&[trader.with_head_look(45.0, -20.0)], &atlas);
    assert_ne!(trader_resting.vertices, trader_looking.vertices);

    // Baby villager lists an empty arms container then legs first (head at index
    // 3); head look turns the head and leaves the leading leg cube untouched.
    let baby = EntityModelInstance::villager(144, [0.0, 64.0, 0.0], 0.0, true);
    let baby_resting = entity_model_textured_mesh(&[baby], &atlas);
    let baby_looking = entity_model_textured_mesh(&[baby.with_head_look(45.0, -20.0)], &atlas);
    assert_ne!(baby_resting.vertices, baby_looking.vertices);
    assert_eq!(baby_resting.vertices[0..24], baby_looking.vertices[0..24]);
}

#[test]
fn villager_family_swings_its_legs_when_walking() {
    // `VillagerModel.setupAnim` swings the legs `cos(pos * 0.6662 [+ π]) * 1.4 *
    // speed * 0.5` (half the `HumanoidModel` amplitude). A standing villager is inert
    // and a walking one differs, for the adult and baby layers and the wandering
    // trader (which reuses the adult layer). The adult-size legs (12 tall) also lift
    // the feet and splay along Z; the baby's short 3-px legs swing too but the motion
    // stays inside the larger head/body bounding box, so only the adult-size models
    // assert the extent change. The combined `arms` part and unhappy head shake defer.
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
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing villager is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking villager differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking villager's feet should lift off the ground"
            );
        }
    }
}

fn villager_texture_images() -> Vec<EntityModelTextureImage> {
    villager_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
