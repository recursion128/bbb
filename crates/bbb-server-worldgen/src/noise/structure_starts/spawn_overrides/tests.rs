use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use crate::noise::nether_fortress::{FortressPiece, FortressPieceKind, FortressStructureStart};
use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_pieces::{JigsawHeightAccess, JigsawPieceGraph};
use crate::noise::structure_starts::JigsawStructureStart;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap};

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

fn plan(startup: &DatapackStartup) -> NoiseStructurePlan {
    let biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    NoiseStructurePlan::from_startup(startup, &biomes).unwrap()
}

fn bounds(min: [i32; 3], max: [i32; 3]) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: min[0],
            y: min[1],
            z: min[2],
        },
        max: BlockPos {
            x: max[0],
            y: max[1],
            z: max[2],
        },
    }
}

fn jigsaw_start(
    startup: &DatapackStartup,
    plan: &NoiseStructurePlan,
    structure_id: &str,
    start_chunk: [i32; 2],
    piece_boxes: &[JigsawBoundingBox],
) -> JigsawStructureStart {
    let graph = plan
        .jigsaw_piece_graph(
            startup,
            "minecraft:village_plains",
            0,
            start_chunk[0],
            start_chunk[1],
            -64,
            384,
            &FlatHeight,
        )
        .unwrap();
    remap_graph(graph, structure_id, start_chunk, piece_boxes, plan)
}

fn remap_graph(
    mut graph: JigsawPieceGraph,
    structure_id: &str,
    start_chunk: [i32; 2],
    piece_boxes: &[JigsawBoundingBox],
    plan: &NoiseStructurePlan,
) -> JigsawStructureStart {
    let prototype = graph.pieces[0].clone();
    graph.structure_id = structure_id.to_string();
    graph.start_chunk = start_chunk;
    graph.generation_anchor = piece_boxes[0].min;
    graph.pieces = piece_boxes
        .iter()
        .map(|piece_box| {
            let mut piece = prototype.clone();
            piece.position = piece_box.min;
            piece.bounding_box = *piece_box;
            piece
        })
        .collect();
    graph.junctions.clear();
    JigsawStructureStart::from_graph(plan, graph, 0).unwrap()
}

fn local_jigsaw_data(starts: &[JigsawStructureStart]) -> NoiseStructureChunkData {
    NoiseStructureChunkData::from_supplied_starts(0, 0, starts.iter().cloned(), starts.iter())
}

#[test]
fn piece_and_full_queries_distinguish_gaps_and_preserve_empty_overrides() {
    let startup = startup();
    let plan = plan(&startup);
    let separated = [
        bounds([0, 64, 0], [0, 64, 0]),
        bounds([10, 64, 0], [10, 64, 0]),
    ];
    let trial = jigsaw_start(
        &startup,
        &plan,
        "minecraft:trial_chambers",
        [0, 0],
        &separated,
    );
    let trial_data = local_jigsaw_data(&[trial]);
    let gap = BlockPos { x: 5, y: 64, z: 0 };
    assert!(trial_data
        .spawn_override_at(&plan, VanillaEntityMobCategory::Monster, gap, |_, _| None)
        .is_none());
    let piece_match = trial_data
        .spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            separated[0].min,
            |_, _| None,
        )
        .unwrap();
    assert_eq!(piece_match.structure_id, "minecraft:trial_chambers");
    assert!(piece_match.override_.spawns.entries.is_empty());

    let ancient = jigsaw_start(
        &startup,
        &plan,
        "minecraft:ancient_city",
        [0, 0],
        &separated,
    );
    let ancient_data = local_jigsaw_data(&[ancient]);
    let full_match = ancient_data
        .spawn_override_at(&plan, VanillaEntityMobCategory::Monster, gap, |_, _| None)
        .unwrap();
    assert_eq!(full_match.structure_id, "minecraft:ancient_city");
    assert!(full_match.override_.spawns.entries.is_empty());
}

#[test]
fn full_query_includes_the_twelve_block_terrain_adjustment_and_endpoints() {
    let startup = startup();
    let plan = plan(&startup);
    assert_ne!(
        plan.structures()["minecraft:ancient_city"].terrain_adjustment,
        StructureTerrainAdjustment::None
    );
    let ancient = jigsaw_start(
        &startup,
        &plan,
        "minecraft:ancient_city",
        [0, 0],
        &[bounds([0, 64, 0], [0, 64, 0])],
    );
    let data = local_jigsaw_data(&[ancient]);
    let query = |x| {
        data.spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            BlockPos { x, y: 64, z: 0 },
            |_, _| None,
        )
    };
    assert!(query(12).is_some());
    assert!(query(13).is_none());
}

