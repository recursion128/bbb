//! Containers and their menus: chests, barrels, droppers, shulker boxes,
//! lecterns, chiseled bookshelves, decorated pots and shelves.

use super::*;

mod brewing_stand;
mod crafting_table;
mod ender_chest;
mod furnace;
mod loot_access;
mod menu_close;
mod recipe_book;
mod recipe_components;
mod recipe_crafted;
mod utility_menus;
mod vehicle;

fn chest_state(facing: &str, kind: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("type".to_string(), kind.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &properties,
    )
    .expect("minecraft:chest is a vanilla 26.1 block")
}

fn opened_screen(tick: &TickResult) -> Option<bbb_protocol::packets::OpenScreen> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenScreen(screen),
            ..
        } => Some(screen.clone()),
        _ => None,
    })
}

fn container_contents(tick: &TickResult) -> Vec<bbb_protocol::packets::ContainerSetContent> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetContent(content),
                ..
            } => Some(content.clone()),
            _ => None,
        })
        .collect()
}

/// `ChestBlock.useWithoutItem` (`ChestBlock.java:255-268`) reaching
/// `ServerPlayer.openMenu` (`ServerPlayer.java:1319-1342`): the second menu
/// this server can open.
///
/// The order is vanilla's: the open-screen packet goes out at `:1337`, before
/// `initMenu` (`:1338`) hands the synchronizer the whole menu. The container
/// id is `nextContainerCounter`'s first answer (`:1314-1316`), which is one
/// and not `InventoryMenu`'s zero.
/// `isValidSlotIndex` (`AbstractContainerMenu.java:119-121`) accepts two indexes
/// that address no slot at all, and `SLOT_CLICKED_OUTSIDE` (`:40`) is the one a
/// client sends to throw its carried stack away (`:404-412`). Nothing on the
/// click path may read a slot for it, so this drives the sentinel through the
/// handler and out the other side.
#[test]
fn a_click_outside_the_window_reads_no_slot() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID,
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for (slot_num, item_id) in [(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: Default::default(),
                },
            }),
        );
    }

    let outside = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: crate::inventory::SLOT_CLICKED_OUTSIDE,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    // An empty carried stack makes the branch a no-op (`:405`), so the menu is
    // exactly where it was and the session is still open.
    assert!(
        !outside
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{outside:?}"
    );
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
}

#[test]
fn a_right_clicked_chest_opens_a_three_row_menu_over_its_block_entity() {
    let (sender, receiver) = mailbox();
    let table = "minecraft:chests/igloo_chest";
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        table,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [{
                "rolls": 1,
                "bonus_rolls": 1,
                "conditions": [{"condition": "minecraft:entity_properties", "entity": "this",
                    "predicate": {"type": "minecraft:player", "movement": {
                        "x": {"min": 4.0, "max": 6.0}
                    }, "equipment": {
                        "mainhand": {"items": "minecraft:stone", "count": {"min": 1}}
                    }}}],
                "entries": [{"type": "minecraft:item", "name": "minecraft:stone"}]
            }]
        }),
    ));
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest_state("north", "single"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    // The chest starts with one stack in it so the initial content proves the
    // block entity is what the menu was built over.
    let stone = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 5,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut items = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
    items[7] = crate::tests::native_stack(stone.clone());
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest_mut)
        .expect("the write created the chest's block entity")
        .set_items(items));

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a chest opens its screen");
    assert_eq!(screen.container_id, 1);
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.chest",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`ChestBlockEntity.DEFAULT_NAME` (`ChestBlockEntity.java:30`)"
    );

    let contents = container_contents(&opened);
    assert_eq!(contents.len(), 1, "{contents:?}");
    assert_eq!(contents[0].container_id, 1);
    assert_eq!(contents[0].items.len(), 63);
    assert_eq!(contents[0].items[7], stone);
    // `ChestMenu`'s player rows start at `rows * 9`, and the fixture stocks
    // the first hotbar slot -- menu slot 27 + 27.
    assert_eq!(contents[0].items[54].count, 64);

    // `ContainerOpenersCounter.incrementOpeners` (`:28-38`) crosses zero, so
    // `onOpen` plays the chest's open sound (`ChestBlockEntity.java:34-38`)
    // and `openerCountChanged` raises the lid block event (`:48-50`,
    // `:195-198`).
    let sounds = sounds_sent(&opened);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.chest.open")
                .expect("a registered sound"),
        }
    );
    assert_eq!(sounds[0].volume, 0.5, "`ChestBlockEntity.java:116`");
    assert_eq!(
        block_events_sent(&opened),
        vec![bbb_protocol::packets::BlockEvent {
            pos: at,
            b0: 1,
            b1: 1,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_CHEST_ID,
        }],
        "`ChestBlock.EVENT_SET_OPEN_COUNT` with the new opener count"
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary::empty(),
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: screen.container_id,
            state_id: contents[0].state_id,
            slot_num: 7,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID,
        2.0,
    );
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .set_container_loot_table(table, 31));
    let start = world.publication().players[0].position;
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 0.25,
                ..start
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let reopened = right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(
        opened_screen(&reopened).map(|screen| screen.container_id),
        Some(2)
    );
    let new_contents = container_contents(&reopened);
    assert_eq!(new_contents[0].items[54], stone);
    assert_eq!(
        new_contents[0].items[..27]
            .iter()
            .map(|item| item.count)
            .sum::<i32>(),
        3,
        "the loot predicate sees the main hand after the old cursor is returned"
    );
    for (index, state) in [
        chest_state("north", "single"),
        barrel_state("north", false),
        shulker_box_state("up"),
        dropper_state("north", false),
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:dispenser",
            &BTreeMap::from([
                ("facing".into(), "north".into()),
                ("triggered".into(), "false".into()),
            ]),
        )
        .unwrap(),
        hopper_down_enabled(),
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_CRAFTER_ID,
        )
        .unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
        );
        world.set_block(
            Java26BuiltinDimension::Overworld,
            at,
            state,
            SET_BLOCK_UPDATE_ALL,
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, at),
            Some(state)
        );
        let entity = world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .unwrap();
        entity
            .randomizable_container_mut()
            .unwrap()
            .1
            .fill(bbb_server_types::item_stack::RuntimeItemStack::empty());
        assert!(entity.set_container_loot_table(table, 31));
        let opened = right_click(&sender, &receiver, &mut world, 1, at, index as i32 + 3);
        assert!(opened_screen(&opened).is_some(), "{opened:?}");
        let contents = container_contents(&opened);
        let size = match index {
            5 => 5,
            3 | 4 | 6 => 9,
            _ => 27,
        };
        let slots = contents[0].items[..size].to_vec();
        assert_eq!(
            slots.iter().map(|item| item.count).sum::<i32>(),
            3,
            "one base roll plus two luck rolls, with a live player predicate"
        );
        let entity = world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .unwrap();
        assert_eq!(entity.container_loot_table_id(), None);
        let record = crate::tests::save_native_fixture_block_entity(entity);
        assert!(!record.iter().any(|field| field.name == "LootTable"));
        let mut restored = crate::tests::load_native_fixture_block_entity(at, state, &record);
        assert_eq!(
            restored
                .randomizable_container_mut()
                .unwrap()
                .1
                .iter()
                .map(|stack| stack.summary().clone())
                .collect::<Vec<_>>(),
            slots,
        );
        if index == 5 || index == 6 {
            assert_eq!(
                opened_screen(&opened).unwrap().menu_type_id,
                if index == 5 {
                    bbb_protocol::menu_types::VANILLA_MENU_TYPE_HOPPER_ID
                } else {
                    bbb_protocol::menu_types::VANILLA_MENU_TYPE_CRAFTER_3X3_ID
                }
            );
            assert_eq!(contents[0].items.len(), if index == 5 { 41 } else { 46 });
            if index == 6 {
                assert_eq!(
                    opened
                        .events
                        .iter()
                        .filter(|event| matches!(
                            event,
                            WorldEvent::Send {
                                packet: ClientboundPlayPacket::ContainerSetData(_),
                                ..
                            }
                        ))
                        .count(),
                    10
                );
            }
            let slot = slots.iter().position(|item| item.count > 0).unwrap();
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::ContainerClick(ContainerClick {
                    container_id: contents[0].container_id,
                    state_id: contents[0].state_id,
                    slot_num: slot as i16,
                    button_num: 0,
                    input: ContainerInput::QuickMove,
                    changed_slots: BTreeMap::new(),
                    carried_item: HashedStack::empty(),
                }),
            );
            let entity = world
                .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
                .unwrap();
            assert_eq!(
                entity.randomizable_container_mut().unwrap().1[slot],
                bbb_server_types::item_stack::RuntimeItemStack::empty()
            );
            if index == 6 {
                let changed = play(
                    &sender,
                    &receiver,
                    &mut world,
                    1,
                    PlayServerbound::ContainerSlotStateChanged(
                        bbb_protocol::packets::ContainerSlotStateChanged {
                            slot_id: slot as i32,
                            container_id: contents[0].container_id,
                            new_state: false,
                        },
                    ),
                );
                assert!(changed.events.iter().any(|event| matches!(event,
                    WorldEvent::Send { packet: ClientboundPlayPacket::ContainerSetData(data), .. }
                        if data.id == slot as i16 && data.value == 1)));
                let entity = world
                    .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
                    .unwrap();
                assert!(entity.crafter().unwrap().is_slot_disabled(slot));
                let record = crate::tests::save_native_fixture_block_entity(entity);
                let restored = crate::tests::load_native_fixture_block_entity(at, state, &record);
                assert!(restored.crafter().unwrap().is_slot_disabled(slot));
            }
        }
    }
}

