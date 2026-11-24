use super::*;

#[test]
fn first_person_map_decorations_keep_builtin_order_after_registry_injection() {
    let root = unique_item_model_temp_dir("map-label-custom-font");
    write_flat_item_runtime_fixture(&root, &["filled_map"]);
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    for byte in b' '..=b'~' {
        glyphs.insert_first_wins(
            char::from(byte),
            bbb_client_render::HudAsciiGlyph {
                width: 9,
                height: 8,
                advance: 9.0,
                bearing_top: 7.0,
                uv: bbb_client_render::HudUvRect {
                    min: [f32::from(byte) / 128.0, 0.0],
                    max: [(f32::from(byte) + 1.0) / 128.0, 1.0],
                },
                ..Default::default()
            },
        );
    }
    let default_glyphs = NativeItemRuntime::empty_for_test()
        .map_text_fonts()
        .unwrap()
        .glyphs(None)
        .clone();
    let fonts = bbb_client_render::HudFontGlyphBook::from_fonts(
        "minecraft:default",
        std::collections::BTreeMap::from([
            ("minecraft:default".into(), default_glyphs),
            ("example:map".into(), glyphs),
        ]),
    );
    let item_runtime = NativeItemRuntime::load_with_locale_and_fonts(
        &bbb_client_assets::PackRoots::from_root(&root).unwrap(),
        "en_us",
        Some(fonts),
    )
    .unwrap();
    let mut map = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    map.component_patch.map_id = Some(9);
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.record_registry_data(RegistryData {
        registry: "minecraft:map_decoration_type".to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:injected".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 24,
    });
    assert!(world
        .registry_content("minecraft:map_decoration_type")
        .is_none());
    let registry_packet = world.registries().registries.last().unwrap();
    assert!(registry_packet.client_built_in);
    assert!(!registry_packet.configuration_synchronized);
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 9,
        scale: 0,
        locked: false,
        decorations: Some(vec![
            bbb_protocol::packets::MapDecoration {
                type_id: 0,
                x: 0,
                y: 0,
                rot: 0,
                name: Some(
                    bbb_protocol::RichTextComponent::try_from_component_nbt(
                        bbb_protocol::component_styled_literal_nbt(
                            "Player",
                            &bbb_protocol::ComponentStyle {
                                bold: Some(true),
                                italic: Some(true),
                                color: Some(0x55ff55),
                                obfuscated: Some(true),
                                font: Some("example:map".into()),
                                ..Default::default()
                            }
                        )
                        .unwrap()
                    )
                    .unwrap()
                ),
            },
            bbb_protocol::packets::MapDecoration {
                type_id: 1,
                x: -20,
                y: 30,
                rot: 7,
                name: Some(bbb_protocol::RichTextComponent::plain("Frame")),
            },
        ]),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![(1 << 2) | 2],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );

    assert_eq!(models.map_surfaces.len(), 1);
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(models.map_decoration_surfaces.len(), 2);
    assert_eq!(models.map_text_surfaces.len(), 2);
    let player = &models.map_decoration_surfaces[0];
    assert_eq!(player.submission.type_id, 0);
    assert_eq!(
        player.submission.texture.vanilla_sprite_id(),
        "minecraft:player"
    );
    assert_eq!(
        (player.submission.order, player.submission.submit_sequence),
        (0, 1)
    );
    assert_eq!(player.submission.decoration_index, 0);
    let frame = &models.map_decoration_surfaces[1];
    assert_eq!(frame.submission.type_id, 1);
    assert_eq!(
        frame.submission.texture.vanilla_sprite_id(),
        "minecraft:frame"
    );
    assert_eq!(
        (frame.submission.order, frame.submission.submit_sequence),
        (0, 2)
    );
    assert_eq!(frame.submission.decoration_index, 1);
    assert_eq!(models.map_text_surfaces[0].submission.type_id, 0);
    assert_eq!(models.map_text_surfaces[0].submission.text, "Player");
    assert_eq!(models.map_text_surfaces[0].submission.width, 60.0);
    assert_eq!(models.map_text_surfaces[0].vertex_count(), 52);
    assert_eq!(
        (
            models.map_text_surfaces[0].submission.order,
            models.map_text_surfaces[0].submission.submit_sequence
        ),
        (1, 0)
    );
    assert_eq!(models.map_text_surfaces[1].submission.type_id, 1);
    assert_eq!(models.map_text_surfaces[1].submission.text, "Frame");
    assert_eq!(
        (
            models.map_text_surfaces[1].submission.order,
            models.map_text_surfaces[1].submission.submit_sequence
        ),
        (1, 1)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn marker_armor_stand_equipment_uses_brightest_covered_light_cell() {
    use bbb_client_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{EquipmentSlotUpdate, SetEquipment, Vec3d};

    let root = unique_item_model_temp_dir("marker-armor-stand-equipment-light");
    write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let mut block_light = [0_u8; 2048];
    // Lit cell at (1, 6, 3), while feet at (1, 5, 3) remain dark (0, 0).
    let index = (6 << 8) | (3 << 4) | 1;
    block_light[index / 2] = 12 << ((index % 2) * 4);
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos { x: 0, z: 0 },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: vec![std::sync::Arc::new(ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: PalettedContainerData {
                domain: PaletteDomain::BlockStates,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![bbb_client_world::vanilla_air_block_state_id()],
                packed_data: Vec::new(),
                entry_count: 4096,
            },
            biomes: PalettedContainerData {
                domain: PaletteDomain::Biomes,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![0],
                packed_data: Vec::new(),
                entry_count: 64,
            },
        })],
        block_entities: std::sync::Arc::default(),
        light: {
            let mut light = LightData::default();
            light.set_sky_packed(1, [0; 2048]);
            light.set_block_packed(1, block_light);
            light
        },
    });
    const STAND_ID: i32 = 901;
    let mut add = protocol_add_entity(STAND_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    add.position = Vec3d {
        x: 1.5,
        y: 5.0,
        z: 3.5,
    };
    world.apply_add_entity(add);
    let accessors =
        bbb_client_world::vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: STAND_ID,
        values: vec![EntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.marker_mask | accessors.show_arms_mask),
        }],
    }));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: STAND_ID,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: item_runtime.item_protocol_id("minecraft:head_item"),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            },
        ],
    }));

    let instances = crate::entity_scene::entity_model_instances_from_world_at_partial_tick(
        &world,
        Some(&item_runtime),
        1.0,
    );
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].render_state.light_coords, 12 << 4);
    let models = held_item_models_with_variant_textures(
        &instances,
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(models.residency.instances.len(), 2);
    for instance in &models.residency.instances {
        assert_eq!(instance.light, [12.0 / 15.0, 0.0]);
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_display_propagates_glowing_outline_to_ordinary_and_special_layers() {
    use bbb_client_world::{
        DisplayBillboardConstraints, DisplayEntityRenderContent, DisplayEntityRenderSourceState,
        DisplayItemContext, DisplayTransformation, EntityVec3, TerrainLight,
    };

    let root = unique_item_model_temp_dir("item-display-glowing-outline");
    write_composite_special_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:composite_special"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let make_source =
        |appears_glowing: bool, glow_color_override: i32| DisplayEntityRenderSourceState {
            entity_id: 910,
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
            position: EntityVec3::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            billboard: DisplayBillboardConstraints::Fixed,
            brightness_override: -1,
            shadow_radius: 0.0,
            shadow_strength: 1.0,
            glow_color_override,
            appears_glowing,
            light: TerrainLight { sky: 12, block: 6 },
            transformation: DisplayTransformation::default(),
            content: DisplayEntityRenderContent::Item {
                stack: stack.clone(),
                context: DisplayItemContext::Fixed,
            },
        };

    let models = crate::item_models::display_entity_item_models(
        &[
            make_source(false, -1),
            make_source(true, -1),
            make_source(true, 0x12_34_56),
        ],
        &WorldStore::new(),
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        0,
        &crate::item_models::ItemRegistryKeySet::default(),
        None,
        false,
    );

    assert_eq!(models.residency.special_items.len(), 3);
    assert_eq!(models.residency.special_items[0].outline_color, None);
    assert_eq!(
        models.residency.special_items[1].outline_color,
        Some(bbb_client_render::ENTITY_DEFAULT_OUTLINE_COLOR)
    );
    assert_eq!(
        models.residency.special_items[2].outline_color,
        Some(0xFF12_3456)
    );
    let outline_instances: Vec<_> = models
        .residency
        .instances
        .iter()
        .filter(|instance| instance.target == ItemModelDrawTarget::EntityOutline)
        .collect();
    assert_eq!(outline_instances.len(), 4);
    assert!(outline_instances
        .iter()
        .all(|instance| instance.phase == ItemModelDrawPhase::Outline));
    assert_eq!(
        outline_instances[0].tint,
        argb_outline_tint(bbb_client_render::ENTITY_DEFAULT_OUTLINE_COLOR)
    );
    assert_eq!(outline_instances[2].tint, argb_outline_tint(0xFF12_3456));

    std::fs::remove_dir_all(root).unwrap();
}
