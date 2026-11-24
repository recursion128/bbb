use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::tests::{chat_command, join_loaded, mailbox, operator_config, system_chats};
use crate::world::ConnectionId;
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_datapack::loot::{LootElement, LootElementValue};
use bbb_server_types::save::Java26BuiltinDimension;

/// `/give` over a live session: `Inventory.add` (`GiveCommand.java:64`) puts
/// the default stack into the player inventory and words
/// `commands.give.success.single` (`:93-97`).
#[test]
fn give_puts_a_default_stack_into_the_live_inventory() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let stone = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 2,
        ..ItemStackSummary::empty()
    });
    let inventory = &mut world.levels[ConnectionId(1)].inventory;
    inventory.set_inventory_slot(8, stone.clone());
    inventory.set_ender_chest_runtime_slot(3, stone).unwrap();
    let original = inventory.slot(8).cloned().unwrap();
    let given = chat_command(&sender, &receiver, &mut world, 1, "give op diamond 3");
    assert!(
        system_chats(&given)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.give.success.single")),
        "{:?}",
        system_chats(&given)
    );
    assert!(
        given.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(entity), .. }
                if entity.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
        )),
        "the successful insertion publishes its fake item: {:?}",
        given.events
    );
    let slots = &world.publication().players[0].inventory.slots;
    let diamond = slots
        .iter()
        .find(|slot| slot.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID));
    assert_eq!(diamond.map(|slot| slot.count), Some(3), "{slots:?}");
    assert_eq!(
        world.levels[ConnectionId(1)].inventory.slot(8),
        Some(&original)
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op enderchest.3 with dirt 2",
    );
    chat_command(&sender, &receiver, &mut world, 1, "clear op");
    assert!(world.levels[ConnectionId(1)]
        .inventory
        .slot(8)
        .unwrap()
        .is_empty());
}

fn loot_config(id: &str, table: &str) -> crate::world::ServerWorldConfig {
    let mut config = operator_config(&[("op", 1)]);
    add_loot_table(&mut config, id, table);
    config
}

fn add_loot_table(config: &mut crate::world::ServerWorldConfig, id: &str, table: &str) {
    let bbb_protocol::ScannedCommandResourceOrId::Inline { value, .. } =
        bbb_protocol::scan_command_resource_or_id(table, 0).unwrap()
    else {
        unreachable!()
    };
    let mut startup = config
        .datapacks
        .as_deref()
        .cloned()
        .unwrap_or_else(|| (*crate::registry_sync::builtin_datapack_startup_shared()).clone());
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    registry.elements.retain(|element| element.id != id);
    registry.elements.push(LootElement {
        id: id.into(),
        source_pack: "test".into(),
        value: LootElementValue::Table(
            bbb_server_datapack::loot::parse_loot_table_nbt(&value, None).unwrap(),
        ),
    });
    registry.elements.sort_by(|a, b| a.id.cmp(&b.id));
    config.datapacks = Some(std::sync::Arc::new(startup));
}

#[test]
fn loot_give_keeps_native_results_and_reports_actual_inventory_delivery() {
    let table = "{type:'minecraft:chest',random_sequence:'bbb:command_delivery',pools:[{rolls:1,conditions:[{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player'}}],entries:[{type:'minecraft:item',name:'minecraft:raw_iron',functions:[{function:'minecraft:furnace_smelt'},{function:'minecraft:set_count',count:{type:'minecraft:uniform',min:2,max:5}},{function:'minecraft:set_components',components:{'minecraft:lock':{predicates:{'minecraft:max_damage':{}}},'minecraft:custom_data':{bbb:1}}}]}]}]}";
    let config = loot_config("bbb:delivery", table);
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "scoreboard objectives add obj dummy",
    );

    let untouched = world.levels[ConnectionId(1)]
        .inventory
        .slot(8)
        .cloned()
        .unwrap();
    let delivered = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute store result score #loot obj run loot give @a loot bbb:delivery",
    );
    assert!(
        system_chats(&delivered)
            .iter()
            .any(|(_, text)| text.contains("commands.drop.success.multiple")),
        "{:?}",
        system_chats(&delivered)
    );
    assert_eq!(world.scoreboard().score("#loot", "obj").unwrap().value, 2);
    assert_eq!(
        world.levels[ConnectionId(1)].inventory.slot(8),
        Some(&untouched)
    );
    let first = world.levels[ConnectionId(1)].inventory.inventory_slots()[0].clone();
    let second = &world.levels[ConnectionId(2)].inventory.inventory_slots()[0];
    assert!((2..=5).contains(&first.count()));
    assert_eq!(
        first.item_identity_summary().item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID)
    );
    assert!(first.same_item_same_components(second));
    assert_eq!(first.count(), second.count());
    assert!(world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());

    assert!(!delivered.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(entity), .. }
            if entity.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID
    )));
    let pickup = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.item.pickup",
        )
        .unwrap(),
    };
    assert!(!delivered.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } if sound.sound == pickup
    )));

    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival op");
    let full = bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_COBBLESTONE_ID),
        count: 64,
        component_patch: Default::default(),
    });
    for slot in 0..36 {
        world.levels[ConnectionId(1)]
            .inventory
            .set_inventory_slot(slot, full.clone());
    }
    let delivered = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute store result score #loot obj run loot give @a loot bbb:delivery",
    );
    assert_eq!(world.scoreboard().score("#loot", "obj").unwrap().value, 1);
    assert!(
        system_chats(&delivered)
            .iter()
            .any(|(_, text)| text.contains("commands.drop.success.single")),
        "{:?}",
        system_chats(&delivered)
    );
    let received =
        world.levels[ConnectionId(2)].inventory.inventory_slots()[0].count() - first.count();
    assert!((2..=5).contains(&received));
    assert!(world.levels[ConnectionId(2)].inventory.inventory_slots()[0]
        .same_item_same_components(&first));
    assert!(world.levels[ConnectionId(2)].inventory.inventory_slots()[1].is_empty());
    assert!(world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
}

