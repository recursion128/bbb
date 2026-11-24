use bbb_protocol::packets::BlockPos;

use crate::noise::buried_treasure::BuriedTreasureGenerationStub;
use crate::noise::structure_nbt::{compound, entry, int, string};
use crate::noise::structure_pieces::{JigsawHeightAccess, JigsawRotation};
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use super::*;

struct FlatHeight;

impl JigsawHeightAccess for FlatHeight {
    fn first_free_height(&self, _x: i32, _z: i32, _heightmap: StructureHeightmap) -> Option<i32> {
        Some(64)
    }
}

fn startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

fn village_start(
    startup: &DatapackStartup,
    plan: &NoiseStructurePlan,
    chunk: [i32; 2],
) -> JigsawStructureStart {
    let graph = plan
        .jigsaw_piece_graph(
            startup,
            "minecraft:village_plains",
            0,
            chunk[0],
            chunk[1],
            -64,
            384,
            &FlatHeight,
        )
        .unwrap();
    JigsawStructureStart::from_graph(plan, graph, 0).unwrap()
}

fn buried_start(chunk: [i32; 2]) -> BuriedTreasureStructureStart {
    BuriedTreasureStructureStart::from_generation_stub(
        BuriedTreasureGenerationStub::new(
            chunk,
            BlockPos {
                x: chunk[0] * 16 + 8,
                y: 64,
                z: chunk[1] * 16 + 8,
            },
            BlockPos {
                x: chunk[0] * 16 + 9,
                y: 90,
                z: chunk[1] * 16 + 9,
            },
        ),
        0,
    )
}

fn complete_overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
    let biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let refs = biomes.iter().map(String::as_str).collect::<Vec<_>>();
    NoiseStructurePlan::from_startup(startup, &refs).unwrap()
}

#[test]
fn builtin_jigsaw_start_round_trips_and_hydrates_cleanly() {
    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let start = village_start(&startup, &plan, [0, 0]);
    assert!(!start.pieces().is_empty());
    assert_eq!(start.structure_id(), "minecraft:village_plains");
    assert_eq!(start.references(), 0);

    let natural = JigsawStructureChunkData::from_supplied_starts(0, 0, [start.clone()], [&start]);
    let mut data = natural.clone();
    assert!(data.try_add_reference(start.structure_id()));
    assert!(!data.try_add_reference(start.structure_id()));
    let encoded = data.to_nbt();
    let mut decoded = JigsawStructureChunkData::from_nbt(&encoded, 0, 0, &plan, &startup);
    let stabilized = JigsawStructureChunkData::from_nbt(&decoded.to_nbt(), 0, 0, &plan, &startup);
    assert_eq!(stabilized, decoded);
    assert!(decoded.has_valid_start("minecraft:village_plains"));
    assert!(!decoded.try_add_reference("minecraft:village_plains"));
    assert!(decoded.has_bbb_starts());
    assert_eq!(
        decoded.references()["minecraft:village_plains"],
        [pack_chunk(0, 0)]
    );

    decoded.hydrate_bbb_starts(natural);
    assert!(!decoded.has_bbb_starts());
    let restored = &decoded.starts()[0];
    assert_eq!(restored.structure_id(), start.structure_id());
    assert_eq!(restored.start_chunk(), start.start_chunk());
    assert_eq!(restored.references(), 1);
    assert_eq!(restored.terrain_adjustment(), start.terrain_adjustment());
    assert_eq!(restored.junctions(), start.junctions());
    assert_eq!(restored.pieces(), start.pieces());
}

#[test]
fn reference_window_handles_negative_chunks_and_stops_after_eight() {
    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let start = village_start(&startup, &plan, [-3, -5]);
    let bounds = start.bounding_box();
    let target = [bounds.min.x.div_euclid(16), bounds.min.z.div_euclid(16)];
    assert!((i64::from(target[0]) + 3).abs() <= 8);
    assert!((i64::from(target[1]) + 5).abs() <= 8);

    let referenced =
        JigsawStructureChunkData::from_supplied_starts(target[0], target[1], [], [&start]);
    assert_eq!(
        referenced.references()["minecraft:village_plains"],
        [pack_chunk(-3, -5)]
    );
    assert_eq!(unpack_chunk(pack_chunk(-3, -5)), [-3, -5]);

    let outside = JigsawStructureChunkData::from_supplied_starts(6, -5, [], [&start]);
    assert!(outside.references().is_empty());
}

