use bbb_server_datapack::mob_spawn::{MobSpawnList, MobSpawnerData, WeightedMobSpawnerData};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use super::*;
use crate::redstone::is_redstone_conductor;

mod surface_slime;

fn builtin_startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

fn player(uuid: u128, x: f64, z: f64, spectator: bool) -> SpawnPlayer {
    SpawnPlayer {
        uuid: Uuid::from_u128(uuid),
        position: Vec3d { x, y: 64.0, z },
        spectator,
        creative: false,
        dead_or_dying: false,
        bounds: Aabb {
            min_x: x - 0.3,
            min_y: 64.0,
            min_z: z - 0.3,
            max_x: x + 0.3,
            max_y: 65.8,
            max_z: z + 0.3,
        },
    }
}

#[test]
fn natural_spawn_counter_is_a_radius_eight_union() {
    let first = player(1, 0.0, 0.0, false);
    let second = player(2, 16.0, 0.0, false);
    let counted = natural_spawn_counted_chunks(&[first, second], false);
    assert_eq!(counted.len(), 17 * 18);
    assert!(counted.contains(&(-8, -8)));
    assert!(counted.contains(&(9, 8)));

    let negative = player(3, -0.01, -0.01, false);
    let counted = natural_spawn_counted_chunks(&[negative], false);
    assert!(counted.contains(&(-9, -9)));
    assert!(counted.contains(&(7, 7)));
}

#[test]
fn spectator_counter_participation_is_independent_of_spawn_proximity() {
    let spectator = player(1, 8.0, 8.0, true);
    assert!(natural_spawn_counted_chunks(&[spectator], false).is_empty());
    assert_eq!(
        natural_spawn_counted_chunks(&[spectator], true).len(),
        17 * 17
    );
    assert!(!players_close_to_chunk(&[spectator], 0, 0));
    assert!(player_ids_close_to_chunk(&[spectator], 0, 0).is_empty());
}

#[test]
fn chunk_spawn_proximity_uses_the_strict_128_block_boundary() {
    let inside = player(1, 135.999, 8.0, false);
    let edge = player(2, 136.0, 8.0, false);
    assert!(players_close_to_chunk(&[inside], 0, 0));
    assert!(!players_close_to_chunk(&[edge], 0, 0));
    assert_eq!(player_ids_close_to_chunk(&[inside], 0, 0), [inside.uuid]);
    assert!(player_ids_close_to_chunk(&[edge], 0, 0).is_empty());
}

#[test]
fn live_difficulty_controls_the_filtered_spawn_categories() {
    let startup = builtin_startup();
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    world.cold.mob_spawns = load_mob_spawns(Some(&startup)).unwrap();
    world.admin.set_difficulty(Difficulty::Peaceful);
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let peaceful = world
        .prepare_natural_spawn_tick(&world.require_level_id(&dimension), 17 * 17)
        .unwrap();
    assert!(!peaceful
        .categories
        .contains(&VanillaEntityMobCategory::Monster));

    world.admin.set_difficulty(Difficulty::Hard);
    let hard = world
        .prepare_natural_spawn_tick(&world.require_level_id(&dimension), 17 * 17)
        .unwrap();
    assert!(hard.categories.contains(&VanillaEntityMobCategory::Monster));
}

#[test]
fn natural_spawn_requires_the_entire_chunk_inside_the_live_border() {
    let border = WorldBorder::from_settings(8.0, 8.0, 0.2, 5.0, 5, 300, 32.0, 0, 32.0, 0);
    assert!(world_border_contains_chunk(border, (0, 0), 0));
    assert!(!world_border_contains_chunk(border, (-1, 0), 0));
    assert!(!world_border_contains_chunk(border, (1, 0), 0));

    let mut levels = SimulationChunkLevels::default();
    levels.rebuild([
        ((0, 0), super::super::CHUNK_ENTITY_TICKING_LEVEL),
        ((1, 0), super::super::CHUNK_ENTITY_TICKING_LEVEL),
    ]);
    assert!(candidate_chunk_allows_spawning(
        &levels,
        border,
        0,
        (0, 0),
        (0, 0)
    ));
    assert!(!candidate_chunk_allows_spawning(
        &levels,
        border,
        0,
        (0, 0),
        (1, 0)
    ));
}