#[test]
fn loot_spawn_keeps_native_stacks_at_target_and_publishes_item_entities() {
    let table = "{type:'minecraft:chest',pools:[{rolls:1,entries:[{type:'minecraft:item',name:'minecraft:diamond',functions:[{function:'minecraft:set_count',count:3},{function:'minecraft:set_components',components:{'minecraft:lock':{predicates:{'minecraft:max_damage':{}}},'minecraft:custom_data':{bbb:1}}}]}]}]}";
    let mut world = ServerWorld::new(loot_config("bbb:spawn", table)).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "scoreboard objectives add obj dummy",
    );
    let position = world.levels[ConnectionId(1)].position();
    let target = bbb_protocol::packets::Vec3d {
        x: position.x + 2.25,
        y: position.y,
        z: position.z + 2.75,
    };
    let spawned = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(
            "execute store result score #loot obj run loot spawn {} {} {} loot bbb:spawn",
            target.x, target.y, target.z
        ),
    );
    assert!(
        system_chats(&spawned)
            .iter()
            .any(|(_, text)| text.contains("commands.drop.success.single")),
        "{:?}",
        system_chats(&spawned)
    );
    assert_eq!(world.scoreboard().score("#loot", "obj").unwrap().value, 1);
    let drops = world.stored_items_for_test(Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].1, target);
    assert_eq!(drops[0].2, "minecraft:diamond");
    assert_eq!(drops[0].3, 3);
    assert_eq!(drops[0].4, crate::entity::item::DEFAULT_PICKUP_DELAY);
    let stack = world
        .stored_item_stack_for_test(Java26BuiltinDimension::Overworld, drops[0].0)
        .unwrap();
    let resolver = world.item_stack_resolver();
    let compound = |name: &str, value| {
        NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: name.into(),
            value,
        }])
    };
    assert_eq!(
        stack
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                &resolver
            )
            .unwrap(),
        Some(compound("bbb", NbtSummaryValue::Int(1)))
    );
    assert_eq!(
        stack
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                &resolver
            )
            .unwrap(),
        Some(compound(
            "predicates",
            compound(
                "minecraft:max_damage",
                NbtSummaryValue::Compound(Vec::new())
            )
        ))
    );
    assert!(spawned.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(entity), .. } if entity.id == drops[0].0
    )), "{:?}", spawned.events);
}

#[test]
fn loot_location_checks_use_the_execution_dimension_and_current_daylight() {
    let table = "{type:'minecraft:chest',pools:[{rolls:1,conditions:[{condition:'minecraft:location_check',predicate:{dimension:'minecraft:the_nether'}}],entries:[{type:'minecraft:item',name:'minecraft:diamond'}]}]}";
    let mut world = ServerWorld::new(loot_config("bbb:dimension", table)).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let diamonds = |world: &ServerWorld| {
        world.publication().players[0]
            .inventory
            .slots
            .iter()
            .filter(|slot| slot.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID))
            .map(|slot| slot.count)
            .sum::<i32>()
    };
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "loot give op loot bbb:dimension",
    );
    assert_eq!(diamonds(&world), 0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run loot give op loot bbb:dimension",
    );
    assert_eq!(diamonds(&world), 1);
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Overworld
    );

    let mut position = world.levels[ConnectionId(1)].position();
    position.y = 100.0;
    world.levels[ConnectionId(1)].set_position(position);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    let daylight_modifier = "item modify entity op hotbar.0 {function:'minecraft:set_count',count:7,conditions:[{condition:'minecraft:location_check',predicate:{light:{light:{min:12}}}}]}";
    for (time, expected) in [(6000, 7), (18000, 1)] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("time set {time}"),
        );
        for _ in 0..40 {
            world.tick(Vec::new());
        }
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "item replace entity op hotbar.0 with dirt",
        );
        chat_command(&sender, &receiver, &mut world, 1, daylight_modifier);
        assert_eq!(
            world.levels[ConnectionId(1)]
                .inventory
                .slot(0)
                .unwrap()
                .count(),
            expected,
            "time {time}"
        );
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item modify entity op hotbar.0 {function:'minecraft:set_count',count:7,conditions:[{condition:'minecraft:location_check',predicate:{light:{light:{max:4}}}}]}",
    );
    assert_eq!(
        world.levels[ConnectionId(1)]
            .inventory
            .slot(0)
            .unwrap()
            .count(),
        7
    );
}