#[test]
fn overlapping_overrides_use_frozen_registry_order() {
    let startup = startup();
    let plan = plan(&startup);
    let shared = [bounds([0, 64, 0], [0, 64, 0])];
    let ancient = jigsaw_start(&startup, &plan, "minecraft:ancient_city", [0, 0], &shared);
    let outpost = jigsaw_start(
        &startup,
        &plan,
        "minecraft:pillager_outpost",
        [0, 0],
        &shared,
    );
    assert!(
        plan.structure_order()
            .iter()
            .position(|id| id == "minecraft:ancient_city")
            .unwrap()
            < plan
                .structure_order()
                .iter()
                .position(|id| id == "minecraft:pillager_outpost")
                .unwrap()
    );
    let data = local_jigsaw_data(&[ancient, outpost]);
    let found = data
        .spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            shared[0].min,
            |_, _| None,
        )
        .unwrap();
    assert_eq!(found.structure_id, "minecraft:ancient_city");
}

#[test]
fn fortress_full_box_membership_is_separate_from_general_piece_selection() {
    let startup = startup();
    let plan = plan(&startup);
    let piece = FortressPiece {
        kind: FortressPieceKind::Start,
        generation_depth: 0,
        bounding_box: bounds([0, 64, 0], [10, 70, 10]),
        orientation: HorizontalDirection::South,
        random_seed: 0,
        chest_pending: false,
        spawner_placed: false,
    };
    let fortress = FortressStructureStart::from_parts(
        [0, 0],
        0,
        plan.structures()[FORTRESS_ID].terrain_adjustment,
        vec![piece],
    );
    let starts = NoiseStructureChunkData::from_supplied_starts(0, 0, [], [])
        .with_fortress_starts([fortress]);
    let (data, _) =
        NoiseStructureChunkData::references_from_complete_source_window(&starts, |x, z| {
            Ok::<_, &str>(if [x, z] == [0, 0] {
                starts.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap();
    assert!(data.fortress_full_box_contains(&plan, BlockPos { x: 5, y: 65, z: 5 }, |_, _| None,));
    assert!(!data.fortress_full_box_contains(&plan, BlockPos { x: 11, y: 65, z: 5 }, |_, _| None,));
    let general = data
        .spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            BlockPos { x: 5, y: 65, z: 5 },
            |_, _| None,
        )
        .unwrap();
    assert_eq!(general.structure_id, FORTRESS_ID);
}

#[test]
fn negative_block_coordinates_select_the_floor_chunk() {
    let startup = startup();
    let plan = plan(&startup);
    let point = bounds([-1, 64, -1], [-1, 64, -1]);
    let start = jigsaw_start(
        &startup,
        &plan,
        "minecraft:pillager_outpost",
        [-1, -1],
        &[point],
    );
    let data = NoiseStructureChunkData::from_supplied_starts(-1, -1, [start.clone()], [&start]);
    assert!(data
        .spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            point.min,
            |_, _| None,
        )
        .is_some());
}

#[test]
fn end_city_respects_custom_spawn_overrides_and_terrain_adjustment() {
    let mut startup = startup();
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end_city")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    json["terrain_adaptation"] = serde_json::json!("beard_thin");
    json["spawn_overrides"] = serde_json::json!({
        "monster": {
            "bounding_box": "full",
            "spawns": [{
                "type": "minecraft:enderman",
                "weight": 5,
                "minCount": 1,
                "maxCount": 2
            }]
        }
    });
    element.source_json = std::sync::Arc::from(serde_json::to_vec(&json).unwrap());
    element.source = Some(
        bbb_registry_codec::decode_loaded_registry_json(
            &element.source_json,
            DatapackLoadLimits::default().max_element_bytes,
        )
        .unwrap(),
    );

    let plan = plan(&startup);
    let definition = &plan.structures()["minecraft:end_city"];
    let piece = crate::noise::end_city::EndCityPiece::new(
        "minecraft:end_city/base_floor",
        BlockPos {
            x: 10,
            y: 64,
            z: 10,
        },
        crate::noise::structure_pieces::JigsawRotation::None,
        true,
        [5, 4, 5],
    );
    let start = crate::noise::end_city::EndCityStructureStart::from_parts(
        "minecraft:end_city",
        [0, 0],
        0,
        vec![piece],
    )
    .with_terrain_adjustment(definition.terrain_adjustment);
    // Piece ends at x=14; 12-block BeardThin inflation must reference chunk [1, 0].
    assert!(start.intersects_chunk(1, 0));

    let source_data =
        NoiseStructureChunkData::from_supplied_starts(0, 0, [], []).with_end_city_starts([start]);
    let (data, _) =
        NoiseStructureChunkData::references_from_complete_source_window(&source_data, |x, z| {
            Ok::<_, &str>(if [x, z] == [0, 0] {
                source_data.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        })
        .unwrap();

    let found = data
        .spawn_override_at(
            &plan,
            VanillaEntityMobCategory::Monster,
            BlockPos {
                x: 12,
                y: 65,
                z: 12,
            },
            |_, _| None,
        )
        .unwrap();
    assert_eq!(found.structure_id, "minecraft:end_city");
    assert_eq!(
        found.override_.spawns.entries[0].data.entity_type,
        "minecraft:enderman"
    );
}
