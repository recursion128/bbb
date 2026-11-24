use super::*;

use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityLifecycleEvent, SpawnerSpawnData};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_PIG_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(value.to_bits())
}

fn permissive_light_rules() -> NbtSummaryValue {
    let full = || NbtSummaryValue::List(vec![NbtSummaryValue::Int(0), NbtSummaryValue::Int(15)]);
    NbtSummaryValue::Compound(vec![
        entry("block_light_limit", full()),
        entry("sky_light_limit", full()),
    ])
}

fn persistent_stack(resource_id: &str) -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(resource_id)),
            entry("count", NbtSummaryValue::Int(1)),
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn restore_saved_entities(
    candidate: &mut Java26WorldCandidate,
    writes: &[bbb_server_persistence::Java26EntityChunkWrite],
) {
    for write in writes {
        let data =
            Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
                .unwrap();
        candidate
            .dimensions
            .entry(write.dimension.clone())
            .or_insert_with(|| Java26LoadedDimension::new(write.dimension.clone()))
            .entity_chunks
            .insert(
                (write.chunk_x, write.chunk_z),
                Java26LoadedEntityChunk {
                    timestamp: 100,
                    data,
                },
            );
    }
}

#[test]
fn spawner_enchantment_providers_mutate_canonical_stacks_and_allow_empty_results() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID as ENCHANTMENTS,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID as STORED,
    };
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let crossbow = persistent_stack("minecraft:crossbow");
    let enchanted = world
        .apply_spawner_enchantment_provider_for_test(
            Java26BuiltinDimension::Overworld,
            "minecraft:pillager_spawn_crossbow",
            &crossbow,
            1.0,
        )
        .expect("the built-in single provider resolves");
    let enchantments = enchanted
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            &world.item_stack_resolver(),
        )
        .unwrap();
    assert_eq!(
        enchantments,
        Some(NbtSummaryValue::Compound(vec![entry(
            "minecraft:piercing",
            NbtSummaryValue::Int(1),
        )]))
    );

    let removed = crossbow
        .clone()
        .with_component_nbt(ENCHANTMENTS, None, &world.item_stack_resolver())
        .unwrap();
    let unchanged = world
        .apply_spawner_enchantment_provider_for_test(
            Java26BuiltinDimension::Overworld,
            "minecraft:pillager_spawn_crossbow",
            &removed,
            1.0,
        )
        .unwrap();
    assert_eq!(unchanged, removed);

    let book = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:enchanted_book")),
            entry("count", NbtSummaryValue::Int(3)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "minecraft:custom_data",
                        NbtSummaryValue::Compound(vec![entry("marker", NbtSummaryValue::Int(17))]),
                    ),
                    entry(
                        "minecraft:stored_enchantments",
                        NbtSummaryValue::Compound(vec![entry(
                            "minecraft:unbreaking",
                            NbtSummaryValue::Int(1),
                        )]),
                    ),
                ]),
            ),
        ]),
        &world.item_stack_resolver(),
    )
    .unwrap();
    let enchanted_book = world
        .apply_spawner_enchantment_provider_for_test(
            Java26BuiltinDimension::Overworld,
            "minecraft:pillager_spawn_crossbow",
            &book,
            1.0,
        )
        .unwrap();
    let expected = book
        .with_component_nbt(
            STORED,
            Some(&NbtSummaryValue::Compound(vec![
                entry("minecraft:piercing", NbtSummaryValue::Int(1)),
                entry("minecraft:unbreaking", NbtSummaryValue::Int(1)),
            ])),
            &world.item_stack_resolver(),
        )
        .unwrap();
    assert_eq!(enchanted_book, expected);

    let stick = persistent_stack("minecraft:stick");
    let unchanged = world
        .apply_spawner_enchantment_provider_for_test(
            Java26BuiltinDimension::Overworld,
            "minecraft:mob_spawn_equipment",
            &stick,
            1.0,
        )
        .expect("an item without enchantability is a successful empty selection");
    assert_eq!(unchanged, stick);
}