/// `ServerPlayerGameMode.useItemOn` (`ServerPlayerGameMode.java:358-365`): a
/// spectator never reaches `useWithoutItem`, but it does reach the clicked
/// block's `MenuProvider`, so a spectator opens the chest it clicks.
///
/// What a spectator does not do is count as an opener:
/// `ChestBlockEntity.startOpen` (`ChestBlockEntity.java:131`) skips it, so the
/// lid stays shut and no sound plays, and `stopOpen` (`:141`) carries the same
/// gate so the close does not decrement a count that was never taken.
#[test]
fn a_spectator_opens_the_chest_menu_without_becoming_an_opener() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Spectator,
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "ghost", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest_state("north", "single"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("a spectator reaches the menu provider");
    assert_eq!(screen.container_id, 1);
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID
    );
    assert_eq!(container_contents(&opened).len(), 1);
    assert!(
        sounds_sent(&opened).is_empty(),
        "the opener count never crossed zero, so `onOpen` never ran"
    );
    assert!(block_events_sent(&opened).is_empty(), "the lid stays shut");

    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
            container_id: 1,
        }),
    );
    assert!(
        block_events_sent(&closed).is_empty() && sounds_sent(&closed).is_empty(),
        "`stopOpen` skips a spectator too, so the close decrements nothing"
    );

    let reopened = right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(opened_screen(&reopened).unwrap().container_id, 2);

    let table = "minecraft:chests/igloo_chest";
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .set_container_loot_table(table, 31));
    let refused = right_click(&sender, &receiver, &mut world, 1, at, 3);
    assert!(opened_screen(&refused).is_none());
    assert!(refused.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::ContainerClose(close), .. }
            if close.container_id == 2)));
    assert_eq!(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .unwrap()
            .container_loot_table_id(),
        Some(table)
    );
    let _ = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .randomizable_container_mut()
        .unwrap()
        .0
        .take();
    let after_refusal = right_click(&sender, &receiver, &mut world, 1, at, 4);
    assert_eq!(opened_screen(&after_refusal).unwrap().container_id, 4);

    // The block that has no `MenuProvider` override answers `PASS`, and the
    // item leg behind it is refused for a spectator either way, so nothing at
    // all comes back.
    let stone = BlockPos { x: 0, y: 63, z: 0 };
    let missed = right_click(&sender, &receiver, &mut world, 1, stone, 5);
    assert!(opened_screen(&missed).is_none());
}

fn trapped_chest_state(facing: &str, kind: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("type".to_string(), kind.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:trapped_chest",
        &properties,
    )
    .expect("minecraft:trapped_chest is a vanilla 26.1 block")
}

fn hopper_down_enabled() -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "down".to_string()),
        ("enabled".to_string(), "true".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:hopper",
        &properties,
    )
    .expect("minecraft:hopper is a vanilla 26.1 block")
}

fn hopper_enabled(world: &mut ServerWorld, pos: BlockPos) -> Option<&str> {
    let state = world.block_state(Java26BuiltinDimension::Overworld, pos)?;
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("enabled")
        .map(String::as_str)
}

/// `TrappedChestBlockEntity.signalOpenCount` (`TrappedChestBlockEntity.java:18-27`)
/// fans neighbours when the opener count moves, so a hopper beside an opened
/// trapped chest locks and unlocks again when the menu closes.
#[test]
fn opening_a_trapped_chest_locks_a_hopper_beside_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        12.0,
    );
    let chest = BlockPos { x: 10, y: 64, z: 1 };
    let hopper = BlockPos { x: 11, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        chest,
        trapped_chest_state("north", "single"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        hopper,
        hopper_down_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(hopper_enabled(&mut world, hopper), Some("true"));

    let opened = right_click(&sender, &receiver, &mut world, 1, chest, 1);
    assert!(opened_screen(&opened).is_some());
    assert_eq!(hopper_enabled(&mut world, hopper), Some("false"));
    assert_eq!(
        block_events_sent(&opened),
        vec![bbb_protocol::packets::BlockEvent {
            pos: chest,
            b0: 1,
            b1: 1,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_TRAPPED_CHEST_ID,
        }],
        "`ChestBlock.EVENT_SET_OPEN_COUNT` still fires on a trapped chest"
    );

    for _ in 0..12 {
        world.tick(receiver.drain());
        assert_eq!(hopper_enabled(&mut world, hopper), Some("false"));
    }
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, chest)
            .unwrap()
            .chest()
            .unwrap()
            .open_count(),
        1,
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
    );
    assert_eq!(hopper_enabled(&mut world, hopper), Some("true"));
    let raised = world.game_events(Java26BuiltinDimension::Overworld).0;
    for kind in [
        crate::level_event::VanillaGameEvent::CONTAINER_OPEN,
        crate::level_event::VanillaGameEvent::CONTAINER_CLOSE,
    ] {
        assert!(raised.iter().any(|event| event.game_event == kind
            && event.pos == chest
            && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))));
    }
}

