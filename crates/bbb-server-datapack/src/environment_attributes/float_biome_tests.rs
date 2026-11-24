use super::*;
use crate::dynamic_registry::RawRegistryElement;
use bbb_facts::environment_attribute_types::vanilla_environment_attribute;
use serde_json::{json, Value};

const SURFACE_SLIME: &str = "minecraft:gameplay/surface_slime_spawn_chance";
fn sample(
    env: &DimensionFloatAttribute,
    biome: &str,
    clock: impl Fn(&str) -> Option<i64>,
) -> Option<f32> {
    env.sample_unclamped(Some(biome), clock)
        .map(|value| env.sanitize(value))
}
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
fn compile(s: &DatapackStartup) -> DimensionFloatAttribute {
    let sources = EnvironmentAttributeSources::new(s).unwrap();
    DimensionFloatAttribute::from_attribute(
        &sources,
        "minecraft:overworld",
        vanilla_environment_attribute(SURFACE_SLIME).unwrap(),
    )
    .unwrap()
}
#[test]
fn builtin_moon_max_and_nether_default() {
    let s = startup();
    let env = compile(&s);
    let phases = [0.5, 0.375, 0.25, 0.125, 0.0, 0.125, 0.25, 0.375];
    for day in -9_i64..10 {
        assert_eq!(
            sample(&env, "minecraft:swamp", |_| Some(day * 24000)),
            Some(phases[day.rem_euclid(8) as usize])
        );
    }
    let sources = EnvironmentAttributeSources::new(&s).unwrap();
    let nether = DimensionFloatAttribute::from_attribute(
        &sources,
        "minecraft:the_nether",
        vanilla_environment_attribute(SURFACE_SLIME).unwrap(),
    )
    .unwrap();
    assert_eq!(sample(&nether, "", |_| None), Some(0.0));
    assert_eq!(sample(&env, "", |_| None), None);
}

fn biome(s: &mut DatapackStartup, value: Value) {
    let entry = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:plains")
        .unwrap();
    let mut json: Value = serde_json::from_slice(&entry.source_json).unwrap();
    json["attributes"] = value;
    entry.source_json = serde_json::to_vec(&json).unwrap().into();
    entry.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&entry.source_json).unwrap());
}

#[test]
fn ordered_layers_sanitize_only_after_all_modifiers() {
    let mut s = startup();
    dimension(&mut s, json!({SURFACE_SLIME:0.8}), json!([]));
    biome(
        &mut s,
        json!({SURFACE_SLIME:{"modifier":"add","argument":0.7}}),
    );
    let env = compile(&s);
    assert_eq!(
        env.sample_unclamped(Some("minecraft:plains"), |_| None),
        Some(1.5)
    );
    assert_eq!(env.sample_unclamped(None, |_| None), Some(0.8));
    assert_eq!(sample(&env, "minecraft:plains", |_| None), Some(1.0));
    assert_eq!(sample(&env, "unknown", |_| None), None);
    timeline(
        &mut s,
        "test:a",
        json!({"clock":"test:clock","tracks":{SURFACE_SLIME:{"modifier":"subtract","keyframes":[{"ticks":0,"value":0.75}]}}}),
    );
    timeline(
        &mut s,
        "test:b",
        json!({"clock":"test:clock","tracks":{SURFACE_SLIME:{"modifier":"multiply","keyframes":[{"ticks":0,"value":0.5}]}}}),
    );
    dimension(
        &mut s,
        json!({SURFACE_SLIME:0.8}),
        json!(["test:a", "test:b"]),
    );
    assert_eq!(
        sample(&compile(&s), "minecraft:plains", |_| Some(0)),
        Some(0.375)
    );
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .insert(
            "test:ordered".into(),
            vec!["test:b".into(), "test:a".into()],
        );
    dimension(&mut s, json!({SURFACE_SLIME:0.8}), json!("#test:ordered"));
    assert_eq!(
        sample(&compile(&s), "minecraft:plains", |_| Some(0)),
        Some(0.0)
    );
}

#[test]
fn custom_clock_easing_period_and_missing_context() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!(["test:eased"]));
    timeline(
        &mut s,
        "test:eased",
        json!({"clock":"test:clock","period_ticks":200,"tracks":{SURFACE_SLIME:{"ease":"out_back","keyframes":[{"ticks":0,"value":0.0},{"ticks":100,"value":1.0}]}}}),
    );
    let env = compile(&s);
    assert_eq!(
        sample(&env, "", |clock| {
            assert_eq!(clock, "test:clock");
            Some(50)
        }),
        Some(1.0)
    );
    assert_eq!(sample(&env, "", |_| Some(-150)), Some(1.0));
    assert_eq!(sample(&env, "", |_| None), None);
    dimension(&mut s, json!({}), json!([]));
    assert_eq!(sample(&compile(&s), "unknown", |_| None), Some(0.0));
    dimension(&mut s, json!({}), json!(["test:absent"]));
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionFloatAttribute::from_attribute(
            &sources,
            "minecraft:overworld",
            vanilla_environment_attribute(SURFACE_SLIME).unwrap()
        ))
        .is_err());
}

#[test]
fn final_sanitize_preserves_nan_and_clamps_infinities() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!([]));
    let mut env = compile(&s);
    for (base, expected) in [
        (f32::NEG_INFINITY, 0.0),
        (f32::INFINITY, 1.0),
        (-1.0, 0.0),
        (2.0, 1.0),
    ] {
        env.base = base;
        assert_eq!(sample(&env, "", |_| None), Some(expected));
    }
    env.base = f32::NAN;
    assert!(sample(&env, "", |_| None).unwrap().is_nan());
}