#[test]
fn loot_boolean_environment_checks_read_execution_dimensions_and_compare_both_values() {
    let pools = [
        ("piglins_zombify", true, "diamond"),
        ("piglins_zombify", false, "emerald"),
        ("water_evaporates", true, "gold_ingot"),
        ("water_evaporates", false, "iron_ingot"),
    ]
    .map(|(attribute, value, item)| {
        serde_json::json!({
            "rolls": 1,
            "conditions": [{"condition": "minecraft:environment_attribute_check",
                "attribute": format!("minecraft:gameplay/{attribute}"), "value": value}],
            "entries": [{"type": "minecraft:item", "name": format!("minecraft:{item}")}]
        })
    });
    let table = serde_json::json!({"type": "minecraft:chest", "pools": pools}).to_string();
    let mut world = ServerWorld::new(loot_config("bbb:boolean_environment", &table)).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for (dimension, expected) in [
        ("overworld", [1, 0, 0, 1]),
        ("the_nether", [0, 1, 1, 0]),
        ("the_end", [1, 0, 0, 1]),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, "clear op");
        let output = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!(
                "execute in minecraft:{dimension} run loot give op loot bbb:boolean_environment"
            ),
        );
        for (item_id, count) in [
            bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID,
            bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID,
            bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID,
            bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID,
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(
                world.publication().players[0]
                    .inventory
                    .slots
                    .iter()
                    .filter(|slot| slot.item_id == Some(item_id))
                    .map(|slot| slot.count)
                    .sum::<i32>(),
                count,
                "{dimension}: {:?}",
                system_chats(&output)
            );
        }
    }
    for (dimension, expected) in [("overworld", 1), ("the_nether", 7)] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "item replace entity op hotbar.0 with dirt",
        );
        chat_command(&sender, &receiver, &mut world, 1,
            &format!("execute in minecraft:{dimension} run item modify entity op hotbar.0 {{function:'minecraft:set_count',count:7,conditions:[{{condition:'minecraft:environment_attribute_check',attribute:'minecraft:gameplay/water_evaporates',value:true}}]}}"));
        assert_eq!(
            world.levels[ConnectionId(1)]
                .inventory
                .slot(0)
                .unwrap()
                .count(),
            expected
        );
    }
}

