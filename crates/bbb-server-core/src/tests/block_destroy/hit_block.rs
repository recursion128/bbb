//! `EnchantmentHelper.onHitBlock` for the stack a survival dig starts with
//! (`ServerPlayerGameMode.java:187-196`).

use super::*;

use super::player_overrides::{item, Dig, MINER};

/// The stack the menu broadcast last published for the held hotbar slot.
fn republished_held_stack(tick: &TickResult) -> Option<ItemStackSummary> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.slot == 36 => Some(update.item.clone()),
            _ => None,
        })
        .next_back()
}

/// The built-in pack with `minecraft:efficiency` given `hit_block` entries.
fn hit_block_config(hit_block: serde_json::Value) -> ServerWorldConfig {
    let mut config = datapack_config();
    let mut startup = (**config.datapacks.as_ref().expect("datapacks")).clone();
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:enchantment")
        .expect("the enchantment registry")
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:efficiency")
        .expect("efficiency");
    let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    json["effects"]["minecraft:hit_block"] = hit_block;
    element.source_json = serde_json::to_vec(&json).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
    config.datapacks = Some(Arc::new(startup));
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..config
    }
}

/// `EnchantmentHelper.onHitBlock` (`ServerPlayerGameMode.java:187-196`): the
/// start of a survival dig runs each `hit_block` entry whose requirements
/// match `blockHitContext`, here `change_item_damage` on the main hand, and a
/// stack the damage breaks announces the mainhand break.
#[test]
fn starting_a_dig_runs_the_hit_block_effects_of_the_held_stack() {
    let positions = [
        BlockPos { x: 2, y: 64, z: 2 },
        BlockPos { x: 5, y: 64, z: 2 },
        BlockPos { x: 8, y: 64, z: 2 },
    ];
    let locations: Vec<_> = positions.iter().map(|pos| serde_json::json!({
        "condition": "minecraft:location_check", "offsetX": 1,
        "predicate": {"block": {"nbt": format!(
            "{{id:'minecraft:chest',x:{},y:{},z:{},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}",
            pos.x + 1, pos.y, pos.z
        )}}
    })).collect();
    let location = serde_json::json!({"condition": "minecraft:any_of", "terms": locations});
    let player = serde_json::json!({
        "condition": "minecraft:entity_properties", "entity": "this",
        "predicate": {"type": "minecraft:player",
            "nbt": "{SelectedItem:{id:'minecraft:diamond_pickaxe',count:1}}"}
    });
    let config = hit_block_config(serde_json::json!([
        {
            "effect": {"type": "minecraft:change_item_damage", "amount": 3},
            "requirements": {"condition": "minecraft:all_of", "terms": [
                {"condition": "minecraft:weather_check", "raining": false}, location, player
            ]}
        },
        {
            "effect": {"type": "minecraft:change_item_damage", "amount": 100},
            "requirements": {"condition": "minecraft:all_of", "terms": [
                {"condition": "minecraft:weather_check", "raining": true}, location, player
            ]}
        }
    ]));
    let mut dig = Dig::with_config(config, true);
    for pos in positions {
        let setup = dig.command(&format!(
            "setblock {} {} {} chest{{Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
            pos.x + 1,
            pos.y,
            pos.z
        ));
        let success = bbb_protocol::component_translatable_nbt(
            "commands.setblock.success",
            vec![
                NbtSummaryValue::Int(pos.x + 1),
                NbtSummaryValue::Int(pos.y),
                NbtSummaryValue::Int(pos.z),
            ],
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap();
        assert!(
            setup.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::SystemChat(update)
                } if *connection_id == MINER && update.content.component() == &success
            )),
            "{:?}",
            system_chats(&setup)
        );
    }
    let deopped = dig.command("deop miner");
    assert!(system_chats(&deopped).contains(&(MINER, "commands.deop.success miner".into())));
    let [mismatch, pos, other] = positions;
    let iron_pickaxe = dig.enchanted(item("minecraft:iron_pickaxe"), "minecraft:efficiency");
    dig.set(mismatch, block_state_id("minecraft:obsidian"));
    dig.hold(iron_pickaxe.clone());
    let tick = dig.dig(mismatch);
    assert_eq!(
        dig.world.player_inventory_slot_for_test(MINER, 0),
        iron_pickaxe
    );
    assert_ne!(dig.block(mismatch), Some(block_state_id("minecraft:air")));
    assert!(!tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::EntityEvent(update),
        } if *connection_id == MINER
            && update.event_id == crate::item_durability::ENTITY_EVENT_MAINHAND_ITEM_BROKEN
    )));
    let pickaxe = dig.enchanted(item("minecraft:diamond_pickaxe"), "minecraft:efficiency");
    dig.set(pos, block_state_id("minecraft:obsidian"));
    dig.hold(pickaxe.clone());

    let tick = dig.dig(pos);
    let held = republished_held_stack(&tick).expect("the damaged stack is republished");
    assert_eq!(held.component_patch.damage, Some(3), "{held:?}");
    assert_ne!(dig.block(pos), Some(block_state_id("minecraft:air")));

    let max_damage = crate::inventory::item_prototype_int_component(
        pickaxe.item_id.expect("a pickaxe"),
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    )
    .expect("a pickaxe is damageable");
    let mut worn = pickaxe;
    worn.component_patch.damage = Some(max_damage - 2);
    worn.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    dig.set(other, block_state_id("minecraft:obsidian"));
    dig.hold(worn);
    let tick = dig.dig(other);
    assert_eq!(
        republished_held_stack(&tick),
        Some(ItemStackSummary::empty())
    );
    let breaks: Vec<_> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityEvent(update),
            } => Some((*connection_id, update.event_id)),
            _ => None,
        })
        .collect();
    assert!(
        breaks.contains(&(
            MINER,
            crate::item_durability::ENTITY_EVENT_MAINHAND_ITEM_BROKEN
        )),
        "{breaks:?}"
    );
}

