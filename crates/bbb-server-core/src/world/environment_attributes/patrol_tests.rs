use super::*;
use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::DatapackStartupOptions;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use std::sync::Arc;

#[test]
fn patrol_environment_reads_raw_biome_and_preserves_frozen_reload_generation() {
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        datapacks: Some(Arc::new(startup.clone())),
        ..crate::world::ServerWorldConfig::default()
    })
    .unwrap();
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[clock];
    state.total_ticks = 120000;
    world.set_world_clock_state_for_test(clock, state);
    let mushroom = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:mushroom_fields")
        .unwrap();
    let plains = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .unwrap();
    let pos = (4..12)
        .flat_map(|x| (4..12).map(move |z| BlockPos { x, y: 65, z }))
        .find(|pos| {
            fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z)
                != (pos.x >> 2, pos.y >> 2, pos.z >> 2)
        })
        .unwrap();
    let fuzzy = fuzzy_biome_quart_position(biome_zoom_seed(world.cold.seed), pos.x, pos.y, pos.z);
    let chunks = &mut world.level_mut_by_id(&world.require_level_id(&key)).chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    let column = chunks.column_mut(0, 0).unwrap();
    column
        .set_noise_biome(fuzzy.0, fuzzy.1, fuzzy.2, plains)
        .unwrap();
    column
        .set_noise_biome(pos.x >> 2, pos.y >> 2, pos.z >> 2, mushroom)
        .unwrap();
    assert_eq!(
        world.can_pillager_patrol_spawn(&world.require_level_id(&key), pos),
        Some(false)
    );
    world
        .level_mut_by_id(&world.require_level_id(&key))
        .chunks
        .column_mut(0, 0)
        .unwrap()
        .set_noise_biome(pos.x >> 2, pos.y >> 2, pos.z >> 2, plains)
        .unwrap();
    assert_eq!(
        world.can_pillager_patrol_spawn(&world.require_level_id(&key), pos),
        Some(true)
    );
    let prepared = world
        .prepare_noise_reloadable_resources(&Arc::new(startup.clone()))
        .unwrap();
    world.install_noise_reloadable_resources(prepared);
    assert_eq!(
        world.can_pillager_patrol_spawn(&world.require_level_id(&key), pos),
        Some(true)
    );
    assert_eq!(world.world_clock_states_for_test()[clock], state);
}

#[test]
fn patrol_world_keeps_boolean_modifier_argument_until_its_endpoint() {
    let mut config = crate::tests::datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let timeline = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:timeline")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:early_game")
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&timeline.source_json).unwrap();
    value["tracks"]["minecraft:gameplay/can_pillager_patrol_spawn"]["ease"] =
        serde_json::json!("out_back");
    value["tracks"]["minecraft:gameplay/can_pillager_patrol_spawn"]["modifier"] =
        serde_json::json!("and");
    timeline.source_json = serde_json::to_vec(&value).unwrap().into();
    timeline.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
            .unwrap(),
    );
    let mut world = ServerWorld::new(config).unwrap();
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let pos = BlockPos { x: 4, y: 65, z: 4 };
    let plains = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .unwrap();
    let chunks = &mut world
        .levels
        .get_mut(&world.require_level_id(&key))
        .unwrap()
        .chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    chunks
        .column_mut(0, 0)
        .unwrap()
        .set_noise_biome(1, 16, 1, plains)
        .unwrap();
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    for (ticks, expected) in [(0, false), (60_000, false), (120_000, true)] {
        let mut state = world.world_clock_states_for_test()[clock];
        state.total_ticks = ticks;
        world.set_world_clock_state_for_test(clock, state);
        assert_eq!(
            world.can_pillager_patrol_spawn(&world.require_level_id(&key), pos),
            Some(expected)
        );
    }
}