#[test]
fn loot_environment_attributes_sample_origin_biomes_and_current_clock() {
    let table = "{type:'minecraft:chest',pools:[{rolls:1,conditions:[{condition:'minecraft:environment_attribute_check',attribute:'minecraft:gameplay/piglins_zombify',value:true},{condition:'minecraft:environment_attribute_check',attribute:'minecraft:gameplay/water_evaporates',value:true}],entries:[{type:'minecraft:item',name:'minecraft:diamond'}]}]}";
    let mut config = loot_config("bbb:layered_environment", table);
    add_loot_table(&mut config, "bbb:environment_numbers", "{type:'minecraft:chest',pools:[{rolls:1,entries:[{type:'minecraft:item',name:'minecraft:dirt',functions:[{function:'minecraft:set_custom_model_data',floats:{mode:'replace_all',values:[{type:'minecraft:environment_attribute',attribute:'minecraft:gameplay/surface_slime_spawn_chance'},{type:'minecraft:environment_attribute',attribute:'minecraft:gameplay/cat_waking_up_gift_chance'}]}},{function:'minecraft:set_count',count:{type:'minecraft:environment_attribute',attribute:'minecraft:gameplay/cat_waking_up_gift_chance'}}]}]}]}");
    let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (registry, id) in [
        ("minecraft:dimension_type", "minecraft:overworld"),
        ("minecraft:worldgen/biome", "minecraft:plains"),
        ("minecraft:worldgen/biome", "minecraft:desert"),
        ("minecraft:timeline", "minecraft:early_game"),
    ] {
        let element = startup
            .registries
            .registries
            .iter_mut()
            .find(|r| r.registry_id == registry)
            .unwrap()
            .elements
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
        match registry {
            "minecraft:dimension_type" => {
                value["timelines"] = serde_json::json!(["minecraft:early_game"]);
                value["attributes"]["minecraft:gameplay/piglins_zombify"] =
                    serde_json::json!(false);
                value["attributes"]["minecraft:gameplay/surface_slime_spawn_chance"] =
                    serde_json::json!(0.8);
                value["attributes"]["minecraft:gameplay/cat_waking_up_gift_chance"] =
                    serde_json::json!(0.5);
            }
            "minecraft:worldgen/biome" => {
                value["attributes"]["minecraft:gameplay/piglins_zombify"] =
                    serde_json::json!(id == "minecraft:desert");
                value["attributes"]["minecraft:gameplay/surface_slime_spawn_chance"] = serde_json::json!({"modifier":"add","argument":if id == "minecraft:desert" { 0.7 } else { 0.0 }});
            }
            _ => {
                value = serde_json::json!({"clock": "minecraft:overworld", "period_ticks": 24000,
                "tracks": {"minecraft:gameplay/water_evaporates": {
                    "keyframes": [{"ticks": 0, "value": false}, {"ticks": 12000, "value": true}]
                }, "minecraft:gameplay/surface_slime_spawn_chance": {
                    "modifier":"add", "keyframes": [{"ticks":0,"value":-0.75},{"ticks":12000,"value":-1.0}]
                }}})
            }
        }
        element.source_json = serde_json::to_vec(&value).unwrap().into();
        element.source = Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
                .unwrap(),
        );
        element.codec_value = if registry == "minecraft:timeline" {
            let data =
                bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap();
            let decoded =
                bbb_registry_codec::decode_exact_synchronized_registry_element(registry, &data)
                    .unwrap();
            Some(decoded.into_canonical().unwrap())
        } else {
            None
        };
    }
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let level = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    let biome_ids = ["minecraft:plains", "minecraft:desert"].map(|id| {
        world
            .registries
            .registry_element_index("minecraft:worldgen/biome", id)
            .unwrap()
    });
    let chunks = &mut world.level_mut_by_id(&level).chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    let column = chunks.column_mut(0, 0).unwrap();
    for (quart_x, biome) in biome_ids.into_iter().enumerate() {
        column
            .set_noise_biome(quart_x as i32, 16, 0, biome)
            .unwrap();
    }
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    for (ticks, x, expected, chance) in [
        (0, 4.5, 0, 0.75_f32),
        (12000, 0.5, 0, 0.0),
        (12000, 4.5, 1, 0.5),
    ] {
        let mut state = world.world_clock_states_for_test()[clock];
        state.total_ticks = ticks;
        state.paused = true;
        world.set_world_clock_state_for_test(clock, state);
        chat_command(&sender, &receiver, &mut world, 1, "clear op");
        let output = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("execute positioned {x} 64 0.5 run loot give op loot bbb:layered_environment"),
        );
        assert_eq!(
            world.publication().players[0]
                .inventory
                .slots
                .iter()
                .filter(
                    |slot| slot.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
                )
                .map(|slot| slot.count)
                .sum::<i32>(),
            expected,
            "time {ticks}, x {x}: {:?}",
            system_chats(&output)
        );
        chat_command(&sender, &receiver, &mut world, 1,
            &format!("execute positioned {x} 64 0.5 run loot replace entity op hotbar.0 loot bbb:environment_numbers"));
        let resolver = world.item_stack_resolver();
        let actual = world.levels[ConnectionId(1)].inventory.slot(0).unwrap().component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            &resolver,
        ).unwrap().unwrap();
        assert_eq!(
            crate::loot_runtime::nbt_field(&actual, "floats"),
            Some(&bbb_protocol::NbtSummaryValue::List(vec![
                bbb_protocol::NbtSummaryValue::Float(chance.to_bits()),
                bbb_protocol::NbtSummaryValue::Float(0.5_f32.to_bits())
            ])),
            "time {ticks}, x {x}"
        );
        assert_eq!(
            world.levels[ConnectionId(1)]
                .inventory
                .slot(0)
                .unwrap()
                .count(),
            1
        );
    }
}