#[test]
fn a_weathered_copper_chest_keeps_its_lid_open_and_plays_its_own_sounds() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:weathered_copper_chest",
        &BTreeMap::from([
            ("facing".into(), "north".into()),
            ("type".into(), "single".into()),
            ("waterlogged".into(), "false".into()),
        ]),
    )
    .unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).unwrap();
    assert!(sounds_sent(&opened).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id(
                "minecraft:block.copper_chest_weathered.open"
            )
            .unwrap(),
        }));
    for _ in 0..12 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .unwrap()
            .chest()
            .unwrap()
            .open_count(),
        1
    );
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    assert!(block_events_sent(&closed)
        .iter()
        .any(|event| event.pos == at && event.b1 == 0));
    assert!(sounds_sent(&closed).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id(
                "minecraft:block.copper_chest_weathered.close"
            )
            .unwrap(),
        }));
}

fn barrel_state(facing: &str, open: bool) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        (
            "open".to_string(),
            if open { "true" } else { "false" }.to_string(),
        ),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:barrel",
        &properties,
    )
    .expect("minecraft:barrel is a vanilla 26.1 block")
}

fn barrel_is_open(world: &mut ServerWorld, pos: BlockPos) -> Option<&str> {
    let state = world.block_state(Java26BuiltinDimension::Overworld, pos)?;
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("open")
        .map(String::as_str)
}

/// `CONTAINER_OPEN` / `CONTAINER_CLOSE` raised at `at` by `connection`.
fn container_game_events(
    world: &ServerWorld,
    at: BlockPos,
    connection: u64,
) -> Vec<crate::level_event::VanillaGameEvent> {
    use crate::level_event::VanillaGameEvent;
    let player = world.player_entity_id_for_test(ConnectionId(connection));
    world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .map(|event| event.recorded)
        .filter(|event| {
            event.pos == at
                && event.source_entity_id == Some(player)
                && (event.game_event == VanillaGameEvent::CONTAINER_OPEN
                    || event.game_event == VanillaGameEvent::CONTAINER_CLOSE)
        })
        .map(|event| event.game_event)
        .collect()
}

/// `BarrelBlockEntity.onOpen` / `onClose` (`BarrelBlockEntity.java:30-38`)
/// set `BarrelBlock.OPEN` and play the barrel sounds; the openers counter
/// raises the container game events (`ContainerOpenersCounter.java:32,44`).
#[test]
fn opening_a_barrel_sets_open_and_closing_it_clears_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        barrel_state("north", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(barrel_is_open(&mut world, at), Some("false"));

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a barrel opens its screen");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.barrel",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`BarrelBlockEntity.DEFAULT_NAME` (`BarrelBlockEntity.java:26`)"
    );
    assert_eq!(barrel_is_open(&mut world, at), Some("true"));
    let sounds = sounds_sent(&opened);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.barrel.open")
                .expect("a registered sound"),
        }
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
    );
    assert_eq!(barrel_is_open(&mut world, at), Some("false"));
    assert_eq!(
        container_game_events(&world, at, 1),
        [
            crate::level_event::VanillaGameEvent::CONTAINER_OPEN,
            crate::level_event::VanillaGameEvent::CONTAINER_CLOSE
        ]
    );
}

/// `ContainerOpenersCounter.recheckOpeners` (`ContainerOpenersCounter.java:57-87`)
/// via `BarrelBlock.tick`: an open barrel stays open while its viewer counts
/// and closes once the viewer turns spectator.
#[test]
fn an_open_barrel_rechecks_its_openers() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        barrel_state("north", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());
    for _ in 0..12 {
        world.tick(receiver.drain());
    }
    assert_eq!(barrel_is_open(&mut world, at), Some("true"));
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Spectator,
        PlayerAbilities::for_game_type(GameType::Spectator),
    );
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    assert_eq!(barrel_is_open(&mut world, at), Some("false"));
}

fn dropper_state(facing: &str, triggered: bool) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        (
            "triggered".to_string(),
            if triggered { "true" } else { "false" }.to_string(),
        ),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:dropper",
        &properties,
    )
    .expect("minecraft:dropper is a vanilla 26.1 block")
}

/// `DispenserMenu` (`DispenserMenu.java:21-28`) opens as `generic_3x3` with
/// the dropper's default name.
#[test]
fn opening_a_dropper_opens_the_three_by_three_menu() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        dropper_state("north", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a dropper opens its screen");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_3X3_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.dropper",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`DropperBlockEntity.DEFAULT_NAME` (`DropperBlockEntity.java:8`)"
    );
}

fn shulker_box_state(facing: &str) -> i32 {
    let properties = BTreeMap::from([("facing".to_string(), facing.to_string())]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:shulker_box",
        &properties,
    )
    .expect("minecraft:shulker_box is a vanilla 26.1 block")
}

fn hopper_south_enabled() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:hopper",
        &BTreeMap::from([
            ("facing".to_string(), "south".to_string()),
            ("enabled".to_string(), "true".to_string()),
        ]),
    )
    .expect("minecraft:hopper is a vanilla 26.1 block")
}

/// `ShulkerBoxMenu` (`ShulkerBoxMenu.java:17-32`) opens as `shulker_box` with
/// `container.shulkerBox`, plays the open sound, and raises the lid event.
#[test]
fn opening_a_shulker_box_opens_its_menu_and_starts_the_lid() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a shulker box opens its screen");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_SHULKER_BOX_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.shulkerBox",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`ShulkerBoxBlockEntity.DEFAULT_NAME` (`ShulkerBoxBlockEntity.java:44`)"
    );
    let sounds = sounds_sent(&opened);
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.shulker_box.open")
                .expect("a registered sound"),
        }
    );
    assert_eq!(sounds[0].volume, 0.5);
    assert_eq!(
        block_events_sent(&opened),
        vec![bbb_protocol::packets::BlockEvent {
            pos: at,
            b0: 1,
            b1: 1,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_SHULKER_BOX_ID,
        }],
        "`ShulkerBoxBlockEntity.EVENT_SET_OPEN_COUNT` with the new opener count"
    );
    let status = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.shulker_box())
        .map(crate::block_entity::shulker_box::ShulkerBoxBlockEntity::animation_status);
    assert_eq!(
        status,
        Some(crate::block_entity::shulker_box::AnimationStatus::Opening)
    );

    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id: 1 }),
    );
    assert_eq!(
        sounds_sent(&closed)
            .iter()
            .map(|sound| sound.sound.clone())
            .collect::<Vec<_>>(),
        vec![SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.shulker_box.close")
                .expect("a registered sound"),
        }]
    );
    let shulker = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.shulker_box())
        .expect("the shulker box is still standing");
    assert_eq!(shulker.open_count(), 0);
    // The lid had only advanced one opening tick, so the matching close tick
    // reaches `CLOSED` in the same `tickBlockEntities` pass.
    assert!(shulker.is_closed());
    // ShulkerBoxBlockEntity.java:176,189.
    assert_eq!(
        container_game_events(&world, at, 1),
        [
            crate::level_event::VanillaGameEvent::CONTAINER_OPEN,
            crate::level_event::VanillaGameEvent::CONTAINER_CLOSE
        ]
    );
}

