use std::sync::Arc;

use bbb_protocol::entity_types::{VanillaEntityMobCategory, VANILLA_ENTITY_TYPE_PIG_ID};
use serde_json::{json, Value};

use super::*;
use crate::dynamic_registry::{DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement};
use crate::registry_data_loader::DatapackLoadLimits;
use crate::startup::{DatapackStartup, DatapackStartupOptions};

fn element(id: &str, value: &Value) -> RawRegistryElement {
    RawRegistryElement {
        id: id.to_string(),
        source_pack: "test".to_string(),
        source_json: Arc::from(serde_json::to_vec(value).unwrap()),
        source: None,
        codec_value: None,
    }
}

fn generation(biomes: &[(&str, Value)], structures: &[(&str, Value)]) -> DatapackGeneration {
    DatapackGeneration {
        registries: vec![
            LoadedDynamicRegistry {
                registry_id: BIOME_REGISTRY.to_string(),
                elements: biomes
                    .iter()
                    .map(|(id, value)| element(id, value))
                    .collect(),
            },
            LoadedDynamicRegistry {
                registry_id: STRUCTURE_REGISTRY.to_string(),
                elements: structures
                    .iter()
                    .map(|(id, value)| element(id, value))
                    .collect(),
            },
        ],
        worldgen: Default::default(),
    }
}

fn valid_biome() -> Value {
    json!({
        "spawners": {
            "monster": [{
                "type": "minecraft:zombie",
                "weight": 10,
                "minCount": 1,
                "maxCount": 4
            }]
        },
        "spawn_costs": {}
    })
}

fn valid_structure() -> Value {
    json!({"spawn_overrides": {}})
}

fn build(biome: Value, structure: Value) -> Result<DatapackMobSpawnCatalog, MobSpawnCatalogError> {
    DatapackMobSpawnCatalog::from_generation(&generation(
        &[("minecraft:test", biome)],
        &[("minecraft:test", structure)],
    ))
}

#[test]
fn source_catalog_preserves_registry_and_weighted_order_and_queries() {
    let first = json!({
        "creature_spawn_probability": 0.25,
        "spawners": {
            "monster": [
                {"type":"minecraft:zombie", "weight":0, "minCount":1, "maxCount":1},
                {"type":"minecraft:pig", "weight":7, "minCount":2, "maxCount":3},
                {"type":"minecraft:zombie", "weight":5, "minCount":4, "maxCount":4}
            ],
            "ambient": []
        },
        "spawn_costs": {
            "minecraft:zombie": {"energy_budget":-2.5, "charge":0.0}
        }
    });
    let second = json!({"spawners": {}, "spawn_costs": {}});
    let structure = json!({
        "spawn_overrides": {
            "monster": {
                "bounding_box": "piece",
                "spawns": [{"type":"minecraft:item", "weight":0, "minCount":1, "maxCount":1}]
            },
            "creature": {"bounding_box":"full", "spawns":[]}
        }
    });
    let catalog = DatapackMobSpawnCatalog::from_generation(&generation(
        &[("minecraft:z_first", first), ("minecraft:a_second", second)],
        &[
            ("minecraft:z_structure", structure),
            ("minecraft:a_structure", valid_structure()),
        ],
    ))
    .unwrap();

    assert_eq!(
        catalog
            .biomes()
            .iter()
            .map(|biome| biome.biome_id.as_str())
            .collect::<Vec<_>>(),
        ["minecraft:z_first", "minecraft:a_second"]
    );
    assert_eq!(
        catalog
            .structures()
            .iter()
            .map(|structure| structure.structure_id.as_str())
            .collect::<Vec<_>>(),
        ["minecraft:z_structure", "minecraft:a_structure"]
    );
    let biome = catalog.biome("z_first").unwrap();
    assert_eq!(biome.creature_spawn_probability, 0.25);
    let monsters = biome.spawns(VanillaEntityMobCategory::Monster);
    assert_eq!(monsters.total_weight, 12);
    assert_eq!(
        monsters
            .entries
            .iter()
            .map(|entry| (entry.data.entity_type.as_str(), entry.weight))
            .collect::<Vec<_>>(),
        [
            ("minecraft:zombie", 0),
            ("minecraft:pig", 7),
            ("minecraft:zombie", 5)
        ]
    );
    assert_eq!(
        biome.spawn_cost("zombie").unwrap(),
        biome
            .spawn_cost_by_id(biome.spawn_cost("zombie").unwrap().entity_type_id)
            .unwrap()
    );
    assert!(biome.spawn_cost("minecraft:pig").is_none());
    assert_eq!(
        biome.spawns(VanillaEntityMobCategory::Creature),
        &MobSpawnList::default()
    );

    let structure = catalog.structure("z_structure").unwrap();
    let monster = structure
        .override_for(VanillaEntityMobCategory::Monster)
        .unwrap();
    assert_eq!(monster.bounding_box, StructureSpawnBoundingBox::Piece);
    assert_eq!(monster.spawns.total_weight, 0);
    assert_eq!(
        monster.spawns.entries[0].data.entity_type_id,
        VANILLA_ENTITY_TYPE_PIG_ID
    );
    let creature = structure
        .override_for(VanillaEntityMobCategory::Creature)
        .unwrap();
    assert_eq!(creature.bounding_box, StructureSpawnBoundingBox::Full);
    assert!(creature.spawns.entries.is_empty());
    assert!(structure
        .override_for(VanillaEntityMobCategory::Ambient)
        .is_none());
    assert!(catalog.biome("minecraft:missing").is_none());
    assert!(catalog.structure("minecraft:missing").is_none());
}

