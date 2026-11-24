//! The utility block menus: anvil, enchanting table, grindstone, smithing
//! table, stonecutter, loom, cartography table and beacon.

use super::*;
use crate::tests::entities::console_command_answer;
use bbb_protocol::item_types as items;
use bbb_server_types::item_stack::RuntimeItemStack;

const AT: BlockPos = BlockPos { x: 0, y: 64, z: 1 };

struct Table {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    /// The menu's own slot count, ahead of the player inventory.
    size: i16,
}

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

/// A survival session with `hotbar` in its first hotbar slots, standing next
/// to `block` at [`AT`], which it has just right-clicked open.
fn open_table(block_id: i32, size: i16, hotbar: &[ItemStackSummary]) -> (Table, TickResult) {
    let (sender, receiver) = mailbox();
    let mut config = recipe_crafted::utility_config(
        block_id,
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..datapack_config()
        },
    );
    if block_id == bbb_protocol::block_types::VANILLA_BLOCK_ENCHANTING_TABLE_ID {
        use crate::tests::parse_advancement;
        use bbb_server_datapack::advancement::AdvancementElement;
        use serde_json::json;

        let mut startup = (*config.datapacks.take().unwrap()).clone();
        for (name, selected) in [("nbt", "diamond"), ("wrong_nbt", "stick")] {
            let predicate =
                json!({"nbt":format!("{{SelectedItem:{{id:'minecraft:{selected}',count:1}}}}")});
            for (suffix, player) in [
                ("object", predicate.clone()),
                (
                    "list",
                    json!([{"condition":"entity_properties","entity":"this","predicate":predicate}]),
                ),
            ] {
                startup.advancements.advancements.push(AdvancementElement {
                    id: format!("test:{name}_{suffix}"),
                    source_pack: "test".into(),
                    advancement: parse_advancement(&json!({"criteria":{"enchanted":{
                        "trigger":"minecraft:enchanted_item","conditions":{
                            "player":player,"item":{"items":"minecraft:iron_sword"},"levels":1
                        }
                    }}}))
                    .unwrap(),
                });
            }
        }
        for (name, team) in [
            ("world_conditions", "enchanters"),
            ("wrong_world_conditions", "other"),
        ] {
            startup.advancements.advancements.push(AdvancementElement {
                id: format!("test:{name}"),
                source_pack: "test".into(),
                advancement: parse_advancement(&json!({"criteria":{"enchanted":{
                    "trigger":"minecraft:enchanted_item","conditions":{"player":[
                        {"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":team}},
                        {"condition":"minecraft:entity_scores","entity":"this","scores":{"wisdom":2}},
                        {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                            "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"wisdom"},"range":1},
                        {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
                        {"condition":"minecraft:environment_attribute_check",
                            "attribute":"minecraft:gameplay/water_evaporates","value":false},
                        {"condition":"minecraft:location_check","predicate":{"biomes":"minecraft:plains"}}
                    ]}
                }}})).unwrap(),
            });
        }
        config.datapacks = Some(Arc::new(startup));
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_experience_levels_for_test(ConnectionId(1), 30);
    for (index, item) in hotbar.iter().enumerate() {
        world.stock_menu_slot_for_test(ConnectionId(1), 36 + index as i16, item.clone());
    }
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block_id).unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        AT,
        state,
        SET_BLOCK_UPDATE_ALL
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, AT, 1);
    (
        Table {
            sender,
            receiver,
            world,
            size,
        },
        opened,
    )
}

impl Table {
    fn play(&mut self, packet: PlayServerbound) -> TickResult {
        play(&self.sender, &self.receiver, &mut self.world, 1, packet)
    }