#[test]
fn random_spawn_height_does_not_add_twice_to_heightmap_first_available() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let chunks = &mut world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    let first_available = chunks
        .final_heightmap_first_available(0, 0, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 0, 0)
        .unwrap();
    for seed in 0..1_000 {
        let position =
            random_position_within(chunks, (0, 0), &GameRandom::with_seed(seed)).unwrap();
        assert!(position.y <= first_available);
    }
}

#[test]
fn initial_position_projects_air_at_the_exclusive_build_height() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let chunks = &mut world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    let max_y = chunks.min_y() + i32::try_from(chunks.section_count()).unwrap() * 16;
    let projected = block_state(
        chunks,
        BlockPos {
            x: 0,
            y: max_y,
            z: 0,
        },
    );
    assert_eq!(projected, Some(chunks.air_state_id()));
    assert!(!is_redstone_conductor(projected.unwrap()));
}

#[test]
fn weighted_spawn_selection_uses_the_retained_integer_weights() {
    let zombie = MobSpawnerData {
        entity_type_id: 1,
        entity_type: "minecraft:zombie".to_owned(),
        min_count: 1,
        max_count: 4,
    };
    let cow = MobSpawnerData {
        entity_type_id: 2,
        entity_type: "minecraft:cow".to_owned(),
        min_count: 4,
        max_count: 4,
    };
    let list = MobSpawnList {
        entries: vec![
            WeightedMobSpawnerData {
                data: zombie,
                weight: 1,
            },
            WeightedMobSpawnerData {
                data: cow.clone(),
                weight: 3,
            },
        ],
        total_weight: 4,
    };
    // Weights 1:3: about 3 000 cows in 4 000 picks (sd ~27).
    let random = GameRandom::with_seed(0);
    let cows = (0..4_000)
        .filter(|_| select_weighted_spawn(&list, &random) == Some(&cow))
        .count();
    assert!((2_850..=3_150).contains(&cows), "{cows}");
    assert!(select_weighted_spawn(&MobSpawnList::default(), &GameRandom::with_seed(0)).is_none());
}

#[test]
fn startup_retains_the_builtin_biome_spawn_catalog() {
    let startup = builtin_startup();
    let catalog = load_mob_spawns(Some(&startup)).unwrap().unwrap();
    let plains = catalog.biome("minecraft:plains").unwrap();
    assert!(!plains
        .spawns(VanillaEntityMobCategory::Creature)
        .entries
        .is_empty());
    assert!(load_mob_spawns(None).unwrap().is_none());
}

#[test]
fn fuzzy_spawn_lists_and_rough_spawn_costs_sample_distinct_biomes() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let plains = world
        .registries
        .registry_element_index(BIOME_REGISTRY, "minecraft:plains")
        .unwrap();
    let desert = world
        .registries
        .registry_element_index(BIOME_REGISTRY, "minecraft:desert")
        .unwrap();
    let zoom_seed = biome_zoom_seed(world.cold.seed);
    let position = (4..12)
        .flat_map(|x| (4..12).flat_map(move |z| (64..68).map(move |y| BlockPos { x, y, z })))
        .find(|position| {
            fuzzy_biome_quart_position(zoom_seed, position.x, position.y, position.z)
                != (
                    position.x.div_euclid(4),
                    position.y.div_euclid(4),
                    position.z.div_euclid(4),
                )
        })
        .expect("the fuzzy zoom selects a neighboring quart in this interior sample");
    let fuzzy = fuzzy_biome_quart_position(zoom_seed, position.x, position.y, position.z);
    let rough = (
        position.x.div_euclid(4),
        position.y.div_euclid(4),
        position.z.div_euclid(4),
    );
    {
        let chunks = &mut world
            .level_mut_by_id(&world.require_level_id(&dimension))
            .chunks;
        chunks.try_ensure_served_full_column(0, 0).unwrap();
        for (quart, biome) in [(rough, plains), (fuzzy, desert)] {
            chunks
                .column_mut(quart.0.div_euclid(4), quart.2.div_euclid(4))
                .unwrap()
                .set_noise_biome(quart.0.rem_euclid(4), quart.1, quart.2.rem_euclid(4), biome)
                .unwrap();
        }
    }
    let chunks = &world
        .level_by_id(&world.require_level_id(&dimension))
        .chunks;

    assert_eq!(
        rough_biome_resource_id(&world.registries, chunks, (0, 0), position),
        Some("minecraft:plains")
    );
    assert_eq!(
        fuzzy_biome_resource_id(&world.registries, chunks, zoom_seed, position),
        Some("minecraft:desert")
    );
}

