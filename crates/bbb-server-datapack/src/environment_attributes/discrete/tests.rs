use super::*;
use crate::dynamic_registry::RawRegistryElement;
use crate::environment_attributes::DimensionBooleanEnvironment;
use crate::environment_attributes::{DatapackStartup, TIMELINE};
use bbb_facts::environment_attribute_types::vanilla_environment_attribute;
use bbb_registry_codec::registry_codec::canonical_direct_timeline;
use serde_json::{json, Value};

const ANCHOR: &str = "minecraft:gameplay/respawn_anchor_works";

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
    let e = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&e.source_json).unwrap();
    value["attributes"] = attributes;
    value["timelines"] = timelines;
    e.source_json = serde_json::to_vec(&value).unwrap().into();
    e.source = Some(bbb_registry_codec::decode_direct_registry_json(&e.source_json).unwrap());
}
fn biome(s: &mut DatapackStartup, attributes: Value) {
    let e = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:plains")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&e.source_json).unwrap();
    value["attributes"] = attributes;
    e.source_json = serde_json::to_vec(&value).unwrap().into();
    e.source = Some(bbb_registry_codec::decode_direct_registry_json(&e.source_json).unwrap());
}
fn timeline(s: &mut DatapackStartup, id: &str, value: Value) {
    let source_json: std::sync::Arc<[u8]> = serde_json::to_vec(&value).unwrap().into();
    let source = bbb_registry_codec::decode_direct_registry_json(&source_json).unwrap();
    let registry = s
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap();
    registry.elements.retain(|e| e.id != id);
    registry.elements.push(RawRegistryElement {
        id: id.into(),
        source_pack: "test".into(),
        source_json,
        source: Some(source),
        codec_value: None,
    });
}
fn bed(s: &DatapackStartup) -> DimensionBedEnvironment {
    let sources = EnvironmentAttributeSources::new(s).unwrap();
    DimensionBedEnvironment::from_sources(&sources, "minecraft:overworld").unwrap()
}
fn anchor(s: &DatapackStartup) -> DimensionBooleanEnvironment {
    let sources = EnvironmentAttributeSources::new(s).unwrap();
    DimensionBooleanEnvironment::from_attribute(
        &sources,
        "minecraft:overworld",
        vanilla_environment_attribute(ANCHOR).unwrap(),
    )
    .unwrap()
}
fn rule(sleep: &str, spawn: &str) -> Value {
    json!({"can_sleep":sleep,"can_set_spawn":spawn})
}
fn truth(op: &str, base: bool, arg: bool) -> bool {
    match op {
        "override" => arg,
        "and" => base && arg,
        "nand" => !(base && arg),
        "or" => base || arg,
        "nor" => !(base || arg),
        "xor" => base != arg,
        "xnor" => base == arg,
        _ => unreachable!(),
    }
}

#[test]
fn builtins_defaults_and_availability_use_the_supplied_darkness() {
    for (rule, light, dark) in [
        (BedAvailability::Always, true, true),
        (BedAvailability::WhenDark, false, true),
        (BedAvailability::Never, false, false),
    ] {
        assert_eq!(rule.allows(false), light);
        assert_eq!(rule.allows(true), dark);
    }
    let s = startup();
    let sources = EnvironmentAttributeSources::new(&s).unwrap();
    for (dimension, explodes, works) in [
        ("minecraft:overworld", false, false),
        ("minecraft:overworld_caves", false, false),
        ("minecraft:the_nether", true, true),
        ("minecraft:the_end", true, false),
    ] {
        let bed = DimensionBedEnvironment::from_sources(&sources, dimension)
            .unwrap()
            .sample("minecraft:plains", |_| None)
            .unwrap();
        assert_eq!(bed.explodes, explodes);
        assert_eq!(
            bed.can_sleep,
            if explodes {
                BedAvailability::Never
            } else {
                BedAvailability::WhenDark
            }
        );
        assert_eq!(
            bed.can_set_spawn,
            if explodes {
                BedAvailability::Never
            } else {
                BedAvailability::Always
            }
        );
        assert_eq!(
            bed.error_message,
            if explodes {
                None
            } else {
                BedRule::default_value().error_message
            }
        );
        assert_eq!(
            DimensionBooleanEnvironment::from_attribute(
                &sources,
                dimension,
                vanilla_environment_attribute(ANCHOR).unwrap()
            )
            .unwrap()
            .sample("minecraft:plains", |_| None),
            Some(works)
        );
    }
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionBedEnvironment::from_sources(&sources, "test:missing"))
        .is_err());
    assert!(EnvironmentAttributeSources::new(&s)
        .and_then(|sources| DimensionBooleanEnvironment::from_attribute(
            &sources,
            "test:missing",
            vanilla_environment_attribute(ANCHOR).unwrap()
        ))
        .is_err());
}