/// `ShulkerBoxBlock.canOpen` (`ShulkerBoxBlock.java:88-95`): a closed lid that
/// would open into a solid neighbour stays shut. The click still consumes.
#[test]
fn a_solid_block_above_keeps_a_closed_shulker_box_shut() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let above = BlockPos { x: 0, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .expect("minecraft:stone has a default state");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        above,
        stone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let missed = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&missed).is_none());
    assert!(sounds_sent(&missed).is_empty());
}

/// `HopperBlockEntity.ejectItems` will not insert a shulker-box item into a
/// shulker box (`ShulkerBoxBlockEntity.java:238-240`).
#[test]
fn a_hopper_does_not_insert_a_shulker_box_into_a_shulker_box() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let hopper = BlockPos { x: 0, y: 64, z: 0 };
    let dest = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        dest,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        hopper,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(BlockEntity::hopper_mut)
        .expect("the write created the hopper")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let _ = world.tick(receiver.drain());
    let hopper_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(|block_entity| block_entity.hopper())
        .expect("hopper")
        .items()[0]
        .clone();
    assert_eq!(
        hopper_item.item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID)
    );
    let dest_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, dest)
        .and_then(|block_entity| block_entity.shulker_box())
        .expect("shulker")
        .items()[0]
        .clone();
    assert!(dest_item.item_id().is_none());
}

fn lectern_state(has_book: bool) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        (
            "has_book".to_string(),
            if has_book { "true" } else { "false" }.to_string(),
        ),
        ("powered".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:lectern",
        &properties,
    )
    .expect("minecraft:lectern is a vanilla 26.1 block")
}

fn lectern_has_book(world: &mut ServerWorld, pos: BlockPos) -> Option<&str> {
    let state = world.block_state(Java26BuiltinDimension::Overworld, pos)?;
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("has_book")
        .map(String::as_str)
}

fn two_page_writable_book() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID],
            writable_book_pages: vec!["one".into(), "two".into()],
            writable_book_page_filters: vec![None, None],
            ..Default::default()
        }.into(),
    }
}

/// `LecternBlock.useItemOn` places a lectern-books item; the next empty-hand
/// click opens `LecternMenu`.
#[test]
fn placing_a_book_on_a_lectern_then_opens_its_menu() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lectern_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: two_page_writable_book(),
        }),
    );
    let placed = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(lectern_has_book(&mut world, at), Some("true"));
    assert!(
        sounds_sent(&placed).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:item.book.put")
                    .expect("a registered sound"),
            }),
        "{placed:?}"
    );
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 2);
    let screen = opened_screen(&opened).expect("a lectern with a book opens its screen");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_LECTERN_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.lectern",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap()
    );
    let turned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerButtonClick(ContainerButtonClick {
            container_id: 1,
            button_id: 2,
        }),
    );
    let page = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.lectern())
        .expect("lectern")
        .page();
    assert_eq!(page, 1);
    assert!(turned.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetData(data),
            ..
        } if data.value == 1
    )));
}

/// `LecternBlock.placeBook` hands the player to `setBook`, whose `resolveBook`
/// (`LecternBlockEntity.java:152-153`, `:177-184`) resolves the written pages.
#[test]
fn a_written_book_placed_on_a_lectern_is_resolved() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lectern_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let tick = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        10,
        r#"give alice written_book[written_book_content={title:"t",author:"a",pages:[{selector:"@s"}]}]"#,
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&tick).0,
        Some(1)
    );
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    let book = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.lectern())
        .expect("lectern")
        .original_book();
    let content = book
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap()
        .expect("the book is on the lectern");
    let NbtSummaryValue::Compound(fields) = &content else {
        panic!("written book content is a compound");
    };
    assert!(fields
        .iter()
        .any(|entry| entry.name == "resolved" && entry.value == NbtSummaryValue::Byte(1)));
    assert!(format!("{content:?}").contains("\"alice\""), "{content:?}");
}

fn chiseled_bookshelf_state(facing: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("slot_0_occupied".to_string(), "false".to_string()),
        ("slot_1_occupied".to_string(), "false".to_string()),
        ("slot_2_occupied".to_string(), "false".to_string()),
        ("slot_3_occupied".to_string(), "false".to_string()),
        ("slot_4_occupied".to_string(), "false".to_string()),
        ("slot_5_occupied".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chiseled_bookshelf",
        &properties,
    )
    .expect("minecraft:chiseled_bookshelf is a vanilla 26.1 block")
}

fn bookshelf_slot_occupied(world: &mut ServerWorld, pos: BlockPos, slot: usize) -> Option<bool> {
    let state = world.block_state(Java26BuiltinDimension::Overworld, pos)?;
    Some(crate::block_behavior::chiseled_bookshelf::slot_occupied(
        state, slot,
    ))
}

fn click_bookshelf_slot(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::North,
                cursor_x: 0.9,
                cursor_y: 0.75,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

/// `ChiseledBookShelfBlock.useItemOn` / `useWithoutItem`
/// (`ChiseledBookShelfBlock.java:76-125`): a north-face cursor at slot 0
/// inserts, then the same click takes the book back.
#[test]
fn placing_a_book_in_a_chiseled_bookshelf_then_takes_it_back() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chiseled_bookshelf_state("north"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }),
    );
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chiseled_bookshelf_mut)
        .unwrap()
        .set_slot(
            1,
            crate::tests::native_stack(ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        );
    let inserted = click_bookshelf_slot(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(bookshelf_slot_occupied(&mut world, at, 0), Some(true));
    let bookshelf = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.chiseled_bookshelf())
        .expect("bookshelf");
    assert_eq!(
        bookshelf.items()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID)
    );
    assert_eq!(bookshelf.last_interacted_slot(), 0);
    assert_eq!(bookshelf.redstone_signal(), 1);
    assert!(
        sounds_sent(&inserted).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.chiseled_bookshelf.insert"
                )
                .expect("a registered sound"),
            }),
        "{inserted:?}"
    );

    let untouched = bookshelf.original_items()[1].clone();
    let removed = click_bookshelf_slot(&sender, &receiver, &mut world, 1, at, 2);
    assert_eq!(bookshelf_slot_occupied(&mut world, at, 0), Some(false));
    let bookshelf = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.chiseled_bookshelf())
        .expect("bookshelf");
    assert!(bookshelf.items()[0].item_id().is_none());
    assert_eq!(bookshelf.original_items()[1], untouched);
    assert_eq!(bookshelf.redstone_signal(), 1);
    assert!(
        sounds_sent(&removed).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.chiseled_bookshelf.pickup"
                )
                .expect("a registered sound"),
            }),
        "{removed:?}"
    );
}

