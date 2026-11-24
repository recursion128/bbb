use super::*;
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::{Java26ChunkContext, Java26ChunkData};
use bbb_server_worldgen::noise::nether_fortress::{
    FortressPiece, FortressPieceKind, FortressStructureStart,
};
use bbb_server_worldgen::noise::scattered::HorizontalDirection;
use bbb_server_worldgen::noise::structure_pieces::JigsawBoundingBox;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use bbb_server_worldgen::noise::structures::StructureTerrainAdjustment;

fn restore_structures(chunks: &mut crate::chunk::ChunkStore, data: NoiseStructureChunkData) {
    let [x, z] = data.chunk();
    chunks.try_load_column(x, z).unwrap();
    let mut document = bbb_server_persistence::chunk_writer::full_chunk_document(
        chunks.column(x, z).unwrap(),
        x,
        z,
        &|_| Some("minecraft:plains".into()),
        chunks.min_y(),
    )
    .unwrap();
    document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "structures")
        .unwrap()
        .value = data.to_nbt();
    let saved = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: x,
            expected_z: z,
            min_section_y: chunks.min_y().div_euclid(16),
            section_count: chunks.section_count(),
        },
    )
    .unwrap();
    chunks.restore_saved_generation_metadata(&saved).unwrap();
}

#[test]
fn location_structures_use_referenced_pieces_and_live_tags_and_reward_once() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let tags = startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap();
    tags.tags
        .insert("test:pieces".into(), vec!["minecraft:fortress".into()]);
    startup.advancements = AdvancementSet {
        advancements: [
            ("test:direct", json!("minecraft:fortress")),
            ("test:list", json!(["minecraft:fortress"])),
            ("test:tag", json!("#test:pieces")),
        ]
        .into_iter()
        .map(|(id, structures)| {
            let location = json!({"structures": structures});
            let player = if id == "test:tag" {
                json!([{"condition": "minecraft:location_check", "predicate": location}])
            } else {
                json!({"location": location})
            };
            AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: parse_advancement(&json!({
                    "criteria": {"located": {
                        "trigger": "minecraft:location",
                        "conditions": {"player": player}
                    }},
                    "rewards": {"experience": 7}
                }))
                .unwrap(),
            }
        })
        .collect(),
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "explorer", 1);
    let player = ConnectionId(1);
    let mut position = world.levels[player].position();
    let chunk = [
        position.x.floor() as i32 >> 4,
        position.z.floor() as i32 >> 4,
    ];
    let source_chunk = [chunk[0] - 1, chunk[1]];
    let min_x = chunk[0] * 16;
    let min_z = chunk[1] * 16;
    let pieces = [0, 12]
        .into_iter()
        .map(|offset| FortressPiece {
            kind: FortressPieceKind::Start,
            generation_depth: 0,
            bounding_box: JigsawBoundingBox {
                min: BlockPos {
                    x: min_x + offset,
                    y: 60,
                    z: min_z,
                },
                max: BlockPos {
                    x: min_x + offset + 3,
                    y: 100,
                    z: min_z + 15,
                },
            },
            orientation: HorizontalDirection::South,
            random_seed: 0,
            chest_pending: false,
            spawner_placed: false,
        })
        .collect();
    let source =
        NoiseStructureChunkData::from_supplied_starts(source_chunk[0], source_chunk[1], [], [])
            .with_fortress_starts([FortressStructureStart::from_parts(
                source_chunk,
                0,
                StructureTerrainAdjustment::None,
                pieces,
            )]);
    let empty_target = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], []);
    let target =
        NoiseStructureChunkData::references_from_complete_source_window(&empty_target, |x, z| {
            Ok::<_, &str>(if [x, z] == source_chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap()
        .0;
    let dimension = world.levels[player].dimension;
    let chunks = &mut world.levels.get_mut(&dimension).unwrap().chunks;
    restore_structures(chunks, source);
    restore_structures(chunks, target);
    let check_command_location = |world: &mut ServerWorld, request_id, expected| {
        for (offset, structures) in (0_u64..).zip([
            "'minecraft:fortress'",
            "['minecraft:fortress']",
            "'#test:pieces'",
        ]) {
            world.set_inventory_slot_for_test(
                player,
                0,
                crate::entity::item::plain_stack("minecraft:dirt", 1)
                    .unwrap()
                    .into_summary(),
            );
            crate::tests::console_command(
                &sender,
                &receiver,
                world,
                request_id + offset,
                &format!("execute as explorer at @s run item modify entity @s hotbar.0 {{function:'minecraft:set_count',count:7,conditions:[{{condition:'minecraft:location_check',predicate:{{structures:{structures}}}}}]}}"),
            );
            assert_eq!(
                world.levels[player].inventory.slot(0).unwrap().count(),
                expected,
                "{structures}"
            );
        }
    };
    position.x = f64::from(min_x + 8) + 0.5;
    position.y = 70.0;
    position.z = f64::from(min_z + 8) + 0.5;
    world.levels[player].set_position(position);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    for id in ["test:direct", "test:list", "test:tag"] {
        assert!(!world.player_advancement_is_done(player, id));
    }
    assert_eq!(world.levels[player].experience.total(), 0);
    check_command_location(&mut world, 2, 1);
    position.x = f64::from(min_x + 1) + 0.5;
    world.levels[player].set_position(position);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    for id in ["test:direct", "test:list", "test:tag"] {
        assert!(world.player_advancement_is_done(player, id));
    }
    assert_eq!(world.levels[player].experience.total(), 21);
    check_command_location(&mut world, 5, 7);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert_eq!(world.levels[player].experience.total(), 21);
}

/// `StructureManager.getStructureWithPieceAt` reads the start chunk back from
/// storage, so a piece still matches after its start column unloaded.
#[test]
fn location_structures_match_a_piece_whose_start_column_unloaded() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    startup.advancements = AdvancementSet {
        advancements: vec![AdvancementElement {
            id: "test:direct".into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "criteria": {"located": {
                    "trigger": "minecraft:location",
                    "conditions": {"player": {"location": {"structures": "minecraft:fortress"}}}
                }}
            }))
            .unwrap(),
        }],
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "explorer", 1);
    let player = ConnectionId(1);
    let mut position = world.levels[player].position();
    let chunk = [
        position.x.floor() as i32 >> 4,
        position.z.floor() as i32 >> 4,
    ];
    let source_chunk = [chunk[0] - 6, chunk[1]];
    let (min_x, min_z) = (chunk[0] * 16, chunk[1] * 16);
    let piece = FortressPiece {
        kind: FortressPieceKind::Start,
        generation_depth: 0,
        bounding_box: JigsawBoundingBox {
            min: BlockPos {
                x: min_x,
                y: 60,
                z: min_z,
            },
            max: BlockPos {
                x: min_x + 15,
                y: 100,
                z: min_z + 15,
            },
        },
        orientation: HorizontalDirection::South,
        random_seed: 0,
        chest_pending: false,
        spawner_placed: false,
    };
    let source =
        NoiseStructureChunkData::from_supplied_starts(source_chunk[0], source_chunk[1], [], [])
            .with_fortress_starts([FortressStructureStart::from_parts(
                source_chunk,
                0,
                StructureTerrainAdjustment::None,
                vec![piece],
            )]);
    let empty_target = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], []);
    let target =
        NoiseStructureChunkData::references_from_complete_source_window(&empty_target, |x, z| {
            Ok::<_, &str>(if [x, z] == source_chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap()
        .0;
    let dimension = world.levels[player].dimension;
    let chunks = &mut world.levels.get_mut(&dimension).unwrap().chunks;
    restore_structures(chunks, source);
    restore_structures(chunks, target);
    assert!(chunks
        .unload_saved_column(source_chunk[0], source_chunk[1])
        .is_some());
    position.x = f64::from(min_x + 8) + 0.5;
    position.y = 70.0;
    position.z = f64::from(min_z + 8) + 0.5;
    world.levels[player].set_position(position);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(world.player_advancement_is_done(player, "test:direct"));
}

/// The start column of a piece may be on disk only, never read this session:
/// its stored record answers, as `StructureManager.fillStartsForStructure`
/// brings the chunk to `STRUCTURE_STARTS` (`StructureManager.java:71-80`).
/// A player's own tickets read every column a piece under them can start in
/// (ticket levels reach 11 chunks past the view distance, references 8), so
/// the piece column here is loaded without tickets and asked directly.
#[test]
fn location_structures_match_a_piece_whose_start_column_was_never_read() {
    let root = std::env::temp_dir().join(format!(
        "bbb-stored-structure-start-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let spawn = ServerWorld::fixture_spawn_block();
    // Far from the spawn chunks, whose tickets would read every column here.
    let chunk = [(spawn[0] >> 4) + 100, spawn[2] >> 4];
    let source_chunk = [chunk[0] - 6, chunk[1]];
    let (min_x, min_z) = (chunk[0] * 16, chunk[1] * 16);
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let level_source = crate::world_creation::default_overworld_level_source().unwrap();
    // Saved FULL neighbours keep the piece column's load from generating them,
    // which would read the start column's whole window.
    let saved = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dz| (chunk[0] + dx, chunk[1] + dz)))
        .chain([(source_chunk[0], source_chunk[1])])
        .map(|(x, z)| {
            let column = level_source
                .generate_chunk(x, z, fixture.biome_id())
                .into_column();
            (x, z, column)
        })
        .collect::<Vec<_>>();
    let created = crate::world_creation::created_save(
        &bbb_server_persistence::Java26WorldCreation {
            level_name: "bbb-stored-structure-start".into(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: "easy".into(),
            spawn_position: spawn,
            last_played: 1,
        },
        &fixture,
        &saved
            .iter()
            .map(|(x, z, column)| (*x, *z, column))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let storage = bbb_server_persistence::Java26WorldStorage::open(
        bbb_server_types::save::WorldSavePaths::new(&root).unwrap(),
        bbb_server_types::save::WorldSaveLimits::default(),
    )
    .unwrap();
    storage
        .create_world(&created, bbb_server_persistence::RegionCompression::Zlib, 1)
        .unwrap();
    let dimension: bbb_server_types::save::Java26LevelKey =
        bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
    let piece = FortressPiece {
        kind: FortressPieceKind::Start,
        generation_depth: 0,
        bounding_box: JigsawBoundingBox {
            min: BlockPos {
                x: min_x,
                y: -64,
                z: min_z,
            },
            max: BlockPos {
                x: min_x + 15,
                y: 100,
                z: min_z + 15,
            },
        },
        orientation: HorizontalDirection::South,
        random_seed: 0,
        chest_pending: false,
        spawner_placed: false,
    };
    let source =
        NoiseStructureChunkData::from_supplied_starts(source_chunk[0], source_chunk[1], [], [])
            .with_fortress_starts([FortressStructureStart::from_parts(
                source_chunk,
                0,
                StructureTerrainAdjustment::None,
                vec![piece],
            )]);
    // The start column's record carries the start; this session never loads it.
    let directory = storage.paths().dimension_dir(&dimension).join("region");
    // Close the creator storage before patching the region directly (`10983c63fd`).
    storage.close().unwrap();
    {
        let region = bbb_server_persistence::RegionFileStore::new(
            directory.join(format!(
                "r.{}.{}.mca",
                source_chunk[0] >> 5,
                source_chunk[1] >> 5
            )),
            &directory,
            bbb_server_types::save::WorldSaveLimits::default(),
        )
        .unwrap();
        let mut document = region
            .read_chunk(source_chunk[0], source_chunk[1])
            .unwrap()
            .unwrap();
        document
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == "structures")
            .unwrap()
            .value = source.to_nbt();
        region
            .write_chunk(
                source_chunk[0],
                source_chunk[1],
                &document,
                bbb_server_persistence::RegionCompression::Zlib,
                1,
            )
            .unwrap();
    }

    let storage = bbb_server_persistence::Java26WorldStorage::open(
        bbb_server_types::save::WorldSavePaths::new(&root).unwrap(),
        bbb_server_types::save::WorldSaveLimits::default(),
    )
    .unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert!(candidate.dimensions[&dimension]
        .stored_chunks
        .contains(&(source_chunk[0], source_chunk[1])));
    let mut world = ServerWorld::restore(crate::tests::datapack_config(), &candidate).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    let empty_target = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], []);
    let target =
        NoiseStructureChunkData::references_from_complete_source_window(&empty_target, |x, z| {
            Ok::<_, &str>(if [x, z] == source_chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap()
        .0;
    let overworld =
        crate::world::LevelId::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    restore_structures(&mut world.level_mut_by_id(&overworld).chunks, target);
    let query = format!(
        "execute positioned {} {} {} if predicate {{condition:'minecraft:location_check',predicate:{{structures:'minecraft:fortress'}}}}",
        min_x + 8,
        spawn[1],
        min_z + 8
    );
    let answer = crate::tests::console_command(&sender, &receiver, &mut world, 1, &query);
    assert_eq!(
        crate::tests::console_command_answer(&answer).0,
        Some(1),
        "{query}"
    );
    assert!(world
        .level_by_id(&overworld)
        .chunks
        .is_stored_unread((source_chunk[0], source_chunk[1])));
    storage.close().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