#[test]
fn missing_categories_share_the_empty_spawn_list() {
    let catalog = build(valid_biome(), valid_structure()).unwrap();
    let biome = catalog.biome("test").unwrap();
    assert!(std::ptr::eq(
        biome.spawns(VanillaEntityMobCategory::Creature),
        biome.spawns(VanillaEntityMobCategory::Ambient)
    ));
}

#[test]
fn builtin_26_1_mob_spawn_catalog_matches_owned_inventory() {
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    let catalog = DatapackMobSpawnCatalog::from_generation(&startup.registries).unwrap();

    assert_eq!(catalog.biomes().len(), 65);
    assert_eq!(
        catalog
            .biomes()
            .iter()
            .flat_map(|biome| &biome.categories)
            .map(|(_, spawns)| spawns.entries.len())
            .sum::<usize>(),
        785
    );
    assert_eq!(
        catalog
            .biomes()
            .iter()
            .map(|biome| biome.costs.len())
            .sum::<usize>(),
        5
    );
    assert_eq!(
        catalog
            .biomes()
            .iter()
            .filter(|biome| biome.creature_spawn_probability != 0.1)
            .count(),
        5
    );
    assert_eq!(catalog.structures().len(), 34);
    assert_eq!(
        catalog
            .structures()
            .iter()
            .map(|structure| structure.overrides.len())
            .sum::<usize>(),
        23
    );
    assert_eq!(
        catalog
            .structures()
            .iter()
            .flat_map(|structure| &structure.overrides)
            .map(|spawn_override| spawn_override.spawns.entries.len())
            .sum::<usize>(),
        9
    );

    let fortress = catalog.structure("minecraft:fortress").unwrap();
    let monsters = fortress
        .override_for(VanillaEntityMobCategory::Monster)
        .unwrap();
    assert_eq!(monsters.bounding_box, StructureSpawnBoundingBox::Piece);
    assert_eq!(
        monsters
            .spawns
            .entries
            .iter()
            .map(|entry| entry.data.entity_type.as_str())
            .collect::<Vec<_>>(),
        [
            "minecraft:blaze",
            "minecraft:zombified_piglin",
            "minecraft:wither_skeleton",
            "minecraft:skeleton",
            "minecraft:magma_cube"
        ]
    );
}

#[test]
fn mob_spawn_catalog_honors_loaded_registry_json_depth_and_width_limits() {
    let mut wide = Vec::with_capacity(5000);
    for idx in 0..5000 {
        wide.push(serde_json::json!(idx));
    }
    let mut deep = serde_json::json!(0);
    for _ in 0..70 {
        deep = serde_json::json!([deep]);
    }
    let mut biome = valid_biome();
    biome["extra_wide"] = serde_json::Value::Array(wide);
    biome["extra_deep"] = deep;
    let gen = generation(
        &[("example:wide_biome", biome)],
        &[("example:empty_structure", valid_structure())],
    );
    let catalog = DatapackMobSpawnCatalog::from_generation(&gen).unwrap();
    assert!(catalog.biome("example:wide_biome").is_some());
}
