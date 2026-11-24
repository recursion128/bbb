use super::*;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHEEP_ID};
use bbb_server_types::save::Java26BuiltinDimension;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
fn world() -> ServerWorld {
    world_with_config(crate::tests::datapack_config())
}
fn world_with_config(config: crate::world::ServerWorldConfig) -> ServerWorld {
    let mut world = ServerWorld::new(config).unwrap();
    let chunks = &mut world
        .level_mut_by_id(&world.require_level_id(&key()))
        .chunks;
    for x in -1..=1 {
        for z in -1..=1 {
            chunks.try_ensure_served_full_column(x, z).unwrap();
        }
    }
    let grass = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:grass_block",
        &std::collections::BTreeMap::from([("snowy".into(), "false".into())]),
    )
    .unwrap();
    for x in 0..16 {
        for z in 0..16 {
            chunks.set_live_block_state(x, 63, z, grass).unwrap();
        }
    }
    chunks.flush_live_relights().unwrap();
    world
}
fn runtime<'a>(world: &'a ServerWorld, key: &'a Java26LevelKey) -> OriginalRuntime<'a> {
    let level = world.level_by_id(&world.require_level_id(key));
    let random = chunk_random(world.cold.seed, key.as_str(), (0, 0));
    OriginalRuntime {
        environment: NaturalEnvironment {
            dimension_id: world.require_level_id(key),
            spawn_environment: world.cold.dimension_sky_lights.spawn_context(
                Some("minecraft:overworld"),
                &world.registries,
                &world.world_clocks,
            ),
            chunks: &level.chunks,
            simulation_levels: &level.simulation_levels,
            world_border: level.world_border,
            players: &[],
            alive_players: &[],
            generation_region: true,
            difficulty: Difficulty::Hard,
            dimension_resource_id: key.as_str(),
            registries: &world.registries,
            datapacks: world.cold.config.datapacks.as_deref(),
            world_seed: world.cold.seed,
            biome_zoom_seed: biome_zoom_seed(world.cold.seed),
            game_time: 123,
            overworld_clock_time: 4_800_000,
            halloween: false,
            spawn_dimension: spawn_dimension_facts(
                world.cold.config.datapacks.as_deref(),
                Some("minecraft:overworld"),
                false,
            ),
            items: level.entities.item_publication(),
        },
        has_ceiling: false,
        store: EntityStore::with_random(random.clone(), EntityIdCounter::new()),
        random,
    }
}

#[test]
fn original_chunk_runtime_places_and_saves_owned_creatures_without_live_mutation() {
    let world = world();
    let key = key();
    let mut runtime = runtime(&world, &key);
    let live = world.levels[&world.require_level_id(&key)].entities.ids();
    for type_id in [VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHEEP_ID] {
        let top = runtime.top_position(type_id, 8, 8).unwrap();
        assert!(runtime.placement_allows(type_id, top).unwrap());
        let position = Vec3d {
            x: 8.0,
            y: f64::from(top.y),
            z: 8.0,
        };
        assert!(runtime
            .collision_and_rules_allow(type_id, position)
            .unwrap());
        let mob = runtime.create_mob(type_id).unwrap();
        assert!(runtime
            .finish_mob(mob, position, 91.0, None)
            .unwrap()
            .is_some());
    }
    let records = crate::entity::write_entity_chunk(&runtime.store, (0, 0)).unwrap();
    assert_eq!(records.entities().len(), 2);
    let mut restored = EntityStore::new(2, EntityIdCounter::new());
    assert_eq!(
        crate::entity::read_entity_chunk(&mut restored, &records)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        world.levels[&world.require_level_id(&key)].entities.ids(),
        live
    );
}

#[test]
fn original_chunk_region_difficulty_uses_zero_inhabited_time_and_actual_moon() {
    let world = moon_variant_world();
    let key = key();
    let mut runtime = runtime(&world, &key);
    runtime.environment.difficulty = Difficulty::Normal;
    runtime.environment.overworld_clock_time = 1_000_000;
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    let moon = runtime
        .environment
        .spawn_environment
        .moon_brightness(runtime.environment.chunks, pos)
        .unwrap();
    assert_eq!(moon, 0.0);
    let expected =
        crate::entity::difficulty_special_multiplier(Difficulty::Normal, 1_000_000, 0, moon);
    assert_ne!(
        expected,
        crate::entity::difficulty_special_multiplier(Difficulty::Normal, 1_000_000, 0, 1.0)
    );
    assert_eq!(
        runtime
            .environment
            .spawn_context("minecraft:cow", pos)
            .map(|context| context.special_multiplier),
        Some(expected)
    );
}

