use super::*;
use crate::input::CreativeInventoryHudState;

#[test]
fn audio_scene_tracks_listener_and_selected_entity_positions() {
    let mut world = WorldStore::new();
    let standing_pose = local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0);
    world.set_local_player_pose(standing_pose);
    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        id: 123,
        uuid: uuid::Uuid::from_u128(123),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: bbb_protocol::packets::Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    let listener = audio_listener_state_from_world(&world);

    assert_eq!(
        listener,
        Some(AudioListenerState {
            position: [
                10.0,
                64.0 + world.local_player_eye_height(standing_pose),
                -5.0
            ],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );
    assert_eq!(
        audio_entity_position_from_world(&world, 123),
        Some([1.0, 2.0, 3.0])
    );

    let sneaking_pose = LocalPlayerPoseState {
        sneaking: true,
        ..local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0)
    };
    world.set_local_player_pose(sneaking_pose);
    let listener = audio_listener_state_from_world(&world);
    assert_eq!(
        listener,
        Some(AudioListenerState {
            position: [
                10.0,
                64.0 + world.local_player_eye_height(sneaking_pose),
                -5.0
            ],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
    let listener = audio_listener_state_from_world(&world);
    assert_eq!(
        listener,
        Some(AudioListenerState {
            position: [1.0, 2.0 + f64::from(0.2751_f32), 3.0],
            y_rot: 0.0,
            x_rot: 0.0,
        })
    );

    assert_eq!(
        world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
            entity_ids: vec![123],
        }),
        1
    );
    let listener = audio_listener_state_from_world(&world);
    assert!(audio_entity_position_from_world(&world, 123).is_none());
    assert_eq!(
        listener,
        Some(AudioListenerState {
            position: [
                10.0,
                64.0 + world.local_player_eye_height(sneaking_pose),
                -5.0
            ],
            y_rot: 90.0,
            x_rot: -10.0,
        })
    );
}

#[test]
fn hud_inventory_screen_projects_open_local_inventory_layout() {
    let mut world = WorldStore::new();
    assert_eq!(hud_inventory_screen(&world, None, Some(36), 0.0), None);

    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: bbb_protocol::packets::ItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

    assert_eq!(screen.width, 176);
    assert_eq!(screen.height, 166);
    assert_eq!(
        screen.background_layers,
        vec![
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Texture("textures/gui/container/inventory.png"),
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            ),
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Sprite("recipe_book/button"),
                104,
                61,
                20,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            ),
        ]
    );
    assert_eq!(screen.hovered_slot_id, Some(36));
    assert_eq!(screen.tooltip, None);
    assert_eq!(screen.slots.len(), 46);
    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!(hotbar.x, 8);
    assert_eq!(hotbar.y, 142);
    assert!(hotbar.icon.is_none());
    let offhand = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
    assert_eq!(offhand.x, 77);
    assert_eq!(offhand.y, 62);
}

#[test]
fn hud_inventory_screen_projects_local_player_entity_preview() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen_with_local_state(
        &world,
        None,
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((10, 80)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.entity_previews.len(), 1);
    let preview = &screen.entity_previews[0];
    assert_eq!(preview.entity.entity_id, 42);
    assert_eq!(preview.entity.position, [0.0; 3]);
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 26,
            y: 8,
            width: 49,
            height: 70,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_eq!(preview.scale, 30.0);
    assert!(preview.depth_isolated);
    assert_close3(preview.translation, [0.0, 0.9625, 0.0]);

    let x_angle = ((50.5_f32 - 10.0) / 40.0).atan();
    let y_angle = ((43.0_f32 - 80.0) / 40.0).atan();
    let expected_yaw = x_angle * 20.0;
    let expected_pitch = y_angle * 20.0;
    let expected_camera = quaternion_x(expected_pitch.to_radians());
    assert!((preview.entity.render_state.body_rot - (180.0 + expected_yaw)).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_yaw - expected_yaw).abs() < 1.0e-6);
    assert!((preview.entity.render_state.head_pitch + expected_pitch).abs() < 1.0e-6);
    assert_close4(
        preview.rotation,
        quaternion_mul([0.0, 0.0, 1.0, 0.0], expected_camera),
    );
    assert_close4(
        preview
            .override_camera_rotation
            .expect("inventory preview should override the camera rotation"),
        expected_camera,
    );
    assert_eq!(
        preview.entity.render_state.light_coords,
        ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
    assert_eq!(preview.entity.render_state.outline_color, 0);
    assert!(!preview.entity.render_state.appears_glowing);
}

