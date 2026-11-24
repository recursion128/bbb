use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::packets::BlockPos;
use serde_json::{json, Value};

use crate::noise::biome::NoiseBiomeSource;
use crate::tests::{retained_from_json, RecordingLevel};
use crate::WorldGenLevel;
use bbb_server_datapack::dynamic_registry::{
    DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement,
};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_datapack::tag_loader::{LoadedRegistryTags, TagLoadReport};
use bbb_server_datapack::worldgen::biome::BiomeFeatures;
use bbb_server_datapack::worldgen::{ConfiguredFeature, WorldgenElement, WorldgenRegistries};

use super::{NoiseFeatureInvocation, NoiseFeaturesPlan, BIOME_REGISTRY, PLACED_FEATURE_REGISTRY};

struct ExecutorLevel {
    inner: RecordingLevel,
    no_op: Arc<ConfiguredFeature>,
    biome_queries: RefCell<Vec<(BlockPos, String)>>,
    biome_answers: BTreeMap<String, bool>,
}

impl ExecutorLevel {
    fn new() -> Self {
        Self {
            inner: RecordingLevel::admitting(),
            no_op: Arc::new(retained_from_json(
                br#"{"type":"minecraft:no_op","config":{}}"#,
            )),
            biome_queries: RefCell::new(Vec::new()),
            biome_answers: BTreeMap::new(),
        }
    }
}

impl WorldGenLevel for ExecutorLevel {
    fn ensure_can_write(&self, origin: BlockPos) -> bool {
        self.inner.ensure_can_write(origin)
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        self.inner.get_block(pos)
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        self.inner.set_block(pos, state, update_flags)
    }

    fn get_min_y(&self) -> i32 {
        self.inner.get_min_y()
    }

    fn get_max_y(&self) -> i32 {
        self.inner.get_max_y()
    }

    fn get_sea_level(&self) -> i32 {
        self.inner.get_sea_level()
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
        self.inner.schedule_fluid_tick(pos, delay);
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        self.inner.schedule_block_tick(pos, delay);
    }

    fn next_level_random_boolean(&mut self) -> bool {
        self.inner.next_level_random_boolean()
    }

    fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
        self.inner.mark_pos_for_postprocessing(pos);
    }

    fn configured_feature(&self, id: &str) -> Option<Arc<ConfiguredFeature>> {
        (id == "minecraft:no_op").then(|| self.no_op.clone())
    }

    fn biome_filter_should_place(&self, origin: BlockPos, top_feature: &str) -> bool {
        self.biome_queries
            .borrow_mut()
            .push((origin, top_feature.to_string()));
        self.biome_answers
            .get(top_feature)
            .copied()
            .unwrap_or(false)
    }
}

fn element(id: &str, source: Value) -> RawRegistryElement {
    RawRegistryElement {
        id: id.to_string(),
        source_pack: "test".to_string(),
        source_json: serde_json::to_vec(&source).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(
                &serde_json::to_vec(&(source)).unwrap(),
            )
            .unwrap(),
        ),
        codec_value: None,
    }
}

fn generation(biomes: &[(&str, Value)], placed_features: &[&str]) -> DatapackGeneration {
    let placed = placed_features
        .iter()
        .map(|id| {
            let configured = format!(
                "test_configured:{}",
                id.split_once(':').map_or(*id, |(_, path)| path)
            );
            (*id, json!({"feature": configured, "placement": []}))
        })
        .collect::<Vec<_>>();
    generation_with(biomes, &placed)
}

fn generation_with(
    biomes: &[(&str, Value)],
    placed_features: &[(&str, Value)],
) -> DatapackGeneration {
    let mut worldgen = WorldgenRegistries::default();
    for (id, source) in placed_features {
        let json =
            bbb_registry_codec::parse_registry_json(&serde_json::to_vec(source).unwrap()).unwrap();
        let element = WorldgenElement::decode(PLACED_FEATURE_REGISTRY, &json)
            .unwrap()
            .unwrap();
        worldgen.insert(id.to_string(), element);
    }
    for (id, source) in biomes {
        let json =
            bbb_registry_codec::parse_registry_json(&serde_json::to_vec(source).unwrap()).unwrap();
        if let Ok(features) = BiomeFeatures::decode(&json) {
            worldgen.biome_features.insert(id.to_string(), features);
        }
    }
    DatapackGeneration {
        registries: vec![
            LoadedDynamicRegistry {
                registry_id: BIOME_REGISTRY.to_string(),
                elements: biomes
                    .iter()
                    .map(|(id, source)| element(id, source.clone()))
                    .collect(),
            },
            LoadedDynamicRegistry {
                registry_id: PLACED_FEATURE_REGISTRY.to_string(),
                elements: placed_features
                    .iter()
                    .map(|(id, source)| element(id, source.clone()))
                    .collect(),
            },
        ],
        worldgen,
    }
}

fn biome(steps: Value) -> Value {
    json!({"features": steps})
}

fn placed_feature_tags(tags: &[(&str, &[&str])]) -> TagLoadReport {
    TagLoadReport {
        registries: vec![LoadedRegistryTags {
            registry_id: PLACED_FEATURE_REGISTRY.to_string(),
            tags: tags
                .iter()
                .map(|(id, entries)| {
                    (
                        (*id).to_string(),
                        entries.iter().map(|entry| (*entry).to_string()).collect(),
                    )
                })
                .collect(),
        }],
        warnings: Vec::new(),
    }
}

fn names(invocations: &[NoiseFeatureInvocation]) -> Vec<&str> {
    invocations
        .iter()
        .map(|invocation| invocation.placed_feature.as_str())
        .collect()
}

fn ephemeral_options() -> DatapackStartupOptions {
    DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    }
}

#[test]
fn possible_biome_encounter_order_owns_feature_identity_and_duplicates_are_ignored() {
    let generation = generation(
        &[
            ("minecraft:a", biome(json!([["minecraft:one"]]))),
            ("minecraft:b", biome(json!([["minecraft:two"]]))),
        ],
        &["minecraft:one", "minecraft:two"],
    );
    let first = NoiseFeaturesPlan::from_registry(
        &generation,
        &["minecraft:a", "minecraft:b", "minecraft:a"],
    )
    .unwrap();
    let without_duplicate =
        NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a", "minecraft:b"]).unwrap();
    let reversed =
        NoiseFeaturesPlan::from_registry(&generation, &["minecraft:b", "minecraft:a"]).unwrap();

    let first = first
        .invocations_for_chunk(0, 0, 0, &["minecraft:a", "minecraft:b"])
        .unwrap();
    let without_duplicate = without_duplicate
        .invocations_for_chunk(0, 0, 0, &["minecraft:a", "minecraft:b"])
        .unwrap();
    let reversed = reversed
        .invocations_for_chunk(0, 0, 0, &["minecraft:a", "minecraft:b"])
        .unwrap();
    assert_eq!(first, without_duplicate);
    assert_eq!(names(&first), ["minecraft:two", "minecraft:one"]);
    assert_eq!(names(&reversed), ["minecraft:one", "minecraft:two"]);
}