#[test]
fn spawner_minecart_preserves_configured_health_gear_and_passengers_on_restart() {
    let table = serde_json::json!({
        "type": "minecraft:equipment",
        "pools": [{
            "rolls": 1,
            "conditions": [{
                "condition": "minecraft:entity_properties",
                "entity": "this",
                "predicate": {
                    "type": "minecraft:pig",
                    "team": "blue",
                    "passenger": {"type": "minecraft:chicken", "team": "red"},
                    "distance": {"absolute": {"max": 0}}
                }
            }, {
                "condition": "minecraft:location_check",
                "offsetX": 1,
                "predicate": {"biomes": "minecraft:plains", "block": {
                    "blocks": "minecraft:chest",
                    "nbt": "{id:'minecraft:chest',x:1,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"
                }}
            }, {
                "condition": "minecraft:value_check",
                "value": {"type": "minecraft:score",
                    "target": {"type": "minecraft:fixed", "name": "#enabled"},
                    "score": "gear"},
                "range": 1
            }, {
                "condition": "minecraft:time_check",
                "clock": "minecraft:the_end",
                "value": 107
            }, {
                "condition": "minecraft:environment_attribute_check",
                "attribute": "minecraft:gameplay/water_evaporates",
                "value": false
            }, {
                "condition": "minecraft:weather_check",
                "raining": false,
                "thundering": false
            }],
            "entries": [{
                "type": "minecraft:item",
                "name": "minecraft:iron_ingot",
                "functions": [
                    {"function": "minecraft:furnace_smelt"},
                    {"function": "minecraft:set_count", "count": 1},
                    {"function": "minecraft:set_components", "components": {
                        "minecraft:custom_data": {"spawner_gear": true}
                    }},
                    {"function": "minecraft:copy_custom_data",
                        "source": {"type": "minecraft:storage", "source": "test:gear"},
                        "ops": [{"source": "marker", "target": "storage_marker", "op": "replace"}]
                    },
                    {"function": "minecraft:enchant_with_levels", "levels": 30,
                        "options": ["minecraft:mending"]
                    },
                    {"function": "minecraft:set_enchantments", "enchantments": {
                        "minecraft:sharpness": 2
                    }},
                    {"function": "minecraft:filtered",
                        "item_filter": {"items": "#test:gear_filter"},
                        "on_pass": {"function": "minecraft:set_custom_data", "tag": "{filtered:1}"},
                        "on_fail": {"function": "minecraft:set_custom_data", "tag": "{filtered:0}"}
                    }
                ]
            }]
        }]
    });
    for (score, clock, weather, marker, equipped) in [
        (0, 107, "clear", 17, false),
        (1, 108, "clear", 17, false),
        (1, 107, "rain", 17, false),
        (1, 107, "clear", 17, true),
        (1, 107, "clear", 29, true),
    ] {
        let root = std::env::temp_dir().join(format!("bbb-spawner-gear-{}", Uuid::new_v4()));
        let pack = root.join("gear");
        for directory in [
            "data/minecraft/loot_table/equipment",
            "data/test/recipe",
            "data/test/tags/item",
        ] {
            std::fs::create_dir_all(pack.join(directory)).unwrap();
        }
        std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"spawner gear","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
        std::fs::write(
            pack.join("data/minecraft/loot_table/equipment/trial_chamber_melee.json"),
            serde_json::to_vec(&table).unwrap(),
        )
        .unwrap();
        std::fs::write(
            pack.join("data/test/tags/item/gear_smelt_input.json"),
            r#"{"values":["minecraft:iron_ingot"]}"#,
        )
        .unwrap();
        let write_gear_sources = |repair_cost, filter_item| {
            std::fs::write(
                pack.join("data/test/recipe/gear_smelt.json"),
                serde_json::to_vec(&serde_json::json!({
                    "type": "minecraft:smelting", "category": "misc",
                    "ingredient": "#test:gear_smelt_input",
                    "result": {"id": "minecraft:iron_sword", "components": {
                        "minecraft:repair_cost": repair_cost
                    }},
                    "experience": 0, "cookingtime": 200
                }))
                .unwrap(),
            )
            .unwrap();
            std::fs::write(
                pack.join("data/test/tags/item/gear_filter.json"),
                serde_json::to_vec(&serde_json::json!({"values": [filter_item]})).unwrap(),
            )
            .unwrap();
        };
        write_gear_sources(1, "minecraft:iron_sword");
        let options = bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        };
        let startup = bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap();
        let selected = startup.selection.data_packs().clone();
        let mut config = ServerWorldConfig {
            datapacks: Some(Arc::new(startup)),
            ..ServerWorldConfig::default()
        };
        let mut world =
            ServerWorld::restore(config.clone(), &restore_candidate(10, None, Vec::new())).unwrap();
        let (sender, receiver) = mailbox();
        let setup = console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "setblock 1 64 0 chest{Items:[{Slot:0b,id:\"minecraft:diamond\",count:3}]}",
        );
        assert_eq!(entities::console_command_answer(&setup).0, Some(1));
        for command in [
            "team add blue".to_owned(),
            "team add red".to_owned(),
            "scoreboard objectives add gear dummy".to_owned(),
            format!("scoreboard players set #enabled gear {score}"),
            "time of minecraft:the_end pause".to_owned(),
            format!("time of minecraft:the_end set {clock}"),
            format!("weather {weather} 100000t"),
            format!("data modify storage test:gear marker set value {marker}"),
        ] {
            let setup = console_command(&sender, &receiver, &mut world, 1, &command);
            let answer = entities::console_command_answer(&setup);
            assert!(answer.0.is_some(), "{command}: {answer:?}");
        }
        let setup = console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fillbiome -8 56 -8 8 72 8 minecraft:plains",
        );
        assert!(entities::console_command_answer(&setup).0.is_some());
        for _ in 0..24 {
            world.tick(Vec::new());
        }
        let weather_query = format!(
            "execute if predicate {{condition:'minecraft:weather_check',raining:{},thundering:false}}",
            weather == "rain"
        );
        let setup = console_command(&sender, &receiver, &mut world, 1, &weather_query);
        assert_eq!(
            entities::console_command_answer(&setup).0,
            Some(1),
            "{weather_query}"
        );
        for repair_cost in 1..=(if equipped && marker == 17 { 2 } else { 1 }) {
            if repair_cost == 2 {
                write_gear_sources(2, "minecraft:iron_ingot");
                let reloaded =
                    bbb_server_datapack::startup::DatapackStartup::reload_with_data_packs(
                        &options,
                        &selected,
                        world.datapacks().unwrap(),
                    )
                    .unwrap();
                let generation = Arc::new(reloaded);
                enqueue(
                    &sender,
                    WorldCommand::Datapacks {
                        startup: ReloadedDatapacks::Built(Arc::clone(&generation)),
                    },
                )
                .unwrap();
                world.tick(receiver.drain());
                config.datapacks = Some(generation);
            }
            let minecart = world.spawn_spawner_minecart_for_test(
                Java26BuiltinDimension::Overworld,
                Vec3d {
                    x: 4.5,
                    y: 64.0,
                    z: 4.5,
                },
            );
            let root_nbt = vec![
                entry("id", NbtSummaryValue::string("minecraft:pig")),
                entry("Health", NbtSummaryValue::Float(8.0_f32.to_bits())),
                entry("Team", NbtSummaryValue::string("blue")),
                entry(
                    "Tags",
                    NbtSummaryValue::List(vec![NbtSummaryValue::string(
                        "spawner_team_root".to_owned(),
                    )]),
                ),
                entry(
                    "Pos",
                    NbtSummaryValue::List(vec![double(0.5), double(64.0), double(0.5)]),
                ),
                entry(
                    "Passengers",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("id", NbtSummaryValue::string("minecraft:chicken")),
                        entry("Team", NbtSummaryValue::string("red")),
                        entry(
                            "Tags",
                            NbtSummaryValue::List(vec![NbtSummaryValue::string(
                                "spawner_team_child".to_owned(),
                            )]),
                        ),
                    ])]),
                ),
            ];
            let data = world
                .spawner_minecart_data_mut_for_test(Java26BuiltinDimension::Overworld, minecart)
                .unwrap();
            data.set_spawn_delay(0);
            data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
            data.set_next_spawn_data(Some(SpawnerSpawnData::new(
                root_nbt,
                Some(permissive_light_rules()),
                Some(NbtSummaryValue::Compound(vec![
                    entry(
                        "loot_table",
                        NbtSummaryValue::string(
                            "minecraft:equipment/trial_chamber_melee".to_owned(),
                        ),
                    ),
                    entry(
                        "slot_drop_chances",
                        NbtSummaryValue::Float(0.5_f32.to_bits()),
                    ),
                ])),
            )));

            let before = world
                .entity_tracking_counts(Java26BuiltinDimension::Overworld)
                .0;
            let events_before = world.game_events(Java26BuiltinDimension::Overworld).1;
            world
                .queue_spawner_minecart_cycle_for_test(Java26BuiltinDimension::Overworld, minecart);
            world.run_pending_spawner_minecart_attempts_for_test(Java26BuiltinDimension::Overworld);

            assert_eq!(
                world
                    .entity_tracking_counts(Java26BuiltinDimension::Overworld)
                    .0,
                before + 2
            );
            let pig = world
                .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
                .into_iter()
                .find(|id| {
                    world
                        .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                        .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIG_ID)
                })
                .expect("pig was published");
            assert_eq!(
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
                8.0
            );
            let passengers = world
                .stored_passengers_for_test(Java26BuiltinDimension::Overworld, pig)
                .to_vec();
            assert_eq!(passengers.len(), 1);
            let chicken = passengers[0];
            assert_eq!(
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, chicken)
                    .unwrap()
                    .entity_type_id(),
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            );
            let pig_uuid = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
                .unwrap()
                .uuid();
            let chicken_uuid = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, chicken)
                .unwrap()
                .uuid();
            let pig_member = pig_uuid.to_string();
            let chicken_member = chicken_uuid.to_string();
            let spawned_game_events = world.game_events(Java26BuiltinDimension::Overworld);
            let spawned_effects =
                world.drain_spawner_effects_for_test(Java26BuiltinDimension::Overworld);
            for (id, tag, team, other) in [
                (pig, "spawner_team_root", "blue", "red"),
                (chicken, "spawner_team_child", "red", "blue"),
            ] {
                let selected = console_command(
                    &sender, &receiver, &mut world, 2,
                    &format!("execute if entity @e[tag={tag},team={team}] unless entity @e[tag={tag},team={other}]"),
                );
                assert_eq!(entities::console_command_answer(&selected).0, Some(1));
                let NbtSummaryValue::Compound(record) = world
                    .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
                    .unwrap()
                else {
                    panic!("the actual spawned entity has canonical data")
                };
                assert!(!record.iter().any(|entry| entry.name == "Team"));
            }
            let entity = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
                .unwrap();
            if !equipped {
                assert!(entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand)
                    .is_none());
                continue;
            }
            let gear = entity
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .unwrap()
                .clone();
            assert_eq!(
                gear.item_id(),
                Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_SWORD_ID)
            );
            assert_eq!(gear.count(), 1);
            assert_eq!(
                gear.component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
                    &world.item_stack_resolver(),
                )
                .unwrap(),
                Some(NbtSummaryValue::Int(repair_cost))
            );
            let expected_custom_data = Some(NbtSummaryValue::Compound(vec![
                entry("spawner_gear", NbtSummaryValue::Byte(1)),
                entry("storage_marker", NbtSummaryValue::Int(marker)),
                entry(
                    "filtered",
                    NbtSummaryValue::Int(i32::from(repair_cost == 1)),
                ),
            ]));
            assert_eq!(
                gear.component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                    &world.item_stack_resolver(),
                )
                .unwrap(),
                expected_custom_data
            );
            assert_eq!(
                gear.component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                    &world.item_stack_resolver(),
                )
                .unwrap(),
                Some(NbtSummaryValue::Compound(vec![
                    entry("minecraft:mending", NbtSummaryValue::Int(1)),
                    entry("minecraft:sharpness", NbtSummaryValue::Int(2)),
                ]))
            );
            let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
                panic!("the spawned pig is living");
            };
            assert_eq!(
                data.mob
                    .as_ref()
                    .unwrap()
                    .drop_chance(EquipmentSlot::MainHand),
                0.5
            );
            let (events, count) = spawned_game_events;
            assert_eq!(count, events_before + 1);
            let event = events.last().unwrap();
            assert_eq!(event.game_event, VanillaGameEvent::ENTITY_PLACE);
            assert_eq!(event.source_entity_id, Some(pig));
            let expected_passengers = passengers.to_vec();
            let (level_events, lifecycle) = spawned_effects;
            assert!(level_events.iter().any(|event| event.event_type == 2004));
            assert!(lifecycle.iter().any(|event| {
                matches!(
                    event,
                    EntityLifecycleEvent::PassengersChanged { vehicle_id, passenger_ids }
                        if *vehicle_id == pig && passenger_ids == &expected_passengers
                )
            }));
            assert!(lifecycle.iter().any(|event| {
                matches!(
                    event,
                    EntityLifecycleEvent::Broadcast {
                        entity_id,
                        packet: ClientboundPlayPacket::EntityEvent(event),
                    } if *entity_id == pig && event.event_id == 20
                )
            }));

            if repair_cost == 1 {
                let (joined, loaded) =
                    join_loaded_ticks(&sender, &receiver, &mut world, 1, "passenger-witness", 1);
                for (team, other, member) in [
                    ("blue", "red", &pig_member),
                    ("red", "blue", &chicken_member),
                ] {
                    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
                        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(packet) }
                        if packet.name == team && packet.players.iter().any(|player| player == member)
                    )));
                    assert!(!joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
                        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(packet) }
                        if packet.name == other && packet.players.iter().any(|player| player == member)
                    )));
                }
            }
            world.tick(Vec::new());
            assert!(world.remove_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                minecart,
                crate::entity::RemovalReason::Discarded,
            ));

            let request = world
                .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
                .unwrap();
            for (team, other, member) in [
                ("blue", "red", &pig_member),
                ("red", "blue", &chicken_member),
            ] {
                assert!(request
                    .scoreboard
                    .teams
                    .iter()
                    .any(|entry| entry.name == team
                        && entry.players.iter().any(|player| player == member)));
                assert!(!request
                    .scoreboard
                    .teams
                    .iter()
                    .any(|entry| entry.name == other
                        && entry.players.iter().any(|player| player == member)));
            }
            let mut candidate = restore_candidate(10, None, Vec::new());
            candidate.scoreboard = request.scoreboard.clone();
            restore_saved_entities(&mut candidate, &request.entity_writes);
            let mut restored = ServerWorld::restore(config.clone(), &candidate).unwrap();
            let restored_pig = restored
                .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
                .into_iter()
                .find(|id| {
                    restored
                        .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                        .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIG_ID)
                })
                .unwrap();
            assert_eq!(
                restored
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_pig)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
                8.0
            );
            let restored_entity = restored
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_pig)
                .unwrap();
            assert_eq!(restored_entity.uuid(), pig_uuid);
            assert_eq!(
                restored
                    .scoreboard()
                    .players_team(&pig_member)
                    .unwrap()
                    .name,
                "blue"
            );
            assert_eq!(
                restored_entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand),
                Some(&gear)
            );
            let crate::entity::EntityData::GeneralLiving(data) = restored_entity.data() else {
                panic!("the restored pig is living");
            };
            assert_eq!(
                data.mob
                    .as_ref()
                    .unwrap()
                    .drop_chance(EquipmentSlot::MainHand),
                0.5
            );
            let restored_passengers = restored
                .stored_passengers_for_test(Java26BuiltinDimension::Overworld, restored_pig);
            assert_eq!(restored_passengers.len(), 1);
            let restored_chicken = restored_passengers[0];
            assert_eq!(
                restored
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_chicken)
                    .unwrap()
                    .entity_type_id(),
                VANILLA_ENTITY_TYPE_CHICKEN_ID
            );
            assert_eq!(
                restored
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_chicken)
                    .unwrap()
                    .uuid(),
                chicken_uuid
            );
            assert_eq!(
                restored
                    .scoreboard()
                    .players_team(&chicken_member)
                    .unwrap()
                    .name,
                "red"
            );
            restored.tick(Vec::new());
            assert!(restored
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_pig)
                .is_some());
            assert!(restored
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, restored_chicken)
                .is_some());
            for entity_id in [chicken, pig] {
                assert!(world.remove_stored_entity_for_test(
                    Java26BuiltinDimension::Overworld,
                    entity_id,
                    crate::entity::RemovalReason::Discarded,
                ));
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn spawner_equipment_reads_a_tamed_wolfs_owner_team_across_dimensions() {
    let config = datapack_config_with_loot_table(
        "minecraft:equipment/trial_chamber_melee",
        &serde_json::json!({
            "type": "minecraft:equipment",
            "pools": [{
                "rolls": 1,
                "conditions": [{
                    "condition": "minecraft:entity_properties", "entity": "this",
                    "predicate": {"type": "minecraft:wolf", "team": "owners"}
                }],
                "entries": [{"type": "minecraft:item", "name": "minecraft:diamond_sword"}]
            }]
        }),
    );
    for owner_name in ["owner", "other"] {
        let mut world =
            ServerWorld::restore(config.clone(), &restore_candidate(10, None, Vec::new())).unwrap();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "owner", 1);
        join_loaded(&sender, &receiver, &mut world, 2, "other", 2);
        let owner_uuid = world
            .entity_reads()
            .players()
            .find(|player| player.name == owner_name)
            .unwrap()
            .id;
        assert_eq!(
            owner_uuid,
            Uuid::from_u128(if owner_name == "owner" { 1 } else { 2 })
        );
        for text in [
            "team add owners",
            "team join owners owner",
            "execute in minecraft:the_nether run forceload add 0 0",
        ] {
            let tick = console_command(&sender, &receiver, &mut world, 1, text);
            assert_eq!(entities::console_command_answer(&tick).0, Some(1));
        }
        for (connection, name, x) in [(1, "owner", 4), (2, "other", 8)] {
            let profile_uuid = world
                .entity_reads()
                .players()
                .find(|player| player.name == name)
                .unwrap()
                .id;
            assert_eq!(profile_uuid, Uuid::from_u128(u128::from(connection)));
            for text in [
                format!("execute in minecraft:the_nether run fill {x} 63 4 {x} 63 4 stone"),
                format!("execute in minecraft:the_nether run fill {x} 64 4 {x} 67 4 air"),
            ] {
                console_command(&sender, &receiver, &mut world, 1, &text);
            }
            for y in 63..=67 {
                assert_eq!(
                    world.block_state(Java26BuiltinDimension::Nether, BlockPos { x, y, z: 4 }),
                    Some(block_state_id(if y == 63 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    }))
                );
            }
            let tick = console_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!(
                    "execute in minecraft:the_nether run tp {name} {} 64 4.5",
                    f64::from(x) + 0.5
                ),
            );
            assert_eq!(entities::console_command_answer(&tick).0, Some(1));
            let id = tick
                .events
                .iter()
                .find_map(|event| match event {
                    WorldEvent::Send {
                        connection_id,
                        packet: ClientboundPlayPacket::PlayerPosition(update),
                    } if *connection_id == ConnectionId(connection) => Some(update.id),
                    _ => None,
                })
                .unwrap();
            play(
                &sender,
                &receiver,
                &mut world,
                connection,
                PlayServerbound::AcceptTeleportation { id },
            );
            assert_eq!(
                world.player_dimension(ConnectionId(connection)),
                Java26BuiltinDimension::Nether
            );
            assert!(world.player_health_for_test(ConnectionId(connection)) > 0.0);
        }
        for text in ["fill 0 63 0 0 63 0 grass_block", "fill 0 64 0 0 67 0 air"] {
            console_command(&sender, &receiver, &mut world, 1, text);
        }
        for y in 63..=67 {
            assert_eq!(
                world.block_state(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x: 0, y, z: 0 }
                ),
                Some(if y == 63 {
                    bbb_protocol::block_state_types::vanilla_block_default_state_id(
                        bbb_protocol::block_types::VANILLA_BLOCK_GRASS_BLOCK_ID,
                    )
                    .unwrap()
                } else {
                    block_state_id("minecraft:air")
                })
            );
        }
        let minecart = world.spawn_spawner_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            Vec3d {
                x: 4.5,
                y: 64.0,
                z: 4.5,
            },
        );
        let owner_nbt = crate::entity::save::uuid_int_array(owner_uuid);
        let data = world
            .spawner_minecart_data_mut_for_test(Java26BuiltinDimension::Overworld, minecart)
            .unwrap();
        data.set_spawn_delay(0);
        data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
        data.set_next_spawn_data(Some(SpawnerSpawnData::new(
            vec![
                entry("id", NbtSummaryValue::string("minecraft:wolf")),
                entry("Owner", owner_nbt.clone()),
                entry("NoAI", NbtSummaryValue::Byte(1)),
                entry(
                    "Pos",
                    NbtSummaryValue::List(vec![double(0.5), double(64.0), double(0.5)]),
                ),
            ],
            Some(permissive_light_rules()),
            Some(NbtSummaryValue::Compound(vec![entry(
                "loot_table",
                NbtSummaryValue::string("minecraft:equipment/trial_chamber_melee"),
            )])),
        )));
        let before = world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0;
        world.queue_spawner_minecart_cycle_for_test(Java26BuiltinDimension::Overworld, minecart);
        world.run_pending_spawner_minecart_attempts_for_test(Java26BuiltinDimension::Overworld);
        assert_eq!(
            world
                .entity_tracking_counts(Java26BuiltinDimension::Overworld)
                .0,
            before + 1
        );
        let wolf = world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| {
                        entity.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID
                    })
            })
            .unwrap();
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, wolf)
            .unwrap();
        let wolf_uuid = entity.uuid();
        assert_eq!(entity.tamable_owner(), Some((true, Some(owner_uuid))));
        let gear = entity
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .cloned();
        if owner_name == "owner" {
            assert_eq!(
                gear.as_ref().unwrap().item_id(),
                Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID)
            );
        } else {
            assert!(gear.is_none());
        }
        let Some(NbtSummaryValue::Compound(record)) =
            world.stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, wolf)
        else {
            panic!("the spawned wolf has canonical save data");
        };
        assert_eq!(
            record
                .iter()
                .find(|field| field.name == "Owner")
                .map(|field| &field.value),
            Some(&owner_nbt)
        );
        if owner_name == "owner" {
            assert_eq!(gear.as_ref().unwrap().count(), 1);
            let request = world
                .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
                .unwrap();
            let mut candidate = restore_candidate(10, None, Vec::new());
            restore_saved_entities(&mut candidate, &request.entity_writes);
            let restored = ServerWorld::restore(config.clone(), &candidate).unwrap();
            let id = restored
                .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, wolf_uuid)
                .unwrap();
            let entity = restored
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap();
            assert_eq!(entity.uuid(), wolf_uuid);
            assert_eq!(entity.tamable_owner(), Some((true, Some(owner_uuid))));
            assert_eq!(
                entity
                    .living()
                    .unwrap()
                    .item_by_slot(EquipmentSlot::MainHand),
                gear.as_ref()
            );
        }
    }
}