#[test]
fn loot_replaces_slots_and_inserts_into_only_the_named_chest_half() {
    let table = "{type:'minecraft:chest',pools:[{rolls:1,entries:[{type:'minecraft:item',name:'minecraft:diamond',functions:[{function:'minecraft:set_count',count:3},{function:'minecraft:set_components',components:{'minecraft:custom_data':{bbb:1}}}]}]}]}";
    let fish = "{type:'minecraft:fishing',pools:[{rolls:1,conditions:[{condition:'minecraft:match_tool',predicate:{items:'minecraft:diamond'}}],entries:[{type:'minecraft:item',name:'minecraft:emerald'}]}]}";
    let mut config = loot_config("bbb:slots", table);
    add_loot_table(&mut config, "bbb:fish", fish);
    for (id, item, count) in [
        ("book", "book", 64),
        ("fuel", "coal", 1),
        ("bottle", "glass_bottle", 1),
        ("ingredient", "nether_wart", 1),
        ("shulker", "shulker_box", 1),
    ] {
        add_loot_table(&mut config, &format!("bbb:{id}"), &format!("{{type:'minecraft:chest',pools:[{{rolls:1,entries:[{{type:'minecraft:item',name:'minecraft:{item}',functions:[{{function:'minecraft:set_count',count:{count}}}]}}]}}]}}"));
    }
    let tags = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap())
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:item")
        .unwrap();
    tags.tags
        .get_mut("minecraft:bookshelf_books")
        .unwrap()
        .push("minecraft:diamond".into());
    *tags.tags.get_mut("minecraft:brewing_fuel").unwrap() = vec!["minecraft:coal".into()];
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let level = world.levels[ConnectionId(1)].dimension;
    let position = world.levels[ConnectionId(1)].position();
    let pos = bbb_protocol::packets::BlockPos {
        x: position.x.floor() as i32 + 2,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    let other = bbb_protocol::packets::BlockPos {
        x: pos.x + 1,
        ..pos
    };
    let at = format!("{} {} {}", pos.x, pos.y, pos.z);
    let beside = format!("{} {} {}", other.x, other.y, other.z);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    for command in [
        format!("setblock {at} chest[facing=north,type=left]"),
        format!("setblock {beside} chest[facing=north,type=right]"),
        "item replace entity op hotbar.1 with dirt".into(),
        "item replace entity op hotbar.2 with dirt".into(),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, &command);
    }
    let replaced_entity = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "loot replace entity op hotbar.0 3 loot bbb:slots",
    );
    assert!(replaced_entity.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::ContainerSetSlot(update) }
        if (update.slot == 37 || update.slot == 38) && update.item.count == 0)));
    let inventory = &world.levels[ConnectionId(1)].inventory;
    let prototype = inventory.slot(0).unwrap().clone();
    assert_eq!(prototype.count(), 3);
    assert_eq!(prototype.item_resource_id(), Some("minecraft:diamond"));
    assert!(inventory.slot(1).unwrap().is_empty());
    assert!(inventory.slot(2).unwrap().is_empty());
    let replaced = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("loot replace block {at} container.25 2147483647 loot bbb:slots"),
    );
    assert!(system_chats(&replaced)
        .iter()
        .any(|(_, text)| text.contains("commands.drop.success.multiple")));
    let items = world.container_items_at_by_key(&level, pos).unwrap();
    assert!(items[25].same_item_same_components(&prototype));
    assert!(items[26].is_empty());
    assert!(world
        .container_items_at_by_key(&level, other)
        .unwrap()
        .iter()
        .all(RuntimeItemStack::is_empty));
    let mut items = vec![crate::entity::item::plain_stack("minecraft:stone", 64).unwrap(); 27];
    let mut explicit = prototype.with_count(63).into_summary();
    let patch = &mut explicit.component_patch;
    patch.max_stack_size = Some(64);
    patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    patch.added_type_ids.sort_unstable();
    items[0] = RuntimeItemStack::from_summary(explicit);
    items[1] = prototype
        .with_count(63)
        .with_component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            None,
            &world.item_stack_resolver(),
        )
        .unwrap();
    items[2] = crate::entity::item::plain_stack("minecraft:diamond", 63).unwrap();
    world
        .write_container_items_at_by_key(&level, pos, items)
        .unwrap();
    let inserted = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("loot insert {at} loot bbb:slots"),
    );
    assert!(system_chats(&inserted)
        .iter()
        .any(|(_, text)| text.contains("commands.drop.success.single")));
    let items = world.container_items_at_by_key(&level, pos).unwrap();
    assert_eq!(items[0].count(), 64);
    assert_eq!(items[1].count(), 63);
    assert_eq!(items[2].count(), 63);
    assert!(world
        .container_items_at_by_key(&level, other)
        .unwrap()
        .iter()
        .all(RuntimeItemStack::is_empty));
    let full = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("loot insert {at} loot bbb:slots"),
    );
    assert!(system_chats(&full)
        .iter()
        .any(|(_, text)| text.contains("commands.drop.success.single")));
    // Fishing, tools and hand sources share every destination's executor.

    for (destination, suffix) in [
        ("replace entity op armor.head".to_owned(), "diamond"),
        (format!("replace block {at} container.24 2"), "mainhand"),
        (format!("insert {beside}"), "offhand"),
    ] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "item replace entity op weapon.offhand with diamond",
        );
        let rolled = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("loot {destination} fish bbb:fish {at} {suffix}"),
        );
        assert!(
            system_chats(&rolled)
                .iter()
                .any(|(_, text)| text.contains("commands.drop.success.")),
            "{:?}",
            system_chats(&rolled)
        );
    }
    assert_eq!(
        world.levels[ConnectionId(1)]
            .inventory
            .slot(39)
            .unwrap()
            .item_resource_id(),
        Some("minecraft:emerald")
    );
    assert_eq!(
        world.container_items_at_by_key(&level, other).unwrap()[0].item_resource_id(),
        Some("minecraft:emerald")
    );
    let places = [
        ("furnace", 2),
        ("brewing_stand", 4),
        ("chiseled_bookshelf", 6),
        ("oak_shelf", 8),
        ("shulker_box", 10),
    ];
    for (block, dz) in places {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("setblock {} {} {} {block}", pos.x, pos.y, pos.z + dz),
        );
    }
    let furnace_at = format!("{} {} {}", pos.x, pos.y, pos.z + 2);
    let brewing_at = format!("{} {} {}", pos.x, pos.y, pos.z + 4);
    let bookshelf_at = format!("{} {} {}", pos.x, pos.y, pos.z + 6);
    let shelf_at = format!("{} {} {}", pos.x, pos.y, pos.z + 8);
    let shulker_at = format!("{} {} {}", pos.x, pos.y, pos.z + 10);
    for command in [
        format!("loot replace block {furnace_at} container.1 2 loot bbb:slots"),
        format!("loot insert {furnace_at} loot bbb:slots"),
        format!("loot replace block {brewing_at} container.0 5 loot bbb:slots"),
        format!("loot replace block {brewing_at} container.3 loot bbb:ingredient"),
        format!("loot replace block {brewing_at} container.4 loot bbb:fuel"),
        format!("loot replace block {brewing_at} container.0 loot bbb:bottle"),
        format!("loot replace block {brewing_at} container.0 loot bbb:bottle"),
        format!("loot insert {bookshelf_at} loot bbb:book"),
        format!("loot replace block {bookshelf_at} container.0 loot bbb:slots"),
        format!("loot replace block {bookshelf_at} container.1 loot bbb:slots"),
        format!("loot replace block {shelf_at} container.0 loot bbb:book"),
        format!("loot replace block {shelf_at} container.0 loot bbb:slots"),
        format!("loot insert {shulker_at} loot bbb:shulker"),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, &command);
    }
    let at_offset = |dz| bbb_protocol::packets::BlockPos {
        z: pos.z + dz,
        ..pos
    };
    let furnace = world
        .container_items_at_by_key(&level, at_offset(2))
        .unwrap();
    assert_eq!(furnace[0].count(), 3);
    assert!(furnace[1..].iter().all(RuntimeItemStack::is_empty));
    let brewing = world
        .container_items_at_by_key(&level, at_offset(4))
        .unwrap();
    assert_eq!(
        brewing[0].item_resource_id(),
        Some("minecraft:glass_bottle")
    );
    assert!(brewing[1..3].iter().all(RuntimeItemStack::is_empty));
    assert_eq!(brewing[3].item_resource_id(), Some("minecraft:nether_wart"));
    assert_eq!(brewing[4].item_resource_id(), Some("minecraft:coal"));
    let bookshelf = world
        .level_by_id(&level)
        .chunks
        .block_entities()
        .get(at_offset(6))
        .unwrap()
        .chiseled_bookshelf()
        .unwrap();
    assert_eq!(
        bookshelf.original_items()[0].item_resource_id(),
        Some("minecraft:book")
    );
    assert_eq!(bookshelf.original_items()[0].count(), 64);
    assert_eq!(
        bookshelf.original_items()[1].item_resource_id(),
        Some("minecraft:diamond")
    );
    assert_eq!(bookshelf.redstone_signal(), 2);
    let body = world
        .level_by_id(&level)
        .chunks
        .block_entities()
        .get(at_offset(6))
        .unwrap();
    let resolver = world.item_stack_resolver();
    let saved = crate::block_entity::saved_record_with_resolver(body, &resolver).unwrap();
    let restored = crate::block_entity::BlockEntity::load_static_with_resolver(
        at_offset(6),
        body.block_state(),
        &saved,
        &resolver,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        restored.chiseled_bookshelf().unwrap().original_items(),
        bookshelf.original_items()
    );
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_property(
            world
                .level_by_id(&level)
                .chunks
                .get_block_state(pos.x, pos.y, pos.z + 6)
                .unwrap(),
            "slot_1_occupied"
        ),
        Some("true")
    );
    let shelf = world
        .container_items_at_by_key(&level, at_offset(8))
        .unwrap();
    assert_eq!(shelf[0].item_resource_id(), Some("minecraft:book"));
    assert_eq!(shelf[0].count(), 64);
    assert_eq!(
        world
            .container_items_at_by_key(&level, at_offset(10))
            .unwrap()[0]
            .item_resource_id(),
        Some("minecraft:shulker_box")
    );
}