    fn click(&mut self, slot_num: i16, button_num: i8) -> TickResult {
        let inventory = self.world.player_inventory_for_test(ConnectionId(1));
        let (container_id, state_id) = (inventory.container_id(), inventory.state_id());
        self.play(PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num,
            button_num,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }))
    }

    /// The menu slot of hotbar slot `index`.
    fn hotbar_slot(&self, index: i16) -> i16 {
        self.size + 27 + index
    }

    /// Moves the whole stack in hotbar slot `index` into menu slot `slot`.
    fn place(&mut self, index: i16, slot: i16) {
        self.click(self.hotbar_slot(index), 0);
        self.click(slot, 0);
    }

    fn button(&mut self, button_id: i32) -> TickResult {
        let container_id = self
            .world
            .player_inventory_for_test(ConnectionId(1))
            .container_id();
        self.play(PlayServerbound::ContainerButtonClick(
            ContainerButtonClick {
                container_id,
                button_id,
            },
        ))
    }

    fn close(&mut self) {
        let container_id = self
            .world
            .player_inventory_for_test(ConnectionId(1))
            .container_id();
        self.play(PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id,
        }));
    }

    fn menu(&mut self) -> Vec<RuntimeItemStack> {
        self.world
            .player_inventory_for_test(ConnectionId(1))
            .container_items()
    }

    fn carried(&mut self) -> RuntimeItemStack {
        self.world
            .player_inventory_for_test(ConnectionId(1))
            .carried()
            .clone()
    }

    fn hotbar(&mut self, index: usize) -> RuntimeItemStack {
        self.world
            .player_inventory_for_test(ConnectionId(1))
            .inventory_slots()[index]
            .clone()
    }

    fn stat(&self, stat: &str) -> i32 {
        self.world
            .player_stat_for_test(ConnectionId(1), bbb_server_types::stats::custom_stat(stat))
    }

    fn crafted(&self, item_id: i32) -> i32 {
        self.world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                item_id,
            ),
        )
    }
}

fn assert_screen(opened: &TickResult, menu_type_id: i32, title: &str) {
    let screen = opened_screen(opened).expect("the block opens its menu");
    assert_eq!(screen.menu_type_id, menu_type_id);
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            title,
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap()
    );
}

fn level_event_types(tick: &TickResult) -> Vec<i32> {
    level_events_sent(tick)
        .into_iter()
        .filter(|(_, event)| event.pos == AT)
        .map(|(_, event)| event.event_type)
        .collect()
}

fn enchantment_holder(resource_id: &str) -> i32 {
    crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == resource_id)
        .unwrap() as i32
}

/// `AnvilMenu`: a rename costs one level, the result carries the name, the
/// take spends the level and plays the anvil event, and closing hands an
/// input back.
#[test]
fn an_anvil_renames_for_a_level_and_returns_its_inputs_on_close() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_ANVIL_ID,
        3,
        &[
            stack(items::VANILLA_ITEM_IRON_SWORD_ID, 1),
            stack(items::VANILLA_ITEM_IRON_SWORD_ID, 1),
        ],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_ANVIL_ID,
        "container.repair",
    );
    assert_eq!(table.stat("minecraft:interact_with_anvil"), 1);

    table.place(0, 0);
    table.play(PlayServerbound::RenameItem(
        bbb_protocol::packets::RenameItem {
            name: "Blade".to_string(),
        },
    ));
    let result = table.menu()[2].clone();
    assert_eq!(result.item_id(), Some(items::VANILLA_ITEM_IRON_SWORD_ID));
    assert_eq!(
        result.summary().component_patch.custom_name.as_deref(),
        Some("Blade")
    );
    // The result slot must encode for `container_set_slot`.
    bbb_protocol::packets::validate_data_component_patch_summary(&result.summary().component_patch)
        .expect("the renamed result encodes");
    assert_eq!(
        table
            .world
            .player_inventory_for_test(ConnectionId(1))
            .utility_menu_data(),
        Some(&[1][..])
    );

    let taken = table.click(2, 0);
    assert_eq!(table.carried(), result);
    assert!(table.menu()[0].is_empty());
    assert_eq!(table.world.experience_level_for_test(ConnectionId(1)), 29);
    let events = level_event_types(&taken);
    assert!(
        events == [1030] || events == [1029],
        "the take plays the anvil event, got {events:?}"
    );
    table.click(table.hotbar_slot(0), 0);

    table.place(1, 1);
    table.close();
    assert_eq!(
        table.hotbar(1).item_id(),
        Some(items::VANILLA_ITEM_IRON_SWORD_ID)
    );

    let raw_name = bbb_protocol::NbtSummaryValue::string_from_utf16_slice(&[0xd800]);
    let raw = RuntimeItemStack::from_summary(stack(items::VANILLA_ITEM_IRON_SWORD_ID, 1))
        .with_component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            Some(&raw_name),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .unwrap();
    let (mut table, _) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_ANVIL_ID,
        3,
        &[raw.summary().clone()],
    );
    table.place(0, 0);
    table.play(PlayServerbound::RenameItem(
        bbb_protocol::packets::RenameItem {
            name: "\u{fffd}".into(),
        },
    ));
    assert_eq!(
        table
            .world
            .player_inventory_for_test(ConnectionId(1))
            .utility_menu_data(),
        Some(&[1][..])
    );
    let renamed = table.menu()[2]
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            &table.world.item_stack_resolver(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        bbb_server_datapack::language::component_plain_text(&renamed),
        [0xfffd]
    );
}