#[test]
fn spawner_minecart_uses_live_floor_tags_and_light_for_spawn_placements() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let floor = BlockPos { x: 0, y: 63, z: 0 };
    let grass = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:grass_block")
            .unwrap()
            .id,
    )
    .unwrap();
    world.set_spawner_floor_for_test(Java26BuiltinDimension::Overworld, floor, grass);
    let minecart = world.spawn_spawner_minecart_for_test(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    let data = world
        .spawner_minecart_data_mut_for_test(Java26BuiltinDimension::Overworld, minecart)
        .unwrap();
    data.set_spawn_delay(0);
    data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
    data.set_next_spawn_data(Some(SpawnerSpawnData::new(
        vec![
            entry("id", NbtSummaryValue::string("minecraft:pig")),
            entry(
                "Pos",
                NbtSummaryValue::List(vec![double(0.5), double(64.0), double(0.5)]),
            ),
        ],
        None,
        None,
    )));

    let before = world
        .entity_tracking_counts(Java26BuiltinDimension::Overworld)
        .0;
    world.queue_spawner_minecart_cycle_for_test(Java26BuiltinDimension::Overworld, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(Java26BuiltinDimension::Overworld);

    assert_eq!(
        world
            .entity_tracking_counts(Java26BuiltinDimension::Overworld)
            .0,
        before + 1
    );
    assert!(world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .any(|id| world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIG_ID)));

    let dimension = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 2, y: 64, z: 0 };
    assert!(world.level_light_facts_for_test(dimension, pos).2 >= 13);
    let data = world
        .spawner_minecart_data_mut_for_test(dimension, minecart)
        .unwrap();
    data.set_spawn_delay(0);
    data.set_next_spawn_data(Some(SpawnerSpawnData::new(
        vec![
            entry("id", NbtSummaryValue::string("minecraft:shulker")),
            entry(
                "Pos",
                NbtSummaryValue::List(vec![double(2.5), double(64.0), double(0.5)]),
            ),
        ],
        None,
        None,
    )));
    let before = world.entity_tracking_counts(dimension).0;
    world.queue_spawner_minecart_cycle_for_test(dimension, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(dimension);
    assert_eq!(world.entity_tracking_counts(dimension).0, before + 1);
    assert!(world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .any(|id| {
            world
                .stored_entity_for_test(dimension, id)
                .is_some_and(|entity| {
                    entity.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID
                        && entity.position()
                            == Vec3d {
                                x: 2.5,
                                y: 64.0,
                                z: 0.5,
                            }
                })
        }));
}

