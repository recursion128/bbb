use super::*;
use crate::world::loot_context::entity_scoreboard::LootEntityLevels;
use crate::world::spawner_minecarts::MinecartSpawnerRuntime;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_persistence::Java26WorldClockState;
use bbb_server_types::save::Java26BuiltinDimension;
use serde_json::{json, Value};
use std::sync::Arc;

const ATTRIBUTE: &str = "minecraft:gameplay/surface_slime_spawn_chance";
const CLOCK: &str = "test:surface_slime";

fn edit(startup: &mut DatapackStartup, registry: &str, id: &str, update: impl FnOnce(&mut Value)) {
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
    update(&mut json);
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

fn setup() -> (ServerWorld, Java26LevelKey, BlockPos) {
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
            source_pack: "file/surface-slime".into(),
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
        |v| {
            v["attributes"][ATTRIBUTE] = json!(0.125);
            v["timelines"] = json!(["minecraft:moon"]);
        },
    );
    for (biome, value) in [("minecraft:plains", 0.75), ("minecraft:swamp", 0.0)] {
        edit(startup, BIOME_REGISTRY, biome, |v| {
            v["attributes"][ATTRIBUTE] = json!(value)
        });
    }
    edit(startup, "minecraft:timeline", "minecraft:moon", |v| {
        *v = json!({"clock": CLOCK, "period_ticks": 20, "tracks": {
            ATTRIBUTE: {"modifier": "multiply", "keyframes": [
                {"ticks": 0, "value": 1.0}, {"ticks": 10, "value": 0.0}
            ]}
        }});
    });
    let mut world = ServerWorld::new(config).unwrap();
    let key = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let pos = (4..12)
        .flat_map(|x| (4..12).map(move |z| BlockPos { x, y: 65, z }))
        .find(|p| {
            fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), p.x, p.y, p.z)
                != (p.x >> 2, p.y >> 2, p.z >> 2)
        })
        .unwrap();
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z);
    for (quart, name) in [
        (fuzzy, "minecraft:swamp"),
        ((pos.x >> 2, pos.y >> 2, pos.z >> 2), "minecraft:plains"),
    ] {
        let id = world
            .registries
            .registry_element_index(BIOME_REGISTRY, name)
            .unwrap();
        let chunks = &mut world
            .levels
            .get_mut(&world.require_level_id(&key))
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
    let air = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:air",
        &std::collections::BTreeMap::new(),
    )
    .unwrap();
    assert!(world
        .levels
        .get_mut(&world.require_level_id(&key))
        .unwrap()
        .chunks
        .set_block_state(pos.x, pos.y - 1, pos.z, air)
        .is_some());
    (world, key, pos)
}

fn observe(environment: &SpawnPredicateLevel<'_>, pos: BlockPos) -> Option<f32> {
    environment.surface_slime_spawn_chance(pos)
}

fn both(
    world: &mut ServerWorld,
    key: &Java26LevelKey,
    pos: BlockPos,
    missing_clock: bool,
) -> [Option<f32>; 2] {
    let builtin = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
    let dimension = builtin.require_level_id(key);
    let clocks = if missing_clock {
        &builtin.world_clocks
    } else {
        &world.world_clocks
    };
    let context = world.cold.dimension_sky_lights.spawn_context(
        world
            .cold
            .level_dimension_types
            .get(&dimension)
            .map(String::as_str),
        &world.registries,
        clocks,
    );
    let mut environment = environment_for_test(world, key);
    environment.spawn_environment = context;
    let probe = environment.predicate_level();
    let chance = probe.surface_slime_spawn_chance(pos);
    assert_eq!(probe.missing(), chance.is_none());
    let natural = observe(&probe, pos);
    let level_names = std::sync::Arc::clone(&world.levels.names);
    let (level, others) = world.levels.split_mut(&dimension).unwrap();
    let runtime = MinecartSpawnerRuntime::new(
        &mut level.chunks,
        &mut world.maps,
        world.cold.generate_structures,
        &mut level.entities,
        &[],
        Difficulty::Easy,
        Java26BuiltinDimension::Overworld,
        &world.registries,
        world.cold.config.datapacks.as_deref(),
        &world.datapack_tables.enchantment_registry,
        &world.cold.loot_item_tags,
        None,
        world.cold.seed,
        world.game_time,
        0,
        false,
        None,
        context,
        &mut world.scoreboard,
        level_names,
        LootEntityLevels::Other(others),
        clocks,
        world.cold.dimension_sky_lights.loot_environment_context(
            world
                .cold
                .level_dimension_types
                .get(&dimension)
                .map(String::as_str),
            &world.registries,
            clocks,
            crate::world::environment_attributes::ScalarWeather::from_level(&level.weather),
        ),
        level.sky_darkening.map(i32::from),
        crate::loot_runtime::LootWeather {
            raining: level.weather.is_raining(),
            thundering: level.weather.is_thundering(),
        },
        &world.cold.command_storage,
    );
    [natural, observe(&runtime.predicate_level(), pos)]
}

#[test]
fn surface_slime_both_production_adapters_use_raw_probability_and_fuzzy_surface_tag() {
    let (mut world, key, pos) = setup();
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", CLOCK)
        .unwrap() as usize;
    let overworld = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    world.set_world_clock_state_for_test(
        overworld,
        Java26WorldClockState {
            total_ticks: 10,
            ..Default::default()
        },
    );
    for chance in both(&mut world, &key, pos, false) {
        assert_eq!(chance, Some(0.75));
    }
    world.set_world_clock_state_for_test(
        clock,
        Java26WorldClockState {
            total_ticks: 10,
            ..Default::default()
        },
    );
    for chance in both(&mut world, &key, pos, false) {
        assert_eq!(chance, Some(0.0));
    }
}