#[test]
fn complete_reference_window_round_trips_buried_treasure_with_jigsaw_starts() {
    let startup = startup();
    let plan = complete_overworld_plan(&startup);
    let chunk = [-3, 5];
    let jigsaw = village_start(&startup, &plan, chunk);
    let buried = buried_start(chunk);
    let target =
        NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [jigsaw.clone()], [])
            .with_buried_treasure_start(buried.clone());
    let (mut references, _) = NoiseStructureChunkData::references_from_complete_source_window(
        &target,
        |source_x, source_z| {
            Ok::<_, &str>(if [source_x, source_z] == chunk {
                target.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap();

    assert_eq!(
        references.references()[BURIED_TREASURE_ID],
        [pack_chunk(chunk[0], chunk[1])]
    );
    assert_eq!(
        references.references()[jigsaw.structure_id()],
        [pack_chunk(chunk[0], chunk[1])]
    );
    assert!(references.try_add_reference(BURIED_TREASURE_ID));
    assert!(!references.try_add_reference(BURIED_TREASURE_ID));
    let mut decoded = NoiseStructureChunkData::from_nbt(
        &references.to_nbt(),
        chunk[0],
        chunk[1],
        &plan,
        &startup,
    );
    let stabilized =
        NoiseStructureChunkData::from_nbt(&decoded.to_nbt(), chunk[0], chunk[1], &plan, &startup);
    assert_eq!(stabilized, decoded);
    assert!(decoded.has_valid_start(BURIED_TREASURE_ID));
    assert!(!decoded.try_add_reference(BURIED_TREASURE_ID));
    assert_eq!(decoded.references(), references.references());

    decoded.hydrate_bbb_starts(target);
    let restored_buried = decoded.buried_treasure_start().unwrap();
    assert_eq!(restored_buried.references(), 1);
    assert_eq!(restored_buried.structure_id(), buried.structure_id());
    assert_eq!(restored_buried.start_chunk(), buried.start_chunk());
    assert_eq!(restored_buried.piece(), buried.piece());
}

#[test]
fn reference_nbt_reload_preserves_saved_sources() {
    let startup = startup();
    let plan = complete_overworld_plan(&startup);
    let mut generated = NoiseStructureChunkData::from_supplied_starts(0, 0, [], []);
    let source_order = [
        pack_chunk(-8, -8),
        pack_chunk(-8, 0),
        pack_chunk(0, -8),
        pack_chunk(0, 0),
        pack_chunk(8, 8),
    ];
    generated
        .references
        .insert("minecraft:mineshaft".to_string(), source_order.to_vec());
    let restored = NoiseStructureChunkData::from_nbt(&generated.to_nbt(), 0, 0, &plan, &startup);
    assert_eq!(restored.references()["minecraft:mineshaft"], source_order);
}

#[test]
fn stored_references_find_pieces_in_the_source_start() {
    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let source_chunk = [-3, -5];
    let start = village_start(&startup, &plan, source_chunk);
    let position = start
        .pieces()
        .iter()
        .map(|piece| piece.bounding_box.min)
        .find(|position| [position.x.div_euclid(16), position.z.div_euclid(16)] != source_chunk)
        .expect("the village extends beyond its starting chunk");
    let target = [position.x.div_euclid(16), position.z.div_euclid(16)];
    let target_data =
        JigsawStructureChunkData::from_supplied_starts(target[0], target[1], [], [&start]);
    let source_data = JigsawStructureChunkData::from_supplied_starts(
        source_chunk[0],
        source_chunk[1],
        [start.clone()],
        [&start],
    );
    assert!(target_data.referenced_piece_contains(
        position,
        |id| id == "minecraft:village_plains",
        |x, z| ([x, z] == source_chunk).then_some(&source_data)
    ));
}

#[test]
fn random_spread_jigsaw_task_retries_until_the_anchor_biome_matches() {
    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let jigsaw_sets = plan
        .sets()
        .iter()
        .filter(|set| {
            set.structures
                .iter()
                .all(|selection| plan.structures()[&selection.structure].jigsaw.is_some())
        })
        .collect::<Vec<_>>();
    assert_eq!(jigsaw_sets.len(), 3);
    assert!(jigsaw_sets.iter().all(|set| matches!(
        &set.placement,
        crate::noise::structures::NoiseStructurePlacement::RandomSpread { .. }
    )));
    let chunk = plan.first_candidate("minecraft:villages", 0);
    let order = plan
        .random_spread_attempt_order("minecraft:villages", 0, chunk[0], chunk[1])
        .unwrap();
    let plains = order
        .iter()
        .position(|id| id == "minecraft:village_plains")
        .unwrap();
    let mut biome_probes = Vec::new();
    let generated = plan.generate_missing_random_spread_jigsaw_starts(
        &startup,
        0,
        chunk[0],
        chunk[1],
        -64,
        384,
        &[],
        &FlatHeight,
        |position| {
            biome_probes.push(position);
            Some("minecraft:plains".to_string())
        },
    );
    assert_eq!(
        generated
            .iter()
            .map(JigsawStructureStart::structure_id)
            .collect::<Vec<_>>(),
        ["minecraft:village_plains"]
    );
    assert_eq!(biome_probes.len(), plains + 1);
    assert_eq!(generated[0], village_start(&startup, &plan, chunk));

    let mut skipped_biome_probes = 0;
    let skipped = plan.generate_missing_random_spread_jigsaw_starts(
        &startup,
        0,
        chunk[0],
        chunk[1],
        -64,
        384,
        &generated,
        &FlatHeight,
        |_| {
            skipped_biome_probes += 1;
            Some("minecraft:plains".to_string())
        },
    );
    assert!(skipped.is_empty());
    assert_eq!(skipped_biome_probes, 0);
}

#[test]
fn vanilla_jigsaw_start_recomputes_element_box_on_load() {
    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let pos = BlockPos { x: 4, y: 64, z: 6 };
    let element = bbb_server_datapack::structure_pool::StructurePoolElement::Single {
        legacy: false,
        location: "minecraft:village/plains/town_centers/plains_fountain_01".to_string(),
        processors: bbb_server_datapack::structure_pool::ProcessorListHolder::InlineEmpty,
        projection: bbb_server_datapack::structure_pool::StructurePoolProjection::Rigid,
        liquid_settings: None,
    };
    let element_box =
        element_bounding_box(&element, &startup.structures, pos, JigsawRotation::None).unwrap();
    let inflated_piece = compound(vec![
        string("id", "minecraft:jigsaw"),
        entry(
            "BB",
            NbtSummaryValue::IntArray(vec![
                element_box.min.x,
                element_box.min.y,
                element_box.min.z,
                element_box.max.x,
                element_box.max.y + 7,
                element_box.max.z,
            ]),
        ),
        int("O", -1),
        int("GD", 0),
        int("PosX", pos.x),
        int("PosY", pos.y),
        int("PosZ", pos.z),
        int("ground_level_delta", 0),
        entry(
            "pool_element",
            compound(vec![
                string("element_type", "minecraft:single_pool_element"),
                string(
                    "location",
                    "minecraft:village/plains/town_centers/plains_fountain_01",
                ),
                string("projection", "rigid"),
                entry(
                    "processors",
                    compound(vec![entry("processors", NbtSummaryValue::List(Vec::new()))]),
                ),
            ]),
        ),
        string("rotation", "NONE"),
        entry("junctions", NbtSummaryValue::List(Vec::new())),
    ]);
    let vanilla_nbt = compound(vec![
        entry(
            "starts",
            compound(vec![entry(
                "minecraft:village_plains",
                compound(vec![
                    string("id", "minecraft:village_plains"),
                    int("ChunkX", 0),
                    int("ChunkZ", 0),
                    int("references", 0),
                    entry("Children", NbtSummaryValue::List(vec![inflated_piece])),
                ]),
            )]),
        ),
        entry("References", compound(Vec::new())),
    ]);

    let restored = JigsawStructureChunkData::from_nbt(&vanilla_nbt, 0, 0, &plan, &startup);
    assert_eq!(restored.starts[0].pieces[0].bounding_box, element_box);
}

#[test]
fn ancient_city_start_accepts_get_or_create_empty_wall_template() {
    use crate::random::WorldgenRandom;

    use crate::test_support::SeamLevel;

    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:deep_dark"]).unwrap();
    let placement = &plan.set("minecraft:ancient_cities").unwrap().placement;
    let mut candidate = None;
    for grid_x in -8..8 {
        for grid_z in -8..8 {
            let Some((chunk_x, chunk_z)) =
                placement.random_spread_candidate(0, grid_x * 24, grid_z * 24)
            else {
                continue;
            };
            if plan.random_spread_attempt_order("minecraft:ancient_cities", 0, chunk_x, chunk_z)
                == Some(vec!["minecraft:ancient_city".to_string()])
            {
                candidate = Some([chunk_x, chunk_z]);
                break;
            }
        }
        if candidate.is_some() {
            break;
        }
    }
    let [chunk_x, chunk_z] = candidate.expect("ancient_cities candidate");
    let generated = plan.generate_missing_random_spread_jigsaw_starts(
        &startup,
        0,
        chunk_x,
        chunk_z,
        -64,
        384,
        &[],
        &FlatHeight,
        |_| Some("minecraft:deep_dark".to_string()),
    );
    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].structure_id(), "minecraft:ancient_city");
    assert!(!generated[0].pieces().is_empty());

    let mut level = SeamLevel::admitting();
    let placement = generated[0].place_in_chunk(
        &startup,
        &mut level,
        chunk_block_bounds(chunk_x, chunk_z),
        &mut WorldgenRandom::with_seed(0),
    );
    assert!(placement.attempted_pieces > 0);
}

#[test]
fn village_start_places_in_chunk() {
    use crate::random::WorldgenRandom;

    use crate::test_support::SeamLevel;

    let startup = startup();
    let plan = NoiseStructurePlan::from_startup(&startup, &["minecraft:plains"]).unwrap();
    let start = village_start(&startup, &plan, [-3, -5]);
    assert!(start.intersects_chunk(-3, -5));

    let mut level = SeamLevel::admitting();
    let placement = start.place_in_chunk(
        &startup,
        &mut level,
        chunk_block_bounds(-3, -5),
        &mut WorldgenRandom::with_seed(0),
    );
    assert!(placement.attempted_pieces > 0);
}

#[test]
fn official_border_proto_chunk_and_forced_placed_starts_survive_save_reload() {
    use crate::noise::beardifier::{JigsawBeardifier, TerrainAdjustingStart};
    use crate::noise::structure_nbt::{compound, entry, int, string};
    use crate::random::WorldgenRandom;
    use crate::test_support::SeamLevel;

    let startup = startup();
    let plan = complete_overworld_plan(&startup);

    // 1. Official 26.1 border proto-chunk (`Status < features`): serialized `starts`
    // with non-natural piece template, non-zero `ground_level_delta`, and `junctions`.
    let custom_element = bbb_server_datapack::structure_pool::StructurePoolElement::Single {
        legacy: false,
        location: "minecraft:village/plains/town_centers/plains_fountain_01".to_string(),
        processors: bbb_server_datapack::structure_pool::ProcessorListHolder::InlineEmpty,
        projection: bbb_server_datapack::structure_pool::StructurePoolProjection::Rigid,
        liquid_settings: None,
    };
    let custom_junction = crate::noise::structure_pieces::JigsawGraphJunction {
        piece_index: 0,
        source_x: 5,
        source_ground_y: 64,
        source_z: 6,
        delta_y: -4,
        destination_projection:
            bbb_server_datapack::structure_pool::StructurePoolProjection::TerrainMatching,
    };
    let custom_piece = crate::noise::structure_pieces::JigsawPiece {
        element: custom_element.clone(),
        position: BlockPos { x: 2, y: 64, z: 3 },
        ground_level_delta: -4,
        rotation: JigsawRotation::Clockwise90,
        bounding_box: JigsawBoundingBox {
            min: BlockPos { x: 2, y: 64, z: 3 },
            max: BlockPos { x: 8, y: 70, z: 9 },
        },
    };
    let vanilla_proto_nbt = compound(vec![
        entry(
            "starts",
            compound(vec![entry(
                "minecraft:village_plains",
                compound(vec![
                    string("id", "minecraft:village_plains"),
                    int("ChunkX", 0),
                    int("ChunkZ", 0),
                    int("references", 0),
                    entry(
                        "Children",
                        NbtSummaryValue::List(vec![piece_to_nbt(
                            &custom_piece,
                            bbb_server_datapack::structure_pool::StructureLiquidSettings::IgnoreWaterlogging,
                            std::iter::once(&custom_junction),
                        )]),
                    ),
                ]),
            )]),
        ),
        entry(
            "References",
            compound(vec![entry(
                "minecraft:village_plains",
                NbtSummaryValue::LongArray(vec![pack_chunk(0, 0)]),
            )]),
        ),
    ]);

    let mut loaded = NoiseStructureChunkData::from_nbt(&vanilla_proto_nbt, 0, 0, &plan, &startup);
    assert!(!loaded.has_bbb_starts());
    assert!(loaded.is_explicit_start("minecraft:village_plains"));
    assert_eq!(loaded.starts()[0].pieces()[0].ground_level_delta, -4);
    assert_eq!(
        loaded.starts()[0].junctions(),
        std::slice::from_ref(&custom_junction)
    );

    // Hydrating with a different BBB natural start must NOT overwrite the official proto-chunk start.
    let natural_start = village_start(&startup, &plan, [0, 0]);
    let natural_chunk = NoiseStructureChunkData::from_supplied_starts(0, 0, [natural_start], []);
    loaded.hydrate_bbb_starts(natural_chunk);

    // Save through BBB -> reload -> check lossless starts, junctions, ground_level_delta, beardifier & FEATURES.
    let saved = loaded.to_nbt();
    let saved_starts = saved
        .compound()
        .unwrap()
        .field("starts")
        .unwrap()
        .compound()
        .unwrap();
    assert!(saved_starts.field("minecraft:village_plains").is_some());

    let reloaded = NoiseStructureChunkData::from_nbt(&saved, 0, 0, &plan, &startup);
    assert_eq!(reloaded, loaded);
    assert_eq!(reloaded.starts()[0].pieces()[0].ground_level_delta, -4);
    assert_eq!(
        reloaded.starts()[0].junctions(),
        std::slice::from_ref(&custom_junction)
    );

    let terrain_starts = reloaded
        .resolve_referenced_terrain_starts(&plan, |x, z| {
            Ok::<_, &str>(if [x, z] == [0, 0] {
                reloaded.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap();
    assert_eq!(terrain_starts.len(), 1);
    assert!(matches!(
        terrain_starts[0],
        TerrainAdjustingStart::Jigsaw { .. }
    ));
    let beardifier = JigsawBeardifier::from_ordered_terrain_starts(0, 0, &terrain_starts);
    assert!(beardifier.sample(5, 64, 6) != 0.0);

    let mut level = SeamLevel::admitting();
    let placement = reloaded.starts()[0].place_in_chunk(
        &startup,
        &mut level,
        chunk_block_bounds(0, 0),
        &mut WorldgenRandom::with_seed(0),
    );
    assert!(placement.attempted_pieces > 0);

    // 2. `/place structure` forced start marked explicit round-trips losslessly without natural RNG match.
    let placed_start = village_start(&startup, &plan, [7, -9]);
    let forced = NoiseStructureChunkData::from_supplied_starts(
        7,
        -9,
        [placed_start.clone()],
        [&placed_start],
    )
    .with_explicit_starts();
    let forced_reloaded =
        NoiseStructureChunkData::from_nbt(&forced.to_nbt(), 7, -9, &plan, &startup);
    assert!(!forced_reloaded.has_bbb_starts());
    assert!(forced_reloaded.is_explicit_start(placed_start.structure_id()));
    let restored_forced = &forced_reloaded.starts()[0];
    assert_eq!(restored_forced.structure_id(), placed_start.structure_id());
    assert_eq!(restored_forced.start_chunk(), placed_start.start_chunk());
    assert_eq!(
        restored_forced.terrain_adjustment(),
        placed_start.terrain_adjustment()
    );
    assert_eq!(restored_forced.junctions(), placed_start.junctions());
    assert_eq!(restored_forced.pieces().len(), placed_start.pieces().len());
    for (restored_piece, original_piece) in
        restored_forced.pieces().iter().zip(placed_start.pieces())
    {
        assert_eq!(restored_piece.element, original_piece.element);
        assert_eq!(restored_piece.position, original_piece.position);
        assert_eq!(
            restored_piece.ground_level_delta,
            original_piece.ground_level_delta
        );
        assert_eq!(restored_piece.rotation, original_piece.rotation);
    }
    let stabilized =
        NoiseStructureChunkData::from_nbt(&forced_reloaded.to_nbt(), 7, -9, &plan, &startup);
    assert_eq!(stabilized, forced_reloaded);
}
