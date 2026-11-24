use super::*;

macro_rules! raw_json {
    ($($token:tt)*) => {
        Json::from(serde_json::json!($($token)*))
    };
}
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn item_model_semantic_counts_accept_exact_limits_and_reject_overflow() {
    let exact = PackResourceStack::default();
    exact
        .admit_resource_collection_count(
            MAX_ITEM_MODEL_DEFINITIONS,
            MAX_ITEM_MODEL_DEFINITIONS,
            "item model definitions",
        )
        .unwrap();
    exact
        .admit_resource_collection_count(
            MAX_ITEM_MODEL_REFERENCES,
            MAX_ITEM_MODEL_REFERENCES,
            "item model references",
        )
        .unwrap();

    for (maximum, label) in [
        (MAX_ITEM_MODEL_DEFINITIONS, "item model definitions"),
        (MAX_ITEM_MODEL_REFERENCES, "item model references"),
    ] {
        let stack = PackResourceStack::default();
        let error = stack
            .admit_resource_collection_count(maximum + 1, maximum, label)
            .unwrap_err()
            .to_string();
        assert!(error.contains(label), "{error}");
        assert!(stack.ensure_resource_data_admitted().is_err());
    }
}

#[test]
fn item_model_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
    let Some(source) = local_vanilla_source(&[
        "net",
        "minecraft",
        "client",
        "renderer",
        "item",
        "ItemModels.java",
    ]) else {
        return;
    };
    let fixtures = BTreeMap::from([
        ("minecraft:empty", r#"{"model":{"type":"minecraft:empty"}}"#),
        (
            "minecraft:model",
            r#"{"model":{"type":"minecraft:model","model":"minecraft:item/test"}}"#,
        ),
        (
            "minecraft:range_dispatch",
            r#"{"model":{"type":"minecraft:range_dispatch","property":"minecraft:custom_model_data","entries":[]}}"#,
        ),
        (
            "minecraft:special",
            r#"{"model":{"type":"minecraft:special","base":"minecraft:item/template_skull","model":{"type":"minecraft:head","kind":"zombie"}}}"#,
        ),
        (
            "minecraft:composite",
            r#"{"model":{"type":"minecraft:composite","models":[{"type":"minecraft:empty"}]}}"#,
        ),
        (
            "minecraft:bundle/selected_item",
            r#"{"model":{"type":"minecraft:bundle/selected_item"}}"#,
        ),
        (
            "minecraft:select",
            r#"{"model":{"type":"minecraft:select","property":"minecraft:display_context","cases":[]}}"#,
        ),
        (
            "minecraft:condition",
            r#"{"model":{"type":"minecraft:condition","property":"minecraft:using_item","on_true":{"type":"minecraft:empty"},"on_false":{"type":"minecraft:empty"}}}"#,
        ),
    ]);

    assert_eq!(
        vanilla_bootstrap_ids(&source),
        fixtures
            .keys()
            .map(|model_type| (*model_type).to_string())
            .collect::<BTreeSet<_>>()
    );
    for (model_type, json) in fixtures {
        ClientItemDefinition::from_json_bytes(json.as_bytes())
            .unwrap_or_else(|err| panic!("{model_type} fixture should parse: {err}"));
    }
}

#[test]
fn item_special_model_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
    let Some(source) = local_vanilla_source(&[
        "net",
        "minecraft",
        "client",
        "renderer",
        "special",
        "SpecialModelRenderers.java",
    ]) else {
        return;
    };
    let fixtures = BTreeMap::from([
        (
            "minecraft:bed",
            r#"{"type":"minecraft:bed","texture":"minecraft:red","part":"head"}"#,
        ),
        ("minecraft:bell", r#"{"type":"minecraft:bell"}"#),
        (
            "minecraft:banner",
            r#"{"type":"minecraft:banner","color":"red","attachment":"wall"}"#,
        ),
        (
            "minecraft:book",
            r#"{"type":"minecraft:book","open_angle":45.0,"page1":0.2,"page2":0.8}"#,
        ),
        ("minecraft:conduit", r#"{"type":"minecraft:conduit"}"#),
        (
            "minecraft:chest",
            r#"{"type":"minecraft:chest","texture":"minecraft:normal","openness":0.5,"chest_type":"left"}"#,
        ),
        (
            "minecraft:copper_golem_statue",
            r#"{"type":"minecraft:copper_golem_statue","pose":"star","texture":"minecraft:textures/entity/copper_golem/copper_golem.png"}"#,
        ),
        (
            "minecraft:head",
            r#"{"type":"minecraft:head","kind":"dragon","texture":"minecraft:dragon/custom","animation":0.25}"#,
        ),
        (
            "minecraft:player_head",
            r#"{"type":"minecraft:player_head"}"#,
        ),
        (
            "minecraft:shulker_box",
            r#"{"type":"minecraft:shulker_box","texture":"minecraft:shulker_red","openness":0.25}"#,
        ),
        ("minecraft:shield", r#"{"type":"minecraft:shield"}"#),
        ("minecraft:trident", r#"{"type":"minecraft:trident"}"#),
        (
            "minecraft:decorated_pot",
            r#"{"type":"minecraft:decorated_pot"}"#,
        ),
        (
            "minecraft:standing_sign",
            r#"{"type":"minecraft:standing_sign","wood_type":"oak","attachement":"wall","texture":"minecraft:custom_oak"}"#,
        ),
        (
            "minecraft:hanging_sign",
            r#"{"type":"minecraft:hanging_sign","wood_type":"bamboo","attachment":"ceiling","texture":"minecraft:custom_bamboo"}"#,
        ),
        (
            "minecraft:end_cube",
            r#"{"type":"minecraft:end_cube","effect":"gateway"}"#,
        ),
    ]);

    assert_eq!(
        vanilla_bootstrap_ids(&source),
        fixtures
            .keys()
            .map(|model_type| (*model_type).to_string())
            .collect::<BTreeSet<_>>()
    );
    for (model_type, json) in fixtures {
        let parsed = parse_item_special_model(
            &Json::from_slice(json.as_bytes())
                .unwrap_or_else(|err| panic!("{model_type} fixture JSON must parse: {err}")),
        )
        .unwrap_or_else(|err| panic!("{model_type} fixture should parse: {err}"));
        assert_eq!(parsed.model_type(), model_type);
    }
}

#[test]
fn item_special_model_rejects_unknown_renderer_and_non_finite_parameters() {
    let unknown = raw_json!({"type": "minecraft:not_registered"});
    assert!(parse_item_special_model(&unknown)
        .unwrap_err()
        .to_string()
        .contains("unsupported item special model type"));

    let non_finite = raw_json!({
        "type": "minecraft:book",
        "open_angle": 1e100,
        "page1": 0.0,
        "page2": 0.0
    });
    assert!(parse_item_special_model(&non_finite)
        .unwrap_err()
        .to_string()
        .contains("must be a finite f32"));
}

#[test]
fn item_special_model_defaults_and_standing_sign_typo_follow_vanilla_codecs() {
    let parse = |value| parse_item_special_model(&value).unwrap();
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:banner",
            "color": "blue"
        })),
        ItemSpecialModel::Banner {
            color: "blue".to_string(),
            attachment: ItemSpecialStandingAttachment::Ground,
        }
    );
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:chest",
            "texture": "minecraft:normal"
        })),
        ItemSpecialModel::Chest {
            texture: "minecraft:normal".to_string(),
            openness: 0.0,
            chest_type: ItemSpecialChestType::Single,
        }
    );
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:head",
            "kind": "zombie"
        })),
        ItemSpecialModel::Head {
            kind: ItemSpecialSkullKind::Zombie,
            texture: None,
            animation: 0.0,
        }
    );
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:shulker_box",
            "texture": "minecraft:shulker"
        })),
        ItemSpecialModel::ShulkerBox {
            texture: "minecraft:shulker".to_string(),
            openness: 0.0,
        }
    );
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:standing_sign",
            "wood_type": "oak",
            "attachment": "wall"
        })),
        ItemSpecialModel::StandingSign {
            wood_type: "oak".to_string(),
            attachment: ItemSpecialStandingAttachment::Ground,
            texture: None,
        }
    );
    assert_eq!(
        parse(raw_json!({
            "type": "minecraft:hanging_sign",
            "wood_type": "spruce"
        })),
        ItemSpecialModel::HangingSign {
            wood_type: "spruce".to_string(),
            attachment: ItemSpecialHangingAttachment::CeilingMiddle,
            texture: None,
        }
    );
}