/// `HopperBlockEntity.ejectItems` into `ChiseledBookShelfBlockEntity`
/// (`:74-76`, `:69-71`): a book moves, stone does not, occupancy follows.
#[test]
fn a_hopper_inserts_a_book_into_a_chiseled_bookshelf_and_refuses_stone() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let hopper = BlockPos { x: 0, y: 64, z: 0 };
    let dest = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        dest,
        chiseled_bookshelf_state("north"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        hopper,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(BlockEntity::hopper_mut)
        .expect("the write created the hopper")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let _ = world.tick(receiver.drain());
    let hopper_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(|block_entity| block_entity.hopper())
        .expect("hopper")
        .items()[0]
        .clone();
    assert_eq!(
        hopper_item.item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(bookshelf_slot_occupied(&mut world, dest, 0), Some(false));

    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(BlockEntity::hopper_mut)
        .expect("hopper")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let _ = world.tick(receiver.drain());
    let hopper_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(|block_entity| block_entity.hopper())
        .expect("hopper")
        .items()[0]
        .clone();
    assert!(hopper_item.item_id().is_none());
    assert_eq!(bookshelf_slot_occupied(&mut world, dest, 0), Some(true));
    let bookshelf = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, dest)
        .and_then(|block_entity| block_entity.chiseled_bookshelf())
        .expect("bookshelf");
    assert_eq!(
        bookshelf.items()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID)
    );
    assert_eq!(bookshelf.redstone_signal(), 1);
}

#[test]
fn a_hopper_extracts_bookshelf_stacks_by_effective_components() {
    use bbb_server_types::item_stack::RuntimeItemStack;

    for accepts in [true, false] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let hopper = BlockPos { x: 0, y: 64, z: 0 };
        let source = BlockPos { x: 0, y: 65, z: 0 };
        for (pos, state) in [
            (hopper, hopper_south_enabled()),
            (source, chiseled_bookshelf_state("north")),
        ] {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                pos,
                state,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            ));
        }
        let resolver = world.item_stack_resolver();
        let stack = |components, count| {
            RuntimeItemStack::try_read(
                &NbtSummaryValue::Compound(vec![
                    nbt("id", NbtSummaryValue::String("minecraft:book".into())),
                    nbt("count", NbtSummaryValue::Int(count)),
                    nbt("components", NbtSummaryValue::Compound(components)),
                ]),
                &resolver,
            )
            .unwrap()
        };
        let removed = stack(
            vec![nbt("!minecraft:rarity", NbtSummaryValue::Compound(vec![]))],
            62,
        );
        let incoming = stack(
            vec![nbt("minecraft:max_stack_size", NbtSummaryValue::Int(64))],
            2,
        );
        let target = world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
            .and_then(BlockEntity::hopper_mut)
            .unwrap();
        target.items_mut().fill(removed);
        if accepts {
            target.items_mut()[4] = stack(vec![], 62);
        }
        assert!(world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, source)
            .and_then(BlockEntity::chiseled_bookshelf_mut)
            .unwrap()
            .set_slot(0, incoming));
        let _ = world.tick(receiver.drain());
        let items = world
            .get_block_entity(Java26BuiltinDimension::Overworld, hopper)
            .and_then(BlockEntity::hopper)
            .unwrap()
            .items();
        assert!(items[..4].iter().all(|item| item.count() == 62));
        assert_eq!(items[4].count(), if accepts { 64 } else { 62 });
        let remaining = world
            .get_block_entity(Java26BuiltinDimension::Overworld, source)
            .and_then(BlockEntity::chiseled_bookshelf)
            .unwrap()
            .items()[0]
            .count();
        assert_eq!(remaining, if accepts { 0 } else { 2 });
    }
}

fn decorated_pot_state() -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("cracked".to_string(), "false".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:decorated_pot",
        &properties,
    )
    .expect("minecraft:decorated_pot is a vanilla 26.1 block")
}

/// A hopper facing a decorated pot inserts one item into its single slot.
#[test]
fn a_hopper_inserts_stone_into_a_decorated_pot() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let hopper = BlockPos { x: 0, y: 64, z: 0 };
    let dest = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        dest,
        decorated_pot_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        hopper,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(BlockEntity::hopper_mut)
        .expect("the write created the hopper")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 2,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let _ = world.tick(receiver.drain());
    let hopper_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(|block_entity| block_entity.hopper())
        .expect("hopper")
        .items()[0]
        .clone();
    assert_eq!(hopper_item.count(), 1);
    let pot = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, dest)
        .and_then(|block_entity| block_entity.decorated_pot())
        .expect("pot");
    assert_eq!(
        pot.item().item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(pot.item().count(), 1);
    assert!(pot.redstone_signal().unwrap() > 0);
}

fn oak_shelf_state(powered: bool) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        (
            "powered".to_string(),
            if powered { "true" } else { "false" }.to_string(),
        ),
        ("side_chain".to_string(), "unconnected".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_shelf",
        &properties,
    )
    .expect("minecraft:oak_shelf is a vanilla 26.1 block")
}

fn click_shelf_slot(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::North,
                cursor_x: 0.9,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

/// `ShelfBlock.useItemOn` (`ShelfBlock.java:176-188`) swaps the selected stack
/// with the hit slot when the shelf is unpowered.
#[test]
fn swapping_with_an_unpowered_shelf_takes_the_selected_stack() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        oak_shelf_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let swapped = click_shelf_slot(&sender, &receiver, &mut world, 1, at, 1);
    let shelf = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.shelf())
        .expect("shelf");
    assert_eq!(
        shelf.items()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(shelf.items()[0].count(), 64);
    assert!(shelf.redstone_signal().unwrap() > 0);
    assert!(
        sounds_sent(&swapped).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.shelf.place_item")
                    .expect("a registered sound"),
            }),
        "{swapped:?}"
    );
}

/// A hopper facing a shelf inserts one item into the first vacant slot; the
/// shelf's `setChanged` raises `BLOCK_ACTIVATE` and sends the block update
/// that carries its new contents to watching clients
/// (`ShelfBlockEntity.java:87-100`).
#[test]
fn a_hopper_inserts_stone_into_a_shelf() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let hopper = BlockPos { x: 0, y: 64, z: 0 };
    let dest = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        dest,
        oak_shelf_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        hopper,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(BlockEntity::hopper_mut)
        .expect("the write created the hopper")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 2,
        component_patch: DataComponentPatchSummary::default().into(),
    });
    let tick = world.tick(receiver.drain());
    assert!(
        tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BlockEntityData(packet),
            } if packet.pos == dest
        )),
        "the shelf's new contents reach the client"
    );
    assert!(world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .iter()
        .any(|event| event.recorded.pos == dest
            && event.recorded.game_event == crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE));
    let hopper_item = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, hopper)
        .and_then(|block_entity| block_entity.hopper())
        .expect("hopper")
        .items()[0]
        .clone();
    assert_eq!(hopper_item.count(), 1);
    let shelf = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, dest)
        .and_then(|block_entity| block_entity.shelf())
        .expect("shelf");
    assert_eq!(
        shelf.items()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(shelf.items()[0].count(), 1);
}