#[test]
fn hit_block_conditions_read_the_named_world_clock() {
    let mut config = ServerWorldConfig {
        game_type: GameType::Survival,
        ..datapack_config()
    };
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:efficiency")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = serde_json::json!({"minecraft:hit_block":[{
        "effect":{"type":"minecraft:change_item_damage","amount":3},
        "requirements":{"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
    }, {
        "effect":{"type":"minecraft:change_item_damage","amount":7},
        "requirements":{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:player","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":"minecraft:diamond","count":1}}
            }}}
    }]});
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:location_check",
            "predicate":{"dimension":"minecraft:overworld"}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties",
            "entity":"this","predicate":{}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-hit-block-cargo-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Hit block cargo","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, value) in [
        ("data/minecraft/enchantment/efficiency.json", document),
        ("data/test/loot_table/hit_vehicle.json", cargo),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    for (time, wear) in [(107, 3), (108, 0)] {
        let mut dig = Dig::with_config(config.clone(), true);
        for input in [
            "time of minecraft:the_end pause".to_string(),
            "time of minecraft:overworld pause".to_string(),
            format!("time of minecraft:the_end set {time}"),
            "time of minecraft:overworld set 6000".to_string(),
        ] {
            let tick = dig.command(&input);
            assert!(
                system_chats(&tick)
                    .iter()
                    .any(|(_, line)| line.starts_with("commands.time.")),
                "{input}: {:?}",
                system_chats(&tick)
            );
        }
        assert_eq!(dig.world.clock_total_ticks("minecraft:the_end"), Some(time));
        assert_eq!(
            dig.world.clock_total_ticks("minecraft:overworld"),
            Some(6000)
        );
        let pos = BlockPos { x: 2, y: 64, z: 2 };
        dig.set(pos, block_state_id("minecraft:obsidian"));
        let tool = dig.enchanted(item("minecraft:diamond_pickaxe"), "minecraft:efficiency");
        dig.hold(tool);
        let result = dig.dig(pos);
        assert_eq!(
            dig.world
                .player_inventory_slot_for_test(MINER, 0)
                .component_patch
                .damage
                .unwrap_or(0),
            wear
        );
        assert_ne!(dig.block(pos), Some(block_state_id("minecraft:air")));
        if wear != 0 {
            assert_eq!(
                republished_held_stack(&result)
                    .unwrap()
                    .component_patch
                    .damage,
                Some(wear)
            );
        }
        if time != 107 {
            continue;
        }
        for x in 1..=3 {
            for z in -2..=0 {
                for y in 63..=67 {
                    let position = BlockPos { x, y, z };
                    let state = block_state_id(if y == 63 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    });
                    if dig.block(position) != Some(state) {
                        dig.set(position, state);
                    }
                }
            }
        }
        let boat_uuid = Uuid::from_u128(6201);
        dig.command("summon minecraft:oak_chest_boat 2.5 64 -0.5 {UUID:[I;0,0,0,6201],NoGravity:1b,LootTable:'test:hit_vehicle',LootTableSeed:37L}");
        let boat = dig
            .world
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, boat_uuid)
            .unwrap();
        assert_eq!(
            dig.world.external_player_vehicle_state_for_test(MINER).2,
            None
        );
        let tool = dig.enchanted(item("minecraft:diamond_pickaxe"), "minecraft:efficiency");
        dig.hold(tool.clone());
        let unmounted = dig.dig(pos);
        assert_eq!(
            dig.world
                .player_inventory_slot_for_test(MINER, 0)
                .component_patch
                .damage,
            Some(3)
        );
        assert_eq!(
            republished_held_stack(&unmounted)
                .unwrap()
                .component_patch
                .damage,
            Some(3)
        );
        let body = dig
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap();
        assert_eq!(body.uuid(), boat_uuid);
        assert_eq!(
            body.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
        );
        assert!(crate::entity::item_slots::container_has_loot(body));
        for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
            assert!(crate::entity::item_slots::read(body, slot as i32)
                .unwrap()
                .is_some_and(|stack| stack.is_empty()));
        }
        let NbtSummaryValue::Compound(before) = dig
            .world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap()
        else {
            panic!("pending hit vehicle")
        };
        assert!(before.iter().any(|entry| entry.name == "LootTable"
            && entry.value == NbtSummaryValue::String("test:hit_vehicle".into())));
        assert!(
            before
                .iter()
                .any(|entry| entry.name == "LootTableSeed"
                    && entry.value == NbtSummaryValue::Long(37))
        );
        assert!(!before.iter().any(|entry| entry.name == "Items"));
        let mounted = dig.command(&format!("ride miner mount {boat_uuid}"));
        for id in mounted.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } if *connection_id == MINER => Some(update.id),
            _ => None,
        }) {
            play(
                &dig.sender,
                &dig.receiver,
                &mut dig.world,
                1,
                PlayServerbound::AcceptTeleportation { id },
            );
        }
        assert_eq!(
            dig.world.external_player_vehicle_state_for_test(MINER).2,
            Some(boat)
        );
        dig.hold(tool.clone());
        let mounted_hit = dig.dig(pos);
        assert_eq!(
            dig.world.external_player_vehicle_state_for_test(MINER).2,
            Some(boat)
        );
        assert_eq!(
            dig.world
                .player_inventory_slot_for_test(MINER, 0)
                .component_patch
                .damage,
            Some(10)
        );
        assert_eq!(
            republished_held_stack(&mounted_hit)
                .unwrap()
                .component_patch
                .damage,
            Some(10)
        );
        assert_ne!(dig.block(pos), Some(block_state_id("minecraft:air")));
        let body = dig
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap();
        assert_eq!(body.uuid(), boat_uuid);
        assert!(!crate::entity::item_slots::container_has_loot(body));
        let mut counts = BTreeMap::new();
        for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
            if let Some(stack) = crate::entity::item_slots::read(body, slot as i32)
                .unwrap()
                .filter(|stack| !stack.is_empty())
            {
                *counts
                    .entry(stack.item_resource_id().unwrap().to_owned())
                    .or_insert(0) += stack.count();
            }
        }
        assert_eq!(
            counts,
            BTreeMap::from([
                ("minecraft:apple".to_string(), 1),
                ("minecraft:diamond".to_string(), 1),
            ])
        );
        let NbtSummaryValue::Compound(after) = dig
            .world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap()
        else {
            panic!("unpacked hit vehicle")
        };
        assert!(!after
            .iter()
            .any(|entry| entry.name == "LootTable" || entry.name == "LootTableSeed"));
        assert!(after.iter().any(|entry| entry.name == "Items"));
        let dismounted = dig.command("ride miner dismount");
        for id in dismounted.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } if *connection_id == MINER => Some(update.id),
            _ => None,
        }) {
            play(
                &dig.sender,
                &dig.receiver,
                &mut dig.world,
                1,
                PlayServerbound::AcceptTeleportation { id },
            );
        }
        assert_eq!(
            dig.world.external_player_vehicle_state_for_test(MINER).2,
            None
        );
        dig.hold(tool);
        let dismounted_hit = dig.dig(pos);
        assert_eq!(
            dig.world
                .player_inventory_slot_for_test(MINER, 0)
                .component_patch
                .damage,
            Some(3)
        );
        assert_eq!(
            republished_held_stack(&dismounted_hit)
                .unwrap()
                .component_patch
                .damage,
            Some(3)
        );
        assert_ne!(dig.block(pos), Some(block_state_id("minecraft:air")));
    }
}

