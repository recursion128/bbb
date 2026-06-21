use super::*;

#[test]
fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_VILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.51, -10.51, -4.51],
            size: [9.02, 11.02, 9.02],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(
        ADULT_VILLAGER_JACKET[0],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_VILLAGER_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HEAD.as_slice(),
        ADULT_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HAT.as_slice(),
        ADULT_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_NOSE.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_BODY.as_slice(),
        ADULT_VILLAGER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_JACKET.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ADULT_VILLAGER_ARMS.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
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
            ModelCubeDesc {
                min: [-1.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [5.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        BABY_VILLAGER_BB_MAIN[0],
        ModelCubeDesc {
            min: [-2.7, -8.2, -1.7],
            size: [4.4, 6.4, 3.4],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &BABY_VILLAGER_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_VILLAGER_ARMS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[0],
        [-3.0, 1.4025, -0.9599],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_RIGHT_HAND.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[1],
        [0.0, 0.9024, -1.8175],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_MIDDLE_ARM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[1],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[2],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part_tree(
        &BABY_VILLAGER_PARTS[3],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HEAD.as_slice(),
        BABY_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[4],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[5],
        [0.5, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BB_MAIN.as_slice(),
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
    assert_eq!(adult[0].parts, ADULT_VILLAGER_TEXTURED_PARTS.as_slice());
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].collector_order, adult[0].submit_sequence), (0, 0));

    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::VillagerBase);
    assert_eq!(baby[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(baby[0].model_layer, MODEL_LAYER_VILLAGER_BABY);
    assert_eq!(baby[0].texture, VILLAGER_BABY_TEXTURE_REF);
    assert_eq!(baby[0].parts, BABY_VILLAGER_TEXTURED_PARTS.as_slice());
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].collector_order, baby[0].submit_sequence), (0, 0));

    assert_eq!(trader.len(), 1);
    assert_eq!(trader[0].kind, EntityModelLayerKind::WanderingTraderBase);
    assert_eq!(trader[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(trader[0].model_layer, MODEL_LAYER_WANDERING_TRADER);
    assert_eq!(trader[0].texture, WANDERING_TRADER_TEXTURE_REF);
    assert_eq!(trader[0].parts, ADULT_VILLAGER_TEXTURED_PARTS.as_slice());
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

    assert_eq!(
        ADULT_VILLAGER_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-4.51, -10.51, -4.51],
            size: [9.02, 11.02, 9.02],
            uv_size: [8.0, 10.0, 8.0],
            tex: [32.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_VILLAGER_TEXTURED_JACKET[0],
        TexturedModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            uv_size: [8.0, 20.0, 6.0],
            tex: [0.0, 38.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_VILLAGER_TEXTURED_ARMS[1].tex, [44.0, 22.0]);
    assert!(ADULT_VILLAGER_TEXTURED_ARMS[1].mirror);
    assert!(ADULT_VILLAGER_TEXTURED_LEFT_LEG[0].mirror);
    assert_eq!(
        ADULT_VILLAGER_TEXTURED_PARTS[0].children,
        ADULT_VILLAGER_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(
        ADULT_VILLAGER_TEXTURED_HEAD_CHILDREN[0].children,
        ADULT_VILLAGER_TEXTURED_HAT_CHILDREN.as_slice()
    );
    assert_eq!(
        ADULT_VILLAGER_TEXTURED_PARTS[4].pose,
        ADULT_VILLAGER_PARTS[4].pose
    );

    assert_eq!(
        BABY_VILLAGER_TEXTURED_RIGHT_HAND,
        [
            TexturedModelCubeDesc {
                min: [-1.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                uv_size: [2.0, 4.0, 2.0],
                tex: [36.0, 15.0],
                mirror: false,
            },
            TexturedModelCubeDesc {
                min: [5.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                uv_size: [2.0, 4.0, 2.0],
                tex: [16.0, 15.0],
                mirror: false,
            },
        ]
    );
    assert_eq!(BABY_VILLAGER_TEXTURED_RIGHT_LEG[0].tex, [8.0, 23.0]);
    assert_eq!(BABY_VILLAGER_TEXTURED_LEFT_LEG[0].tex, [0.0, 23.0]);
    assert_eq!(
        BABY_VILLAGER_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-4.3, -4.3, -3.8],
            size: [8.6, 8.6, 7.6],
            uv_size: [8.0, 8.0, 7.0],
            tex: [0.0, 30.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_VILLAGER_TEXTURED_BB_MAIN[0],
        TexturedModelCubeDesc {
            min: [-2.7, -8.2, -1.7],
            size: [4.4, 6.4, 3.4],
            uv_size: [4.0, 6.0, 3.0],
            tex: [16.0, 21.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_VILLAGER_TEXTURED_PARTS[0].children,
        BABY_VILLAGER_TEXTURED_ARMS_CHILDREN.as_slice()
    );
    assert_eq!(
        BABY_VILLAGER_TEXTURED_PARTS[3].children,
        BABY_VILLAGER_TEXTURED_HEAD_CHILDREN.as_slice()
    );
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