#[test]
fn dimension_biome_ordered_timelines_preserve_the_canonical_message_and_cache() {
    let mut s = startup();
    dimension(
        &mut s,
        json!({BED:rule("never", "never"), ANCHOR:true}),
        json!("#test:order"),
    );
    biome(
        &mut s,
        json!({BED:rule("always", "when_dark"), ANCHOR:{"modifier":"xor","argument":true}}),
    );
    let message = json!({"translate":"test:sleep","fallback":"晚安","with":[{"text":"moon","bold":true}],"extra":[{"text":"!","color":"#aabbcc"}],"italic":false});
    let mut final_rule = rule("when_dark", "never");
    final_rule["error_message"] = message;
    for (id, bed_rule, bool_value) in [
        (
            "test:a",
            rule("always", "always"),
            json!({"modifier":"or","keyframes":[{"ticks":0,"value":true}]}),
        ),
        (
            "test:b",
            final_rule,
            json!({"modifier":"xor","keyframes":[{"ticks":0,"value":true}]}),
        ),
    ] {
        timeline(
            &mut s,
            id,
            json!({"clock":"test:clock","tracks":{BED:{"keyframes":[{"ticks":0,"value":bed_rule}]},ANCHOR:bool_value}}),
        );
    }
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .insert("test:order".into(), vec!["test:a".into(), "test:b".into()]);
    let bed_env = bed(&s);
    let anchor_env = anchor(&s);
    let canonical = canonical_direct_timeline(
        s.registries
            .element(TIMELINE, "test:b")
            .unwrap()
            .source
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    let canonical_rule = required(&canonical, "tracks")
        .and_then(|tracks| required(tracks, BED))
        .and_then(|track| required(track, "keyframes"))
        .unwrap();
    let Nbt::List(frames) = canonical_rule else {
        panic!("frames");
    };
    let expected_message = field(required(&frames[0], "value").unwrap(), "error_message").cloned();
    let sampled = bed_env
        .sample("minecraft:plains", |clock| {
            assert_eq!(clock, "test:clock");
            Some(5)
        })
        .unwrap();
    assert_eq!(sampled.can_sleep, BedAvailability::WhenDark);
    assert_eq!(sampled.can_set_spawn, BedAvailability::Never);
    assert_eq!(sampled.error_message, expected_message);
    assert_eq!(
        anchor_env.sample("minecraft:plains", |_| Some(5)),
        Some(false)
    );
    assert_eq!(bed_env.sample("test:missing", |_| Some(5)), None);
    assert_eq!(anchor_env.sample("test:missing", |_| Some(5)), None);
    assert_eq!(bed_env.sample("minecraft:plains", |_| None), None);
    assert_eq!(anchor_env.sample("minecraft:plains", |_| None), None);
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
        bed(&s)
            .sample("minecraft:plains", |_| Some(5))
            .unwrap()
            .can_sleep,
        BedAvailability::Always
    );
    assert_eq!(
        anchor(&s).sample("minecraft:plains", |_| Some(5)),
        Some(true)
    );
    assert_eq!(
        bed_env.sample("minecraft:plains", |_| Some(5)),
        Some(sampled)
    );
    assert_eq!(
        anchor_env.sample("minecraft:plains", |_| Some(5)),
        Some(false)
    );
}