#[test]
fn hud_creative_inventory_tab_projects_local_player_entity_preview_only_on_inventory_tab() {
    let mut world = world_with_dimension_height(0, "minecraft:overworld", 384);
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    let state = CreativeInventoryHudState {
        selected_tab: "minecraft:inventory".to_string(),
        search: crate::input::text_field::TextField::new(50),
        search_focused: false,
        scroll_row: 0,
        hovered: None,
        cursor_position: Some((60, 80)),
        saved_hotbars: Vec::new(),
        hotbar_key_bindings: crate::options::key_bindings::KeyBindings::default(),
        quick_craft_button_num: None,
        quick_craft_slots: Vec::new(),
    };

    assert!(creative_inventory::hud_creative_inventory_player_preview(
        &world,
        None,
        CreativeInventoryTabType::Category,
        &state,
        0.0,
    )
    .is_none());

    let preview = creative_inventory::hud_creative_inventory_player_preview(
        &world,
        None,
        CreativeInventoryTabType::Inventory,
        &state,
        0.0,
    )
    .expect("creative inventory player preview");
    assert_eq!(preview.entity.entity_id, 42);
    assert_eq!(preview.entity.position, [0.0; 3]);
    assert_eq!(preview.lighting, GuiItemLightingEntry::EntityInUi);
    assert_eq!(
        preview.rect,
        HudEntityPreviewRect {
            x: 73,
            y: 6,
            width: 32,
            height: 43,
        }
    );
    assert_eq!(preview.scissor, None);
    assert_eq!(preview.scale, 20.0);
    assert!(preview.depth_isolated);
    assert_close3(preview.translation, [0.0, 0.9625, 0.0]);

    let x_angle = ((89.0_f32 - 60.0) / 40.0).atan();
    let y_angle = ((27.5_f32 - 80.0) / 40.0).atan();
    let expected_yaw = x_angle * 20.0;
    let expected_pitch = y_angle * 20.0;
    let expected_camera = quaternion_x(expected_pitch.to_radians());
    assert!(
        (preview.entity.render_state.body_rot - (180.0 + expected_yaw)).abs() < 2.0e-5,
        "body rotation {:?}, expected {:?}",
        preview.entity.render_state.body_rot,
        180.0 + expected_yaw
    );
    assert!(
        (preview.entity.render_state.head_yaw - expected_yaw).abs() < 2.0e-5,
        "head yaw {:?}, expected {expected_yaw:?}",
        preview.entity.render_state.head_yaw
    );
    assert!(
        (preview.entity.render_state.head_pitch + expected_pitch).abs() < 2.0e-5,
        "head pitch {:?}, expected {:?}",
        preview.entity.render_state.head_pitch,
        -expected_pitch
    );
    assert_close4(
        preview.rotation,
        quaternion_mul([0.0, 0.0, 1.0, 0.0], expected_camera),
    );
    assert_eq!(preview.override_camera_rotation, Some(expected_camera));
}