#[test]
fn hit_block_conditions_read_loaded_biomes_light_and_referenced_pieces() {
    use bbb_server_worldgen::noise::nether_fortress::{
        FortressPiece, FortressPieceKind, FortressStructureStart,
    };
    use bbb_server_worldgen::noise::scattered::HorizontalDirection;
    use bbb_server_worldgen::noise::structure_pieces::JigsawBoundingBox;
    use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
    use bbb_server_worldgen::noise::structures::StructureTerrainAdjustment;

    let config = ServerWorldConfig {
        game_type: GameType::Survival,
        ..datapack_config_with_enchantment_effects(
            "minecraft:efficiency",
            serde_json::json!({
                "minecraft:hit_block":[{"effect":{"type":"minecraft:change_item_damage","amount":3},
                    "requirements":{"condition":"minecraft:location_check","offsetY":1,"predicate":{
                        "biomes":"minecraft:desert","light":{"light":{"min":15}},
                        "structures":"minecraft:fortress"
                    }}}]
            }),
        )
    };
    for (x, biome, lit, wear) in [
        (2, "minecraft:desert", true, 3),
        (2, "minecraft:plains", true, 0),
        (2, "minecraft:desert", false, 0),
        (8, "minecraft:desert", true, 0),
    ] {
        let mut dig = Dig::with_config(config.clone(), true);
        let source_chunk = [-1, 0];
        let source = NoiseStructureChunkData::from_supplied_starts(-1, 0, [], [])
            .with_fortress_starts([FortressStructureStart::from_parts(
                source_chunk,
                0,
                StructureTerrainAdjustment::None,
                [0, 12]
                    .into_iter()
                    .map(|offset| FortressPiece {
                        kind: FortressPieceKind::Start,
                        generation_depth: 0,
                        bounding_box: JigsawBoundingBox {
                            min: BlockPos {
                                x: offset,
                                y: 60,
                                z: 0,
                            },
                            max: BlockPos {
                                x: offset + 3,
                                y: 100,
                                z: 15,
                            },
                        },
                        orientation: HorizontalDirection::South,
                        random_seed: 0,
                        chest_pending: false,
                        spawner_placed: false,
                    })
                    .collect(),
            )]);
        let empty = NoiseStructureChunkData::from_supplied_starts(0, 0, [], []);
        let target =
            NoiseStructureChunkData::references_from_complete_source_window(&empty, |x, z| {
                Ok::<_, &str>(if [x, z] == source_chunk {
                    source.clone()
                } else {
                    NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
                })
            })
            .unwrap()
            .0;
        let biome_id = dig
            .world
            .synchronized_registries()
            .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
            .unwrap();
        let mut chunks =
            crate::chunk::ChunkStore::new(default_overworld_level_source().unwrap(), biome_id);
        let startup = config.datapacks.as_ref().unwrap();
        let shape = bbb_server_datapack::dimension_type::dimension_type_shape(
            &startup.registries,
            "minecraft:overworld",
        )
        .unwrap();
        chunks
            .configure_ordinary_structure_generation(Arc::clone(startup), 0, shape, false, None)
            .unwrap();
        restore_test_structure_metadata(&mut chunks, source);
        restore_test_structure_metadata(&mut chunks, target);
        dig.world
            .install_chunk_store_for_test(Java26BuiltinDimension::Overworld, chunks);
        assert!(dig.world.set_noise_biome_for_test(
            Java26BuiltinDimension::Overworld,
            (x >> 2, 16, 0),
            biome
        ));
        dig.command("time of minecraft:overworld pause");
        dig.command("time of minecraft:overworld set 18000");
        let pos = BlockPos { x, y: 64, z: 2 };
        let above = BlockPos { y: 65, ..pos };
        dig.set(pos, block_state_id("minecraft:obsidian"));
        if lit {
            dig.set(
                above,
                super::player_overrides::state(
                    "minecraft:light",
                    &[("level", "15"), ("waterlogged", "false")],
                ),
            );
        }
        let tool = dig.enchanted(item("minecraft:diamond_pickaxe"), "minecraft:efficiency");
        dig.hold(tool);
        dig.dig(pos);
        let brightness = dig
            .world
            .entity_level_light_for_test(Java26BuiltinDimension::Overworld, above)
            .0
            .unwrap();
        assert_eq!(brightness == 15, lit, "x={x}, {biome}, light={brightness}");
        assert_eq!(
            dig.world
                .player_inventory_slot_for_test(MINER, 0)
                .component_patch
                .damage
                .unwrap_or(0),
            wear,
            "x={x}, {biome}, lit={lit}"
        );
    }
}