#[test]
fn loot_delivers_only_playable_stacks_into_empty_jukeboxes() {
    let dir = std::env::temp_dir().join(format!("bbb-loot-jukebox-{}", uuid::Uuid::new_v4()));
    let pack = dir.join("test");
    std::fs::create_dir_all(pack.join("data/test/jukebox_song")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"loot jukebox","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(pack.join("data/test/jukebox_song/custom.json"), r#"{"sound_event":"minecraft:music_disc.cat","description":"Command song","length_in_seconds":30,"comparator_output":9}"#).unwrap();
    std::fs::write(pack.join("data/test/loot_table/playable.json"), r#"{"type":"minecraft:chest","pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:dirt"}]},{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:stone","functions":[{"function":"minecraft:set_count","count":3},{"function":"minecraft:set_components","components":{"minecraft:jukebox_playable":"test:custom"}}]}]}]}"#).unwrap();
    std::fs::write(pack.join("data/test/loot_table/plain.json"), r#"{"type":"minecraft:chest","pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:dirt"}]}]}"#).unwrap();
    std::fs::write(
        pack.join("data/test/loot_table/empty.json"),
        r#"{"type":"minecraft:chest","pools":[]}"#,
    )
    .unwrap();
    let replacement = r#"{"type":"minecraft:chest","pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:stone","functions":[{"function":"minecraft:set_count","count":3},{"function":"minecraft:set_components","components":{"minecraft:jukebox_playable":"test:custom"}}]}]}]}"#;
    std::fs::write(
        pack.join("data/test/loot_table/replacement.json"),
        replacement,
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/loot_table/overwrite.json"),
        replacement.replace("\"count\":3", "\"count\":4"),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    let mut config = operator_config(&[("op", 1)]);
    config.datapacks = Some(std::sync::Arc::new(startup));
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    let level = world.levels[ConnectionId(1)].dimension;
    let position = world.levels[ConnectionId(1)].position();
    let first = bbb_protocol::packets::BlockPos {
        x: position.x.floor() as i32 + 2,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    for (offset, destination) in [(0, "insert"), (2, "replace block")] {
        let pos = bbb_protocol::packets::BlockPos {
            z: first.z + offset,
            ..first
        };
        let at = format!("{} {} {}", pos.x, pos.y, pos.z);
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("setblock {at} jukebox"),
        );
        let suffix = if offset == 0 { "" } else { " container.0" };
        let plain = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("loot {destination} {at}{suffix} loot test:plain"),
        );
        assert!(!system_chats(&plain)
            .iter()
            .any(|(_, text)| text.contains("commands.drop.success.single")));
        assert!(world
            .level_by_id(&level)
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .jukebox()
            .unwrap()
            .item()
            .is_empty());
        let table = if offset == 0 {
            "test:playable"
        } else {
            "test:replacement"
        };
        let delivered = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("loot {destination} {at}{suffix} loot {table}"),
        );
        assert!(
            system_chats(&delivered)
                .iter()
                .any(|(_, text)| text.contains("commands.drop.success.single")),
            "{:?}",
            system_chats(&delivered)
        );
        let body = world
            .level_by_id(&level)
            .chunks
            .block_entities()
            .get(pos)
            .unwrap();
        let record = body.jukebox().unwrap();
        assert_eq!(record.item().item_resource_id(), Some("minecraft:stone"));
        assert_eq!(record.item().count(), 3);
        assert_eq!(record.song_player().song_id(), Some("test:custom"));
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_property(
                body.block_state(),
                "has_record"
            ),
            Some("true")
        );
        let resolver = world.item_stack_resolver();
        let saved = crate::block_entity::saved_record_with_resolver(body, &resolver).unwrap();
        let restored = crate::block_entity::BlockEntity::load_static_with_resolver(
            pos,
            body.block_state(),
            &saved,
            &resolver,
        )
        .unwrap()
        .unwrap();
        assert_eq!(restored.jukebox().unwrap().item(), record.item());
        assert_eq!(
            restored.jukebox().unwrap().song_player().song_id(),
            Some("test:custom")
        );
        assert_eq!(
            world.with_level(level, |level| level
                .analog_output_signal(pos, bbb_protocol::packets::Direction::North)),
            9
        );
        let song = world
            .registries
            .registry_element_index("minecraft:jukebox_song", "test:custom")
            .unwrap();
        assert!(delivered.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::LevelEvent(update), .. } if update.event_type == 1010 && update.pos == pos && update.data == song)));
        let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        assert!(events.iter().any(|event| event.pos == pos
            && event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE));
        for table in ["test:overwrite", "test:plain", "test:empty"] {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!("loot replace block {at} container.0 loot {table}"),
            );
            let unchanged = world
                .level_by_id(&level)
                .chunks
                .block_entities()
                .get(pos)
                .unwrap()
                .jukebox()
                .unwrap();
            assert_eq!(unchanged.item(), restored.jukebox().unwrap().item());
            assert_eq!(unchanged.song_player().song_id(), Some("test:custom"));
        }
    }
}

