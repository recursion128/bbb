use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::Value;

use super::super::biome::NoiseBiomeSource;
use super::*;

fn startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
    overworld_plan_result(startup).unwrap()
}

fn overworld_plan_result(
    startup: &DatapackStartup,
) -> Result<NoiseStructurePlan, NoiseStructuresError> {
    let source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap())
}

fn mutate_registry_source(
    startup: &mut DatapackStartup,
    registry_id: &str,
    element_id: &str,
    mutate: impl FnOnce(&mut Value),
) {
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == registry_id)
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == element_id)
        .unwrap();
    let mut source: Value = serde_json::from_slice(&element.source_json).unwrap();
    mutate(&mut source);
    element.source_json = std::sync::Arc::from(serde_json::to_vec(&source).unwrap());
    element.source = Some(
        bbb_registry_codec::decode_loaded_registry_json(
            &element.source_json,
            DatapackLoadLimits::default().max_element_bytes,
        )
        .unwrap(),
    );
}

#[test]
fn loaded_pools_ignore_unknown_java_strings_without_changing_the_structure_plan() {
    let builtin = startup();
    let expected = overworld_plan(&builtin);
    let root = std::env::temp_dir().join(format!(
        "bbb-structure-pool-source-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let pack = root.join("pool-source");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        br#"{"pack":{"description":"pool source","min_format":101,"max_format":101}}"#,
    )
    .unwrap();
    let registry = builtin
        .registries
        .registry("minecraft:worldgen/template_pool")
        .unwrap();
    for element in &registry.elements {
        let (namespace, id) = element.id.split_once(':').unwrap();
        let path = pack
            .join("data")
            .join(namespace)
            .join("worldgen/template_pool")
            .join(format!("{id}.json"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let source = std::str::from_utf8(&element.source_json)
            .unwrap()
            .replace('{', r#"{"\ud800":"\ud801","note":[null,"\ud800"],"#);
        std::fs::write(path, source).unwrap();
    }
    let options = DatapackStartupOptions {
        datapacks_dir: Some(root.clone()),
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    };
    let loaded = DatapackStartup::load(&options).unwrap();
    let actual = overworld_plan(&loaded);
    assert_eq!(actual, expected);
    struct FlatHeight;
    impl crate::noise::structure_pieces::JigsawHeightAccess for FlatHeight {
        fn first_free_height(
            &self,
            _x: i32,
            _z: i32,
            _heightmap: StructureHeightmap,
        ) -> Option<i32> {
            Some(64)
        }
    }
    let graph = |plan: &NoiseStructurePlan, startup: &DatapackStartup| {
        plan.jigsaw_piece_graph(
            startup,
            "minecraft:village_plains",
            0,
            0,
            0,
            -64,
            384,
            &FlatHeight,
        )
        .unwrap()
    };
    let expected_graph = graph(&expected, &builtin);
    assert!(!expected_graph.pieces.is_empty());
    assert_eq!(graph(&actual, &loaded), expected_graph);

    std::fs::write(
        pack.join("data/minecraft/worldgen/template_pool/empty.json"),
        br#"{"fallback":"\ud800","elements":[],"note":"\ud800"}"#,
    )
    .unwrap();
    let invalid = DatapackStartup::load(&options).unwrap();
    assert!(overworld_plan_result(&invalid).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn structure_decoration_keeps_step_order() {
    let startup = startup();
    let plan = overworld_plan(&startup);
    let invocations = plan.decoration_invocations_for_chunk(0, 0, 0);
    assert_eq!(
        invocations
            .iter()
            .filter(|invocation| invocation.step == 3)
            .map(|invocation| invocation.structure.as_str())
            .collect::<Vec<_>>(),
        [
            "minecraft:buried_treasure",
            "minecraft:mineshaft",
            "minecraft:mineshaft_mesa",
            "minecraft:trail_ruins",
            "minecraft:trial_chambers",
        ]
    );
}

#[test]
fn random_spread_candidate_stays_in_its_floor_divided_cell() {
    let startup = startup();
    let plan = overworld_plan(&startup);
    let villages = &plan.set("minecraft:villages").unwrap().placement;
    for (source_x, source_z, cell_x, cell_z) in [(0, 0, 0, 0), (-1, -1, -1, -1), (-34, 57, -1, 1)] {
        let (x, z) = villages
            .random_spread_candidate(0, source_x, source_z)
            .unwrap();
        assert!((cell_x * 34..cell_x * 34 + 26).contains(&x), "{x}");
        assert!((cell_z * 34..cell_z * 34 + 26).contains(&z), "{z}");
        assert_eq!(
            villages.random_spread_candidate(0, x, z),
            Some((x, z)),
            "every chunk of a cell shares its candidate"
        );
    }
    let (x, z) = villages.random_spread_candidate(0, 0, 0).unwrap();
    assert_eq!(
        plan.is_random_spread_structure_chunk("minecraft:villages", 0, x, z),
        Some(true)
    );
    assert_eq!(
        plan.is_random_spread_structure_chunk("minecraft:villages", 0, x + 1, z),
        Some(false)
    );
    assert_ne!(
        villages.random_spread_candidate(1, 0, 0),
        villages.random_spread_candidate(0, 0, 0)
    );
    assert_eq!(
        plan.is_random_spread_structure_chunk("minecraft:strongholds", 0, 0, 0),
        None
    );
}

#[test]
fn built_in_frequency_reducers_and_outpost_exclusion_are_deterministic() {
    let mut startup = startup();
    let plan = overworld_plan(&startup);
    let first_hits = |set: &str, radius: i32, limit: usize| {
        let mut hits = Vec::new();
        for x in -radius..=radius {
            for z in -radius..=radius {
                if plan.is_random_spread_structure_chunk(set, 0, x, z) == Some(true) {
                    hits.push((x, z));
                    if hits.len() == limit {
                        return hits;
                    }
                }
            }
        }
        hits
    };
    for (set, radius) in [
        ("minecraft:buried_treasures", 64),
        ("minecraft:mineshafts", 128),
        ("minecraft:pillager_outposts", 512),
    ] {
        let hits = first_hits(set, radius, 3);
        assert_eq!(hits.len(), 3, "{set}");
        assert_eq!(hits, first_hits(set, radius, 3));
    }

    let common = CommonPlacement {
        locate_offset: [0; 3],
        frequency: 0.25,
        salt: 17,
        exclusion_zone: None,
    };
    let chunks = (-64..64).flat_map(|x| (-64..64).map(move |z| (x, z)));
    let passing = chunks
        .clone()
        .filter(|&(x, z)| common.passes_frequency(0, x, z))
        .count();
    assert!(
        (3_500..4_700).contains(&passing),
        "{passing} of 16384 pass 0.25"
    );
    let pass = chunks
        .clone()
        .find(|&(x, z)| common.passes_frequency(0, x, z))
        .unwrap();
    let miss = chunks
        .clone()
        .find(|&(x, z)| !common.passes_frequency(0, x, z))
        .unwrap();

    mutate_registry_source(
        &mut startup,
        STRUCTURE_SET_REGISTRY,
        "minecraft:strongholds",
        |source| {
            source["placement"]["frequency"] = serde_json::json!(0.25);
            source["placement"]["salt"] = serde_json::json!(17);
        },
    );
    let plan = overworld_plan(&startup);
    let rings = BTreeMap::from([(
        "minecraft:strongholds".to_string(),
        BTreeSet::from([[pass.0, pass.1], [miss.0, miss.1]]),
    )]);
    assert_eq!(
        plan.is_structure_chunk("minecraft:strongholds", 0, pass.0, pass.1, &rings),
        Some(true)
    );
    assert_eq!(
        plan.is_structure_chunk("minecraft:strongholds", 0, miss.0, miss.1, &rings),
        Some(false)
    );

    mutate_registry_source(
        &mut startup,
        STRUCTURE_SET_REGISTRY,
        "minecraft:strongholds",
        |source| {
            source["placement"]["frequency"] = serde_json::json!(1.0);
            source["placement"]["exclusion_zone"] =
                serde_json::json!({"other_set":"minecraft:villages", "chunk_count":1});
        },
    );
    let mut plan = overworld_plan(&startup);
    let (x, z) = plan
        .set("minecraft:villages")
        .unwrap()
        .placement
        .random_spread_candidate(0, 0, 0)
        .unwrap();
    plan.restrict_sets(&["minecraft:strongholds".to_string()]);
    let rings = BTreeMap::from([(
        "minecraft:strongholds".to_string(),
        BTreeSet::from([[x, z]]),
    )]);
    assert!(plan.set("minecraft:villages").is_none());
    assert_eq!(
        plan.is_structure_chunk("minecraft:strongholds", 0, x, z, &rings),
        Some(false)
    );
}

#[test]
fn multi_structure_sets_try_every_member() {
    let startup = startup();
    let plan = overworld_plan(&startup);
    let attempts = |set_id: &str| {
        let [x, z] = plan.first_candidate(set_id, 0);
        let mut order = plan.random_spread_attempt_order(set_id, 0, x, z).unwrap();
        order.sort();
        order
    };
    let sorted = |ids: &[&str]| {
        let mut ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        ids.sort();
        ids
    };
    assert_eq!(
        attempts("minecraft:villages"),
        sorted(&[
            "minecraft:village_desert",
            "minecraft:village_snowy",
            "minecraft:village_plains",
            "minecraft:village_savanna",
            "minecraft:village_taiga",
        ])
    );
    assert_eq!(
        attempts("minecraft:shipwrecks"),
        sorted(&["minecraft:shipwreck", "minecraft:shipwreck_beached"])
    );
    assert_eq!(
        attempts("minecraft:ruined_portals"),
        sorted(&[
            "minecraft:ruined_portal_desert",
            "minecraft:ruined_portal",
            "minecraft:ruined_portal_mountain",
            "minecraft:ruined_portal_ocean",
            "minecraft:ruined_portal_jungle",
            "minecraft:ruined_portal_swamp",
            "minecraft:ruined_portal_nether",
        ])
    );
    assert_eq!(
        attempts("minecraft:mineshafts"),
        sorted(&["minecraft:mineshaft", "minecraft:mineshaft_mesa"])
    );
    assert_eq!(
        attempts("minecraft:ocean_ruins"),
        sorted(&["minecraft:ocean_ruin_warm", "minecraft:ocean_ruin_cold"])
    );
}

#[test]
fn custom_decoration_steps_terrain_adjustment_and_concentric_rings_keep_owners() {
    let mut startup = startup();
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:end_city",
        |json| {
            json["step"] = serde_json::json!("vegetal_decoration");
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
        },
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:fortress",
        |json| json["step"] = serde_json::json!("top_layer_modification"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:nether_fossil",
        |json| json["step"] = serde_json::json!("fluid_springs"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:buried_treasure",
        |json| json["step"] = serde_json::json!("raw_generation"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:mineshaft",
        |json| json["step"] = serde_json::json!("lakes"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:igloo",
        |json| json["step"] = serde_json::json!("local_modifications"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:desert_pyramid",
        |json| json["step"] = serde_json::json!("underground_ores"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:monument",
        |json| json["step"] = serde_json::json!("underground_decoration"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:mansion",
        |json| json["step"] = serde_json::json!("top_layer_modification"),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:stronghold",
        |json| json["step"] = serde_json::json!("underground_structures"),
    );
    let concentric_placement = serde_json::json!({
        "type": "minecraft:concentric_rings",
        "salt": 10387313,
        "distance": 4,
        "spread": 2,
        "count": 3,
        "preferred_biomes": "#minecraft:stronghold_biased_to"
    });
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure_set",
        "minecraft:villages",
        |json| json["placement"] = concentric_placement.clone(),
    );
    mutate_registry_source(
        &mut startup,
        "minecraft:worldgen/structure_set",
        "minecraft:end_cities",
        |json| json["placement"] = concentric_placement.clone(),
    );

    let all_biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let plan = NoiseStructurePlan::from_startup(&startup, &all_biomes).unwrap();

    for (set_id, expected_owner) in [
        ("minecraft:end_cities", StructureStartSetOwner::EndCity),
        (
            "minecraft:nether_complexes",
            StructureStartSetOwner::NetherComplex,
        ),
        (
            "minecraft:nether_fossils",
            StructureStartSetOwner::NetherFossil,
        ),
        (
            "minecraft:buried_treasures",
            StructureStartSetOwner::BuriedTreasure,
        ),
        ("minecraft:mineshafts", StructureStartSetOwner::Mineshaft),
        ("minecraft:igloos", StructureStartSetOwner::Igloo),
        (
            "minecraft:desert_pyramids",
            StructureStartSetOwner::DesertPyramid,
        ),
        (
            "minecraft:ocean_monuments",
            StructureStartSetOwner::OceanMonument,
        ),
        (
            "minecraft:woodland_mansions",
            StructureStartSetOwner::WoodlandMansion,
        ),
        ("minecraft:strongholds", StructureStartSetOwner::Stronghold),
        ("minecraft:villages", StructureStartSetOwner::Jigsaw),
    ] {
        let set = plan.set(set_id).unwrap();
        assert_eq!(plan.structure_start_owner(set), expected_owner, "{set_id}");
    }

    let end_city = &plan.structures()["minecraft:end_city"];
    assert_eq!(end_city.step, StructureDecorationStep::VegetalDecoration);
    assert_eq!(
        end_city.terrain_adjustment,
        StructureTerrainAdjustment::BeardThin
    );
    assert_eq!(end_city.spawn_overrides.len(), 1);
    let invocations = plan.decoration_invocations_for_chunk(0, 0, 0);
    assert_eq!(
        invocations
            .iter()
            .find(|inv| inv.structure == "minecraft:end_city")
            .map(|inv| inv.step),
        Some(StructureDecorationStep::VegetalDecoration.ordinal())
    );
    assert_eq!(
        invocations
            .iter()
            .find(|inv| inv.structure == "minecraft:fortress")
            .map(|inv| inv.step),
        Some(StructureDecorationStep::TopLayerModification.ordinal())
    );

    let ring_chunks = plan
        .set("minecraft:villages")
        .unwrap()
        .placement
        .concentric_ring_positions_with(0, |_, _, _, _, _| None)
        .unwrap();
    assert_eq!(ring_chunks.len(), 3);
    let rings = std::collections::BTreeMap::from([(
        "minecraft:villages".to_string(),
        ring_chunks.iter().map(|&(x, z)| [x, z]).collect(),
    )]);
    struct FlatHeight;
    impl crate::noise::structure_pieces::JigsawHeightAccess for FlatHeight {
        fn first_free_height(&self, _x: i32, _z: i32, _: StructureHeightmap) -> Option<i32> {
            Some(64)
        }
    }
    let starts = plan.generate_missing_jigsaw_starts_with_rings(
        &startup,
        0,
        ring_chunks[0].0,
        ring_chunks[0].1,
        &rings,
        -64,
        384,
        &[],
        &FlatHeight,
        |_| Some("minecraft:plains".to_string()),
    );
    assert_eq!(starts.len(), 1);
    assert!(starts[0].structure_id().starts_with("minecraft:village_"));
}

#[test]
fn structures_ignore_extra_fields_reject_wrong_types_and_accept_inline_holders() {
    let mut startup = startup();
    mutate_registry_source(
        &mut startup,
        STRUCTURE_REGISTRY,
        "minecraft:village_plains",
        |json| {
            json["_comment"] = serde_json::json!("custom datapack metadata");
            json["start_pool"] = serde_json::json!({
                "fallback": "minecraft:empty",
                "elements": []
            });
        },
    );
    mutate_registry_source(
        &mut startup,
        STRUCTURE_SET_REGISTRY,
        "minecraft:villages",
        |json| {
            json["_note"] = serde_json::json!("extra metadata ignored");
            json["structures"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "weight": 1,
                    "structure": {
                        "type": "minecraft:igloo",
                        "biomes": "minecraft:plains",
                        "step": "surface_structures",
                        "spawn_overrides": {},
                        "terrain_adaptation": "none",
                        "_extra": true
                    }
                }));
        },
    );
    let plan = overworld_plan(&startup);
    let villages = plan.set("minecraft:villages").unwrap();
    let inline_entry = villages.structures.last().unwrap();
    assert!(inline_entry.structure.starts_with("inline:"));
    assert_eq!(
        plan.structures()[&inline_entry.structure].type_id,
        "minecraft:igloo"
    );
    let inline_start_pool = &plan.structures()["minecraft:village_plains"]
        .jigsaw
        .as_ref()
        .unwrap()
        .start_pool;
    assert!(plan.pools().contains(inline_start_pool));

    let projected = plan.projected_set("minecraft:villages").unwrap();
    assert!(std::sync::Arc::ptr_eq(&plan.pools, &projected.pools));
    assert!(std::sync::Arc::ptr_eq(
        &plan.structures,
        &projected.structures
    ));

    let mut bad_spread = startup.clone();
    mutate_registry_source(
        &mut bad_spread,
        STRUCTURE_SET_REGISTRY,
        "minecraft:villages",
        |json| {
            json["placement"]["spread_type"] = serde_json::json!(123);
        },
    );
    assert!(overworld_plan_result(&bad_spread).is_err());

    let mut bad_freq = startup;
    mutate_registry_source(
        &mut bad_freq,
        STRUCTURE_SET_REGISTRY,
        "minecraft:villages",
        |json| {
            json["placement"]["frequency"] = serde_json::json!("half");
        },
    );
    assert!(overworld_plan_result(&bad_freq).is_err());
}