#[test]
fn item_special_model_collects_direct_texture_fields() {
    let models = [
        raw_json!({
            "type": "minecraft:bed",
            "texture": "minecraft:red",
            "part": "foot"
        }),
        raw_json!({
            "type": "minecraft:chest",
            "texture": "minecraft:normal"
        }),
        raw_json!({
            "type": "minecraft:copper_golem_statue",
            "pose": "standing",
            "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
        }),
        raw_json!({
            "type": "minecraft:head",
            "kind": "dragon",
            "texture": "minecraft:dragon/custom"
        }),
        raw_json!({
            "type": "minecraft:shulker_box",
            "texture": "minecraft:shulker"
        }),
        raw_json!({
            "type": "minecraft:standing_sign",
            "wood_type": "oak",
            "texture": "minecraft:custom_oak"
        }),
        raw_json!({
            "type": "minecraft:hanging_sign",
            "wood_type": "spruce",
            "texture": "minecraft:custom_spruce"
        }),
        raw_json!({"type": "minecraft:decorated_pot"}),
    ];
    let mut references = BTreeSet::new();
    for model in models {
        parse_item_special_model(&model)
            .unwrap()
            .collect_texture_references(&mut references);
    }
    assert_eq!(
        references,
        BTreeSet::from([
            "minecraft:custom_oak".to_string(),
            "minecraft:custom_spruce".to_string(),
            "minecraft:dragon/custom".to_string(),
            "minecraft:normal".to_string(),
            "minecraft:red".to_string(),
            "minecraft:shulker".to_string(),
            "minecraft:textures/entity/copper_golem/copper_golem.png".to_string(),
        ])
    );
}

#[test]
fn item_tint_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
    let Some(source) = local_vanilla_source(&[
        "net",
        "minecraft",
        "client",
        "color",
        "item",
        "ItemTintSources.java",
    ]) else {
        return;
    };
    let fixtures = BTreeMap::from([
        (
            "minecraft:custom_model_data",
            r#"{"type":"minecraft:custom_model_data","default":16777215}"#,
        ),
        (
            "minecraft:constant",
            r#"{"type":"minecraft:constant","value":16711935}"#,
        ),
        (
            "minecraft:dye",
            r#"{"type":"minecraft:dye","default":16777215}"#,
        ),
        (
            "minecraft:grass",
            r#"{"type":"minecraft:grass","temperature":0.5,"downfall":0.7}"#,
        ),
        (
            "minecraft:firework",
            r#"{"type":"minecraft:firework","default":16777215}"#,
        ),
        (
            "minecraft:potion",
            r#"{"type":"minecraft:potion","default":16253176}"#,
        ),
        (
            "minecraft:map_color",
            r#"{"type":"minecraft:map_color","default":4603950}"#,
        ),
        (
            "minecraft:team",
            r#"{"type":"minecraft:team","default":16777215}"#,
        ),
    ]);

    assert_eq!(
        vanilla_bootstrap_ids(&source),
        fixtures
            .keys()
            .map(|tint_type| (*tint_type).to_string())
            .collect::<BTreeSet<_>>()
    );
    for (tint_type, tint) in fixtures {
        let json = format!(
            r#"{{"model":{{"type":"minecraft:model","model":"minecraft:item/test","tints":[{tint}]}}}}"#
        );
        ClientItemDefinition::from_json_bytes(json.as_bytes())
            .unwrap_or_else(|err| panic!("{tint_type} fixture should parse: {err}"));
    }
}