#[test]
fn rough_spawn_cost_retains_the_starting_chunk_after_candidate_drift() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let plains = world
        .registries
        .registry_element_index(BIOME_REGISTRY, "minecraft:plains")
        .unwrap();
    let desert = world
        .registries
        .registry_element_index(BIOME_REGISTRY, "minecraft:desert")
        .unwrap();
    let position = BlockPos { x: 16, y: 64, z: 0 };
    {
        let chunks = &mut world
            .level_mut_by_id(&world.require_level_id(&dimension))
            .chunks;
        for chunk_x in 0..=1 {
            chunks.try_ensure_served_full_column(chunk_x, 0).unwrap();
        }
        for (chunk_x, biome) in [(0, plains), (1, desert)] {
            chunks
                .column_mut(chunk_x, 0)
                .unwrap()
                .set_noise_biome(0, position.y.div_euclid(4), 0, biome)
                .unwrap();
        }
    }
    let chunks = &world
        .level_by_id(&world.require_level_id(&dimension))
        .chunks;

    assert_eq!(
        rough_biome_resource_id(&world.registries, chunks, (0, 0), position),
        Some("minecraft:plains")
    );
    assert_eq!(
        rough_biome_resource_id(&world.registries, chunks, (1, 0), position),
        Some("minecraft:desert")
    );
}

#[test]
fn spawn_dimension_facts_read_retained_dimension_type_ambient_light() {
    let startup = builtin_startup();
    let facts = spawn_dimension_facts(Some(&startup), Some("minecraft:the_end"), false)
        .expect("the built-in End dimension type remains available");
    assert_eq!(facts.ambient_light, 0.25);
}

#[test]
fn production_runtime_publishes_a_natural_creature_into_the_level_store() {
    let world = spawn_one_creature(builtin_startup());
    assert!(!world
        .stored_entity_ids_for_test(bbb_server_types::save::Java26BuiltinDimension::Overworld)
        .is_empty());
}

#[test]
fn production_natural_cat_uses_loaded_sound_registry_before_publication() {
    let mut startup = builtin_startup();
    let biomes = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == BIOME_REGISTRY)
        .unwrap();
    for biome in &mut biomes.elements {
        let mut json: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
        json["spawners"]["creature"] = serde_json::json!([
            {"type": "minecraft:cat", "weight": 1, "minCount": 1, "maxCount": 1}
        ]);
        biome.source_json = serde_json::to_vec(&json).unwrap().into();
        biome.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&biome.source_json).unwrap());
        biome.source_json = serde_json::to_vec(&biome.json()).unwrap().into();
        biome.codec_value = None;
    }
    let mut world = spawn_one_creature(startup);
    let level =
        world.level_mut_by_id(&bbb_server_types::save::Java26BuiltinDimension::Overworld.into());
    let ids = level.entities.ids();
    assert!(!ids.is_empty(), "natural cat finalization must publish");
    for id in ids {
        assert_eq!(
            level.entities.get(id).unwrap().entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID
        );
        let bbb_protocol::NbtSummaryValue::Compound(fields) =
            level.entities.command_data(id).unwrap()
        else {
            panic!("cat data");
        };
        assert!(fields.iter().any(|field| field.name == "sound_variant"
            && matches!(&field.value, bbb_protocol::NbtSummaryValue::String(id)
                if id == "minecraft:classic" || id == "minecraft:royal")));
    }
}