#[test]
fn hotbar_item_icons_use_using_item_model_for_selected_slot_only() {
    let root = unique_runtime_temp_dir("hotbar-using-item");
    write_runtime_bow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let using_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                using_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, using_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: using_uv.min,
            max: using_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_selected_item_condition_for_selected_slot_only() {
    let root = unique_runtime_temp_dir("hotbar-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let selected_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                selected_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(1));

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_uses_selected_item_condition_for_local_selected_slot_only() {
    let root = unique_runtime_temp_dir("inventory-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let selected_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                selected_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(1));
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    let non_selected = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    let selected = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
    assert_eq!(
        non_selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_container_screen_uses_selected_item_condition_for_server_opened_hotbar_slot_only() {
    let root = unique_runtime_temp_dir("container-selected-condition");
    write_runtime_selected_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let selected_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                selected_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, selected_uv);

    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 63];
    items[54] = stack.clone();
    items[55] = stack;
    world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
        container_id: 7,
        state_id: 1,
        items,
        carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
    });
    assert!(world.set_local_selected_hotbar_slot(1));

    let screen = hud_inventory_screen(&world, Some(&item_runtime), None, 0.0).unwrap();

    let non_selected = screen.slots.iter().find(|slot| slot.slot_id == 54).unwrap();
    let selected = screen.slots.iter().find(|slot| slot.slot_id == 55).unwrap();
    assert_eq!(
        non_selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        selected.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: selected_uv.min,
            max: selected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_icons_use_carried_item_condition_only_when_marked_carried() {
    let root = unique_runtime_temp_dir("hud-carried-condition");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let carried_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                carried_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(normal_uv, carried_uv);

    let world = WorldStore::new();
    let ordinary_icon = hud_item_icon(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &stack,
        HudItemSlotState::default(),
        world.local_player_team_color_rgb(),
        0.0,
    )
    .unwrap();
    let carried_icon = hud_item_icon(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &stack,
        HudItemSlotState {
            carried_item: true,
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
        0.0,
    )
    .unwrap();

    assert_eq!(
        ordinary_icon.layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(
        carried_icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(ordinary_icon.foil, HudItemFoil::None);

    let mut foiled_stack = stack.clone();
    foiled_stack.component_patch.enchantment_glint_override = Some(true);
    let foiled_icon = hud_item_icon(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &foiled_stack,
        HudItemSlotState::default(),
        world.local_player_team_color_rgb(),
        0.0,
    )
    .unwrap();
    assert_eq!(foiled_icon.foil, HudItemFoil::Standard);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_item_foil_for_stack_projects_special_clock_and_compass_glint() {
    let root = unique_runtime_temp_dir("hud-special-foil");
    write_runtime_special_foil_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let foiled_stack = |resource_id: &str| {
        let mut stack = item_stack(item_runtime.item_protocol_id(resource_id).unwrap(), 1);
        stack.component_patch.enchantment_glint_override = Some(true);
        stack
    };

    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:clock")),
        HudItemFoil::Special
    );
    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:compass")),
        HudItemFoil::Special
    );
    assert_eq!(
        hud_item_foil_for_stack(&item_runtime, &foiled_stack("minecraft:spyglass")),
        HudItemFoil::Standard
    );
    assert_eq!(
        hud_item_foil_for_stack(
            &item_runtime,
            &item_stack(item_runtime.item_protocol_id("minecraft:clock").unwrap(), 1),
        ),
        HudItemFoil::None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn gui_item_model_projects_special_leaf_into_resident_geometry() {
    let root = unique_runtime_temp_dir("hud-special-item");
    write_runtime_special_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(
        item_runtime
            .item_protocol_id("minecraft:special_trident")
            .unwrap(),
        1,
    );

    let model = hud_block_item_model(
        &HudItemModelContext::new(&WorldStore::new(), Some(&item_runtime)),
        &stack,
        HudItemSlotState::default(),
        None,
        &TerrainTextureState::default(),
    )
    .expect("special leaf should use GUI resident geometry");

    assert_eq!(model.lighting, GuiItemLightingEntry::Items3d);
    assert!(matches!(
        model.layers.as_ref(),
        [HudResidentItemLayer::Special {
            model: bbb_client_render::SpecialItemModel::Trident { .. },
            ..
        }]
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources"]
fn local_vanilla_scaffolding_item_keeps_model_geometry_and_display() {
    let roots =
        bbb_client_assets::PackRoots::from_environment().expect("discover local Vanilla resources");
    let item_runtime = NativeItemRuntime::load(&roots).expect("load local Vanilla item runtime");
    let terrain = TerrainTextureState::default();
    let scaffolding = item_stack(
        item_runtime
            .item_protocol_id("minecraft:scaffolding")
            .expect("Vanilla scaffolding item id"),
        1,
    );
    let model = hud_block_item_model(
        &HudItemModelContext::new(&WorldStore::new(), Some(&item_runtime)),
        &scaffolding,
        HudItemSlotState::default(),
        None,
        &terrain,
    )
    .expect("real scaffolding item projects block geometry");

    let resolved = item_runtime
        .resolve_item_model(
            &scaffolding,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::FirstPersonRightHand,
            ),
        )
        .unwrap();
    let [ResolvedItemModelLayer::Cuboid {
        parts,
        display_transform: Some(display),
        ..
    }] = resolved.layers.as_slice()
    else {
        panic!("scaffolding must keep its selected cuboid model")
    };
    assert_eq!(display.rotation, [0.0, 45.0, 0.0]);
    assert_eq!(display.scale, [0.4; 3]);
    assert!(parts
        .iter()
        .all(|p| p.model_id.as_ref() == "minecraft:block/scaffolding_stable"));
    let corners = parts
        .iter()
        .flat_map(|p| ItemModelAssetSource::Model(p.geometry.clone()).build())
        .flat_map(|q| q.corners)
        .collect::<Vec<_>>();
    for axis in 0..3 {
        assert_eq!(
            corners
                .iter()
                .map(|p| p[axis])
                .fold(f32::INFINITY, f32::min),
            0.0
        );
        assert_eq!(
            corners
                .iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max),
            16.0
        );
    }
    assert!(
        parts
            .iter()
            .flat_map(|p| ItemModelAssetSource::Model(p.geometry.clone()).build())
            .any(|q| q.normal == [0.0, 1.0, 0.0]
                && q.corners.iter().all(|p| p[1] == 16.0)
                && q.uvs[0][0] != q.uvs[2][0]
                && q.uvs[0][1] != q.uvs[2][1]),
        "the textured full-width top must survive the item bake"
    );
    assert!(!model.layers.is_empty());
    assert!(model.layers.iter().all(|layer| matches!(
        layer,
        HudResidentItemLayer::Geometry {
            atlas: ItemModelAtlasKind::Items,
            source: ItemModelAssetSource::Model(_),
            ..
        }
    )));
}

#[test]
fn gui_item_model_reuses_bundle_selected_context_for_special_leaf() {
    let root = unique_runtime_temp_dir("hud-bundle-selected-special-item");
    write_runtime_special_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bundle"),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            ],
            bundle_contents_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: item_runtime
                    .item_protocol_id("minecraft:special_trident")
                    .unwrap(),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let world = WorldStore::new();
    let terrain = TerrainTextureState::default();

    assert!(
        hud_block_item_model(
            &HudItemModelContext::new(&world, Some(&item_runtime)),
            &stack,
            HudItemSlotState::default(),
            world.local_player_team_color_rgb(),
            &terrain,
        )
        .is_none(),
        "closed bundle stays on its generated compatibility icon"
    );
    let model = hud_block_item_model(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &stack,
        HudItemSlotState {
            bundle_selected_item_index: Some(0),
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
        &terrain,
    )
    .expect("selected bundle child should retain its special leaf");
    assert!(model
        .layers
        .iter()
        .any(|layer| matches!(layer, HudResidentItemLayer::Special { .. })));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_selected_bundle_child_layers() {
    let root = unique_runtime_temp_dir("inventory-bundle-selected-special-item");
    write_runtime_special_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let bundle = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bundle"),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            ],
            bundle_contents_items: vec![bbb_protocol::packets::ItemStackTemplateSummary {
                item_id: item_runtime
                    .item_protocol_id("minecraft:special_trident")
                    .unwrap(),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let mut world = WorldStore::new();
    assert!(
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: bundle,
        })
    );
    assert!(world.open_local_inventory());
    assert!(world.apply_local_select_bundle_item(36, 0));

    let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0)
        .expect("local inventory screen");
    let slot = screen
        .slots
        .iter()
        .find(|slot| slot.slot_id == 36)
        .expect("selected bundle slot");
    let model = slot
        .block_model
        .as_ref()
        .expect("selected bundle child resident model");
    assert!(matches!(
        model.layers.as_ref(),
        [
            HudResidentItemLayer::Geometry { .. },
            HudResidentItemLayer::Special {
                model: bbb_client_render::SpecialItemModel::Trident { .. },
                ..
            },
            HudResidentItemLayer::Geometry { .. },
        ]
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn gui_item_model_reuses_live_carried_context_for_special_leaf() {
    let root = unique_runtime_temp_dir("hud-carried-special-item");
    write_runtime_special_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(
        item_runtime
            .item_protocol_id("minecraft:carried_special")
            .unwrap(),
        1,
    );
    let world = WorldStore::new();
    let terrain = TerrainTextureState::default();

    assert!(
        hud_block_item_model(
            &HudItemModelContext::new(&world, Some(&item_runtime)),
            &stack,
            HudItemSlotState::default(),
            world.local_player_team_color_rgb(),
            &terrain,
        )
        .is_none(),
        "ordinary slot resolves the generated false branch"
    );
    let model = hud_block_item_model(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &stack,
        HudItemSlotState {
            carried_item: true,
            ..HudItemSlotState::default()
        },
        world.local_player_team_color_rgb(),
        &terrain,
    )
    .expect("cursor item should resolve the carried special branch");
    assert!(matches!(
        model.layers.as_ref(),
        [HudResidentItemLayer::Special {
            model: bbb_client_render::SpecialItemModel::Bell { .. },
            ..
        }]
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_cursor_item_as_carried_floating_item() {
    let root = unique_runtime_temp_dir("hud-cursor-carried-condition");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 3);
    let normal_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let carried_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                carried_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_cursor_item(ProtocolSetCursorItem {
        item: stack.clone(),
    });
    assert!(world.open_local_inventory());

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((40, 60)),
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
    assert_eq!(
        hotbar.icon.as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: normal_uv.min,
            max: normal_uv.max,
        }
    );
    assert_eq!(screen.floating_items.len(), 1);
    let cursor = &screen.floating_items[0];
    assert_eq!((cursor.x, cursor.y), (32, 52));
    assert_eq!(
        cursor.icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(cursor.icon.count_label, Some(HudItemCountLabel::new("3")));
    assert!(cursor.block_model.is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hud_inventory_screen_projects_quick_craft_cursor_remainder() {
    let root = unique_runtime_temp_dir("hud-quick-craft-cursor-remainder");
    write_runtime_carried_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 11);
    let carried_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                carried_item: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;

    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    world.apply_set_cursor_item(ProtocolSetCursorItem { item: stack });

    let screen = hud_inventory_screen_with_local_state(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        InventoryHudLocalState {
            cursor_position: Some((40, 60)),
            quick_craft_button_num: Some(0),
            quick_craft_slots: vec![9, 10, 11],
            ..InventoryHudLocalState::default()
        },
        0.0,
    )
    .unwrap();

    assert_eq!(screen.floating_items.len(), 1);
    let cursor = &screen.floating_items[0];
    assert_eq!((cursor.x, cursor.y), (32, 52));
    assert_eq!(
        cursor.icon.layers[0].uv,
        HudUvRect {
            min: carried_uv.min,
            max: carried_uv.max,
        }
    );
    assert_eq!(cursor.icon.count_label, Some(HudItemCountLabel::new("2")));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_local_use_ticks_into_use_duration_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-use-duration-range-dispatch");
    write_runtime_bow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let expected_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                using_item: true,
                use_context: item_runtime.item_model_use_context_for_stack(&stack, 13),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let initial_using_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                using_item: true,
                use_context: item_runtime.item_model_use_context_for_stack(&stack, 0),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(expected_uv, initial_using_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);
    world.advance_local_using_item_ticks(13);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: expected_uv.min,
            max: expected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_apply_synced_custom_crossbow_charge_time_effect() {
    let root = unique_runtime_temp_dir("hotbar-crossbow-custom-charge-time-range-dispatch");
    write_runtime_crossbow_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: 0,
        level: 2,
    }];
    let default_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                using_item: true,
                use_context: item_runtime.item_model_use_context_for_stack(&stack, 10),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let expected_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                using_item: true,
                use_context: item_runtime
                    .item_model_use_context_for_stack_with_crossbow_charge_duration(
                        &stack,
                        10,
                        Some(15),
                    ),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(default_uv, expected_uv);

    let mut world = WorldStore::new();
    world.record_registry_data(bbb_protocol::packets::RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![bbb_protocol::packets::RegistryDataEntry {
            id: "example:patient_loading".to_string(),
            raw_data: Some(crossbow_charge_time_enchantment_nbt(0.75)),
        }],
        raw_payload_len: 0,
    });
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));
    world.set_local_using_item(true);
    world.advance_local_using_item_ticks(10);

    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);

    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: expected_uv.min,
            max: expected_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_main_hand_owner_context() {
    let root = unique_runtime_temp_dir("hotbar-main-hand");
    write_runtime_main_hand_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let right_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                owner_main_hand_left: Some(false),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let left_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                owner_main_hand_left: Some(true),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, right_uv);
    assert_ne!(fallback_uv, left_uv);
    assert_ne!(right_uv, left_uv);

    let mut no_owner_world = WorldStore::new();
    no_owner_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_owner_icons = hotbar_item_icons(&no_owner_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_owner_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_add_entity(test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let right_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        right_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: right_uv.min,
            max: right_uv.max,
        }
    );

    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 42,
            values: vec![test_humanoid_arm_data(15, 0)],
        })
    );
    let left_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        left_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: left_uv.min,
            max: left_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn gui_item_team_tint_distinguishes_local_player_and_fake_item_owners() {
    use bbb_protocol::packets::{
        ChatFormatting, PlayerTeamMethod, PlayerTeamParameters, SetPlayerTeam, TeamCollisionRule,
        TeamVisibility,
    };

    let root = unique_runtime_temp_dir("gui-item-team-tint");
    write_runtime_team_tint_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let rgb_tint = |rgb: u32| {
        [
            ((rgb >> 16) & 0xff) as f32 / 255.0,
            ((rgb >> 8) & 0xff) as f32 / 255.0,
            (rgb & 0xff) as f32 / 255.0,
            1.0,
        ]
    };
    let mut world = world_with_dimension(0, "minecraft:overworld");
    let player = test_add_entity(42, VANILLA_ENTITY_TYPE_PLAYER_ID);
    let player_uuid = player.uuid;
    world.apply_add_entity(player);
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "green".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![player_uuid.to_string().into()],
    }));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });

    let hotbar = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        hotbar[0].as_ref().unwrap().layers[0].tint,
        rgb_tint(0x55ff55)
    );

    let fake = hud_item_icon(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        &stack,
        HudItemSlotState::default(),
        None,
        0.0,
    )
    .unwrap();
    assert_eq!(fake.layers[0].tint, rgb_tint(0x123456));
    assert!(inventory_screen_slot_uses_fake_item(
        InventoryScreenBackground::LocalInventory,
        0
    ));
    assert!(inventory_screen_slot_uses_fake_item(
        InventoryScreenBackground::CraftingTable,
        0
    ));
    assert!(inventory_screen_slot_uses_fake_item(
        InventoryScreenBackground::Crafter,
        45
    ));
    assert!(!inventory_screen_slot_uses_fake_item(
        InventoryScreenBackground::LocalInventory,
        1
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_world_dimension_select_context() {
    let root = unique_runtime_temp_dir("hotbar-context-dimension");
    write_runtime_context_dimension_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let overworld_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_dimension: Some("minecraft:overworld"),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let nether_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_dimension: Some("minecraft:the_nether"),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, overworld_uv);
    assert_ne!(fallback_uv, nether_uv);
    assert_ne!(overworld_uv, nether_uv);

    let mut no_level_world = WorldStore::new();
    no_level_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_level_icons = hotbar_item_icons(&no_level_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_level_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut overworld = world_with_dimension(0, "minecraft:overworld");
    overworld.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let overworld_icons = hotbar_item_icons(&overworld, Some(&item_runtime), 0.0);
    assert_eq!(
        overworld_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: overworld_uv.min,
            max: overworld_uv.max,
        }
    );

    let mut nether = world_with_dimension(1, "minecraft:the_nether");
    nether.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let nether_icons = hotbar_item_icons(&nether, Some(&item_runtime), 0.0);
    assert_eq!(
        nether_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: nether_uv.min,
            max: nether_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_context_entity_type_select() {
    let root = unique_runtime_temp_dir("hotbar-context-entity-type");
    write_runtime_context_entity_type_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let player_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let cow_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:cow"),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, player_uv);
    assert_ne!(fallback_uv, cow_uv);
    assert_ne!(player_uv, cow_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: player_uv.min,
            max: player_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_local_player_view_entity_condition() {
    let root = unique_runtime_temp_dir("hotbar-view-entity-condition");
    write_runtime_view_entity_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let view_entity_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                view_entity: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, view_entity_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: view_entity_uv.min,
            max: view_entity_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_extended_view_condition_when_shift_is_down() {
    let root = unique_runtime_temp_dir("hotbar-extended-view-condition");
    write_runtime_extended_view_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let extended_view_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                view_entity: true,
                shift_down: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, extended_view_uv);
    let ground_shift_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                view_entity: true,
                shift_down: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Ground,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(fallback_uv, ground_shift_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let inactive_icons =
        hotbar_item_icons_with_extended_view(&world, Some(&item_runtime), 0.0, false);
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );
    let active_icons = hotbar_item_icons_with_extended_view(&world, Some(&item_runtime), 0.0, true);
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: extended_view_uv.min,
            max: extended_view_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_keybind_down_condition_when_default_key_is_down() {
    let root = unique_runtime_temp_dir("hotbar-keybind-down-condition");
    write_runtime_keybind_down_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let keybind_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                view_entity: true,
                keybind: ItemModelKeybindContext {
                    quick_actions: true,
                    ..ItemModelKeybindContext::default()
                },
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, keybind_uv);
    let unrelated_key_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                view_entity: true,
                keybind: ItemModelKeybindContext {
                    forward: true,
                    ..ItemModelKeybindContext::default()
                },
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(fallback_uv, unrelated_key_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let inactive_icons = hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        0.0,
        false,
        ItemModelKeybindContext::default(),
    );
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );
    let active_icons = hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        0.0,
        false,
        ItemModelKeybindContext {
            quick_actions: true,
            ..ItemModelKeybindContext::default()
        },
    );
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: keybind_uv.min,
            max: keybind_uv.max,
        }
    );
    assert_eq!(inactive_icons[0].as_ref().unwrap().pop, 0.0);

    // `ClientPacketListener.handleContainerSetSlot` sets `popTime` to 5 when a
    // hotbar slot's count grows, and `Gui.extractSlot` subtracts this frame's
    // `getGameTimeDeltaPartialTick(false)` from it.
    assert!(
        world.apply_container_set_slot(bbb_protocol::packets::ContainerSetSlot {
            container_id: 0,
            state_id: 1,
            slot: 36,
            item: item_stack(0, 3),
        })
    );
    let popped_icons = hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        0.25,
        false,
        ItemModelKeybindContext::default(),
    );
    assert_eq!(popped_icons[0].as_ref().unwrap().pop, 4.75);
    world.advance_hotbar_pop_times(5);
    let settled_icons = hotbar_item_icons_with_input_context(
        &HudItemModelContext::new(&world, Some(&item_runtime)),
        0.25,
        false,
        ItemModelKeybindContext::default(),
    );
    assert_eq!(settled_icons[0].as_ref().unwrap().pop, -0.25);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_fishing_rod_cast_condition_for_selected_local_rod() {
    let root = unique_runtime_temp_dir("hotbar-fishing-rod-cast-condition");
    write_runtime_fishing_rod_cast_condition_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let cast_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                selected_item: true,
                view_entity: true,
                fishing_rod_cast: true,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, cast_uv);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 1,
        item: stack,
    });
    assert!(world.set_local_selected_hotbar_slot(0));

    let no_bobber_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_bobber_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    world.apply_add_entity(bbb_protocol::packets::AddEntity {
        data: 42,
        ..test_add_entity(700, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: cast_uv.min,
            max: cast_uv.max,
        }
    );
    assert_eq!(
        icons[1].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_use_vanilla_zero_partial_cooldown_model_property() {
    let root = unique_runtime_temp_dir("hotbar-cooldown-range-dispatch");
    write_runtime_cooldown_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut stack = item_stack(0, 1);
    stack.component_patch.use_cooldown_group = Some("minecraft:ender_pearl".to_string());
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let active_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                cooldown_progress: 0.75,
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_eq!(
        item_runtime
            .resolve_item_model(
                &stack,
                &bbb_client_render::item_model::ItemModelContext {
                    cooldown_progress: 0.7,
                    ..bbb_client_render::item_model::ItemModelContext::new(
                        bbb_client_assets::BlockModelDisplayContext::Gui
                    )
                }
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv,
        fallback_uv
    );
    assert_ne!(fallback_uv, active_uv);

    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let inactive_icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        inactive_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    world.apply_cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    });
    world.advance_item_cooldowns(5);
    let active_icons = hotbar_item_icons(&world, Some(&item_runtime), 1.0);
    assert_eq!(
        active_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: active_uv.min,
            max: active_uv.max,
        }
    );
    assert_eq!(
        active_icons[0].as_ref().unwrap().cooldown_progress,
        Some(0.7)
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_world_time_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-time-range-dispatch");
    write_runtime_time_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let selected_at_time = |stack: &bbb_protocol::packets::ItemStackSummary, day_time| {
        item_runtime
            .resolve_item_model(
                stack,
                &bbb_client_render::item_model::ItemModelContext {
                    context_entity_type: Some("minecraft:player"),
                    context_dimension: Some("minecraft:overworld"),
                    time: Some(bbb_client_render::item_model::ItemModelTimeContext {
                        game_time: day_time,
                        day_time,
                        sun_angle_degrees: None,
                        moon_phase_index: None,
                    }),
                    ..bbb_client_render::item_model::ItemModelContext::new(
                        bbb_client_assets::BlockModelDisplayContext::Gui,
                    )
                },
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    };
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let night_uv = selected_at_time(&stack, 18_000);
    assert_ne!(fallback_uv, night_uv);

    let mut no_time_world = WorldStore::new();
    no_time_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_time_icons = hotbar_item_icons(&no_time_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_time_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut world, 18_000);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: night_uv.min,
            max: night_uv.max,
        }
    );

    let wobbled_stack = item_stack(1, 1);
    let wobbled_fallback_uv = item_runtime
        .resolve_item_model(
            &wobbled_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let wobbled_uv = selected_at_time(&wobbled_stack, 18_000);
    assert_ne!(wobbled_uv, wobbled_fallback_uv);

    let random_stack = item_stack(2, 1);
    let random_fallback_uv = item_runtime
        .resolve_item_model(
            &random_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let random_uv = selected_at_time(&random_stack, 18_000);
    assert_ne!(random_uv, random_fallback_uv);

    let moon_phase_stack = item_stack(3, 1);
    let moon_phase_fallback_uv = item_runtime
        .resolve_item_model(
            &moon_phase_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let new_moon_uv = selected_at_time(&moon_phase_stack, 96_000);
    assert_ne!(new_moon_uv, moon_phase_fallback_uv);

    let mut moon_phase_world = world_with_dimension(0, "minecraft:overworld");
    set_world_day_time(&mut moon_phase_world, 96_000);
    moon_phase_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: moon_phase_stack,
    });
    let moon_phase_icons = hotbar_item_icons(&moon_phase_world, Some(&item_runtime), 0.0);
    assert_eq!(
        moon_phase_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: new_moon_uv.min,
            max: new_moon_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_spawn_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-spawn-compass-range-dispatch");
    write_runtime_spawn_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let east_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                context_dimension: Some("minecraft:overworld"),
                compass: Some(bbb_client_render::item_model::ItemModelCompassContext {
                    game_time: 18_000,
                    level_dimension: "minecraft:overworld",
                    owner_position: [0.5, 64.0, 0.5],
                    owner_y_rot_degrees: 0.0,
                    spawn: Some(bbb_client_render::item_model::ItemModelCompassTarget {
                        dimension: "minecraft:overworld",
                        pos: [10, 64, 0],
                    }),
                    recovery: None,
                }),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let wobbled_stack = item_stack(1, 1);
    let wobbled_fallback_uv = item_runtime
        .resolve_item_model(
            &wobbled_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let wobbled_east_uv = item_runtime
        .resolve_item_model(
            &wobbled_stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                context_dimension: Some("minecraft:overworld"),
                compass: Some(bbb_client_render::item_model::ItemModelCompassContext {
                    game_time: 18_000,
                    level_dimension: "minecraft:overworld",
                    owner_position: [0.5, 64.0, 0.5],
                    owner_y_rot_degrees: 0.0,
                    spawn: Some(bbb_client_render::item_model::ItemModelCompassTarget {
                        dimension: "minecraft:overworld",
                        pos: [10, 64, 0],
                    }),
                    recovery: None,
                }),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(wobbled_fallback_uv, wobbled_east_uv);

    let invalid_spin_stack = item_stack(2, 1);
    let invalid_spin_fallback_uv = item_runtime
        .resolve_item_model(
            &invalid_spin_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let mut invalid_spin_world = world_with_dimension(0, "minecraft:overworld");
    invalid_spin_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(&mut invalid_spin_world, "minecraft:the_nether", [10, 64, 0]);
    invalid_spin_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: invalid_spin_stack,
    });
    let invalid_spin_icons = hotbar_item_icons(&invalid_spin_world, Some(&item_runtime), 0.0);
    assert_ne!(
        invalid_spin_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: invalid_spin_fallback_uv.min,
            max: invalid_spin_fallback_uv.max,
        }
    );

    let no_target_spin_stack = item_stack(3, 1);
    let no_target_spin_fallback_uv = item_runtime
        .resolve_item_model(
            &no_target_spin_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let mut no_target_spin_world = world_with_dimension(0, "minecraft:overworld");
    no_target_spin_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    no_target_spin_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: no_target_spin_stack,
    });
    let no_target_spin_icons = hotbar_item_icons(&no_target_spin_world, Some(&item_runtime), 0.0);
    assert_ne!(
        no_target_spin_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: no_target_spin_fallback_uv.min,
            max: no_target_spin_fallback_uv.max,
        }
    );

    let mut no_pose_world = world_with_dimension(0, "minecraft:overworld");
    set_default_spawn(&mut no_pose_world, "minecraft:overworld", [10, 64, 0]);
    no_pose_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_pose_icons = hotbar_item_icons(&no_pose_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_pose_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(&mut world, "minecraft:overworld", [10, 64, 0]);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut wrong_dimension_world = world_with_dimension(0, "minecraft:overworld");
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    set_default_spawn(
        &mut wrong_dimension_world,
        "minecraft:the_nether",
        [10, 64, 0],
    );
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let fallback_uv = HudUvRect {
        min: fallback_uv.min,
        max: fallback_uv.max,
    };
    let east_uv = HudUvRect {
        min: east_uv.min,
        max: east_uv.max,
    };
    let mut selected_fallback = false;
    let mut selected_east = false;
    for _ in 0..32 {
        let icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
        let uv = icons[0].as_ref().unwrap().layers[0].uv;
        assert!(uv == fallback_uv || uv == east_uv);
        selected_fallback |= uv == fallback_uv;
        selected_east |= uv == east_uv;
    }
    assert!(selected_fallback && selected_east);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_lodestone_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-lodestone-compass-range-dispatch");
    write_runtime_lodestone_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let lodestone_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![67],
            lodestone_target: Some(bbb_protocol::packets::LodestoneTargetSummary {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos { x: 10, y: 64, z: 0 },
            }),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let fallback_uv = item_runtime
        .resolve_item_model(
            &lodestone_stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let east_uv = item_runtime
        .resolve_item_model(
            &lodestone_stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                context_dimension: Some("minecraft:overworld"),
                compass: Some(bbb_client_render::item_model::ItemModelCompassContext {
                    game_time: 18_000,
                    level_dimension: "minecraft:overworld",
                    owner_position: [0.5, 64.0, 0.5],
                    owner_y_rot_degrees: 0.0,
                    spawn: None,
                    recovery: None,
                }),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let mut world = world_with_dimension(0, "minecraft:overworld");
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: lodestone_stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut missing_component_world = world_with_dimension(0, "minecraft:overworld");
    missing_component_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    missing_component_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: item_stack(0, 1),
    });
    let missing_component_icons =
        hotbar_item_icons(&missing_component_world, Some(&item_runtime), 0.0);
    let uv = missing_component_icons[0].as_ref().unwrap().layers[0].uv;
    assert!([fallback_uv, east_uv]
        .iter()
        .any(|sprite| { uv.min == sprite.min && uv.max == sprite.max }));

    let mut wrong_dimension_stack = lodestone_stack;
    wrong_dimension_stack
        .component_patch
        .lodestone_target
        .as_mut()
        .unwrap()
        .dimension = "minecraft:the_nether".to_string();
    let mut wrong_dimension_world = world_with_dimension(0, "minecraft:overworld");
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: wrong_dimension_stack,
    });
    let wrong_dimension_icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
    let uv = wrong_dimension_icons[0].as_ref().unwrap().layers[0].uv;
    assert!([fallback_uv, east_uv]
        .iter()
        .any(|sprite| { uv.min == sprite.min && uv.max == sprite.max }));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hotbar_item_icons_project_recovery_compass_range_dispatch() {
    let root = unique_runtime_temp_dir("hotbar-recovery-compass-range-dispatch");
    write_runtime_recovery_compass_range_dispatch_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = item_stack(0, 1);
    let fallback_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(
                bbb_client_assets::BlockModelDisplayContext::Gui,
            ),
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    let east_uv = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext {
                context_entity_type: Some("minecraft:player"),
                context_dimension: Some("minecraft:overworld"),
                compass: Some(bbb_client_render::item_model::ItemModelCompassContext {
                    game_time: 18_000,
                    level_dimension: "minecraft:overworld",
                    owner_position: [0.5, 64.0, 0.5],
                    owner_y_rot_degrees: 0.0,
                    spawn: None,
                    recovery: Some(bbb_client_render::item_model::ItemModelCompassTarget {
                        dimension: "minecraft:overworld",
                        pos: [10, 64, 0],
                    }),
                }),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    bbb_client_assets::BlockModelDisplayContext::Gui,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .unwrap()
        .layers[0]
        .uv;
    assert_ne!(fallback_uv, east_uv);

    let mut no_pose_world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:overworld", [10, 64, 0])),
    );
    no_pose_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let no_pose_icons = hotbar_item_icons(&no_pose_world, Some(&item_runtime), 0.0);
    assert_eq!(
        no_pose_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:overworld", [10, 64, 0])),
    );
    world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let icons = hotbar_item_icons(&world, Some(&item_runtime), 0.0);
    assert_eq!(
        icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: east_uv.min,
            max: east_uv.max,
        }
    );

    let mut missing_recovery_world =
        world_with_dimension_last_death_location(0, "minecraft:overworld", None);
    missing_recovery_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    missing_recovery_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack.clone(),
    });
    let missing_recovery_icons =
        hotbar_item_icons(&missing_recovery_world, Some(&item_runtime), 0.0);
    assert_eq!(
        missing_recovery_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    let mut wrong_dimension_world = world_with_dimension_last_death_location(
        0,
        "minecraft:overworld",
        Some(("minecraft:the_nether", [10, 64, 0])),
    );
    wrong_dimension_world.set_local_player_pose(local_player_pose([0.5, 64.0, 0.5], 0.0, 0.0));
    wrong_dimension_world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    let wrong_dimension_icons = hotbar_item_icons(&wrong_dimension_world, Some(&item_runtime), 0.0);
    assert_eq!(
        wrong_dimension_icons[0].as_ref().unwrap().layers[0].uv,
        HudUvRect {
            min: fallback_uv.min,
            max: fallback_uv.max,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dropped_item_models_use_world_trim_material_select_context() {
    let root = unique_runtime_temp_dir("dropped-trim-material-select");
    write_runtime_trim_material_select_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    record_trim_material_registry(&mut world);
    world.apply_add_entity(test_add_entity(700, 71));
    let mut stack = item_stack(0, 1);
    stack.component_patch.armor_trim_material_id = Some(0);
    assert!(
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 700,
            values: vec![test_item_stack_data(8, stack)],
        })
    );
    let fallback = crate::item_models::dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    let trim_material_keys = crate::item_models::world_trim_material_keys(&world).unwrap();
    let trimmed = crate::item_models::dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet {
            trim_materials: Some(trim_material_keys.clone()),
            ..Default::default()
        },
    );
    assert_eq!(fallback.residency.assets.len(), 1);
    assert_eq!(trimmed.residency.assets.len(), 1);
    assert_eq!(fallback.handled_entity_ids, trimmed.handled_entity_ids);
    assert_ne!(fallback.residency.assets, trimmed.residency.assets);

    std::fs::remove_dir_all(root).unwrap();
}