#[test]
fn loot_kill_rolls_custom_table_without_killing_or_consuming_it() {
    let table = "{type:'minecraft:entity',pools:[{rolls:1,conditions:[{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:cow'}},{condition:'minecraft:entity_properties',entity:'attacker',predicate:{type:'minecraft:player'}},{condition:'minecraft:entity_properties',entity:'attacking_player',predicate:{type:'minecraft:player'}},{condition:'minecraft:damage_source_properties',predicate:{is_direct:true,tags:[{id:'minecraft:bypasses_armor',expected:true}]}}],entries:[{type:'minecraft:item',name:'minecraft:diamond',functions:[{function:'minecraft:set_name',entity:'this',name:{selector:'@s'}},{function:'minecraft:set_lore',mode:'replace_all',entity:'attacker',lore:[{selector:'@s'}]}]}]}]}";
    let mut world = ServerWorld::new(loot_config("bbb:kill", table)).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    chat_command(&sender, &receiver, &mut world, 1, "summon cow ~2 ~ ~");
    chat_command(&sender, &receiver, &mut world, 1, "data merge entity @e[type=cow,limit=1] {DeathLootTable:'bbb:kill',DeathLootTableSeed:1234L}");
    for _ in 0..2 {
        let rolled = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "loot replace entity op hotbar.0 kill @e[type=cow,limit=1]",
        );
        assert!(
            system_chats(&rolled)
                .iter()
                .any(|(_, text)| text.contains("commands.drop.success.single_with_table")),
            "{:?}",
            system_chats(&rolled)
        );
    }
    let level = world.levels[ConnectionId(1)].dimension;
    let cow = world
        .level_by_id(&level)
        .entities
        .ids()
        .into_iter()
        .find_map(|id| {
            world.level_by_id(&level).entities.get(id).filter(|entity| {
                entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
            })
        })
        .unwrap();
    assert!(cow.is_alive_with_health());
    assert_eq!(
        crate::entity::mob_death_loot::entity_loot_table(cow),
        Some("bbb:kill")
    );
    let stack = world.levels[ConnectionId(1)].inventory.slot(0).unwrap();
    assert_eq!(stack.item_resource_id(), Some("minecraft:diamond"));
    let resolver = world.item_stack_resolver();
    assert!(format!(
        "{:?}",
        stack
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                &resolver
            )
            .unwrap()
    )
    .contains("cow"));
    assert!(format!(
        "{:?}",
        stack
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
                &resolver
            )
            .unwrap()
    )
    .contains("op"));
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID,
        world.levels[ConnectionId(1)].position(),
        crate::entity::EntityData::Marker(crate::entity::MarkerData::default()),
    );
    let no_table = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "loot give op kill @e[type=marker,limit=1]",
    );
    assert!(
        system_chats(&no_table)
            .iter()
            .any(|(_, text)| text.contains("commands.drop.no_loot_table.entity")),
        "{:?}",
        system_chats(&no_table)
    );
}