fn spawn_one_creature(startup: DatapackStartup) -> ServerWorld {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..super::super::ServerWorldConfig::default()
    })
    .unwrap();
    world.set_test_default_spawn(
        bbb_server_types::save::Java26BuiltinDimension::Nether,
        Vec3d::default(),
    );
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let level = world.level_mut_by_id(&world.require_level_id(&dimension));
    for chunk_x in -1..=1 {
        for chunk_z in -1..=1 {
            level
                .chunks
                .try_ensure_served_full_column(chunk_x, chunk_z)
                .unwrap();
        }
    }
    let grass = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:grass_block",
        &std::collections::BTreeMap::from([("snowy".to_owned(), "false".to_owned())]),
    )
    .unwrap();
    for x in 0..16 {
        for z in 0..16 {
            assert!(level.chunks.set_block_state(x, 63, z, grass).is_some());
        }
    }
    level
        .simulation_levels
        .rebuild([((0, 0), super::super::CHUNK_ENTITY_TICKING_LEVEL)]);

    let spawn_player = player(1, -32.0, 8.0, false);
    let alive_players = vec![spawn_player.position];
    let mut tick = NaturalSpawnTickState {
        players: vec![spawn_player],
        alive_players,
        categories: vec![VanillaEntityMobCategory::Creature],
        state: NaturalSpawnState::new(17 * 17),
        biome_zoom_seed: biome_zoom_seed(world.cold.seed),
        spawn_dimension: None,
        overworld_clock_time: 0,
        halloween: false,
        fatal: false,
    };
    for _ in 0..2_000 {
        world.spawn_naturally_in_chunk(&world.require_level_id(&dimension), (0, 0), &mut tick);
        if !world
            .stored_entity_ids_for_test(bbb_server_types::save::Java26BuiltinDimension::Overworld)
            .is_empty()
        {
            break;
        }
    }

    assert!(!tick.fatal);
    world
}

#[test]
fn natural_spawn_wide_collision_rejects_missing_neighbor_without_loading() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let level = world.level_by_id(&world.require_level_id(&dimension));
    let position = BlockPos {
        x: 12,
        y: 100,
        z: 8,
    };
    let narrow = spawn_aabb(0.6, 1.8, position);
    let wide = spawn_aabb(8.0, 1.8, position);
    assert!(collision_free(
        &level.chunks,
        &level.entities,
        &[],
        narrow,
        false
    ));
    assert!(!collision_free(
        &level.chunks,
        &level.entities,
        &[],
        wide,
        false
    ));
    assert!(box_contains_liquid(&level.chunks, wide));
    assert!(!level.chunks.is_loaded(1, 0));

    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(1, 0)
        .unwrap();
    let level = world.level_by_id(&world.require_level_id(&dimension));
    assert!(collision_free(
        &level.chunks,
        &level.entities,
        &[],
        wide,
        false
    ));
    assert!(!box_contains_liquid(&level.chunks, wide));
}

#[test]
fn natural_spawn_height_outside_is_air_even_without_horizontal_column() {
    let world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    let environment = environment_for_test(&world, &dimension);
    let probe = environment.predicate_level();
    for y in [environment.chunks.min_y() - 1, probe.max_y_exclusive()] {
        let position = BlockPos {
            x: 16_000,
            y,
            z: 16_000,
        };
        assert!(probe.block_is(position, "minecraft:air"));
        assert!(!probe.has_liquid(position));
        assert!(!probe.missing());
    }
    assert!(!environment.chunks.is_loaded(1_000, 1_000));
}

#[test]
fn natural_spawn_pathfinding_light_cost_applies_sky_darken_before_ambient_curve() {
    let mut world = ServerWorld::new(super::super::ServerWorldConfig::default()).unwrap();
    let dimension = Java26LevelKey::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let mut environment = environment_for_test(&world, &dimension);
    let position = BlockPos { x: 8, y: 100, z: 8 };
    assert_eq!(environment.chunks.sky_light_level(8, 100, 8), 15);
    assert_eq!(environment.chunks.block_light_level(8, 100, 8), 0);
    let facts = environment.spawn_dimension.as_mut().unwrap();
    facts.sky_darken = 11;
    facts.ambient_light = 0.25;
    let brightness = 4.0_f32 / 15.0;
    let expected = 0.25 + 0.75 * brightness / (4.0 - 3.0 * brightness) - 0.5;
    assert!((pathfinding_light_cost(&environment, position) - expected).abs() < 1.0e-6);
    environment.spawn_dimension.as_mut().unwrap().sky_darken = 20;
    assert_eq!(pathfinding_light_cost(&environment, position), -0.25);
}

mod player_filters;
