use super::*;
use crate::environment_attributes::{DatapackStartup, TIMELINE};
use serde_json::{json, Value};

const ATTRIBUTE: &str = "minecraft:gameplay/can_pillager_patrol_spawn";
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
fn compile(s: &DatapackStartup) -> DimensionBooleanEnvironment {
    let sources = EnvironmentAttributeSources::new(s).unwrap();
    DimensionBooleanEnvironment::from_attribute(
        &sources,
        "minecraft:overworld",
        bbb_facts::environment_attribute_types::vanilla_environment_attribute(ATTRIBUTE).unwrap(),
    )
    .unwrap()
}
#[test]
fn builtin_threshold_biome_and_missing_context() {
    let env = compile(&startup());
    for (ticks, expected) in [(-1, false), (119999, false), (120000, true), (240000, true)] {
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(ticks)),
            Some(expected)
        );
        assert_eq!(
            env.sample("minecraft:mushroom_fields", |_| Some(ticks)),
            Some(false)
        );
    }
    assert_eq!(env.sample("minecraft:plains", |_| None), None);
    assert_eq!(env.sample("test:unknown", |_| Some(120000)), None);
    assert_eq!(env.sample_dimension(|_| Some(120000)), Some(true));
}
#[test]
fn default_and_every_boolean_modifier_truth_table() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!([]));
    assert_eq!(compile(&s).sample("minecraft:plains", |_| None), Some(true));
    for name in ["override", "and", "nand", "or", "nor", "xor", "xnor"] {
        for base in [false, true] {
            for arg in [false, true] {
                dimension(&mut s, json!({ATTRIBUTE:base}), json!(["test:bool"]));
                let value = json!({"clock":"test:clock","tracks":{ATTRIBUTE:{"modifier":name,"keyframes":[{"ticks":0,"value":arg}]}}});
                s.registries
                    .registries
                    .iter_mut()
                    .find(|r| r.registry_id == TIMELINE)
                    .unwrap()
                    .elements
                    .retain(|e| e.id != "test:bool");
                timeline(&mut s, "test:bool", value);
                let expected = match name {
                    "override" => arg,
                    "and" => base && arg,
                    "nand" => !base || !arg,
                    "or" => base || arg,
                    "nor" => !base && !arg,
                    "xor" => base != arg,
                    _ => base == arg,
                };
                assert_eq!(
                    compile(&s).sample("minecraft:plains", |clock| {
                        assert_eq!(clock, "test:clock");
                        Some(7)
                    }),
                    Some(expected),
                    "{name} {base} {arg}"
                );
            }
        }
    }
}
#[test]
fn dimension_biome_then_ordered_tag_tracks() {
    let mut s = startup();
    dimension(&mut s, json!({ATTRIBUTE:true}), json!("#test:order"));
    for (id, op, arg) in [("test:a", "or", true), ("test:b", "xor", true)] {
        timeline(
            &mut s,
            id,
            json!({"clock":"test:clock","tracks":{ATTRIBUTE:{"modifier":op,"keyframes":[{"ticks":0,"value":arg}]}}}),
        );
    }
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .insert("test:order".into(), vec!["test:a".into(), "test:b".into()]);
    assert_eq!(
        compile(&s).sample("minecraft:mushroom_fields", |_| Some(0)),
        Some(false)
    );
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .get_mut("test:order")
        .unwrap()
        .reverse();
    assert_eq!(
        compile(&s).sample("minecraft:mushroom_fields", |_| Some(0)),
        Some(true)
    );
}
#[test]
fn periodic_floor_mod_duplicate_keyframes_and_nonperiod_clamp() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!(["test:period"]));
    timeline(
        &mut s,
        "test:period",
        json!({"clock":"test:clock","period_ticks":10,"tracks":{ATTRIBUTE:{"keyframes":[{"ticks":2,"value":false},{"ticks":2,"value":true},{"ticks":7,"value":false}]}}}),
    );
    let env = compile(&s);
    for (ticks, expected) in [
        (-7, true),
        (-1, false),
        (0, false),
        (1, false),
        (2, true),
        (6, true),
        (7, false),
        (12, true),
    ] {
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(ticks)),
            Some(expected),
            "{ticks}"
        );
    }
    dimension(&mut s, json!({}), json!(["test:absolute"]));
    timeline(
        &mut s,
        "test:absolute",
        json!({"clock":"test:clock","tracks":{ATTRIBUTE:{"keyframes":[{"ticks":2,"value":false},{"ticks":7,"value":true}]}}}),
    );
    let env = compile(&s);
    for (ticks, expected) in [(-20, false), (6, false), (7, true), (200, true)] {
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(ticks)),
            Some(expected)
        );
    }
}

#[test]
fn easing_precedes_boolean_step_for_both_directions_and_period_wrap() {
    for ease in ["out_back", "linear", "constant"] {
        for from in [false, true] {
            let mut s = startup();
            dimension(&mut s, json!({}), json!(["test:eased"]));
            timeline(
                &mut s,
                "test:eased",
                json!({
                    "clock":"test:clock", "period_ticks":20,
                    "tracks":{ATTRIBUTE:{"ease":ease,"keyframes":[
                        {"ticks":0,"value":from},{"ticks":10,"value":!from}
                    ]}}
                }),
            );
            let env = compile(&s);
            for (ticks, expected) in [
                (0, from),
                (1, from),
                (5, if ease == "out_back" { !from } else { from }),
                (9, if ease == "out_back" { !from } else { from }),
                (10, !from),
                (15, if ease == "out_back" { from } else { !from }),
                (20, from),
                (-15, if ease == "out_back" { !from } else { from }),
            ] {
                assert_eq!(
                    env.sample("minecraft:plains", |_| Some(ticks)),
                    Some(expected),
                    "{ease} {from} {ticks}"
                );
            }
        }
    }
    let mut s = startup();
    dimension(
        &mut s,
        json!({ATTRIBUTE:true}),
        json!(["test:constant-argument"]),
    );
    timeline(
        &mut s,
        "test:constant-argument",
        json!({
            "clock":"test:clock", "period_ticks":20,
            "tracks":{ATTRIBUTE:{"modifier":"and","ease":"out_back","keyframes":[
                {"ticks":0,"value":false},{"ticks":10,"value":true}
            ]}}
        }),
    );
    let env = compile(&s);
    for (ticks, expected) in [(-15, false), (5, false), (10, true)] {
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(ticks)),
            Some(expected)
        );
    }
}

#[test]
fn biome_lookup_is_required_only_when_a_patrol_biome_layer_exists() {
    let mut s = startup();
    dimension(&mut s, json!({ATTRIBUTE:false}), json!([]));
    let registry = s
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap();
    for element in &mut registry.elements {
        let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
        if let Some(attributes) = value.get_mut("attributes").and_then(Value::as_object_mut) {
            attributes.remove(ATTRIBUTE);
        }
        element.source_json = serde_json::to_vec(&value).unwrap().into();
        element.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
    }
    assert_eq!(compile(&s).sample("test:unknown", |_| None), Some(false));
    let element = s
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:plains")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    value["attributes"][ATTRIBUTE] = json!({"modifier":"or","argument":true});
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
    let env = compile(&s);
    assert_eq!(env.sample("test:unknown", |_| None), None);
    assert_eq!(env.sample("minecraft:plains", |_| None), Some(true));
    assert_eq!(
        env.sample("minecraft:mushroom_fields", |_| None),
        Some(false)
    );
}