#[test]
fn every_boolean_operation_applies_at_dimension_biome_and_timeline_layers() {
    let mut s = startup();
    for op in ["override", "and", "nand", "or", "nor", "xor", "xnor"] {
        for base in [false, true] {
            for arg in [false, true] {
                dimension(
                    &mut s,
                    json!({ANCHOR:{"modifier":op,"argument":arg}}),
                    json!([]),
                );
                biome(&mut s, json!({}));
                assert_eq!(
                    anchor(&s).sample("test:missing", |_| None),
                    Some(truth(op, false, arg))
                );
                dimension(&mut s, json!({ANCHOR:base}), json!([]));
                biome(&mut s, json!({ANCHOR:{"modifier":op,"argument":arg}}));
                assert_eq!(
                    anchor(&s).sample("minecraft:plains", |_| None),
                    Some(truth(op, base, arg))
                );
                biome(&mut s, json!({}));
                dimension(&mut s, json!({ANCHOR:base}), json!(["test:bool"]));
                timeline(
                    &mut s,
                    "test:bool",
                    json!({"clock":"test:clock","tracks":{ANCHOR:{"modifier":op,"keyframes":[{"ticks":7,"value":arg}]}}}),
                );
                for time in [i64::MIN, 0, 7, i64::MAX] {
                    assert_eq!(
                        anchor(&s).sample("test:missing", |_| Some(time)),
                        Some(truth(op, base, arg)),
                        "{op} {base} {arg}"
                    );
                }
            }
        }
    }
}

#[test]
fn override_uses_eased_step_but_boolean_operations_use_constant_arguments() {
    let mut s = startup();
    dimension(&mut s, json!({ANCHOR:true}), json!(["test:ease"]));
    for op in ["override", "and", "nand", "or", "nor", "xor", "xnor"] {
        timeline(
            &mut s,
            "test:ease",
            json!({"clock":"test:clock","tracks":{ANCHOR:{"modifier":op,"ease":"out_back","keyframes":[{"ticks":0,"value":false},{"ticks":10,"value":true}]},BED:{"ease":"out_back","keyframes":[{"ticks":0,"value":rule("never", "never")},{"ticks":10,"value":rule("always", "always")}]}}}),
        );
        let env = anchor(&s);
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(0)),
            Some(truth(op, true, false))
        );
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(5)),
            Some(truth(op, true, op == "override")),
            "{op}"
        );
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(10)),
            Some(truth(op, true, true))
        );
        assert_eq!(
            bed(&s)
                .sample("minecraft:plains", |_| Some(5))
                .unwrap()
                .can_sleep,
            BedAvailability::Always
        );
    }
    timeline(
        &mut s,
        "test:ease",
        json!({"clock":"test:clock","tracks":{ANCHOR:{"ease":"constant","keyframes":[{"ticks":0,"value":false},{"ticks":10,"value":true}]}}}),
    );
    let env = anchor(&s);
    assert_eq!(env.sample("minecraft:plains", |_| Some(9)), Some(false));
    assert_eq!(env.sample("minecraft:plains", |_| Some(10)), Some(true));
}

#[test]
fn periods_negative_clocks_duplicate_frames_and_nonperiod_bounds() {
    let mut s = startup();
    dimension(&mut s, json!({}), json!(["test:period"]));
    timeline(
        &mut s,
        "test:period",
        json!({"clock":"test:clock","period_ticks":10,"tracks":{ANCHOR:{"keyframes":[{"ticks":2,"value":false},{"ticks":2,"value":true},{"ticks":7,"value":false}]},BED:{"keyframes":[{"ticks":2,"value":rule("always", "never")},{"ticks":7,"value":rule("never", "always")}]}}}),
    );
    let anchors = anchor(&s);
    let beds = bed(&s);
    for (time, expected) in [
        (i64::MIN, true),
        (-8, true),
        (-3, false),
        (-1, false),
        (0, false),
        (1, false),
        (2, true),
        (6, true),
        (7, false),
        (9, false),
        (10, false),
        (12, true),
        (i64::MAX, false),
    ] {
        assert_eq!(
            anchors.sample("minecraft:plains", |_| Some(time)),
            Some(expected),
            "{time}"
        );
        assert_eq!(
            beds.sample("minecraft:plains", |_| Some(time))
                .unwrap()
                .can_sleep,
            if expected {
                BedAvailability::Always
            } else {
                BedAvailability::Never
            },
            "{time}"
        );
    }
    timeline(
        &mut s,
        "test:period",
        json!({"clock":"test:clock","tracks":{BED:{"keyframes":[{"ticks":2,"value":rule("never","never")},{"ticks":7,"value":rule("always","always")}]}}}),
    );
    let env = bed(&s);
    for (time, rule) in [
        (i64::MIN, BedAvailability::Never),
        (6, BedAvailability::Never),
        (7, BedAvailability::Always),
        (i64::MAX, BedAvailability::Always),
    ] {
        assert_eq!(
            env.sample("minecraft:plains", |_| Some(time))
                .unwrap()
                .can_sleep,
            rule
        );
    }
}
