use super::*;
use bbb_facts::environment_attribute_types::vanilla_environment_attribute;

const SKY: &str = "minecraft:gameplay/sky_light_level";
fn compile_sky(s: &DatapackStartup, dimension: &str) -> Result<DimensionFloatAttribute> {
    let sources = EnvironmentAttributeSources::new(s)?;
    DimensionFloatAttribute::from_attribute(
        &sources,
        dimension,
        vanilla_environment_attribute(SKY).unwrap(),
    )
}
use crate::dynamic_registry::RawRegistryElement;
use crate::registry_data_loader::DatapackLoadLimits;
use crate::startup::DatapackStartupOptions;
use bbb_registry_codec::registry_codec::CanonicalRegistryCodecValue;
use std::sync::OnceLock;

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
fn dimension(
    startup: &mut DatapackStartup,
    attributes: serde_json::Value,
    timelines: serde_json::Value,
) {
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    value["attributes"] = attributes;
    value["timelines"] = timelines;
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
}
fn nbt(value: serde_json::Value) -> Nbt {
    match value {
        serde_json::Value::String(s) => Nbt::string(s),
        serde_json::Value::Number(n) if n.is_i64() => Nbt::Int(n.as_i64().unwrap() as i32),
        serde_json::Value::Number(n) => Nbt::Float((n.as_f64().unwrap() as f32).to_bits()),
        serde_json::Value::Array(a) => Nbt::List(a.into_iter().map(nbt).collect()),
        serde_json::Value::Object(a) => Nbt::Compound(
            a.into_iter()
                .map(|(k, v)| bbb_protocol::NbtSummaryEntry {
                    name: k.into(),
                    value: nbt(v),
                })
                .collect(),
        ),
        _ => panic!("unsupported fixture"),
    }
}
fn timeline(startup: &mut DatapackStartup, id: &str, value: serde_json::Value) {
    let source_json: std::sync::Arc<[u8]> = serde_json::to_vec(&value).unwrap().into();
    let source = bbb_registry_codec::decode_direct_registry_json(&source_json).unwrap();
    startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .elements
        .push(RawRegistryElement {
            id: id.into(),
            source_pack: "test".into(),
            source_json,
            codec_value: Some(CanonicalRegistryCodecValue {
                network_nbt: bbb_registry_codec::decode_exact_synchronized_registry_element(
                    TIMELINE, &source,
                )
                .unwrap()
                .projected_network_nbt()
                .unwrap(),
                references: vec![],
            }),
            source: Some(source),
        });
}
#[test]
fn builtin_day_wrap_and_missing_clock() {
    let s = startup();
    let sky = compile_sky(&s, "minecraft:overworld").unwrap();
    assert_eq!(sky.sample_unclamped(None, |_| Some(6000)), Some(15.0));
    assert_eq!(sky.sample_unclamped(None, |_| Some(18000)), Some(4.0));
    assert_eq!(
        sky.sample_unclamped(None, |_| Some(-1)),
        sky.sample_unclamped(None, |_| Some(23999))
    );
    assert_eq!(sky.sample_unclamped(None, |_| None), None);
}