/// `EnchantmentMenu`: an enchantable item and lapis show offers, the first
/// button enchants the item for one lapis and one level, and the
/// `enchanted_item` trigger completes `story/enchant_item`.
#[test]
fn an_enchanting_table_enchants_for_lapis_and_levels() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_ENCHANTING_TABLE_ID,
        2,
        &[
            stack(items::VANILLA_ITEM_IRON_SWORD_ID, 1),
            stack(items::VANILLA_ITEM_LAPIS_LAZULI_ID, 3),
            stack(items::VANILLA_ITEM_DIAMOND_ID, 1),
        ],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_ENCHANTMENT_ID,
        "container.enchant",
    );
    table.place(0, 0);
    table.place(1, 1);
    table.play(PlayServerbound::SetCarriedItem { slot: 2 });
    let offers = table
        .world
        .player_inventory_for_test(ConnectionId(1))
        .enchantment_offers();
    assert!(offers.costs[0] >= 1, "{offers:?}");
    assert!(offers.clues[0] >= 0, "{offers:?}");

    for (index, command) in [
        "team add enchanters",
        "team join enchanters alice",
        "scoreboard objectives add wisdom dummy",
        "scoreboard players set alice wisdom 2",
        "scoreboard players set #enabled wisdom 1",
        "time of minecraft:the_end pause",
        "time of minecraft:the_end set 107",
    ]
    .into_iter()
    .enumerate()
    {
        let tick = console_command(
            &table.sender,
            &table.receiver,
            &mut table.world,
            700 + index as u64,
            command,
        );
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    table.button(0);
    let enchanted = table.menu()[0].clone();
    assert!(!enchanted.summary().component_patch.enchantments.is_empty());
    assert_eq!(table.menu()[1].count(), 2);
    assert_eq!(table.world.experience_level_for_test(ConnectionId(1)), 29);
    assert_eq!(table.stat("minecraft:enchant_item"), 1);
    assert!(table
        .world
        .player_advancement_is_done(ConnectionId(1), "minecraft:story/enchant_item"));
    for suffix in ["object", "list"] {
        assert!(table
            .world
            .player_advancement_is_done(ConnectionId(1), &format!("test:nbt_{suffix}")));
        assert!(!table
            .world
            .player_advancement_is_done(ConnectionId(1), &format!("test:wrong_nbt_{suffix}")));
    }
    assert!(table
        .world
        .player_advancement_is_done(ConnectionId(1), "test:world_conditions"));
    assert!(!table
        .world
        .player_advancement_is_done(ConnectionId(1), "test:wrong_world_conditions"));

    table.close();
    assert_eq!(table.hotbar(0), enchanted);
    assert_eq!(table.hotbar(1).count(), 2);
}

/// `GrindstoneMenu`: an enchanted sword loses its enchantment, and the take
/// pops its experience with the grindstone event.
#[test]
fn a_grindstone_disenchants_and_pops_experience() {
    let mut sword = stack(items::VANILLA_ITEM_IRON_SWORD_ID, 1);
    sword.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        ],
        enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: enchantment_holder("minecraft:sharpness"),
            level: 1,
        }],
        ..Default::default()
    }
    .into();
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_GRINDSTONE_ID,
        3,
        &[sword],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GRINDSTONE_ID,
        "container.grindstone_title",
    );
    table.place(0, 0);
    let result = table.menu()[2].clone();
    assert_eq!(result.item_id(), Some(items::VANILLA_ITEM_IRON_SWORD_ID));
    assert!(result.summary().component_patch.enchantments.is_empty());

    let total = table.world.experience_total_for_test(ConnectionId(1));
    let taken = table.click(2, 0);
    assert_eq!(table.carried(), result);
    assert!(table.menu()[0].is_empty());
    assert_eq!(level_event_types(&taken), [1042]);
    // Sharpness I's minimum cost is one: one point, which the player standing
    // beside the block absorbs within the same tick.
    assert_eq!(
        table.world.experience_total_for_test(ConnectionId(1)),
        total + 1
    );
}