#[test]
fn loot_mine_reads_tool_and_block_entity_without_removing_either() {
    let table = "{type:'minecraft:block',pools:[{rolls:1,conditions:[{condition:'minecraft:match_tool',predicate:{items:'minecraft:diamond_pickaxe'}},{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player'}},{condition:'minecraft:location_check',offsetY:1,predicate:{block:{nbt:{id:'minecraft:chest',Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}}}}],entries:[{type:'minecraft:item',name:'minecraft:chest',functions:[{function:'minecraft:copy_components',source:'block_entity',include:['minecraft:custom_name']},{function:'minecraft:set_lore',mode:'replace_all',entity:'this',lore:[{selector:'@s'}]}]}]}]}";
    let mut world = ServerWorld::new(loot_config("minecraft:blocks/chest", table)).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "tick freeze");
    let position = world.levels[ConnectionId(1)].position();
    let at = format!(
        "{} {} {}",
        position.x.floor() as i32 + 2,
        position.y.floor() as i32,
        position.z.floor() as i32
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("setblock {at} chest"),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge block {at} {{CustomName:{{text:'Mine chest'}}}}"),
    );
    let above = format!(
        "{} {} {}",
        position.x.floor() as i32 + 2,
        position.y.floor() as i32 + 1,
        position.z.floor() as i32
    );
    for command in [
        format!("setblock {above} chest"),
        format!("item replace block {above} container.0 with diamond 3"),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, &command);
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op weapon.offhand with diamond_pickaxe",
    );
    for suffix in ["diamond_pickaxe", "offhand"] {
        let rolled = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("loot replace entity op hotbar.0 mine {at} {suffix}"),
        );
        assert!(
            system_chats(&rolled)
                .iter()
                .any(|(_, text)| text.contains("commands.drop.success.single_with_table")),
            "{:?}",
            system_chats(&rolled)
        );
    }
    let player = &world.levels[ConnectionId(1)];
    let drop = player.inventory.slot(0).unwrap();
    assert_eq!(drop.item_resource_id(), Some("minecraft:chest"));
    assert_eq!(drop.count(), 1);
    assert!(drop
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            &world.item_stack_resolver()
        )
        .unwrap()
        .is_some());
    assert!(format!(
        "{:?}",
        drop.component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
            &world.item_stack_resolver()
        )
        .unwrap()
    )
    .contains("op"));
    assert_eq!(
        player.inventory.slot(40).unwrap().item_resource_id(),
        Some("minecraft:diamond_pickaxe")
    );
    let pos = bbb_protocol::packets::BlockPos {
        x: position.x.floor() as i32 + 2,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    assert!(world
        .level_by_id(&player.dimension)
        .chunks
        .block_entities()
        .get(pos)
        .unwrap()
        .chest()
        .is_some());
    let air = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("loot give op mine {} {} {}", pos.x, pos.y + 3, pos.z),
    );
    assert!(system_chats(&air)
        .iter()
        .any(|(_, text)| text.contains("commands.drop.no_loot_table.block")));
}

mod environment_scalar;