#[test]
fn dimension_modifier_ordered_tag_tracks_and_custom_clocks() {
    use serde_json::json;
    let mut s = startup();
    dimension(
        &mut s,
        json!({SKY:{"modifier":"add","argument":15.0}}),
        json!("#test:order"),
    );
    timeline(
        &mut s,
        "test:second",
        json!({"clock":"test:clock","tracks":{SKY:{"modifier":"subtract","keyframes":[{"ticks":0,"value":5.0}]}}}),
    );
    timeline(
        &mut s,
        "test:first",
        json!({"clock":"test:clock","tracks":{SKY:{"modifier":"multiply","keyframes":[{"ticks":0,"value":0.5}]}}}),
    );
    s.tags
        .registries
        .iter_mut()
        .find(|r| r.registry_id == TIMELINE)
        .unwrap()
        .tags
        .insert(
            "test:order".into(),
            vec!["test:first".into(), "test:second".into()],
        );
    let sky = compile_sky(&s, "minecraft:overworld").unwrap();
    assert_eq!(
        sky.sample_unclamped(None, |id| (id == "test:clock").then_some(0)),
        Some(10.0)
    );
    assert_eq!(sky.sample_unclamped(None, |_| None), None);
}
#[test]
fn modifiers_and_duplicate_tick_segments() {
    use serde_json::json;
    for (name, expected) in [
        ("override", 2.0),
        ("add", 12.0),
        ("subtract", 8.0),
        ("multiply", 20.0),
        ("minimum", 2.0),
        ("maximum", 10.0),
    ] {
        let m = Modifier::parse(name).unwrap();
        assert_eq!(
            m.apply(10.0, m.argument(&Nbt::Float(2.0_f32.to_bits())).unwrap()),
            expected
        );
    }
    let m = Modifier::AlphaBlend;
    assert_eq!(
        m.apply(
            10.0,
            m.argument(&nbt(json!({"value":2.0,"alpha":0.25}))).unwrap()
        ),
        8.0
    );
    assert!(Modifier::parse("oops").is_err());
    let t=Track::compile(&nbt(json!({"clock":"test:c"})),&nbt(json!({"keyframes":[{"ticks":0,"value":1.0},{"ticks":5,"value":2.0},{"ticks":5,"value":9.0},{"ticks":10,"value":10.0}]}))).unwrap();
    assert_eq!(t.sample(-1).value, 1.0);
    assert_eq!(t.sample(5).value, 9.0);
    assert_eq!(t.sample(11).value, 10.0);
}
#[test]
fn easing_endpoints_and_bezier() {
    let names = [
        "constant",
        "linear",
        "in_back",
        "in_bounce",
        "in_circ",
        "in_cubic",
        "in_elastic",
        "in_expo",
        "in_quad",
        "in_quart",
        "in_quint",
        "in_sine",
        "in_out_back",
        "in_out_bounce",
        "in_out_circ",
        "in_out_cubic",
        "in_out_elastic",
        "in_out_expo",
        "in_out_quad",
        "in_out_quart",
        "in_out_quint",
        "in_out_sine",
        "out_back",
        "out_bounce",
        "out_circ",
        "out_cubic",
        "out_elastic",
        "out_expo",
        "out_quad",
        "out_quart",
        "out_quint",
        "out_sine",
    ];
    for name in names {
        let source = serde_json::to_vec(&serde_json::json!({"clock":"test:c","tracks":{SKY:{"ease":name,"keyframes":[{"ticks":0,"value":0.0},{"ticks":10,"value":15.0}]}}})).unwrap();
        let data = bbb_registry_codec::decode_direct_registry_json(&source).unwrap();
        let canonical =
            bbb_registry_codec::decode_exact_synchronized_registry_element(TIMELINE, &data)
                .unwrap()
                .projected_network_nbt()
                .unwrap();
        let track = field(field(&canonical, "tracks").unwrap(), SKY).unwrap();
        let compiled = Track::compile(&canonical, track).unwrap();
        assert_eq!(compiled.sample(0).value, 0.0);
        assert_eq!(compiled.sample(10).value, 15.0);
        let e = compiled.easing;
        assert!(e.apply(0.0).abs() < 0.00001, "{name}");
        assert!(
            (e.apply(1.0) - if name == "constant" { 0.0 } else { 1.0 }).abs() < 0.00001,
            "{name}"
        );
        assert!(e.apply(0.4).is_finite());
    }
    let e = Easing::compile(Some(&nbt(
        serde_json::json!({"cubic_bezier":[0.0,0.0,1.0,1.0]}),
    )))
    .unwrap();
    assert!((e.apply(0.3) - 0.3).abs() < 0.00001);
}

#[test]
fn exact_codec_alpha_blend_interpolates_value_and_weight_without_early_clamp() {
    use serde_json::json;
    let mut s = startup();
    dimension(&mut s, json!({SKY:10.0}), json!(["test:alpha"]));
    timeline(
        &mut s,
        "test:alpha",
        json!({"clock":"test:c","tracks":{SKY:{"modifier":"alpha_blend","keyframes":[{"ticks":0,"value":{"value":30.0,"alpha":0.0}},{"ticks":10,"value":2.0}]}}}),
    );
    let sky = compile_sky(&s, "minecraft:overworld").unwrap();
    assert_eq!(sky.sample_unclamped(None, |_| Some(0)), Some(10.0));
    assert_eq!(sky.sample_unclamped(None, |_| Some(5)), Some(13.0));
    assert_eq!(sky.sample_unclamped(None, |_| Some(10)), Some(2.0));
    dimension(
        &mut s,
        json!({SKY:{"modifier":"alpha_blend","argument":2.0}}),
        json!([]),
    );
    assert_eq!(
        compile_sky(&s, "minecraft:overworld")
            .unwrap()
            .sample_unclamped(None, |_| None),
        Some(2.0)
    );
    dimension(
        &mut s,
        json!({SKY:{"modifier":"multiply","argument":4.0}}),
        json!([]),
    );
    assert_eq!(
        compile_sky(&s, "minecraft:overworld")
            .unwrap()
            .sample_unclamped(None, |_| None),
        Some(60.0)
    );
}

#[test]
fn exact_codec_easing_and_final_sanitize_preserve_overshoot() {
    use serde_json::json;
    let mut s = startup();
    dimension(&mut s, json!({}), json!(["test:ease"]));
    timeline(
        &mut s,
        "test:ease",
        json!({"clock":"test:c","tracks":{SKY:{"ease":"in_back","modifier":"multiply","keyframes":[{"ticks":0,"value":0.0},{"ticks":10,"value":1.0}]}}}),
    );
    let sky = compile_sky(&s, "minecraft:overworld").unwrap();
    assert_eq!(
        sky.sample_unclamped(None, |_| Some(2))
            .map(|value| sky.sanitize(value)),
        Some(0.0)
    );
    assert_eq!(
        sky.sample_unclamped(None, |_| Some(10))
            .map(|value| sky.sanitize(value)),
        Some(15.0)
    );
    dimension(
        &mut s,
        json!({SKY:{"modifier":"nope","argument":1.0}}),
        json!([]),
    );
    assert!(compile_sky(&s, "minecraft:overworld").is_err());
    assert!(compile_sky(&s, "test:missing").is_err());
}