#[test]
fn original_chunk_ceiling_scan_and_missing_columns_fail_without_generation() {
    let world = world();
    let key = key();
    let mut runtime = runtime(&world, &key);
    let normal = runtime
        .top_position(VANILLA_ENTITY_TYPE_COW_ID, 8, 8)
        .unwrap();
    runtime.has_ceiling = true;
    let ceiling = runtime
        .top_position(VANILLA_ENTITY_TYPE_COW_ID, 8, 8)
        .unwrap();
    assert!(ceiling.y < normal.y);
    assert!(runtime
        .top_position(VANILLA_ENTITY_TYPE_COW_ID, 100_000, 100_000)
        .is_err());
    assert!(!runtime.environment.chunks.is_loaded(6250, 6250));
}

fn moon_variant_world() -> ServerWorld {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for registry in &mut startup.registries.registries {
        for element in &mut registry.elements {
            let changed = match registry.registry_id.as_str() {
                "minecraft:dimension_type" if element.id == "minecraft:overworld" => {
                    let mut json: serde_json::Value =
                        serde_json::from_slice(&element.source_json).unwrap();
                    json["timelines"] = serde_json::json!([]);
                    element.source_json = serde_json::to_vec(&json).unwrap().into();
                    element.source = Some(
                        bbb_registry_codec::decode_direct_registry_json(&element.source_json)
                            .unwrap(),
                    );
                    let mut json: serde_json::Value =
                        serde_json::from_slice(&element.source_json).unwrap();
                    json["attributes"]["minecraft:visual/moon_phase"] =
                        serde_json::json!("new_moon");
                    element.source_json = serde_json::to_vec(&json).unwrap().into();
                    element.source = Some(
                        bbb_registry_codec::decode_direct_registry_json(&element.source_json)
                            .unwrap(),
                    );
                    true
                }
                "minecraft:worldgen/biome" => {
                    let mut json: serde_json::Value =
                        serde_json::from_slice(&element.source_json).unwrap();
                    if let Some(attributes) = json
                        .get_mut("attributes")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        attributes.remove("minecraft:visual/moon_phase");
                    }
                    element.source_json = serde_json::to_vec(&json).unwrap().into();
                    element.source = Some(
                        bbb_registry_codec::decode_direct_registry_json(&element.source_json)
                            .unwrap(),
                    );
                    true
                }
                "minecraft:pig_variant" => {
                    let moon = if element.id == "minecraft:warm" {
                        1.0
                    } else {
                        0.0
                    };
                    let mut json: serde_json::Value =
                        serde_json::from_slice(&element.source_json).unwrap();
                    json["spawn_conditions"] = if element.id == "minecraft:temperate" {
                        serde_json::json!([{"priority": -2}])
                    } else {
                        let conditional = serde_json::json!({"priority": -1, "condition": {"type": "minecraft:moon_brightness", "range": {"min": moon, "max": moon}}});
                        serde_json::json!([
                            {"priority": -2},
                            conditional.clone(),
                            conditional,
                            {"priority": -2}
                        ])
                    };
                    element.source_json = serde_json::to_vec(&json).unwrap().into();
                    element.source = Some(
                        bbb_registry_codec::decode_direct_registry_json(&element.source_json)
                            .unwrap(),
                    );
                    true
                }
                _ => false,
            };
            if changed {
                element.source_json = serde_json::to_vec(&element.json()).unwrap().into();
                element.codec_value = None;
            }
        }
    }
    world_with_config(config)
}

