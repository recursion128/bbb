use super::*;

fn click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    slot_num: i16,
    button_num: i8,
) -> TickResult {
    let inventory = world.player_inventory_for_test(ConnectionId(1));
    let (container_id, state_id) = (inventory.container_id(), inventory.state_id());
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num,
            button_num,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    )
}

fn hotbar(world: &mut ServerWorld, slot: usize) -> (Option<i32>, i32) {
    let item = world
        .player_inventory_for_test(ConnectionId(1))
        .inventory_slots()[slot]
        .clone();
    (item.item_id(), item.count())
}

/// `CraftingTableBlock.useWithoutItem` opens `CraftingMenu`: its 3x3 grid
/// matches a shaped recipe, taking the result consumes one of each input and
/// learns the recipe, and closing the menu (by packet or by losing the block)
/// hands the rest of the grid back to the inventory.
#[test]
fn a_crafting_table_crafts_a_three_by_three_recipe_and_returns_the_grid_on_close() {
    use bbb_protocol::item_types::{VANILLA_ITEM_CHEST_ID, VANILLA_ITEM_OAK_PLANKS_ID};
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(recipe_crafted::config(
        datapack_config(),
        "minecraft:chest",
        serde_json::json!([{ "items":"minecraft:oak_planks", "count":2 }]),
    ));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_OAK_PLANKS_ID),
            count: 64,
            component_patch: Default::default(),
        },
    );
    let dimension = Java26BuiltinDimension::Overworld;
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let table = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_CRAFTING_TABLE_ID,
    )
    .unwrap();
    assert!(world.set_block(dimension, at, table, SET_BLOCK_UPDATE_ALL));

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("the crafting table opens its menu");
    assert_eq!(screen.container_id, 1);
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_CRAFTING_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.crafting",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap()
    );
    let contents = container_contents(&opened);
    assert_eq!(contents[0].items.len(), 46);
    // The first hotbar slot is `CraftingMenu.USE_ROW_SLOT_START`.
    assert_eq!(contents[0].items[37].count, 64);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_crafting_table"),
        ),
        1
    );

    // Two planks in each ring cell of the grid: `minecraft:chest`.
    click(&sender, &receiver, &mut world, 37, 0);
    let mut last = None;
    for slot in [1, 2, 3, 4, 6, 7, 8, 9] {
        click(&sender, &receiver, &mut world, slot, 1);
        last = Some(click(&sender, &receiver, &mut world, slot, 1));
    }
    let published = last.unwrap().events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.container_id == 1
                && update.slot == 0
                && update.item.item_id == Some(VANILLA_ITEM_CHEST_ID)
        )
    });
    assert!(published, "the result slot is synced as the grid fills");
    click(&sender, &receiver, &mut world, 37, 0);

    let taken = click(&sender, &receiver, &mut world, 0, 0);
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:crafted"));
    assert!(world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:chest"));
    assert!(taken.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::RecipeBookAdd(add),
            ..
        } if add.entries.iter().any(|entry| entry.notification)
    )));
    assert!(
        world.player_advancement_is_done(ConnectionId(1), "minecraft:recipes/decorations/chest")
    );
    click(&sender, &receiver, &mut world, 38, 0);
    assert_eq!(hotbar(&mut world, 1), (Some(VANILLA_ITEM_CHEST_ID), 1));
    assert_eq!(
        hotbar(&mut world, 0),
        (Some(VANILLA_ITEM_OAK_PLANKS_ID), 48)
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
    );
    assert_eq!(
        hotbar(&mut world, 0),
        (Some(VANILLA_ITEM_OAK_PLANKS_ID), 56)
    );

    // A table that disappears under an open menu closes it on the next tick
    // and still hands the grid back.
    right_click(&sender, &receiver, &mut world, 1, at, 2);
    let container_id = world
        .player_inventory_for_test(ConnectionId(1))
        .container_id();
    assert_eq!(container_id, 2);
    for (slot_num, button_num) in [(37, 0), (5, 1), (37, 0)] {
        click(&sender, &receiver, &mut world, slot_num, button_num);
    }
    assert_eq!(
        hotbar(&mut world, 0),
        (Some(VANILLA_ITEM_OAK_PLANKS_ID), 55)
    );
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    assert!(world.set_block(dimension, at, air, SET_BLOCK_UPDATE_ALL));
    let closed = world.tick(receiver.drain());
    assert!(closed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerClose(close),
            ..
        } if close.container_id == 2
    )));
    assert_eq!(
        hotbar(&mut world, 0),
        (Some(VANILLA_ITEM_OAK_PLANKS_ID), 56)
    );
}

fn map_extending_fixture(
    mut world: ServerWorld,
    data: Option<bbb_server_persistence::Java26MapSavedData>,
    exploration: bool,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    };
    use bbb_protocol::item_types::{VANILLA_ITEM_FILLED_MAP_ID, VANILLA_ITEM_PAPER_ID};
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    if let Some(data) = data {
        world.install_map_for_test(5, &data, true).unwrap();
    }
    let resolver = world.item_stack_resolver();
    let mut source =
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_FILLED_MAP_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID],
                map_id: Some(5),
                ..Default::default()
            }
            .into(),
        })
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            Some(&NbtSummaryValue::String("Survey".into())),
            &resolver,
        )
        .unwrap();
    if exploration {
        source = source
            .with_component_patch(
                &DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID],
                    map_decorations: Some(vec![bbb_protocol::packets::ItemMapDecorationSummary {
                        id: "destination".into(),
                        decoration_type_id: bbb_facts::vanilla_map_decoration_type_for_resource_id(
                            "minecraft:mansion",
                        )
                        .unwrap()
                        .type_id,
                        decoration_type_resource_id: "minecraft:mansion".into(),
                        x_bits: 64_f64.to_bits(),
                        z_bits: (-64_f64).to_bits(),
                        rotation_bits: 0_f32.to_bits(),
                    }]),
                    ..Default::default()
                },
                &resolver,
            )
            .unwrap();
    }
    world.stock_menu_slot_for_test(ConnectionId(1), 36, source.summary().clone());
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        37,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_PAPER_ID),
            count: 8,
            component_patch: Default::default(),
        },
    );
    // Carried map decorations enter the actual saved map before the recipe reads it.
    world.tick(Vec::new());
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let table = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_CRAFTING_TABLE_ID,
    )
    .unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        table,
        SET_BLOCK_UPDATE_ALL
    ));
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    click(&sender, &receiver, &mut world, 37, 0);
    click(&sender, &receiver, &mut world, 5, 0);
    click(&sender, &receiver, &mut world, 38, 0);
    for slot in [1, 2, 3, 4, 6, 7, 8, 9] {
        click(&sender, &receiver, &mut world, slot, 1);
    }
    (sender, receiver, world)
}