#[test]
fn item_model_catalog_loads_simple_models_and_properties() {
    let root = unique_temp_dir("item-model-simple");
    let items = root
        .join("sources")
        .join(crate::MC_VERSION)
        .join("assets")
        .join("minecraft")
        .join("items");
    write_json(
        &items.join("apple.json"),
        r#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/apple"
          }
        }"#,
    );
    write_json(
        &items.join("iron_spear.json"),
        r#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/iron_spear"
          },
          "swap_animation_scale": 1.95
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();

    assert_eq!(catalog.len(), 2);
    let apple = catalog.definition("apple").unwrap();
    assert_eq!(
        apple.model,
        ItemModelDefinition::Model {
            model: "minecraft:item/apple".to_string(),
            transformation: None,
            tints: Vec::new(),
        }
    );
    assert_eq!(apple.properties, ClientItemProperties::default());
    assert_eq!(
        catalog.model_references("minecraft:iron_spear").unwrap(),
        vec!["minecraft:item/iron_spear".to_string()]
    );
    assert_eq!(
        catalog
            .definition("minecraft:iron_spear")
            .unwrap()
            .properties,
        ClientItemProperties {
            hand_animation_on_swap: true,
            oversized_in_gui: false,
            swap_animation_scale: 1.95,
        }
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_parses_model_tint_sources() {
    let root = unique_temp_dir("item-model-tints");
    let items = item_dir(&root);
    write_json(
        &items.join("filled_map.json"),
        r#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/filled_map",
            "tints": [
              { "type": "minecraft:constant", "value": -1 },
              { "type": "minecraft:map_color", "default": 4603950 },
              { "type": "minecraft:custom_model_data", "index": 2, "default": 1193046 },
              { "type": "minecraft:team", "default": 16711680 }
            ]
          }
        }"#,
    );
    write_json(
        &items.join("mixed_tints.json"),
        r#"{
          "model": {
            "type": "minecraft:composite",
            "models": [
              {
                "type": "minecraft:model",
                "model": "minecraft:item/leather_horse_armor",
                "tints": [
                  { "type": "minecraft:dye", "default": -6265536 }
                ]
              },
              {
                "type": "minecraft:model",
                "model": "minecraft:block/grass_block",
                "tints": [
                  { "type": "minecraft:grass", "temperature": 0.5, "downfall": 1.0 },
                  { "type": "minecraft:firework", "default": -7697782 },
                  { "type": "minecraft:potion", "default": -13083194 }
                ]
              }
            ]
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();
    let ItemModelDefinition::Model {
        model,
        transformation,
        tints,
    } = &catalog.definition("filled_map").unwrap().model
    else {
        panic!("filled map should parse as a model item definition");
    };

    assert_eq!(model, "minecraft:item/filled_map");
    assert_eq!(transformation, &None);
    assert_eq!(
        tints,
        &vec![
            ItemTintSource::Constant { value: -1 },
            ItemTintSource::MapColor {
                default_color: 4603950,
            },
            ItemTintSource::CustomModelData {
                index: 2,
                default_color: 1193046,
            },
            ItemTintSource::Team {
                default_color: 16711680,
            },
        ]
    );
    assert_eq!(
        catalog.tint_source_type_counts(),
        BTreeMap::from([
            ("minecraft:constant".to_string(), 1),
            ("minecraft:custom_model_data".to_string(), 1),
            ("minecraft:dye".to_string(), 1),
            ("minecraft:firework".to_string(), 1),
            ("minecraft:grass".to_string(), 1),
            ("minecraft:map_color".to_string(), 1),
            ("minecraft:potion".to_string(), 1),
            ("minecraft:team".to_string(), 1),
        ])
    );
    assert_eq!(
        catalog.model_references("mixed_tints").unwrap(),
        vec![
            "minecraft:block/grass_block".to_string(),
            "minecraft:item/leather_horse_armor".to_string(),
        ]
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_rejects_invalid_model_tints() {
    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:block/grass_block",
            "tints": [
              { "type": "minecraft:grass", "temperature": 1.5, "downfall": 1.0 }
            ]
          }
        }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("must be in range"));

    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/test",
            "tints": [
              { "type": "minecraft:unknown_tint" }
            ]
          }
        }"#,
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("unsupported item tint source type"));
}