/// `SmithingMenu`: the netherite upgrade spends one of each input, learns
/// its recipe and plays the smithing event.
#[test]
fn a_smithing_table_upgrades_to_netherite() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_SMITHING_TABLE_ID,
        4,
        &[
            stack(
                items::VANILLA_ITEM_NETHERITE_UPGRADE_SMITHING_TEMPLATE_ID,
                2,
            ),
            stack(items::VANILLA_ITEM_DIAMOND_SWORD_ID, 1),
            stack(items::VANILLA_ITEM_NETHERITE_INGOT_ID, 2),
        ],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_SMITHING_ID,
        "container.upgrade",
    );
    table.place(0, 0);
    table.place(1, 1);
    table.place(2, 2);
    assert_eq!(
        table.menu()[3].item_id(),
        Some(items::VANILLA_ITEM_NETHERITE_SWORD_ID)
    );

    let taken = table.click(3, 0);
    assert!(table
        .world
        .player_advancement_is_done(ConnectionId(1), "test:crafted"));
    assert_eq!(
        table.carried().item_id(),
        Some(items::VANILLA_ITEM_NETHERITE_SWORD_ID)
    );
    let menu = table.menu();
    assert_eq!(menu[0].count(), 1);
    assert!(menu[1].is_empty());
    assert_eq!(menu[2].count(), 1);
    assert!(menu[3].is_empty());
    assert_eq!(level_event_types(&taken), [1044]);
    assert_eq!(table.crafted(items::VANILLA_ITEM_NETHERITE_SWORD_ID), 1);
    assert!(table
        .world
        .player_knows_recipe_for_test(ConnectionId(1), "minecraft:netherite_sword_smithing"));
}

/// `StonecutterMenu`: a button selects a recipe, and each take spends one
/// input and keeps the selection for the rest.
#[test]
fn a_stonecutter_cuts_the_selected_recipe() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_STONECUTTER_ID,
        2,
        &[stack(items::VANILLA_ITEM_STONE_ID, 2)],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_STONECUTTER_ID,
        "container.stonecutter",
    );
    table.place(0, 0);
    assert!(table.menu()[1].is_empty());
    table.button(0);
    let result = table.menu()[1].clone();
    assert!(!result.is_empty());

    table.click(1, 0);
    assert!(table
        .world
        .player_advancement_is_done(ConnectionId(1), "test:crafted"));
    assert_eq!(table.carried(), result);
    assert_eq!(table.menu()[0].count(), 1);
    assert_eq!(table.menu()[1], result);
    assert_eq!(table.crafted(result.item_id().unwrap()), result.count());
}

/// `LoomMenu`: a banner and a dye offer the patterns that need no item, the
/// button adds one layer, and the take spends the banner and the dye.
#[test]
fn a_loom_adds_a_pattern_layer() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_LOOM_ID,
        4,
        &[
            stack(items::VANILLA_ITEM_WHITE_BANNER_ID, 1),
            stack(items::VANILLA_ITEM_RED_DYE_ID, 1),
        ],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_LOOM_ID,
        "container.loom",
    );
    table.place(0, 0);
    table.place(1, 1);
    table.button(0);
    let result = table.menu()[3].clone();
    assert_eq!(result.item_id(), Some(items::VANILLA_ITEM_WHITE_BANNER_ID));
    assert_eq!(
        result.summary().component_patch.banner_pattern_layers.len(),
        1
    );

    table.click(3, 0);
    assert_eq!(table.carried(), result);
    let menu = table.menu();
    assert!(menu[0].is_empty() && menu[1].is_empty() && menu[3].is_empty());
}