fn extending_map_data(scale: i8, locked: bool) -> bbb_server_persistence::Java26MapSavedData {
    bbb_server_persistence::Java26MapSavedData {
        dimension: "minecraft:overworld".into(),
        center_x: 64,
        center_z: -64,
        scale,
        colors: vec![23; 128 * 128],
        tracking_position: true,
        unlimited_tracking: false,
        locked,
        banners: Vec::new(),
        frames: Vec::new(),
    }
}

#[test]
fn map_extending_result_exits_scale_saved_maps_and_survive_restart() {
    use bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID;
    for input in [
        ContainerInput::Pickup,
        ContainerInput::QuickMove,
        ContainerInput::Throw,
        ContainerInput::Swap,
    ] {
        let root = std::env::temp_dir().join(format!("bbb-map-extending-{}", uuid::Uuid::new_v4()));
        let storage = created_fixture_save(&root, "map extending");
        let baseline = storage.load_candidate().unwrap();
        let config = ServerWorldConfig {
            game_type: GameType::Survival,
            ..datapack_config()
        };
        let world = spawn_pinned_restore(config.clone(), &baseline);
        let original = extending_map_data(1, true);
        let (sender, receiver, mut world) =
            map_extending_fixture(world, Some(original.clone()), false);
        let inventory = world.player_inventory_for_test(ConnectionId(1));
        let preview = inventory.menu_items().unwrap()[0].clone();
        assert_eq!(preview.component_patch.map_id, Some(5));
        assert_eq!(
            preview.component_patch.map_post_processing,
            Some(bbb_protocol::packets::MapPostProcessingSummary::Scale)
        );
        let (container_id, state_id) = (inventory.container_id(), inventory.state_id());
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id,
                state_id,
                slot_num: 0,
                button_num: 0,
                input,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        let inventory = world.player_inventory_for_test(ConnectionId(1));
        assert!(inventory.menu_items().unwrap()[..10]
            .iter()
            .all(|item| item.item_id.is_none()));
        let result = match input {
            ContainerInput::Pickup => inventory.carried().clone(),
            ContainerInput::QuickMove | ContainerInput::Swap => inventory
                .inventory_slots()
                .into_iter()
                .find(|item| item.item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID))
                .unwrap(),
            ContainerInput::Throw => world
                .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
                .into_iter()
                .find_map(|id| {
                    let entity =
                        world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
                    let crate::entity::EntityData::Item(item) = entity.data() else {
                        return None;
                    };
                    (item.item().item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID))
                        .then(|| item.item().clone())
                })
                .unwrap(),
            _ => unreachable!(),
        };
        let new_id = result.map_id().unwrap();
        assert_ne!(new_id, 5);
        assert_eq!(result.summary().component_patch.map_post_processing, None);
        assert_eq!(
            result.summary().component_patch.custom_name.as_deref(),
            Some("Survey")
        );
        if input == ContainerInput::Pickup {
            click(&sender, &receiver, &mut world, 37, 0);
        }
        let request = world
            .save_request(1_786_000_000_020, 120, RegionCompression::Zlib)
            .unwrap();
        assert_eq!(request.map_writes[&5], original);
        assert_eq!(request.map_writes[&new_id].scale, 2);
        assert!(!request.map_writes[&new_id].locked);
        // A held map may already have drawn terrain, never brightness 3.
        assert!(request.map_writes[&new_id]
            .colors
            .iter()
            .all(|color| *color != 23));
        storage.save_snapshot(&baseline, &request).unwrap();
        if input == ContainerInput::Pickup {
            let reopened = storage.load_candidate().unwrap();
            assert_eq!(reopened.load_map(new_id).unwrap().unwrap().scale, 2);
            let mut restored = spawn_pinned_restore(config, &reopened);
            join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
            let item = restored
                .player_inventory_for_test(ConnectionId(1))
                .inventory_slots()
                .into_iter()
                .find(|item| item.map_id() == Some(new_id))
                .unwrap();
            assert_eq!(
                item.summary().component_patch.custom_name.as_deref(),
                Some("Survey")
            );
            assert_eq!(item.summary().component_patch.map_post_processing, None);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn map_extending_rejects_missing_max_scale_and_saved_exploration_maps() {
    for (label, data, exploration) in [
        ("missing", None, false),
        ("maximum scale", Some(extending_map_data(4, false)), false),
        ("exploration", Some(extending_map_data(0, false)), true),
    ] {
        let world = spawn_pinned_world(datapack_config());
        let (_, _, mut world) = map_extending_fixture(world, data, exploration);
        assert!(
            world
                .player_inventory_for_test(ConnectionId(1))
                .menu_items()
                .unwrap()[0]
                .item_id
                .is_none(),
            "{label}"
        );
    }
}