#[test]
fn item_model_catalog_preserves_model_transformations() {
    let definition = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:composite",
            "transformation": [
              1.0, 0.0, 0.0, 0.0,
              0.0, 1.0, 0.0, 0.0,
              0.0, 0.0, 1.0, 0.0,
              0.0, 0.0, 0.0, 1.0
            ],
            "models": [
              {
                "type": "minecraft:model",
                "model": "minecraft:item/transformed",
                "transformation": {
                  "left_rotation": [0.0, -0.0, 0.0, 1.0],
                  "right_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [0.6666667, -0.6666667, -0.6666667],
                  "translation": [0.5, 0.0, 0.5]
                }
              },
              {
                "type": "minecraft:special",
                "base": "minecraft:item/template_banner",
                "model": {
                  "type": "minecraft:banner",
                  "color": "white"
                },
                "transformation": {
                  "left_rotation": [0.0, -0.0, 0.0, 1.0],
                  "right_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [0.6666667, -0.6666667, -0.6666667],
                  "translation": [0.5, 0.0, 0.5]
                }
              },
              {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "transformation": {
                  "left_rotation": { "angle": 0.0, "axis": [0.0, 1.0, 0.0] },
                  "right_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [1.0, 1.0, 1.0],
                  "translation": [0.0, 0.0, 0.0]
                },
                "on_true": { "type": "minecraft:empty" },
                "on_false": { "type": "minecraft:empty" }
              },
              {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:use_duration",
                "scale": 0.05,
                "transformation": {
                  "left_rotation": [0.0, 0.0, 0.0, 1.0],
                  "right_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [1.0, 1.0, 1.0],
                  "translation": [0.0, 0.0, 0.0]
                },
                "entries": [
                  {
                    "threshold": 0.65,
                    "model": {
                      "type": "minecraft:model",
                      "model": "minecraft:item/transformed_stage",
                      "transformation": [
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        0.0, 0.0, 0.0, 1.0
                      ]
                    }
                  }
                ]
              },
              {
                "type": "minecraft:select",
                "property": "minecraft:display_context",
                "transformation": {
                  "left_rotation": [0.0, 0.0, 0.0, 1.0],
                  "right_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [1.0, 1.0, 1.0],
                  "translation": [0.0, 0.0, 0.0]
                },
                "cases": [
                  {
                    "when": "gui",
                    "model": {
                      "type": "minecraft:model",
                      "model": "minecraft:item/transformed_gui"
                    }
                  }
                ]
              }
            ]
          }
        }"#,
    )
    .unwrap();

    let ItemModelDefinition::Composite {
        transformation,
        models,
    } = &definition.model
    else {
        panic!("root should parse as a composite item model");
    };

    assert_eq!(transformation, &Some(Mat4::IDENTITY));
    assert_eq!(definition.model.transformation_count(), 7);
    let ItemModelDefinition::Model { transformation, .. } = &models[0] else {
        panic!("first child should parse as a model item definition");
    };
    assert_eq!(
        transformation,
        &Some(
            Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
                * Mat4::from_scale(Vec3::new(0.6666667, -0.6666667, -0.6666667))
        )
    );
    let ItemModelDefinition::Condition { transformation, .. } = &models[2] else {
        panic!("third child should parse as a condition item definition");
    };
    assert_eq!(transformation, &Some(Mat4::IDENTITY));
}

#[test]
fn item_model_matrix_transformation_is_row_major() {
    let matrix = parse_transformation(&raw_json!([
        1.0, 0.0, 0.0, 2.0, 0.0, 1.0, 0.0, 3.0, 0.0, 0.0, 1.0, 4.0, 0.0, 0.0, 0.0, 1.0
    ]))
    .unwrap();
    assert_eq!(matrix, Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0)));
}

#[test]
fn item_model_catalog_rejects_invalid_model_transformations() {
    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/test",
            "transformation": [1.0, 0.0]
          }
        }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("must have 16 numeric values"));

    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/test",
            "transformation": {
              "left_rotation": [0.0, 0.0, 0.0, 1.0],
              "scale": [1.0, 1.0, 1.0],
              "translation": [0.0, 0.0, 0.0]
            }
          }
        }"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("\"right_rotation\""));

    for invalid_rotation in [
        serde_json::json!([0.0, 0.0, 0.0, 0.0]),
        serde_json::json!({"angle": 1.0, "axis": [0.0, 0.0, 0.0]}),
    ] {
        let definition = serde_json::json!({
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test",
                "transformation": {
                    "translation": [0.0, 0.0, 0.0],
                    "left_rotation": invalid_rotation,
                    "scale": [1.0, 1.0, 1.0],
                    "right_rotation": [0.0, 0.0, 0.0, 1.0]
                }
            }
        });
        let err = ClientItemDefinition::from_json_bytes(
            serde_json::to_string(&definition).unwrap().as_bytes(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("finite non-zero f32 norm"));
    }
}

#[test]
fn item_model_catalog_preserves_property_payloads() {
    let definition = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:condition",
            "property": "minecraft:has_component",
            "component": "minecraft:lodestone_tracker",
            "ignore_default": true,
            "on_true": {
              "type": "minecraft:range_dispatch",
              "property": "minecraft:compass",
              "target": "lodestone",
              "scale": 32.0,
              "entries": [
                {
                  "threshold": 0.015625,
                  "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/compass_01"
                  }
                }
              ]
            },
            "on_false": {
              "type": "minecraft:select",
              "property": "minecraft:local_time",
              "pattern": "MM-dd",
              "time_zone": "GMT",
              "cases": [
                {
                  "when": "12-25",
                  "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/chest_christmas"
                  }
                }
              ],
              "fallback": {
                "type": "minecraft:model",
                "model": "minecraft:item/chest"
              }
            }
          }
        }"#,
    )
    .unwrap();

    let ItemModelDefinition::Condition {
        condition,
        on_true,
        on_false,
        ..
    } = &definition.model
    else {
        panic!("root should parse as a condition item model");
    };
    assert_eq!(
        condition,
        &ItemModelCondition::HasComponent {
            component:
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            ignore_default: true,
        }
    );

    let ItemModelDefinition::RangeDispatch {
        property, entries, ..
    } = on_true.as_ref()
    else {
        panic!("true branch should parse as a range dispatch item model");
    };
    assert_eq!(
        property,
        &ItemModelRangeProperty::Compass {
            target: ItemModelCompassTarget::Lodestone,
            wobble: true,
        }
    );
    assert_eq!(entries.len(), 1);

    let ItemModelDefinition::Select {
        property, cases, ..
    } = on_false.as_ref()
    else {
        panic!("false branch should parse as a select item model");
    };
    assert_eq!(
        property,
        &ItemModelSelectProperty::LocalTime {
            pattern: "MM-dd".to_string(),
            locale: String::new(),
            time_zone: Some("GMT".to_string()),
        }
    );
    assert_eq!(
        cases[0].when,
        ItemModelSelectWhen::Values(vec![ItemModelScalar::String("12-25".into())])
    );
}

/// The condition a pack writes as `{"property": ..., <fields>}`.
fn condition_of(fields: serde_json::Value) -> ItemModelCondition {
    let mut model = serde_json::json!({
        "type": "minecraft:condition",
        "on_true": {"type": "minecraft:empty"},
        "on_false": {"type": "minecraft:empty"}
    });
    model
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    let definition = ClientItemDefinition::from_json_bytes(
        serde_json::json!({ "model": model }).to_string().as_bytes(),
    )
    .unwrap();
    let ItemModelDefinition::Condition { condition, .. } = definition.model else {
        panic!("condition definition");
    };
    condition
}