#[test]
fn original_chunk_pig_uses_empty_attribute_reader_variant_and_preserves_saved_variant() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    use bbb_protocol::NbtSummaryValue;
    let world = moon_variant_world();
    let key = key();
    let mut runtime = runtime(&world, &key);
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    let pig_variants = |environment: &NaturalEnvironment<'_>| -> Vec<String> {
        environment
            .spawn_context("minecraft:pig", pos)
            .unwrap()
            .variants
            .into_iter()
            .filter(|(registry, ..)| *registry == "minecraft:pig_variant")
            .map(|(_, id, _)| id.to_owned())
            .collect()
    };
    assert_eq!(
        pig_variants(&runtime.environment),
        ["minecraft:warm", "minecraft:warm"]
    );
    runtime.environment.generation_region = false;
    assert_eq!(
        pig_variants(&runtime.environment),
        ["minecraft:cold", "minecraft:cold"]
    );
    runtime.environment.generation_region = true;
    let mob = runtime.create_mob(VANILLA_ENTITY_TYPE_PIG_ID).unwrap();
    assert!(runtime
        .finish_mob(
            mob,
            Vec3d {
                x: 8.0,
                y: 64.0,
                z: 8.0
            },
            0.0,
            None
        )
        .unwrap()
        .is_some());
    let records = crate::entity::write_entity_chunk(&runtime.store, (0, 0)).unwrap();
    let assert_variant = |records: &bbb_server_persistence::Java26EntityChunkData,
                          expected: &str| {
        assert_eq!(records.entities().len(), 1);
        assert!(records
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .any(|entry| entry.name == "variant"
                && entry.value == NbtSummaryValue::String(expected.into())));
    };
    assert_variant(&records, "minecraft:warm");
    let mut restored = EntityStore::new(42, EntityIdCounter::new());
    assert_eq!(
        crate::entity::read_entity_chunk(&mut restored, &records)
            .unwrap()
            .len(),
        1
    );
    assert_variant(
        &crate::entity::write_entity_chunk(&restored, (0, 0)).unwrap(),
        "minecraft:warm",
    );
    let mut ordinary_store = EntityStore::new(43, EntityIdCounter::new());
    let ordinary = create_natural_spawn_root(
        &mut ordinary_store,
        VANILLA_ENTITY_TYPE_PIG_ID,
        Vec3d {
            x: 8.0,
            y: 64.0,
            z: 8.0,
        },
    )
    .unwrap();
    runtime.environment.generation_region = false;
    finalize_and_publish_natural_spawn(
        &mut NaturalSpawnLevel {
            environment: &runtime.environment,
            store: &mut ordinary_store,
        },
        ordinary,
        SpawnReason::Natural,
        None,
        &runtime.random,
    )
    .unwrap();
    assert_variant(
        &crate::entity::write_entity_chunk(&ordinary_store, (0, 0)).unwrap(),
        "minecraft:cold",
    );
}

#[test]
fn original_chunk_slime_uses_default_probability_and_refuses_the_surface() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID;
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for registry in &mut startup.registries.registries {
        if registry.registry_id == "minecraft:dimension_type" {
            let overworld = registry
                .elements
                .iter_mut()
                .find(|element| element.id == "minecraft:overworld")
                .unwrap();
            let mut json: serde_json::Value =
                serde_json::from_slice(&overworld.source_json).unwrap();
            json["timelines"] = serde_json::json!([]);
            overworld.source_json = serde_json::to_vec(&json).unwrap().into();
            overworld.source = Some(
                bbb_registry_codec::decode_direct_registry_json(&overworld.source_json).unwrap(),
            );
            overworld.source_json = serde_json::to_vec(&overworld.json()).unwrap().into();
            overworld.codec_value = None;
        }
        if registry.registry_id == "minecraft:worldgen/biome" {
            for element in &mut registry.elements {
                let mut json: serde_json::Value =
                    serde_json::from_slice(&element.source_json).unwrap();
                json["attributes"]["minecraft:gameplay/surface_slime_spawn_chance"] =
                    serde_json::json!(1.0);
                element.source = Some(
                    bbb_registry_codec::decode_direct_registry_json(
                        &serde_json::to_vec(&json).unwrap(),
                    )
                    .unwrap(),
                );
                element.source_json = serde_json::to_vec(&element.json()).unwrap().into();
                element.codec_value = None;
            }
        }
    }
    let mut world = world_with_config(config);
    let key = key();
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    let q = fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z);
    let swamp = world
        .registries
        .registry_element_index(BIOME_REGISTRY, "minecraft:swamp")
        .unwrap();
    world
        .level_mut_by_id(&world.require_level_id(&key))
        .chunks
        .column_mut(q.0 >> 2, q.2 >> 2)
        .unwrap()
        .set_noise_biome(q.0 & 3, q.1, q.2 & 3, swamp)
        .unwrap();
    let mut runtime = runtime(&world, &key);
    assert_eq!(
        runtime
            .environment
            .spawn_environment
            .surface_slime_spawn_chance(runtime.environment.chunks, pos),
        Some(1.0)
    );
    assert!(!runtime
        .collision_and_rules_allow(
            VANILLA_ENTITY_TYPE_SLIME_ID,
            Vec3d {
                x: 8.0,
                y: 64.0,
                z: 8.0
            }
        )
        .unwrap());
    assert!(runtime.store.ids().is_empty());
}