/// The click machinery is one implementation: `doClick`'s `PICKUP` branch runs
/// over a chest slot exactly as it does over an inventory slot, and the write
/// lands in the block entity behind the menu.
#[test]
fn a_click_in_a_chest_menu_writes_through_to_the_block_entity_and_the_save() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest_state("north", "single"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let state_id = container_contents(&opened)[0].state_id;

    // Menu slot 54 is the first hotbar slot, which the fixture stocks with a
    // full stack of stone; slot 0 is the chest's first slot.
    let picked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 1,
            state_id,
            slot_num: 54,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(!picked.events.is_empty());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 1,
            state_id: state_id + 2,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    let stored = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest)
        .expect("the chest is still there")
        .items()
        .to_vec();
    assert_eq!(
        stored[0].summary().clone(),
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 64,
            component_patch: DataComponentPatchSummary::default().into(),
        },
        "the stack moved out of the player's hotbar and into the chest"
    );

    // `BlockEntity.setChanged` marked the record, so the save carries it.
    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the edited column is written");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");
    assert_eq!(parsed.block_entities().len(), 1);
    let loaded = crate::tests::load_native_fixture_block_entity(
        at,
        chest_state("north", "single"),
        &parsed.block_entities()[0],
    );
    assert_eq!(loaded.chest().expect("a chest").items(), stored.as_slice());
}

/// `ServerPlayer.tick` (`ServerPlayer.java:580-584`): the menu is broadcast
/// and then dropped when `stillValid` fails, which for a `ChestMenu` is
/// `Container.stillValidBlockEntity` (`Container.java:94-102`) — the block
/// entity is still the same one, and the player is still within
/// `isWithinBlockInteractionRange(pos, 4.0F)`.
#[test]
fn walking_out_of_range_closes_the_chest_menu_from_the_server_side() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest_state("north", "single"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());

    // The walk out of range is the fixture; the close is the subject.
    let walked = place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 12.5,
        },
        None,
    );
    let closed = walked
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerClose(close),
                ..
            } => Some(*close),
            _ => None,
        })
        .expect("walking away closes the menu from the server side");
    assert_eq!(closed.container_id, 1);
    // `ChestMenu.removed` (`ChestMenu.java:100-103`) runs `stopOpen`, so the
    // lid closes and the close sound plays.
    let next = world.tick(receiver.drain());
    let sounds: Vec<_> = sounds_sent(&walked)
        .into_iter()
        .chain(sounds_sent(&next))
        .collect();
    assert_eq!(sounds.len(), 1, "{sounds:?}");
    assert_eq!(
        sounds[0].sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.chest.close")
                .expect("a registered sound"),
        }
    );
    assert_eq!(
        block_events_sent(&walked)
            .into_iter()
            .chain(block_events_sent(&next))
            .collect::<Vec<_>>(),
        vec![bbb_protocol::packets::BlockEvent {
            pos: at,
            b0: 1,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_CHEST_ID,
        }]
    );
    assert_eq!(
        world
            .get_block_entity(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::chest)
            .expect("the chest is still there")
            .open_count(),
        0
    );

    // The player inventory is the open menu again, so a click that names
    // container zero is accepted once more.
    let clicked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(
        !clicked.events.is_empty(),
        "the inventory menu answers clicks again: {clicked:?}"
    );
}

/// `DoubleBlockCombiner.combineWithNeigbour` (`DoubleBlockCombiner.java:14-61`)
/// through `ChestMenu.sixRows` (`ChestMenu.java:45-47`): two halves are one
/// `CompoundContainer` (`CompoundContainer.java:11-14`), so slot 27 of the
/// menu is slot 0 of the *second* block entity.
#[test]
fn a_double_chest_opens_one_six_row_menu_over_both_halves() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // `getConnectedDirection` (`ChestBlock.java:199-202`): a north-facing
    // right half reaches counter-clockwise, i.e. west.
    let right = BlockPos { x: 0, y: 64, z: 1 };
    let left = BlockPos { x: -1, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        right,
        chest_state("north", "right"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    world.set_block(
        Java26BuiltinDimension::Overworld,
        left,
        chest_state("north", "left"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let marker = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 3,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut items = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
    items[0] = crate::tests::native_stack(marker.clone());
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, left)
        .and_then(BlockEntity::chest_mut)
        .expect("the second half has a block entity")
        .set_items(items);

    let opened = right_click(&sender, &receiver, &mut world, 1, right, 1);
    let screen = opened_screen(&opened).expect("clicking a double chest opens one screen");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X6_ID
    );
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.chestDouble",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`ChestBlock.java:116`"
    );
    let contents = container_contents(&opened);
    assert_eq!(contents[0].items.len(), 90);
    assert_eq!(
        contents[0].items[27], marker,
        "the second block entity's slot 0 is menu slot 27"
    );

    // `CompoundContainer.startOpen` (`CompoundContainer.java:75-78`) reaches
    // both halves, so both raise their lid event -- but only the right half
    // plays a sound (`ChestBlockEntity.java:105`).
    assert_eq!(block_events_sent(&opened).len(), 2);
    assert_eq!(sounds_sent(&opened).len(), 1);
    assert_eq!(
        sounds_sent(&opened)[0].position,
        Vec3d {
            x: 0.0,
            y: 64.5,
            z: 1.5
        }
    );
    for _ in 0..7 {
        world.tick(receiver.drain());
    }
    for pos in [right, left] {
        assert_eq!(
            world
                .get_block_entity(Java26BuiltinDimension::Overworld, pos)
                .unwrap()
                .chest()
                .unwrap()
                .open_count(),
            1
        );
    }
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    assert_eq!(block_events_sent(&closed).len(), 2);
    assert_eq!(sounds_sent(&closed).len(), 1);
}

/// The other direction: a chest record an official save carried reaches the
/// runtime with its items on board.
#[test]
fn a_chest_record_from_a_loaded_save_reaches_the_runtime_with_its_items() {
    let mut data = vec![0_i64; 256];
    // The same nibble arithmetic the sign fixture uses: cell (1, 0, 2).
    data[2] = 1 << 4;
    let section = NbtSummaryValue::Compound(vec![
        nbt("Y", NbtSummaryValue::Byte(-4)),
        nbt(
            "block_states",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "palette",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Compound(vec![nbt(
                            "Name",
                            NbtSummaryValue::string("minecraft:stone"),
                        )]),
                        NbtSummaryValue::Compound(vec![
                            nbt("Name", NbtSummaryValue::string("minecraft:chest")),
                            nbt(
                                "Properties",
                                NbtSummaryValue::Compound(vec![
                                    nbt("facing", NbtSummaryValue::string("north")),
                                    nbt("type", NbtSummaryValue::string("single")),
                                    nbt("waterlogged", NbtSummaryValue::string("false")),
                                ]),
                            ),
                        ]),
                    ]),
                ),
                nbt("data", NbtSummaryValue::LongArray(data)),
            ]),
        ),
        nbt(
            "biomes",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "minecraft:plains".to_string(),
                )]),
            )]),
        ),
    ]);
    let record = NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::string("minecraft:chest")),
        nbt("x", NbtSummaryValue::Int(1)),
        nbt("y", NbtSummaryValue::Int(-64)),
        nbt("z", NbtSummaryValue::Int(2)),
        nbt("keepPacked", NbtSummaryValue::Byte(0)),
        nbt("components", NbtSummaryValue::Compound(Vec::new())),
        nbt(
            "Items",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                nbt("Slot", NbtSummaryValue::Byte(2)),
                nbt("id", NbtSummaryValue::string("minecraft:stone")),
                nbt("count", NbtSummaryValue::Int(17)),
            ])]),
        ),
    ]);
    let mut raw = restore_fixture_chunk_raw(vec![section]);
    for entry in raw.entries_mut() {
        if entry.name == "block_entities" {
            entry.value = NbtSummaryValue::List(vec![record.clone()]);
        }
    }
    let chunk = Java26ChunkData::parse(raw, restore_chunk_context()).unwrap();
    let candidate = restore_candidate(1_000, Some(chunk), Vec::new());
    let world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let at = BlockPos { x: 1, y: -64, z: 2 };
    let chest = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest)
        .expect("the saved chest record became a runtime object");
    assert_eq!(
        chest.items()[2].summary().clone(),
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 17,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    );
    assert!(chest.items()[0].item_id().is_none());
}