#[test]
fn item_model_catalog_types_condition_properties() {
    use serde_json::json;
    for (fields, expected) in [
        (
            json!({"property": "minecraft:broken"}),
            ItemModelCondition::Broken,
        ),
        (json!({"property": "damaged"}), ItemModelCondition::Damaged),
        (
            json!({"property": "minecraft:fishing_rod/cast"}),
            ItemModelCondition::FishingRodCast,
        ),
        (
            json!({"property": "minecraft:bundle/has_selected_item"}),
            ItemModelCondition::BundleHasSelectedItem,
        ),
        (
            json!({"property": "minecraft:custom_model_data", "index": 1}),
            ItemModelCondition::CustomModelData { index: 1 },
        ),
        (
            json!({"property": "minecraft:selected"}),
            ItemModelCondition::Selected,
        ),
        (
            json!({"property": "minecraft:carried"}),
            ItemModelCondition::Carried,
        ),
        (
            json!({"property": "minecraft:using_item"}),
            ItemModelCondition::UsingItem,
        ),
        (
            json!({"property": "minecraft:view_entity"}),
            ItemModelCondition::ViewEntity,
        ),
        (
            json!({"property": "minecraft:extended_view"}),
            ItemModelCondition::ExtendedView,
        ),
        (
            json!({"property": "minecraft:keybind_down", "keybind": "key.use"}),
            ItemModelCondition::KeybindDown {
                keybind: "key.use".to_string(),
            },
        ),
        (
            json!({"property": "minecraft:component", "predicate": "minecraft:rarity", "value": {}}),
            ItemModelCondition::Component(ItemComponentPredicate::AnyValue(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            )),
        ),
        (
            json!({"property": "minecraft:component", "predicate": "minecraft:damage", "value": {"durability": {"min": 3}}}),
            ItemModelCondition::Component(ItemComponentPredicate::Damage {
                damage: None,
                durability: Some(IntBounds {
                    min: 3,
                    max: i32::MAX,
                }),
            }),
        ),
        (
            json!({"property": "minecraft:component", "predicate": "minecraft:trim", "value": {"material": ["minecraft:gold", "#minecraft:shiny"]}}),
            ItemModelCondition::Component(ItemComponentPredicate::Trim {
                material: Some(ItemModelHolderSet(vec![
                    ItemModelHolder::Key("minecraft:gold".to_string()),
                    ItemModelHolder::Tag("minecraft:shiny".to_string()),
                ])),
                pattern: None,
            }),
        ),
        (
            json!({"property": "minecraft:component", "predicate": "minecraft:trim", "value": {"unknown": 1}}),
            ItemModelCondition::Unsupported("minecraft:component".to_string()),
        ),
        (
            json!({"property": "minecraft:not_a_condition"}),
            ItemModelCondition::Unsupported("minecraft:not_a_condition".to_string()),
        ),
    ] {
        assert_eq!(condition_of(fields.clone()), expected, "{fields}");
    }
}