#[test]
fn sorter_keeps_the_cross_step_dag_instead_of_sorting_steps_independently() {
    let generation = generation(
        &[
            ("minecraft:a", biome(json!([[], ["minecraft:shared"]]))),
            (
                "minecraft:b",
                biome(json!([["minecraft:shared"], ["minecraft:later"]])),
            ),
        ],
        &["minecraft:shared", "minecraft:later"],
    );
    let plan =
        NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a", "minecraft:b"]).unwrap();
    let invocations = plan
        .invocations_for_chunk(0, 0, 0, &["minecraft:a", "minecraft:b"])
        .unwrap();

    assert_eq!(
        invocations
            .iter()
            .map(|invocation| (
                invocation.step,
                invocation.global_index,
                invocation.placed_feature.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (0, 0, "minecraft:shared"),
            (1, 0, "minecraft:shared"),
            (1, 1, "minecraft:later"),
        ]
    );
}

#[test]
fn neighborhood_intersection_deduplicates_and_ignores_known_non_possible_biomes() {
    let generation = generation(
        &[
            (
                "minecraft:a",
                biome(json!([["minecraft:one", "minecraft:two"]])),
            ),
            (
                "minecraft:b",
                biome(json!([["minecraft:two", "minecraft:three"]])),
            ),
            ("minecraft:outside", json!({"not_features": true})),
        ],
        &["minecraft:one", "minecraft:two", "minecraft:three"],
    );
    let plan =
        NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a", "minecraft:b"]).unwrap();
    let invocations = plan
        .invocations_for_chunk(
            0,
            0,
            0,
            &[
                "minecraft:a",
                "minecraft:outside",
                "minecraft:a",
                "minecraft:b",
            ],
        )
        .unwrap();

    assert_eq!(
        names(&invocations),
        ["minecraft:one", "minecraft:two", "minecraft:three"]
    );
    assert!(plan
        .invocations_for_chunk(0, 0, 0, &["minecraft:outside"])
        .unwrap()
        .is_empty());
    let error = plan
        .invocations_for_chunk(0, 0, 0, &["minecraft:missing"])
        .unwrap_err()
        .to_string();
    assert!(error.contains("neighborhood_biomes[0]"), "{error}");
    assert!(error.contains("unknown biome minecraft:missing"), "{error}");
}

#[test]
fn source_exact_feature_holders_expand_tags_in_order_and_reuse_named_identity() {
    let generation = generation(
        &[
            ("minecraft:a", biome(json!(["#minecraft:ordered"]))),
            ("minecraft:b", biome(json!([["minecraft:two"]]))),
        ],
        &["minecraft:one", "minecraft:two"],
    );
    let tags = placed_feature_tags(&[("minecraft:ordered", &["minecraft:two", "minecraft:one"])]);
    let plan = NoiseFeaturesPlan::from_registry_with_tags(
        &generation,
        &tags,
        &["minecraft:a", "minecraft:b"],
    )
    .unwrap();
    let invocations = plan
        .invocations_for_chunk(42, 1, -1, &["minecraft:a", "minecraft:b"])
        .unwrap();

    assert_eq!(names(&invocations), ["minecraft:two", "minecraft:one"]);
    assert_eq!(invocations[0].global_index, 0);
    assert_eq!(invocations[1].global_index, 1);
}

#[test]
fn equal_placed_feature_values_keep_each_biomes_declared_key() {
    let feature = json!({
        "feature": "minecraft:no_op",
        "placement": [{"type": "minecraft:biome"}]
    });
    let generation = generation_with(
        &[
            ("minecraft:a", biome(json!([["minecraft:one"]]))),
            ("minecraft:b", biome(json!([["minecraft:two"]]))),
        ],
        &[
            ("minecraft:one", feature.clone()),
            ("minecraft:two", feature),
        ],
    );
    let plan =
        NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a", "minecraft:b"]).unwrap();
    let origin = BlockPos { x: 0, y: -64, z: 0 };
    for (biome, declared, other) in [
        ("minecraft:a", "minecraft:one", "minecraft:two"),
        ("minecraft:b", "minecraft:two", "minecraft:one"),
    ] {
        let invocations = plan.invocations_for_chunk(42, 0, 0, &[biome]).unwrap();
        assert_eq!(names(&invocations), [declared]);
        assert!(plan.biome_contains_top_feature(biome, declared));
        assert!(!plan.biome_contains_top_feature(biome, other));
        let mut level = ExecutorLevel::new();
        level.biome_answers.insert(declared.to_string(), true);
        let (_, summary) = plan
            .place_step_for_chunk(42, 0, 0, -64, 0, &[biome], &mut level)
            .unwrap();
        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.placed, 1);
        assert_eq!(names(&summary.placed_invocations), [declared]);
        assert_eq!(
            *level.biome_queries.borrow(),
            [(origin, declared.to_string())]
        );
    }
}