/// `CartographyTableMenu`: paper zooms a map out, and the take makes the
/// new map and spends the inputs.
#[test]
fn a_cartography_table_zooms_a_map_out() {
    let mut map = stack(items::VANILLA_ITEM_FILLED_MAP_ID, 1);
    map.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
        ],
        map_id: Some(5),
        ..Default::default()
    }
    .into();
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_CARTOGRAPHY_TABLE_ID,
        3,
        &[map, stack(items::VANILLA_ITEM_PAPER_ID, 1)],
    );
    table
        .world
        .install_map_for_test(
            5,
            &bbb_server_persistence::Java26MapSavedData {
                dimension: "minecraft:overworld".to_string(),
                center_x: 0,
                center_z: 0,
                scale: 0,
                colors: vec![0; 128 * 128],
                tracking_position: true,
                unlimited_tracking: false,
                locked: false,
                banners: Vec::new(),
                frames: Vec::new(),
            },
            false,
        )
        .unwrap();
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID,
        "container.cartography_table",
    );
    table.place(0, 0);
    table.place(1, 1);
    let result = table.menu()[2].clone();
    assert_eq!(
        result.summary().component_patch.map_post_processing,
        Some(bbb_protocol::packets::MapPostProcessingSummary::Scale)
    );

    table.click(2, 0);
    let carried = table.carried();
    assert_eq!(carried.item_id(), Some(items::VANILLA_ITEM_FILLED_MAP_ID));
    assert!(carried
        .summary()
        .component_patch
        .map_post_processing
        .is_none());
    assert!(carried.map_id().is_some_and(|id| id != 5));
    let menu = table.menu();
    assert!(menu.iter().all(RuntimeItemStack::is_empty));
}

/// `BeaconMenu.updateEffects`: the payment is spent and the block entity
/// takes the chosen power, which the menu data then reports.
#[test]
fn a_beacon_spends_its_payment_on_a_power() {
    let (mut table, opened) = open_table(
        bbb_protocol::block_types::VANILLA_BLOCK_BEACON_ID,
        1,
        &[stack(items::VANILLA_ITEM_IRON_INGOT_ID, 2)],
    );
    assert_screen(
        &opened,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_BEACON_ID,
        "container.beacon",
    );
    assert_eq!(table.stat("minecraft:interact_with_beacon"), 1);
    // The payment slot holds one item.
    table.place(0, 0);
    assert_eq!(table.menu()[0].count(), 1);
    assert_eq!(table.carried().count(), 1);
    table.click(table.hotbar_slot(0), 0);

    let speed = bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID;
    let set = table.play(PlayServerbound::SetBeacon(
        bbb_protocol::packets::SetBeacon {
            primary_effect: Some(speed),
            secondary_effect: None,
        },
    ));
    assert!(table.menu()[0].is_empty());
    let beacon = table
        .world
        .get_block_entity(Java26BuiltinDimension::Overworld, AT)
        .and_then(crate::block_entity::BlockEntity::beacon)
        .unwrap();
    assert_eq!(beacon.primary_power(), Some("minecraft:speed"));
    assert_eq!(beacon.secondary_power(), None);
    assert!(set.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetData(data),
            ..
        } if data.id == 1 && i32::from(data.value) == speed + 1
    )));
}

/// `BeaconBlockEntity.createMenu` (`BeaconBlockEntity.java:318-325`): a locked
/// beacon refuses a player without the key, and still counts the interaction.
#[test]
fn a_locked_beacon_needs_its_key() {
    let (mut table, _) = open_table(bbb_protocol::block_types::VANILLA_BLOCK_BEACON_ID, 1, &[]);
    table.close();
    let tick = crate::tests::entities::console_command(
        &table.sender,
        &table.receiver,
        &mut table.world,
        20,
        "data merge block 0 64 1 {lock:{items:\"minecraft:diamond\"}}",
    );
    assert!(console_command_answer(&tick).0.is_some());
    let refused = right_click(&table.sender, &table.receiver, &mut table.world, 1, AT, 1);
    assert!(opened_screen(&refused).is_none());
    assert!(refused.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(chat),
            ..
        } if format!("{chat:?}").contains("container.isLocked")
    )));
    assert_eq!(table.stat("minecraft:interact_with_beacon"), 2);

    table.world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        stack(items::VANILLA_ITEM_DIAMOND_ID, 1),
    );
    let opened = right_click(&table.sender, &table.receiver, &mut table.world, 1, AT, 1);
    assert!(opened_screen(&opened).is_some());
}
