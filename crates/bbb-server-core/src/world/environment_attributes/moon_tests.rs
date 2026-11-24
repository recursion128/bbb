use super::*;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_persistence::{Java26WorldClockState, Java26WorldStorage, RegionCompression};
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::save::{Java26BuiltinDimension, WorldSaveLimits};
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use serde_json::{json, Value};
use std::sync::Arc;

const MOON: &str = "minecraft:visual/moon_phase";
const CLOCK: &str = "test:moon";

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn edit(startup: &mut DatapackStartup, registry: &str, id: &str, change: impl FnOnce(&mut Value)) {
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == registry)
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    change(&mut json);
    element.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&json).unwrap())
            .unwrap(),
    );
    element.source_json = serde_json::to_vec(element.json().unwrap()).unwrap().into();
    element.codec_value = if registry == "minecraft:timeline" {
        let data = bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap();
        let decoded =
            bbb_registry_codec::decode_exact_synchronized_registry_element(registry, &data)
                .unwrap();
        Some(decoded.into_canonical().unwrap())
    } else {
        None
    };
}

fn config(timeline: bool) -> crate::world::ServerWorldConfig {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:world_clock")
        .unwrap()
        .elements
        .push(RawRegistryElement {
            id: CLOCK.into(),
            source_pack: "file/moon-tests".into(),
            source_json: b"{}".to_vec().into(),
            source: Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(json!({}))).unwrap(),
                )
                .unwrap(),
            ),
            codec_value: None,
        });
    edit(
        startup,
        "minecraft:dimension_type",
        "minecraft:overworld",
        |value| {
            value["timelines"] = if timeline {
                json!(["minecraft:moon"])
            } else {
                json!([])
            };
        },
    );
    edit(startup, "minecraft:timeline", "minecraft:moon", |value| {
        *value = json!({"clock": CLOCK, "period_ticks": 20, "tracks": {
            MOON: {"keyframes": [{"ticks": 0, "value": "full_moon"}, {"ticks": 10, "value": "new_moon"}]}
        }});
    });
    for (biome, phase) in [
        ("minecraft:plains", "full_moon"),
        ("minecraft:desert", "new_moon"),
    ] {
        edit(startup, "minecraft:worldgen/biome", biome, |value| {
            value["attributes"][MOON] = json!(phase);
        });
    }
    config
}

fn clock(world: &ServerWorld, id: &str) -> usize {
    world
        .registries
        .registry_element_index("minecraft:world_clock", id)
        .unwrap() as usize
}

fn set_biome(world: &mut ServerWorld, quart: (i32, i32, i32), biome: &str) {
    let id = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", biome)
        .unwrap();
    let chunks = &mut world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks;
    chunks
        .try_ensure_served_full_column(quart.0 >> 2, quart.2 >> 2)
        .unwrap();
    chunks
        .column_mut(quart.0 >> 2, quart.2 >> 2)
        .unwrap()
        .set_noise_biome(quart.0 & 3, quart.1, quart.2 & 3, id)
        .unwrap();
}

fn brightness(world: &ServerWorld, pos: BlockPos) -> Option<f32> {
    world
        .cold
        .dimension_sky_lights
        .spawn_context(
            world
                .cold
                .level_dimension_types
                .get(&world.require_level_id(&key()))
                .map(String::as_str),
            &world.registries,
            &world.world_clocks,
        )
        .moon_brightness(&world.levels[&world.require_level_id(&key())].chunks, pos)
}