#[test]
fn spawner_minecart_iron_golem_requires_floor_support_and_two_clear_head_blocks() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let minecart = world.spawn_spawner_minecart_for_test(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    let data = world
        .spawner_minecart_data_mut_for_test(Java26BuiltinDimension::Overworld, minecart)
        .unwrap();
    data.set_spawn_delay(0);
    data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
    data.set_next_spawn_data(Some(SpawnerSpawnData::new(
        vec![
            entry("id", NbtSummaryValue::string("minecraft:iron_golem")),
            entry(
                "Pos",
                NbtSummaryValue::List(vec![double(0.5), double(64.0), double(0.5)]),
            ),
        ],
        Some(permissive_light_rules()),
        None,
    )));

    world.queue_spawner_minecart_cycle_for_test(Java26BuiltinDimension::Overworld, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(Java26BuiltinDimension::Overworld);
    assert!(world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .all(|id| world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .is_none_or(|entity| entity.entity_type_id() != VANILLA_ENTITY_TYPE_IRON_GOLEM_ID)));

    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id,
    )
    .unwrap();
    world.set_spawner_floor_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 0, y: 63, z: 0 },
        stone,
    );
    world.queue_spawner_minecart_cycle_for_test(Java26BuiltinDimension::Overworld, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(Java26BuiltinDimension::Overworld);
    assert!(world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .any(|id| world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID)));
}

