use super::*;
use bbb_protocol::data_component_types as component_id;

pub(super) fn write_item_model_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "model_component");
    write_json(
        &assets.join("items").join("model_component.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/model_component"
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("alternate_model_component.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/alternate_model_component"
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "model_component", &[40, 120, 180, 255]);
    write_flat_item_model_and_texture(&assets, "alternate_model_component", &[180, 80, 40, 255]);
}

pub(super) fn write_special_item_model_leaf_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        root,
        &[
            "special_selector",
            "bundle",
            "nested_special",
            "decomposed_special",
        ],
    );
    write_json(
        &assets.join("items").join("special_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:composite",
                    "transformation": [
                        1.0, 0.0, 0.0, 2.0,
                        0.0, 1.0, 0.0, 3.0,
                        0.0, 0.0, 1.0, 4.0,
                        0.0, 0.0, 0.0, 1.0
                    ],
                    "models": [
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/special_ordinary",
                            "transformation": [
                                2.0, 0.0, 0.0, 0.0,
                                0.0, 2.0, 0.0, 0.0,
                                0.0, 0.0, 2.0, 0.0,
                                0.0, 0.0, 0.0, 1.0
                            ]
                        },
                        {
                            "type": "minecraft:condition",
                            "property": "minecraft:custom_model_data",
                            "index": 0,
                            "transformation": [
                                1.0, 0.0, 0.0, 5.0,
                                0.0, 1.0, 0.0, 0.0,
                                0.0, 0.0, 1.0, 0.0,
                                0.0, 0.0, 0.0, 1.0
                            ],
                            "on_true": {
                                "type": "minecraft:special",
                                "base": "minecraft:item/special_base",
                                "transformation": [
                                    3.0, 0.0, 0.0, 0.0,
                                    0.0, 3.0, 0.0, 0.0,
                                    0.0, 0.0, 3.0, 0.0,
                                    0.0, 0.0, 0.0, 1.0
                                ],
                                "model": {
                                    "type": "minecraft:decorated_pot"
                                }
                            },
                            "on_false": {
                                "type": "minecraft:model",
                                "model": "minecraft:item/special_fallback"
                            }
                        }
                    ]
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "special_ordinary", &[40, 120, 180, 255]);
    write_flat_item_model_and_texture(&assets, "special_base", &[180, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "special_fallback", &[80, 180, 40, 255]);
    write_json(
        &assets
            .join("models")
            .join("item")
            .join("special_ordinary.json"),
        r#"{
            "parent": "minecraft:builtin/generated",
            "display": {
                "gui": { "translation": [1, 0, 0] }
            },
            "textures": { "layer0": "minecraft:item/special_ordinary" }
        }"#,
    );
    write_json(
        &assets.join("models").join("item").join("special_base.json"),
        r#"{
            "parent": "minecraft:builtin/generated",
            "display": {
                "gui": { "translation": [5, 0, 0] }
            },
            "textures": { "layer0": "minecraft:item/special_base" }
        }"#,
    );
    write_json(
        &assets.join("items").join("bundle.json"),
        r#"{
                "model": {
                    "type": "minecraft:bundle/selected_item"
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("nested_special.json"),
        r#"{
                "model": {
                    "type": "minecraft:special",
                    "base": "minecraft:item/nested_special_base",
                    "transformation": [
                        4.0, 0.0, 0.0, 0.0,
                        0.0, 4.0, 0.0, 0.0,
                        0.0, 0.0, 4.0, 0.0,
                        0.0, 0.0, 0.0, 1.0
                    ],
                    "model": {
                        "type": "minecraft:trident"
                    }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "nested_special_base", &[120, 80, 180, 255]);
    write_json(
        &assets.join("items").join("decomposed_special.json"),
        r#"{
                "model": {
                    "type": "minecraft:special",
                    "base": "minecraft:item/decomposed_special_base",
                    "transformation": {
                        "translation": [1.0, 2.0, 3.0],
                        "left_rotation": [0.0, 0.0, 0.70710677, 0.70710677],
                        "scale": [2.0, 3.0, 4.0],
                        "right_rotation": {
                            "angle": 1.5707964,
                            "axis": [1.0, 0.0, 0.0]
                        }
                    },
                    "model": {
                        "type": "minecraft:bell"
                    }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "decomposed_special_base", &[180, 120, 80, 255]);
}

pub(super) fn write_display_context_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "display_selector");
    write_json(
        &assets.join("items").join("display_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:display_context",
                    "cases": [
                        {
                            "when": "gui",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/display_gui" }
                        },
                        {
                            "when": "ground",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/display_ground" }
                        },
                        {
                            "when": "fixed",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/display_fixed" }
                        },
                        {
                            "when": "thirdperson_righthand",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/display_thirdperson_right" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/display_fallback" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "display_gui", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "display_ground", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "display_fixed", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "display_thirdperson_right", &[160, 40, 120, 255]);
    write_flat_item_model_and_texture(&assets, "display_fallback", &[40, 40, 40, 255]);
}

pub(super) fn write_trim_material_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "iron_chestplate");
    write_json(
        &assets.join("items").join("iron_chestplate.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:trim_material",
                    "cases": [
                        {
                            "when": "minecraft:quartz",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_quartz_trim" }
                        },
                        {
                            "when": "minecraft:diamond",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate_diamond_trim" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/iron_chestplate" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "iron_chestplate", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(
        &assets,
        "iron_chestplate_quartz_trim",
        &[200, 200, 190, 255],
    );
    write_flat_item_model_and_texture(
        &assets,
        "iron_chestplate_diamond_trim",
        &[120, 200, 210, 255],
    );
}

pub(super) fn write_charge_type_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item 0 = crossbow, item 1 = firework_rocket, item 2 = arrow.
    write_item_registry_fixture(root, &["crossbow", "firework_rocket", "arrow"]);
    write_json(
        &assets.join("items").join("crossbow.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:charge_type",
                    "cases": [
                        {
                            "when": "arrow",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_arrow" }
                        },
                        {
                            "when": "rocket",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_firework" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_arrow", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_firework", &[120, 40, 80, 255]);
}

pub(super) fn write_use_tick_range_dispatch_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item ids: 0 = bow, 1 = crossbow, 2 = firework_rocket, 3 = arrow,
    // 4 = brush, 5 = apple, 6 = ender_eye.
    write_item_registry_fixture(
        root,
        &[
            "bow",
            "crossbow",
            "firework_rocket",
            "arrow",
            "brush",
            "apple",
            "ender_eye",
        ],
    );
    write_json(
        &assets.join("items").join("bow.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/bow" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:use_duration",
                        "scale": 0.05,
                        "entries": [
                            {
                                "threshold": 0.65,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_1" }
                            },
                            {
                                "threshold": 0.9,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_2" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/bow_pulling_0" }
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("crossbow.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:charge_type",
                    "cases": [
                        {
                            "when": "arrow",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_arrow" }
                        },
                        {
                            "when": "rocket",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_firework" }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:condition",
                        "property": "minecraft:using_item",
                        "on_false": { "type": "minecraft:model", "model": "minecraft:item/crossbow" },
                        "on_true": {
                            "type": "minecraft:range_dispatch",
                            "property": "minecraft:crossbow/pull",
                            "entries": [
                                {
                                    "threshold": 0.58,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_1" }
                                },
                                {
                                    "threshold": 1.0,
                                    "model": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_2" }
                                }
                            ],
                            "fallback": { "type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_0" }
                        }
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("brush.json"),
        r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_cycle",
                    "period": 10.0,
                    "scale": 0.1,
                    "entries": [
                        {
                            "threshold": 0.25,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_0" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_1" }
                        },
                        {
                            "threshold": 0.75,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/brush_brushing_2" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/brush" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("apple.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/apple" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:use_duration",
                        "remaining": true,
                        "scale": 0.05,
                        "entries": [
                            {
                                "threshold": 0.25,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/apple_remaining_low" }
                            },
                            {
                                "threshold": 0.75,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/apple_remaining_high" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/apple_remaining_empty" }
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("ender_eye.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/ender_eye" },
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:use_duration",
                        "remaining": true,
                        "scale": 0.05,
                        "entries": [
                            {
                                "threshold": 0.25,
                                "model": { "type": "minecraft:model", "model": "minecraft:item/ender_eye_remaining" }
                            }
                        ],
                        "fallback": { "type": "minecraft:model", "model": "minecraft:item/ender_eye_remaining_empty" }
                    }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "bow", &[80, 120, 160, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_0", &[160, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_1", &[120, 160, 80, 255]);
    write_flat_item_model_and_texture(&assets, "bow_pulling_2", &[160, 120, 80, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_arrow", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_firework", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_0", &[70, 100, 130, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_1", &[100, 130, 70, 255]);
    write_flat_item_model_and_texture(&assets, "crossbow_pulling_2", &[130, 70, 100, 255]);
    write_flat_item_model_and_texture(&assets, "brush", &[90, 90, 90, 255]);
    write_flat_item_model_and_texture(&assets, "brush_brushing_0", &[120, 90, 90, 255]);
    write_flat_item_model_and_texture(&assets, "brush_brushing_1", &[90, 120, 90, 255]);
    write_flat_item_model_and_texture(&assets, "brush_brushing_2", &[90, 90, 120, 255]);
    write_flat_item_model_and_texture(&assets, "apple", &[110, 20, 20, 255]);
    write_flat_item_model_and_texture(&assets, "apple_remaining_empty", &[45, 45, 45, 255]);
    write_flat_item_model_and_texture(&assets, "apple_remaining_low", &[170, 90, 60, 255]);
    write_flat_item_model_and_texture(&assets, "apple_remaining_high", &[210, 40, 40, 255]);
    write_flat_item_model_and_texture(&assets, "ender_eye", &[40, 120, 80, 255]);
    write_flat_item_model_and_texture(&assets, "ender_eye_remaining_empty", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "ender_eye_remaining", &[120, 180, 80, 255]);
}

pub(super) fn write_main_hand_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "hand_selector");
    write_json(
        &assets.join("items").join("hand_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:main_hand",
                    "cases": [
                        {
                            "when": "left",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_left" }
                        },
                        {
                            "when": "right",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/hand_selector_right" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/hand_selector" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "hand_selector", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "hand_selector_left", &[120, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "hand_selector_right", &[80, 120, 40, 255]);
}

pub(super) fn write_stack_string_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item 0 = beehive, item 1 = cmd_selector.
    write_item_registry_fixture(root, &["beehive", "cmd_selector"]);
    write_json(
        &assets.join("items").join("beehive.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:block_state",
                    "block_state_property": "honey_level",
                    "cases": [
                        {
                            "when": "5",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/beehive_honey" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/beehive_empty" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("cmd_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:custom_model_data",
                    "index": 1,
                    "cases": [
                        {
                            "when": "blue",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_blue" }
                        },
                        {
                            "when": ["green", "lime"],
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_green" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/cmd_plain" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "beehive_empty", &[90, 70, 40, 255]);
    write_flat_item_model_and_texture(&assets, "beehive_honey", &[220, 180, 40, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_plain", &[40, 40, 40, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_blue", &[40, 80, 220, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_green", &[40, 180, 80, 255]);
}

pub(super) fn write_custom_model_data_condition_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "cmd_flag_condition");
    write_json(
        &assets.join("items").join("cmd_flag_condition.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:custom_model_data",
                    "index": 1,
                    "on_true": { "type": "minecraft:model", "model": "minecraft:item/cmd_flag_true" },
                    "on_false": { "type": "minecraft:model", "model": "minecraft:item/cmd_flag_false" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "cmd_flag_true", &[40, 180, 80, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_flag_false", &[120, 40, 40, 255]);
}

pub(super) fn write_local_time_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_fixture(root, &["seasonal_chest"]);
    write_json(
        &assets.join("items").join("seasonal_chest.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "MM-dd",
                    "time_zone": "GMT",
                    "cases": [
                        {
                            "when": ["12-24", "12-25", "12-26"],
                            "model": { "type": "minecraft:model", "model": "minecraft:item/seasonal_chest_christmas" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/seasonal_chest_normal" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "seasonal_chest_normal", &[80, 60, 40, 255]);
    write_flat_item_model_and_texture(&assets, "seasonal_chest_christmas", &[180, 30, 30, 255]);
}

pub(super) fn write_component_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    let item_ids = [
        "rarity_selector",
        "stack_size_selector",
        "glint_selector",
        "damage_component_selector",
        "max_damage_component_selector",
        "item_model_component_selector",
        "map_id_component_selector",
        "dyed_color_component_selector",
        "map_color_component_selector",
        "custom_name_component_selector",
        "item_name_component_selector",
        "block_item_name_component_selector",
        "damage_type_component_selector",
        "custom_model_data_component_selector",
        "minimum_attack_charge_component_selector",
        "lore_component_selector",
    ];
    write_item_registry_fixture(root, &item_ids);
    let mut catalog = item_registry_override_value(&item_ids);
    copy_item_registry_profile(&mut catalog, "damage_component_selector", "elytra");
    catalog["default_component_codec_values"]["minecraft:damage_component_selector"]
        [component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID.to_string()] =
        serde_json::json!(432);
    copy_item_registry_profile(&mut catalog, "max_damage_component_selector", "elytra");
    catalog["default_component_codec_values"]["minecraft:max_damage_component_selector"]
        [component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID.to_string()] =
        serde_json::json!(432);
    catalog["default_component_codec_values"]["minecraft:block_item_name_component_selector"]
        [component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string()] =
        serde_json::json!({"translate": "block.minecraft.block_item_name_component_selector"});
    catalog["block_items"] = serde_json::json!(["minecraft:block_item_name_component_selector"]);
    catalog["default_component_codec_values"]["minecraft:damage_type_component_selector"]
        [component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID.to_string()] =
        serde_json::json!("minecraft:spear");
    write_item_registry_override_value(root, &catalog);
    write_json(
        &assets.join("items").join("rarity_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:rarity",
                    "cases": [
                        {
                            "when": "common",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_rarity_common" }
                        },
                        {
                            "when": "rare",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_rarity_rare" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_rarity_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("stack_size_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:max_stack_size",
                    "cases": [
                        {
                            "when": 16,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_stack_size_16" }
                        },
                        {
                            "when": 64,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_stack_size_64" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_stack_size_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("glint_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:enchantment_glint_override",
                    "cases": [
                        {
                            "when": true,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_glint_true" }
                        },
                        {
                            "when": false,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_glint_false" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_glint_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("damage_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:damage",
                    "cases": [
                        {
                            "when": 0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_damage_0" }
                        },
                        {
                            "when": 7,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_damage_7" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_damage_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("max_damage_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:max_damage",
                    "cases": [
                        {
                            "when": 99,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_max_damage_99" }
                        },
                        {
                            "when": 432,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_max_damage_432" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_max_damage_fallback" }
                }
            }"#,
    );
    let item_model_component_select = r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:item_model",
                    "cases": [
                        {
                            "when": "minecraft:item_model_component_selector",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_model_default" }
                        },
                        {
                            "when": "minecraft:item_model_component_selector_alt_root",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_model_alt" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_item_model_fallback" }
                }
            }"#;
    write_json(
        &assets
            .join("items")
            .join("item_model_component_selector.json"),
        item_model_component_select,
    );
    write_json(
        &assets
            .join("items")
            .join("item_model_component_selector_alt_root.json"),
        item_model_component_select,
    );
    write_json(
        &assets.join("items").join("map_id_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:map_id",
                    "cases": [
                        {
                            "when": 123,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_map_id_123" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_map_id_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("dyed_color_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:dyed_color",
                    "cases": [
                        {
                            "when": 1193046,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_dyed_color_123456" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_dyed_color_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("map_color_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:map_color",
                    "cases": [
                        {
                            "when": 4548489,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_map_color_456789" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_map_color_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("custom_name_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:custom_name",
                    "cases": [
                        {
                            "when": "Named Selector",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_custom_name_named" }
                        },
                        {
                            "when": { "text": "Object Selector" },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_custom_name_object" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_custom_name_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("item_name_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:item_name",
                    "cases": [
                        {
                            "when": "item.minecraft.item_name_component_selector",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_literal_key" }
                        },
                        {
                            "when": { "translate": "item.minecraft.item_name_component_selector" },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_default" }
                        },
                        {
                            "when": "Patched Item Name",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_literal" }
                        },
                        {
                            "when": { "text": "Object Item Name" },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_object" }
                        },
                        {
                            "when": {
                                "translate": "example.prototype_name",
                                "with": [{"text": "Argument", "bold": true}],
                                "color": "gold",
                                "extra": [{"text": " suffix", "italic": true}]
                            },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_object" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("block_item_name_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:item_name",
                    "cases": [
                        {
                            "when": "block.minecraft.block_item_name_component_selector",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_literal_key" }
                        },
                        {
                            "when": { "translate": "block.minecraft.block_item_name_component_selector" },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_block_default" }
                        },
                        {
                            "when": { "translate": "item.minecraft.block_item_name_component_selector" },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_default" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_item_name_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("damage_type_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:damage_type",
                    "cases": [
                        {
                            "when": "minecraft:spear",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_damage_type_spear" }
                        },
                        {
                            "when": "example:custom",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_damage_type_custom" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_damage_type_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("custom_model_data_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:custom_model_data",
                    "cases": [
                        {
                            "when": {
                                "floats": [0.25, 1.5],
                                "flags": [true, false],
                                "strings": ["first", "second"],
                                "colors": [1193046]
                            },
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_custom_model_data_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_custom_model_data_fallback" }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("minimum_attack_charge_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:minimum_attack_charge",
                    "cases": [
                        {
                            "when": 0.125,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_minimum_attack_charge_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_minimum_attack_charge_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("lore_component_selector.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:component",
                    "component": "minecraft:lore",
                    "cases": [
                        {
                            "when": [
                                { "text": "Styled", "bold": true },
                                "Plain"
                            ],
                            "model": { "type": "minecraft:model", "model": "minecraft:item/component_lore_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/component_lore_fallback" }
                }
            }"#,
    );
    for (model_id, color) in [
        ("component_rarity_common", [80, 80, 80, 255]),
        ("component_rarity_rare", [80, 180, 220, 255]),
        ("component_rarity_fallback", [30, 30, 30, 255]),
        ("component_stack_size_16", [120, 80, 40, 255]),
        ("component_stack_size_64", [40, 120, 80, 255]),
        ("component_stack_size_fallback", [30, 50, 30, 255]),
        ("component_glint_true", [180, 80, 220, 255]),
        ("component_glint_false", [70, 70, 120, 255]),
        ("component_glint_fallback", [40, 40, 80, 255]),
        ("component_damage_0", [40, 140, 180, 255]),
        ("component_damage_7", [180, 80, 40, 255]),
        ("component_damage_fallback", [50, 40, 30, 255]),
        ("component_max_damage_99", [180, 120, 40, 255]),
        ("component_max_damage_432", [40, 180, 120, 255]),
        ("component_max_damage_fallback", [30, 60, 40, 255]),
        ("component_item_model_default", [40, 90, 180, 255]),
        ("component_item_model_alt", [180, 90, 40, 255]),
        ("component_item_model_fallback", [50, 50, 70, 255]),
        ("component_map_id_123", [50, 120, 210, 255]),
        ("component_map_id_fallback", [35, 50, 70, 255]),
        ("component_dyed_color_123456", [0x12, 0x34, 0x56, 255]),
        ("component_dyed_color_fallback", [60, 35, 70, 255]),
        ("component_map_color_456789", [0x45, 0x67, 0x89, 255]),
        ("component_map_color_fallback", [35, 65, 60, 255]),
        ("component_custom_name_named", [210, 160, 80, 255]),
        ("component_custom_name_object", [80, 180, 210, 255]),
        ("component_custom_name_fallback", [70, 45, 35, 255]),
        ("component_item_name_default", [95, 150, 210, 255]),
        ("component_item_name_literal", [210, 120, 95, 255]),
        ("component_item_name_object", [120, 210, 95, 255]),
        ("component_item_name_literal_key", [210, 95, 170, 255]),
        ("component_item_name_block_default", [95, 210, 190, 255]),
        ("component_item_name_fallback", [45, 55, 70, 255]),
        ("component_damage_type_spear", [170, 105, 45, 255]),
        ("component_damage_type_custom", [45, 170, 105, 255]),
        ("component_damage_type_fallback", [55, 45, 70, 255]),
        ("component_custom_model_data_match", [120, 65, 205, 255]),
        ("component_custom_model_data_fallback", [45, 35, 70, 255]),
        ("component_minimum_attack_charge_match", [205, 145, 65, 255]),
        (
            "component_minimum_attack_charge_fallback",
            [70, 45, 35, 255],
        ),
        ("component_lore_match", [65, 205, 145, 255]),
        ("component_lore_fallback", [35, 70, 45, 255]),
    ] {
        write_flat_item_model_and_texture(&assets, model_id, &color);
    }
}

pub(super) fn write_value_aware_range_dispatch_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        root,
        &[
            "damage_dispatch",
            "cmd_dispatch",
            "count_dispatch",
            "bundle_dispatch",
            "apple",
            "bee_nest",
        ],
    );
    // `minecraft:damage` (normalize default true). Entries listed out of
    // threshold order to prove the resolver sorts before selecting.
    write_json(
        &assets.join("items").join("damage_dispatch.json"),
        r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:damage",
                    "entries": [
                        {
                            "threshold": 0.9,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/damage_low" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/damage_half" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/damage_fallback" }
                }
            }"#,
    );
    // `minecraft:custom_model_data` index 1, scale 2.0 (value = floats[1] * 2).
    write_json(
        &assets.join("items").join("cmd_dispatch.json"),
        r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:custom_model_data",
                    "index": 1,
                    "scale": 2.0,
                    "entries": [
                        {
                            "threshold": 3.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_3" }
                        },
                        {
                            "threshold": 0.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_0" }
                        },
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/cmd_1" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/cmd_fallback" }
                }
            }"#,
    );
    // `minecraft:count` normalize default true (value = count / maxStackSize).
    write_json(
        &assets.join("items").join("count_dispatch.json"),
        r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:count",
                    "entries": [
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/count_full" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/count_half" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/count_fallback" }
                }
            }"#,
    );
    // `minecraft:bundle/fullness` uses BundleContents weight.
    write_json(
        &assets.join("items").join("bundle_dispatch.json"),
        r#"{
                "model": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:bundle/fullness",
                    "entries": [
                        {
                            "threshold": 1.0,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_full" }
                        },
                        {
                            "threshold": 0.5,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_half" }
                        },
                        {
                            "threshold": 0.55,
                            "model": { "type": "minecraft:model", "model": "minecraft:item/bundle_nested" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/bundle_fallback" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "damage_half", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "damage_low", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "damage_fallback", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_0", &[10, 20, 30, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_1", &[40, 50, 60, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_3", &[70, 80, 90, 255]);
    write_flat_item_model_and_texture(&assets, "cmd_fallback", &[100, 110, 120, 255]);
    write_flat_item_model_and_texture(&assets, "count_half", &[20, 80, 140, 255]);
    write_flat_item_model_and_texture(&assets, "count_full", &[40, 160, 220, 255]);
    write_flat_item_model_and_texture(&assets, "count_fallback", &[100, 40, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_half", &[140, 80, 20, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_nested", &[180, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_full", &[220, 180, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_fallback", &[40, 40, 100, 255]);
}

pub(super) fn write_bundle_selected_item_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_fixture(root, &["bundle", "apple"]);
    write_json(
        &assets.join("items").join("bundle.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:display_context",
                    "cases": [
                        {
                            "when": "gui",
                            "model": {
                                "type": "minecraft:condition",
                                "property": "minecraft:bundle/has_selected_item",
                                "on_false": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/bundle"
                                },
                                "on_true": {
                                    "type": "minecraft:composite",
                                    "models": [
                                        {
                                            "type": "minecraft:model",
                                            "model": "minecraft:item/bundle_open_back"
                                        },
                                        {
                                            "type": "minecraft:bundle/selected_item"
                                        },
                                        {
                                            "type": "minecraft:model",
                                            "model": "minecraft:item/bundle_open_front"
                                        }
                                    ]
                                }
                            }
                        }
                    ],
                    "fallback": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bundle"
                    }
                }
        }"#,
    );
    write_json(
        &assets.join("items").join("apple.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/apple",
                    "tints": [
                        { "type": "minecraft:team", "default": 16777215 }
                    ]
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "bundle", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_open_back", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_open_front", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "apple", &[200, 40, 40, 255]);
}

pub(super) fn write_single_item_registry_fixture(root: &Path, item_id: &str) {
    write_item_registry_fixture(root, &[item_id]);
}

pub(super) fn write_item_registry_fixture(root: &Path, item_ids: &[&str]) {
    write_item_registry_override(root, item_ids);
}

pub(super) fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, color: &[u8]) {
    write_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
                "parent": "minecraft:builtin/generated",
                "textures": {{
                    "layer0": "minecraft:item/{model_id}"
                }}
            }}"#
        ),
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join(format!("{model_id}.png")),
        1,
        1,
        color,
    );
}

pub(super) fn write_item_registry_fixture_defaults(root: &Path) {
    let mut catalog = item_registry_override_value(&[
        "test_combo",
        "disc_fragment_5",
        "painting",
        "test_sword_item",
    ]);
    copy_item_registry_profile(&mut catalog, "test_sword_item", "iron_sword");
    catalog["default_component_codec_values"]["minecraft:test_combo"]
        [component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID.to_string()] =
        serde_json::json!({"slot": "chest"});
    write_item_registry_override_value(root, &catalog);
}

pub(super) fn item_registry_override_path(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join(".bbb")
        .join("item_registry_catalog.json")
}

pub(super) fn write_item_registry_override(root: &Path, item_ids: &[&str]) {
    write_item_registry_override_value(root, &item_registry_override_value(item_ids));
}

pub(super) fn write_item_registry_override_with_profiles(
    root: &Path,
    item_ids: &[&str],
    profiles: &[(&str, &str)],
) {
    let mut catalog = item_registry_override_value(item_ids);
    for (target, source) in profiles {
        copy_item_registry_profile(&mut catalog, target, source);
    }
    write_item_registry_override_value(root, &catalog);
}

pub(super) fn write_item_registry_override_value(root: &Path, catalog: &serde_json::Value) {
    write_json(
        &item_registry_override_path(root),
        &serde_json::to_string_pretty(catalog).unwrap(),
    );
}

pub(super) fn item_registry_override_value(item_ids: &[&str]) -> serde_json::Value {
    let vanilla = serde_json::to_value(ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    let resource_ids = item_ids
        .iter()
        .map(|item_id| format!("minecraft:{item_id}"))
        .collect::<Vec<_>>();
    let mut catalog = serde_json::json!({
        "resource_ids": resource_ids,
        "protocol_ids": item_ids
            .iter()
            .enumerate()
            .map(|(id, item_id)| (format!("minecraft:{item_id}"), serde_json::json!(id as i32)))
            .collect::<serde_json::Map<_, _>>()
    });
    for field in ["default_component_codec_values", "crafting_remainders"] {
        let mut values = serde_json::Map::new();
        for resource_id in catalog["resource_ids"].as_array().unwrap() {
            let resource_id = resource_id.as_str().unwrap();
            if field == "default_component_codec_values" {
                let mut components = vanilla[field]["minecraft:stick"]
                    .as_object()
                    .unwrap()
                    .clone();
                components.insert(
                    component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string(),
                    serde_json::json!({"translate": format!("item.minecraft.{}", &resource_id[10..])}),
                );
                for type_id in [
                    component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                    component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                ] {
                    let key = type_id.to_string();
                    if let Some(value) = vanilla[field][resource_id].get(&key) {
                        components.insert(key, value.clone());
                    }
                }
                values.insert(
                    resource_id.to_string(),
                    serde_json::Value::Object(components),
                );
            }
        }
        catalog[field] = serde_json::Value::Object(values);
    }
    for field in ["block_items", "spawn_egg_items"] {
        catalog[field] = serde_json::json!([]);
    }
    catalog
}

pub(super) fn copy_item_registry_profile(
    catalog: &mut serde_json::Value,
    target_item_id: &str,
    vanilla_item_id: &str,
) {
    let vanilla = serde_json::to_value(ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    let target = format!("minecraft:{target_item_id}");
    let source = format!("minecraft:{vanilla_item_id}");
    let item_name_key = component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string();
    let item_name = catalog["default_component_codec_values"][&target][&item_name_key].clone();
    for field in ["default_component_codec_values", "crafting_remainders"] {
        if let Some(value) = vanilla[field].get(&source) {
            catalog[field][&target] = value.clone();
        }
    }
    catalog["default_component_codec_values"][&target][item_name_key] = item_name;
    for field in ["block_items", "spawn_egg_items"] {
        if vanilla[field]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(&source))
            && !catalog[field]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str() == Some(&target))
        {
            catalog[field]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::String(target.clone()));
        }
    }
}

pub(super) fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

pub(super) fn player_skin_png_bytes() -> Vec<u8> {
    let mut image = image::RgbaImage::new(64, 64);
    for y in 0..64 {
        for x in 0..64 {
            image.put_pixel(x, y, image::Rgba([x as u8, y as u8, 31, 255]));
        }
    }
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .unwrap();
    cursor.into_inner()
}

pub(super) fn player_profile_texture_png_bytes() -> Vec<u8> {
    let mut image = image::RgbaImage::new(64, 32);
    for y in 0..32 {
        for x in 0..64 {
            image.put_pixel(x, y, image::Rgba([x as u8, y as u8, 63, 255]));
        }
    }
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .unwrap();
    cursor.into_inner()
}

pub(super) fn pump_test_dynamic_textures(runtime: &NativeItemRuntime) -> usize {
    let skin_png = player_skin_png_bytes();
    let profile_png = player_profile_texture_png_bytes();
    let requests = runtime.take_pending_dynamic_texture_requests();
    let count = requests.len();
    runtime.apply_dynamic_texture_fetch_results(requests.into_iter().map(|request| {
        let bytes = match request.kind {
            bbb_protocol::DynamicTextureFetchKind::Skin => skin_png.clone(),
            bbb_protocol::DynamicTextureFetchKind::Cape
            | bbb_protocol::DynamicTextureFetchKind::Elytra => profile_png.clone(),
        };
        DynamicTextureFetchResult {
            kind: request.kind,
            url: request.url,
            handle: request.handle,
            png_bytes: Ok(bytes),
            from_network: true,
        }
    }));
    count
}

pub(super) fn drain_all_player_skin_download_results(
    runtime: &NativeItemRuntime,
) -> Vec<NativeDynamicPlayerSkinDownload> {
    let mut downloads = Vec::new();
    loop {
        let batch = runtime.drain_dynamic_player_skin_download_results();
        if batch.is_empty() {
            return downloads;
        }
        downloads.extend(batch);
    }
}

pub(super) fn drain_all_player_texture_download_results(
    runtime: &NativeItemRuntime,
) -> Vec<NativeDynamicPlayerTextureDownload> {
    let mut downloads = Vec::new();
    loop {
        let batch = runtime.drain_dynamic_player_texture_download_results();
        if batch.is_empty() {
            return downloads;
        }
        downloads.extend(batch);
    }
}

pub(super) fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8).unwrap();
}

pub(super) fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("bbb-client-{label}-{nanos}-{id}"))
}

pub(super) fn build_component_defaults(
    codecs: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
) -> bbb_render_types::ItemComponentDefaults {
    let mut patches = BTreeMap::new();
    for (&item_id, values) in &codecs {
        let mut patch = DataComponentPatchSummary::default();
        for (&type_id, value) in values {
            let nbt = bbb_protocol::json_value_to_nbt_summary(value).unwrap();
            bbb_registry_codec::read_persistent_component_into_with_resolver(
                type_id, &nbt, None, &mut patch,
            )
            .unwrap();
            patch.added_type_ids.push(type_id);
        }
        patches.insert(item_id, patch);
    }
    bbb_render_types::ItemComponentDefaults::from_btree_maps(codecs, patches)
}

pub(super) fn stack_component_defaults(
    runtime: &NativeItemRuntime,
) -> bbb_render_types::ItemComponentDefaults {
    let mut defaults = runtime.default_component_codec_values_by_protocol_id();
    for values in defaults.values_mut() {
        values.retain(|type_id, _| {
            matches!(
                *type_id,
                component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
                    | component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
                    | component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID
                    | component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID
                    | component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID
            )
        });
    }
    build_component_defaults(defaults)
}
