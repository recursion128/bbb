use super::*;
use crate::dynamic_registry::RawRegistryElement;
use crate::environment_attributes::{DatapackStartup, TIMELINE};
use bbb_protocol::environment_attribute_types::vanilla_environment_attribute;
use serde_json::{json, Value};

const SKY: &str = "minecraft:visual/sky_color";
const CLOUD: &str = "minecraft:visual/cloud_color";

fn patch(startup: &mut DatapackStartup, registry: &str, id: &str, patch: Value) {
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
    let mut value: Value = serde_json::from_slice(&element.source_json).unwrap();
    for (key, field) in patch.as_object().unwrap() {
        value[key] = field.clone();
    }
    element.source_json = serde_json::to_vec(&value).unwrap().into();
    element.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap());
}

fn timeline(startup: &mut DatapackStartup, id: &str, value: Value) {
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
            source: Some(source),
            codec_value: None,
        });
}

fn compile(startup: &DatapackStartup, attribute: &str) -> DimensionColorEnvironment {
    let sources = EnvironmentAttributeSources::new(startup).unwrap();
    DimensionColorEnvironment::from_attribute(
        &sources,
        "minecraft:overworld",
        vanilla_environment_attribute(attribute).unwrap(),
    )
    .unwrap()
}

#[test]
fn packed_colors_apply_biome_then_ordered_eased_tracks() {
    let mut startup = DatapackStartup::load(&crate::startup::DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: crate::registry_data_loader::DatapackLoadLimits::default(),
    })
    .unwrap();
    patch(
        &mut startup,
        "minecraft:dimension_type",
        "minecraft:overworld",
        json!({"attributes":{},"timelines":[]}),
    );
    let cloud = compile(&startup, CLOUD);
    assert!(!cloud.varies_by_biome());
    assert_eq!(cloud.sample(None, |_| None), Some(0));

    patch(
        &mut startup,
        "minecraft:dimension_type",
        "minecraft:overworld",
        json!({"attributes":{SKY:"#204060",CLOUD:"#80406080"}}),
    );
    assert_eq!(
        compile(&startup, CLOUD).sample(None, |_| None),
        Some(0x8040_6080)
    );
    patch(
        &mut startup,
        "minecraft:worldgen/biome",
        "minecraft:plains",
        json!({"attributes":{
            SKY:{"modifier":"add","argument":"#102030"},
            CLOUD:{"modifier":"add","argument":"#00102030"}
        }}),
    );
    let cloud = compile(&startup, CLOUD);
    assert!(cloud.varies_by_biome());
    assert_eq!(
        cloud.sample(Some("minecraft:plains"), |_| None),
        Some(0x8050_80b0)
    );
    assert_eq!(cloud.sample(None, |_| None), None);
    assert_eq!(cloud.sample(Some("test:absent"), |_| None), None);

    timeline(
        &mut startup,
        "test:first",
        json!({
            "clock":"test:color","period_ticks":200,"tracks":{
                SKY:{"modifier":"subtract","ease":"in_quad","keyframes":[
                    {"ticks":0,"value":"#000000"},{"ticks":100,"value":"#204060"}
                ]},
                CLOUD:{"modifier":"multiply","ease":"in_quad","keyframes":[
                    {"ticks":0,"value":"#40404040"},{"ticks":100,"value":"#c0c0c0c0"}
                ]}
            }
        }),
    );
    timeline(
        &mut startup,
        "test:last",
        json!({"clock":"test:color","tracks":{
            SKY:{"modifier":"multiply","keyframes":[{"ticks":0,"value":"#808080"}]},
            CLOUD:{"modifier":"subtract","keyframes":[{"ticks":0,"value":"#00102030"}]}
        }}),
    );
    patch(
        &mut startup,
        "minecraft:dimension_type",
        "minecraft:overworld",
        json!({"timelines":["test:first","test:last"]}),
    );
    let sky = compile(&startup, SKY);
    let cloud = compile(&startup, CLOUD);
    for ticks in [50, -150] {
        let clock = |id: &str| (id == "test:color").then_some(ticks);
        assert_eq!(
            sky.sample(Some("minecraft:plains"), clock),
            Some(0xff14_283c)
        );
        assert_eq!(
            cloud.sample(Some("minecraft:plains"), clock),
            Some(0x300e_1012)
        );
    }
    assert_eq!(cloud.sample(Some("minecraft:plains"), |_| None), None);
}