/// The recipe book's unlock through a save and a restart, both halves of it.
///
/// The craft is a real one: `ResultSlot.checkTakeAchievements`
/// (`ResultSlot.java:50-59`) reaches `RecipeCraftingHolder.awardUsedRecipes`
/// (`RecipeCraftingHolder.java:17-26`) and `ServerRecipeBook.addRecipes`
/// (`ServerRecipeBook.java:61-80`) puts the id in `known` *and* `highlight`.
/// `ServerRecipeBook.Packed.CODEC` (`ServerRecipeBook.java:150-158`) writes
/// those two as `recipes` and `toBeDisplayed`, and
/// `ServerRecipeBook.apply` (`ServerRecipeBook.java:131-137`) restores both --
/// so a rejoined session has to write the same two lists back out.
#[test]
fn a_crafted_recipe_unlock_survives_a_save_and_a_restart_in_both_book_halves() {
    use bbb_protocol::item_types::{VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_IRON_INGOT_ID};

    let (sender, receiver) = mailbox();
    let mut world =
        ServerWorld::restore(datapack_config(), &restore_candidate(0, None, vec![])).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 9, "newcomer", 9);
    let initial_request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    let initial = &initial_request.player_patches[0].created;
    let initial_recipes = [
        "minecraft:chiseled_stone_bricks_from_stone_stonecutting",
        "minecraft:cobblestone_from_stone_stonecutting",
        "minecraft:cobblestone_slab_from_stone_stonecutting",
        "minecraft:cobblestone_stairs_from_stone_stonecutting",
        "minecraft:cobblestone_wall_from_stone_stonecutting",
        // The `minecraft:tick` criterion of `recipes/decorations/crafting_table`.
        "minecraft:crafting_table",
        "minecraft:smooth_stone",
        "minecraft:stone_brick_slab_from_stone_stonecutting",
        "minecraft:stone_brick_stairs_from_stone_stonecutting",
        "minecraft:stone_brick_wall_from_stone_stonecutting",
        "minecraft:stone_bricks",
        "minecraft:stone_bricks_from_stone_stonecutting",
        "minecraft:stone_button",
        "minecraft:stone_pressure_plate",
        "minecraft:stone_slab",
        "minecraft:stone_slab_from_stone_stonecutting",
        "minecraft:stone_stairs",
        "minecraft:stone_stairs_from_stone_stonecutting",
        "minecraft:stonecutter",
    ];
    assert_eq!(initial.recipe_book_recipes, initial_recipes);
    assert_eq!(initial.recipe_book_to_be_displayed, initial_recipes);
    let mut expected_recipes = initial.recipe_book_recipes.clone();
    expected_recipes.push("minecraft:flint_and_steel".to_string());
    expected_recipes.sort();
    for (slot_num, item_id) in [(1, VANILLA_ITEM_IRON_INGOT_ID), (2, VANILLA_ITEM_FLINT_ID)] {
        play(
            &sender,
            &receiver,
            &mut world,
            9,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            }),
        );
    }
    play(
        &sender,
        &receiver,
        &mut world,
        9,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(world.player_knows_recipe_for_test(ConnectionId(9), "minecraft:flint_and_steel"));

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let saved_craft = patch
        .created
        .inventory
        .runtime_slots()
        .into_iter()
        .find(|stack| {
            stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_FLINT_AND_STEEL_ID)
        })
        .expect("quick-moved craft is in the saved inventory");
    assert_eq!(patch.created.recipe_book_recipes, expected_recipes);
    assert_eq!(patch.created.recipe_book_to_be_displayed, expected_recipes);
    let saved_recipes = patch.created.recipe_book_recipes.clone();
    let saved_highlights = patch.created.recipe_book_to_be_displayed.clone();
    let uuid = patch.uuid;
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, uuid).unwrap();
    assert_eq!(parsed.recipe_book().recipes(), saved_recipes.as_slice());
    assert_eq!(
        parsed.recipe_book().to_be_displayed(),
        saved_highlights.as_slice()
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        datapack_config(),
        &restore_candidate(0, None, vec![(uuid, parsed)]),
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "newcomer", 9);
    assert!(world.player_knows_recipe_for_test(ConnectionId(9), "minecraft:flint_and_steel"));
    // The highlight half never reaches the wire, so the next save is where the
    // rejoined session reports what it restored.
    let request = world
        .save_request(1_786_000_003_000, 203, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let restored_craft = patch
        .created
        .inventory
        .runtime_slots()
        .into_iter()
        .find(|stack| stack.item_id() == saved_craft.item_id())
        .unwrap();
    assert!(saved_craft.same_item_same_components(&restored_craft));
    assert_eq!(patch.created.recipe_book_recipes, saved_recipes);
    assert_eq!(patch.created.recipe_book_to_be_displayed, saved_highlights);
}

/// The live item entities of one level as `(item resource id, count, position)`.
fn dropped_ground_items(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
) -> Vec<(String, i32, Vec3d)> {
    world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(dimension, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                entity.position(),
            ))
        })
        .collect()
}