#[test]
fn item_model_catalog_collects_nested_bow_model_references() {
    let root = unique_temp_dir("item-model-bow");
    let items = item_dir(&root);
    write_json(
        &items.join("bow.json"),
        r#"{
          "model": {
            "type": "minecraft:condition",
            "on_false": {
              "type": "minecraft:model",
              "model": "minecraft:item/bow"
            },
            "on_true": {
              "type": "minecraft:range_dispatch",
              "entries": [
                {
                  "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bow_pulling_1"
                  },
                  "threshold": 0.65
                },
                {
                  "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bow_pulling_2"
                  },
                  "threshold": 0.9
                }
              ],
              "fallback": {
                "type": "minecraft:model",
                "model": "minecraft:item/bow_pulling_0"
              },
              "property": "minecraft:use_duration",
              "scale": 0.05
            },
            "property": "minecraft:using_item"
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();
    let bow = catalog.definition("minecraft:bow").unwrap();

    let ItemModelDefinition::Condition {
        condition, on_true, ..
    } = &bow.model
    else {
        panic!("bow root should be a condition model");
    };
    assert_eq!(condition, &ItemModelCondition::UsingItem);
    let ItemModelDefinition::RangeDispatch {
        property,
        scale,
        entries,
        ..
    } = on_true.as_ref()
    else {
        panic!("bow true branch should be range dispatch");
    };
    assert_eq!(
        property,
        &ItemModelRangeProperty::UseDuration { remaining: false }
    );
    assert_eq!(*scale, 0.05);
    assert_eq!(entries.len(), 2);
    assert_eq!(
        catalog.model_references("minecraft:bow").unwrap(),
        vec![
            "minecraft:item/bow".to_string(),
            "minecraft:item/bow_pulling_0".to_string(),
            "minecraft:item/bow_pulling_1".to_string(),
            "minecraft:item/bow_pulling_2".to_string(),
        ]
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_sorts_range_dispatch_entries_by_threshold() {
    let root = unique_temp_dir("item-model-range-sort");
    let items = item_dir(&root);
    write_json(
        &items.join("bow.json"),
        r#"{
          "model": {
            "type": "minecraft:range_dispatch",
            "entries": [
              {
                "model": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/bow_pulling_2"
                },
                "threshold": 0.9
              },
              {
                "model": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/bow_pulling_1"
                },
                "threshold": 0.65
              }
            ],
            "fallback": {
              "type": "minecraft:model",
              "model": "minecraft:item/bow"
            },
            "property": "minecraft:use_duration"
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();
    let bow = catalog.definition("minecraft:bow").unwrap();

    let ItemModelDefinition::RangeDispatch { entries, .. } = &bow.model else {
        panic!("bow should parse as a range dispatch item model");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].threshold, 0.65);
    assert_eq!(entries[1].threshold, 0.9);
    let ItemModelDefinition::Model { model, .. } = entries[0].model.as_ref() else {
        panic!("first sorted entry should be a model item model");
    };
    assert_eq!(model, "minecraft:item/bow_pulling_1");
    let ItemModelDefinition::Model { model, .. } = entries[1].model.as_ref() else {
        panic!("second sorted entry should be a model item model");
    };
    assert_eq!(model, "minecraft:item/bow_pulling_2");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_parses_select_cases_and_pack_overrides() {
    let root = unique_temp_dir("item-model-select");
    let pack = root.join("resource-pack");
    write_json(
        &item_dir(&root).join("beehive.json"),
        r#"{
          "model": {
            "type": "minecraft:model",
            "model": "minecraft:item/beehive"
          }
        }"#,
    );
    write_json(
        &pack
            .join("assets")
            .join("minecraft")
            .join("items")
            .join("beehive.json"),
        r#"{
          "model": {
            "type": "minecraft:select",
            "block_state_property": "honey_level",
            "cases": [
              {
                "model": {
                  "type": "minecraft:model",
                  "model": "minecraft:block/beehive_honey"
                },
                "when": "5"
              }
            ],
            "fallback": {
              "type": "minecraft:model",
              "model": "minecraft:block/beehive_empty"
            },
            "property": "minecraft:block_state"
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .with_resource_pack_dirs([pack])
        .load_item_model_catalog()
        .unwrap();
    let beehive = catalog.definition("beehive").unwrap();

    let ItemModelDefinition::Select {
        property, cases, ..
    } = &beehive.model
    else {
        panic!("beehive should resolve to the pack override select model");
    };
    assert_eq!(
        property,
        &ItemModelSelectProperty::BlockState {
            property: "honey_level".into()
        }
    );
    assert_eq!(cases.len(), 1);
    assert_eq!(
        cases[0].when,
        ItemModelSelectWhen::Values(vec![ItemModelScalar::String("5".into())])
    );
    assert_eq!(
        catalog.model_references("beehive").unwrap(),
        vec![
            "minecraft:block/beehive_empty".to_string(),
            "minecraft:block/beehive_honey".to_string(),
        ]
    );
    assert_eq!(
        catalog.root_type_counts(),
        BTreeMap::from([("minecraft:select".to_string(), 1)])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_parses_special_composite_and_empty_models() {
    let root = unique_temp_dir("item-model-special");
    let items = item_dir(&root);
    write_json(
        &items.join("white_banner.json"),
        r#"{
          "model": {
            "type": "minecraft:special",
            "base": "minecraft:item/template_banner",
            "model": {
              "type": "minecraft:banner",
              "color": "white",
              "attachment": "ground"
            }
          }
        }"#,
    );
    write_json(
        &items.join("bundle.json"),
        r#"{
          "model": {
            "type": "minecraft:composite",
            "models": [
              {
                "type": "minecraft:model",
                "model": "minecraft:item/bundle"
              },
              {
                "type": "minecraft:bundle/selected_item"
              }
            ]
          }
        }"#,
    );
    write_json(
        &items.join("air.json"),
        r#"{
          "model": {
            "type": "minecraft:empty"
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();

    let ItemModelDefinition::Special { special, .. } =
        &catalog.definition("white_banner").unwrap().model
    else {
        panic!("white banner should parse as a special item model");
    };
    assert_eq!(special.model_type(), "minecraft:banner");
    assert_eq!(
        special,
        &ItemSpecialModel::Banner {
            color: "white".to_string(),
            attachment: ItemSpecialStandingAttachment::Ground,
        }
    );
    assert_eq!(
        catalog.model_references("white_banner").unwrap(),
        vec!["minecraft:item/template_banner".to_string()]
    );
    assert_eq!(
        catalog.model_references("bundle").unwrap(),
        vec!["minecraft:item/bundle".to_string()]
    );
    assert!(catalog.model_references("air").unwrap().is_empty());
    assert_eq!(
        catalog.root_type_counts(),
        BTreeMap::from([
            ("minecraft:composite".to_string(), 1),
            ("minecraft:empty".to_string(), 1),
            ("minecraft:special".to_string(), 1),
        ])
    );
    assert_eq!(
        catalog.special_model_type_counts(),
        BTreeMap::from([("minecraft:banner".to_string(), 1)])
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_structures_copper_golem_statue_special_models() {
    let root = unique_temp_dir("item-model-copper-golem-statue");
    let items = item_dir(&root);
    write_json(
        &items.join("copper_golem_statue.json"),
        r#"{
          "model": {
            "type": "minecraft:select",
            "property": "minecraft:block_state",
            "block_state_property": "copper_golem_pose",
            "cases": [
              {
                "when": "sitting",
                "model": {
                  "type": "minecraft:special",
                  "base": "minecraft:item/template_copper_golem_statue",
                  "model": {
                    "type": "minecraft:copper_golem_statue",
                    "pose": "sitting",
                    "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                  }
                }
              },
              {
                "when": "running",
                "model": {
                  "type": "minecraft:special",
                  "base": "minecraft:item/template_copper_golem_statue",
                  "model": {
                    "type": "minecraft:copper_golem_statue",
                    "pose": "running",
                    "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                  }
                }
              },
              {
                "when": "star",
                "model": {
                  "type": "minecraft:special",
                  "base": "minecraft:item/template_copper_golem_statue",
                  "model": {
                    "type": "minecraft:copper_golem_statue",
                    "pose": "star",
                    "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                  }
                }
              }
            ],
            "fallback": {
              "type": "minecraft:special",
              "base": "minecraft:item/template_copper_golem_statue",
              "model": {
                "type": "minecraft:copper_golem_statue",
                "pose": "standing",
                "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
              }
            }
          }
        }"#,
    );

    let catalog = PackRoots::from_root(&root)
        .unwrap()
        .load_item_model_catalog()
        .unwrap();

    let ItemModelDefinition::Select {
        property,
        cases,
        fallback,
        ..
    } = &catalog.definition("copper_golem_statue").unwrap().model
    else {
        panic!("copper golem statue should parse as a select item model");
    };
    assert_eq!(
        property,
        &ItemModelSelectProperty::BlockState {
            property: "copper_golem_pose".into()
        }
    );

    let mut poses = BTreeSet::new();
    let mut textures = BTreeSet::new();
    let models = cases
        .iter()
        .map(|case| case.model.as_ref())
        .chain(fallback.as_ref().map(|model| model.as_ref()));
    for model in models {
        let ItemModelDefinition::Special { base, special, .. } = model else {
            panic!("copper golem pose arm should be a special model");
        };
        assert_eq!(base, "minecraft:item/template_copper_golem_statue");
        let ItemSpecialModel::CopperGolemStatue { pose, texture } = special else {
            panic!("copper golem special should be structured");
        };
        poses.insert(*pose);
        textures.insert(texture.clone());
    }

    assert_eq!(
        poses,
        BTreeSet::from([
            CopperGolemStatuePose::Running,
            CopperGolemStatuePose::Sitting,
            CopperGolemStatuePose::Standing,
            CopperGolemStatuePose::Star,
        ])
    );
    assert_eq!(
        textures,
        BTreeSet::from(["minecraft:textures/entity/copper_golem/copper_golem.png".to_string()])
    );
    assert_eq!(
        catalog.model_references("copper_golem_statue").unwrap(),
        vec!["minecraft:item/template_copper_golem_statue".to_string()]
    );
    assert_eq!(
        catalog
            .special_texture_references("copper_golem_statue")
            .unwrap(),
        vec!["minecraft:textures/entity/copper_golem/copper_golem.png".to_string()]
    );
    assert_eq!(
        catalog.special_model_type_counts(),
        BTreeMap::from([("minecraft:copper_golem_statue".to_string(), 4)])
    );
    assert_eq!(catalog.special_models().len(), 4);
    assert!(catalog
        .special_models()
        .iter()
        .all(|model| matches!(model, ItemSpecialModel::CopperGolemStatue { .. })));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_model_catalog_rejects_invalid_copper_golem_statue_special_models() {
    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:special",
            "base": "minecraft:item/template_copper_golem_statue",
            "model": {
              "type": "minecraft:copper_golem_statue",
              "pose": "sleeping",
              "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
            }
          }
        }"#,
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("unsupported copper golem statue pose"));

    let err = ClientItemDefinition::from_json_bytes(
        br#"{
          "model": {
            "type": "minecraft:special",
            "base": "minecraft:item/template_copper_golem_statue",
            "model": {
              "type": "minecraft:copper_golem_statue",
              "pose": "standing",
              "texture": 42
            }
          }
        }"#,
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("field \"texture\" must be a string"));
}

#[test]
#[ignore = "requires local vanilla 26.1 sources"]
fn loads_local_vanilla_item_model_catalog() {
    let catalog = PackRoots::from_environment()
        .unwrap()
        .load_item_model_catalog()
        .unwrap();
    assert_eq!(catalog.len(), 1506);
    assert_eq!(
        catalog.root_type_counts(),
        BTreeMap::from([
            ("minecraft:composite".to_string(), 16),
            ("minecraft:condition".to_string(), 7),
            ("minecraft:model".to_string(), 1359),
            ("minecraft:range_dispatch".to_string(), 2),
            ("minecraft:select".to_string(), 71),
            ("minecraft:special".to_string(), 51),
        ])
    );
    assert_eq!(
        catalog.model_references("minecraft:apple").unwrap(),
        vec!["minecraft:item/apple".to_string()]
    );
    assert_eq!(
        catalog.model_references("minecraft:bow").unwrap(),
        vec![
            "minecraft:item/bow".to_string(),
            "minecraft:item/bow_pulling_0".to_string(),
            "minecraft:item/bow_pulling_1".to_string(),
            "minecraft:item/bow_pulling_2".to_string(),
        ]
    );
    assert_eq!(
        catalog.model_references("minecraft:beehive").unwrap(),
        vec![
            "minecraft:block/beehive_empty".to_string(),
            "minecraft:block/beehive_honey".to_string(),
        ]
    );
    assert_eq!(
        catalog.tint_source_type_counts(),
        BTreeMap::from([
            ("minecraft:constant".to_string(), 12),
            ("minecraft:dye".to_string(), 50),
            ("minecraft:firework".to_string(), 1),
            ("minecraft:grass".to_string(), 6),
            ("minecraft:map_color".to_string(), 1),
            ("minecraft:potion".to_string(), 4),
        ])
    );
    assert_eq!(
        catalog.special_model_type_counts(),
        BTreeMap::from([
            ("minecraft:banner".to_string(), 16),
            ("minecraft:bed".to_string(), 32),
            ("minecraft:chest".to_string(), 13),
            ("minecraft:conduit".to_string(), 1),
            ("minecraft:copper_golem_statue".to_string(), 32),
            ("minecraft:decorated_pot".to_string(), 1),
            ("minecraft:head".to_string(), 6),
            ("minecraft:player_head".to_string(), 1),
            ("minecraft:shield".to_string(), 2),
            ("minecraft:shulker_box".to_string(), 17),
            ("minecraft:trident".to_string(), 2),
        ])
    );
    assert_eq!(
        catalog.property_type_counts(),
        BTreeMap::from([
            ("minecraft:block_state".to_string(), 12),
            ("minecraft:broken".to_string(), 1),
            ("minecraft:bundle/has_selected_item".to_string(), 17),
            ("minecraft:charge_type".to_string(), 1),
            ("minecraft:compass".to_string(), 3),
            ("minecraft:context_dimension".to_string(), 1),
            ("minecraft:crossbow/pull".to_string(), 1),
            ("minecraft:display_context".to_string(), 26),
            ("minecraft:fishing_rod/cast".to_string(), 1),
            ("minecraft:has_component".to_string(), 2),
            ("minecraft:local_time".to_string(), 2),
            ("minecraft:time".to_string(), 2),
            ("minecraft:trim_material".to_string(), 29),
            ("minecraft:use_cycle".to_string(), 1),
            ("minecraft:use_duration".to_string(), 1),
            ("minecraft:using_item".to_string(), 5),
        ])
    );
    assert_eq!(catalog.transformation_count(), 83);
}

fn item_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(crate::MC_VERSION)
        .join("assets")
        .join("minecraft")
        .join("items")
}

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn local_vanilla_source(relative: &[&str]) -> Option<String> {
    let roots = PackRoots::from_environment().ok()?;
    let mut path = roots.sources_dir.clone();
    for segment in relative {
        path.push(segment);
    }
    path.is_file()
        .then(|| std::fs::read_to_string(path).ok())
        .flatten()
}

fn vanilla_bootstrap_ids(source: &str) -> BTreeSet<String> {
    Regex::new(r#"ID_MAPPER\.put\(\s*Identifier\.withDefaultNamespace\("([^"]+)"\)"#)
        .unwrap()
        .captures_iter(source)
        .map(|capture| format!("minecraft:{}", &capture[1]))
        .collect()
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
}

/// A select case keeps the units the pack wrote, so a `\uD800` case and a
/// `�` case stay two cases instead of one.
#[test]
fn item_model_select_cases_keep_the_units_the_pack_wrote() {
    let definition = ClientItemDefinition::from_json_bytes(
        br#"{
            "model": {
                "type": "minecraft:select",
                "property": "minecraft:custom_model_data",
                "index": 0,
                "cases": [
                    {
                        "when": "tag\uD800",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/a" }
                    },
                    {
                        "when": "tag\uFFFD",
                        "model": { "type": "minecraft:model", "model": "minecraft:item/b" }
                    }
                ],
                "fallback": { "type": "minecraft:model", "model": "minecraft:item/c" }
            }
        }"#,
    )
    .unwrap();
    let ItemModelDefinition::Select { cases, .. } = &definition.model else {
        panic!("root should parse as a select item model");
    };
    assert_eq!(cases.len(), 2);
    let string =
        |units: Vec<u16>| ItemModelSelectWhen::Values(vec![ItemModelScalar::String(units.into())]);
    assert_eq!(
        cases[0].when,
        string("tag".encode_utf16().chain([0xd800]).collect())
    );
    assert_eq!(
        cases[1].when,
        string("tag\u{fffd}".encode_utf16().collect())
    );
}

#[test]
fn item_model_catalog_counts_duplicate_ast_leaves_and_enforces_depth_and_node_caps() {
    // 1. 500 duplicate leaves in a composite model still count all 501 AST nodes and 500 leaf
    //    references toward the catalog's derived resource budget (not just 1 deduplicated ID).
    let leaf = r#"{"type":"minecraft:model","model":"minecraft:item/stick"}"#;
    let children_500 = std::iter::repeat_n(leaf, 500).collect::<Vec<_>>().join(",");
    let dup_json =
        format!(r#"{{"model":{{"type":"minecraft:composite","models":[{children_500}]}}}}"#);
    let parsed = ClientItemDefinition::from_json_bytes(dup_json.as_bytes()).unwrap();
    assert_eq!(parsed.model_references().len(), 1);
    assert_eq!(parsed.model.ast_node_count(), 501);
    assert_eq!(parsed.model.ast_model_reference_count(), 500);

    let root = unique_temp_dir("item-model-duplicate-leaf-budget");
    let pack = root.join("sources").join(crate::MC_VERSION);
    write_json(
        &pack
            .join("assets")
            .join("minecraft")
            .join("items")
            .join("dup_stick.json"),
        &dup_json,
    );
    // Raw JSON is ~28 KiB, while derived logical bytes for 501 AST nodes + 500 leaf refs is
    // ~128 KiB. A 64 KiB budget would have passed if only 1 deduplicated ref were counted.
    let tight_budget = crate::resource_budget::ResourceDataGenerationBudget::new(64 * 1024);
    let tight_stack = PackResourceStack::from_roots_with_data_budget([pack.clone()], tight_budget);
    assert!(ItemModelCatalog::load_resource_stack(&tight_stack).is_err());

    let ample_budget = crate::resource_budget::ResourceDataGenerationBudget::new(512 * 1024);
    let ample_stack = PackResourceStack::from_roots_with_data_budget([pack], ample_budget);
    let catalog = ItemModelCatalog::load_resource_stack(&ample_stack).unwrap();
    assert_eq!(catalog.len(), 1);
    std::fs::remove_dir_all(root).unwrap();

    // 2. Exceeding MAX_ITEM_MODEL_NODES_PER_DEFINITION fails fast during parse.
    let too_many_children = std::iter::repeat_n(leaf, MAX_ITEM_MODEL_NODES_PER_DEFINITION + 1)
        .collect::<Vec<_>>()
        .join(",");
    let oversized_json =
        format!(r#"{{"model":{{"type":"minecraft:composite","models":[{too_many_children}]}}}}"#);
    assert!(ClientItemDefinition::from_json_bytes(oversized_json.as_bytes()).is_err());

    // 3. Exceeding MAX_ITEM_MODEL_AST_DEPTH fails fast during parse.
    let mut nested = r#"{"type":"minecraft:empty"}"#.to_string();
    for _ in 0..=MAX_ITEM_MODEL_AST_DEPTH {
        nested = format!(
            r#"{{"type":"minecraft:condition","property":"minecraft:using_item","on_true":{nested},"on_false":{{"type":"minecraft:empty"}}}}"#
        );
    }
    let deep_json = format!(r#"{{"model":{nested}}}"#);
    assert!(ClientItemDefinition::from_json_bytes(deep_json.as_bytes()).is_err());
}