#[test]
fn a_minecart_spawner_respects_the_live_nearby_class_limit() {
    let dimension = Java26BuiltinDimension::Overworld;
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "class-limit-witness", 1);
    world.tick(Vec::new());
    let minecart = world.spawn_spawner_minecart_for_test(
        dimension,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    let data = world
        .spawner_minecart_data_mut_for_test(dimension, minecart)
        .unwrap();
    data.set_spawn_delay(0);
    data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
    data.set_next_spawn_data(Some(SpawnerSpawnData::new(
        vec![
            entry("id", NbtSummaryValue::String("minecraft:pig".into())),
            entry(
                "Pos",
                NbtSummaryValue::List(vec![double(0.5), double(64.0), double(0.5)]),
            ),
        ],
        Some(permissive_light_rules()),
        None,
    )));
    let mut residents = Vec::new();
    for i in 0..6 {
        residents.push(
            world
                .spawn_stored_entity_for_test(
                    dimension,
                    VANILLA_ENTITY_TYPE_PIG_ID,
                    Vec3d {
                        x: 3.5 + f64::from(i % 3),
                        y: 64.0,
                        z: 3.5 + f64::from(i / 3),
                    },
                    crate::entity::EntityData::GeneralLiving(
                        crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap(),
                    ),
                )
                .0,
        );
    }
    let pigs = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(dimension)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(dimension, *id)
                    .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_PIG_ID)
            })
            .count()
    };
    assert_eq!(pigs(&world), 6);
    world.queue_spawner_minecart_cycle_for_test(dimension, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(dimension);
    assert_eq!(pigs(&world), 6);
    assert!(world.remove_stored_entity_for_test(
        dimension,
        residents[0],
        crate::entity::RemovalReason::Discarded
    ));
    assert_eq!(pigs(&world), 5);
    world
        .spawner_minecart_data_mut_for_test(dimension, minecart)
        .unwrap()
        .set_spawn_delay(0);
    world.queue_spawner_minecart_cycle_for_test(dimension, minecart);
    world.run_pending_spawner_minecart_attempts_for_test(dimension);
    assert_eq!(pigs(&world), 6);
}
