use super::*;
use crate::environment_attributes::{DatapackStartup, TIMELINE};
use serde_json::{json, Value};
fn startup() -> DatapackStartup {
    static START: std::sync::OnceLock<DatapackStartup> = std::sync::OnceLock::new();
    START
        .get_or_init(|| {
            DatapackStartup::load(&crate::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: crate::registry_data_loader::DatapackLoadLimits::default(),
            })
            .unwrap()
        })
        .clone()
}
use crate::dynamic_registry::RawRegistryElement;

fn dimension(s: &mut DatapackStartup, attributes: Value, timelines: Value) {
    let element = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    value["attributes"] = attributes;
    value["timelines"] = timelines;
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
}
fn timeline(s: &mut DatapackStartup, id: &str, value: Value) {
    let source_json: std::sync::Arc<[u8]> = serde_json::to_vec(&value).unwrap().into();
    let source = bbb_registry_codec::decode_direct_registry_json(&source_json).unwrap();
    s.registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .elements
        .push(RawRegistryElement {
            id: id.into(),
            source_pack: "test".into(),
            source_json,
            source: Some(source),
            codec_value: None,
        });
}
fn compile(s: &DatapackStartup) -> DimensionMoonEnvironment {
    let sources = EnvironmentAttributeSources::new(s).unwrap();
    DimensionMoonEnvironment::from_sources(&sources, "minecraft:overworld").unwrap()
}
#[test]
fn builtin_boundaries_and_negative_periods() {
    let env = compile(&startup());
    let phases = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];
    for day in -10_i64..18 {
        for offset in [0, 1, 23999] {
            assert_eq!(
                env.sample("minecraft:plains", |_| Some(day * 24000 + offset)),
                Some(phases[day.rem_euclid(8) as usize])
            );
        }
    }
    assert_eq!(env.sample("minecraft:plains", |_| None), None);
    assert_eq!(env.sample("", |_| Some(0)), Some(1.0));
}

fn biome(s: &mut DatapackStartup, phase: &str) {
    let element = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:plains")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    value["attributes"] = json!({ATTRIBUTE:phase});
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
}

#[test]
fn dimension_biome_and_ordered_timeline_layers() {
    let mut s = startup();
    dimension(&mut s, json!({ATTRIBUTE:"new_moon"}), json!([]));
    assert_eq!(compile(&s).sample("", |_| None), Some(0.0));
    biome(&mut s, "third_quarter");
    let env = compile(&s);
    assert_eq!(env.sample("minecraft:plains", |_| None), Some(0.5));
    assert_eq!(env.sample("minecraft:desert", |_| None), Some(0.0));
    assert_eq!(env.sample("", |_| None), None);
    for (id, phase) in [("test:a", "full_moon"), ("test:b", "waxing_crescent")] {
        timeline(
            &mut s,
            id,
            json!({"clock":"test:clock","tracks":{ATTRIBUTE:{"keyframes":[{"ticks":0,"value":phase}]}}}),
        );
    }
    dimension(
        &mut s,
        json!({ATTRIBUTE:"new_moon"}),
        json!(["test:a", "test:b"]),
    );
    assert_eq!(
        compile(&s).sample("minecraft:plains", |_| Some(10)),
        Some(0.25)
    );
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .insert("test:order".into(), vec!["test:b".into(), "test:a".into()]);
    dimension(&mut s, json!({ATTRIBUTE:"new_moon"}), json!("#test:order"));
    assert_eq!(
        compile(&s).sample("minecraft:plains", |_| Some(10)),
        Some(1.0)
    );
}

#[test]
fn custom_clock_period_step_and_absolute_clamps() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!(["test:period"]));
    timeline(
        &mut s,
        "test:period",
        json!({"clock":"test:clock","period_ticks":10,"tracks":{ATTRIBUTE:{"keyframes":[{"ticks":2,"value":"new_moon"},{"ticks":7,"value":"full_moon"}]}}}),
    );
    let env = compile(&s);
    for (ticks, value) in [
        (-11, 1.0),
        (-8, 0.0),
        (0, 1.0),
        (2, 0.0),
        (6, 0.0),
        (7, 1.0),
        (12, 0.0),
    ] {
        assert_eq!(
            env.sample("", |clock| {
                assert_eq!(clock, "test:clock");
                Some(ticks)
            }),
            Some(value)
        );
    }
    assert_eq!(env.sample("", |_| None), None);
    dimension(&mut s, json!({}), json!(["test:absolute"]));
    timeline(
        &mut s,
        "test:absolute",
        json!({"clock":"test:clock","tracks":{ATTRIBUTE:{"keyframes":[{"ticks":2,"value":"new_moon"},{"ticks":7,"value":"full_moon"}]}}}),
    );
    for (ticks, value) in [(i64::MIN, 0.0), (6, 0.0), (7, 1.0), (i64::MAX, 1.0)] {
        assert_eq!(compile(&s).sample("", |_| Some(ticks)), Some(value));
    }
}

#[test]
fn defaults_and_invalid_loaded_context() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!([]));
    assert_eq!(compile(&s).sample("unknown:biome", |_| None), Some(1.0));
    for phase in [
        "full_moon",
        "waning_gibbous",
        "third_quarter",
        "waning_crescent",
        "new_moon",
        "waxing_crescent",
        "first_quarter",
        "waxing_gibbous",
    ] {
        dimension(
            &mut s,
            json!({ATTRIBUTE:{"modifier":"override","argument":phase}}),
            json!([]),
        );
        assert!(compile(&s).sample("", |_| None).is_some());
    }
    dimension(&mut s, json!({ATTRIBUTE:"invalid"}), json!([]));
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionMoonEnvironment::from_sources(&sources, "minecraft:overworld"))
        .is_err());
    dimension(&mut s, json!({}), json!(["test:missing"]));
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionMoonEnvironment::from_sources(&sources, "minecraft:overworld"))
        .is_err());
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionMoonEnvironment::from_sources(&sources, "test:missing"))
        .is_err());
}

#[test]
fn eased_step_overshoot_switches_before_endpoint() {
    for (ease, middle) in [("out_back", 1.0), ("constant", 0.0), ("linear", 0.0)] {
        let mut s = startup();
        dimension(&mut s, json!({}), json!(["test:eased"]));
        timeline(
            &mut s,
            "test:eased",
            json!({
                "clock":"test:clock", "tracks":{ATTRIBUTE:{"ease":ease,
                    "keyframes":[{"ticks":0,"value":"new_moon"},{"ticks":100,"value":"full_moon"}]}}
            }),
        );
        let env = compile(&s);
        for (ticks, expected) in [(0, 0.0), (50, middle), (100, 1.0)] {
            assert_eq!(
                env.sample("", |_| Some(ticks)),
                Some(expected),
                "{ease} at {ticks}"
            );
        }
    }
}