#[test]
fn executor_keeps_plan_order_counts_false_and_passes_the_exact_top_feature() {
    let plain = json!({"feature": "minecraft:no_op", "placement": []});
    let generation = generation_with(
        &[(
            "minecraft:a",
            biome(json!([
                ["minecraft:false", "minecraft:true"],
                ["minecraft:later"]
            ])),
        )],
        &[
            (
                "minecraft:false",
                json!({"feature": "minecraft:no_op", "placement": [{"type": "minecraft:biome"}]}),
            ),
            ("minecraft:true", plain.clone()),
            (
                "minecraft:later",
                json!({"feature": "minecraft:no_op", "placement": [{"type": "minecraft:count", "count": 1}]}),
            ),
        ],
    );
    let plan = NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a"]).unwrap();
    let mut level = ExecutorLevel::new();
    level
        .biome_answers
        .insert("minecraft:false".to_string(), false);

    let placements = (0..plan.step_count())
        .map(|step| {
            plan.place_step_for_chunk(
                42,
                i32::MAX,
                -2,
                -64,
                step,
                &["minecraft:a", "minecraft:a"],
                &mut level,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        placements
            .iter()
            .map(|(_, summary)| summary.attempted)
            .sum::<usize>(),
        3
    );
    assert_eq!(
        placements
            .iter()
            .map(|(_, summary)| summary.placed)
            .sum::<usize>(),
        2
    );
    let placed_invocations = placements
        .into_iter()
        .flat_map(|(_, summary)| summary.placed_invocations)
        .collect::<Vec<_>>();
    assert_eq!(
        names(&placed_invocations),
        ["minecraft:true", "minecraft:later"]
    );
    assert_eq!(
        *level.biome_queries.borrow(),
        [(
            BlockPos {
                x: -16,
                y: -64,
                z: -32,
            },
            "minecraft:false".to_string(),
        )]
    );
}

#[test]
fn single_step_executor_places_an_ordered_prefix_across_steps() {
    let ids = [
        "minecraft:zero",
        "minecraft:first",
        "minecraft:second",
        "minecraft:later",
    ];
    let placed = ids
        .iter()
        .enumerate()
        .map(|(count, id)| {
            let placement = json!([{"type": "minecraft:count", "count": count + 1}]);
            (
                *id,
                json!({"feature": "minecraft:no_op", "placement": placement}),
            )
        })
        .collect::<Vec<_>>();
    let generation = generation_with(
        &[(
            "minecraft:a",
            biome(json!([
                ["minecraft:zero"],
                ["minecraft:first", "minecraft:second"],
                ["minecraft:later"]
            ])),
        )],
        &placed,
    );
    let plan = NoiseFeaturesPlan::from_registry(&generation, &["minecraft:a"]).unwrap();
    let mut level = ExecutorLevel::new();

    let placements = (0..=1)
        .map(|step| {
            plan.place_step_for_chunk(42, i32::MAX, -2, -64, step, &["minecraft:a"], &mut level)
                .unwrap()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        placements
            .iter()
            .flat_map(|(invocations, _)| invocations)
            .map(|invocation| (
                invocation.step,
                invocation.global_index,
                invocation.placed_feature.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (0, 0, "minecraft:zero"),
            (1, 0, "minecraft:first"),
            (1, 1, "minecraft:second"),
        ]
    );
    assert_eq!(
        placements
            .iter()
            .map(|(_, summary)| summary.attempted)
            .sum::<usize>(),
        3
    );
    assert_eq!(
        placements
            .iter()
            .map(|(_, summary)| summary.placed)
            .sum::<usize>(),
        3
    );
}

#[test]
fn built_in_overworld_possible_biomes_compile_a_feature_plan() {
    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);

    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    assert!(!plan
        .invocations_for_chunk(0, 0, 0, possible)
        .unwrap()
        .is_empty());
}

#[test]
fn built_in_overworld_underground_structures_keep_global_order_and_biome_selection() {
    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let underground_structures = |biomes: &[&str]| {
        plan.invocations_for_chunk(0, 0, 0, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 3)
            .map(|invocation| {
                (
                    invocation.step,
                    invocation.global_index,
                    invocation.placed_feature,
                )
            })
            .collect::<Vec<_>>()
    };

    // `BiomeDefaultFeatures.java:23-25` and `BiomeDefaultFeatures.java:374-376`.
    let forest = underground_structures(&["minecraft:forest"]);
    assert_eq!(forest.len(), 2);
    assert_eq!(
        forest,
        [
            (3, 2, "minecraft:monster_room".to_string()),
            (3, 3, "minecraft:monster_room_deep".to_string()),
        ]
    );

    let swamp = underground_structures(&["minecraft:swamp"]);
    assert_eq!(swamp.len(), 4);
    assert_eq!(
        swamp,
        [
            (3, 0, "minecraft:fossil_upper".to_string()),
            (3, 1, "minecraft:fossil_lower".to_string()),
            (3, 2, "minecraft:monster_room".to_string()),
            (3, 3, "minecraft:monster_room_deep".to_string()),
        ]
    );
}

#[test]
fn built_in_overworld_surface_structures_keep_global_order_and_biome_selection() {
    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let surface_structures = |biomes: &[&str]| {
        plan.invocations_for_chunk(0, 0, 0, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 4)
            .map(|invocation| {
                (
                    invocation.step,
                    invocation.global_index,
                    invocation.placed_feature,
                )
            })
            .collect::<Vec<_>>()
    };

    // `FeatureSorter.java:28-109`; `GenerationStep.java:7-18`.
    // `OverworldBiomes.java:197-210`, `OverworldBiomes.java:220-235`, `OverworldBiomes.java:439-452`.
    assert_eq!(
        surface_structures(possible),
        [
            (4, 0, "minecraft:ice_spike".to_string()),
            (4, 1, "minecraft:ice_patch".to_string()),
            (4, 2, "minecraft:desert_well".to_string()),
            (4, 3, "minecraft:blue_ice".to_string()),
        ]
    );
    assert_eq!(
        surface_structures(&["minecraft:ice_spikes"]),
        [
            (4, 0, "minecraft:ice_spike".to_string()),
            (4, 1, "minecraft:ice_patch".to_string()),
        ]
    );
    assert_eq!(
        surface_structures(&["minecraft:desert"]),
        [(4, 2, "minecraft:desert_well".to_string())]
    );
    for biome in ["minecraft:deep_frozen_ocean", "minecraft:frozen_ocean"] {
        assert_eq!(
            surface_structures(&[biome]),
            [(4, 3, "minecraft:blue_ice".to_string())]
        );
    }
    assert!(surface_structures(&["minecraft:plains"]).is_empty());
}

#[test]
fn built_in_overworld_strongholds_and_underground_ores_keep_global_order_and_biome_selection() {
    const NORMAL_BIOMES: &[&str] = &[
        "minecraft:mushroom_fields",
        "minecraft:deep_frozen_ocean",
        "minecraft:frozen_ocean",
        "minecraft:deep_cold_ocean",
        "minecraft:cold_ocean",
        "minecraft:deep_ocean",
        "minecraft:ocean",
        "minecraft:deep_lukewarm_ocean",
        "minecraft:lukewarm_ocean",
        "minecraft:warm_ocean",
        "minecraft:stony_shore",
        "minecraft:snowy_plains",
        "minecraft:snowy_beach",
        "minecraft:snowy_taiga",
        "minecraft:taiga",
        "minecraft:plains",
        "minecraft:beach",
        "minecraft:forest",
        "minecraft:old_growth_spruce_taiga",
        "minecraft:flower_forest",
        "minecraft:birch_forest",
        "minecraft:dark_forest",
        "minecraft:pale_garden",
        "minecraft:savanna_plateau",
        "minecraft:savanna",
        "minecraft:jungle",
        "minecraft:desert",
        "minecraft:frozen_river",
        "minecraft:river",
        "minecraft:ice_spikes",
        "minecraft:old_growth_pine_taiga",
        "minecraft:sunflower_plains",
        "minecraft:old_growth_birch_forest",
        "minecraft:sparse_jungle",
        "minecraft:bamboo_jungle",
        "minecraft:windswept_savanna",
        "minecraft:deep_dark",
    ];
    const EMERALD_BIOMES: &[&str] = &[
        "minecraft:snowy_slopes",
        "minecraft:windswept_gravelly_hills",
        "minecraft:grove",
        "minecraft:windswept_hills",
        "minecraft:windswept_forest",
        "minecraft:meadow",
        "minecraft:jagged_peaks",
        "minecraft:stony_peaks",
        "minecraft:cherry_grove",
        "minecraft:frozen_peaks",
    ];
    const BADLANDS_BIOMES: &[&str] = &[
        "minecraft:badlands",
        "minecraft:wooded_badlands",
        "minecraft:eroded_badlands",
    ];

    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let invocations = |biomes: &[&str]| plan.invocations_for_chunk(0, 0, 0, biomes).unwrap();
    let underground_ores = |biomes: &[&str]| {
        invocations(biomes)
            .into_iter()
            .filter(|invocation| invocation.step == 6)
            .map(|invocation| (invocation.global_index, invocation.placed_feature))
            .collect::<Vec<_>>()
    };
    let all_invocations = invocations(possible);

    // `GenerationStep.java:7-18`; `FeatureSorter.java:28-115`;
    // `ChunkGenerator.java:361-394`.
    assert!(all_invocations
        .iter()
        .all(|invocation| invocation.step != 5));
    assert_eq!(
        all_invocations
            .iter()
            .filter(|invocation| invocation.step == 6)
            .map(|invocation| (invocation.global_index, invocation.placed_feature.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "minecraft:ore_dirt"),
            (1, "minecraft:ore_gravel"),
            (2, "minecraft:ore_granite_upper"),
            (3, "minecraft:ore_granite_lower"),
            (4, "minecraft:ore_diorite_upper"),
            (5, "minecraft:ore_diorite_lower"),
            (6, "minecraft:ore_andesite_upper"),
            (7, "minecraft:ore_andesite_lower"),
            (8, "minecraft:ore_tuff"),
            (9, "minecraft:ore_coal_upper"),
            (10, "minecraft:ore_coal_lower"),
            (11, "minecraft:ore_iron_upper"),
            (12, "minecraft:ore_iron_middle"),
            (13, "minecraft:ore_iron_small"),
            (14, "minecraft:ore_gold"),
            (15, "minecraft:ore_gold_lower"),
            (16, "minecraft:ore_redstone"),
            (17, "minecraft:ore_redstone_lower"),
            (18, "minecraft:ore_diamond"),
            (19, "minecraft:ore_diamond_medium"),
            (20, "minecraft:ore_diamond_large"),
            (21, "minecraft:ore_diamond_buried"),
            (22, "minecraft:ore_lapis"),
            (23, "minecraft:ore_lapis_buried"),
            (24, "minecraft:ore_copper_large"),
            (25, "minecraft:ore_copper"),
            (26, "minecraft:underwater_magma"),
            (27, "minecraft:ore_clay"),
            (28, "minecraft:ore_gold_extra"),
            (29, "minecraft:disk_grass"),
            (30, "minecraft:disk_sand"),
            (31, "minecraft:disk_clay"),
            (32, "minecraft:disk_gravel"),
            (33, "minecraft:ore_emerald"),
        ]
    );

    let normal = (0..=23).chain([25, 26, 30, 31, 32]).collect::<Vec<_>>();
    let swamp = (0..=23).chain([25, 26, 31]).collect::<Vec<_>>();
    let mangrove = (0..=23).chain([25, 26, 29, 31]).collect::<Vec<_>>();
    let emerald = (0..=23).chain([25, 26, 30, 31, 32, 33]).collect::<Vec<_>>();
    let badlands = (0..=23).chain([25, 26, 28, 30, 31, 32]).collect::<Vec<_>>();
    let dripstone = (0..=24).chain([26, 30, 31, 32]).collect::<Vec<_>>();
    let lush = (0..=23).chain([25, 26, 27, 30, 31, 32]).collect::<Vec<_>>();
    let groups = [
        (NORMAL_BIOMES, normal.as_slice()),
        (&["minecraft:swamp"][..], swamp.as_slice()),
        (&["minecraft:mangrove_swamp"][..], mangrove.as_slice()),
        (EMERALD_BIOMES, emerald.as_slice()),
        (BADLANDS_BIOMES, badlands.as_slice()),
        (&["minecraft:dripstone_caves"][..], dripstone.as_slice()),
        (&["minecraft:lush_caves"][..], lush.as_slice()),
    ];
    let mut covered_biomes = Vec::new();
    for (biomes, expected) in groups {
        for biome in biomes {
            let actual = underground_ores(&[biome])
                .into_iter()
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{biome}");
            covered_biomes.push(*biome);
        }
    }
    covered_biomes.sort_unstable();
    let mut expected_biomes = possible.to_vec();
    expected_biomes.sort_unstable();
    assert_eq!(covered_biomes, expected_biomes);

    let mixed = underground_ores(&["minecraft:plains", "minecraft:dripstone_caves"]);
    assert_eq!(
        mixed.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
        (0..=26).chain([30, 31, 32]).collect::<Vec<_>>()
    );
    assert_eq!(
        mixed
            .iter()
            .filter(|(index, _)| matches!(index, 24 | 25))
            .map(|(index, holder)| (*index, holder.as_str()))
            .collect::<Vec<_>>(),
        [
            (24, "minecraft:ore_copper_large"),
            (25, "minecraft:ore_copper"),
        ]
    );
}

#[test]
fn built_in_overworld_underground_decoration_keeps_global_order_and_biome_selection() {
    const MOUNTAIN_BIOMES: &[&str] = &[
        "minecraft:snowy_slopes",
        "minecraft:windswept_gravelly_hills",
        "minecraft:grove",
        "minecraft:windswept_hills",
        "minecraft:windswept_forest",
        "minecraft:meadow",
        "minecraft:jagged_peaks",
        "minecraft:stony_peaks",
        "minecraft:cherry_grove",
        "minecraft:frozen_peaks",
    ];
    const WORLD_SEED: i64 = 42;
    const CHUNK_X: i32 = 3;
    const CHUNK_Z: i32 = -7;

    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let underground_decoration = |biomes: &[&str]| {
        plan.invocations_for_chunk(WORLD_SEED, CHUNK_X, CHUNK_Z, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 7)
            .collect::<Vec<_>>()
    };

    // `GenerationStep.java:7-18`; `FeatureSorter.java:28-115`;
    // `ChunkGenerator.java:361-394`.
    let all = underground_decoration(possible);
    assert_eq!(
        all.iter()
            .map(|invocation| (invocation.global_index, invocation.placed_feature.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "minecraft:sculk_vein"),
            (1, "minecraft:sculk_patch_deep_dark"),
            (2, "minecraft:dripstone_cluster"),
            (3, "minecraft:pointed_dripstone"),
            (4, "minecraft:ore_infested"),
        ]
    );

    let decoration_seed = super::decoration_seed(WORLD_SEED, CHUNK_X, CHUNK_Z);
    for (invocation, encounter_index) in all.iter().zip([162_i64, 163, 152, 153, 78]) {
        let expected =
            super::expected_feature_seed(decoration_seed, invocation.global_index as i32, 7);
        assert_eq!(invocation.feature_seed, expected);
        assert_ne!(
            invocation.feature_seed,
            super::expected_feature_seed(decoration_seed, encounter_index as i32, 7)
        );
    }

    let empty_biomes = possible
        .iter()
        .copied()
        .filter(|biome| {
            !MOUNTAIN_BIOMES.contains(biome)
                && !matches!(*biome, "minecraft:dripstone_caves" | "minecraft:deep_dark")
        })
        .collect::<Vec<_>>();
    assert_eq!(empty_biomes.len(), 42);
    let empty = Vec::new();
    let mountain = vec![4];
    let dripstone = vec![2, 3];
    let deep_dark = vec![0, 1];
    let groups = [
        (empty_biomes.as_slice(), empty.as_slice()),
        (MOUNTAIN_BIOMES, mountain.as_slice()),
        (&["minecraft:dripstone_caves"][..], dripstone.as_slice()),
        (&["minecraft:deep_dark"][..], deep_dark.as_slice()),
    ];
    let mut covered_biomes = Vec::new();
    for (biomes, expected) in groups {
        for biome in biomes {
            let actual = underground_decoration(&[biome])
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{biome}");
            covered_biomes.push(*biome);
        }
    }
    covered_biomes.sort_unstable();
    let mut expected_biomes = possible.to_vec();
    expected_biomes.sort_unstable();
    assert_eq!(covered_biomes, expected_biomes);

    for biomes in [
        [
            "minecraft:deep_dark",
            "minecraft:dripstone_caves",
            "minecraft:snowy_slopes",
        ],
        [
            "minecraft:snowy_slopes",
            "minecraft:dripstone_caves",
            "minecraft:deep_dark",
        ],
    ] {
        assert_eq!(
            underground_decoration(&biomes)
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>(),
            (0..=4).collect::<Vec<_>>()
        );
    }
}

#[test]
fn built_in_overworld_fluid_springs_keep_global_order_and_biome_selection() {
    const FROZEN_SPRING_BIOMES: &[&str] = &[
        "minecraft:snowy_slopes",
        "minecraft:grove",
        "minecraft:jagged_peaks",
        "minecraft:frozen_peaks",
    ];
    const WORLD_SEED: i64 = 42;
    const CHUNK_X: i32 = 3;
    const CHUNK_Z: i32 = -7;

    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let fluid_springs = |biomes: &[&str]| {
        plan.invocations_for_chunk(WORLD_SEED, CHUNK_X, CHUNK_Z, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 8)
            .collect::<Vec<_>>()
    };

    // `GenerationStep.java:7-18`; `FeatureSorter.java:28-115`;
    // `ChunkGenerator.java:361-394`.
    let all = fluid_springs(possible);
    assert_eq!(
        all.iter()
            .map(|invocation| (invocation.global_index, invocation.placed_feature.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "minecraft:spring_water"),
            (1, "minecraft:spring_lava"),
            (2, "minecraft:spring_lava_frozen"),
        ]
    );

    let decoration_seed = super::decoration_seed(WORLD_SEED, CHUNK_X, CHUNK_Z);
    for (invocation, encounter_index) in all.iter().zip([34_i64, 35, 79]) {
        let expected =
            super::expected_feature_seed(decoration_seed, invocation.global_index as i32, 8);
        assert_eq!(invocation.feature_seed, expected);
        assert_ne!(
            invocation.feature_seed,
            super::expected_feature_seed(decoration_seed, encounter_index as i32, 8)
        );
    }

    let normal_biomes = possible
        .iter()
        .copied()
        .filter(|biome| !FROZEN_SPRING_BIOMES.contains(biome) && *biome != "minecraft:deep_dark")
        .collect::<Vec<_>>();
    assert_eq!(normal_biomes.len(), 49);
    let normal = vec![0, 1];
    let frozen = vec![0, 1, 2];
    let empty = Vec::new();
    let groups = [
        (normal_biomes.as_slice(), normal.as_slice()),
        (FROZEN_SPRING_BIOMES, frozen.as_slice()),
        (&["minecraft:deep_dark"][..], empty.as_slice()),
    ];
    let mut covered_biomes = Vec::new();
    for (biomes, expected) in groups {
        for biome in biomes {
            let actual = fluid_springs(&[biome])
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{biome}");
            covered_biomes.push(*biome);
        }
    }
    covered_biomes.sort_unstable();
    let mut expected_biomes = possible.to_vec();
    expected_biomes.sort_unstable();
    assert_eq!(covered_biomes, expected_biomes);

    for biomes in [
        [
            "minecraft:deep_dark",
            "minecraft:plains",
            "minecraft:snowy_slopes",
        ],
        [
            "minecraft:snowy_slopes",
            "minecraft:plains",
            "minecraft:deep_dark",
        ],
    ] {
        assert_eq!(
            fluid_springs(&biomes)
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>(),
            (0..=2).collect::<Vec<_>>()
        );
    }
}

#[test]
fn built_in_overworld_vegetal_decoration_keeps_global_order_and_biome_selection() {
    const EXPECTED_FEATURES: &[(usize, i64, &str)] = &[
        (0, 36, "minecraft:glow_lichen"),
        (1, 147, "minecraft:trees_windswept_savanna"),
        (2, 145, "minecraft:bamboo"),
        (3, 146, "minecraft:bamboo_vegetation"),
        (4, 143, "minecraft:trees_sparse_jungle"),
        (5, 136, "minecraft:trees_badlands"),
        (6, 121, "minecraft:bamboo_light"),
        (7, 122, "minecraft:trees_jungle"),
        (8, 117, "minecraft:patch_tall_grass"),
        (9, 118, "minecraft:trees_savanna"),
        (10, 119, "minecraft:flower_warm"),
        (11, 123, "minecraft:patch_grass_jungle"),
        (12, 120, "minecraft:patch_grass_savanna"),
        (13, 113, "minecraft:pale_garden_vegetation"),
        (14, 114, "minecraft:pale_moss_patch"),
        (15, 115, "minecraft:pale_garden_flowers"),
        (16, 116, "minecraft:flower_pale_garden"),
        (17, 111, "minecraft:dark_forest_vegetation"),
        (18, 106, "minecraft:flower_forest_flowers"),
        (19, 107, "minecraft:trees_flower_forest"),
        (20, 108, "minecraft:flower_flower_forest"),
        (21, 98, "minecraft:forest_flowers"),
        (22, 109, "minecraft:wildflowers_birch_forest"),
        (23, 142, "minecraft:birch_tall"),
        (24, 110, "minecraft:trees_birch"),
        (25, 99, "minecraft:trees_birch_and_oak_leaf_litter"),
        (26, 90, "minecraft:patch_tall_grass_2"),
        (27, 155, "minecraft:lush_caves_ceiling_vegetation"),
        (28, 156, "minecraft:cave_vines"),
        (29, 157, "minecraft:lush_caves_clay"),
        (30, 158, "minecraft:lush_caves_vegetation"),
        (31, 159, "minecraft:rooted_azalea_tree"),
        (32, 160, "minecraft:spore_blossom"),
        (33, 161, "minecraft:classic_vines_cave_feature"),
        (34, 141, "minecraft:patch_sunflower"),
        (35, 94, "minecraft:patch_grass_meadow"),
        (36, 95, "minecraft:flower_meadow"),
        (37, 96, "minecraft:trees_meadow"),
        (38, 97, "minecraft:wildflowers_meadow"),
        (39, 88, "minecraft:trees_windswept_forest"),
        (40, 84, "minecraft:patch_large_fern"),
        (41, 140, "minecraft:trees_old_growth_pine_taiga"),
        (42, 102, "minecraft:trees_old_growth_spruce_taiga"),
        (43, 85, "minecraft:trees_taiga"),
        (44, 83, "minecraft:trees_grove"),
        (45, 81, "minecraft:trees_windswept_hills"),
        (46, 80, "minecraft:trees_snowy"),
        (47, 76, "minecraft:trees_mangrove"),
        (48, 64, "minecraft:trees_swamp"),
        (49, 65, "minecraft:flower_swamp"),
        (50, 46, "minecraft:trees_water"),
        (51, 82, "minecraft:patch_bush"),
        (52, 91, "minecraft:trees_plains"),
        (53, 92, "minecraft:flower_plains"),
        (54, 93, "minecraft:patch_grass_plain"),
        (55, 148, "minecraft:flower_cherry"),
        (56, 149, "minecraft:trees_cherry"),
        (57, 47, "minecraft:flower_default"),
        (58, 103, "minecraft:patch_grass_taiga"),
        (59, 100, "minecraft:patch_grass_forest"),
        (60, 86, "minecraft:patch_grass_taiga_2"),
        (61, 66, "minecraft:patch_grass_normal"),
        (62, 67, "minecraft:patch_dead_bush"),
        (63, 104, "minecraft:brown_mushroom_old_growth"),
        (64, 105, "minecraft:red_mushroom_old_growth"),
        (65, 68, "minecraft:patch_waterlily"),
        (66, 69, "minecraft:brown_mushroom_swamp"),
        (67, 70, "minecraft:red_mushroom_swamp"),
        (68, 48, "minecraft:patch_grass_badlands"),
        (69, 132, "minecraft:patch_dry_grass_desert"),
        (70, 133, "minecraft:patch_dead_bush_2"),
        (71, 127, "minecraft:patch_dry_grass_badlands"),
        (72, 128, "minecraft:patch_dead_bush_badlands"),
        (73, 49, "minecraft:brown_mushroom_normal"),
        (74, 50, "minecraft:red_mushroom_normal"),
        (75, 134, "minecraft:patch_sugar_cane_desert"),
        (76, 129, "minecraft:patch_sugar_cane_badlands"),
        (77, 112, "minecraft:patch_leaf_litter"),
        (78, 71, "minecraft:patch_sugar_cane_swamp"),
        (79, 37, "minecraft:mushroom_island_vegetation"),
        (80, 38, "minecraft:brown_mushroom_taiga"),
        (81, 39, "minecraft:red_mushroom_taiga"),
        (82, 51, "minecraft:patch_pumpkin"),
        (83, 135, "minecraft:patch_cactus_desert"),
        (84, 130, "minecraft:patch_cactus_decorated"),
        (85, 72, "minecraft:patch_firefly_bush_swamp"),
        (86, 73, "minecraft:patch_firefly_bush_near_water_swamp"),
        (87, 74, "minecraft:seagrass_swamp"),
        (88, 40, "minecraft:patch_sugar_cane"),
        (89, 41, "minecraft:patch_firefly_bush_near_water"),
        (90, 137, "minecraft:seagrass_river"),
        (91, 124, "minecraft:vines"),
        (92, 144, "minecraft:patch_melon_sparse"),
        (93, 125, "minecraft:patch_melon"),
        (94, 89, "minecraft:patch_berry_common"),
        (95, 87, "minecraft:patch_berry_rare"),
        (96, 60, "minecraft:warm_ocean_vegetation"),
        (97, 59, "minecraft:seagrass_warm"),
        (98, 61, "minecraft:sea_pickle"),
        (99, 57, "minecraft:seagrass_deep_warm"),
        (100, 58, "minecraft:kelp_warm"),
        (101, 56, "minecraft:seagrass_normal"),
        (102, 55, "minecraft:seagrass_deep"),
        (103, 54, "minecraft:seagrass_cold"),
        (104, 52, "minecraft:seagrass_deep_cold"),
        (105, 53, "minecraft:kelp_cold"),
    ];
    const BIOME_SELECTIONS: &[(&str, &[usize])] = &[
        ("minecraft:mushroom_fields", &[0, 79, 80, 81, 88, 89]),
        (
            "minecraft:deep_frozen_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:frozen_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:deep_cold_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 104, 105],
        ),
        (
            "minecraft:cold_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 103, 105],
        ),
        (
            "minecraft:deep_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 102, 105],
        ),
        (
            "minecraft:ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 101, 105],
        ),
        (
            "minecraft:deep_lukewarm_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 99, 100],
        ),
        (
            "minecraft:lukewarm_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 97, 100],
        ),
        (
            "minecraft:warm_ocean",
            &[0, 50, 57, 68, 73, 74, 82, 88, 89, 96, 97, 98],
        ),
        ("minecraft:stony_shore", &[0, 57, 68, 73, 74, 82, 88, 89]),
        (
            "minecraft:swamp",
            &[0, 48, 49, 61, 62, 65, 66, 67, 73, 74, 78, 82, 85, 86, 87],
        ),
        ("minecraft:mangrove_swamp", &[0, 47, 61, 62, 65, 87, 89]),
        ("minecraft:snowy_slopes", &[0, 82]),
        (
            "minecraft:snowy_plains",
            &[0, 46, 57, 68, 73, 74, 82, 88, 89],
        ),
        ("minecraft:snowy_beach", &[0, 57, 68, 73, 74, 82, 88, 89]),
        (
            "minecraft:windswept_gravelly_hills",
            &[0, 45, 51, 57, 68, 73, 74, 82, 88, 89],
        ),
        ("minecraft:grove", &[0, 44, 82]),
        (
            "minecraft:windswept_hills",
            &[0, 45, 51, 57, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:snowy_taiga",
            &[0, 40, 43, 57, 60, 80, 81, 82, 88, 89, 95],
        ),
        (
            "minecraft:windswept_forest",
            &[0, 39, 51, 57, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:taiga",
            &[0, 40, 43, 57, 60, 80, 81, 82, 88, 89, 94],
        ),
        (
            "minecraft:plains",
            &[0, 26, 51, 52, 53, 54, 73, 74, 82, 88, 89],
        ),
        ("minecraft:meadow", &[0, 26, 35, 36, 37, 38]),
        ("minecraft:beach", &[0, 57, 68, 73, 74, 82, 88, 89]),
        (
            "minecraft:forest",
            &[0, 21, 25, 51, 57, 59, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:old_growth_spruce_taiga",
            &[0, 40, 42, 57, 58, 62, 63, 64, 73, 74, 82, 88, 89, 94],
        ),
        (
            "minecraft:flower_forest",
            &[0, 18, 19, 20, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:birch_forest",
            &[0, 21, 22, 24, 51, 57, 59, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:dark_forest",
            &[0, 17, 21, 57, 59, 73, 74, 77, 82, 88, 89],
        ),
        (
            "minecraft:pale_garden",
            &[0, 13, 14, 15, 16, 59, 82, 88, 89],
        ),
        (
            "minecraft:savanna_plateau",
            &[0, 8, 9, 10, 12, 73, 74, 82, 88, 89],
        ),
        ("minecraft:savanna", &[0, 8, 9, 10, 12, 73, 74, 82, 88, 89]),
        (
            "minecraft:jungle",
            &[0, 6, 7, 10, 11, 73, 74, 82, 88, 89, 91, 93],
        ),
        (
            "minecraft:badlands",
            &[0, 68, 71, 72, 73, 74, 76, 82, 84, 89],
        ),
        ("minecraft:desert", &[0, 57, 68, 69, 70, 73, 74, 75, 82, 83]),
        (
            "minecraft:wooded_badlands",
            &[0, 5, 68, 71, 72, 73, 74, 76, 82, 84, 89],
        ),
        ("minecraft:jagged_peaks", &[0]),
        ("minecraft:stony_peaks", &[0]),
        (
            "minecraft:frozen_river",
            &[0, 50, 51, 57, 68, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:river",
            &[0, 50, 51, 57, 68, 73, 74, 82, 88, 89, 90],
        ),
        ("minecraft:ice_spikes", &[0, 46, 57, 68, 73, 74, 82, 88, 89]),
        (
            "minecraft:old_growth_pine_taiga",
            &[0, 40, 41, 57, 58, 62, 63, 64, 73, 74, 82, 88, 89, 94],
        ),
        (
            "minecraft:sunflower_plains",
            &[0, 26, 34, 52, 53, 54, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:old_growth_birch_forest",
            &[0, 21, 22, 23, 51, 57, 59, 73, 74, 82, 88, 89],
        ),
        (
            "minecraft:sparse_jungle",
            &[0, 4, 10, 11, 73, 74, 82, 88, 89, 91, 92],
        ),
        (
            "minecraft:bamboo_jungle",
            &[0, 2, 3, 10, 11, 73, 74, 82, 88, 89, 91, 93],
        ),
        (
            "minecraft:eroded_badlands",
            &[0, 68, 71, 72, 73, 74, 76, 82, 84, 89],
        ),
        (
            "minecraft:windswept_savanna",
            &[0, 1, 57, 61, 73, 74, 82, 88, 89],
        ),
        ("minecraft:cherry_grove", &[0, 26, 54, 55, 56]),
        ("minecraft:frozen_peaks", &[0]),
        (
            "minecraft:dripstone_caves",
            &[0, 26, 52, 53, 54, 73, 74, 82],
        ),
        ("minecraft:lush_caves", &[0, 26, 27, 28, 29, 30, 31, 32, 33]),
        ("minecraft:deep_dark", &[0, 26, 52, 53, 54, 73, 74, 82]),
    ];
    const WORLD_SEED: i64 = 42;
    const CHUNK_X: i32 = 3;
    const CHUNK_Z: i32 = -7;

    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let vegetal_decoration = |biomes: &[&str]| {
        plan.invocations_for_chunk(WORLD_SEED, CHUNK_X, CHUNK_Z, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 9)
            .collect::<Vec<_>>()
    };

    // `GenerationStep.java:7-18`; `FeatureSorter.java:28-115`;
    // `ChunkGenerator.java:361-394`.
    let all = vegetal_decoration(possible);
    assert_eq!(all.len(), 106);
    assert_eq!(
        all.iter()
            .map(|invocation| (invocation.global_index, invocation.placed_feature.as_str()))
            .collect::<Vec<_>>(),
        EXPECTED_FEATURES
            .iter()
            .map(|(global_index, _, holder)| (*global_index, *holder))
            .collect::<Vec<_>>()
    );
    let local_encounters = possible
        .iter()
        .map(|biome| plan.biome_step_indices.get(*biome).unwrap()[9].len())
        .sum::<usize>();
    assert_eq!(local_encounters, 503);
    assert_eq!(plan.steps[9].features.len(), 106);
    let mut encounter_indices = BTreeMap::new();
    for biome in possible {
        for (step, step_indices) in plan
            .biome_step_indices
            .get(*biome)
            .unwrap()
            .iter()
            .enumerate()
        {
            for &global_index in step_indices {
                let holder = plan.steps[step].features[global_index].as_str();
                let next_index = encounter_indices.len();
                encounter_indices.entry(holder).or_insert(next_index);
            }
        }
    }

    let decoration_seed = super::decoration_seed(WORLD_SEED, CHUNK_X, CHUNK_Z);
    for (invocation, (global_index, encounter_index, holder)) in all.iter().zip(EXPECTED_FEATURES) {
        assert_eq!(invocation.global_index, *global_index);
        assert_eq!(encounter_indices[*holder], *encounter_index as usize);
        assert_eq!(
            invocation.feature_seed,
            super::expected_feature_seed(decoration_seed, *global_index as i32, 9)
        );
        if *global_index as i64 != *encounter_index {
            assert_ne!(
                invocation.feature_seed,
                super::expected_feature_seed(decoration_seed, *encounter_index as i32, 9)
            );
        }
    }

    assert_eq!(BIOME_SELECTIONS.len(), 54);
    let mut covered_biomes = Vec::new();
    for (biome, expected) in BIOME_SELECTIONS {
        let actual = vegetal_decoration(&[biome])
            .into_iter()
            .map(|invocation| invocation.global_index)
            .collect::<Vec<_>>();
        assert_eq!(actual, *expected, "{biome}");
        covered_biomes.push(*biome);
    }
    covered_biomes.sort_unstable();
    let mut expected_biomes = possible.to_vec();
    expected_biomes.sort_unstable();
    assert_eq!(covered_biomes, expected_biomes);

    let expected_mixed = [
        0, 1, 26, 27, 28, 29, 30, 31, 32, 33, 50, 57, 61, 68, 73, 74, 82, 88, 89, 96, 97, 98,
    ];
    for biomes in [
        [
            "minecraft:windswept_savanna",
            "minecraft:lush_caves",
            "minecraft:warm_ocean",
        ],
        [
            "minecraft:warm_ocean",
            "minecraft:lush_caves",
            "minecraft:windswept_savanna",
        ],
    ] {
        assert_eq!(
            vegetal_decoration(&biomes)
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>(),
            expected_mixed
        );
    }
}

#[test]
fn built_in_overworld_top_layer_modification_keeps_global_order_and_biome_selection() {
    const WORLD_SEED: i64 = 42;
    const CHUNK_X: i32 = 3;
    const CHUNK_Z: i32 = -7;

    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let biome_source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    let possible = biome_source.overworld_biome_order().unwrap();
    assert_eq!(possible.len(), 54);
    assert!(!possible.contains(&"minecraft:the_void"));
    let plan = NoiseFeaturesPlan::from_registry(&startup.registries, possible).unwrap();
    let top_layer_modification = |biomes: &[&str]| {
        plan.invocations_for_chunk(WORLD_SEED, CHUNK_X, CHUNK_Z, biomes)
            .unwrap()
            .into_iter()
            .filter(|invocation| invocation.step == 10)
            .collect::<Vec<_>>()
    };

    // `GenerationStep.java:7-18`; `FeatureSorter.java:28-115`;
    // `ChunkGenerator.java:361-394`; `OverworldBiomes.java:48-55`;
    // `OverworldBiomes.java:926-942`; `BiomeDefaultFeatures.java:405-406`.
    let all = top_layer_modification(possible);
    assert_eq!(
        all.iter()
            .map(|invocation| (invocation.global_index, invocation.placed_feature.as_str()))
            .collect::<Vec<_>>(),
        [(0, "minecraft:freeze_top_layer")]
    );
    let local_encounters = possible
        .iter()
        .map(|biome| plan.biome_step_indices.get(*biome).unwrap()[10].len())
        .sum::<usize>();
    assert_eq!(local_encounters, 54);
    assert_eq!(plan.steps[10].features, ["minecraft:freeze_top_layer"]);

    let mut encounter_indices = BTreeMap::new();
    for biome in possible {
        for (step, step_indices) in plan
            .biome_step_indices
            .get(*biome)
            .unwrap()
            .iter()
            .enumerate()
        {
            for &global_index in step_indices {
                let holder = plan.steps[step].features[global_index].as_str();
                let next_index = encounter_indices.len();
                encounter_indices.entry(holder).or_insert(next_index);
            }
        }
    }
    assert_eq!(encounter_indices["minecraft:freeze_top_layer"], 42);

    let decoration_seed = super::decoration_seed(WORLD_SEED, CHUNK_X, CHUNK_Z);
    assert_eq!(
        all[0].feature_seed,
        super::expected_feature_seed(decoration_seed, 0, 10)
    );
    assert_ne!(
        all[0].feature_seed,
        super::expected_feature_seed(decoration_seed, 42, 10)
    );

    for biome in possible {
        assert_eq!(
            top_layer_modification(&[*biome])
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>(),
            [0],
            "{biome}"
        );
    }
    for biomes in [
        [
            "minecraft:mushroom_fields",
            "minecraft:frozen_peaks",
            "minecraft:deep_dark",
        ],
        [
            "minecraft:deep_dark",
            "minecraft:frozen_peaks",
            "minecraft:mushroom_fields",
        ],
    ] {
        assert_eq!(
            top_layer_modification(&biomes)
                .into_iter()
                .map(|invocation| invocation.global_index)
                .collect::<Vec<_>>(),
            [0]
        );
    }
}