#[test]
fn moon_clock_disk_restore_preserves_fraction_rate_pause_and_live_sampling() {
    let config = config(true);
    let root = std::env::temp_dir().join(format!("bbb-moon-clock-restart-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "moon-clock-restart");
    let paths = storage.paths().clone();
    let candidate = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        storage.load_bootstrap().unwrap(),
        &config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let mut world = ServerWorld::restore(config.clone(), &candidate).unwrap();
    let pos = BlockPos { x: 4, y: 65, z: 4 };
    set_biome(&mut world, (1, 16, 1), "minecraft:plains");
    let index = clock(&world, CLOCK);
    let state = Java26WorldClockState {
        total_ticks: 9,
        partial_tick: 0.75,
        rate: 0.5,
        paused: false,
    };
    world.set_world_clock_state_for_test(index, state);
    assert_eq!(brightness(&world, pos), Some(1.0));
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        storage.load_bootstrap().unwrap(),
        &config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let mut restored = ServerWorld::restore(config, &candidate).unwrap();
    assert_eq!(restored.world_clock_states_for_test()[index], state);
    assert_eq!(brightness(&restored, pos), Some(1.0));
    restored.tick(Vec::new());
    assert_eq!(
        restored.world_clock_states_for_test()[index].total_ticks,
        10
    );
    assert_eq!(
        restored.world_clock_states_for_test()[index].partial_tick,
        0.25
    );
    assert_eq!(brightness(&restored, pos), Some(0.0));
    let paused = Java26WorldClockState {
        paused: true,
        ..state
    };
    restored.set_world_clock_state_for_test(index, paused);
    let request = restored
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        storage.load_bootstrap().unwrap(),
        &restored.cold.config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let mut paused_world = ServerWorld::restore(restored.cold.config.clone(), &candidate).unwrap();
    paused_world.tick(Vec::new());
    assert_eq!(paused_world.world_clock_states_for_test()[index], paused);
    assert_eq!(brightness(&paused_world, pos), Some(1.0));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn moon_difficulty_samples_raw_biome_at_each_height_not_fuzzy_or_chunk_origin() {
    let mut world = ServerWorld::new(config(false)).unwrap();
    let index = clock(&world, "minecraft:overworld");
    world.set_world_clock_state_for_test(
        index,
        Java26WorldClockState {
            total_ticks: 2_000_000,
            ..Default::default()
        },
    );
    let pos = (4..12)
        .flat_map(|x| (4..12).map(move |z| BlockPos { x, y: 65, z }))
        .find(|p| {
            fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), p.x, p.y, p.z)
                != (p.x >> 2, p.y >> 2, p.z >> 2)
        })
        .unwrap();
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z);
    set_biome(&mut world, fuzzy, "minecraft:desert");
    set_biome(
        &mut world,
        (pos.x >> 2, pos.y >> 2, pos.z >> 2),
        "minecraft:plains",
    );
    let high = BlockPos {
        y: pos.y + 16,
        ..pos
    };
    set_biome(
        &mut world,
        (high.x >> 2, high.y >> 2, high.z >> 2),
        "minecraft:desert",
    );
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks
        .increment_inhabited_time(0, 0, 3_600_000);
    assert_eq!(brightness(&world, pos), Some(1.0));
    assert_eq!(brightness(&world, high), Some(0.0));
    let low_difficulty = world
        .effective_difficulty_at(&world.require_level_id(&key()), pos)
        .unwrap();
    assert!(
        low_difficulty
            > world
                .effective_difficulty_at(&world.require_level_id(&key()), high)
                .unwrap()
    );
    set_biome(&mut world, fuzzy, "minecraft:plains");
    assert_eq!(
        world.effective_difficulty_at(&world.require_level_id(&key()), pos),
        Some(low_difficulty)
    );
}

#[test]
fn moon_difficulty_missing_full_uses_zero_inputs_without_loading() {
    let mut world = ServerWorld::new(config(false)).unwrap();
    let pos = BlockPos {
        x: 16000,
        y: 65,
        z: 16000,
    };
    let evaluate = |world: &ServerWorld| {
        world
            .cold
            .dimension_sky_lights
            .spawn_context(
                world
                    .cold
                    .level_dimension_types
                    .get(&world.require_level_id(&key()))
                    .map(String::as_str),
                &world.registries,
                &world.world_clocks,
            )
            .effective_difficulty(
                &world.levels[&world.require_level_id(&key())].chunks,
                pos,
                Difficulty::Hard,
                2_000_000,
            )
    };
    let before = world.levels[&world.require_level_id(&key())]
        .chunks
        .column_count();
    assert_eq!(
        evaluate(&world),
        Some(crate::entity::effective_difficulty(
            Difficulty::Hard,
            2_000_000,
            0,
            0.0
        ))
    );
    assert_eq!(
        world.levels[&world.require_level_id(&key())]
            .chunks
            .column_count(),
        before
    );
    assert!(world.levels[&world.require_level_id(&key())]
        .chunks
        .column(pos.x >> 4, pos.z >> 4)
        .is_none());
    set_biome(
        &mut world,
        (pos.x >> 2, pos.y >> 2, pos.z >> 2),
        "minecraft:plains",
    );
    assert_eq!(
        evaluate(&world),
        Some(crate::entity::effective_difficulty(
            Difficulty::Hard,
            2_000_000,
            0,
            1.0
        ))
    );
}