/// `Player.drop(ItemStack, boolean)` (`Player.java:582-583`) through
/// `LivingEntity.createItemStackToDrop` (`LivingEntity.java:3372-3400`): a
/// thrown slot leaves the menu as one item entity at the player's hand height,
/// with the 40-tick pickup delay and the player as its thrower.
#[test]
fn throwing_a_slot_drops_the_stack_out_of_the_players_hand() {
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::stat_types::{
        VANILLA_STAT_TYPE_CUSTOM_ID, VANILLA_STAT_TYPE_ITEM_DROPPED_ID,
    };
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 5,
                component_patch: Default::default(),
            },
        }),
    );
    let uuid = stocked.snapshot.players[0].uuid;
    // The session position the click tick starts from, which is what the drop
    // reads (`LivingEntity.java:3378`).
    let (_, position, _, _, _) = world.player_transition_state_for_test(ConnectionId(1));

    let thrown = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(
        !thrown
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{thrown:?}"
    );

    let drops = dropped_ground_items(&world, Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 1, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:stone");
    assert_eq!(drops[0].1, 1);
    // `this.getEyeY() - 0.3F` (`LivingEntity.java:3377-3378`) with the player's
    // own x and z. The entity tick moves the drop inside the same tick, so the
    // read position is one throw step away from the spawn point.
    let eye_y = position.y
        + f64::from(
            bbb_protocol::entity_types::vanilla_entity_type_eye_height(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .expect("the player type carries an eye height"),
        )
        - 0.3;
    assert!((drops[0].2.x - position.x).abs() < 0.5, "{:?}", drops[0].2);
    assert!((drops[0].2.y - eye_y).abs() < 0.3, "{:?}", drops[0].2);
    assert!((drops[0].2.z - position.z).abs() < 0.5, "{:?}", drops[0].2);

    let entity = world
        .stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)[0],
        )
        .expect("a live entity");
    let crate::entity::EntityData::Item(item) = entity.data() else {
        panic!("the thrown stack became an item entity");
    };
    // `setPickUpDelay(40)` (`LivingEntity.java:3379`) with the one decrement the
    // same tick's `ItemEntity.tick` makes (`ItemEntity.java:120-122`).
    assert_eq!(
        item.pickup_delay(),
        crate::entity::item::DROP_PICKUP_DELAY - 1
    );
    // `setThrower(this)` (`LivingEntity.java:3381`) under `thrownFromHand`.
    assert_eq!(item.thrower(), Some(uuid));
    // `ServerPlayer.drop` (`ServerPlayer.java:2003-2013`) awards both.
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(VANILLA_STAT_TYPE_ITEM_DROPPED_ID, VANILLA_ITEM_STONE_ID)
        ),
        1
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(VANILLA_STAT_TYPE_CUSTOM_ID, 24)
        ),
        1
    );
}

/// `doClick`'s outside-click arm (`AbstractContainerMenu.java:406-416`): the
/// cursor stack is dropped with `thrownFromHand` set.
#[test]
fn clicking_outside_the_window_drops_the_cursor_stack() {
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::stat_types::{
        VANILLA_STAT_TYPE_CUSTOM_ID, VANILLA_STAT_TYPE_ITEM_DROPPED_ID,
    };
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 5,
                component_patch: Default::default(),
            },
        }),
    );
    // Take the stack onto the cursor first.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert!(dropped_ground_items(&world, Java26BuiltinDimension::Overworld).is_empty());

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: crate::inventory::SLOT_CLICKED_OUTSIDE,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    let drops = dropped_ground_items(&world, Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 1, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:stone");
    assert_eq!(drops[0].1, 5);
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(VANILLA_STAT_TYPE_ITEM_DROPPED_ID, VANILLA_ITEM_STONE_ID)
        ),
        5
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(VANILLA_STAT_TYPE_CUSTOM_ID, 24)
        ),
        1
    );
}

/// `handleSetCreativeModeSlot` (`ServerGamePacketListenerImpl.java:2001-2025`):
/// a negative slot drops the stack through `Player.drop`, and the connection's
/// own `TickThrottler(20, 1480)` (`:244`) refuses it once the threshold is
/// reached.
#[test]
fn a_creative_negative_slot_drops_until_the_spam_throttle_closes() {
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_DROPPED_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stack = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let dropped_stat = StatKey::new(VANILLA_STAT_TYPE_ITEM_DROPPED_ID, VANILLA_ITEM_STONE_ID);
    let negative_slot = |world: &mut ServerWorld| {
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: -1,
                item: stack.clone(),
            }),
        )
    };

    negative_slot(&mut world);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), dropped_stat), 1);

    // `TickThrottler.isUnderThreshold` (`TickThrottler.java:18-20`): at the
    // threshold the drop is refused and the counter is not incremented.
    world.set_drop_spam_count_for_test(ConnectionId(1), 1480);
    negative_slot(&mut world);
    assert_eq!(world.player_stat_for_test(ConnectionId(1), dropped_stat), 1);

    // A written slot is not throttled at all: it is not a drop.
    let written = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: stack,
        }),
    );
    assert_eq!(
        written.snapshot.players[0].inventory.slots[0].item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
}

#[test]
fn creative_slot_rejects_disabled_or_unknown_item_ids() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let rejected = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(999_999),
                count: 1,
                component_patch: Default::default(),
            },
        }),
    );
    assert_eq!(
        rejected.snapshot.players[0].inventory.slots[0].item_id,
        None
    );
}

/// The stack a player drops is the same stack: its component patch survives the
/// projection into the item entity, and the entity reaches the save.
#[test]
fn a_thrown_stack_keeps_its_components_and_reaches_the_save() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
        damage: Some(7),
        ..DataComponentPatchSummary::default()
    };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_DIAMOND_SWORD_ID),
                count: 1,
                component_patch: patch.into(),
            },
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    let entity_id = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| matches!(entity.data(), crate::entity::EntityData::Item(_)))
        })
        .expect("the thrown sword is an item entity");
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("a live entity");
    let crate::entity::EntityData::Item(item) = entity.data() else {
        panic!("the thrown sword became an item entity");
    };
    let stack = item.item().clone();
    assert_eq!(stack.item_resource_id(), Some("minecraft:diamond_sword"));
    assert_eq!(stack.summary().component_patch.damage, Some(7));

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let saved_items = saved_entities(&request)
        .iter()
        .filter(|entity| {
            entity.iter().any(|entry| {
                entry.name == "id" && entry.value == NbtSummaryValue::string("minecraft:item")
            })
        })
        .count();
    assert_eq!(saved_items, 1);
}

fn container_data(tick: &TickResult) -> Vec<bbb_protocol::packets::ContainerSetData> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetData(data),
                ..
            } => Some(*data),
            _ => None,
        })
        .collect()
}

fn furnace_state(lit: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:furnace",
        &BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            (
                "lit".to_string(),
                if lit { "true" } else { "false" }.to_string(),
            ),
        ]),
    )
    .expect("minecraft:furnace is a vanilla 26.1 block")
}

fn furnace_payload(
    world: &mut ServerWorld,
    at: BlockPos,
) -> crate::block_entity::furnace::AbstractFurnaceBlockEntity {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace())
        .expect("the furnace block entity is live")
        .clone()
}

mod item_projection;

/// `handleContainerClick` (ServerGamePacketListenerImpl.java:1919-1920): a
/// spectator's click resends the menu and moves nothing.
#[test]
fn a_spectator_click_resends_the_menu_and_leaves_the_slots_alone() {
    use bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Spectator,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "ghost", 1);
    let ingot = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_IRON_INGOT_ID),
        count: 3,
        component_patch: Default::default(),
    };
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ingot.clone());
    let clicked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert_eq!(
        world.player_inventory_slot_for_test(ConnectionId(1), 0),
        ingot
    );
    assert!(clicked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetContent(_),
            ..
        }
    )));
}

/// `LecternBlock.useItemOn` admits by `#minecraft:lectern_books`
/// (`LecternBlock.java:227`), so a pack that adds the plain book lets it rest.
#[test]
fn a_lectern_takes_what_the_lectern_books_tag_names() {
    let root = std::env::temp_dir().join(format!("bbb-lectern-books-{}", Uuid::new_v4()));
    let pack = root.join("test");
    let tag = pack.join("data/minecraft/tags/item/lectern_books.json");
    std::fs::create_dir_all(tag.parent().unwrap()).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"lectern books","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(&tag, r#"{"values":["minecraft:book"]}"#).unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lectern_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(lectern_has_book(&mut world, at), Some("true"));
}
