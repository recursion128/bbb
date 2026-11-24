use super::*;
use crate::commands::{LocateKind, LocateRequest, LocateSearchResult};
use crate::entity::SpawnerSpawnData;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_datapack::worldgen::{WorldgenElement, PLACED_FEATURE_REGISTRY};

#[test]
fn located_structure_starts_save_restart_and_resume_without_forcing_full() {
    let chunk = mineshaft_chunk();
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let mut startup = (*crate::chunk::noise_generator::overworld_startup(true)).clone();
    startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap()
        .tags
        .insert("bbb:maps".into(), vec!["minecraft:mineshaft".into()]);
    let tables = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    for (id, kind) in [("bbb:map", "chest"), ("bbb:equipment_map", "equipment")] {
        tables.elements.push(bbb_server_datapack::loot::LootElement {
            id: id.into(), source_pack: "test".into(),
            value: bbb_server_datapack::loot::LootElementValue::Table(
                bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
                    "type": kind, "pools": [{"rolls": 1, "entries": [{
                        "type": "item", "name": "map", "functions": [
                            {"function": "set_count", "count": 3},
                            {"function": "set_components", "components": {"custom_data": {"input": 1}}},
                            {"function": "exploration_map", "destination": "bbb:maps", "search_radius": 0}
                        ]
                    }]}]
                }), None).unwrap()
            ),
        });
    }
    tables
        .elements
        .push(bbb_server_datapack::loot::LootElement {
            id: "bbb:predicate_vehicle_map".into(),
            source_pack: "test".into(),
            value: bbb_server_datapack::loot::LootElementValue::Table(
                bbb_server_datapack::loot::parse_loot_table(
                    &serde_json::json!({
                        "type":"minecraft:chest","pools":[{"rolls":1,"entries":[{
                            "type":"minecraft:item","name":"minecraft:map","functions":[{
                                "function":"minecraft:exploration_map","destination":"bbb:maps",
                                "search_radius":0,"skip_existing_chunks":false
                            }]
                        }]}]
                    }),
                    None,
                )
                .unwrap(),
            ),
        });
    tables.elements.sort_by(|a, b| a.id.cmp(&b.id));
    startup
        .advancements
        .advancements
        .push(bbb_server_datapack::advancement::AdvancementElement {
            id: "bbb:predicate_vehicle_map".into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(&serde_json::json!({
                "criteria":{"cargo":{"trigger":"minecraft:tick","conditions":{"player":{
                    "vehicle":{"type":"minecraft:oak_chest_boat","slots":{
                        "container.*":{"items":"minecraft:filled_map","count":1}
                    }}
                }}}}
            }))
            .unwrap(),
        });
    startup
        .advancements
        .advancements
        .sort_by(|a, b| a.id.cmp(&b.id));
    let predicates = &mut startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:predicate")
        .unwrap()
        .elements;
    for (id, item) in [
        ("bbb:riding_filled_map", "minecraft:filled_map"),
        ("bbb:riding_blank_map", "minecraft:map"),
    ] {
        predicates.push(bbb_server_datapack::loot::LootElement {
            id: id.into(),
            source_pack: "test".into(),
            value: bbb_server_datapack::loot::LootElementValue::Predicate(Box::new(
                bbb_server_datapack::loot::parse_loot_condition(
                    &serde_json::json!({
                        "condition":"minecraft:entity_properties","entity":"this",
                        "predicate":{"vehicle":{"type":"minecraft:oak_chest_boat",
                            "slots":{"container.*":{"items":item,"count":1}}
                        }}
                    }),
                    None,
                )
                .unwrap(),
            )),
        });
    }
    predicates.sort_by(|left, right| left.id.cmp(&right.id));
    for (enchantment, effects) in [
        (
            "minecraft:soul_speed",
            serde_json::json!({"minecraft:tick":[{
                "effect":{"type":"minecraft:change_item_damage","amount":1},
                "requirements":{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":"minecraft:player","vehicle":{
                        "type":"minecraft:oak_chest_boat","slots":{
                            "container.*":{"items":"minecraft:filled_map","count":1}
                        }
                    }}}
            }]}),
        ),
        (
            "minecraft:efficiency",
            serde_json::json!({"minecraft:hit_block":[{
                "effect":{"type":"minecraft:change_item_damage","amount":7},
                "requirements":{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":"minecraft:player","vehicle":{
                        "type":"minecraft:oak_chest_boat","slots":{
                            "container.*":{"items":"minecraft:filled_map","count":1}
                        }
                    }}}
            }]}),
        ),
        (
            "minecraft:protection",
            serde_json::json!({"minecraft:damage_protection":[{
                "effect":{"type":"minecraft:add","value":20},
                "requirements":{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"type":"minecraft:player","vehicle":{
                        "type":"minecraft:oak_chest_boat","slots":{
                            "container.*":{"items":"minecraft:filled_map","count":1}
                        }
                    }}}
            }]}),
        ),
    ] {
        let enchanted =
            crate::tests::datapack_config_with_enchantment_effects(enchantment, effects);
        *startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:enchantment")
            .unwrap()
            .elements
            .iter_mut()
            .find(|element| element.id == enchantment)
            .unwrap() = enchanted
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", enchantment)
            .unwrap()
            .clone();
    }
    let config = ServerWorldConfig {
        datapacks: Some(std::sync::Arc::new(startup)),
        ..creative_fixture_config()
    };
    let restore =
        |candidate: &Java26WorldCandidate| ServerWorld::restore(config.clone(), candidate).unwrap();
    let root =
        std::env::temp_dir().join(format!("bbb-locate-starts-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let created = Java26CreatedSave {
        level: crate::world_creation::created_level_document(&Java26WorldCreation {
            level_name: "located structure restart".into(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().into(),
            spawn_position: [0, 64, 0],
            last_played: 1_786_000_000_000,
        })
        .unwrap(),
        world_gen_settings: noise_candidate().world_gen_settings.document,
        chunks: Vec::new(),
    };
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let baseline = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    let mut world = restore(&baseline);
    let locate = LocateRequest {
        kind: LocateKind::Structure,
        level: dimension.clone(),
        origin: BlockPos {
            x: chunk.0 * 16,
            y: 70,
            z: chunk.1 * 16,
        },
        target_display: "minecraft:mineshaft".into(),
        allowed_ids: vec!["minecraft:mineshaft".into()],
        target_is_tag: false,
    };
    let found = world.locate_live_target(&locate);
    assert_eq!(
        found,
        LocateSearchResult::Found {
            pos: BlockPos {
                x: chunk.0 * 16,
                y: 0,
                z: chunk.1 * 16
            },
            resource_id: "minecraft:mineshaft".into(),
        }
    );
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(chunks.generation_state(chunk.0, chunk.1).is_none());
    let expected_starts = chunks
        .structure_component(chunk.0, chunk.1)
        .unwrap()
        .to_nbt();
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == chunk));
    storage.save_snapshot(&baseline, &request).unwrap();
    world.acknowledge_save_success(&request);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    let saved_chunk = &saved.dimensions[&dimension].chunks[&chunk].data;
    assert_eq!(saved_chunk.status(), Java26ChunkStatus::StructureStarts);
    assert_eq!(saved_chunk.structures(), &expected_starts);
    let mut restarted = restore(&saved);
    assert_eq!(restarted.locate_live_target(&locate), found);
    let chunks = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(chunks.generation_state(chunk.0, chunk.1).is_none());
    assert_eq!(
        chunks
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .to_nbt(),
        expected_starts
    );

    let contents = restarted.seeded_container_fill_for_test(
        Java26BuiltinDimension::Overworld,
        locate.origin,
        "bbb:map",
        1,
    );
    let map = contents.iter().find(|stack| !stack.is_empty()).unwrap();
    let map = map.summary();
    assert_eq!(
        map.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(map.count, 1);
    assert_eq!(map.component_patch.map_id, Some(0));
    assert!(map.component_patch.custom_data.is_none());
    assert_eq!(map.component_patch.map_color, Some(5393476));
    let target = &map.component_patch.map_decorations.as_ref().unwrap()[0];
    assert_eq!(target.id, bbb_protocol::JavaString::from("+"));
    assert_eq!(target.x_bits, f64::from(locate.origin.x).to_bits());
    assert_eq!(target.z_bits, f64::from(locate.origin.z).to_bits());
    let contents = restarted.seeded_container_fill_for_test(
        Java26BuiltinDimension::Overworld,
        locate.origin,
        "bbb:map",
        1,
    );
    let untouched = contents
        .iter()
        .find(|stack| !stack.is_empty())
        .unwrap()
        .summary();
    assert_eq!(
        untouched.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_MAP_ID)
    );
    assert_eq!(contents.iter().map(|stack| stack.count()).sum::<i32>(), 3);
    assert!(untouched.component_patch.custom_data.is_some());
    let reference_save = restarted
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(reference_save.map_last_id, Some(0));
    assert_eq!(reference_save.map_writes.len(), 1);
    assert!(reference_save
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == chunk));
    storage.save_snapshot(&saved, &reference_save).unwrap();
    restarted.acknowledge_save_success(&reference_save);
    assert!(restarted
        .chunk_store_for_test(Java26BuiltinDimension::Overworld)
        .pending_whole_rewrites()
        .next()
        .is_none());
    let saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert_eq!(saved.map_last_id, 0);
    assert_eq!(saved.maps.len(), 1);
    let mut restarted = restore(&saved);
    let chunks = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert_eq!(
        chunks
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts()[0]
            .references(),
        1
    );
    let mut resumed = ChunkStore::new_overworld_carvers_with_structure_generation(
        crate::chunk::noise_generator::overworld_program(true, 0),
        true,
    )
    .unwrap();
    let data = &saved.dimensions[&dimension].chunks[&chunk].data;
    assert!(resumed.hydrate_saved_generation_column(chunk.0, chunk.1, data.status()));
    resumed.restore_saved_generation_metadata(data).unwrap();
    resumed
        .finish_restored_generation_batch(&std::collections::BTreeSet::from([chunk]))
        .unwrap();
    assert!(resumed
        .find_nearest_map_structure(locate.origin, &locate.allowed_ids, 0, true)
        .unwrap()
        .is_none());
    resumed.try_load_column(chunk.0, chunk.1).unwrap();
    assert!(matches!(
        resumed.generation_state(chunk.0, chunk.1),
        Some(bbb_server_types::chunk_data::ChunkGenerationState::Carvers { .. })
    ));
    assert_eq!(
        resumed
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts(),
        chunks
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts(),
    );
    restarted.install_chunk_store_for_test(Java26BuiltinDimension::Overworld, resumed);
    let request = restarted
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == chunk));
    storage.save_snapshot(&saved, &request).unwrap();
    restarted.acknowledge_save_success(&request);
    assert_eq!(
        crate::tests::read_stored_candidate(storage.load_candidate().unwrap()).dimensions
            [&dimension]
            .chunks[&chunk]
            .data
            .status(),
        Java26ChunkStatus::Carvers
    );
    assert_eq!(restarted.locate_live_target(&locate), found);
    storage.close().unwrap();

    let gear_paths = WorldSavePaths::new(root.join("equipment")).unwrap();
    let gear_storage =
        Java26WorldStorage::open(gear_paths.clone(), WorldSaveLimits::default()).unwrap();
    gear_storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let gear_baseline = crate::tests::read_stored_candidate(gear_storage.load_candidate().unwrap());
    let mut gear_world = restore(&gear_baseline);
    let builtin = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let forced = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        1,
        &format!(
            "execute in minecraft:overworld run forceload add {} {}",
            chunk.0 * 16,
            chunk.1 * 16,
        ),
    );
    let Ok(AdminResult::Command(report)) = &admin_outcome(&forced).result else {
        panic!("forceload returns a command report");
    };
    assert_eq!(report.result, Some(1), "{report:?}");
    gear_world.ensure_entity_transfer_full_chunk_for_test(builtin, chunk);
    let (min_y, height) = gear_world.dimension_type_height(builtin).unwrap();
    let y = min_y + height - 8;
    for (operation_id, bottom, top, block) in [
        (2, y - 1, y - 1, "minecraft:grass_block"),
        (3, y, y + 2, "minecraft:air"),
    ] {
        let command = format!(
            "execute in minecraft:overworld run fill {} {bottom} {} {} {top} {} {block}",
            chunk.0 * 16 + 1,
            chunk.1 * 16 + 1,
            chunk.0 * 16 + 6,
            chunk.1 * 16 + 3,
        );
        crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut gear_world,
            operation_id,
            &command,
        );
    }
    let state = |id| {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(id)
                .unwrap()
                .id,
        )
        .unwrap()
    };
    let grass = state("minecraft:grass_block");
    let air = state("minecraft:air");
    for x in 1..=6 {
        for z in 1..=3 {
            for dy in -1..=2 {
                let pos = BlockPos {
                    x: chunk.0 * 16 + x,
                    y: y + dy,
                    z: chunk.1 * 16 + z,
                };
                let block = if dy == -1 { grass } else { air };
                assert_eq!(gear_world.block_state(builtin, pos), Some(block));
            }
        }
    }
    let minecart = gear_world.spawn_spawner_minecart_for_test(
        builtin,
        Vec3d {
            x: f64::from(chunk.0 * 16) + 8.5,
            y: f64::from(y),
            z: f64::from(chunk.1 * 16) + 8.5,
        },
    );
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut spawned = Vec::new();
    for (x, filled) in [(1.5, true), (4.5, false)] {
        let position = Vec3d {
            x: f64::from(chunk.0 * 16) + x,
            y: f64::from(y),
            z: f64::from(chunk.1 * 16) + 1.5,
        };
        let before = gear_world.stored_entity_ids_for_test(builtin);
        let full =
            || NbtSummaryValue::List(vec![NbtSummaryValue::Int(0), NbtSummaryValue::Int(15)]);
        let data = gear_world
            .spawner_minecart_data_mut_for_test(builtin, minecart)
            .unwrap();
        data.set_spawn_delay(0);
        data.set_spawn_parameters(20, 20, 1, 6, 16, 4);
        data.set_next_spawn_data(Some(SpawnerSpawnData::new(
            vec![
                entry("id", NbtSummaryValue::String("minecraft:zombie".into())),
                entry("NoAI", NbtSummaryValue::Byte(1)),
                entry("NoGravity", NbtSummaryValue::Byte(1)),
                entry(
                    "Pos",
                    NbtSummaryValue::List(
                        [position.x, position.y, position.z]
                            .into_iter()
                            .map(|value| NbtSummaryValue::Double(value.to_bits()))
                            .collect(),
                    ),
                ),
            ],
            Some(NbtSummaryValue::Compound(vec![
                entry("block_light_limit", full()),
                entry("sky_light_limit", full()),
            ])),
            Some(NbtSummaryValue::Compound(vec![
                entry(
                    "loot_table",
                    NbtSummaryValue::String("bbb:equipment_map".into()),
                ),
                entry(
                    "slot_drop_chances",
                    NbtSummaryValue::Float(1.0_f32.to_bits()),
                ),
            ])),
        )));
        gear_world.queue_spawner_minecart_cycle_for_test(builtin, minecart);
        gear_world.run_pending_spawner_minecart_attempts_for_test(builtin);
        let entity = gear_world
            .stored_entity_ids_for_test(builtin)
            .into_iter()
            .filter(|id| !before.contains(id))
            .find_map(|id| {
                gear_world
                    .stored_entity_for_test(builtin, id)
                    .filter(|entity| {
                        entity.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID
                    })
            })
            .expect("the actual EQUIPMENT spawn publishes a zombie");
        assert_eq!(entity.position(), position);
        let stack = entity
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .expect("the actual EQUIPMENT table supplies its map")
            .clone();
        let summary = stack.summary();
        if filled {
            assert_eq!(
                summary.item_id,
                Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
            );
            assert_eq!(summary.count, 1);
            assert_eq!(summary.component_patch.map_id, Some(0));
            assert!(summary.component_patch.custom_data.is_none());
            assert_eq!(
                summary.component_patch.map_color,
                map.component_patch.map_color
            );
            let target = &summary.component_patch.map_decorations.as_ref().unwrap()[0];
            assert_eq!(target.id, bbb_protocol::JavaString::from("+"));
            assert_eq!(target.x_bits, f64::from(locate.origin.x).to_bits());
            assert_eq!(target.z_bits, f64::from(locate.origin.z).to_bits());
        } else {
            assert_eq!(
                summary.item_id,
                Some(bbb_protocol::item_types::VANILLA_ITEM_MAP_ID)
            );
            assert_eq!(summary.count, 3);
            assert!(summary.component_patch.map_id.is_none());
            assert_eq!(
                summary.component_patch.custom_data,
                untouched.component_patch.custom_data
            );
        }
        spawned.push((entity.uuid(), position, stack));
    }
    assert_eq!(
        gear_world
            .chunk_store_for_test(builtin)
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts()[0]
            .references(),
        1
    );
    for (operation, command, result) in [
        (899, "gamerule respawn_radius 0".to_owned(), 0),
        (
            900,
            format!(
                "setworldspawn {} {y} {}",
                chunk.0 * 16 + 3,
                chunk.1 * 16 + 2
            ),
            1,
        ),
    ] {
        let outcome = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut gear_world,
            operation,
            &command,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&outcome).0,
            Some(result)
        );
    }
    join_loaded(&sender, &receiver, &mut gear_world, 1, "explorer", 1);
    let summon = crate::tests::entities::console_command(
        &sender, &receiver, &mut gear_world, 901,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['predicate_map'],LootTable:'bbb:predicate_vehicle_map',LootTableSeed:37L}}",
            f64::from(chunk.0 * 16) + 8.5, y + 3, f64::from(chunk.1 * 16) + 8.5),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&summon).0,
        Some(1)
    );
    let boat = gear_world.stored_entity_ids_for_test(builtin).into_iter().find(|id| {
        let NbtSummaryValue::Compound(fields) = gear_world.stored_entity_command_data_for_test(builtin, *id).unwrap() else {
            return false;
        };
        fields.iter().any(|entry| entry.name == "Tags" && matches!(&entry.value,
            NbtSummaryValue::List(tags) if tags.contains(&NbtSummaryValue::String("predicate_map".into()))))
    }).unwrap();
    let boat_uuid = gear_world
        .stored_entity_for_test(builtin, boat)
        .unwrap()
        .uuid();
    let NbtSummaryValue::Compound(pending) = gear_world
        .stored_entity_command_data_for_test(builtin, boat)
        .unwrap()
    else {
        panic!("pending boat saves its actual compound");
    };
    assert!(pending.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("bbb:predicate_vehicle_map".into())));
    assert!(pending
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(pending
        .iter()
        .find(|entry| entry.name == "Items")
        .is_none_or(
            |entry| matches!(&entry.value, NbtSummaryValue::List(items) if items.is_empty())
        ));
    let ride = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        902,
        &format!("ride explorer mount {boat_uuid}"),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&ride).0,
        Some(1)
    );
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    for _ in 0..20 {
        if gear_world.player_advancement_is_done(ConnectionId(1), "bbb:predicate_vehicle_map") {
            break;
        }
        gear_world.tick(Vec::new());
    }
    assert!(gear_world.player_advancement_is_done(ConnectionId(1), "bbb:predicate_vehicle_map"));
    let body = gear_world.stored_entity_for_test(builtin, boat).unwrap();
    assert_eq!(body.uuid(), boat_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let NbtSummaryValue::Compound(generated) = gear_world
        .stored_entity_command_data_for_test(builtin, boat)
        .unwrap()
    else {
        panic!("generated boat saves its actual compound");
    };
    assert!(!generated.iter().any(|entry| entry.name == "LootTable"));
    let cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(cargo.len(), 1);
    let cargo_map = &cargo[0];
    let cargo_summary = cargo_map.summary();
    assert_eq!(
        cargo_summary.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(cargo_summary.count, 1);
    assert_eq!(cargo_summary.component_patch.map_id, Some(1));
    let destination = &cargo_summary
        .component_patch
        .map_decorations
        .as_ref()
        .unwrap()[0];
    assert_eq!(destination.id, bbb_protocol::JavaString::from("+"));
    assert_eq!(destination.x_bits, f64::from(locate.origin.x).to_bits());
    assert_eq!(destination.z_bits, f64::from(locate.origin.z).to_bits());
    let dismount = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        903,
        "ride explorer dismount",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&dismount).0,
        Some(1)
    );
    let objective = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        1100,
        "scoreboard objectives add cargo_probe dummy",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&objective).0,
        Some(1)
    );
    let mut operation = 1100;
    let mut predicate_cargo = Vec::new();
    for (tag, selector_first, conditions) in [
        (
            "command_map",
            false,
            [
                (
                    "as explorer in minecraft:the_nether if predicate bbb:riding_filled_map",
                    1,
                ),
                (
                    "as explorer in minecraft:the_nether if predicate bbb:riding_blank_map",
                    0,
                ),
                (
                    "as explorer in minecraft:the_nether unless predicate bbb:riding_blank_map",
                    1,
                ),
            ],
        ),
        (
            "selector_map",
            true,
            [
                (
                    "if entity @a[name=explorer,predicate=bbb:riding_filled_map]",
                    1,
                ),
                (
                    "if entity @a[name=explorer,predicate=bbb:riding_blank_map]",
                    0,
                ),
                (
                    "if entity @a[name=explorer,predicate=!bbb:riding_blank_map]",
                    1,
                ),
            ],
        ),
    ] {
        operation += 1;
        let summon = crate::tests::entities::console_command(
            &sender, &receiver, &mut gear_world, operation,
            &format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['{tag}'],LootTable:'bbb:predicate_vehicle_map',LootTableSeed:37L}}",
                f64::from(chunk.0 * 16) + 10.5, y + 3, f64::from(chunk.1 * 16) + 8.5),
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&summon).0,
            Some(1)
        );
        let probe = gear_world
            .stored_entity_ids_for_test(builtin)
            .into_iter()
            .find(|id| {
                let NbtSummaryValue::Compound(fields) = gear_world
                    .stored_entity_command_data_for_test(builtin, *id)
                    .unwrap()
                else {
                    return false;
                };
                fields.iter().any(|entry| {
                    entry.name == "Tags"
                        && matches!(&entry.value,
                NbtSummaryValue::List(tags) if tags.contains(&NbtSummaryValue::String(tag.into())))
                })
            })
            .unwrap();
        let probe_uuid = gear_world
            .stored_entity_for_test(builtin, probe)
            .unwrap()
            .uuid();
        let NbtSummaryValue::Compound(pending) = gear_world
            .stored_entity_command_data_for_test(builtin, probe)
            .unwrap()
        else {
            panic!("pending predicate cargo");
        };
        assert!(pending.iter().any(|entry| entry.name == "LootTable"
            && entry.value == NbtSummaryValue::String("bbb:predicate_vehicle_map".into())));
        assert!(
            pending
                .iter()
                .any(|entry| entry.name == "LootTableSeed"
                    && entry.value == NbtSummaryValue::Long(37))
        );
        assert!(pending
            .iter()
            .find(|entry| entry.name == "Items")
            .is_none_or(
                |entry| matches!(&entry.value, NbtSummaryValue::List(items) if items.is_empty())
            ));
        operation += 1;
        let ride = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut gear_world,
            operation,
            &format!("ride explorer mount {probe_uuid}"),
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&ride).0,
            Some(1)
        );
        assert_eq!(
            gear_world
                .external_player_vehicle_state_for_test(ConnectionId(1))
                .2,
            Some(probe)
        );
        assert!(crate::entity::item_slots::container_has_loot(
            gear_world.stored_entity_for_test(builtin, probe).unwrap()
        ));
        if selector_first {
            operation += 1;
            let selected = crate::tests::entities::console_command(
                &sender,
                &receiver,
                &mut gear_world,
                operation,
                "effect give @a[name=explorer,predicate=bbb:riding_filled_map] minecraft:glowing 1",
            );
            assert_eq!(
                crate::tests::entities::console_command_answer(&selected).0,
                Some(1)
            );
        }
        for (condition, expected) in conditions {
            operation += 1;
            let sentinel = crate::tests::entities::console_command(
                &sender,
                &receiver,
                &mut gear_world,
                operation,
                &format!("scoreboard players set #{tag} cargo_probe -7"),
            );
            assert_eq!(
                crate::tests::entities::console_command_answer(&sentinel).0,
                Some(-7)
            );
            assert_eq!(
                gear_world
                    .scoreboard()
                    .score(&format!("#{tag}"), "cargo_probe")
                    .map(|score| score.value),
                Some(-7)
            );
            operation += 1;
            let result = crate::tests::entities::console_command(
                &sender,
                &receiver,
                &mut gear_world,
                operation,
                &format!("execute store result score #{tag} cargo_probe {condition}"),
            );
            if expected == 1 {
                assert_eq!(
                    crate::tests::entities::console_command_answer(&result).0,
                    Some(1)
                );
            }
            assert_eq!(
                gear_world
                    .scoreboard()
                    .score(&format!("#{tag}"), "cargo_probe")
                    .map(|score| score.value),
                Some(expected)
            );
        }
        let body = gear_world.stored_entity_for_test(builtin, probe).unwrap();
        assert_eq!(body.uuid(), probe_uuid);
        assert!(!crate::entity::item_slots::container_has_loot(body));
        let NbtSummaryValue::Compound(generated) = gear_world
            .stored_entity_command_data_for_test(builtin, probe)
            .unwrap()
        else {
            panic!("generated predicate cargo");
        };
        assert!(!generated.iter().any(|entry| entry.name == "LootTable"));
        let cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
            .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
            .filter(|stack| !stack.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(cargo.len(), 1);
        let summary = cargo[0].summary();
        assert_eq!(
            summary.item_id,
            Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
        );
        assert_eq!(summary.count, 1);
        let map_id = summary.component_patch.map_id.unwrap();
        let destination = &summary.component_patch.map_decorations.as_ref().unwrap()[0];
        assert_eq!(destination.id, bbb_protocol::JavaString::from("+"));
        assert_eq!(destination.x_bits, f64::from(locate.origin.x).to_bits());
        assert_eq!(destination.z_bits, f64::from(locate.origin.z).to_bits());
        predicate_cargo.push((probe_uuid, cargo, map_id));
        operation += 1;
        let dismount = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut gear_world,
            operation,
            "ride explorer dismount",
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&dismount).0,
            Some(1)
        );
    }
    for (operation, command) in [
        (904, "gamemode survival explorer"),
        (
            905,
            "item replace entity explorer armor.feet with iron_boots[enchantments={soul_speed:1}]",
        ),
    ] {
        let outcome = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut gear_world,
            operation,
            command,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&outcome).0,
            Some(1)
        );
    }
    let boots = gear_world.player_inventory_slot_for_test(ConnectionId(1), 36);
    assert_eq!(crate::item_durability::damage_and_max(&boots).unwrap().0, 0);
    let summon = crate::tests::entities::console_command(
        &sender, &receiver, &mut gear_world, 906,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['enchantment_map'],LootTable:'bbb:predicate_vehicle_map',LootTableSeed:37L}}",
            f64::from(chunk.0 * 16) + 10.5, y + 3, f64::from(chunk.1 * 16) + 8.5),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&summon).0,
        Some(1)
    );
    let enchanted_boat = gear_world.stored_entity_ids_for_test(builtin).into_iter().find(|id| {
        let NbtSummaryValue::Compound(fields) = gear_world.stored_entity_command_data_for_test(builtin, *id).unwrap() else {
            return false;
        };
        fields.iter().any(|entry| entry.name == "Tags" && matches!(&entry.value,
            NbtSummaryValue::List(tags) if tags.contains(&NbtSummaryValue::String("enchantment_map".into()))))
    }).unwrap();
    let enchanted_uuid = gear_world
        .stored_entity_for_test(builtin, enchanted_boat)
        .unwrap()
        .uuid();
    let NbtSummaryValue::Compound(pending) = gear_world
        .stored_entity_command_data_for_test(builtin, enchanted_boat)
        .unwrap()
    else {
        panic!("pending enchantment cargo");
    };
    assert!(pending.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("bbb:predicate_vehicle_map".into())));
    assert!(pending
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(pending
        .iter()
        .find(|entry| entry.name == "Items")
        .is_none_or(
            |entry| matches!(&entry.value, NbtSummaryValue::List(items) if items.is_empty())
        ));
    assert!(gear_world.player_advancement_is_done(ConnectionId(1), "bbb:predicate_vehicle_map"));
    let ride = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        907,
        &format!("ride explorer mount {enchanted_uuid}"),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&ride).0,
        Some(1)
    );
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(enchanted_boat)
    );
    for _ in 0..20 {
        let boots = gear_world.player_inventory_slot_for_test(ConnectionId(1), 36);
        if crate::item_durability::damage_and_max(&boots).unwrap().0 > 0 {
            break;
        }
        gear_world.tick(Vec::new());
    }
    let boots = gear_world.player_inventory_slot_for_test(ConnectionId(1), 36);
    assert!(crate::item_durability::damage_and_max(&boots).unwrap().0 > 0);
    let body = gear_world
        .stored_entity_for_test(builtin, enchanted_boat)
        .unwrap();
    assert_eq!(body.uuid(), enchanted_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let NbtSummaryValue::Compound(generated) = gear_world
        .stored_entity_command_data_for_test(builtin, enchanted_boat)
        .unwrap()
    else {
        panic!("generated enchantment cargo");
    };
    assert!(!generated.iter().any(|entry| entry.name == "LootTable"));
    let enchanted_cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(enchanted_cargo.len(), 1);
    let enchanted_map = enchanted_cargo[0].summary();
    assert_eq!(
        enchanted_map.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(enchanted_map.count, 1);
    assert_eq!(enchanted_map.component_patch.map_id, Some(4));
    let destination = &enchanted_map
        .component_patch
        .map_decorations
        .as_ref()
        .unwrap()[0];
    assert_eq!(destination.id, bbb_protocol::JavaString::from("+"));
    assert_eq!(destination.x_bits, f64::from(locate.origin.x).to_bits());
    assert_eq!(destination.z_bits, f64::from(locate.origin.z).to_bits());
    let dismount = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        908,
        "ride explorer dismount",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&dismount).0,
        Some(1)
    );
    for id in dismount.events.iter().filter_map(|event| match event {
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if *connection_id == ConnectionId(1) => Some(update.id),
        _ => None,
    }) {
        play(
            &sender,
            &receiver,
            &mut gear_world,
            1,
            PlayServerbound::AcceptTeleportation { id },
        );
    }
    for (operation, command) in [
        (909, "item replace entity explorer armor.feet with air"),
        (910, "item replace entity explorer weapon.mainhand with diamond_pickaxe[enchantments={efficiency:1}]"),
    ] {
        let outcome = crate::tests::entities::console_command(
            &sender, &receiver, &mut gear_world, operation, command,
        );
        assert_eq!(crate::tests::entities::console_command_answer(&outcome).0, Some(1));
    }
    let tool = gear_world.player_inventory_slot_for_test(ConnectionId(1), 0);
    assert_eq!(crate::item_durability::damage_and_max(&tool).unwrap().0, 0);
    let hit_pos = BlockPos {
        x: chunk.0 * 16 + 12,
        y: y + 3,
        z: chunk.1 * 16 + 8,
    };
    let block = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        911,
        &format!(
            "setblock {} {} {} minecraft:stone",
            hit_pos.x, hit_pos.y, hit_pos.z
        ),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&block).0,
        Some(1)
    );
    let summon = crate::tests::entities::console_command(
        &sender, &receiver, &mut gear_world, 912,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['hit_map'],LootTable:'bbb:predicate_vehicle_map',LootTableSeed:37L}}",
            f64::from(chunk.0 * 16) + 10.5, y + 3, f64::from(chunk.1 * 16) + 8.5),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&summon).0,
        Some(1)
    );
    let hit_boat = gear_world.stored_entity_ids_for_test(builtin).into_iter().find(|id| {
        let NbtSummaryValue::Compound(fields) = gear_world.stored_entity_command_data_for_test(builtin, *id).unwrap() else {
            return false;
        };
        fields.iter().any(|entry| entry.name == "Tags" && matches!(&entry.value,
            NbtSummaryValue::List(tags) if tags.contains(&NbtSummaryValue::String("hit_map".into()))))
    }).unwrap();
    let hit_uuid = gear_world
        .stored_entity_for_test(builtin, hit_boat)
        .unwrap()
        .uuid();
    let NbtSummaryValue::Compound(pending) = gear_world
        .stored_entity_command_data_for_test(builtin, hit_boat)
        .unwrap()
    else {
        panic!("pending hit cargo");
    };
    assert!(pending.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("bbb:predicate_vehicle_map".into())));
    assert!(pending
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(pending
        .iter()
        .find(|entry| entry.name == "Items")
        .is_none_or(
            |entry| matches!(&entry.value, NbtSummaryValue::List(items) if items.is_empty())
        ));
    let mounted = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        913,
        &format!("ride explorer mount {hit_uuid}"),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&mounted).0,
        Some(1)
    );
    for id in mounted.events.iter().filter_map(|event| match event {
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if *connection_id == ConnectionId(1) => Some(update.id),
        _ => None,
    }) {
        play(
            &sender,
            &receiver,
            &mut gear_world,
            1,
            PlayServerbound::AcceptTeleportation { id },
        );
    }
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(hit_boat)
    );
    assert!(crate::entity::item_slots::container_has_loot(
        gear_world
            .stored_entity_for_test(builtin, hit_boat)
            .unwrap()
    ));
    play(
        &sender,
        &receiver,
        &mut gear_world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: hit_pos,
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    let tool = gear_world.player_inventory_slot_for_test(ConnectionId(1), 0);
    assert_eq!(crate::item_durability::damage_and_max(&tool).unwrap().0, 7);
    assert_eq!(
        gear_world.block_state(builtin, hit_pos),
        Some(state("minecraft:stone"))
    );
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(hit_boat)
    );
    let body = gear_world
        .stored_entity_for_test(builtin, hit_boat)
        .unwrap();
    assert_eq!(body.uuid(), hit_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let NbtSummaryValue::Compound(generated) = gear_world
        .stored_entity_command_data_for_test(builtin, hit_boat)
        .unwrap()
    else {
        panic!("generated hit cargo");
    };
    assert!(!generated.iter().any(|entry| entry.name == "LootTable"));
    let hit_cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(hit_cargo.len(), 1);
    let hit_map = hit_cargo[0].summary();
    assert_eq!(
        hit_map.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(hit_map.count, 1);
    assert_eq!(hit_map.component_patch.map_id, Some(5));
    let destination = &hit_map.component_patch.map_decorations.as_ref().unwrap()[0];
    assert_eq!(destination.id, bbb_protocol::JavaString::from("+"));
    assert_eq!(destination.x_bits, f64::from(locate.origin.x).to_bits());
    assert_eq!(destination.z_bits, f64::from(locate.origin.z).to_bits());
    let dismount = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        914,
        "ride explorer dismount",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&dismount).0,
        Some(1)
    );
    for id in dismount.events.iter().filter_map(|event| match event {
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if *connection_id == ConnectionId(1) => Some(update.id),
        _ => None,
    }) {
        play(
            &sender,
            &receiver,
            &mut gear_world,
            1,
            PlayServerbound::AcceptTeleportation { id },
        );
    }
    let equipped = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        915,
        "item replace entity explorer armor.feet with iron_boots[enchantments={protection:1}]",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&equipped).0,
        Some(1)
    );
    let summon = crate::tests::entities::console_command(
        &sender, &receiver, &mut gear_world, 916,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['damage_map'],LootTable:'bbb:predicate_vehicle_map',LootTableSeed:37L}}",
            f64::from(chunk.0 * 16) + 10.5, y + 3, f64::from(chunk.1 * 16) + 8.5),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&summon).0,
        Some(1)
    );
    let damage_boat = gear_world.stored_entity_ids_for_test(builtin).into_iter().find(|id| {
        let NbtSummaryValue::Compound(fields) = gear_world.stored_entity_command_data_for_test(builtin, *id).unwrap() else {
            return false;
        };
        fields.iter().any(|entry| entry.name == "Tags" && matches!(&entry.value,
            NbtSummaryValue::List(tags) if tags.contains(&NbtSummaryValue::String("damage_map".into()))))
    }).unwrap();
    let damage_uuid = gear_world
        .stored_entity_for_test(builtin, damage_boat)
        .unwrap()
        .uuid();
    let NbtSummaryValue::Compound(pending) = gear_world
        .stored_entity_command_data_for_test(builtin, damage_boat)
        .unwrap()
    else {
        panic!("pending damage cargo");
    };
    assert!(pending.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("bbb:predicate_vehicle_map".into())));
    assert!(pending
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(pending
        .iter()
        .find(|entry| entry.name == "Items")
        .is_none_or(
            |entry| matches!(&entry.value, NbtSummaryValue::List(items) if items.is_empty())
        ));
    let mounted = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        917,
        &format!("ride explorer mount {damage_uuid}"),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&mounted).0,
        Some(1)
    );
    for id in mounted.events.iter().filter_map(|event| match event {
        WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if *connection_id == ConnectionId(1) => Some(update.id),
        _ => None,
    }) {
        play(
            &sender,
            &receiver,
            &mut gear_world,
            1,
            PlayServerbound::AcceptTeleportation { id },
        );
    }
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(damage_boat)
    );
    assert!(crate::entity::item_slots::container_has_loot(
        gear_world
            .stored_entity_for_test(builtin, damage_boat)
            .unwrap()
    ));
    let health = gear_world.player_health_for_test(ConnectionId(1));
    assert!(health > 10.0);
    let hurt = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        918,
        "damage explorer 10 minecraft:cactus",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&hurt).0,
        Some(1)
    );
    let taken = health - gear_world.player_health_for_test(ConnectionId(1));
    assert!((taken - 1.968).abs() < 0.0001, "damage taken: {taken}");
    assert_eq!(
        gear_world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(damage_boat)
    );
    let body = gear_world
        .stored_entity_for_test(builtin, damage_boat)
        .unwrap();
    assert_eq!(body.uuid(), damage_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    let NbtSummaryValue::Compound(generated) = gear_world
        .stored_entity_command_data_for_test(builtin, damage_boat)
        .unwrap()
    else {
        panic!("generated damage cargo");
    };
    assert!(!generated.iter().any(|entry| entry.name == "LootTable"));
    let damage_cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(damage_cargo.len(), 1);
    let damage_map = damage_cargo[0].summary();
    assert_eq!(
        damage_map.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(damage_map.count, 1);
    assert_eq!(damage_map.component_patch.map_id, Some(6));
    let destination = &damage_map.component_patch.map_decorations.as_ref().unwrap()[0];
    assert_eq!(destination.id, bbb_protocol::JavaString::from("+"));
    assert_eq!(destination.x_bits, f64::from(locate.origin.x).to_bits());
    assert_eq!(destination.z_bits, f64::from(locate.origin.z).to_bits());
    let dismount = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut gear_world,
        919,
        "ride explorer dismount",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&dismount).0,
        Some(1)
    );
    let gear_request = gear_world
        .save_request(1_786_000_000_003, 103, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(gear_request.map_last_id, Some(6));
    assert_eq!(gear_request.map_writes.len(), 7);
    let gear_map = &gear_request.map_writes[&0];
    assert_eq!(gear_map.dimension, dimension.as_str());
    assert_eq!(gear_map.scale, 2);
    assert_eq!(gear_map, &saved.maps[&0]);
    let cargo_saved_map = &gear_request.map_writes[&1];
    assert_eq!(cargo_saved_map, gear_map);
    let enchanted_saved_map = &gear_request.map_writes[&4];
    assert_eq!(enchanted_saved_map, gear_map);
    let hit_saved_map = &gear_request.map_writes[&5];
    assert_eq!(hit_saved_map, gear_map);
    let damage_saved_map = &gear_request.map_writes[&6];
    assert_eq!(damage_saved_map, gear_map);
    for (_, _, id) in &predicate_cargo {
        assert_eq!(&gear_request.map_writes[id], gear_map);
    }
    assert!(gear_request
        .entity_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == chunk));
    gear_storage
        .save_snapshot(&gear_baseline, &gear_request)
        .unwrap();
    gear_world.acknowledge_save_success(&gear_request);
    gear_storage.close().unwrap();
    let gear_storage = Java26WorldStorage::open(gear_paths, WorldSaveLimits::default()).unwrap();
    let gear_saved = crate::tests::read_stored_candidate(gear_storage.load_candidate().unwrap());
    assert_eq!(gear_saved.map_last_id, 6);
    assert_eq!(gear_saved.maps.len(), 7);
    assert_eq!(&gear_saved.maps[&0], gear_map);
    assert_eq!(&gear_saved.maps[&1], cargo_saved_map);
    assert_eq!(&gear_saved.maps[&4], enchanted_saved_map);
    assert_eq!(&gear_saved.maps[&5], hit_saved_map);
    assert_eq!(&gear_saved.maps[&6], damage_saved_map);
    for (_, _, id) in &predicate_cargo {
        assert_eq!(&gear_saved.maps[id], &gear_request.map_writes[id]);
    }
    let gear_restored = restore(&gear_saved);
    assert_eq!(
        gear_restored
            .chunk_store_for_test(builtin)
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts()[0]
            .references(),
        1
    );
    for (uuid, position, stack) in spawned {
        let entity = gear_restored
            .stored_entity_ids_for_test(builtin)
            .into_iter()
            .find_map(|id| {
                gear_restored
                    .stored_entity_for_test(builtin, id)
                    .filter(|entity| entity.uuid() == uuid)
            })
            .expect("the EQUIPMENT mob survives the actual save and restore");
        assert_eq!(entity.position(), position);
        assert_eq!(
            entity
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand),
            Some(&stack)
        );
    }
    let restored_boat = gear_restored
        .stored_entity_ids_for_test(builtin)
        .into_iter()
        .find_map(|id| {
            gear_restored
                .stored_entity_for_test(builtin, id)
                .filter(|body| body.uuid() == boat_uuid)
        })
        .expect("the actual map cargo boat survives the existing disk cycle");
    assert!(!crate::entity::item_slots::container_has_loot(
        restored_boat
    ));
    let restored_cargo = (0..crate::entity::item_slots::container_size(restored_boat).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(restored_boat, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(restored_cargo, cargo);
    for (uuid, cargo, _) in predicate_cargo {
        let body = gear_restored
            .stored_entity_ids_for_test(builtin)
            .into_iter()
            .find_map(|id| {
                gear_restored
                    .stored_entity_for_test(builtin, id)
                    .filter(|body| body.uuid() == uuid)
            })
            .expect("the predicate map cargo survives the existing disk cycle");
        assert!(!crate::entity::item_slots::container_has_loot(body));
        let restored_cargo = (0..crate::entity::item_slots::container_size(body).unwrap())
            .filter_map(|slot| crate::entity::item_slots::read(body, slot as i32).unwrap())
            .filter(|stack| !stack.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(restored_cargo, cargo);
    }
    let restored_boat = gear_restored
        .stored_entity_ids_for_test(builtin)
        .into_iter()
        .find_map(|id| {
            gear_restored
                .stored_entity_for_test(builtin, id)
                .filter(|body| body.uuid() == enchanted_uuid)
        })
        .expect("the enchantment map cargo survives the existing disk cycle");
    assert!(!crate::entity::item_slots::container_has_loot(
        restored_boat
    ));
    let restored_cargo = (0..crate::entity::item_slots::container_size(restored_boat).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(restored_boat, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(restored_cargo, enchanted_cargo);
    let restored_boat = gear_restored
        .stored_entity_ids_for_test(builtin)
        .into_iter()
        .find_map(|id| {
            gear_restored
                .stored_entity_for_test(builtin, id)
                .filter(|body| body.uuid() == hit_uuid)
        })
        .expect("the hit map cargo survives the existing disk cycle");
    assert!(!crate::entity::item_slots::container_has_loot(
        restored_boat
    ));
    let restored_cargo = (0..crate::entity::item_slots::container_size(restored_boat).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(restored_boat, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(restored_cargo, hit_cargo);
    let restored_boat = gear_restored
        .stored_entity_ids_for_test(builtin)
        .into_iter()
        .find_map(|id| {
            gear_restored
                .stored_entity_for_test(builtin, id)
                .filter(|body| body.uuid() == damage_uuid)
        })
        .expect("the damage map cargo survives the existing disk cycle");
    assert!(!crate::entity::item_slots::container_has_loot(
        restored_boat
    ));
    let restored_cargo = (0..crate::entity::item_slots::container_size(restored_boat).unwrap())
        .filter_map(|slot| crate::entity::item_slots::read(restored_boat, slot as i32).unwrap())
        .filter(|stack| !stack.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(restored_cargo, damage_cargo);
    gear_storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ordinary_structure_starts_restart_and_promote_to_real_terrain() {
    let chunk = mineshaft_chunk();
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    for debug in [false, true] {
        let mut startup =
            Arc::new((*crate::chunk::noise_generator::overworld_startup(true)).clone());
        if !debug {
            let generation = &mut Arc::get_mut(&mut startup).unwrap().registries;
            let mut typed = Vec::new();
            let registry = generation
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:worldgen/placed_feature")
                .unwrap();
            for (id, height, block) in [
                (
                    "minecraft:lake_lava_underground",
                    134,
                    "minecraft:gold_block",
                ),
                ("minecraft:lake_lava_surface", 135, "minecraft:lapis_block"),
                ("minecraft:ore_coal_upper", 136, "minecraft:sea_lantern"),
            ] {
                let feature = registry
                    .elements
                    .iter_mut()
                    .find(|element| element.id == id)
                    .unwrap();
                let source = serde_json::json!({
                    "feature": {"type": "minecraft:fill_layer", "config": {
                        "height": height, "state": {"Name": block},
                    }},
                    "placement": [],
                });
                feature.source_json = serde_json::to_vec(&source).unwrap().into();
                feature.source = Some(
                    bbb_registry_codec::decode_direct_registry_json(
                        &serde_json::to_vec(&(source)).unwrap(),
                    )
                    .unwrap(),
                );
                feature.codec_value = None;
                let json = bbb_registry_codec::parse_registry_json(&feature.source_json).unwrap();
                typed.push((id, WorldgenElement::decode(PLACED_FEATURE_REGISTRY, &json)));
            }
            for (id, element) in typed {
                generation
                    .worldgen
                    .insert(id.to_string(), element.unwrap().unwrap());
            }
        }
        let restore = |candidate: &Java26WorldCandidate| {
            ServerWorld::restore(
                ServerWorldConfig {
                    datapacks: Some(Arc::clone(&startup)),
                    ..creative_fixture_config()
                },
                candidate,
            )
            .unwrap()
        };
        let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
        let mut document = candidate.world_gen_settings.document.clone();
        let data = compound_field_mut(document.entries_mut(), "data");
        data.iter_mut()
            .find(|entry| entry.name == "generate_structures")
            .unwrap()
            .value = NbtSummaryValue::Byte(1);
        let dimensions = compound_field_mut(data, "dimensions");
        let overworld = compound_field_mut(dimensions, "minecraft:overworld");
        if debug {
            overworld
                .iter_mut()
                .find(|entry| entry.name == "generator")
                .unwrap()
                .value = crate::chunk::noise_generator::test_support::compound(vec![(
                "type",
                NbtSummaryValue::String("minecraft:debug".into()),
            )]);
        } else {
            let generator = compound_field_mut(overworld, "generator");
            let settings = compound_field_mut(generator, "settings");
            for name in ["features", "lakes"] {
                settings
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .unwrap()
                    .value = NbtSummaryValue::Byte(1);
            }
            settings
                .iter_mut()
                .find(|entry| entry.name == "structure_overrides")
                .unwrap()
                .value =
                NbtSummaryValue::List(vec![NbtSummaryValue::String("minecraft:mineshafts".into())]);
            let layers = &mut settings
                .iter_mut()
                .find(|entry| entry.name == "layers")
                .unwrap()
                .value;
            let NbtSummaryValue::List(layers) = layers else {
                unreachable!()
            };
            for block in ["minecraft:water", "minecraft:short_grass"] {
                layers.push(crate::chunk::noise_generator::test_support::compound(vec![
                    ("height", NbtSummaryValue::Int(1)),
                    ("block", NbtSummaryValue::String(block.into())),
                ]));
            }
        }
        candidate.world_gen_settings =
            bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
                document, 0,
            )
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "bbb-ordinary-starts-restart-{}-{debug}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).unwrap();
        let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
        storage
            .create_world(
                &Java26CreatedSave {
                    level: candidate.level.raw().clone(),
                    world_gen_settings: candidate.world_gen_settings.document,
                    chunks: Vec::new(),
                },
                RegionCompression::Zlib,
                100,
            )
            .unwrap();
        let baseline = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
        let mut world = restore(&baseline);
        let locate = LocateRequest {
            kind: LocateKind::Structure,
            level: dimension.clone(),
            origin: BlockPos {
                x: chunk.0 * 16,
                y: 70,
                z: chunk.1 * 16,
            },
            target_display: "minecraft:mineshaft".into(),
            allowed_ids: vec!["minecraft:mineshaft".into()],
            target_is_tag: false,
        };
        let found = world.locate_live_target(&locate);
        assert_eq!(
            found,
            LocateSearchResult::Found {
                pos: BlockPos {
                    x: chunk.0 * 16,
                    y: 0,
                    z: chunk.1 * 16
                },
                resource_id: "minecraft:mineshaft".into(),
            }
        );
        let starts = world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts()
            .to_vec();
        let request = world
            .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&baseline, &request).unwrap();
        world.acknowledge_save_success(&request);
        storage.close().unwrap();

        let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
        let saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
        assert_eq!(
            saved.dimensions[&dimension].chunks[&chunk].data.status(),
            Java26ChunkStatus::StructureStarts
        );
        let mut world = restore(&saved);
        assert_eq!(world.locate_live_target(&locate), found);
        assert!(world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .generation_state(chunk.0, chunk.1)
            .is_none());
        world.ensure_entity_transfer_full_chunk_for_test(Java26BuiltinDimension::Overworld, chunk);
        let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
        assert!(matches!(
            chunks.generation_state(chunk.0, chunk.1),
            Some(bbb_server_types::chunk_data::ChunkGenerationState::Full { .. })
        ));
        let placed_starts = chunks
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .mineshaft_starts();
        assert_eq!(placed_starts.len(), starts.len());
        for (placed, original) in placed_starts.iter().zip(&starts) {
            assert_eq!(placed.structure_id(), original.structure_id());
            assert_eq!(placed.start_chunk(), original.start_chunk());
            assert_eq!(placed.bounding_box(), original.bounding_box());
        }
        if debug {
            assert_eq!(placed_starts, starts);
        }
        let (y, block) = if debug {
            (60, "minecraft:barrier")
        } else {
            (-64, "minecraft:stone")
        };
        let actual = chunks
            .get_block_state(chunk.0 * 16, y, chunk.1 * 16)
            .unwrap();
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(actual)
                .unwrap()
                .name,
            block,
        );
        if !debug {
            for (y, block) in [
                (64, "minecraft:water"),
                (65, "minecraft:short_grass"),
                (70, "minecraft:gold_block"),
                (71, "minecraft:lapis_block"),
                (72, "minecraft:sea_lantern"),
            ] {
                let state = chunks
                    .get_block_state(chunk.0 * 16, y, chunk.1 * 16)
                    .unwrap();
                assert_eq!(
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                        .unwrap()
                        .name,
                    block
                );
            }
        }
        let placed_support = (-63..60).find_map(|y| {
            (0..16).find_map(|x| {
                (0..16).find_map(|z| {
                    let pos = BlockPos {
                        x: chunk.0 * 16 + x,
                        y,
                        z: chunk.1 * 16 + z,
                    };
                    let state = chunks.get_block_state(pos.x, pos.y, pos.z)?;
                    (bbb_protocol::block_state_types::vanilla_block_state_identity(state)?.name
                        == "minecraft:oak_planks")
                        .then_some((pos, state))
                })
            })
        });
        assert_eq!(
            placed_support.is_some(),
            !debug,
            "only Flat places the mineshaft"
        );
        let request = world
            .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&saved, &request).unwrap();
        world.acknowledge_save_success(&request);
        let mut full = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
        let expected_structures = full.dimensions[&dimension].chunks[&chunk]
            .data
            .structures()
            .clone();
        assert_eq!(
            full.dimensions[&dimension].chunks[&chunk].data.status(),
            Java26ChunkStatus::Full
        );
        let mut settings = full.world_gen_settings.document.clone();
        compound_field_mut(settings.entries_mut(), "data")
            .iter_mut()
            .find(|entry| entry.name == "generate_structures")
            .unwrap()
            .value = NbtSummaryValue::Byte(0);
        if !debug {
            let data = compound_field_mut(settings.entries_mut(), "data");
            let dimensions = compound_field_mut(data, "dimensions");
            let overworld = compound_field_mut(dimensions, "minecraft:overworld");
            let generator = compound_field_mut(overworld, "generator");
            let flat = compound_field_mut(generator, "settings");
            for name in ["features", "lakes"] {
                flat.iter_mut()
                    .find(|entry| entry.name == name)
                    .unwrap()
                    .value = NbtSummaryValue::Byte(0);
            }
        }
        full.world_gen_settings =
            bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
                settings, 0,
            )
            .unwrap();
        let mut restarted = restore(&full);
        assert_eq!(restarted.locate_live_target(&locate), found);
        assert_eq!(
            restarted
                .chunk_store_for_test(Java26BuiltinDimension::Overworld)
                .structure_component(chunk.0, chunk.1)
                .unwrap()
                .to_nbt(),
            expected_structures
        );
        if let Some((pos, state)) = placed_support {
            assert_eq!(
                restarted
                    .chunk_store_for_test(Java26BuiltinDimension::Overworld)
                    .get_block_state(pos.x, pos.y, pos.z),
                Some(state)
            );
        }
        if !debug {
            let chunks = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
            for (y, block) in [
                (70, "minecraft:gold_block"),
                (71, "minecraft:lapis_block"),
                (72, "minecraft:sea_lantern"),
            ] {
                let state = chunks
                    .get_block_state(chunk.0 * 16, y, chunk.1 * 16)
                    .unwrap();
                assert_eq!(
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                        .unwrap()
                        .name,
                    block
                );
            }
            let next = (chunk.0 + 8, chunk.1);
            restarted.ensure_entity_transfer_full_chunk_for_test(
                Java26BuiltinDimension::Overworld,
                next,
            );
            let fresh = restarted.chunk_store_for_test(Java26BuiltinDimension::Overworld);
            for (y, block) in [
                (65, "minecraft:short_grass"),
                (70, "minecraft:air"),
                (71, "minecraft:air"),
                (72, "minecraft:air"),
            ] {
                let state = fresh.get_block_state(next.0 * 16, y, next.1 * 16).unwrap();
                assert_eq!(
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                        .unwrap()
                        .name,
                    block
                );
            }
        }
        let command = format!(
            "execute positioned {} 70 {} run locate structure minecraft:mineshaft",
            chunk.0 * 16,
            chunk.1 * 16
        );
        let (sender, receiver) = mailbox();
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(901, &command, AdminScope::Command),
                request: AdminRequest::ExecuteCommand { command },
            },
        )
        .unwrap();
        let tick = restarted.tick(receiver.drain());
        let Ok(AdminResult::Command(report)) = &admin_outcome(&tick).result else {
            panic!("locate command report")
        };
        assert_eq!(report.result, Some(0));
        let NbtSummaryValue::Compound(message) = report.messages[0].component() else {
            panic!("locate feedback")
        };
        assert_eq!(
            field(message, "translate"),
            &NbtSummaryValue::String("commands.locate.structure.success".into())
        );
        let NbtSummaryValue::List(args) = field(message, "with") else {
            panic!("locate arguments")
        };
        assert_eq!(
            args[0],
            NbtSummaryValue::String("minecraft:mineshaft".into())
        );
        let NbtSummaryValue::Compound(wrapper) = &args[1] else {
            panic!("coordinates wrapper")
        };
        let NbtSummaryValue::List(with) = field(wrapper, "with") else {
            panic!("coordinates")
        };
        let NbtSummaryValue::Compound(coordinates) = &with[0] else {
            panic!("coordinate component")
        };
        assert_eq!(
            field(coordinates, "with"),
            &NbtSummaryValue::List(vec![
                NbtSummaryValue::Int(chunk.0 * 16),
                NbtSummaryValue::String("~".into()),
                NbtSummaryValue::Int(chunk.1 * 16),
            ])
        );
        storage.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// The seed-0 mineshaft candidate nearest the origin.
fn mineshaft_chunk() -> (i32, i32) {
    let [x, z] = crate::chunk::noise_generator::overworld_program(true, 0)
        .find_structure_candidate("minecraft:mineshafts", 256, |_| true)
        .unwrap();
    (x, z)
}
