use super::*;
use bbb_registry_codec::registry_codec::CanonicalRegistryCodecValue;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::DatapackStartupOptions;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use serde_json::{json, Value};
use std::sync::Arc;

fn startup() -> DatapackStartup {
    static START: OnceLock<DatapackStartup> = OnceLock::new();
    START
        .get_or_init(|| {
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            })
            .unwrap()
        })
        .clone()
}

fn dimension(startup: &mut DatapackStartup, clock: Option<&str>, fixed: bool) {
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == DIMENSION_TYPE_REGISTRY)
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    value.as_object_mut().unwrap().remove("default_clock");
    if let Some(clock) = clock {
        value["default_clock"] = json!(clock);
    }
    value["has_fixed_time"] = json!(fixed);
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
}

fn timeline(startup: &mut DatapackStartup, id: &str, value: Value) {
    let source_json: Arc<[u8]> = serde_json::to_vec(&value).unwrap().into();
    let source = bbb_registry_codec::decode_direct_registry_json(&source_json).unwrap();
    let network_nbt = bbb_registry_codec::decode_exact_synchronized_registry_element(
        "minecraft:timeline",
        &source,
    )
    .unwrap();
    startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:timeline")
        .unwrap()
        .elements
        .push(RawRegistryElement {
            id: id.into(),
            source_pack: "test".into(),
            source_json,
            source: Some(source),
            codec_value: Some(CanonicalRegistryCodecValue {
                network_nbt: network_nbt.projected_network_nbt().unwrap(),
                references: vec![],
            }),
        });
}

#[test]
fn dimension_time_marker_compiles_all_timelines_and_both_marker_forms() {
    let mut startup = startup();
    dimension(&mut startup, Some("test:custom"), false);
    timeline(
        &mut startup,
        "test:periodic",
        json!({
            "clock":"test:custom", "period_ticks":10,
            "time_markers":{"test:compact":3,"test:full":{"ticks":4,"show_in_commands":true}}
        }),
    );
    timeline(
        &mut startup,
        "test:absolute",
        json!({
            "clock":"test:custom", "time_markers":{"test:absolute":13}
        }),
    );
    let cache = load_sky_lights(Some(&startup)).unwrap();
    let at = |marker, ticks| {
        cache.time_marker_matches("minecraft:overworld", marker, |clock| {
            assert_eq!(clock, "test:custom");
            Some(ticks)
        })
    };
    assert_eq!(at("test:compact", 13), Some(true));
    assert_eq!(at("test:full", 14), Some(true));
    assert_eq!(at("test:compact", -7), Some(false));
    assert_eq!(at("test:absolute", 13), Some(true));
    assert_eq!(at("test:absolute", 23), Some(false));
    assert_eq!(at("test:missing", 13), Some(false));
    assert_eq!(
        cache.time_marker_matches("minecraft:overworld", "test:compact", |_| None),
        Some(false)
    );
    assert_eq!(
        cache.time_marker_matches("test:unknown", "test:compact", |_| Some(13)),
        None
    );
}

fn world() -> ServerWorld {
    ServerWorld::new(crate::world::ServerWorldConfig {
        datapacks: Some(Arc::new(startup())),
        ..crate::world::ServerWorldConfig::default()
    })
    .unwrap()
}

#[test]
fn dimension_time_marker_exact_tick_fraction_pause_skip_and_restore() {
    let mut world = world();
    let dimension = Java26BuiltinDimension::Overworld.into();
    let marker = "minecraft:roll_village_siege";
    let unknown = Java26LevelKey::parse("test:unknown").unwrap();
    assert_eq!(world.level_id(&unknown), None);
    let index = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[index];
    state.total_ticks = 17_999;
    state.partial_tick = 0.0;
    state.rate = 0.5;
    world.set_world_clock_state_for_test(index, state);
    world.tick_world_clocks();
    assert_eq!(
        world.is_at_dimension_time_marker(&dimension, marker),
        Some(false)
    );
    world.tick_world_clocks();
    assert_eq!(
        world.is_at_dimension_time_marker(&dimension, marker),
        Some(true)
    );
    state = world.world_clock_states_for_test()[index];
    state.paused = true;
    world.set_world_clock_state_for_test(index, state);
    let saved = world.world_clock_snapshot_for_test();
    world.tick_world_clocks();
    world.tick_world_clocks();
    assert_eq!(
        world.is_at_dimension_time_marker(&dimension, marker),
        Some(true)
    );
    state.paused = false;
    state.total_ticks = 17_999;
    state.rate = 2.0;
    world.set_world_clock_state_for_test(index, state);
    world.tick_world_clocks();
    assert_eq!(
        world.is_at_dimension_time_marker(&dimension, marker),
        Some(false)
    );
    world.restore_world_clocks(&saved).unwrap();
    assert_eq!(
        world.is_at_dimension_time_marker(&dimension, marker),
        Some(true)
    );
}

#[test]
fn bright_outside_reads_committed_darkening_and_fixed_time() {
    let mut world = world();
    let overworld = Java26BuiltinDimension::Overworld.into();
    world.level_mut_by_id(&overworld).sky_darkening = Some(3);
    assert_eq!(world.is_bright_outside(&overworld), Some(true));
    world.level_mut_by_id(&overworld).sky_darkening = Some(4);
    assert_eq!(world.is_bright_outside(&overworld), Some(false));
    let mut startup = startup();
    dimension(&mut startup, Some("minecraft:overworld"), true);
    world.cold.dimension_sky_lights = load_sky_lights(Some(&startup)).unwrap();
    world.level_mut_by_id(&overworld).sky_darkening = Some(0);
    assert_eq!(world.is_bright_outside(&overworld), Some(false));
}

#[test]
fn dimension_time_marker_cache_replacement_keeps_live_clock_state() {
    let mut world = world();
    let overworld = Java26BuiltinDimension::Overworld.into();
    let mut startup = startup();
    timeline(
        &mut startup,
        "test:reload",
        json!({"clock":"minecraft:overworld","time_markers":{"test:reload":7}}),
    );
    let index = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    let mut state = world.world_clock_states_for_test()[index];
    state.total_ticks = 7;
    world.set_world_clock_state_for_test(index, state);
    assert_eq!(
        world.is_at_dimension_time_marker(&overworld, "test:reload"),
        Some(false)
    );
    world.cold.dimension_sky_lights = load_sky_lights(Some(&startup)).unwrap();
    assert_eq!(
        world.is_at_dimension_time_marker(&overworld, "test:reload"),
        Some(true)
    );
    assert_eq!(world.world_clock_states_for_test()[index], state);
}
