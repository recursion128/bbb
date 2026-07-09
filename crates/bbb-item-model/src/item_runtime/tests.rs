use super::*;
use crate::skin_runtime::{
    AsyncDynamicPlayerSkinRuntime, AsyncDynamicPlayerTextureRuntime, SkinPngFetcher,
};
use chrono::TimeZone;
use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

fn tooltip_line(text: &str, tint: [f32; 4]) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun::plain(text)],
    }
}

fn italic_tooltip_line(text: &str, tint: [f32; 4], color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            text: text.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(color),
        }],
    }
}

/// A hover-name line: vanilla `getStyledHoverName` wraps the name in the
/// rarity colour and italicizes custom names.
fn name_line(text: &str, tint: [f32; 4], color: u32, italic: bool) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            text: text.to_string(),
            style: HudTextStyle {
                italic,
                ..HudTextStyle::default()
            },
            color: Some(color),
        }],
    }
}

/// A lore line carrying vanilla `ItemLore.LORE_STYLE` (DARK_PURPLE + italic).
fn lore_line(text: &str) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEXT_DARK_PURPLE,
        runs: vec![HudStyledTextRun {
            text: text.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(0xAA_00_AA),
        }],
    }
}

#[test]
fn item_texture_state_indexes_textures_and_uses_missing_fallback() {
    let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
    let apple = SpriteImage::new("minecraft:item/apple", 1, 1, vec![255, 0, 0, 127]).unwrap();

    let state = ItemTextureState::from_images(vec![missing, apple]).unwrap();

    assert_eq!(state.texture_count(), 2);
    assert_ne!(
        state.texture_index("minecraft:item/apple"),
        state.texture_index("minecraft:missingno")
    );
    assert_eq!(
        state.texture_index("custom:item/missing"),
        state.texture_index(MISSING_TEXTURE_ID)
    );
    let sprite_uvs = state.sprite_uvs();
    assert_eq!(sprite_uvs.len(), state.texture_count());
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/apple" && sprite.has_translucent));
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == MISSING_TEXTURE_ID && !sprite.has_translucent));
}

#[test]
fn item_tint_sources_use_stack_component_colors_when_available() {
    let patch = DataComponentPatchSummary {
        custom_model_data_colors: vec![0x01_02_03, 0x04_05_06],
        dyed_color: Some(0x07_08_09),
        map_color: Some(0x0a_0b_0c),
        potion_custom_color: Some(0x0d_0e_0f),
        firework_explosion_colors: vec![0x10_20_30, 0x20_40_60],
        ..DataComponentPatchSummary::default()
    };

    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::CustomModelData {
                index: 1,
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0x04_05_06)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Dye {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0x07_08_09)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::MapColor {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0x0a_0b_0c)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Potion {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0x0d_0e_0f)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Firework {
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0x18_30_48)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::CustomModelData {
                index: 2,
                default_color: 0xff_00_ff,
            },
            Some(&patch),
        ),
        rgb_i32_tint(0xff_00_ff)
    );
    assert_eq!(
        item_tint_source_color(
            &ItemTintSource::Firework {
                default_color: 0xff_00_ff,
            },
            Some(&DataComponentPatchSummary::default()),
        ),
        rgb_i32_tint(0xff_00_ff)
    );
}

#[test]
fn localized_item_name_prefers_item_key_then_block_key() {
    let language = LanguageCatalog::from_json_bytes(
        br#"{
                "item.minecraft.redstone": "Redstone Dust",
                "block.minecraft.stone": "Stone"
            }"#,
    )
    .unwrap();

    assert_eq!(
        localized_item_name(&language, "minecraft:redstone"),
        "Redstone Dust"
    );
    assert_eq!(localized_item_name(&language, "minecraft:stone"), "Stone");
    assert_eq!(
        localized_item_name(&language, "minecraft:missing_item"),
        "item.minecraft.missing_item"
    );
    assert_eq!(
        description_key("item", "custom:tools/hammer"),
        "item.custom.tools.hammer"
    );
}

#[test]
fn native_item_runtime_loads_fixture_and_keeps_missingno_fallback() {
    let root = unique_temp_dir("item-runtime");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_sources(&root);
    write_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
                "item.minecraft.test_combo": "Test Combo",
                "item.unbreakable": "Unbreakable",
                "item.durability": "Durability: %s / %s",
                "item.components": "%s component(s)",
                "container.beehive.bees": "Bees: %s / %s",
                "item.dyed": "Dyed",
                "item.color": "Color: %s",
                "book.byAuthor": "by %1$s",
                "book.generation.0": "Original",
                "book.generation.2": "Copy of a copy"
            }"#,
    );
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "model": {
                    "type": "minecraft:composite",
                    "models": [
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/test_sword",
                            "tints": [
                                { "type": "minecraft:constant", "value": 3368601 },
                                { "type": "minecraft:custom_model_data", "index": 1, "default": 16711935 }
                            ]
                        },
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/missing_model"
                        }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r##"{
                "textures": {
                    "layer0": "minecraft:item/test_sword",
                    "layer1": {
                        "sprite": "custom:item/missing_overlay",
                        "force_translucent": true
                    },
                    "layer2": "minecraft:item/test_overlay"
                }
            }"##,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join("test_overlay.png"),
        1,
        1,
        &[0, 255, 0, 127],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(runtime.item_definition_count(), 1);
    assert_eq!(runtime.item_registry_count(), 1);
    assert_eq!(runtime.item_equipment_slot_count(), 1);
    assert_eq!(runtime.item_mining_profile_count(), 0);
    assert_eq!(
        runtime.item_equipment_slots_by_protocol_id(),
        BTreeMap::from([(0, WorldItemEquipmentSlot::Chest)])
    );
    assert_eq!(runtime.resolved_model_count(), 1);
    assert_eq!(runtime.missing_model_count(), 1);
    assert_eq!(runtime.missing_texture_count(), 1);
    assert_eq!(runtime.texture_count(), 3);
    assert_eq!(runtime.icon_texture_count(), 1);
    assert_ne!(
        runtime.texture_index("minecraft:item/test_sword"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.texture_index("custom:item/missing_overlay"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.texture_index("unknown:item/texture"),
        runtime.texture_index(MISSING_TEXTURE_ID)
    );
    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/test_sword"))
    );
    assert_eq!(
        runtime.default_item_particle_sprite_ids_by_protocol_id(),
        BTreeMap::from([(
            0,
            vec![
                "minecraft:item/test_sword".to_string(),
                MISSING_TEXTURE_ID.to_string(),
                "minecraft:item/test_overlay".to_string(),
            ],
        )])
    );
    assert_eq!(runtime.icon_texture_index_for_protocol_id(1), None);
    let icon = runtime.icon_for_protocol_id(0).unwrap();
    assert_eq!(icon.layers.len(), 3);
    assert_eq!(icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
    assert_eq!(icon.layers[1].tint, rgb_i32_tint(0xff_00_ff));
    assert_eq!(icon.layers[2].tint, ITEM_TINT_WHITE);
    let sprite_uvs = runtime.atlas_sprite_uvs();
    assert_eq!(sprite_uvs.len(), runtime.texture_count());
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/test_sword" && sprite.uv == icon.layers[0].uv));
    assert!(sprite_uvs
        .iter()
        .any(|sprite| sprite.id == "minecraft:item/test_overlay" && sprite.has_translucent));
    assert_eq!(
        icon.layers[1].uv,
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(MISSING_TEXTURE_ID))
            .unwrap()
    );
    assert_eq!(runtime.icon_uv_for_protocol_id(0), Some(icon.layers[0].uv));
    let generated_layers = runtime.generated_item_layers_for_stack_with_trim_materials(
        &ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
        BlockModelDisplayContext::Ground,
        None,
    );
    assert_eq!(generated_layers.len(), 3);
    assert!(!generated_layers[0].translucent);
    assert!(generated_layers[1].translucent);
    assert!(generated_layers[2].translucent);
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
            true
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("13 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    let damaged_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![2, 3],
            max_damage: Some(20),
            damage: Some(3),
            ..DataComponentPatchSummary::default()
        },
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack(&damaged_stack),
        Some(vec![name_line(
            "Test Combo",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(&damaged_stack, true),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Durability: 17 / 20", TOOLTIP_TEXT_WHITE),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("15 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 0,
            component_patch: DataComponentPatchSummary::default(),
        }),
        None
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Custom Pick".to_string()),
                item_name: Some("Renamed Item Name".to_string()),
                lore: vec!["First lore".to_string(), "Second lore".to_string()],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Custom Pick", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, true),
            lore_line("First lore"),
            lore_line("Second lore"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                    title: "Book Title".to_string(),
                    title_filter: None,
                    author: "Alex".to_string(),
                    generation: 0,
                    pages: Vec::new(),
                    page_filters: Vec::new(),
                    resolved: true,
                }),
                item_name: Some("Ignored Item Name".to_string()),
                lore: vec!["Book lore".to_string()],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Book Title", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("by Alex", TOOLTIP_TEXT_GRAY),
            tooltip_line("Original", TOOLTIP_TEXT_GRAY),
            lore_line("Book lore"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                    title: "Copy".to_string(),
                    title_filter: None,
                    author: "   ".to_string(),
                    generation: 2,
                    pages: Vec::new(),
                    page_filters: Vec::new(),
                    resolved: true,
                }),
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Copy", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Copy of a copy", TOOLTIP_TEXT_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Component Item Name".to_string()),
                rarity: Some(ItemRaritySummary::Uncommon),
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![name_line(
            "Component Item Name",
            TOOLTIP_TEXT_YELLOW,
            0xFF_FF_55,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Durable Item".to_string()),
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Durable Item", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Unbreakable", TOOLTIP_TEXT_BLUE),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                bees_count: 2,
                lore: vec!["After bees".to_string()],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Bees: 2 / 3", TOOLTIP_TEXT_GRAY),
            lore_line("After bees"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![44],
                dyed_color: Some(0x12_3A_BC),
                lore: vec!["After dyed".to_string()],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            italic_tooltip_line("Dyed", TOOLTIP_TEXT_GRAY, 0xAA_AA_AA),
            lore_line("After dyed"),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_options(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![44],
                    dyed_color: Some(0x12_3A_BC),
                    ..DataComponentPatchSummary::default()
                },
            },
            true,
        ),
        Some(vec![
            name_line("Test Combo", TOOLTIP_TEXT_WHITE, 0xFF_FF_FF, false),
            tooltip_line("Color: #123ABC", TOOLTIP_TEXT_GRAY),
            tooltip_line("minecraft:test_combo", TOOLTIP_TEXT_DARK_GRAY),
            tooltip_line("14 component(s)", TOOLTIP_TEXT_DARK_GRAY),
        ])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Enchanted Item".to_string()),
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![name_line(
            "Enchanted Item",
            TOOLTIP_TEXT_AQUA,
            0x55_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                item_name: Some("Rare Enchanted Item".to_string()),
                rarity: Some(ItemRaritySummary::Rare),
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![name_line(
            "Rare Enchanted Item",
            TOOLTIP_TEXT_LIGHT_PURPLE,
            0xFF_55_FF,
            false
        )])
    );
    // Styled component runs flow through: the custom name keeps its own keys
    // over the rarity wrapper (colour) while inheriting the custom-name
    // italic; an explicit `italic:0b` lore key beats the injected
    // `ItemLore.LORE_STYLE` and its colour override beats DARK_PURPLE.
    assert_eq!(
        runtime.tooltip_lines_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Bold Red".to_string()),
                custom_name_styled: Some(vec![bbb_protocol::StyledTextRun {
                    text: "Bold Red".to_string(),
                    style: bbb_protocol::ComponentStyle {
                        bold: Some(true),
                        color: Some(0xFF_55_55),
                        ..Default::default()
                    },
                }]),
                lore: vec!["Upright gold".to_string()],
                lore_styled: vec![vec![bbb_protocol::StyledTextRun {
                    text: "Upright gold".to_string(),
                    style: bbb_protocol::ComponentStyle {
                        italic: Some(false),
                        color: Some(0xFF_AA_00),
                        ..Default::default()
                    },
                }]],
                ..DataComponentPatchSummary::default()
            },
        }),
        Some(vec![
            NativeItemTooltipLine {
                text: "Bold Red".to_string(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: vec![HudStyledTextRun {
                    text: "Bold Red".to_string(),
                    style: HudTextStyle {
                        bold: true,
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xFF_55_55),
                }],
            },
            NativeItemTooltipLine {
                text: "Upright gold".to_string(),
                tint: TOOLTIP_TEXT_DARK_PURPLE,
                runs: vec![HudStyledTextRun {
                    text: "Upright gold".to_string(),
                    style: HudTextStyle::default(),
                    color: Some(0xFF_AA_00),
                }],
            },
        ])
    );

    let stack_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                custom_model_data_colors: vec![0x00_00_00, 0x12_34_56],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(stack_icon.layers[0].tint, rgb_i32_tint(0x33_66_99));
    assert_eq!(stack_icon.layers[1].tint, rgb_i32_tint(0x12_34_56));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn special_foil_texture_detection_follows_clock_and_compasses_tag() {
    let root = unique_temp_dir("item-runtime-special-foil");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_source(&root, &["clock", "compass", "recovery_compass", "spyglass"]);
    for item_id in ["clock", "compass", "recovery_compass", "spyglass"] {
        write_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                        "model": {{ "type": "minecraft:model", "model": "minecraft:item/{item_id}" }}
                    }}"#
            ),
        );
        write_flat_item_model_and_texture(&assets, item_id, &[40, 80, 120, 255]);
    }
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item")
            .join("compasses.json"),
        r#"{
                "replace": true,
                "values": ["minecraft:compass"]
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let stack = |resource_id: &str| ItemStackSummary {
        item_id: registry.protocol_id(resource_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };

    assert!(runtime.item_stack_uses_special_foil_texture(&stack("minecraft:clock")));
    assert!(runtime.item_stack_uses_special_foil_texture(&stack("minecraft:compass")));
    assert!(!runtime.item_stack_uses_special_foil_texture(&stack("minecraft:recovery_compass")));
    assert!(!runtime.item_stack_uses_special_foil_texture(&stack("minecraft:spyglass")));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_display_transform_is_retained_per_item() {
    let root = unique_temp_dir("item-display-transform");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_sources(&root);
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("test_combo_alt.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword_alt"
                }
            }"#,
    );
    // An `item/handheld`-style angled third-person transform on the item's own model.
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r##"{
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [0, -90, 55],
                        "translation": [0, 4, 0.5],
                        "scale": [0.85, 0.85, 0.85]
                    }
                },
                "textures": { "layer0": "minecraft:item/test_sword" }
            }"##,
    );
    write_json(
        &assets
            .join("models")
            .join("item")
            .join("test_sword_alt.json"),
        r##"{
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [10, -20, 30],
                        "translation": [1, 8, 2],
                        "scale": [0.4, 0.5, 0.6]
                    }
                },
                "textures": { "layer0": "minecraft:item/test_sword_alt" }
            }"##,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join("test_sword_alt.png"),
        1,
        1,
        &[0, 255, 0, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let transform = runtime
        .item_display_transform(0, BlockModelDisplayContext::ThirdPersonRightHand)
        .unwrap();
    // Vanilla pre-multiplies the JSON translation by 1/16 (and clamps); rotation stays in degrees.
    assert_eq!(transform.rotation, [0.0, -90.0, 55.0]);
    assert_eq!(transform.translation, [0.0, 4.0 / 16.0, 0.5 / 16.0]);
    assert_eq!(transform.scale, [0.85, 0.85, 0.85]);
    // A context the model does not override falls back to the identity transform.
    assert_eq!(
        runtime.item_display_transform(0, BlockModelDisplayContext::Gui),
        Some(BlockModelDisplayTransform::default())
    );
    // An unregistered protocol id has no retained transform (caller uses a parent-model default).
    assert_eq!(
        runtime.item_display_transform(999, BlockModelDisplayContext::ThirdPersonRightHand),
        None
    );
    let stack = |component_patch| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch,
    };
    assert_eq!(
        runtime.item_display_transform_for_stack(
            &stack(DataComponentPatchSummary::default()),
            BlockModelDisplayContext::ThirdPersonRightHand,
        ),
        Some(transform)
    );
    let alternate = runtime
        .item_display_transform_for_stack(
            &stack(DataComponentPatchSummary {
                item_model: Some("minecraft:test_combo_alt".to_string()),
                ..DataComponentPatchSummary::default()
            }),
            BlockModelDisplayContext::ThirdPersonRightHand,
        )
        .unwrap();
    assert_eq!(alternate.rotation, [10.0, -20.0, 30.0]);
    assert_eq!(alternate.translation, [1.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0]);
    assert_eq!(alternate.scale, [0.4, 0.5, 0.6]);
    assert_eq!(
        runtime.item_display_transform_for_stack(
            &stack(DataComponentPatchSummary {
                removed_type_ids: vec![10],
                ..DataComponentPatchSummary::default()
            }),
            BlockModelDisplayContext::ThirdPersonRightHand,
        ),
        None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_armor_asset_query_excludes_head_equippable_non_armor() {
    let root = unique_temp_dir("item-runtime-humanoid-armor-asset");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item DIAMOND_HELMET = registerItem(
                    "diamond_helmet",
                    new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.HELMET)
                );
                public static final Item DIAMOND_CHESTPLATE = registerItem(
                    "diamond_chestplate",
                    new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.CHESTPLATE)
                );
                public static final Item ELYTRA = registerItem(
                    "elytra",
                    new Item.Properties()
                       .component(
                          DataComponents.EQUIPPABLE,
                          Equippable.builder(EquipmentSlot.CHEST)
                             .setAsset(EquipmentAssets.ELYTRA)
                             .build()
                       )
                );
                public static final Item CARVED_PUMPKIN = registerBlock(
                    Blocks.CARVED_PUMPKIN,
                    p -> p.component(
                        DataComponents.EQUIPPABLE,
                        Equippable.builder(EquipmentSlot.HEAD).setSwappable(false).build()
                    )
                );
                public static final Item STONE = registerBlock(Blocks.STONE);
            }"#,
    );
    write_json(
        &assets.join("equipment").join("diamond.json"),
        r#"{
                "layers": {
                    "humanoid": [
                        { "texture": "minecraft:diamond" }
                    ],
                    "humanoid_leggings": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("equipment").join("elytra.json"),
        r#"{
                "layers": {
                    "wings": [
                        { "texture": "minecraft:elytra", "use_player_texture": true }
                    ]
                }
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let helmet_id = registry.protocol_id("minecraft:diamond_helmet").unwrap();
    let chestplate_id = registry
        .protocol_id("minecraft:diamond_chestplate")
        .unwrap();
    let elytra_id = registry.protocol_id("minecraft:elytra").unwrap();
    let pumpkin_id = registry.protocol_id("minecraft:carved_pumpkin").unwrap();
    let stone_id = registry.protocol_id("minecraft:stone").unwrap();

    assert!(runtime.item_has_humanoid_armor_asset(helmet_id));
    assert!(runtime.item_has_humanoid_armor_asset(chestplate_id));
    assert!(!runtime.item_has_humanoid_armor_asset(pumpkin_id));
    assert!(!runtime.item_has_humanoid_armor_asset(stone_id));
    assert!(!runtime.item_has_humanoid_armor_asset(999));
    assert!(runtime.item_equipment_asset_has_humanoid_layer(chestplate_id));
    assert!(!runtime.item_equipment_asset_has_wings_layer(chestplate_id));
    assert!(runtime.item_equipment_asset_has_wings_layer(elytra_id));
    assert_eq!(
        runtime.item_equipment_wings_layer(elytra_id),
        Some(EntityEquipmentLayerTexture {
            texture: ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
            use_player_texture: true,
        })
    );
    assert_eq!(runtime.item_equipment_wings_layer(chestplate_id), None);
    assert!(!runtime.item_equipment_asset_has_humanoid_layer(elytra_id));
    assert!(!runtime.item_equipment_asset_has_wings_layer(pumpkin_id));
    assert!(!runtime.item_equipment_asset_has_humanoid_layer(stone_id));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_head_skull_projection_resolves_static_and_profileless_player_skulls() {
    let root = unique_temp_dir("item-runtime-custom-head-skulls");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_source(
        &root,
        &[
            "skeleton_skull",
            "wither_skeleton_skull",
            "zombie_head",
            "creeper_head",
            "player_head",
            "dragon_head",
            "piglin_head",
            "carved_pumpkin",
        ],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();

    for (resource_id, expected) in [
        (
            "minecraft:skeleton_skull",
            Some(EntityCustomHeadSkull::Skeleton),
        ),
        (
            "minecraft:wither_skeleton_skull",
            Some(EntityCustomHeadSkull::WitherSkeleton),
        ),
        ("minecraft:zombie_head", Some(EntityCustomHeadSkull::Zombie)),
        (
            "minecraft:creeper_head",
            Some(EntityCustomHeadSkull::Creeper),
        ),
        (
            "minecraft:player_head",
            Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                EntityDefaultPlayerSkin::SlimSteve,
            ))),
        ),
        ("minecraft:dragon_head", Some(EntityCustomHeadSkull::Dragon)),
        ("minecraft:piglin_head", Some(EntityCustomHeadSkull::Piglin)),
        ("minecraft:carved_pumpkin", None),
    ] {
        let protocol_id = registry.protocol_id(resource_id).unwrap();
        let stack = ItemStackSummary {
            item_id: Some(protocol_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        assert_eq!(
            runtime.custom_head_skull_for_stack(&stack),
            expected,
            "{resource_id}"
        );
    }
    assert_eq!(
        runtime.custom_head_skull_for_stack(&ItemStackSummary {
            item_id: Some(999),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }),
        None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_head_skull_projection_resolves_profiled_player_head_default_skins() {
    let root = unique_temp_dir("item-runtime-custom-profiled-player-head");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_item_registry_source(&root, &["player_head"]);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let player_head_id = registry.protocol_id("minecraft:player_head").unwrap();

    let mut profiled = ItemStackSummary {
        item_id: Some(player_head_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    profiled
        .component_patch
        .added_type_ids
        .push(DATA_COMPONENT_PROFILE_TYPE_ID);
    assert_eq!(runtime.custom_head_skull_for_stack(&profiled), None);

    profiled.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: Default::default(),
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&profiled),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    let skin_url = "https://textures.minecraft.net/texture/profile-skin";
    let mut dynamic = profiled.clone();
    dynamic.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: skin_url.to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/profile-cape".to_string(),
            }),
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: profile_texture_handle(skin_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Loading,
            }
        )))
    );

    let skin_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
        root.join("skin-cache"),
        TestSkinPngFetcher {
            bytes: player_skin_png_bytes(),
            calls: skin_download_calls.clone(),
        },
    ));
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: profile_texture_handle(skin_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Loading,
            }
        )))
    );
    let downloads = drain_until_player_skin_download_result(&runtime);
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].url, skin_url);
    assert_eq!(
        downloads[0].skin.as_ref().unwrap().handle,
        profile_texture_handle(skin_url)
    );
    assert_eq!(runtime.downloaded_player_skin_count(), 1);
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: profile_texture_handle(skin_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Loading,
            }
        )))
    );
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 1);

    runtime.mark_profile_skin_resolved(skin_url, 12_345);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: 12_345,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Ready,
            }
        )))
    );

    runtime.mark_profile_skin_failed(skin_url);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&dynamic),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            EntityDynamicPlayerSkin {
                handle: profile_texture_handle(skin_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Failed,
            }
        )))
    );

    let mut resource_patched = dynamic.clone();
    resource_patched
        .component_patch
        .profile
        .as_mut()
        .unwrap()
        .skin_patch
        .body = Some(bbb_protocol::packets::ResourceTextureSummary {
        asset_id: "minecraft:entity/player/custom".to_string(),
        texture_path: "minecraft:textures/entity/player/custom.png".to_string(),
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&resource_patched),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
        ))
    );

    let mut patched = profiled.clone();
    patched.component_patch.profile = Some(ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/wide/steve".to_string(),
                texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
            }),
            cape: None,
            elytra: None,
            model: None,
        },
    });
    assert_eq!(
        runtime.custom_head_skull_for_stack(&patched),
        Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve)
        ))
    );

    let mut profile_removed = profiled.clone();
    profile_removed
        .component_patch
        .removed_type_ids
        .push(DATA_COMPONENT_PROFILE_TYPE_ID);
    assert_eq!(
        runtime.custom_head_skull_for_stack(&profile_removed),
        Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve
        )))
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_profile_queues_dynamic_cape_and_elytra_texture_downloads() {
    let root = unique_temp_dir("item-runtime-profile-texture-downloads");
    let runtime = NativeItemRuntime::empty_for_test();
    let texture_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
        root.join("profile-texture-cache"),
        TestSkinPngFetcher {
            bytes: player_profile_texture_png_bytes(),
            calls: texture_download_calls.clone(),
        },
    ));

    let cape_url = "https://textures.minecraft.net/texture/profile-cape";
    let elytra_url = "https://textures.minecraft.net/texture/profile-elytra";
    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: cape_url.to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: elytra_url.to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary::default(),
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );

    let downloads = drain_until_player_texture_download_results(&runtime, 2);
    assert_eq!(downloads.len(), 2);
    let cape = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Cape)
        .expect("cape download result");
    assert_eq!(cape.url, cape_url);
    let cape_texture = cape.texture.as_ref().expect("cape texture");
    assert_eq!(cape_texture.handle, profile_texture_handle(cape_url));
    assert_eq!(cape_texture.size, [64, 32]);

    let elytra = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Elytra)
        .expect("elytra download result");
    assert_eq!(elytra.url, elytra_url);
    let elytra_texture = elytra.texture.as_ref().expect("elytra texture");
    assert_eq!(elytra_texture.handle, profile_texture_handle(elytra_url));
    assert_eq!(elytra_texture.size, [64, 32]);

    assert_eq!(runtime.downloaded_player_texture_count(), 2);
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 2);

    runtime.player_skin_for_profile(&profile);
    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 2);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_profile_resource_texture_patch_suppresses_remote_cape_and_elytra_downloads() {
    let root = unique_temp_dir("item-runtime-profile-texture-patches");
    let runtime = NativeItemRuntime::empty_for_test();
    let texture_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
        root.join("profile-texture-cache"),
        TestSkinPngFetcher {
            bytes: player_profile_texture_png_bytes(),
            calls: texture_download_calls.clone(),
        },
    ));

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/profile-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/profile-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/custom".to_string(),
                texture_path: "minecraft:textures/entity/player/cape/custom.png".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                texture_path: "minecraft:textures/entity/player/elytra/custom.png".to_string(),
            }),
            model: None,
        },
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    for _ in 0..10 {
        assert!(runtime
            .drain_dynamic_player_texture_download_results()
            .is_empty());
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(runtime.downloaded_player_texture_count(), 0);
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_texture_patch_loads_local_cape_and_elytra_textures() {
    let root = unique_temp_dir("item-runtime-profile-resource-textures");
    let assets = assets_dir(&root);
    let cape_texture_path = "minecraft:textures/entity/player/cape/custom.png";
    let elytra_texture_path = "minecraft:textures/entity/player/elytra/custom.png";
    let cape_rgba = vec![
        10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255, 50, 0, 0, 255, 60, 0, 0, 255,
        70, 0, 0, 255, 80, 0, 0, 255,
    ];
    let elytra_rgba = vec![
        0, 10, 0, 255, 0, 20, 0, 255, 0, 30, 0, 255, 0, 40, 0, 255, 0, 50, 0, 255, 0, 60, 0, 255,
    ];
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("cape")
            .join("custom.png"),
        4,
        2,
        &cape_rgba,
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("elytra")
            .join("custom.png"),
        2,
        3,
        &elytra_rgba,
    );

    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
    let texture_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
        root.join("profile-texture-cache"),
        TestSkinPngFetcher {
            bytes: player_profile_texture_png_bytes(),
            calls: texture_download_calls.clone(),
        },
    ));

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/custom".to_string(),
                texture_path: cape_texture_path.to_string(),
            }),
            elytra: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/custom".to_string(),
                texture_path: elytra_texture_path.to_string(),
            }),
            model: None,
        },
    };

    let cape_source =
        local_profile_texture_source_id(EntityDynamicPlayerTextureKind::Cape, cape_texture_path);
    let elytra_source = local_profile_texture_source_id(
        EntityDynamicPlayerTextureKind::Elytra,
        elytra_texture_path,
    );
    let cape = runtime
        .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape)
        .expect("local cape texture");
    let elytra = runtime
        .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra)
        .expect("local elytra texture");

    assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
    assert_eq!(cape.handle, profile_texture_handle(&cape_source));
    assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
    assert_eq!(elytra.handle, profile_texture_handle(&elytra_source));
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

    let downloads = runtime.drain_dynamic_player_texture_download_results();
    assert_eq!(downloads.len(), 2);
    let cape_download = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Cape)
        .expect("cape upload");
    assert_eq!(cape_download.url, cape_source);
    let cape_image = cape_download.texture.as_ref().expect("cape image");
    assert_eq!(cape_image.handle, cape.handle);
    assert_eq!(cape_image.size, [4, 2]);
    assert_eq!(cape_image.rgba, cape_rgba);

    let elytra_download = downloads
        .iter()
        .find(|download| download.kind == DynamicPlayerTextureKind::Elytra)
        .expect("elytra upload");
    assert_eq!(elytra_download.url, elytra_source);
    let elytra_image = elytra_download.texture.as_ref().expect("elytra image");
    assert_eq!(elytra_image.handle, elytra.handle);
    assert_eq!(elytra_image.size, [2, 3]);
    assert_eq!(elytra_image.rgba, elytra_rgba);

    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_texture_patch_failure_does_not_use_remote_or_stale_texture() {
    let root = unique_temp_dir("item-runtime-profile-resource-texture-failure");
    let assets = assets_dir(&root);
    let cape_texture_path = "minecraft:textures/entity/player/cape/missing.png";
    let elytra_texture_path = "minecraft:textures/entity/player/elytra/missing.png";
    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
    let texture_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_texture_downloads_for_test(AsyncDynamicPlayerTextureRuntime::new(
        root.join("profile-texture-cache"),
        TestSkinPngFetcher {
            bytes: player_profile_texture_png_bytes(),
            calls: texture_download_calls.clone(),
        },
    ));
    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: None,
            cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-cape".to_string(),
            }),
            elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-elytra".to_string(),
            }),
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: None,
            cape: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/cape/missing".to_string(),
                texture_path: cape_texture_path.to_string(),
            }),
            elytra: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra/missing".to_string(),
                texture_path: elytra_texture_path.to_string(),
            }),
            model: None,
        },
    };

    assert_eq!(
        runtime.player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
        None
    );
    assert_eq!(
        runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra),
        None
    );
    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_texture_count(), 0);
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("cape")
            .join("missing.png"),
        1,
        1,
        &[1, 2, 3, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("elytra")
            .join("missing.png"),
        1,
        1,
        &[4, 5, 6, 255],
    );

    assert_eq!(
        runtime.player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Cape),
        None
    );
    assert_eq!(
        runtime
            .player_profile_texture_for_profile(&profile, EntityDynamicPlayerTextureKind::Elytra),
        None
    );
    assert!(runtime
        .drain_dynamic_player_texture_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_texture_count(), 0);
    assert_eq!(texture_download_calls.load(Ordering::Relaxed), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_texture_patch_loads_local_body_skin() {
    let root = unique_temp_dir("item-runtime-profile-resource-body-skin");
    let assets = assets_dir(&root);
    let body_texture_path = "minecraft:textures/entity/player/body/custom.png";
    let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
    for y in 0..64 {
        for x in 0..64 {
            body_rgba.extend_from_slice(&[x as u8, y as u8, 127, 255]);
        }
    }
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("body")
            .join("custom.png"),
        64,
        64,
        &body_rgba,
    );

    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
    let skin_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
        root.join("skin-cache"),
        TestSkinPngFetcher {
            bytes: player_skin_png_bytes(),
            calls: skin_download_calls.clone(),
        },
    ));

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/body/custom".to_string(),
                texture_path: body_texture_path.to_string(),
            }),
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    };
    let body_source = local_player_skin_source_id(body_texture_path);
    let expected_skin = EntityDynamicPlayerSkin {
        handle: profile_texture_handle(&body_source),
        fallback: EntityDefaultPlayerSkin::SlimAlex,
        model: EntityPlayerSkinModel::Wide,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(expected_skin)
    );
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

    let downloads = runtime.drain_dynamic_player_skin_download_results();
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].url, body_source);
    let image = downloads[0].skin.as_ref().expect("local body skin image");
    assert_eq!(image.handle, expected_skin.handle);
    assert_eq!(image.rgba, body_rgba);

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::Dynamic(expected_skin)
    );
    assert!(runtime
        .drain_dynamic_player_skin_download_results()
        .is_empty());
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn player_profile_resource_body_patch_failure_does_not_use_remote_or_stale_skin() {
    let root = unique_temp_dir("item-runtime-profile-resource-body-skin-failure");
    let assets = assets_dir(&root);
    let body_texture_path = "minecraft:textures/entity/player/body/missing.png";
    let mut runtime = NativeItemRuntime::empty_for_test();
    runtime.profile_texture_resources =
        PackResourceStack::from_roots([root.join("sources").join(bbb_pack::MC_VERSION)]);
    let skin_download_calls = Arc::new(AtomicUsize::new(0));
    runtime.enable_player_skin_downloads_for_test(AsyncDynamicPlayerSkinRuntime::new(
        root.join("skin-cache"),
        TestSkinPngFetcher {
            bytes: player_skin_png_bytes(),
            calls: skin_download_calls.clone(),
        },
    ));

    let profile = ResolvableProfileSummary {
        kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(0)),
        name: None,
        properties: Vec::new(),
        profile_textures: Some(bbb_protocol::packets::ProfileTexturesSummary {
            skin: Some(bbb_protocol::packets::ProfileSkinTextureSummary {
                url: "https://textures.minecraft.net/texture/remote-skin".to_string(),
                model: bbb_protocol::packets::PlayerModelTypeSummary::Slim,
            }),
            cape: None,
            elytra: None,
        }),
        skin_patch: bbb_protocol::packets::PlayerSkinPatchSummary {
            body: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/body/missing".to_string(),
                texture_path: body_texture_path.to_string(),
            }),
            cape: None,
            elytra: None,
            model: Some(bbb_protocol::packets::PlayerModelTypeSummary::Wide),
        },
    };

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    assert!(runtime
        .drain_dynamic_player_skin_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_skin_count(), 0);
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

    let mut body_rgba = Vec::with_capacity((64 * 64 * 4) as usize);
    for y in 0..64 {
        for x in 0..64 {
            body_rgba.extend_from_slice(&[x as u8, y as u8, 191, 255]);
        }
    }
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("entity")
            .join("player")
            .join("body")
            .join("missing.png"),
        64,
        64,
        &body_rgba,
    );

    assert_eq!(
        runtime.player_skin_for_profile(&profile),
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::SlimAlex)
    );
    assert!(runtime
        .drain_dynamic_player_skin_download_results()
        .is_empty());
    assert_eq!(runtime.downloaded_player_skin_count(), 0);
    assert_eq!(skin_download_calls.load(Ordering::Relaxed), 0);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn native_item_runtime_projects_llama_body_decor_colors() {
    let root = unique_temp_dir("item-runtime-llama-decor");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item WHITE_CARPET = registerBlock(Blocks.WHITE_CARPET, p -> p.component(DataComponents.EQUIPPABLE, Equippable.llamaSwag(DyeColor.WHITE)));
                public static final Item BLACK_CARPET = registerBlock(Blocks.BLACK_CARPET, p -> p.component(DataComponents.EQUIPPABLE, Equippable.llamaSwag(DyeColor.BLACK)));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let white = registry.protocol_id("minecraft:white_carpet").unwrap();
    let black = registry.protocol_id("minecraft:black_carpet").unwrap();
    let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
    let colors = runtime.llama_body_decor_colors_by_protocol_id();

    assert_eq!(runtime.llama_body_decor_color_count(), 2);
    assert_eq!(colors.get(&white), Some(&WorldLlamaBodyDecorColor::White));
    assert_eq!(colors.get(&black), Some(&WorldLlamaBodyDecorColor::Black));
    assert_eq!(colors.get(&horse_armor), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_projects_nautilus_body_armor_materials() {
    let root = unique_temp_dir("item-runtime-nautilus-body-armor");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item IRON_NAUTILUS_ARMOR = registerItem("iron_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.IRON));
                public static final Item GOLDEN_NAUTILUS_ARMOR = registerItem("golden_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.GOLD));
                public static final Item NETHERITE_NAUTILUS_ARMOR = registerItem("netherite_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.NETHERITE).fireResistant());
                public static final Item CHAINMAIL_NAUTILUS_ARMOR = registerItem("chainmail_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.CHAINMAIL));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let iron = registry
        .protocol_id("minecraft:iron_nautilus_armor")
        .unwrap();
    let gold = registry
        .protocol_id("minecraft:golden_nautilus_armor")
        .unwrap();
    let netherite = registry
        .protocol_id("minecraft:netherite_nautilus_armor")
        .unwrap();
    let chainmail = registry
        .protocol_id("minecraft:chainmail_nautilus_armor")
        .unwrap();
    let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
    let materials = runtime.nautilus_body_armor_materials_by_protocol_id();

    assert_eq!(runtime.nautilus_body_armor_material_count(), 3);
    assert_eq!(materials.get(&iron), Some(&WorldArmorMaterialKind::Iron));
    assert_eq!(materials.get(&gold), Some(&WorldArmorMaterialKind::Gold));
    assert_eq!(
        materials.get(&netherite),
        Some(&WorldArmorMaterialKind::Netherite)
    );
    assert_eq!(materials.get(&chainmail), None);
    assert_eq!(materials.get(&horse_armor), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_projects_horse_body_armor_materials() {
    let root = unique_temp_dir("item-runtime-horse-body-armor");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item LEATHER_HORSE_ARMOR = registerItem("leather_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.LEATHER));
                public static final Item COPPER_HORSE_ARMOR = registerItem("copper_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.COPPER));
                public static final Item DIAMOND_HORSE_ARMOR = registerItem("diamond_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
                public static final Item NETHERITE_HORSE_ARMOR = registerItem("netherite_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.NETHERITE).fireResistant());
                public static final Item CHAINMAIL_HORSE_ARMOR = registerItem("chainmail_horse_armor", new Item.Properties().horseArmor(ArmorMaterials.CHAINMAIL));
                public static final Item IRON_NAUTILUS_ARMOR = registerItem("iron_nautilus_armor", new Item.Properties().nautilusArmor(ArmorMaterials.IRON));
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let leather = registry
        .protocol_id("minecraft:leather_horse_armor")
        .unwrap();
    let copper = registry
        .protocol_id("minecraft:copper_horse_armor")
        .unwrap();
    let diamond = registry
        .protocol_id("minecraft:diamond_horse_armor")
        .unwrap();
    let netherite = registry
        .protocol_id("minecraft:netherite_horse_armor")
        .unwrap();
    let chainmail = registry
        .protocol_id("minecraft:chainmail_horse_armor")
        .unwrap();
    let nautilus = registry
        .protocol_id("minecraft:iron_nautilus_armor")
        .unwrap();
    let materials = runtime.horse_body_armor_materials_by_protocol_id();

    assert_eq!(runtime.horse_body_armor_material_count(), 4);
    assert_eq!(
        materials.get(&leather),
        Some(&WorldArmorMaterialKind::Leather)
    );
    assert_eq!(
        materials.get(&copper),
        Some(&WorldArmorMaterialKind::Copper)
    );
    assert_eq!(
        materials.get(&diamond),
        Some(&WorldArmorMaterialKind::Diamond)
    );
    assert_eq!(
        materials.get(&netherite),
        Some(&WorldArmorMaterialKind::Netherite)
    );
    assert_eq!(materials.get(&chainmail), None);
    assert_eq!(materials.get(&nautilus), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_projects_wolf_body_armor_materials_and_max_damage() {
    let root = unique_temp_dir("item-runtime-wolf-body-armor");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item WOLF_ARMOR = registerItem("wolf_armor", new Item.Properties().wolfArmor(ArmorMaterials.ARMADILLO_SCUTE));
                public static final Item HORSE_ARMOR = registerItem("horse_armor", new Item.Properties().horseArmor(ArmorMaterials.DIAMOND));
            }"#,
    );
    write_json(
        &assets.join("equipment").join("armadillo_scute.json"),
        r#"{
                "layers": {
                    "wolf_body": [
                        { "texture": "minecraft:armadillo_scute" },
                        { "texture": "minecraft:armadillo_scute_overlay", "dyeable": {} }
                    ]
                }
            }"#,
    );
    write_json(
        &assets.join("equipment").join("diamond.json"),
        r#"{
                "layers": {
                    "horse_body": [
                        { "texture": "minecraft:diamond" }
                    ]
                }
            }"#,
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let registry = runtime.registry.as_ref().unwrap();
    let wolf_armor = registry.protocol_id("minecraft:wolf_armor").unwrap();
    let horse_armor = registry.protocol_id("minecraft:horse_armor").unwrap();
    let materials = runtime.wolf_body_armor_materials_by_protocol_id();
    let max_damage = runtime.item_max_damage_by_protocol_id();

    assert_eq!(runtime.wolf_body_armor_material_count(), 1);
    assert_eq!(
        materials.get(&wolf_armor),
        Some(&WorldArmorMaterialKind::ArmadilloScute)
    );
    assert_eq!(materials.get(&horse_armor), None);
    assert_eq!(max_damage.get(&wolf_armor), Some(&64));
    assert_eq!(runtime.item_max_damage_count(), 1);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_converts_mining_profiles_for_world() {
    let profile = PackItemMiningProfile {
        default_mining_speed_thousandths: 1_000,
        rules: vec![
            PackItemMiningRule {
                block_names: vec!["minecraft:obsidian".to_string()],
                mining_speed_thousandths: None,
                correct_for_drops: Some(false),
            },
            PackItemMiningRule {
                block_names: vec!["minecraft:stone".to_string()],
                mining_speed_thousandths: Some(4_000),
                correct_for_drops: Some(true),
            },
        ],
    };

    let converted = world_item_mining_profile(&profile);

    assert_eq!(converted.default_mining_speed_thousandths, 1_000);
    assert_eq!(converted.rules.len(), 2);
    assert_eq!(
        converted.rules[0].block_names,
        vec!["minecraft:obsidian".to_string()]
    );
    assert_eq!(converted.rules[0].correct_for_drops, Some(false));
    assert_eq!(
        converted.rules[1].block_names,
        vec!["minecraft:stone".to_string()]
    );
    assert_eq!(converted.rules[1].mining_speed_thousandths, Some(4_000));
    assert_eq!(converted.rules[1].correct_for_drops, Some(true));
}

#[test]
fn native_item_runtime_selects_broken_condition_icon_from_stack_damage() {
    let root = unique_temp_dir("item-runtime-broken");
    write_elytra_damage_condition_fixture(
        &root,
        "minecraft:broken",
        "elytra_broken",
        &[120, 80, 40, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    assert_eq!(
        runtime
            .registry
            .as_ref()
            .and_then(|registry| registry.max_damage("minecraft:elytra")),
        Some(432)
    );
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/elytra"))
        .unwrap();
    let broken_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/elytra_broken"))
        .unwrap();

    assert_eq!(runtime.icon_texture_count(), 1);
    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/elytra"))
    );

    let normal_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(430),
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(normal_icon.layers[0].uv, normal_uv);

    let broken_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(431),
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(broken_icon.layers[0].uv, broken_uv);

    let unbreakable_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(431),
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(unbreakable_icon.layers[0].uv, normal_uv);

    let removed_max_damage_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(431),
                removed_type_ids: vec![2],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(removed_max_damage_icon.layers[0].uv, normal_uv);

    let removed_damage_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![3],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(removed_damage_icon.layers[0].uv, normal_uv);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_damaged_condition_icon_from_stack_damage() {
    let root = unique_temp_dir("item-runtime-damaged");
    write_elytra_damage_condition_fixture(
        &root,
        "minecraft:damaged",
        "elytra_damaged",
        &[80, 120, 40, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/elytra"))
        .unwrap();
    let damaged_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/elytra_damaged"))
        .unwrap();

    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/elytra"))
    );

    let pristine_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        })
        .unwrap();
    assert_eq!(pristine_icon.layers[0].uv, normal_uv);

    let damaged_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(1),
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(damaged_icon.layers[0].uv, damaged_uv);

    let unbreakable_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(1),
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(unbreakable_icon.layers[0].uv, normal_uv);

    let removed_damage_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                damage: Some(1),
                removed_type_ids: vec![3],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(removed_damage_icon.layers[0].uv, normal_uv);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_has_component_icon_from_dyed_color() {
    let root = unique_temp_dir("item-runtime-has-dyed-color");
    write_wolf_armor_has_component_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/wolf_armor"))
        .unwrap();
    let dyed_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/wolf_armor_dyed"))
        .unwrap();

    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/wolf_armor"))
    );

    let default_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        })
        .unwrap();
    assert_eq!(default_icon.layers[0].uv, normal_uv);
    assert_eq!(default_icon.layers[0].tint, ITEM_TINT_WHITE);

    let dyed_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![44],
                dyed_color: Some(0x33_66_99),
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(dyed_icon.layers[0].uv, dyed_uv);
    assert_eq!(dyed_icon.layers[0].tint, rgb_i32_tint(0x33_66_99));

    let removed_dye_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![44],
                removed_type_ids: vec![44],
                dyed_color: Some(0x33_66_99),
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(removed_dye_icon.layers[0].uv, normal_uv);
    assert_eq!(removed_dye_icon.layers[0].tint, ITEM_TINT_WHITE);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_has_component_icon_from_lodestone_tracker() {
    let root = unique_temp_dir("item-runtime-has-lodestone");
    write_compass_has_component_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/compass"))
        .unwrap();
    let lodestone_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/compass_lodestone"))
        .unwrap();

    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/compass"))
    );

    let normal_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        })
        .unwrap();
    assert_eq!(normal_icon.layers[0].uv, normal_uv);

    let lodestone_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![67],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(lodestone_icon.layers[0].uv, lodestone_uv);

    let removed_lodestone_icon = runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![67],
                removed_type_ids: vec![67],
                ..DataComponentPatchSummary::default()
            },
        })
        .unwrap();
    assert_eq!(removed_lodestone_icon.layers[0].uv, normal_uv);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_has_component_defaults_and_nondefault_patches() {
    let root = unique_temp_dir("item-runtime-has-component-defaults");
    write_default_has_component_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |item_id, component_patch| {
        runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch,
            })
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla `HasComponent.get(ignoreDefault=false)` calls
    // `ItemStack.has`, so common prototype components count as present.
    assert_eq!(
        selected(0, DataComponentPatchSummary::default()),
        uv("has_max_stack_present")
    );
    assert_eq!(
        selected(
            0,
            DataComponentPatchSummary {
                removed_type_ids: vec![1],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_max_stack_absent")
    );

    // With `ignore_default=true`, vanilla checks hasNonDefault: both added
    // and removed patches are non-default, while the untouched prototype is
    // not.
    assert_eq!(
        selected(1, DataComponentPatchSummary::default()),
        uv("has_max_stack_unpatched")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![1],
                max_stack_size: Some(16),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_max_stack_patched")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                removed_type_ids: vec![1],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_max_stack_patched")
    );

    // `rarity=common` is also in vanilla `COMMON_ITEM_COMPONENTS`.
    assert_eq!(
        selected(2, DataComponentPatchSummary::default()),
        uv("has_rarity_present")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                removed_type_ids: vec![12],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_rarity_absent")
    );
    assert_eq!(
        selected(3, DataComponentPatchSummary::default()),
        uv("has_enchantments_present")
    );
    assert_eq!(
        selected(
            3,
            DataComponentPatchSummary {
                removed_type_ids: vec![13],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_enchantments_absent")
    );
    assert_eq!(
        selected(4, DataComponentPatchSummary::default()),
        uv("has_stored_enchantments_present")
    );
    assert_eq!(
        selected(
            4,
            DataComponentPatchSummary {
                removed_type_ids: vec![42],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("has_stored_enchantments_absent")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_component_condition_predicates() {
    let root = unique_temp_dir("item-runtime-component-condition");
    write_component_condition_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |item_id, component_patch| {
        runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch,
            })
            .unwrap()
            .layers[0]
            .uv
    };
    let trim_keys = [
        "minecraft:quartz".to_string(),
        "minecraft:diamond".to_string(),
    ];
    let enchantment_keys = [
        "minecraft:sharpness".to_string(),
        "minecraft:mending".to_string(),
    ];
    let attribute_keys = [
        "minecraft:generic.attack_damage".to_string(),
        "minecraft:generic.scale".to_string(),
        "minecraft:generic.armor".to_string(),
        "minecraft:generic.attack_speed".to_string(),
    ];
    let healing_potion_id = 24;
    let selected_with_trim_keys = |item_id, component_patch| {
        runtime
            .icon_for_stack_with_context(
                &ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch,
                },
                None,
                false,
                0.0,
                Some(&trim_keys),
                None,
                None,
                None,
            )
            .unwrap()
            .layers[0]
            .uv
    };
    let selected_with_enchantment_keys = |item_id, component_patch| {
        runtime
            .icon_for_stack_with_model_registry_context(
                &ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch,
                },
                None,
                false,
                ItemModelUseContext::inactive(),
                BlockModelDisplayContext::Gui,
                0.0,
                None,
                Some(&enchantment_keys),
                None,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
                ItemModelKeybindContext::default(),
                false,
            )
            .unwrap()
            .layers[0]
            .uv
    };
    let selected_with_attribute_keys = |item_id, component_patch| {
        runtime
            .icon_for_stack_with_model_registry_context(
                &ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch,
                },
                None,
                false,
                ItemModelUseContext::inactive(),
                BlockModelDisplayContext::Gui,
                0.0,
                None,
                None,
                Some(&attribute_keys),
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
                ItemModelKeybindContext::default(),
                false,
            )
            .unwrap()
            .layers[0]
            .uv
    };

    // `ComponentMatches` with a component-type discriminator uses
    // `AnyValue.matches`, so vanilla common default components are present.
    assert_eq!(
        selected(0, DataComponentPatchSummary::default()),
        uv("component_condition_rarity_present")
    );
    assert_eq!(
        selected(
            0,
            DataComponentPatchSummary {
                removed_type_ids: vec![12],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_rarity_absent")
    );

    // Non-default components are present only when the stack patch carries
    // that component, regardless of the component's boolean payload.
    assert_eq!(
        selected(1, DataComponentPatchSummary::default()),
        uv("component_condition_glint_absent")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![21],
                enchantment_glint_override: Some(false),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_glint_present")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![21],
                enchantment_glint_override: Some(false),
                removed_type_ids: vec![21],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_glint_absent")
    );

    // Vanilla `DamagePredicate.matches` requires the `minecraft:damage`
    // component and matches both damage and durability (`max_damage -
    // damage`) with `MinMaxBounds.Ints`.
    assert_eq!(
        selected(2, DataComponentPatchSummary::default()),
        uv("component_condition_damage_absent")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                added_type_ids: vec![3],
                damage: Some(3),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_damage_present")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                added_type_ids: vec![3],
                damage: Some(4),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_damage_absent")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                added_type_ids: vec![3],
                removed_type_ids: vec![3],
                damage: Some(3),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_damage_absent")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                added_type_ids: vec![3],
                removed_type_ids: vec![2],
                damage: Some(3),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_damage_absent")
    );

    for (item_id, component_id, model_id) in [
        (3, 50, "component_condition_bundle_contents"),
        (4, 56, "component_condition_trim"),
        (5, 68, "component_condition_firework_explosion"),
        (6, 69, "component_condition_fireworks"),
        (7, 64, "component_condition_jukebox_playable"),
        (8, 75, "component_condition_container"),
    ] {
        assert_eq!(
            selected(item_id, DataComponentPatchSummary::default()),
            uv(&format!("{model_id}_absent"))
        );
        assert_eq!(
            selected(
                item_id,
                DataComponentPatchSummary {
                    added_type_ids: vec![component_id],
                    ..DataComponentPatchSummary::default()
                }
            ),
            uv(&format!("{model_id}_present"))
        );
        assert_eq!(
            selected(
                item_id,
                DataComponentPatchSummary {
                    added_type_ids: vec![component_id],
                    removed_type_ids: vec![component_id],
                    ..DataComponentPatchSummary::default()
                }
            ),
            uv(&format!("{model_id}_absent"))
        );
    }

    assert_eq!(
        selected(9, DataComponentPatchSummary::default()),
        uv("component_condition_bundle_contents_constrained_absent")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contents_constrained_present")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contents_constrained_absent")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contents_constrained_absent")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                removed_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contents_constrained_absent")
    );
    assert_eq!(
        selected(
            21,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contains_present")
    );
    assert_eq!(
        selected(
            21,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contains_absent")
    );
    assert_eq!(
        selected(
            21,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                removed_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_contains_absent")
    );
    assert_eq!(
        selected(
            22,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(3),
                bundle_contents_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_count_present")
    );
    assert_eq!(
        selected(
            22,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(2),
                bundle_contents_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_count_absent")
    );
    assert_eq!(
        selected(
            23,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 1,
                    count: 4,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_contains_present")
    );
    assert_eq!(
        selected(
            23,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 1,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_contains_absent")
    );
    assert_eq!(
        selected(
            23,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                removed_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 1,
                    count: 4,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_contains_absent")
    );
    assert_eq!(
        selected(
            24,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_count_present")
    );
    assert_eq!(
        selected(
            24,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_count_absent")
    );
    assert_eq!(
        selected(
            28,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 2,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_tag_contains_present")
    );
    assert_eq!(
        selected(
            28,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 2,
                    count: 2,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_tag_contains_absent")
    );
    assert_eq!(
        selected(
            28,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                removed_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 2,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_tag_contains_absent")
    );
    assert_eq!(
        selected(
            29,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(3),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_tag_count_present")
    );
    assert_eq!(
        selected(
            29,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_tag_count_absent")
    );

    let star_trail = DataComponentPatchSummary {
        added_type_ids: vec![68],
        firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
        firework_explosion_has_trail: Some(true),
        firework_explosion_has_twinkle: Some(false),
        ..DataComponentPatchSummary::default()
    };
    let fireworks_explosion =
        |shape: FireworkExplosionShapeSummary, has_trail: bool, has_twinkle: bool| {
            FireworkExplosionSummary {
                shape,
                colors: Vec::new(),
                fade_colors: Vec::new(),
                has_trail,
                has_twinkle,
            }
        };
    assert_eq!(
        selected(10, star_trail.clone()),
        uv("component_condition_firework_explosion_star_present")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                firework_explosion_shape: Some(FireworkExplosionShapeSummary::Burst),
                ..star_trail.clone()
            }
        ),
        uv("component_condition_firework_explosion_star_absent")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                firework_explosion_has_twinkle: Some(true),
                ..star_trail.clone()
            }
        ),
        uv("component_condition_firework_explosion_star_absent")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                added_type_ids: vec![68],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_firework_explosion_star_absent")
    );

    assert_eq!(
        selected(
            11,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_flight_present")
    );
    assert_eq!(
        selected(
            11,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(4),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_flight_absent")
    );
    assert_eq!(
        selected(
            11,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_flight_absent")
    );
    assert_eq!(
        selected(
            11,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                removed_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_flight_absent")
    );
    assert_eq!(
        selected(
            12,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                fireworks_explosions_count: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_explosions_present")
    );
    assert_eq!(
        selected(
            12,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                fireworks_explosions_count: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_explosions_absent")
    );
    assert_eq!(
        selected(
            12,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_explosions_absent")
    );
    assert_eq!(
        selected(
            12,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                removed_type_ids: vec![69],
                fireworks_flight_duration: Some(2),
                fireworks_explosions_count: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_explosions_absent")
    );
    assert_eq!(
        selected(
            19,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(2),
                fireworks_explosions: vec![
                    fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                    fireworks_explosion(FireworkExplosionShapeSummary::Star, true, false),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_contains_present")
    );
    assert_eq!(
        selected(
            19,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(1),
                fireworks_explosions: vec![fireworks_explosion(
                    FireworkExplosionShapeSummary::Star,
                    false,
                    false,
                )],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_contains_absent")
    );
    assert_eq!(
        selected(
            19,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                removed_type_ids: vec![69],
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(1),
                fireworks_explosions: vec![fireworks_explosion(
                    FireworkExplosionShapeSummary::Star,
                    true,
                    false,
                )],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_contains_absent")
    );
    assert_eq!(
        selected(
            20,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(3),
                fireworks_explosions: vec![
                    fireworks_explosion(FireworkExplosionShapeSummary::Star, true, true),
                    fireworks_explosion(FireworkExplosionShapeSummary::Burst, false, true),
                    fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_count_present")
    );
    assert_eq!(
        selected(
            20,
            DataComponentPatchSummary {
                added_type_ids: vec![69],
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(2),
                fireworks_explosions: vec![
                    fireworks_explosion(FireworkExplosionShapeSummary::Star, true, true),
                    fireworks_explosion(FireworkExplosionShapeSummary::SmallBall, false, false),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_fireworks_count_absent")
    );

    assert_eq!(
        selected(13, DataComponentPatchSummary::default()),
        uv("component_condition_trim_material_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            13,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            13,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_absent")
    );
    assert_eq!(
        selected(
            13,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            13,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                removed_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            14,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            14,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            14,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            14,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                removed_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            34,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_tag_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            34,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_tag_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            34,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                removed_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_material_tag_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            35,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_tag_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            35,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_tag_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            35,
            DataComponentPatchSummary {
                added_type_ids: vec![56],
                removed_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_trim_pattern_tag_absent")
    );

    assert_eq!(
        selected(15, DataComponentPatchSummary::default()),
        uv("component_condition_enchantments_level_absent")
    );
    assert_eq!(
        selected(
            15,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_level_present")
    );
    assert_eq!(
        selected(
            15,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_level_absent")
    );
    assert_eq!(
        selected(
            15,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_level_absent")
    );
    assert_eq!(
        selected(
            15,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                removed_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_level_absent")
    );
    assert_eq!(
        selected(16, DataComponentPatchSummary::default()),
        uv("component_condition_enchantments_empty_present")
    );
    assert_eq!(
        selected(
            16,
            DataComponentPatchSummary {
                removed_type_ids: vec![13],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_empty_absent")
    );

    assert_eq!(
        selected(17, DataComponentPatchSummary::default()),
        uv("component_condition_stored_enchantments_level_absent")
    );
    assert_eq!(
        selected(
            17,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 11,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_level_present")
    );
    assert_eq!(
        selected(
            17,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 11,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_level_absent")
    );
    assert_eq!(
        selected(
            17,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                removed_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 11,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_level_absent")
    );
    assert_eq!(
        selected(18, DataComponentPatchSummary::default()),
        uv("component_condition_stored_enchantments_empty_absent")
    );
    assert_eq!(
        selected(
            18,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_empty_present")
    );
    assert_eq!(
        selected(27, DataComponentPatchSummary::default()),
        uv("component_condition_stored_enchantments_default_present")
    );
    assert_eq!(
        selected(
            27,
            DataComponentPatchSummary {
                removed_type_ids: vec![42],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_default_absent")
    );
    assert_eq!(
        selected(
            25,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            25,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_holder_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            25,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            25,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            25,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                removed_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            26,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_holder_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            26,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            26,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 2,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_holder_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            26,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                removed_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_holder_absent")
    );
    assert_eq!(
        selected(
            30,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_tag_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            30,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_tag_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            30,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 3,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_tag_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            30,
            DataComponentPatchSummary {
                added_type_ids: vec![13],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_enchantments_tag_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            31,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_tag_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            31,
            DataComponentPatchSummary {
                added_type_ids: vec![42],
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_stored_enchantments_tag_absent")
    );
    assert_eq!(
        selected(
            32,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        rarity: Some(ItemRaritySummary::Rare),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_components_present")
    );
    assert_eq!(
        selected(
            32,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_components_absent")
    );
    assert_eq!(
        selected(
            32,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        rarity: Some(ItemRaritySummary::Rare),
                        removed_type_ids: vec![12],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_components_absent")
    );
    let named_bundle_entry = |component_patch| DataComponentPatchSummary {
        added_type_ids: vec![50],
        bundle_contents_item_count: Some(1),
        bundle_contents_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch,
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(
            81,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![6, 9],
                custom_name: Some("Custom Name".to_string()),
                item_name: Some("Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_component_text_present")
    );
    assert_eq!(
        selected(
            81,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![6],
                custom_name: Some("Custom Name".to_string()),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_component_text_absent")
    );
    assert_eq!(
        selected(
            81,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![6, 9],
                removed_type_ids: vec![6],
                custom_name: Some("Custom Name".to_string()),
                item_name: Some("Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_component_text_absent")
    );
    assert_eq!(
        selected(
            82,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![11],
                lore: vec!["Lore one".to_string(), "Lore two".to_string()],
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_lore_present")
    );
    assert_eq!(
        selected(
            82,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![11],
                lore: vec!["Lore two".to_string(), "Lore one".to_string()],
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_lore_absent")
    );
    assert_eq!(
        selected(
            82,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![11],
                removed_type_ids: vec![11],
                lore: vec!["Lore one".to_string(), "Lore two".to_string()],
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_lore_absent")
    );
    assert_eq!(
        selected(
            83,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![4],
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_unbreakable_present")
    );
    assert_eq!(
        selected(83, named_bundle_entry(DataComponentPatchSummary::default())),
        uv("component_condition_bundle_exact_unbreakable_absent")
    );
    assert_eq!(
        selected(
            83,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![4],
                removed_type_ids: vec![4],
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_unbreakable_absent")
    );
    let exact_custom_data = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "owner".to_string(),
            value: NbtSummaryValue::String("Alex".to_string()),
        },
        NbtSummaryEntry {
            name: "level".to_string(),
            value: NbtSummaryValue::Int(7),
        },
    ]);
    assert_eq!(
        selected(
            84,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![0],
                custom_data: Some(exact_custom_data.clone()),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_custom_data_present")
    );
    assert_eq!(
        selected(
            84,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![0],
                custom_data: Some(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "owner".to_string(),
                        value: NbtSummaryValue::String("Alex".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "level".to_string(),
                        value: NbtSummaryValue::Int(7),
                    },
                    NbtSummaryEntry {
                        name: "extra".to_string(),
                        value: NbtSummaryValue::Byte(1),
                    },
                ])),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_custom_data_absent")
    );
    assert_eq!(
        selected(
            84,
            named_bundle_entry(DataComponentPatchSummary {
                added_type_ids: vec![0],
                removed_type_ids: vec![0],
                custom_data: Some(exact_custom_data),
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_exact_custom_data_absent")
    );
    let exact_potion_effect = MobEffectInstanceSummary {
        effect_id: 2,
        amplifier: 1,
        duration: 200,
        ambient: false,
        show_particles: true,
        show_icon: true,
        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
            amplifier: 0,
            duration: 40,
            ambient: false,
            show_particles: true,
            show_icon: false,
            hidden_effect: None,
        })),
    };
    let exact_potion_contents = DataComponentPatchSummary {
        added_type_ids: vec![51],
        potion_id: Some(healing_potion_id),
        potion_custom_color: Some(0x77_88_99),
        potion_custom_effect_count: Some(1),
        potion_custom_effects: vec![exact_potion_effect.clone()],
        potion_custom_name: Some("healing".to_string()),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(85, named_bundle_entry(exact_potion_contents.clone())),
        uv("component_condition_bundle_exact_potion_contents_present")
    );
    assert_eq!(
        selected(
            85,
            named_bundle_entry(DataComponentPatchSummary {
                potion_custom_effects: vec![MobEffectInstanceSummary {
                    duration: 201,
                    ..exact_potion_effect
                }],
                ..exact_potion_contents.clone()
            })
        ),
        uv("component_condition_bundle_exact_potion_contents_absent")
    );
    assert_eq!(
        selected(
            85,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![51],
                ..exact_potion_contents
            })
        ),
        uv("component_condition_bundle_exact_potion_contents_absent")
    );
    let exact_writable_book = DataComponentPatchSummary {
        added_type_ids: vec![54],
        writable_book_pages: vec!["alpha".to_string(), "beta".to_string()],
        writable_book_page_filters: vec![None, Some("filtered beta".to_string())],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(86, named_bundle_entry(exact_writable_book.clone())),
        uv("component_condition_bundle_exact_writable_book_present")
    );
    assert_eq!(
        selected(
            86,
            named_bundle_entry(DataComponentPatchSummary {
                writable_book_page_filters: vec![None, Some("other filtered".to_string())],
                ..exact_writable_book.clone()
            })
        ),
        uv("component_condition_bundle_exact_writable_book_absent")
    );
    assert_eq!(
        selected(
            86,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![54],
                ..exact_writable_book
            })
        ),
        uv("component_condition_bundle_exact_writable_book_absent")
    );
    let exact_firework_explosion = DataComponentPatchSummary {
        added_type_ids: vec![68],
        firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
        firework_explosion_colors: vec![0x11_22_33],
        firework_explosion_fade_colors: vec![0x44_55_66],
        firework_explosion_has_trail: Some(true),
        firework_explosion_has_twinkle: Some(false),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(87, named_bundle_entry(exact_firework_explosion.clone())),
        uv("component_condition_bundle_exact_firework_explosion_present")
    );
    assert_eq!(
        selected(
            87,
            named_bundle_entry(DataComponentPatchSummary {
                firework_explosion_fade_colors: vec![0x01_02_03],
                ..exact_firework_explosion.clone()
            })
        ),
        uv("component_condition_bundle_exact_firework_explosion_absent")
    );
    assert_eq!(
        selected(
            87,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![68],
                ..exact_firework_explosion
            })
        ),
        uv("component_condition_bundle_exact_firework_explosion_absent")
    );
    let exact_fireworks = DataComponentPatchSummary {
        added_type_ids: vec![69],
        fireworks_flight_duration: Some(2),
        fireworks_explosions_count: Some(1),
        fireworks_explosions: vec![FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x11_22_33],
            fade_colors: vec![0x44_55_66],
            has_trail: true,
            has_twinkle: false,
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(88, named_bundle_entry(exact_fireworks.clone())),
        uv("component_condition_bundle_exact_fireworks_present")
    );
    assert_eq!(
        selected(
            88,
            named_bundle_entry(DataComponentPatchSummary {
                fireworks_flight_duration: Some(1),
                ..exact_fireworks.clone()
            })
        ),
        uv("component_condition_bundle_exact_fireworks_absent")
    );
    assert_eq!(
        selected(
            88,
            named_bundle_entry(DataComponentPatchSummary {
                fireworks_explosions: vec![FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Star,
                    colors: vec![0x11_22_33],
                    fade_colors: vec![0x01_02_03],
                    has_trail: true,
                    has_twinkle: false,
                }],
                ..exact_fireworks
            })
        ),
        uv("component_condition_bundle_exact_fireworks_absent")
    );
    let exact_jukebox_playable = DataComponentPatchSummary {
        added_type_ids: vec![64],
        jukebox_direct_song: Some(JukeboxSongSummary {
            sound_event: SoundEventSummary {
                registry_id: Some(286),
                sound_id: None,
                fixed_range_bits: None,
            },
            description: "Test song".to_string(),
            length_in_seconds_bits: 3.5f32.to_bits(),
            comparator_output: 7,
        }),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(89, named_bundle_entry(exact_jukebox_playable.clone())),
        uv("component_condition_bundle_exact_jukebox_playable_present")
    );
    assert_eq!(
        selected(
            89,
            named_bundle_entry(DataComponentPatchSummary {
                jukebox_direct_song: Some(JukeboxSongSummary {
                    comparator_output: 8,
                    ..exact_jukebox_playable.jukebox_direct_song.clone().unwrap()
                }),
                ..exact_jukebox_playable.clone()
            })
        ),
        uv("component_condition_bundle_exact_jukebox_playable_absent")
    );
    assert_eq!(
        selected(
            89,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![64],
                ..exact_jukebox_playable
            })
        ),
        uv("component_condition_bundle_exact_jukebox_playable_absent")
    );
    let exact_trim = DataComponentPatchSummary {
        added_type_ids: vec![56],
        armor_trim_material_direct: Some(TrimMaterialSummary {
            asset_name: "test_material".to_string(),
            override_armor_assets: BTreeMap::from([(
                "minecraft:iron".to_string(),
                "test_material_iron".to_string(),
            )]),
            description: "Test material".to_string(),
        }),
        armor_trim_pattern_direct: Some(TrimPatternSummary {
            asset_id: "minecraft:test_pattern".to_string(),
            description: "Test pattern".to_string(),
            decal: true,
        }),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_trim_keys(90, named_bundle_entry(exact_trim.clone())),
        uv("component_condition_bundle_exact_trim_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            90,
            named_bundle_entry(DataComponentPatchSummary {
                armor_trim_pattern_direct: Some(TrimPatternSummary {
                    decal: false,
                    ..exact_trim.armor_trim_pattern_direct.clone().unwrap()
                }),
                ..exact_trim.clone()
            })
        ),
        uv("component_condition_bundle_exact_trim_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            90,
            named_bundle_entry(DataComponentPatchSummary {
                armor_trim_material_direct: Some(TrimMaterialSummary {
                    description: "Other material".to_string(),
                    ..exact_trim.armor_trim_material_direct.clone().unwrap()
                }),
                ..exact_trim.clone()
            })
        ),
        uv("component_condition_bundle_exact_trim_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            90,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![56],
                ..exact_trim
            })
        ),
        uv("component_condition_bundle_exact_trim_absent")
    );
    let exact_enchantments = DataComponentPatchSummary {
        added_type_ids: vec![13],
        enchantments: vec![
            bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            },
            bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 1,
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_enchantment_keys(91, named_bundle_entry(exact_enchantments.clone())),
        uv("component_condition_bundle_exact_enchantments_present")
    );
    assert_eq!(
        selected(91, named_bundle_entry(exact_enchantments.clone())),
        uv("component_condition_bundle_exact_enchantments_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            91,
            named_bundle_entry(DataComponentPatchSummary {
                enchantments: vec![
                    bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 3,
                    },
                    bbb_protocol::packets::ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 2,
                    },
                ],
                ..exact_enchantments.clone()
            })
        ),
        uv("component_condition_bundle_exact_enchantments_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            91,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![13],
                ..exact_enchantments
            })
        ),
        uv("component_condition_bundle_exact_enchantments_absent")
    );
    let exact_stored_enchantments = DataComponentPatchSummary {
        added_type_ids: vec![42],
        stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 1,
            level: 1,
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_enchantment_keys(92, named_bundle_entry(exact_stored_enchantments.clone())),
        uv("component_condition_bundle_exact_stored_enchantments_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            92,
            named_bundle_entry(DataComponentPatchSummary {
                stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..exact_stored_enchantments.clone()
            })
        ),
        uv("component_condition_bundle_exact_stored_enchantments_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            92,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![42],
                ..exact_stored_enchantments
            })
        ),
        uv("component_condition_bundle_exact_stored_enchantments_absent")
    );
    let exact_villager_variant = DataComponentPatchSummary {
        added_type_ids: vec![83],
        villager_variant_id: Some(2),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(93, named_bundle_entry(exact_villager_variant.clone())),
        uv("component_condition_bundle_exact_villager_variant_present")
    );
    assert_eq!(
        selected(
            93,
            named_bundle_entry(DataComponentPatchSummary {
                villager_variant_id: Some(0),
                ..exact_villager_variant.clone()
            })
        ),
        uv("component_condition_bundle_exact_villager_variant_absent")
    );
    assert_eq!(
        selected(
            93,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![83],
                ..exact_villager_variant
            })
        ),
        uv("component_condition_bundle_exact_villager_variant_absent")
    );
    let exact_lodestone_tracker = DataComponentPatchSummary {
        added_type_ids: vec![67],
        lodestone_target: Some(LodestoneTargetSummary {
            dimension: "minecraft:overworld".to_string(),
            pos: bbb_protocol::packets::chunks::BlockPos {
                x: 10,
                y: 64,
                z: -4,
            },
        }),
        lodestone_tracked: Some(false),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(94, named_bundle_entry(exact_lodestone_tracker.clone())),
        uv("component_condition_bundle_exact_lodestone_tracker_present")
    );
    assert_eq!(
        selected(
            94,
            named_bundle_entry(DataComponentPatchSummary {
                lodestone_tracked: Some(true),
                ..exact_lodestone_tracker.clone()
            })
        ),
        uv("component_condition_bundle_exact_lodestone_tracker_absent")
    );
    assert_eq!(
        selected(
            94,
            named_bundle_entry(DataComponentPatchSummary {
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:the_nether".to_string(),
                    ..exact_lodestone_tracker.lodestone_target.clone().unwrap()
                }),
                ..exact_lodestone_tracker.clone()
            })
        ),
        uv("component_condition_bundle_exact_lodestone_tracker_absent")
    );
    assert_eq!(
        selected(
            94,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![67],
                ..exact_lodestone_tracker
            })
        ),
        uv("component_condition_bundle_exact_lodestone_tracker_absent")
    );
    let exact_attribute_modifiers = DataComponentPatchSummary {
        added_type_ids: vec![16],
        attribute_modifiers: vec![
            AttributeModifierSummary {
                attribute_id: 0,
                modifier_id: "minecraft:test/attack".to_string(),
                amount_bits: 2.5f64.to_bits(),
                operation_id: 0,
                slot_id: 1,
                display_id: 1,
                display_text: None,
            },
            AttributeModifierSummary {
                attribute_id: 1,
                modifier_id: "minecraft:test/scale".to_string(),
                amount_bits: 1.25f64.to_bits(),
                operation_id: 1,
                slot_id: 3,
                display_id: 2,
                display_text: Some("Heavy hit".to_string()),
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_attribute_keys(95, named_bundle_entry(exact_attribute_modifiers.clone())),
        uv("component_condition_bundle_exact_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            95,
            named_bundle_entry(DataComponentPatchSummary {
                attribute_modifiers: vec![
                    AttributeModifierSummary {
                        display_id: 0,
                        display_text: None,
                        ..exact_attribute_modifiers.attribute_modifiers[0].clone()
                    },
                    exact_attribute_modifiers.attribute_modifiers[1].clone(),
                ],
                ..exact_attribute_modifiers.clone()
            })
        ),
        uv("component_condition_bundle_exact_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(
            95,
            named_bundle_entry(DataComponentPatchSummary {
                attribute_modifiers: vec![
                    AttributeModifierSummary {
                        amount_bits: 3.0f64.to_bits(),
                        display_text: None,
                        ..exact_attribute_modifiers.attribute_modifiers[0].clone()
                    },
                    exact_attribute_modifiers.attribute_modifiers[1].clone(),
                ],
                ..exact_attribute_modifiers.clone()
            })
        ),
        uv("component_condition_bundle_exact_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(
            95,
            named_bundle_entry(DataComponentPatchSummary {
                attribute_modifiers: vec![
                    exact_attribute_modifiers.attribute_modifiers[0].clone(),
                    AttributeModifierSummary {
                        display_text: Some("Light hit".to_string()),
                        ..exact_attribute_modifiers.attribute_modifiers[1].clone()
                    },
                ],
                ..exact_attribute_modifiers.clone()
            })
        ),
        uv("component_condition_bundle_exact_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(
            95,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..exact_attribute_modifiers
            })
        ),
        uv("component_condition_bundle_exact_attribute_modifiers_absent")
    );
    let exact_written_book = DataComponentPatchSummary {
        added_type_ids: vec![55],
        written_book: Some(WrittenBookContentSummary {
            title: "Quest".to_string(),
            title_filter: Some("Filtered Quest".to_string()),
            author: "Alex".to_string(),
            generation: 1,
            pages: vec!["First page".to_string(), "Second page".to_string()],
            page_filters: vec![None, Some("Filtered second page".to_string())],
            resolved: true,
        }),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(96, named_bundle_entry(exact_written_book.clone())),
        uv("component_condition_bundle_exact_written_book_present")
    );
    assert_eq!(
        selected(
            96,
            named_bundle_entry(DataComponentPatchSummary {
                written_book: Some(WrittenBookContentSummary {
                    title_filter: None,
                    ..exact_written_book.written_book.clone().unwrap()
                }),
                ..exact_written_book.clone()
            })
        ),
        uv("component_condition_bundle_exact_written_book_absent")
    );
    assert_eq!(
        selected(
            96,
            named_bundle_entry(DataComponentPatchSummary {
                written_book: Some(WrittenBookContentSummary {
                    page_filters: vec![None, None],
                    ..exact_written_book.written_book.clone().unwrap()
                }),
                ..exact_written_book.clone()
            })
        ),
        uv("component_condition_bundle_exact_written_book_absent")
    );
    assert_eq!(
        selected(
            96,
            named_bundle_entry(DataComponentPatchSummary {
                removed_type_ids: vec![55],
                ..exact_written_book
            })
        ),
        uv("component_condition_bundle_exact_written_book_absent")
    );
    assert_eq!(
        selected(
            33,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_components_present")
    );
    assert_eq!(
        selected(
            33,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            rarity: Some(ItemRaritySummary::Rare),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_components_absent")
    );
    assert_eq!(
        selected(
            36,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 2,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![3],
                        damage: Some(3),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_damage_present")
    );
    assert_eq!(
        selected(
            36,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 2,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![3],
                        damage: Some(4),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_damage_absent")
    );
    assert_eq!(
        selected(
            36,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                removed_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 2,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![3],
                        damage: Some(3),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_damage_absent")
    );
    assert_eq!(
        selected(
            37,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![3],
                            damage: Some(3),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![3],
                            damage: Some(3),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_damage_present")
    );
    assert_eq!(
        selected(
            37,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![3],
                            damage: Some(3),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 2,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![3],
                            damage: Some(4),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_damage_absent")
    );
    assert_eq!(
        selected(
            38,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_any_value_present")
    );
    assert_eq!(
        selected(
            38,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        removed_type_ids: vec![12],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_any_value_absent")
    );
    assert_eq!(
        selected(
            39,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![21],
                            enchantment_glint_override: Some(false),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![21],
                            enchantment_glint_override: Some(true),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_any_value_present")
    );
    assert_eq!(
        selected(
            39,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![21],
                            enchantment_glint_override: Some(false),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_any_value_absent")
    );
    assert_eq!(
        selected(
            40,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![13],
                        enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 3,
                        }],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_enchantments_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            40,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![13],
                        enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 0,
                            level: 3,
                        }],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_enchantments_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            40,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![13],
                        enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                            holder_id: 1,
                            level: 3,
                        }],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_enchantments_absent")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            41,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![42],
                            stored_enchantments: vec![
                                bbb_protocol::packets::ItemEnchantmentSummary {
                                    holder_id: 1,
                                    level: 1,
                                },
                            ],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![42],
                            stored_enchantments: vec![
                                bbb_protocol::packets::ItemEnchantmentSummary {
                                    holder_id: 1,
                                    level: 1,
                                },
                            ],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_stored_enchantments_present")
    );
    assert_eq!(
        selected_with_enchantment_keys(
            41,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![42],
                            stored_enchantments: vec![
                                bbb_protocol::packets::ItemEnchantmentSummary {
                                    holder_id: 1,
                                    level: 1,
                                },
                            ],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![42],
                            stored_enchantments: vec![
                                bbb_protocol::packets::ItemEnchantmentSummary {
                                    holder_id: 0,
                                    level: 1,
                                },
                            ],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_stored_enchantments_absent")
    );
    assert_eq!(
        selected(
            42,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: star_trail.clone(),
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_firework_explosion_present")
    );
    assert_eq!(
        selected(
            42,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        firework_explosion_shape: Some(FireworkExplosionShapeSummary::Burst),
                        ..star_trail.clone()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_firework_explosion_absent")
    );
    assert_eq!(
        selected(
            43,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![69],
                            fireworks_flight_duration: Some(3),
                            fireworks_explosions_count: Some(1),
                            fireworks_explosions: vec![fireworks_explosion(
                                FireworkExplosionShapeSummary::Burst,
                                false,
                                true,
                            )],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![69],
                            fireworks_flight_duration: Some(2),
                            fireworks_explosions_count: Some(1),
                            fireworks_explosions: vec![fireworks_explosion(
                                FireworkExplosionShapeSummary::Burst,
                                true,
                                true,
                            )],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_fireworks_present")
    );
    assert_eq!(
        selected(
            43,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![69],
                            fireworks_flight_duration: Some(3),
                            fireworks_explosions_count: Some(1),
                            fireworks_explosions: vec![fireworks_explosion(
                                FireworkExplosionShapeSummary::Burst,
                                false,
                                true,
                            )],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![69],
                            fireworks_flight_duration: Some(1),
                            fireworks_explosions_count: Some(1),
                            fireworks_explosions: vec![fireworks_explosion(
                                FireworkExplosionShapeSummary::Burst,
                                true,
                                true,
                            )],
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_fireworks_absent")
    );
    assert_eq!(
        selected(
            44,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![56],
                        armor_trim_material_id: Some(1),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_trim_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            44,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![56],
                        armor_trim_material_id: Some(1),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_trim_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            44,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![56],
                        armor_trim_material_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_trim_absent")
    );
    assert_eq!(
        selected_with_trim_keys(
            45,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![56],
                            armor_trim_material_id: Some(1),
                            armor_trim_pattern_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![56],
                            armor_trim_material_id: Some(0),
                            armor_trim_pattern_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_trim_present")
    );
    assert_eq!(
        selected_with_trim_keys(
            45,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![56],
                            armor_trim_material_id: Some(1),
                            armor_trim_pattern_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![56],
                            armor_trim_material_id: Some(1),
                            armor_trim_pattern_id: Some(1),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_trim_absent")
    );
    assert_eq!(
        selected(46, DataComponentPatchSummary::default()),
        uv("component_condition_jukebox_playable_song_absent")
    );
    assert_eq!(
        selected(
            46,
            DataComponentPatchSummary {
                added_type_ids: vec![64],
                jukebox_song_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_jukebox_playable_song_present")
    );
    assert_eq!(
        selected(
            46,
            DataComponentPatchSummary {
                added_type_ids: vec![64],
                jukebox_song_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_jukebox_playable_song_absent")
    );
    assert_eq!(
        selected(
            46,
            DataComponentPatchSummary {
                added_type_ids: vec![64],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_jukebox_playable_song_absent")
    );
    assert_eq!(
        selected(
            47,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![64],
                        jukebox_song_id: Some(1),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_jukebox_playable_present")
    );
    assert_eq!(
        selected(
            47,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![64],
                        jukebox_song_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_jukebox_playable_absent")
    );
    assert_eq!(
        selected(
            48,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![64],
                            jukebox_song_id: Some(1),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![64],
                            jukebox_song_id: Some(1),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_jukebox_playable_present")
    );
    assert_eq!(
        selected(
            48,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![64],
                            jukebox_song_id: Some(1),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![64],
                            jukebox_song_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_jukebox_playable_absent")
    );
    assert_eq!(
        selected(49, DataComponentPatchSummary::default()),
        uv("component_condition_potion_contents_absent")
    );
    assert_eq!(
        selected(
            49,
            DataComponentPatchSummary {
                added_type_ids: vec![51],
                potion_id: Some(healing_potion_id),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_potion_contents_present")
    );
    assert_eq!(
        selected(
            49,
            DataComponentPatchSummary {
                added_type_ids: vec![51],
                potion_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_potion_contents_absent")
    );
    assert_eq!(
        selected(
            49,
            DataComponentPatchSummary {
                added_type_ids: vec![51],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_potion_contents_absent")
    );
    assert_eq!(
        selected(
            50,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![51],
                        potion_id: Some(healing_potion_id),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_potion_contents_present")
    );
    assert_eq!(
        selected(
            50,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![51],
                        potion_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_potion_contents_absent")
    );
    assert_eq!(
        selected(
            51,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![51],
                            potion_id: Some(healing_potion_id),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![51],
                            potion_id: Some(healing_potion_id),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_potion_contents_present")
    );
    assert_eq!(
        selected(
            51,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![51],
                            potion_id: Some(healing_potion_id),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![51],
                            potion_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_potion_contents_absent")
    );
    assert_eq!(
        selected(52, DataComponentPatchSummary::default()),
        uv("component_condition_writable_book_pages_absent")
    );
    assert_eq!(
        selected(
            52,
            DataComponentPatchSummary {
                added_type_ids: vec![54],
                writable_book_pages: vec![
                    "alpha".to_string(),
                    "beta".to_string(),
                    "alpha".to_string(),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_writable_book_pages_present")
    );
    assert_eq!(
        selected(
            52,
            DataComponentPatchSummary {
                added_type_ids: vec![54],
                writable_book_pages: vec!["alpha".to_string(), "beta".to_string()],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_writable_book_pages_absent")
    );
    assert_eq!(
        selected(
            52,
            DataComponentPatchSummary {
                added_type_ids: vec![54],
                removed_type_ids: vec![54],
                writable_book_pages: vec![
                    "alpha".to_string(),
                    "beta".to_string(),
                    "alpha".to_string(),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_writable_book_pages_absent")
    );

    let matching_written_book = WrittenBookContentSummary {
        title: "Quest".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 2,
        pages: vec!["First page".to_string(), "Second page".to_string()],
        page_filters: vec![None, None],
        resolved: true,
    };
    assert_eq!(
        selected(53, DataComponentPatchSummary::default()),
        uv("component_condition_written_book_content_absent")
    );
    assert_eq!(
        selected(
            53,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                written_book: Some(matching_written_book.clone()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_content_present")
    );
    assert_eq!(
        selected(
            53,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                written_book: Some(WrittenBookContentSummary {
                    author: "Steve".to_string(),
                    ..matching_written_book.clone()
                }),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_content_absent")
    );
    assert_eq!(
        selected(
            53,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                written_book: Some(WrittenBookContentSummary {
                    resolved: false,
                    ..matching_written_book.clone()
                }),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_content_absent")
    );
    assert_eq!(
        selected(
            53,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                removed_type_ids: vec![55],
                written_book: Some(matching_written_book.clone()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_content_absent")
    );
    assert_eq!(
        selected(
            80,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                written_book: Some(matching_written_book.clone()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_component_page_present")
    );
    assert_eq!(
        selected(
            80,
            DataComponentPatchSummary {
                added_type_ids: vec![55],
                written_book: Some(WrittenBookContentSummary {
                    pages: vec!["Other page".to_string(), "Second page".to_string()],
                    ..matching_written_book.clone()
                }),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_written_book_component_page_absent")
    );
    assert_eq!(
        selected(
            54,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![54],
                        writable_book_pages: vec![
                            "alpha".to_string(),
                            "beta".to_string(),
                            "alpha".to_string(),
                        ],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_writable_book_present")
    );
    assert_eq!(
        selected(
            54,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![54],
                        writable_book_pages: vec!["alpha".to_string(), "beta".to_string()],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_writable_book_absent")
    );
    assert_eq!(
        selected(
            55,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![55],
                            written_book: Some(matching_written_book.clone()),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![55],
                            written_book: Some(matching_written_book.clone()),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_written_book_present")
    );
    assert_eq!(
        selected(
            55,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![55],
                            written_book: Some(matching_written_book),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![55],
                            written_book: Some(WrittenBookContentSummary {
                                title: "Other".to_string(),
                                title_filter: None,
                                author: "Alex".to_string(),
                                generation: 2,
                                pages: vec!["First page".to_string(), "Second page".to_string(),],
                                page_filters: vec![None, None],
                                resolved: true,
                            }),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_written_book_absent")
    );
    assert_eq!(
        selected(56, DataComponentPatchSummary::default()),
        uv("component_condition_villager_variant_absent")
    );
    assert_eq!(
        selected(
            56,
            DataComponentPatchSummary {
                added_type_ids: vec![83],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_villager_variant_present")
    );
    assert_eq!(
        selected(
            56,
            DataComponentPatchSummary {
                added_type_ids: vec![83],
                villager_variant_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_villager_variant_absent")
    );
    assert_eq!(
        selected(
            56,
            DataComponentPatchSummary {
                added_type_ids: vec![83],
                removed_type_ids: vec![83],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_villager_variant_absent")
    );
    assert_eq!(
        selected(
            57,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(2),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_villager_variant_present")
    );
    assert_eq!(
        selected(
            57,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_villager_variant_absent")
    );
    assert_eq!(
        selected(
            58,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![83],
                            villager_variant_id: Some(2),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![83],
                            villager_variant_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_villager_variant_present")
    );
    assert_eq!(
        selected(
            58,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(2),
                container_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 0,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![83],
                            villager_variant_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added_type_ids: vec![83],
                            villager_variant_id: Some(0),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_villager_variant_absent")
    );
    assert_eq!(
        selected(
            70,
            DataComponentPatchSummary {
                added_type_ids: vec![83],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_villager_variant_tag_present")
    );
    assert_eq!(
        selected(
            70,
            DataComponentPatchSummary {
                added_type_ids: vec![83],
                villager_variant_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_villager_variant_tag_absent")
    );
    assert_eq!(
        selected(
            71,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(2),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_villager_variant_tag_present")
    );
    assert_eq!(
        selected(
            71,
            DataComponentPatchSummary {
                added_type_ids: vec![50],
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_bundle_partial_villager_variant_tag_absent")
    );
    assert_eq!(
        selected(
            72,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(2),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_villager_variant_tag_present")
    );
    assert_eq!(
        selected(
            72,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![83],
                        villager_variant_id: Some(0),
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_villager_variant_tag_absent")
    );

    let modifier_with_attribute =
        |attribute_id, id: &str, amount: f64, operation_id, slot_id| AttributeModifierSummary {
            attribute_id,
            modifier_id: id.to_string(),
            amount_bits: amount.to_bits(),
            operation_id,
            slot_id,
            display_id: 0,
            display_text: None,
        };
    let modifier = |id: &str, amount: f64, operation_id, slot_id| AttributeModifierSummary {
        attribute_id: 7,
        modifier_id: id.to_string(),
        amount_bits: amount.to_bits(),
        operation_id,
        slot_id,
        display_id: 0,
        display_text: None,
    };
    assert_eq!(
        selected(59, DataComponentPatchSummary::default()),
        uv("component_condition_attribute_modifiers_absent")
    );
    assert_eq!(
        selected(
            59,
            DataComponentPatchSummary {
                added_type_ids: vec![16],
                attribute_modifiers: vec![
                    modifier("minecraft:test/speed", 1.5, 0, 1),
                    modifier("minecraft:test/scale", 3.0, 1, 3),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_attribute_modifiers_present")
    );
    assert_eq!(
        selected(
            59,
            DataComponentPatchSummary {
                added_type_ids: vec![16],
                attribute_modifiers: vec![
                    modifier("minecraft:test/speed", 0.5, 0, 1),
                    modifier("minecraft:test/scale", 3.0, 1, 3),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_attribute_modifiers_absent")
    );
    assert_eq!(
        selected(
            59,
            DataComponentPatchSummary {
                added_type_ids: vec![16],
                removed_type_ids: vec![16],
                attribute_modifiers: vec![
                    modifier("minecraft:test/speed", 1.5, 0, 1),
                    modifier("minecraft:test/scale", 3.0, 1, 3),
                ],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_attribute_modifiers_absent")
    );
    assert_eq!(
        selected(
            60,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![16],
                        attribute_modifiers: vec![
                            modifier("minecraft:test/speed", 1.5, 0, 1),
                            modifier("minecraft:test/scale", 3.0, 1, 3),
                        ],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_attribute_modifiers_present")
    );
    assert_eq!(
        selected(
            60,
            DataComponentPatchSummary {
                added_type_ids: vec![75],
                container_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 0,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![16],
                        attribute_modifiers: vec![
                            modifier("minecraft:test/speed", 1.5, 0, 1),
                            modifier("minecraft:test/heavy", 2.0, 0, 2),
                        ],
                        ..DataComponentPatchSummary::default()
                    },
                }],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_container_partial_attribute_modifiers_absent")
    );
    let attack_damage_modifier = modifier_with_attribute(0, "minecraft:test/speed", 1.5, 0, 1);
    let scale_modifier = modifier_with_attribute(1, "minecraft:test/speed", 1.5, 0, 1);
    let attribute_patch = |modifier| DataComponentPatchSummary {
        added_type_ids: vec![16],
        attribute_modifiers: vec![modifier],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(61, attribute_patch(attack_damage_modifier.clone())),
        uv("component_condition_attribute_modifiers_attribute_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(61, attribute_patch(attack_damage_modifier.clone())),
        uv("component_condition_attribute_modifiers_attribute_present")
    );
    assert_eq!(
        selected_with_attribute_keys(61, attribute_patch(scale_modifier.clone())),
        uv("component_condition_attribute_modifiers_attribute_absent")
    );
    let container_attribute_patch = |modifier| DataComponentPatchSummary {
        added_type_ids: vec![75],
        container_item_count: Some(1),
        container_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch: attribute_patch(modifier),
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_attribute_keys(
            62,
            container_attribute_patch(attack_damage_modifier.clone())
        ),
        uv("component_condition_container_partial_attribute_modifiers_attribute_present")
    );
    assert_eq!(
        selected_with_attribute_keys(62, container_attribute_patch(scale_modifier.clone())),
        uv("component_condition_container_partial_attribute_modifiers_attribute_absent")
    );
    let bundle_attribute_modifiers_patch = |modifiers| DataComponentPatchSummary {
        added_type_ids: vec![50],
        bundle_contents_item_count: Some(1),
        bundle_contents_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![16],
                attribute_modifiers: modifiers,
                ..DataComponentPatchSummary::default()
            },
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(
            66,
            bundle_attribute_modifiers_patch(vec![
                modifier("minecraft:test/speed", 1.5, 0, 1),
                modifier("minecraft:test/scale", 3.0, 1, 3),
            ])
        ),
        uv("component_condition_bundle_partial_attribute_modifiers_present")
    );
    assert_eq!(
        selected(
            66,
            bundle_attribute_modifiers_patch(vec![
                modifier("minecraft:test/speed", 1.5, 0, 1),
                modifier("minecraft:test/heavy", 2.0, 0, 2),
            ])
        ),
        uv("component_condition_bundle_partial_attribute_modifiers_absent")
    );
    assert_eq!(
        selected(
            66,
            DataComponentPatchSummary {
                removed_type_ids: vec![50],
                ..bundle_attribute_modifiers_patch(vec![
                    modifier("minecraft:test/speed", 1.5, 0, 1),
                    modifier("minecraft:test/scale", 3.0, 1, 3),
                ])
            }
        ),
        uv("component_condition_bundle_partial_attribute_modifiers_absent")
    );
    let bundle_attribute_patch = |modifier| DataComponentPatchSummary {
        added_type_ids: vec![50],
        bundle_contents_item_count: Some(1),
        bundle_contents_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch: attribute_patch(modifier),
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_attribute_keys(67, bundle_attribute_patch(attack_damage_modifier.clone())),
        uv("component_condition_bundle_partial_attribute_modifiers_attribute_present")
    );
    assert_eq!(
        selected_with_attribute_keys(67, bundle_attribute_patch(scale_modifier.clone())),
        uv("component_condition_bundle_partial_attribute_modifiers_attribute_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(68, attribute_patch(attack_damage_modifier.clone())),
        uv("component_condition_attribute_modifiers_attribute_tag_present")
    );
    assert_eq!(
        selected_with_attribute_keys(68, attribute_patch(scale_modifier.clone())),
        uv("component_condition_attribute_modifiers_attribute_tag_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(69, bundle_attribute_patch(attack_damage_modifier)),
        uv("component_condition_bundle_partial_attribute_modifiers_attribute_tag_present")
    );
    assert_eq!(
        selected_with_attribute_keys(69, bundle_attribute_patch(scale_modifier)),
        uv("component_condition_bundle_partial_attribute_modifiers_attribute_tag_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(73, DataComponentPatchSummary::default()),
        uv("component_condition_default_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            73,
            DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_default_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(
            73,
            DataComponentPatchSummary {
                added_type_ids: vec![16],
                attribute_modifiers: Vec::new(),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_default_attribute_modifiers_absent")
    );
    let default_attribute_item = |component_patch| ItemStackTemplateSummary {
        item_id: 73,
        count: 1,
        component_patch,
    };
    let bundle_default_attribute_patch = |component_patch| DataComponentPatchSummary {
        added_type_ids: vec![50],
        bundle_contents_item_count: Some(1),
        bundle_contents_items: vec![default_attribute_item(component_patch)],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_attribute_keys(
            74,
            bundle_default_attribute_patch(DataComponentPatchSummary::default())
        ),
        uv("component_condition_bundle_partial_default_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            74,
            bundle_default_attribute_patch(DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_bundle_partial_default_attribute_modifiers_absent")
    );
    let container_default_attribute_patch = |component_patch| DataComponentPatchSummary {
        added_type_ids: vec![75],
        container_item_count: Some(1),
        container_items: vec![default_attribute_item(component_patch)],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected_with_attribute_keys(
            75,
            container_default_attribute_patch(DataComponentPatchSummary::default())
        ),
        uv("component_condition_container_partial_default_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            75,
            container_default_attribute_patch(DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..DataComponentPatchSummary::default()
            })
        ),
        uv("component_condition_container_partial_default_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(76, DataComponentPatchSummary::default()),
        uv("component_condition_default_armor_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            76,
            DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_default_armor_attribute_modifiers_absent")
    );
    assert_eq!(
        selected_with_attribute_keys(77, DataComponentPatchSummary::default()),
        uv("component_condition_default_mace_attribute_modifiers_present")
    );
    assert_eq!(
        selected_with_attribute_keys(
            77,
            DataComponentPatchSummary {
                removed_type_ids: vec![16],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_default_mace_attribute_modifiers_absent")
    );
    let custom_data_value = |owner: &str| {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "owner".to_string(),
                value: NbtSummaryValue::String(owner.to_string()),
            },
            NbtSummaryEntry {
                name: "level".to_string(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "nested".to_string(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "flag".to_string(),
                    value: NbtSummaryValue::Byte(1),
                }]),
            },
            NbtSummaryEntry {
                name: "lore".to_string(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("one".to_string()),
                    NbtSummaryValue::String("two".to_string()),
                ]),
            },
        ])
    };
    let custom_data_patch = |owner| DataComponentPatchSummary {
        added_type_ids: vec![0],
        custom_data: Some(custom_data_value(owner)),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(63, DataComponentPatchSummary::default()),
        uv("component_condition_custom_data_absent")
    );
    assert_eq!(
        selected(63, custom_data_patch("Alex")),
        uv("component_condition_custom_data_present")
    );
    assert_eq!(
        selected(63, custom_data_patch("Steve")),
        uv("component_condition_custom_data_absent")
    );
    assert_eq!(
        selected(
            63,
            DataComponentPatchSummary {
                removed_type_ids: vec![0],
                custom_data: Some(custom_data_value("Alex")),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_condition_custom_data_absent")
    );
    let bundle_custom_data_patch = |owner| DataComponentPatchSummary {
        added_type_ids: vec![50],
        bundle_contents_item_count: Some(1),
        bundle_contents_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch: custom_data_patch(owner),
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(64, bundle_custom_data_patch("Alex")),
        uv("component_condition_bundle_partial_custom_data_present")
    );
    assert_eq!(
        selected(64, bundle_custom_data_patch("Steve")),
        uv("component_condition_bundle_partial_custom_data_absent")
    );
    assert_eq!(
        selected(
            64,
            DataComponentPatchSummary {
                removed_type_ids: vec![50],
                ..bundle_custom_data_patch("Alex")
            }
        ),
        uv("component_condition_bundle_partial_custom_data_absent")
    );
    let container_custom_data_patch = |owner| DataComponentPatchSummary {
        added_type_ids: vec![75],
        container_item_count: Some(1),
        container_items: vec![ItemStackTemplateSummary {
            item_id: 0,
            count: 1,
            component_patch: custom_data_patch(owner),
        }],
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        selected(65, container_custom_data_patch("Alex")),
        uv("component_condition_container_partial_custom_data_present")
    );
    assert_eq!(
        selected(65, container_custom_data_patch("Steve")),
        uv("component_condition_container_partial_custom_data_absent")
    );
    assert_eq!(
        selected(78, custom_data_patch("Alex")),
        uv("component_condition_custom_data_snbt_present")
    );
    assert_eq!(
        selected(78, custom_data_patch("Steve")),
        uv("component_condition_custom_data_snbt_absent")
    );
    assert_eq!(
        selected(79, bundle_custom_data_patch("Alex")),
        uv("component_condition_bundle_partial_custom_data_snbt_present")
    );
    assert_eq!(
        selected(79, bundle_custom_data_patch("Steve")),
        uv("component_condition_bundle_partial_custom_data_snbt_absent")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_uses_item_model_component_as_root_model() {
    let root = unique_temp_dir("item-runtime-item-model-component");
    write_item_model_component_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |component_patch| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch,
    };

    // Vanilla `Item.Properties.finalizeInitializer` defaults ITEM_MODEL to
    // the item's own id, so an unpatched stack uses the item definition.
    assert_eq!(
        runtime
            .icon_for_stack(&stack(DataComponentPatchSummary::default()))
            .unwrap()
            .layers[0]
            .uv,
        uv("model_component")
    );

    // `ItemModelResolver.appendItemLayers` reads the effective
    // DataComponents.ITEM_MODEL value for the root item model.
    assert_eq!(
        runtime
            .icon_for_stack(&stack(DataComponentPatchSummary {
                added_type_ids: vec![10],
                item_model: Some("minecraft:alternate_model_component".to_string()),
                ..DataComponentPatchSummary::default()
            }))
            .unwrap()
            .layers[0]
            .uv,
        uv("alternate_model_component")
    );
    assert_eq!(
        runtime.item_particle_sprite_ids_for_stack(&stack(DataComponentPatchSummary {
            added_type_ids: vec![10],
            item_model: Some("minecraft:alternate_model_component".to_string()),
            ..DataComponentPatchSummary::default()
        })),
        Some(vec!["minecraft:item/alternate_model_component".to_string()])
    );

    // Removing ITEM_MODEL makes ItemStack.get(ITEM_MODEL) return null; the
    // vanilla resolver clears the output and appends no item layers.
    assert!(runtime
        .icon_for_stack(&stack(DataComponentPatchSummary {
            removed_type_ids: vec![10],
            ..DataComponentPatchSummary::default()
        }))
        .is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_display_context_select() {
    let root = unique_temp_dir("item-runtime-display-context");
    write_display_context_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let rect = |model_id: &str| {
        let uv = uv(model_id);
        ItemSpriteRect {
            min: uv.min,
            max: uv.max,
        }
    };
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };

    // GUI/HUD item icons pass vanilla ItemDisplayContext.GUI.
    assert_eq!(
        runtime.icon_for_stack(&stack).unwrap().layers[0].uv,
        uv("display_gui")
    );

    // Non-living generated consumers pass their actual world contexts.
    assert_eq!(
        runtime
            .generated_item_layers_for_stack_with_trim_materials(
                &stack,
                BlockModelDisplayContext::Ground,
                None,
            )
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_ground")
    );
    assert_eq!(
        runtime
            .generated_item_layers_for_stack_with_trim_materials(
                &stack,
                BlockModelDisplayContext::Fixed,
                None,
            )
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_fixed")
    );

    // Entity-owned generated held items pass their hand display context.
    assert_eq!(
        runtime
            .generated_item_layers_for_stack_with_owner_context(
                &stack,
                BlockModelDisplayContext::ThirdPersonRightHand,
                None,
                None,
                None,
                None,
                false,
                ItemModelUseContext::inactive(),
            )
            .into_iter()
            .next()
            .unwrap()
            .rect,
        rect("display_thirdperson_right")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_trim_material_select() {
    let root = unique_temp_dir("item-runtime-trim-material");
    write_trim_material_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    // Trim-material registry keys by holder id (registration order).
    let trim_keys = [
        "minecraft:quartz".to_string(),
        "minecraft:iron".to_string(),
        "minecraft:diamond".to_string(),
    ];
    let trimmed = |material_id: Option<i32>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            armor_trim_material_id: material_id,
            ..DataComponentPatchSummary::default()
        },
    };
    let selected = |stack: &ItemStackSummary| {
        runtime
            .icon_for_stack_with_context(
                stack,
                None,
                false,
                0.0,
                Some(&trim_keys),
                None,
                None,
                None,
            )
            .unwrap()
            .layers[0]
            .uv
    };

    // No trim component → no match → fallback (plain chestplate).
    assert_eq!(selected(&trimmed(None)), uv("iron_chestplate"));
    // Holder id 0 → "minecraft:quartz" → quartz trim model.
    assert_eq!(
        selected(&trimmed(Some(0))),
        uv("iron_chestplate_quartz_trim")
    );
    // Holder id 2 → "minecraft:diamond" → diamond trim model.
    assert_eq!(
        selected(&trimmed(Some(2))),
        uv("iron_chestplate_diamond_trim")
    );
    // Holder id 1 → "minecraft:iron" (no case) → fallback.
    assert_eq!(selected(&trimmed(Some(1))), uv("iron_chestplate"));
    // Without the registry keys (no world context) → fallback.
    assert_eq!(
        runtime.icon_for_stack(&trimmed(Some(0))).unwrap().layers[0].uv,
        uv("iron_chestplate")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_charge_type_select() {
    let root = unique_temp_dir("item-runtime-charge-type");
    write_charge_type_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    // A charged-projectiles template list with one entry of the given item id.
    let charged = |item_id: i32| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            charged_projectiles_items: vec![ItemStackTemplateSummary {
                item_id,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            ..DataComponentPatchSummary::default()
        },
    };
    let selected = |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

    // Empty crossbow → NONE → no matching case → fallback (plain crossbow).
    assert_eq!(
        selected(&ItemStackSummary {
            item_id: Some(0),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }),
        uv("crossbow")
    );
    // Charged with an arrow (item 2) → ARROW → "arrow" case.
    assert_eq!(selected(&charged(2)), uv("crossbow_arrow"));
    // Charged with a firework_rocket (item 1) → ROCKET → "rocket" case.
    assert_eq!(selected(&charged(1)), uv("crossbow_firework"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_use_tick_range_dispatch_properties() {
    let root = unique_temp_dir("item-runtime-use-tick-range-dispatch");
    write_use_tick_range_dispatch_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |item_id: i32| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let selected = |stack: &ItemStackSummary, using_item: bool, elapsed_ticks: u32| {
        let use_context = if using_item {
            runtime.item_model_use_context_for_stack(stack, elapsed_ticks)
        } else {
            ItemModelUseContext::inactive()
        };
        runtime
            .icon_for_stack_with_context_and_use_context(
                stack,
                None,
                using_item,
                use_context,
                BlockModelDisplayContext::Gui,
                0.0,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .layers[0]
            .uv
    };
    let selected_with_enchantment_keys =
        |stack: &ItemStackSummary,
         using_item: bool,
         elapsed_ticks: u32,
         enchantment_keys: &[String]| {
            let use_context = if using_item {
                runtime.item_model_use_context_for_stack_with_enchantment_keys(
                    stack,
                    elapsed_ticks,
                    Some(enchantment_keys),
                )
            } else {
                ItemModelUseContext::inactive()
            };
            runtime
                .icon_for_stack_with_context_and_use_context(
                    stack,
                    None,
                    using_item,
                    use_context,
                    BlockModelDisplayContext::Gui,
                    0.0,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap()
                .layers[0]
                .uv
        };

    let bow = stack(0);
    assert_eq!(selected(&bow, false, 20), uv("bow"));
    assert_eq!(selected(&bow, true, 0), uv("bow_pulling_0"));
    assert_eq!(selected(&bow, true, 13), uv("bow_pulling_1"));
    assert_eq!(selected(&bow, true, 18), uv("bow_pulling_2"));

    let crossbow = stack(1);
    assert_eq!(selected(&crossbow, false, 25), uv("crossbow"));
    assert_eq!(selected(&crossbow, true, 0), uv("crossbow_pulling_0"));
    assert_eq!(selected(&crossbow, true, 15), uv("crossbow_pulling_1"));
    assert_eq!(selected(&crossbow, true, 25), uv("crossbow_pulling_2"));
    let quick_charge_keys = vec![
        "minecraft:power".to_string(),
        "minecraft:quick_charge".to_string(),
    ];
    let quick_charge_crossbow = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 2,
            }],
            ..DataComponentPatchSummary::default()
        },
    };
    // Vanilla `CrossbowItem.getChargeDuration` starts at 1.25 seconds and
    // `QUICK_CHARGE` adds `-0.25F` per level before `floor(seconds * 20)`.
    // Level 2 therefore charges in 15 ticks, so elapsed 10 ticks crosses
    // the 0.58 `crossbow/pull` threshold; without the registry-backed
    // enchantment context, the same stack still uses the 25 tick default.
    assert_eq!(
        selected(&quick_charge_crossbow, true, 10),
        uv("crossbow_pulling_0")
    );
    assert_eq!(
        selected_with_enchantment_keys(&quick_charge_crossbow, true, 10, &quick_charge_keys),
        uv("crossbow_pulling_1")
    );
    let charged_crossbow = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            charged_projectiles_items: vec![ItemStackTemplateSummary {
                item_id: 3,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            ..DataComponentPatchSummary::default()
        },
    };
    assert_eq!(selected(&charged_crossbow, true, 25), uv("crossbow_arrow"));

    let brush = stack(4);
    assert_eq!(selected(&brush, false, 1), uv("brush"));
    assert_eq!(selected(&brush, true, 0), uv("brush"));
    assert_eq!(selected(&brush, true, 7), uv("brush_brushing_0"));
    assert_eq!(selected(&brush, true, 1), uv("brush_brushing_2"));

    let consumable_apple = ItemStackSummary {
        item_id: Some(5),
        count: 1,
        component_patch: DataComponentPatchSummary {
            consumable: Some(ConsumableSummary {
                consume_seconds: 0.8,
                animation: ItemUseAnimationSummary::Eat,
            }),
            ..DataComponentPatchSummary::default()
        },
    };
    assert_eq!(selected(&consumable_apple, false, 0), uv("apple"));
    // Vanilla `Consumable.consumeTicks()` casts `consumeSeconds * 20.0F` to
    // int, so 0.8 seconds yields a 16 tick remaining-time source.
    assert_eq!(
        selected(&consumable_apple, true, 0),
        uv("apple_remaining_high")
    );
    assert_eq!(
        selected(&consumable_apple, true, 10),
        uv("apple_remaining_low")
    );

    let ender_eye = stack(6);
    assert_eq!(selected(&ender_eye, false, 0), uv("ender_eye"));
    // Vanilla 26.1 `EnderEyeItem.getUseDuration` returns 0, so even an
    // active use context has no remaining ticks for range-dispatch.
    assert_eq!(
        selected(&ender_eye, true, 0),
        uv("ender_eye_remaining_empty")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_main_hand_select_from_owner_context() {
    let root = unique_temp_dir("item-runtime-main-hand-select");
    write_main_hand_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let selected = |owner_main_hand_left| {
        runtime
            .icon_for_stack_with_owner_main_hand(&stack, owner_main_hand_left)
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla `MainHand.get` returns null when there is no living owner, so
    // no case matches and the fallback model is used.
    assert_eq!(selected(None), uv("hand_selector"));
    assert_eq!(selected(Some(false)), uv("hand_selector_right"));
    assert_eq!(selected(Some(true)), uv("hand_selector_left"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_stack_string_select_properties() {
    let root = unique_temp_dir("item-runtime-stack-string-select");
    write_stack_string_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

    let block_state_stack = |properties: BTreeMap<String, String>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_state_properties: properties,
            ..DataComponentPatchSummary::default()
        },
    };
    // Vanilla `ItemBlockState.get` returns null without the component/property,
    // so no case matches and the fallback model is used.
    assert_eq!(
        selected(&block_state_stack(BTreeMap::new())),
        uv("beehive_empty")
    );
    assert_eq!(
        selected(&block_state_stack(BTreeMap::from([(
            "honey_level".to_string(),
            "5".to_string()
        )]))),
        uv("beehive_honey")
    );
    assert_eq!(
        selected(&block_state_stack(BTreeMap::from([(
            "wrong_property".to_string(),
            "5".to_string()
        )]))),
        uv("beehive_empty")
    );

    let custom_model_data_stack = |strings: Vec<&str>| ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            custom_model_data_strings: strings.into_iter().map(str::to_string).collect(),
            ..DataComponentPatchSummary::default()
        },
    };
    // Vanilla `CustomModelDataProperty.get` reads strings[index], not floats.
    assert_eq!(
        selected(&custom_model_data_stack(vec!["ignored", "blue"])),
        uv("cmd_blue")
    );
    assert_eq!(
        selected(&custom_model_data_stack(vec!["ignored", "lime"])),
        uv("cmd_green")
    );
    // Out-of-range index and absent component both produce no selected value.
    assert_eq!(
        selected(&custom_model_data_stack(vec!["blue"])),
        uv("cmd_plain")
    );
    assert_eq!(
        selected(&custom_model_data_stack(Vec::new())),
        uv("cmd_plain")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_custom_model_data_condition_flags() {
    let root = unique_temp_dir("item-runtime-custom-model-data-condition");
    write_custom_model_data_condition_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |flags: Vec<bool>, removed: bool| {
        runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    custom_model_data_flags: flags,
                    removed_type_ids: if removed { vec![17] } else { Vec::new() },
                    ..DataComponentPatchSummary::default()
                },
            })
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla conditional `CustomModelDataProperty.get` reads
    // `CustomModelData.flags[index] == true`, not floats or strings.
    assert_eq!(selected(Vec::new(), false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![true], false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![false, true], false), uv("cmd_flag_true"));
    assert_eq!(selected(vec![true, false], false), uv("cmd_flag_false"));
    assert_eq!(selected(vec![false, true], true), uv("cmd_flag_false"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_local_time_select_property() {
    let root = unique_temp_dir("item-runtime-local-time-select");
    write_local_time_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let stack = |item_id| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let selected = |item_id| runtime.icon_for_stack(&stack(item_id)).unwrap().layers[0].uv;

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(0), uv("seasonal_chest_christmas"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 27, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(0), uv("seasonal_chest_normal"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 24, 21, 38, 7)
            .single()
            .unwrap()
            .timestamp_millis()
            + 123,
    );
    assert_eq!(selected(1), uv("precise_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 24, 21, 38, 8)
            .single()
            .unwrap()
            .timestamp_millis()
            + 123,
    );
    assert_eq!(selected(1), uv("precise_clock_fallback"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 24, 15, 8, 7)
            .single()
            .unwrap()
            .timestamp_millis()
            + 123,
    );
    assert_eq!(selected(2), uv("tokyo_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 24, 15, 8, 8)
            .single()
            .unwrap()
            .timestamp_millis()
            + 123,
    );
    assert_eq!(selected(2), uv("tokyo_clock_fallback"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 0, 8, 7)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(3), uv("utc_offset_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 0, 8, 8)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(3), uv("utc_offset_clock_fallback"));

    // `uuuu-DDD-G` exercises the proleptic-year, day-of-year, and era fields:
    // 2026-12-25 UTC formats to "2026-359-AD" (2026 is not a leap year).
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(4), uv("date_field_clock_match"));

    // A different day-of-year ("2027-001-AD") falls back.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(4), uv("date_field_clock_fallback"));

    // ICU `g`: Julian day number for the local calendar date, padded to the
    // requested minimum width.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 25, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(20), uv("julian_day_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 26, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(20), uv("julian_day_clock_fallback"));

    // ICU quarter fields: `Q/q` widths 1..=5 map to numeric / padded /
    // abbreviated / wide / narrow values. August is Q3.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 8, 9, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(5), uv("quarter_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 10, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(5), uv("quarter_clock_fallback"));

    // ICU localized GMT offset: `O`..`OOO` use the short form, `OOOO`
    // uses the zero-padded long form.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(6), uv("localized_gmt_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(6), uv("localized_gmt_clock_fallback"));

    // ICU `F`: day-of-week-in-month. 2026-08-09 is the second Sunday in August.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 8, 9, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(7), uv("weekday_in_month_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 8, 16, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(7), uv("weekday_in_month_clock_fallback"));

    // ICU root-locale `w`/`W`: weeks start on Monday with minimal-days=1.
    // 2026-01-04 is still week 1; a Sunday-first calendar would format week 2.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 4, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(8), uv("week_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 5, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(8), uv("week_clock_fallback"));

    // ICU root-locale `e`/`c`: local weekday number uses the same Monday-first
    // week data, then widths 3..=6 follow weekday text widths.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 4, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(9), uv("local_weekday_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 5, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(9), uv("local_weekday_clock_fallback"));

    // Narrow zone-name support: ICU `z` widths 1..=3 use the active specific
    // abbreviation, `VV` emits the explicit zone id, and `VVV` emits the
    // exemplar city for IANA zones.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2025, 12, 31, 15, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(10), uv("zone_name_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 15, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(10), uv("zone_name_clock_fallback"));

    // ICU `zzzz` for fixed-offset zones falls back to the long localized-GMT
    // form instead of requiring an IANA display-name table.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(11), uv("fixed_zone_long_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(11), uv("fixed_zone_long_clock_fallback"));

    // ICU offset fields: `ZZZZ` uses long localized-GMT, `ZZZZZ` uses extended
    // ISO8601 with `Z` for UTC, and `XXXX`/`XXXXX` plus `xxxx`/`xxxxx` add the
    // basic / extended width-4/5 forms.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(12), uv("offset_width_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(12), uv("offset_width_clock_fallback"));

    // ICU month text widths: `MMM` is abbreviated, `MMMM` is wide, and
    // `MMMMM` / `LLLLL` are narrow root/en month symbols.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 8, 9, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(13), uv("narrow_month_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 9, 9, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(13), uv("narrow_month_clock_fallback"));

    // ICU AM/PM marker widths: root/en widths 1..=4 use AM/PM and width 5
    // uses the narrow marker.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(14), uv("ampm_width_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(14), uv("ampm_width_clock_fallback"));

    // ICU `A`: milliseconds in day, padded to the requested minimum width.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 1, 2, 3)
            .single()
            .unwrap()
            .timestamp_millis()
            + 4,
    );
    assert_eq!(selected(15), uv("millis_day_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(15), uv("millis_day_clock_fallback"));

    // ICU root/en `w` keeps late December in the current calendar year's final
    // week; it does not switch to next year's week 1 before Jan 1.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 31, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(16), uv("year_end_week_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(16), uv("year_end_week_clock_fallback"));

    // ICU root/en `Y` week-year uses the same Monday/minimal-days=1 week data
    // as `w`; in that supported subset the boundary samples stay in the local
    // calendar year while retaining normal year-width formatting.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 12, 31, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(17), uv("week_year_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(17), uv("week_year_clock_fallback"));

    // ICU en_US week data is Sunday-first with minimal-days=1: 2026-01-04 is
    // the first Sunday and therefore week 2, local weekday 1.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 4, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(18), uv("us_week_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 3, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(18), uv("us_week_clock_fallback"));

    // ICU en_GB week data is Monday-first with minimal-days=4: 2027-01-01 is
    // still week 53 of week-year 2026, and `W` reports the previous month's
    // final week because January's first week starts on 2027-01-04.
    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(19), uv("gb_week_clock_match"));

    runtime.set_local_time_epoch_millis_for_test(
        chrono::Utc
            .with_ymd_and_hms(2027, 1, 4, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis(),
    );
    assert_eq!(selected(19), uv("gb_week_clock_fallback"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_component_select_values() {
    let root = unique_temp_dir("item-runtime-component-select");
    write_component_select_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let selected = |item_id, component_patch| {
        runtime
            .icon_for_stack(&ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch,
            })
            .unwrap()
            .layers[0]
            .uv
    };

    // Vanilla `COMMON_ITEM_COMPONENTS` gives every item `rarity=common`.
    assert_eq!(
        selected(0, DataComponentPatchSummary::default()),
        uv("component_rarity_common")
    );
    assert_eq!(
        selected(
            0,
            DataComponentPatchSummary {
                rarity: Some(ItemRaritySummary::Rare),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_rarity_rare")
    );
    assert_eq!(
        selected(
            0,
            DataComponentPatchSummary {
                rarity: Some(ItemRaritySummary::Rare),
                removed_type_ids: vec![12],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_rarity_fallback")
    );

    // `max_stack_size` is another common default component; numeric cases
    // must match JSON numbers, not strings.
    assert_eq!(
        selected(1, DataComponentPatchSummary::default()),
        uv("component_stack_size_64")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                max_stack_size: Some(16),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_stack_size_16")
    );
    assert_eq!(
        selected(
            1,
            DataComponentPatchSummary {
                max_stack_size: Some(16),
                removed_type_ids: vec![1],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_stack_size_fallback")
    );

    // `enchantment_glint_override` has no common default, so the unset
    // stack falls through while explicit true/false cases both match.
    assert_eq!(
        selected(2, DataComponentPatchSummary::default()),
        uv("component_glint_fallback")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                enchantment_glint_override: Some(true),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_glint_true")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                enchantment_glint_override: Some(false),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_glint_false")
    );
    assert_eq!(
        selected(
            2,
            DataComponentPatchSummary {
                enchantment_glint_override: Some(true),
                removed_type_ids: vec![21],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_glint_fallback")
    );

    // Damageable item defaults project through the item registry: damage=0
    // and max_damage=432 until overridden or removed.
    assert_eq!(
        selected(3, DataComponentPatchSummary::default()),
        uv("component_damage_0")
    );
    assert_eq!(
        selected(
            3,
            DataComponentPatchSummary {
                damage: Some(7),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_damage_7")
    );
    assert_eq!(
        selected(
            3,
            DataComponentPatchSummary {
                damage: Some(7),
                removed_type_ids: vec![3],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_damage_fallback")
    );
    assert_eq!(
        selected(4, DataComponentPatchSummary::default()),
        uv("component_max_damage_432")
    );
    assert_eq!(
        selected(
            4,
            DataComponentPatchSummary {
                max_damage: Some(99),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_max_damage_99")
    );
    assert_eq!(
        selected(
            4,
            DataComponentPatchSummary {
                max_damage: Some(99),
                removed_type_ids: vec![2],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_max_damage_fallback")
    );

    // `Item.Properties.finalizeInitializer` defaults ITEM_MODEL to the
    // item's own id; an explicit item_model patch changes the root model and
    // the component value seen by `ComponentContents.get`.
    assert_eq!(
        selected(5, DataComponentPatchSummary::default()),
        uv("component_item_model_default")
    );
    assert_eq!(
        selected(
            5,
            DataComponentPatchSummary {
                item_model: Some("minecraft:item_model_component_selector_alt_root".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_model_alt")
    );
    assert!(runtime
        .icon_for_stack(&ItemStackSummary {
            item_id: Some(5),
            count: 1,
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![10],
                ..DataComponentPatchSummary::default()
            },
        })
        .is_none());

    // `DataComponents.MAP_ID` wraps an int (`MapId.CODEC`), so component
    // select cases match JSON numbers and removed id 41 suppresses it.
    assert_eq!(
        selected(6, DataComponentPatchSummary::default()),
        uv("component_map_id_fallback")
    );
    assert_eq!(
        selected(
            6,
            DataComponentPatchSummary {
                map_id: Some(123),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_map_id_123")
    );
    assert_eq!(
        selected(
            6,
            DataComponentPatchSummary {
                map_id: Some(123),
                removed_type_ids: vec![41],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_map_id_fallback")
    );

    // `DyedItemColor.CODEC` and `MapItemColor.CODEC` expose their RGB ints
    // to `ComponentContents.get`, with no common default component.
    assert_eq!(
        selected(7, DataComponentPatchSummary::default()),
        uv("component_dyed_color_fallback")
    );
    assert_eq!(
        selected(
            7,
            DataComponentPatchSummary {
                dyed_color: Some(0x12_34_56),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_dyed_color_123456")
    );
    assert_eq!(
        selected(
            7,
            DataComponentPatchSummary {
                dyed_color: Some(0x12_34_56),
                removed_type_ids: vec![44],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_dyed_color_fallback")
    );
    assert_eq!(
        selected(8, DataComponentPatchSummary::default()),
        uv("component_map_color_fallback")
    );
    assert_eq!(
        selected(
            8,
            DataComponentPatchSummary {
                map_color: Some(0x45_67_89),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_map_color_456789")
    );
    assert_eq!(
        selected(
            8,
            DataComponentPatchSummary {
                map_color: Some(0x45_67_89),
                removed_type_ids: vec![45],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_map_color_fallback")
    );

    // `DataComponents.CUSTOM_NAME` uses ComponentSerialization.CODEC;
    // this pins JSON-string / {"text": "..."} literal cases and removed id
    // 6 suppression.
    assert_eq!(
        selected(9, DataComponentPatchSummary::default()),
        uv("component_custom_name_fallback")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                custom_name: Some("Named Selector".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_custom_name_named")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                custom_name: Some("Object Selector".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_custom_name_object")
    );
    assert_eq!(
        selected(
            9,
            DataComponentPatchSummary {
                custom_name: Some("Named Selector".to_string()),
                removed_type_ids: vec![6],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_custom_name_fallback")
    );

    // `DataComponents.ITEM_NAME` also uses ComponentSerialization.CODEC,
    // and `Item.Properties.finalizeInitializer` contributes the default
    // translatable item-name component. Literal strings must not match that
    // default translatable key.
    assert_eq!(
        selected(10, DataComponentPatchSummary::default()),
        uv("component_item_name_default")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                item_name: Some("Patched Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_name_literal")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                item_name: Some("Object Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_name_object")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                item_name: Some("item.minecraft.item_name_component_selector".to_string()),
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_name_literal_key")
    );
    assert_eq!(
        selected(
            10,
            DataComponentPatchSummary {
                removed_type_ids: vec![9],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_name_fallback")
    );
    assert_eq!(
        selected(11, DataComponentPatchSummary::default()),
        uv("component_item_name_block_default")
    );
    assert_eq!(
        selected(
            11,
            DataComponentPatchSummary {
                removed_type_ids: vec![9],
                ..DataComponentPatchSummary::default()
            }
        ),
        uv("component_item_name_fallback")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_resolves_value_aware_range_dispatch() {
    let root = unique_temp_dir("item-runtime-range-dispatch");
    write_value_aware_range_dispatch_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let uv = |model_id: &str| {
        runtime
            .textures
            .texture_uv_rect(runtime.texture_index(&format!("minecraft:item/{model_id}")))
            .unwrap()
    };
    let damage_stack = |damage: Option<i32>, max_damage: Option<i32>| ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            damage,
            max_damage,
            ..DataComponentPatchSummary::default()
        },
    };
    let selected = |stack: &ItemStackSummary| runtime.icon_for_stack(stack).unwrap().layers[0].uv;

    // damage 50/100 = 0.5 lands exactly on the 0.5 threshold (vanilla
    // `lastIndexLessOrEqual` is inclusive), proving sort + boundary.
    assert_eq!(
        selected(&damage_stack(Some(50), Some(100))),
        uv("damage_half")
    );
    // damage 95/100 = 0.95 reaches the top entry.
    assert_eq!(
        selected(&damage_stack(Some(95), Some(100))),
        uv("damage_low")
    );
    // damage 40/100 = 0.4 precedes the first threshold (0.5) → fallback (-1).
    assert_eq!(
        selected(&damage_stack(Some(40), Some(100))),
        uv("damage_fallback")
    );
    // No max_damage → 0/0 = NaN → fallback.
    assert_eq!(selected(&damage_stack(None, None)), uv("damage_fallback"));

    let cmd_stack = |floats: Vec<f32>| ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary {
            custom_model_data_floats: floats.into(),
            ..DataComponentPatchSummary::default()
        },
    };
    // floats[1] = 0.5, scale 2.0 → 1.0 lands on the 1.0 threshold; floats[0]
    // is ignored (index 1), proving index handling, scale, and boundary.
    assert_eq!(selected(&cmd_stack(vec![9.0, 0.5])), uv("cmd_1"));
    // floats[1] = 2.0 * 2.0 = 4.0 reaches the 3.0 entry.
    assert_eq!(selected(&cmd_stack(vec![9.0, 2.0])), uv("cmd_3"));
    // Missing index 1 → 0.0 → the 0.0 entry.
    assert_eq!(selected(&cmd_stack(vec![9.0])), uv("cmd_0"));
    // No custom_model_data at all → 0.0 → the 0.0 entry.
    assert_eq!(selected(&cmd_stack(Vec::new())), uv("cmd_0"));

    let count_stack = |count: i32, max_stack_size: Option<i32>| ItemStackSummary {
        item_id: Some(2),
        count,
        component_patch: DataComponentPatchSummary {
            max_stack_size,
            ..DataComponentPatchSummary::default()
        },
    };
    // Count.get defaults to normalized `count / maxStackSize`, using the
    // item prototype default when the component is absent.
    assert_eq!(selected(&count_stack(32, None)), uv("count_half"));
    assert_eq!(selected(&count_stack(64, None)), uv("count_full"));
    assert_eq!(selected(&count_stack(1, None)), uv("count_fallback"));
    // The max_stack_size component overrides the prototype default.
    assert_eq!(selected(&count_stack(8, Some(16))), uv("count_half"));

    let apple_template = |count: i32| ItemStackTemplateSummary {
        item_id: 4,
        count,
        component_patch: DataComponentPatchSummary::default(),
    };
    let bundle_stack = |items: Vec<ItemStackTemplateSummary>| ItemStackSummary {
        item_id: Some(3),
        count: 1,
        component_patch: DataComponentPatchSummary {
            bundle_contents_item_count: Some(items.len()),
            bundle_contents_items: items,
            ..DataComponentPatchSummary::default()
        },
    };
    // BundleFullness.get sums BundleContents weights; regular items weigh
    // `count / getMaxStackSize`.
    assert_eq!(
        selected(&bundle_stack(vec![apple_template(32)])),
        uv("bundle_half")
    );
    assert_eq!(
        selected(&bundle_stack(vec![apple_template(16)])),
        uv("bundle_fallback")
    );
    // A nested bundle weighs its contents plus the fixed 1/16 bundle item weight.
    assert_eq!(
        selected(&bundle_stack(vec![ItemStackTemplateSummary {
            item_id: 3,
            count: 1,
            component_patch: DataComponentPatchSummary {
                bundle_contents_item_count: Some(1),
                bundle_contents_items: vec![apple_template(32)],
                ..DataComponentPatchSummary::default()
            },
        }])),
        uv("bundle_nested")
    );
    // A beehive-like stack with non-empty BEES component weighs as a full bundle.
    assert_eq!(
        selected(&bundle_stack(vec![ItemStackTemplateSummary {
            item_id: 5,
            count: 1,
            component_patch: DataComponentPatchSummary {
                bees_count: 1,
                ..DataComponentPatchSummary::default()
            },
        }])),
        uv("bundle_full")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_selects_bundle_icon_from_local_selected_item() {
    let root = unique_temp_dir("item-runtime-bundle-selected");
    write_bundle_selected_item_fixture(&root);

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
    let normal_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle"))
        .unwrap();
    let open_back_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_back"))
        .unwrap();
    let open_front_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/bundle_open_front"))
        .unwrap();
    let apple_uv = runtime
        .textures
        .texture_uv_rect(runtime.texture_index("minecraft:item/apple"))
        .unwrap();
    let bundle_stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            bundle_contents_items: vec![ItemStackTemplateSummary {
                item_id: 1,
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }],
            bundle_contents_item_count: Some(1),
            ..DataComponentPatchSummary::default()
        },
    };

    assert_eq!(
        runtime.icon_texture_index_for_protocol_id(0),
        Some(runtime.texture_index("minecraft:item/bundle"))
    );

    let default_icon = runtime.icon_for_stack(&bundle_stack).unwrap();
    assert_eq!(default_icon.layers[0].uv, normal_uv);

    let unselected_icon = runtime
        .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(-1))
        .unwrap();
    assert_eq!(unselected_icon.layers[0].uv, normal_uv);

    let selected_icon = runtime
        .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(0))
        .unwrap();
    assert_eq!(
        selected_icon
            .layers
            .iter()
            .map(|layer| layer.uv)
            .collect::<Vec<_>>(),
        vec![open_back_uv, apple_uv, open_front_uv]
    );

    let out_of_bounds_icon = runtime
        .icon_for_stack_with_bundle_selected_item(&bundle_stack, Some(1))
        .unwrap();
    assert_eq!(out_of_bounds_icon.layers[0].uv, normal_uv);

    let no_contents_icon = runtime
        .icon_for_stack_with_bundle_selected_item(
            &ItemStackSummary {
                item_id: Some(0),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
            Some(0),
        )
        .unwrap();
    assert_eq!(no_contents_icon.layers[0].uv, normal_uv);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_item_runtime_loads_assets_when_registry_source_is_missing() {
    let root = unique_temp_dir("item-runtime-no-registry");
    let assets = assets_dir(&root);
    write_item_atlases(&assets);
    write_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_json(
        &assets.join("models").join("item").join("test_sword.json"),
        r#"{
                "textures": {
                    "layer0": "minecraft:item/test_sword"
                }
            }"#,
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("test_sword.png"),
        1,
        1,
        &[255, 0, 0, 255],
    );

    let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

    assert_eq!(runtime.item_definition_count(), 1);
    assert_eq!(runtime.item_registry_count(), 0);
    assert_eq!(runtime.item_equipment_slot_count(), 0);
    assert_eq!(runtime.item_mining_profile_count(), 0);
    assert!(runtime.item_equipment_slots_by_protocol_id().is_empty());
    assert!(runtime.item_mining_profiles_by_protocol_id().is_empty());
    assert_eq!(runtime.texture_count(), 2);
    assert_eq!(runtime.icon_texture_count(), 1);
    assert!(!runtime.atlas_rgba().is_empty());
    assert_eq!(runtime.icon_texture_index_for_protocol_id(0), None);
    assert_eq!(runtime.icon_uv_for_protocol_id(0), None);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 sources"]
fn loads_local_vanilla_item_runtime_assets() {
    let runtime = NativeItemRuntime::load(&PackRoots::discover().unwrap()).unwrap();

    assert_eq!(runtime.item_definition_count(), 1506);
    assert_eq!(runtime.item_registry_count(), 1506);
    assert_eq!(runtime.texture_count(), 1576);
    assert_eq!(runtime.icon_texture_count(), 1506);
    assert_eq!(runtime.missing_model_count(), 0);
    assert_eq!(runtime.missing_texture_count(), 0);
    assert!(runtime.icon_uv_for_protocol_id(1).is_some());
    assert!(runtime.resolved_model_count() > runtime.item_definition_count());
    assert!(runtime.atlas_size().0 > 0);
    assert!(runtime.atlas_size().1 > 0);
}

fn assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn write_item_atlases(assets: &Path) {
    write_json(
        &assets.join("atlases").join("items.json"),
        r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "item/",
                        "source": "item"
                    }
                ]
            }"#,
    );
    write_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "block/",
                        "source": "block"
                    }
                ]
            }"#,
    );
}

fn write_elytra_damage_condition_fixture(
    root: &Path,
    property: &str,
    true_model: &str,
    true_color: &[u8],
) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item ELYTRA = registerItem(
                    "elytra",
                    Item::new,
                    new Item.Properties().durability(432)
                );
            }"#,
    );
    write_json(
        &assets.join("items").join("elytra.json"),
        &format!(
            r#"{{
                "model": {{
                    "type": "minecraft:condition",
                    "property": "{property}",
                    "on_false": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/elytra"
                    }},
                    "on_true": {{
                        "type": "minecraft:model",
                        "model": "minecraft:item/{true_model}"
                    }}
                }}
            }}"#
        ),
    );
    write_json(
        &assets.join("models").join("item").join("elytra.json"),
        r#"{
                "textures": {
                    "layer0": "minecraft:item/elytra"
                }
            }"#,
    );
    write_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{true_model}.json")),
        &format!(
            r#"{{
                "textures": {{
                    "layer0": "minecraft:item/{true_model}"
                }}
            }}"#
        ),
    );
    write_test_rgba_png(
        &assets.join("textures").join("item").join("elytra.png"),
        1,
        1,
        &[40, 80, 120, 255],
    );
    write_test_rgba_png(
        &assets
            .join("textures")
            .join("item")
            .join(format!("{true_model}.png")),
        1,
        1,
        true_color,
    );
}

fn write_wolf_armor_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "wolf_armor");
    write_json(
        &assets.join("items").join("wolf_armor.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:dyed_color",
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/wolf_armor"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/wolf_armor_dyed",
                        "tints": [
                            { "type": "minecraft:dye", "default": 0 }
                        ]
                    }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "wolf_armor", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "wolf_armor_dyed", &[120, 80, 40, 255]);
}

fn write_compass_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "compass");
    write_json(
        &assets.join("items").join("compass.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:lodestone_tracker",
                    "on_true": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:compass",
                        "target": "lodestone",
                        "scale": 32.0,
                        "entries": [
                            {
                                "threshold": 0.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/compass_lodestone"
                                }
                            }
                        ]
                    },
                    "on_false": {
                        "type": "minecraft:range_dispatch",
                        "property": "minecraft:compass",
                        "target": "spawn",
                        "scale": 32.0,
                        "entries": [
                            {
                                "threshold": 0.0,
                                "model": {
                                    "type": "minecraft:model",
                                    "model": "minecraft:item/compass"
                                }
                            }
                        ]
                    }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "compass", &[40, 120, 80, 255]);
    write_flat_item_model_and_texture(&assets, "compass_lodestone", &[120, 40, 80, 255]);
}

fn write_default_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(
        root,
        &[
            "has_max_stack",
            "has_max_stack_ignore_default",
            "has_rarity",
            "has_enchantments",
            "enchanted_book",
        ],
    );
    write_json(
        &assets.join("items").join("has_max_stack.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:max_stack_size",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_max_stack_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_max_stack_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("has_max_stack_ignore_default.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:max_stack_size",
                    "ignore_default": true,
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_max_stack_patched"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_max_stack_unpatched"
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("has_rarity.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:rarity",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_rarity_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_rarity_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("has_enchantments.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:enchantments",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_enchantments_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("enchanted_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:has_component",
                    "component": "minecraft:stored_enchantments",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_stored_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/has_stored_enchantments_absent"
                    }
                }
            }"#,
    );
    for (model_id, color) in [
        ("has_max_stack_present", [40, 140, 80, 255]),
        ("has_max_stack_absent", [80, 40, 40, 255]),
        ("has_max_stack_patched", [40, 80, 180, 255]),
        ("has_max_stack_unpatched", [40, 40, 80, 255]),
        ("has_rarity_present", [180, 80, 220, 255]),
        ("has_rarity_absent", [60, 40, 80, 255]),
        ("has_enchantments_present", [220, 200, 80, 255]),
        ("has_enchantments_absent", [80, 70, 30, 255]),
        ("has_stored_enchantments_present", [230, 210, 100, 255]),
        ("has_stored_enchantments_absent", [90, 80, 40, 255]),
    ] {
        write_flat_item_model_and_texture(&assets, model_id, &color);
    }
}

fn write_component_condition_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item COMPONENT_CONDITION_RARITY = registerItem("component_condition_rarity");
                public static final Item COMPONENT_CONDITION_GLINT = registerItem("component_condition_glint");
                public static final Item COMPONENT_CONDITION_DAMAGE = registerItem("component_condition_damage", new Item.Properties().durability(10));
                public static final Item COMPONENT_CONDITION_BUNDLE_CONTENTS = registerItem("component_condition_bundle_contents");
                public static final Item COMPONENT_CONDITION_TRIM = registerItem("component_condition_trim");
                public static final Item COMPONENT_CONDITION_FIREWORK_EXPLOSION = registerItem("component_condition_firework_explosion");
                public static final Item COMPONENT_CONDITION_FIREWORKS = registerItem("component_condition_fireworks");
                public static final Item COMPONENT_CONDITION_JUKEBOX_PLAYABLE = registerItem("component_condition_jukebox_playable");
                public static final Item COMPONENT_CONDITION_CONTAINER = registerItem("component_condition_container");
                public static final Item COMPONENT_CONDITION_BUNDLE_CONTENTS_CONSTRAINED = registerItem("component_condition_bundle_contents_constrained");
                public static final Item COMPONENT_CONDITION_FIREWORK_EXPLOSION_STAR = registerItem("component_condition_firework_explosion_star");
                public static final Item COMPONENT_CONDITION_FIREWORKS_FLIGHT = registerItem("component_condition_fireworks_flight");
                public static final Item COMPONENT_CONDITION_FIREWORKS_EXPLOSIONS = registerItem("component_condition_fireworks_explosions");
                public static final Item COMPONENT_CONDITION_TRIM_MATERIAL = registerItem("component_condition_trim_material");
                public static final Item COMPONENT_CONDITION_TRIM_PATTERN = registerItem("component_condition_trim_pattern");
                public static final Item COMPONENT_CONDITION_ENCHANTMENTS_LEVEL = registerItem("component_condition_enchantments_level");
                public static final Item COMPONENT_CONDITION_ENCHANTMENTS_EMPTY = registerItem("component_condition_enchantments_empty");
                public static final Item COMPONENT_CONDITION_STORED_ENCHANTMENTS_LEVEL = registerItem("component_condition_stored_enchantments_level");
                public static final Item COMPONENT_CONDITION_STORED_ENCHANTMENTS_EMPTY = registerItem("component_condition_stored_enchantments_empty");
                public static final Item COMPONENT_CONDITION_FIREWORKS_CONTAINS = registerItem("component_condition_fireworks_contains");
                public static final Item COMPONENT_CONDITION_FIREWORKS_COUNT = registerItem("component_condition_fireworks_count");
                public static final Item COMPONENT_CONDITION_BUNDLE_CONTAINS = registerItem("component_condition_bundle_contains");
                public static final Item COMPONENT_CONDITION_BUNDLE_COUNT = registerItem("component_condition_bundle_count");
                public static final Item COMPONENT_CONDITION_CONTAINER_CONTAINS = registerItem("component_condition_container_contains");
                public static final Item COMPONENT_CONDITION_CONTAINER_COUNT = registerItem("component_condition_container_count");
                public static final Item COMPONENT_CONDITION_ENCHANTMENTS_HOLDER = registerItem("component_condition_enchantments_holder");
                public static final Item COMPONENT_CONDITION_STORED_ENCHANTMENTS_HOLDER = registerItem("component_condition_stored_enchantments_holder");
                public static final Item ENCHANTED_BOOK = registerItem("enchanted_book");
                public static final Item COMPONENT_CONDITION_BUNDLE_TAG_CONTAINS = registerItem("component_condition_bundle_tag_contains");
                public static final Item COMPONENT_CONDITION_CONTAINER_TAG_COUNT = registerItem("component_condition_container_tag_count");
                public static final Item COMPONENT_CONDITION_ENCHANTMENTS_TAG = registerItem("component_condition_enchantments_tag");
                public static final Item COMPONENT_CONDITION_STORED_ENCHANTMENTS_TAG = registerItem("component_condition_stored_enchantments_tag");
                public static final Item COMPONENT_CONDITION_BUNDLE_COMPONENTS = registerItem("component_condition_bundle_components");
                public static final Item COMPONENT_CONDITION_CONTAINER_COMPONENTS = registerItem("component_condition_container_components");
                public static final Item COMPONENT_CONDITION_TRIM_MATERIAL_TAG = registerItem("component_condition_trim_material_tag");
                public static final Item COMPONENT_CONDITION_TRIM_PATTERN_TAG = registerItem("component_condition_trim_pattern_tag");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_DAMAGE = registerItem("component_condition_bundle_partial_damage");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_DAMAGE = registerItem("component_condition_container_partial_damage");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_ANY_VALUE = registerItem("component_condition_bundle_partial_any_value");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_ANY_VALUE = registerItem("component_condition_container_partial_any_value");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_ENCHANTMENTS = registerItem("component_condition_bundle_partial_enchantments");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_STORED_ENCHANTMENTS = registerItem("component_condition_container_partial_stored_enchantments");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_FIREWORK_EXPLOSION = registerItem("component_condition_bundle_partial_firework_explosion");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_FIREWORKS = registerItem("component_condition_container_partial_fireworks");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_TRIM = registerItem("component_condition_bundle_partial_trim");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_TRIM = registerItem("component_condition_container_partial_trim");
                public static final Item COMPONENT_CONDITION_JUKEBOX_PLAYABLE_SONG = registerItem("component_condition_jukebox_playable_song");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_JUKEBOX_PLAYABLE = registerItem("component_condition_bundle_partial_jukebox_playable");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_JUKEBOX_PLAYABLE = registerItem("component_condition_container_partial_jukebox_playable");
                public static final Item COMPONENT_CONDITION_POTION_CONTENTS = registerItem("component_condition_potion_contents");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_POTION_CONTENTS = registerItem("component_condition_bundle_partial_potion_contents");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_POTION_CONTENTS = registerItem("component_condition_container_partial_potion_contents");
                public static final Item COMPONENT_CONDITION_WRITABLE_BOOK_PAGES = registerItem("component_condition_writable_book_pages");
                public static final Item COMPONENT_CONDITION_WRITTEN_BOOK_CONTENT = registerItem("component_condition_written_book_content");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_WRITABLE_BOOK = registerItem("component_condition_bundle_partial_writable_book");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_WRITTEN_BOOK = registerItem("component_condition_container_partial_written_book");
                public static final Item COMPONENT_CONDITION_VILLAGER_VARIANT = registerItem("component_condition_villager_variant");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_VILLAGER_VARIANT = registerItem("component_condition_bundle_partial_villager_variant");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_VILLAGER_VARIANT = registerItem("component_condition_container_partial_villager_variant");
                public static final Item COMPONENT_CONDITION_ATTRIBUTE_MODIFIERS = registerItem("component_condition_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_ATTRIBUTE_MODIFIERS = registerItem("component_condition_container_partial_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_ATTRIBUTE_MODIFIERS_ATTRIBUTE = registerItem("component_condition_attribute_modifiers_attribute");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_ATTRIBUTE_MODIFIERS_ATTRIBUTE = registerItem("component_condition_container_partial_attribute_modifiers_attribute");
                public static final Item COMPONENT_CONDITION_CUSTOM_DATA = registerItem("component_condition_custom_data");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_CUSTOM_DATA = registerItem("component_condition_bundle_partial_custom_data");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_CUSTOM_DATA = registerItem("component_condition_container_partial_custom_data");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_ATTRIBUTE_MODIFIERS = registerItem("component_condition_bundle_partial_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_ATTRIBUTE_MODIFIERS_ATTRIBUTE = registerItem("component_condition_bundle_partial_attribute_modifiers_attribute");
                public static final Item COMPONENT_CONDITION_ATTRIBUTE_MODIFIERS_ATTRIBUTE_TAG = registerItem("component_condition_attribute_modifiers_attribute_tag");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_ATTRIBUTE_MODIFIERS_ATTRIBUTE_TAG = registerItem("component_condition_bundle_partial_attribute_modifiers_attribute_tag");
                public static final Item COMPONENT_CONDITION_VILLAGER_VARIANT_TAG = registerItem("component_condition_villager_variant_tag");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_VILLAGER_VARIANT_TAG = registerItem("component_condition_bundle_partial_villager_variant_tag");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_VILLAGER_VARIANT_TAG = registerItem("component_condition_container_partial_villager_variant_tag");
                public static final Item COMPONENT_CONDITION_DEFAULT_ATTRIBUTE_MODIFIERS = registerItem("component_condition_default_attribute_modifiers", new Item.Properties().sword(ToolMaterial.IRON, 3.0F, -2.4F));
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_DEFAULT_ATTRIBUTE_MODIFIERS = registerItem("component_condition_bundle_partial_default_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_CONTAINER_PARTIAL_DEFAULT_ATTRIBUTE_MODIFIERS = registerItem("component_condition_container_partial_default_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_DEFAULT_ARMOR_ATTRIBUTE_MODIFIERS = registerItem("component_condition_default_armor_attribute_modifiers", new Item.Properties().humanoidArmor(ArmorMaterials.DIAMOND, ArmorType.HELMET));
                public static final Item COMPONENT_CONDITION_DEFAULT_MACE_ATTRIBUTE_MODIFIERS = registerItem("component_condition_default_mace_attribute_modifiers", MaceItem::new, new Item.Properties().attributes(MaceItem.createAttributes()));
                public static final Item COMPONENT_CONDITION_CUSTOM_DATA_SNBT = registerItem("component_condition_custom_data_snbt");
                public static final Item COMPONENT_CONDITION_BUNDLE_PARTIAL_CUSTOM_DATA_SNBT = registerItem("component_condition_bundle_partial_custom_data_snbt");
                public static final Item COMPONENT_CONDITION_WRITTEN_BOOK_COMPONENT_PAGE = registerItem("component_condition_written_book_component_page");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_COMPONENT_TEXT = registerItem("component_condition_bundle_exact_component_text");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_LORE = registerItem("component_condition_bundle_exact_lore");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_UNBREAKABLE = registerItem("component_condition_bundle_exact_unbreakable");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_CUSTOM_DATA = registerItem("component_condition_bundle_exact_custom_data");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_POTION_CONTENTS = registerItem("component_condition_bundle_exact_potion_contents");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_WRITABLE_BOOK = registerItem("component_condition_bundle_exact_writable_book");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_FIREWORK_EXPLOSION = registerItem("component_condition_bundle_exact_firework_explosion");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_FIREWORKS = registerItem("component_condition_bundle_exact_fireworks");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_JUKEBOX_PLAYABLE = registerItem("component_condition_bundle_exact_jukebox_playable");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_TRIM = registerItem("component_condition_bundle_exact_trim");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_ENCHANTMENTS = registerItem("component_condition_bundle_exact_enchantments");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_STORED_ENCHANTMENTS = registerItem("component_condition_bundle_exact_stored_enchantments");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_VILLAGER_VARIANT = registerItem("component_condition_bundle_exact_villager_variant");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_LODESTONE_TRACKER = registerItem("component_condition_bundle_exact_lodestone_tracker");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_ATTRIBUTE_MODIFIERS = registerItem("component_condition_bundle_exact_attribute_modifiers");
                public static final Item COMPONENT_CONDITION_BUNDLE_EXACT_WRITTEN_BOOK = registerItem("component_condition_bundle_exact_written_book");
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("item")
            .join("component_condition_tagged.json"),
        r#"{
                "values": [
                    "minecraft:component_condition_rarity",
                    "minecraft:component_condition_glint"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("villager_type")
            .join("component_condition_villager_types.json"),
        r#"{
                "values": [
                    "minecraft:plains"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("enchantment")
            .join("component_condition_tagged.json"),
        r#"{
                "values": [
                    "minecraft:sharpness"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("trim_material")
            .join("component_condition_trim_materials.json"),
        r#"{
                "values": [
                    "minecraft:diamond"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("trim_pattern")
            .join("component_condition_trim_patterns.json"),
        r#"{
                "values": [
                    "minecraft:sentry"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("jukebox_song")
            .join("component_condition_jukebox_songs.json"),
        r#"{
                "values": [
                    "minecraft:cat"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("potion")
            .join("component_condition_potions.json"),
        r#"{
                "values": [
                    "minecraft:healing"
                ]
            }"#,
    );
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("tags")
            .join("attribute")
            .join("component_condition_damage_attributes.json"),
        r#"{
                "values": [
                    "minecraft:generic.attack_damage"
                ]
            }"#,
    );
    write_json(
        &assets.join("items").join("component_condition_rarity.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:rarity",
                    "value": {},
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_rarity_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_rarity_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("component_condition_glint.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantment_glint_override",
                    "value": {},
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_glint_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_glint_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("component_condition_damage.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:damage",
                    "value": {
                        "damage": 3,
                        "durability": {
                            "min": 4,
                            "max": 8
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_damage_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_damage_absent"
                    }
                }
            }"#,
    );
    for (item_id, predicate) in [
        (
            "component_condition_bundle_contents",
            "minecraft:bundle_contents",
        ),
        ("component_condition_trim", "minecraft:trim"),
        (
            "component_condition_firework_explosion",
            "minecraft:firework_explosion",
        ),
        ("component_condition_fireworks", "minecraft:fireworks"),
        (
            "component_condition_jukebox_playable",
            "minecraft:jukebox_playable",
        ),
        ("component_condition_container", "minecraft:container"),
    ] {
        write_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &format!(
                r#"{{
                        "model": {{
                            "type": "minecraft:condition",
                            "property": "minecraft:component",
                            "predicate": "{predicate}",
                            "value": {{}},
                            "on_true": {{
                                "type": "minecraft:model",
                                "model": "minecraft:item/{item_id}_present"
                            }},
                            "on_false": {{
                                "type": "minecraft:model",
                                "model": "minecraft:item/{item_id}_absent"
                            }}
                        }}
                    }}"#,
            ),
        );
    }
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_contents_constrained.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "size": 1
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_contents_constrained_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_contents_constrained_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_contains.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "items": "minecraft:component_condition_rarity",
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_contains_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_contains_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_count.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "items": [
                                            "minecraft:component_condition_rarity",
                                            "minecraft:component_condition_glint"
                                        ]
                                    },
                                    "count": 2
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_count_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_count_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_contains.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "items": "minecraft:component_condition_glint",
                                    "count": 4
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_contains_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_contains_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_count.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "items": [
                                            "minecraft:component_condition_rarity",
                                            "minecraft:component_condition_glint"
                                        ]
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ],
                            "size": {
                                "min": 2
                            }
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_count_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_count_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_tag_contains.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "items": "#minecraft:component_condition_tagged",
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_tag_contains_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_tag_contains_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_tag_count.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "items": "#minecraft:component_condition_tagged"
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_tag_count_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_tag_count_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_components.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:rarity": "rare"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_components_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_components_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_component_text.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:custom_name": {
                                                "text": "Custom Name"
                                            },
                                            "minecraft:item_name": "Item Name"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_component_text_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_component_text_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_lore.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:lore": [
                                                {
                                                    "text": "Lore one"
                                                },
                                                "Lore two"
                                            ]
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_lore_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_lore_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_unbreakable.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:unbreakable": {}
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_unbreakable_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_unbreakable_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_custom_data.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:custom_data": "{owner:\"Alex\",level:7}"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_custom_data_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_custom_data_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_potion_contents.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:potion_contents": {
                                                "potion": "minecraft:healing",
                                                "custom_color": 7833753,
                                                "custom_effects": [
                                                    {
                                                        "id": "minecraft:haste",
                                                        "amplifier": 1,
                                                        "duration": 200,
                                                        "hidden_effect": {
                                                            "amplifier": 0,
                                                            "duration": 40,
                                                            "ambient": false,
                                                            "show_particles": true,
                                                            "show_icon": false
                                                        }
                                                    }
                                                ],
                                                "custom_name": "healing"
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_potion_contents_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_potion_contents_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_writable_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:writable_book_content": {
                                                "pages": [
                                                    "alpha",
                                                    {
                                                        "raw": "beta",
                                                        "filtered": "filtered beta"
                                                    }
                                                ]
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_writable_book_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_writable_book_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_firework_explosion.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:firework_explosion": {
                                                "shape": "star",
                                                "colors": [
                                                    1122867
                                                ],
                                                "fade_colors": [
                                                    4478310
                                                ],
                                                "has_trail": true,
                                                "has_twinkle": false
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_firework_explosion_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_firework_explosion_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_fireworks.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:fireworks": {
                                                "flight_duration": 2,
                                                "explosions": [
                                                    {
                                                        "shape": "star",
                                                        "colors": [
                                                            1122867
                                                        ],
                                                        "fade_colors": [
                                                            4478310
                                                        ],
                                                        "has_trail": true,
                                                        "has_twinkle": false
                                                    }
                                                ]
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_fireworks_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_fireworks_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_jukebox_playable.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:jukebox_playable": {
                                                "sound_event": "minecraft:entity.cat.ambient",
                                                "description": {
                                                    "text": "Test",
                                                    "extra": [
                                                        {
                                                            "text": " song"
                                                        }
                                                    ]
                                                },
                                                "length_in_seconds": 3.5,
                                                "comparator_output": 7
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_jukebox_playable_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_jukebox_playable_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_trim.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:trim": {
                                                "material": {
                                                    "asset_name": "test_material",
                                                    "override_armor_assets": {
                                                        "minecraft:iron": "test_material_iron"
                                                    },
                                                    "description": {
                                                        "text": "Test material"
                                                    }
                                                },
                                                "pattern": {
                                                    "asset_id": "minecraft:test_pattern",
                                                    "description": {
                                                        "text": "Test",
                                                        "extra": [
                                                            {
                                                                "text": " pattern"
                                                            }
                                                        ]
                                                    },
                                                    "decal": true
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_trim_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_trim_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_enchantments.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:enchantments": {
                                                "minecraft:sharpness": 3,
                                                "minecraft:mending": 1
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_enchantments_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_stored_enchantments.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:stored_enchantments": {
                                                "minecraft:mending": 1
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_stored_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_stored_enchantments_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_villager_variant.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:villager/variant": "minecraft:plains"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_villager_variant_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_villager_variant_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_lodestone_tracker.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:lodestone_tracker": {
                                                "target": {
                                                    "dimension": "minecraft:overworld",
                                                    "pos": [
                                                        10,
                                                        64,
                                                        -4
                                                    ]
                                                },
                                                "tracked": false
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_lodestone_tracker_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_lodestone_tracker_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:attribute_modifiers": [
                                                {
                                                    "type": "minecraft:generic.attack_damage",
                                                    "id": "minecraft:test/attack",
                                                    "amount": 2.5,
                                                    "operation": "add_value",
                                                    "slot": "mainhand",
                                                    "display": {
                                                        "type": "hidden"
                                                    }
                                                },
                                                {
                                                    "type": "minecraft:generic.scale",
                                                    "id": "minecraft:test/scale",
                                                    "amount": 1.25,
                                                    "operation": "add_multiplied_base",
                                                    "slot": "hand",
                                                    "display": {
                                                        "type": "override",
                                                        "value": {
                                                            "text": "Heavy",
                                                            "bold": true,
                                                            "extra": [
                                                                {
                                                                    "text": " hit",
                                                                    "color": "red"
                                                                }
                                                            ]
                                                        }
                                                    }
                                                }
                                            ]
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_exact_written_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "components": {
                                            "minecraft:written_book_content": {
                                                "title": {
                                                    "raw": "Quest",
                                                    "filtered": "Filtered Quest"
                                                },
                                                "author": "Alex",
                                                "generation": 1,
                                                "pages": [
                                                    {
                                                        "text": "First",
                                                        "extra": [
                                                            {
                                                                "text": " page",
                                                                "italic": true
                                                            }
                                                        ]
                                                    },
                                                    {
                                                        "raw": {
                                                            "translate": "Second",
                                                            "with": [
                                                                "page"
                                                            ]
                                                        },
                                                        "filtered": {
                                                            "text": "Filtered second",
                                                            "extra": [
                                                                {
                                                                    "text": " page",
                                                                    "color": "blue"
                                                                }
                                                            ]
                                                        }
                                                    }
                                                ],
                                                "resolved": true
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_written_book_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_exact_written_book_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_components.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "components": {
                                                "minecraft:rarity": "common"
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_components_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_components_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_damage.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:damage": {
                                                "damage": 3,
                                                "durability": {
                                                    "min": 4,
                                                    "max": 8
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_damage_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_damage_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_damage.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:damage": {
                                                    "damage": 3
                                                }
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_damage_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_damage_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_any_value.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:rarity": {}
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_any_value_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_any_value_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_any_value.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:enchantment_glint_override": {}
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_any_value_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_any_value_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_enchantments.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:enchantments": [
                                                {
                                                    "enchantments": "#minecraft:component_condition_tagged",
                                                    "levels": {
                                                        "min": 2
                                                    }
                                                }
                                            ]
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_enchantments_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_stored_enchantments.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:stored_enchantments": [
                                                    {
                                                        "enchantments": "minecraft:mending",
                                                        "levels": 1
                                                    }
                                                ]
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_stored_enchantments_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_stored_enchantments_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_firework_explosion.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:firework_explosion": {
                                                "shape": "star",
                                                "has_trail": true,
                                                "has_twinkle": false
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_firework_explosion_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_firework_explosion_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_fireworks.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:fireworks": {
                                                    "flight_duration": {
                                                        "min": 2,
                                                        "max": 4
                                                    },
                                                    "explosions": {
                                                        "contains": [
                                                            {
                                                                "shape": "burst",
                                                                "has_twinkle": true
                                                            }
                                                        ]
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_fireworks_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_fireworks_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_trim.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:trim": {
                                                "material": "#minecraft:component_condition_trim_materials"
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_trim_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_trim_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_trim.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:trim": {
                                                    "pattern": [
                                                        "#minecraft:component_condition_trim_patterns"
                                                    ]
                                                }
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_trim_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_trim_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_jukebox_playable.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:jukebox_playable": {
                                                "song": "#minecraft:component_condition_jukebox_songs"
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_jukebox_playable_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_jukebox_playable_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_jukebox_playable.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:jukebox_playable": {
                                                    "song": [
                                                        "minecraft:cat"
                                                    ]
                                                }
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_jukebox_playable_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_jukebox_playable_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_potion_contents.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:potion_contents": "#minecraft:component_condition_potions"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_potion_contents_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_potion_contents_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_potion_contents.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:potion_contents": [
                                                    "minecraft:healing"
                                                ]
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_potion_contents_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_potion_contents_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_firework_explosion_star.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:firework_explosion",
                    "value": {
                        "shape": "star",
                        "has_trail": true,
                        "has_twinkle": false
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_firework_explosion_star_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_firework_explosion_star_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_fireworks_flight.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:fireworks",
                    "value": {
                        "flight_duration": {
                            "min": 2,
                            "max": 3
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_flight_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_flight_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_fireworks_explosions.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:fireworks",
                    "value": {
                        "explosions": {
                            "size": 1
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_explosions_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_explosions_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_fireworks_contains.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:fireworks",
                    "value": {
                        "explosions": {
                            "contains": [
                                {
                                    "shape": "star",
                                    "has_trail": true
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_contains_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_contains_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_fireworks_count.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:fireworks",
                    "value": {
                        "explosions": {
                            "count": [
                                {
                                    "test": {
                                        "has_twinkle": true
                                    },
                                    "count": 2
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_count_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_fireworks_count_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_trim_material.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:trim",
                    "value": {
                        "material": "minecraft:diamond"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_material_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_material_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_trim_pattern.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:trim",
                    "value": {
                        "pattern": "minecraft:sentry"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_pattern_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_pattern_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_trim_material_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:trim",
                    "value": {
                        "material": "#minecraft:component_condition_trim_materials"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_material_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_material_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_trim_pattern_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:trim",
                    "value": {
                        "pattern": [
                            "#minecraft:component_condition_trim_patterns"
                        ]
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_pattern_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_trim_pattern_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_jukebox_playable_song.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:jukebox_playable",
                    "value": {
                        "song": "minecraft:cat"
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_jukebox_playable_song_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_jukebox_playable_song_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_potion_contents.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:potion_contents",
                    "value": "minecraft:healing",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_potion_contents_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_potion_contents_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_writable_book_pages.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:writable_book_content",
                    "value": {
                        "pages": {
                            "contains": [
                                "alpha",
                                "beta"
                            ],
                            "count": [
                                {
                                    "test": "alpha",
                                    "count": 2
                                }
                            ],
                            "size": 3
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_writable_book_pages_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_writable_book_pages_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_written_book_content.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:written_book_content",
                    "value": {
                        "author": "Alex",
                        "title": "Quest",
                        "generation": {
                            "min": 1,
                            "max": 2
                        },
                        "resolved": true,
                        "pages": {
                            "contains": [
                                "First page"
                            ],
                            "count": [
                                {
                                    "test": "First page",
                                    "count": 1
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_written_book_content_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_written_book_content_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_written_book_component_page.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:written_book_content",
                    "value": {
                        "pages": {
                            "contains": [
                                {
                                    "text": "First page"
                                }
                            ],
                            "count": [
                                {
                                    "test": {
                                        "text": "First page"
                                    },
                                    "count": 1
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_written_book_component_page_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_written_book_component_page_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_writable_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:writable_book_content": {
                                                "pages": {
                                                    "contains": [
                                                        "alpha",
                                                        "beta"
                                                    ],
                                                    "count": [
                                                        {
                                                            "test": "alpha",
                                                            "count": 2
                                                        }
                                                    ],
                                                    "size": 3
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_writable_book_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_writable_book_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_written_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "count": [
                                {
                                    "test": {
                                        "components": {
                                            "predicates": {
                                                "minecraft:written_book_content": {
                                                    "author": "Alex",
                                                    "title": "Quest",
                                                    "generation": {
                                                        "min": 1,
                                                        "max": 2
                                                    },
                                                    "resolved": true,
                                                    "pages": {
                                                        "contains": [
                                                            "First page"
                                                        ],
                                                        "count": [
                                                            {
                                                                "test": "First page",
                                                                "count": 1
                                                            }
                                                        ],
                                                        "size": 2
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "count": {
                                        "min": 2
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_written_book_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_written_book_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_villager_variant.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:villager/variant",
                    "value": [
                        "minecraft:plains",
                        "minecraft:taiga"
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_villager_variant_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_villager_variant_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_villager_variant.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:villager/variant": "minecraft:plains"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_villager_variant_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_villager_variant_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_villager_variant.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:villager/variant": "minecraft:plains"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_villager_variant_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_villager_variant_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_villager_variant_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:villager/variant",
                    "value": "#minecraft:component_condition_villager_types",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_villager_variant_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_villager_variant_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_villager_variant_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:villager/variant": "#minecraft:component_condition_villager_types"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_villager_variant_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_villager_variant_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_villager_variant_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:villager/variant": "#minecraft:component_condition_villager_types"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_villager_variant_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_villager_variant_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "id": "minecraft:test/speed",
                                    "amount": {
                                        "min": 1.0,
                                        "max": 2.0
                                    },
                                    "operation": "add_value",
                                    "slot": "mainhand"
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "count": [
                                                        {
                                                            "test": {
                                                                "operation": "add_value"
                                                            },
                                                            "count": 1
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_attribute_modifiers_attribute.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "attribute": "minecraft:generic.attack_damage",
                                    "id": "minecraft:test/speed"
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_attribute_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_attribute_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_attribute_modifiers_attribute.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "contains": [
                                                        {
                                                            "attribute": "minecraft:generic.attack_damage",
                                                            "id": "minecraft:test/speed"
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_attribute_modifiers_attribute_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_attribute_modifiers_attribute_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "count": [
                                                        {
                                                            "test": {
                                                                "operation": "add_value"
                                                            },
                                                            "count": 1
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_attribute_modifiers_attribute.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "contains": [
                                                        {
                                                            "attribute": "minecraft:generic.attack_damage",
                                                            "id": "minecraft:test/speed"
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_attribute_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_attribute_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_attribute_modifiers_attribute_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "attribute": "#minecraft:component_condition_damage_attributes",
                                    "id": "minecraft:test/speed"
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_attribute_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_attribute_modifiers_attribute_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_attribute_modifiers_attribute_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "contains": [
                                                        {
                                                            "attribute": "#minecraft:component_condition_damage_attributes",
                                                            "id": "minecraft:test/speed"
                                                        }
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_attribute_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_attribute_modifiers_attribute_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_default_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "attribute": "minecraft:generic.attack_damage",
                                    "id": "minecraft:base_attack_damage",
                                    "amount": 5.0,
                                    "operation": "add_value",
                                    "slot": "mainhand"
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_default_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "contains": [
                                                        {
                                                            "attribute": "minecraft:generic.attack_damage",
                                                            "id": "minecraft:base_attack_damage",
                                                            "amount": 5.0,
                                                            "operation": "add_value",
                                                            "slot": "mainhand"
                                                        }
                                                    ],
                                                    "size": 2
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_default_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_default_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_default_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:attribute_modifiers": {
                                                "modifiers": {
                                                    "contains": [
                                                        {
                                                            "attribute": "minecraft:generic.attack_damage",
                                                            "id": "minecraft:base_attack_damage",
                                                            "amount": 5.0,
                                                            "operation": "add_value",
                                                            "slot": "mainhand"
                                                        }
                                                    ],
                                                    "size": 2
                                                }
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_default_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_default_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_default_armor_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "attribute": "minecraft:generic.armor",
                                    "id": "minecraft:armor.helmet",
                                    "amount": 3.0,
                                    "operation": "add_value",
                                    "slot": "head"
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_armor_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_armor_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_default_mace_attribute_modifiers.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:attribute_modifiers",
                    "value": {
                        "modifiers": {
                            "contains": [
                                {
                                    "attribute": "minecraft:generic.attack_speed",
                                    "id": "minecraft:base_attack_speed",
                                    "amount": {
                                        "min": -3.41,
                                        "max": -3.39
                                    },
                                    "operation": "add_value",
                                    "slot": "mainhand"
                                }
                            ],
                            "size": 2
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_mace_attribute_modifiers_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_default_mace_attribute_modifiers_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_custom_data.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:custom_data",
                    "value": {
                        "owner": "Alex",
                        "nested": {
                            "flag": true
                        },
                        "lore": ["one"]
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_custom_data_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_custom_data_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_custom_data_snbt.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:custom_data",
                    "value": "{owner:\"Alex\",level:7,nested:{flag:true},lore:[\"two\"]}",
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_custom_data_snbt_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_custom_data_snbt_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_custom_data.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:custom_data": {
                                                "owner": "Alex",
                                                "nested": {
                                                    "flag": true
                                                },
                                                "lore": ["two"]
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_custom_data_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_custom_data_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_bundle_partial_custom_data_snbt.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:bundle_contents",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:custom_data": "{owner:\"Alex\",level:7,nested:{flag:true},lore:[\"two\"]}"
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_custom_data_snbt_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_bundle_partial_custom_data_snbt_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_container_partial_custom_data.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:container",
                    "value": {
                        "items": {
                            "contains": [
                                {
                                    "components": {
                                        "predicates": {
                                            "minecraft:custom_data": {
                                                "owner": "Alex",
                                                "nested": {
                                                    "flag": true
                                                },
                                                "lore": ["two"]
                                            }
                                        }
                                    }
                                }
                            ]
                        }
                    },
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_custom_data_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_container_partial_custom_data_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_enchantments_level.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantments",
                    "value": [
                        {
                            "levels": {
                                "min": 2,
                                "max": 4
                            }
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_level_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_level_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_enchantments_empty.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantments",
                    "value": [],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_empty_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_empty_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_stored_enchantments_level.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:stored_enchantments",
                    "value": [
                        {
                            "levels": {
                                "min": 2,
                                "max": 4
                            }
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_level_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_level_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_stored_enchantments_empty.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:stored_enchantments",
                    "value": [],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_empty_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_empty_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_enchantments_holder.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantments",
                    "value": [
                        {
                            "enchantments": "minecraft:sharpness",
                            "levels": {
                                "min": 2
                            }
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_holder_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_holder_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_stored_enchantments_holder.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:stored_enchantments",
                    "value": [
                        {
                            "enchantments": [
                                "minecraft:mending",
                                "minecraft:power"
                            ],
                            "levels": 1
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_holder_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_holder_absent"
                    }
                }
            }"#,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_enchantments_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:enchantments",
                    "value": [
                        {
                            "enchantments": "#minecraft:component_condition_tagged",
                            "levels": {
                                "min": 2
                            }
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_enchantments_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets
            .join("items")
            .join("component_condition_stored_enchantments_tag.json"),
        r##"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:stored_enchantments",
                    "value": [
                        {
                            "enchantments": "#minecraft:component_condition_tagged",
                            "levels": 1
                        }
                    ],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_tag_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_tag_absent"
                    }
                }
            }"##,
    );
    write_json(
        &assets.join("items").join("enchanted_book.json"),
        r#"{
                "model": {
                    "type": "minecraft:condition",
                    "property": "minecraft:component",
                    "predicate": "minecraft:stored_enchantments",
                    "value": [],
                    "on_true": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_default_present"
                    },
                    "on_false": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/component_condition_stored_enchantments_default_absent"
                    }
                }
            }"#,
    );
    for (model_id, color) in [
        ("component_condition_rarity_present", [80, 160, 220, 255]),
        ("component_condition_rarity_absent", [60, 40, 80, 255]),
        ("component_condition_glint_present", [180, 80, 220, 255]),
        ("component_condition_glint_absent", [40, 40, 80, 255]),
        ("component_condition_damage_present", [220, 120, 40, 255]),
        ("component_condition_damage_absent", [40, 80, 60, 255]),
        (
            "component_condition_bundle_contents_present",
            [80, 120, 180, 255],
        ),
        (
            "component_condition_bundle_contents_absent",
            [30, 40, 80, 255],
        ),
        ("component_condition_trim_present", [120, 180, 80, 255]),
        ("component_condition_trim_absent", [40, 80, 30, 255]),
        (
            "component_condition_firework_explosion_present",
            [180, 120, 80, 255],
        ),
        (
            "component_condition_firework_explosion_absent",
            [80, 40, 30, 255],
        ),
        ("component_condition_fireworks_present", [180, 80, 120, 255]),
        ("component_condition_fireworks_absent", [80, 30, 40, 255]),
        (
            "component_condition_jukebox_playable_present",
            [120, 80, 180, 255],
        ),
        (
            "component_condition_jukebox_playable_absent",
            [40, 30, 80, 255],
        ),
        ("component_condition_container_present", [80, 180, 120, 255]),
        ("component_condition_container_absent", [30, 80, 40, 255]),
        (
            "component_condition_bundle_contents_constrained_present",
            [200, 200, 80, 255],
        ),
        (
            "component_condition_bundle_contents_constrained_absent",
            [50, 50, 30, 255],
        ),
        (
            "component_condition_bundle_contains_present",
            [210, 170, 70, 255],
        ),
        (
            "component_condition_bundle_contains_absent",
            [70, 50, 20, 255],
        ),
        (
            "component_condition_bundle_count_present",
            [80, 210, 150, 255],
        ),
        ("component_condition_bundle_count_absent", [30, 70, 50, 255]),
        (
            "component_condition_container_contains_present",
            [90, 190, 230, 255],
        ),
        (
            "component_condition_container_contains_absent",
            [30, 60, 80, 255],
        ),
        (
            "component_condition_container_count_present",
            [190, 110, 230, 255],
        ),
        (
            "component_condition_container_count_absent",
            [60, 30, 80, 255],
        ),
        (
            "component_condition_firework_explosion_star_present",
            [230, 180, 80, 255],
        ),
        (
            "component_condition_firework_explosion_star_absent",
            [70, 50, 30, 255],
        ),
        (
            "component_condition_fireworks_flight_present",
            [220, 100, 40, 255],
        ),
        (
            "component_condition_fireworks_flight_absent",
            [60, 30, 20, 255],
        ),
        (
            "component_condition_fireworks_explosions_present",
            [200, 220, 40, 255],
        ),
        (
            "component_condition_fireworks_explosions_absent",
            [50, 60, 20, 255],
        ),
        (
            "component_condition_fireworks_contains_present",
            [240, 160, 80, 255],
        ),
        (
            "component_condition_fireworks_contains_absent",
            [80, 50, 30, 255],
        ),
        (
            "component_condition_fireworks_count_present",
            [120, 220, 210, 255],
        ),
        (
            "component_condition_fireworks_count_absent",
            [40, 80, 80, 255],
        ),
        (
            "component_condition_trim_material_present",
            [160, 210, 240, 255],
        ),
        (
            "component_condition_trim_material_absent",
            [30, 50, 70, 255],
        ),
        (
            "component_condition_trim_pattern_present",
            [160, 240, 190, 255],
        ),
        ("component_condition_trim_pattern_absent", [30, 70, 50, 255]),
        (
            "component_condition_trim_material_tag_present",
            [210, 190, 240, 255],
        ),
        (
            "component_condition_trim_material_tag_absent",
            [60, 50, 80, 255],
        ),
        (
            "component_condition_trim_pattern_tag_present",
            [190, 240, 220, 255],
        ),
        (
            "component_condition_trim_pattern_tag_absent",
            [50, 80, 70, 255],
        ),
        (
            "component_condition_enchantments_level_present",
            [240, 190, 80, 255],
        ),
        (
            "component_condition_enchantments_level_absent",
            [70, 50, 20, 255],
        ),
        (
            "component_condition_enchantments_empty_present",
            [190, 160, 240, 255],
        ),
        (
            "component_condition_enchantments_empty_absent",
            [50, 40, 70, 255],
        ),
        (
            "component_condition_stored_enchantments_level_present",
            [240, 220, 100, 255],
        ),
        (
            "component_condition_stored_enchantments_level_absent",
            [70, 60, 30, 255],
        ),
        (
            "component_condition_stored_enchantments_empty_present",
            [210, 180, 240, 255],
        ),
        (
            "component_condition_stored_enchantments_empty_absent",
            [60, 50, 80, 255],
        ),
        (
            "component_condition_enchantments_holder_present",
            [250, 210, 90, 255],
        ),
        (
            "component_condition_enchantments_holder_absent",
            [80, 60, 20, 255],
        ),
        (
            "component_condition_stored_enchantments_holder_present",
            [230, 230, 120, 255],
        ),
        (
            "component_condition_stored_enchantments_holder_absent",
            [80, 70, 30, 255],
        ),
        (
            "component_condition_stored_enchantments_default_present",
            [250, 240, 150, 255],
        ),
        (
            "component_condition_stored_enchantments_default_absent",
            [90, 80, 40, 255],
        ),
        (
            "component_condition_bundle_tag_contains_present",
            [210, 230, 110, 255],
        ),
        (
            "component_condition_bundle_tag_contains_absent",
            [70, 80, 30, 255],
        ),
        (
            "component_condition_container_tag_count_present",
            [110, 230, 210, 255],
        ),
        (
            "component_condition_container_tag_count_absent",
            [30, 80, 70, 255],
        ),
        (
            "component_condition_enchantments_tag_present",
            [240, 210, 120, 255],
        ),
        (
            "component_condition_enchantments_tag_absent",
            [80, 70, 30, 255],
        ),
        (
            "component_condition_stored_enchantments_tag_present",
            [230, 220, 140, 255],
        ),
        (
            "component_condition_stored_enchantments_tag_absent",
            [70, 70, 40, 255],
        ),
        (
            "component_condition_bundle_components_present",
            [190, 230, 120, 255],
        ),
        (
            "component_condition_bundle_components_absent",
            [60, 80, 30, 255],
        ),
        (
            "component_condition_bundle_exact_component_text_present",
            [180, 230, 175, 255],
        ),
        (
            "component_condition_bundle_exact_component_text_absent",
            [55, 85, 50, 255],
        ),
        (
            "component_condition_bundle_exact_lore_present",
            [175, 215, 230, 255],
        ),
        (
            "component_condition_bundle_exact_lore_absent",
            [50, 75, 85, 255],
        ),
        (
            "component_condition_bundle_exact_unbreakable_present",
            [230, 210, 175, 255],
        ),
        (
            "component_condition_bundle_exact_unbreakable_absent",
            [85, 70, 50, 255],
        ),
        (
            "component_condition_bundle_exact_custom_data_present",
            [210, 235, 180, 255],
        ),
        (
            "component_condition_bundle_exact_custom_data_absent",
            [70, 90, 55, 255],
        ),
        (
            "component_condition_bundle_exact_potion_contents_present",
            [235, 185, 215, 255],
        ),
        (
            "component_condition_bundle_exact_potion_contents_absent",
            [85, 45, 70, 255],
        ),
        (
            "component_condition_bundle_exact_writable_book_present",
            [190, 205, 245, 255],
        ),
        (
            "component_condition_bundle_exact_writable_book_absent",
            [55, 65, 100, 255],
        ),
        (
            "component_condition_bundle_exact_firework_explosion_present",
            [245, 215, 120, 255],
        ),
        (
            "component_condition_bundle_exact_firework_explosion_absent",
            [100, 75, 30, 255],
        ),
        (
            "component_condition_bundle_exact_fireworks_present",
            [245, 160, 105, 255],
        ),
        (
            "component_condition_bundle_exact_fireworks_absent",
            [105, 55, 35, 255],
        ),
        (
            "component_condition_bundle_exact_jukebox_playable_present",
            [185, 235, 235, 255],
        ),
        (
            "component_condition_bundle_exact_jukebox_playable_absent",
            [45, 95, 95, 255],
        ),
        (
            "component_condition_bundle_exact_trim_present",
            [205, 185, 245, 255],
        ),
        (
            "component_condition_bundle_exact_trim_absent",
            [75, 55, 105, 255],
        ),
        (
            "component_condition_bundle_exact_enchantments_present",
            [250, 225, 135, 255],
        ),
        (
            "component_condition_bundle_exact_enchantments_absent",
            [95, 75, 35, 255],
        ),
        (
            "component_condition_bundle_exact_stored_enchantments_present",
            [225, 245, 155, 255],
        ),
        (
            "component_condition_bundle_exact_stored_enchantments_absent",
            [70, 90, 45, 255],
        ),
        (
            "component_condition_bundle_exact_villager_variant_present",
            [205, 245, 170, 255],
        ),
        (
            "component_condition_bundle_exact_villager_variant_absent",
            [65, 95, 45, 255],
        ),
        (
            "component_condition_bundle_exact_lodestone_tracker_present",
            [165, 220, 245, 255],
        ),
        (
            "component_condition_bundle_exact_lodestone_tracker_absent",
            [45, 80, 100, 255],
        ),
        (
            "component_condition_bundle_exact_attribute_modifiers_present",
            [235, 165, 95, 255],
        ),
        (
            "component_condition_bundle_exact_attribute_modifiers_absent",
            [105, 55, 35, 255],
        ),
        (
            "component_condition_bundle_exact_written_book_present",
            [225, 205, 120, 255],
        ),
        (
            "component_condition_bundle_exact_written_book_absent",
            [95, 75, 40, 255],
        ),
        (
            "component_condition_container_components_present",
            [120, 230, 190, 255],
        ),
        (
            "component_condition_container_components_absent",
            [30, 80, 60, 255],
        ),
        (
            "component_condition_bundle_partial_damage_present",
            [230, 160, 190, 255],
        ),
        (
            "component_condition_bundle_partial_damage_absent",
            [80, 40, 60, 255],
        ),
        (
            "component_condition_container_partial_damage_present",
            [160, 190, 230, 255],
        ),
        (
            "component_condition_container_partial_damage_absent",
            [40, 60, 80, 255],
        ),
        (
            "component_condition_bundle_partial_any_value_present",
            [230, 210, 150, 255],
        ),
        (
            "component_condition_bundle_partial_any_value_absent",
            [80, 70, 40, 255],
        ),
        (
            "component_condition_container_partial_any_value_present",
            [150, 230, 210, 255],
        ),
        (
            "component_condition_container_partial_any_value_absent",
            [40, 80, 70, 255],
        ),
        (
            "component_condition_bundle_partial_enchantments_present",
            [245, 180, 120, 255],
        ),
        (
            "component_condition_bundle_partial_enchantments_absent",
            [90, 50, 30, 255],
        ),
        (
            "component_condition_container_partial_stored_enchantments_present",
            [180, 245, 120, 255],
        ),
        (
            "component_condition_container_partial_stored_enchantments_absent",
            [50, 90, 30, 255],
        ),
        (
            "component_condition_bundle_partial_firework_explosion_present",
            [245, 150, 90, 255],
        ),
        (
            "component_condition_bundle_partial_firework_explosion_absent",
            [90, 40, 25, 255],
        ),
        (
            "component_condition_container_partial_fireworks_present",
            [150, 245, 90, 255],
        ),
        (
            "component_condition_container_partial_fireworks_absent",
            [40, 90, 25, 255],
        ),
        (
            "component_condition_bundle_partial_trim_present",
            [210, 170, 245, 255],
        ),
        (
            "component_condition_bundle_partial_trim_absent",
            [70, 45, 90, 255],
        ),
        (
            "component_condition_container_partial_trim_present",
            [170, 210, 245, 255],
        ),
        (
            "component_condition_container_partial_trim_absent",
            [45, 70, 90, 255],
        ),
        (
            "component_condition_jukebox_playable_song_present",
            [210, 245, 170, 255],
        ),
        (
            "component_condition_jukebox_playable_song_absent",
            [70, 90, 45, 255],
        ),
        (
            "component_condition_bundle_partial_jukebox_playable_present",
            [245, 210, 170, 255],
        ),
        (
            "component_condition_bundle_partial_jukebox_playable_absent",
            [90, 70, 45, 255],
        ),
        (
            "component_condition_container_partial_jukebox_playable_present",
            [170, 245, 210, 255],
        ),
        (
            "component_condition_container_partial_jukebox_playable_absent",
            [45, 90, 70, 255],
        ),
        (
            "component_condition_potion_contents_present",
            [245, 170, 210, 255],
        ),
        (
            "component_condition_potion_contents_absent",
            [90, 45, 70, 255],
        ),
        (
            "component_condition_bundle_partial_potion_contents_present",
            [245, 170, 245, 255],
        ),
        (
            "component_condition_bundle_partial_potion_contents_absent",
            [90, 45, 90, 255],
        ),
        (
            "component_condition_container_partial_potion_contents_present",
            [170, 245, 245, 255],
        ),
        (
            "component_condition_container_partial_potion_contents_absent",
            [45, 90, 90, 255],
        ),
        (
            "component_condition_writable_book_pages_present",
            [210, 245, 245, 255],
        ),
        (
            "component_condition_writable_book_pages_absent",
            [70, 90, 90, 255],
        ),
        (
            "component_condition_written_book_content_present",
            [245, 210, 245, 255],
        ),
        (
            "component_condition_written_book_content_absent",
            [90, 70, 90, 255],
        ),
        (
            "component_condition_written_book_component_page_present",
            [230, 205, 170, 255],
        ),
        (
            "component_condition_written_book_component_page_absent",
            [105, 80, 65, 255],
        ),
        (
            "component_condition_bundle_partial_writable_book_present",
            [210, 245, 210, 255],
        ),
        (
            "component_condition_bundle_partial_writable_book_absent",
            [70, 90, 70, 255],
        ),
        (
            "component_condition_container_partial_written_book_present",
            [245, 210, 210, 255],
        ),
        (
            "component_condition_container_partial_written_book_absent",
            [90, 70, 70, 255],
        ),
        (
            "component_condition_villager_variant_present",
            [190, 245, 210, 255],
        ),
        (
            "component_condition_villager_variant_absent",
            [60, 90, 70, 255],
        ),
        (
            "component_condition_bundle_partial_villager_variant_present",
            [245, 190, 210, 255],
        ),
        (
            "component_condition_bundle_partial_villager_variant_absent",
            [90, 60, 70, 255],
        ),
        (
            "component_condition_container_partial_villager_variant_present",
            [210, 245, 190, 255],
        ),
        (
            "component_condition_container_partial_villager_variant_absent",
            [70, 90, 60, 255],
        ),
        (
            "component_condition_villager_variant_tag_present",
            [180, 245, 220, 255],
        ),
        (
            "component_condition_villager_variant_tag_absent",
            [55, 90, 75, 255],
        ),
        (
            "component_condition_bundle_partial_villager_variant_tag_present",
            [245, 180, 220, 255],
        ),
        (
            "component_condition_bundle_partial_villager_variant_tag_absent",
            [90, 55, 75, 255],
        ),
        (
            "component_condition_container_partial_villager_variant_tag_present",
            [220, 245, 180, 255],
        ),
        (
            "component_condition_container_partial_villager_variant_tag_absent",
            [75, 90, 55, 255],
        ),
        (
            "component_condition_attribute_modifiers_present",
            [180, 220, 245, 255],
        ),
        (
            "component_condition_attribute_modifiers_absent",
            [50, 70, 90, 255],
        ),
        (
            "component_condition_container_partial_attribute_modifiers_present",
            [220, 180, 245, 255],
        ),
        (
            "component_condition_container_partial_attribute_modifiers_absent",
            [70, 50, 90, 255],
        ),
        (
            "component_condition_attribute_modifiers_attribute_present",
            [120, 220, 245, 255],
        ),
        (
            "component_condition_attribute_modifiers_attribute_absent",
            [35, 70, 95, 255],
        ),
        (
            "component_condition_container_partial_attribute_modifiers_attribute_present",
            [230, 150, 245, 255],
        ),
        (
            "component_condition_container_partial_attribute_modifiers_attribute_absent",
            [90, 35, 95, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_present",
            [200, 180, 245, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_absent",
            [60, 50, 90, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_attribute_present",
            [210, 150, 245, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_attribute_absent",
            [80, 35, 95, 255],
        ),
        (
            "component_condition_attribute_modifiers_attribute_tag_present",
            [140, 220, 245, 255],
        ),
        (
            "component_condition_attribute_modifiers_attribute_tag_absent",
            [45, 70, 95, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_attribute_tag_present",
            [220, 170, 245, 255],
        ),
        (
            "component_condition_bundle_partial_attribute_modifiers_attribute_tag_absent",
            [85, 45, 95, 255],
        ),
        (
            "component_condition_default_attribute_modifiers_present",
            [245, 230, 150, 255],
        ),
        (
            "component_condition_default_attribute_modifiers_absent",
            [95, 85, 45, 255],
        ),
        (
            "component_condition_bundle_partial_default_attribute_modifiers_present",
            [245, 210, 150, 255],
        ),
        (
            "component_condition_bundle_partial_default_attribute_modifiers_absent",
            [95, 75, 45, 255],
        ),
        (
            "component_condition_container_partial_default_attribute_modifiers_present",
            [245, 190, 150, 255],
        ),
        (
            "component_condition_container_partial_default_attribute_modifiers_absent",
            [95, 65, 45, 255],
        ),
        (
            "component_condition_default_armor_attribute_modifiers_present",
            [180, 230, 245, 255],
        ),
        (
            "component_condition_default_armor_attribute_modifiers_absent",
            [55, 85, 95, 255],
        ),
        (
            "component_condition_default_mace_attribute_modifiers_present",
            [220, 230, 180, 255],
        ),
        (
            "component_condition_default_mace_attribute_modifiers_absent",
            [85, 90, 55, 255],
        ),
        (
            "component_condition_custom_data_present",
            [245, 210, 120, 255],
        ),
        ("component_condition_custom_data_absent", [95, 70, 35, 255]),
        (
            "component_condition_custom_data_snbt_present",
            [210, 245, 160, 255],
        ),
        (
            "component_condition_custom_data_snbt_absent",
            [70, 95, 45, 255],
        ),
        (
            "component_condition_bundle_partial_custom_data_present",
            [245, 190, 120, 255],
        ),
        (
            "component_condition_bundle_partial_custom_data_absent",
            [95, 55, 35, 255],
        ),
        (
            "component_condition_bundle_partial_custom_data_snbt_present",
            [190, 235, 155, 255],
        ),
        (
            "component_condition_bundle_partial_custom_data_snbt_absent",
            [65, 85, 40, 255],
        ),
        (
            "component_condition_container_partial_custom_data_present",
            [245, 170, 120, 255],
        ),
        (
            "component_condition_container_partial_custom_data_absent",
            [95, 45, 35, 255],
        ),
    ] {
        write_flat_item_model_and_texture(&assets, model_id, &color);
    }
}

fn write_item_model_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "model_component");
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

fn write_display_context_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "display_selector");
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

fn write_trim_material_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "iron_chestplate");
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

fn write_charge_type_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item 0 = crossbow, item 1 = firework_rocket, item 2 = arrow.
    write_item_registry_source(root, &["crossbow", "firework_rocket", "arrow"]);
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

fn write_use_tick_range_dispatch_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item ids: 0 = bow, 1 = crossbow, 2 = firework_rocket, 3 = arrow,
    // 4 = brush, 5 = apple, 6 = ender_eye.
    write_item_registry_source(
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

fn write_main_hand_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "hand_selector");
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

fn write_stack_string_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    // Item 0 = beehive, item 1 = cmd_selector.
    write_item_registry_source(root, &["beehive", "cmd_selector"]);
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

fn write_custom_model_data_condition_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_source(root, "cmd_flag_condition");
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

fn write_local_time_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(
        root,
        &[
            "seasonal_chest",
            "precise_clock",
            "tokyo_clock",
            "utc_offset_clock",
            "date_field_clock",
            "quarter_clock",
            "localized_gmt_clock",
            "weekday_in_month_clock",
            "week_clock",
            "local_weekday_clock",
            "zone_name_clock",
            "fixed_zone_long_clock",
            "offset_width_clock",
            "narrow_month_clock",
            "ampm_width_clock",
            "millis_day_clock",
            "year_end_week_clock",
            "week_year_clock",
            "us_week_clock",
            "gb_week_clock",
            "julian_day_clock",
        ],
    );
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
    write_json(
        &assets.join("items").join("precise_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd'T'HH:mm:ss.SSS EEEE a",
                    "time_zone": "UTC+02:30",
                    "cases": [
                        {
                            "when": "2026-12-25T00:08:07.123 Friday AM",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/precise_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/precise_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("tokyo_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd'T'HH:mm:ss.SSS XXX Z x",
                    "time_zone": "Asia/Tokyo",
                    "cases": [
                        {
                            "when": "2026-12-25T00:08:07.123 +09:00 +0900 +09",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/tokyo_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/tokyo_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("utc_offset_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd'T'HH:mm:ss X xx XXXXX xxxxx ZZZZZ",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-12-25T00:08:07 Z +0000 Z +00:00 Z",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/utc_offset_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/utc_offset_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("date_field_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "uuuu-DDD-G",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-359-AD",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/date_field_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/date_field_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("quarter_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-Q-QQ-QQQ-QQQQ-QQQQQ-q-qq-qqq-qqqq-qqqqq",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-3-03-Q3-3rd quarter-3-3-03-Q3-3rd quarter-3",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/quarter_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/quarter_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("localized_gmt_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd O OO OOO OOOO",
                    "time_zone": "UTC+02:30",
                    "cases": [
                        {
                            "when": "2026-01-01 GMT+2:30 GMT+2:30 GMT+2:30 GMT+02:30",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/localized_gmt_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/localized_gmt_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("weekday_in_month_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd F FF",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-08-09 2 02",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/weekday_in_month_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/weekday_in_month_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("week_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd w ww W WW",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-01-04 1 01 1 01",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/week_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/week_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("local_weekday_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd e ee eee eeee eeeee eeeeee c cc ccc cccc ccccc cccccc E EEEE EEEEE EEEEEE",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-01-04 7 07 Sun Sunday S Su 7 07 Sun Sunday S Su Sun Sunday S Su",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/local_weekday_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/local_weekday_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("zone_name_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd z zzz VV VVV",
                    "time_zone": "Asia/Tokyo",
                    "cases": [
                        {
                            "when": "2026-01-01 JST JST Asia/Tokyo Tokyo",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/zone_name_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/zone_name_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("fixed_zone_long_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd z zzzz",
                    "time_zone": "UTC+02:30",
                    "cases": [
                        {
                            "when": "2026-01-01 +0230 GMT+02:30",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/fixed_zone_long_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/fixed_zone_long_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("offset_width_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd Z ZZZZ ZZZZZ X XX XXX XXXX XXXXX x xx xxx xxxx xxxxx",
                    "time_zone": "UTC+02:30",
                    "cases": [
                        {
                            "when": "2026-01-01 +0230 GMT+02:30 +02:30 +0230 +0230 +02:30 +0230 +02:30 +0230 +0230 +02:30 +0230 +02:30",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/offset_width_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/offset_width_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("narrow_month_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-M-MM-MMM-MMMM-MMMMM-L-LL-LLL-LLLL-LLLLL",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-8-08-Aug-August-A-8-08-Aug-August-A",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/narrow_month_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/narrow_month_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("ampm_width_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd a aaa aaaa aaaaa",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-01-01 PM PM PM p",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/ampm_width_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/ampm_width_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("millis_day_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "HH:mm:ss.SSS A AA AAAAAAAA",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "01:02:03.004 3723004 3723004 03723004",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/millis_day_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/millis_day_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("year_end_week_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd w ww",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-12-31 53 53",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/year_end_week_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/year_end_week_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("week_year_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy YY YYY YYYY YYYYY w ww",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026 26 2026 2026 02026 53 53",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/week_year_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/week_year_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("us_week_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd Y w ww W e c E",
                    "locale": "en_US",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-01-04 2026 2 02 2 1 1 Sun",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/us_week_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/us_week_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("gb_week_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd Y w ww W e c E",
                    "locale": "en_GB",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2027-01-01 2026 53 53 5 5 5 Fri",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/gb_week_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/gb_week_clock_fallback" }
                }
            }"#,
    );
    write_json(
        &assets.join("items").join("julian_day_clock.json"),
        r#"{
                "model": {
                    "type": "minecraft:select",
                    "property": "minecraft:local_time",
                    "pattern": "yyyy-MM-dd g gggggggg",
                    "time_zone": "UTC",
                    "cases": [
                        {
                            "when": "2026-12-25 2461400 02461400",
                            "model": { "type": "minecraft:model", "model": "minecraft:item/julian_day_clock_match" }
                        }
                    ],
                    "fallback": { "type": "minecraft:model", "model": "minecraft:item/julian_day_clock_fallback" }
                }
            }"#,
    );
    write_flat_item_model_and_texture(&assets, "seasonal_chest_normal", &[80, 60, 40, 255]);
    write_flat_item_model_and_texture(&assets, "seasonal_chest_christmas", &[180, 30, 30, 255]);
    write_flat_item_model_and_texture(&assets, "precise_clock_match", &[40, 120, 180, 255]);
    write_flat_item_model_and_texture(&assets, "precise_clock_fallback", &[40, 40, 40, 255]);
    write_flat_item_model_and_texture(&assets, "tokyo_clock_match", &[180, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "tokyo_clock_fallback", &[40, 40, 80, 255]);
    write_flat_item_model_and_texture(&assets, "utc_offset_clock_match", &[120, 180, 40, 255]);
    write_flat_item_model_and_texture(&assets, "utc_offset_clock_fallback", &[30, 30, 80, 255]);
    write_flat_item_model_and_texture(&assets, "date_field_clock_match", &[200, 160, 40, 255]);
    write_flat_item_model_and_texture(&assets, "date_field_clock_fallback", &[40, 20, 60, 255]);
    write_flat_item_model_and_texture(&assets, "quarter_clock_match", &[40, 180, 160, 255]);
    write_flat_item_model_and_texture(&assets, "quarter_clock_fallback", &[60, 30, 80, 255]);
    write_flat_item_model_and_texture(&assets, "localized_gmt_clock_match", &[180, 180, 60, 255]);
    write_flat_item_model_and_texture(&assets, "localized_gmt_clock_fallback", &[40, 50, 90, 255]);
    write_flat_item_model_and_texture(
        &assets,
        "weekday_in_month_clock_match",
        &[120, 180, 220, 255],
    );
    write_flat_item_model_and_texture(
        &assets,
        "weekday_in_month_clock_fallback",
        &[50, 30, 70, 255],
    );
    write_flat_item_model_and_texture(&assets, "week_clock_match", &[70, 160, 210, 255]);
    write_flat_item_model_and_texture(&assets, "week_clock_fallback", &[45, 35, 85, 255]);
    write_flat_item_model_and_texture(&assets, "local_weekday_clock_match", &[80, 190, 140, 255]);
    write_flat_item_model_and_texture(&assets, "local_weekday_clock_fallback", &[35, 55, 90, 255]);
    write_flat_item_model_and_texture(&assets, "zone_name_clock_match", &[150, 185, 230, 255]);
    write_flat_item_model_and_texture(&assets, "zone_name_clock_fallback", &[30, 60, 100, 255]);
    write_flat_item_model_and_texture(
        &assets,
        "fixed_zone_long_clock_match",
        &[130, 175, 215, 255],
    );
    write_flat_item_model_and_texture(
        &assets,
        "fixed_zone_long_clock_fallback",
        &[35, 65, 95, 255],
    );
    write_flat_item_model_and_texture(&assets, "offset_width_clock_match", &[145, 190, 220, 255]);
    write_flat_item_model_and_texture(&assets, "offset_width_clock_fallback", &[40, 70, 100, 255]);
    write_flat_item_model_and_texture(&assets, "narrow_month_clock_match", &[155, 195, 225, 255]);
    write_flat_item_model_and_texture(&assets, "narrow_month_clock_fallback", &[45, 75, 105, 255]);
    write_flat_item_model_and_texture(&assets, "ampm_width_clock_match", &[165, 200, 230, 255]);
    write_flat_item_model_and_texture(&assets, "ampm_width_clock_fallback", &[50, 80, 110, 255]);
    write_flat_item_model_and_texture(&assets, "millis_day_clock_match", &[175, 205, 235, 255]);
    write_flat_item_model_and_texture(&assets, "millis_day_clock_fallback", &[55, 85, 115, 255]);
    write_flat_item_model_and_texture(&assets, "year_end_week_clock_match", &[185, 210, 240, 255]);
    write_flat_item_model_and_texture(&assets, "year_end_week_clock_fallback", &[60, 90, 120, 255]);
    write_flat_item_model_and_texture(&assets, "week_year_clock_match", &[195, 215, 245, 255]);
    write_flat_item_model_and_texture(&assets, "week_year_clock_fallback", &[65, 95, 125, 255]);
    write_flat_item_model_and_texture(&assets, "us_week_clock_match", &[205, 220, 245, 255]);
    write_flat_item_model_and_texture(&assets, "us_week_clock_fallback", &[70, 100, 130, 255]);
    write_flat_item_model_and_texture(&assets, "gb_week_clock_match", &[210, 225, 245, 255]);
    write_flat_item_model_and_texture(&assets, "gb_week_clock_fallback", &[75, 105, 135, 255]);
    write_flat_item_model_and_texture(&assets, "julian_day_clock_match", &[215, 230, 245, 255]);
    write_flat_item_model_and_texture(&assets, "julian_day_clock_fallback", &[80, 110, 140, 255]);
}

fn write_component_select_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item RARITY_SELECTOR = registerItem("rarity_selector");
                public static final Item STACK_SIZE_SELECTOR = registerItem("stack_size_selector");
                public static final Item GLINT_SELECTOR = registerItem("glint_selector");
                public static final Item DAMAGE_COMPONENT_SELECTOR = registerItem(
                    "damage_component_selector",
                    Item::new,
                    new Item.Properties().durability(432)
                );
                public static final Item MAX_DAMAGE_COMPONENT_SELECTOR = registerItem(
                    "max_damage_component_selector",
                    Item::new,
                    new Item.Properties().durability(432)
                );
                public static final Item ITEM_MODEL_COMPONENT_SELECTOR = registerItem("item_model_component_selector");
                public static final Item MAP_ID_COMPONENT_SELECTOR = registerItem("map_id_component_selector");
                public static final Item DYED_COLOR_COMPONENT_SELECTOR = registerItem("dyed_color_component_selector");
                public static final Item MAP_COLOR_COMPONENT_SELECTOR = registerItem("map_color_component_selector");
                public static final Item CUSTOM_NAME_COMPONENT_SELECTOR = registerItem("custom_name_component_selector");
                public static final Item ITEM_NAME_COMPONENT_SELECTOR = registerItem("item_name_component_selector");
                public static final Item BLOCK_ITEM_NAME_COMPONENT_SELECTOR = registerBlock(Blocks.BLOCK_ITEM_NAME_COMPONENT_SELECTOR);
            }"#,
    );
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
    ] {
        write_flat_item_model_and_texture(&assets, model_id, &color);
    }
}

fn write_value_aware_range_dispatch_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(
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

fn write_bundle_selected_item_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_source(root, &["bundle", "apple"]);
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
    write_flat_item_model_and_texture(&assets, "bundle", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_open_back", &[120, 80, 40, 255]);
    write_flat_item_model_and_texture(&assets, "bundle_open_front", &[80, 120, 40, 255]);
    write_flat_item_model_and_texture(&assets, "apple", &[200, 40, 40, 255]);
}

fn write_single_item_registry_source(root: &Path, item_id: &str) {
    write_item_registry_source(root, &[item_id]);
}

fn write_item_registry_source(root: &Path, item_ids: &[&str]) {
    let declarations = item_ids
        .iter()
        .map(|item_id| {
            let constant = item_id.to_ascii_uppercase();
            format!("public static final Item {constant} = registerItem(\"{item_id}\");")
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        &format!(
            r#"public class Items {{
                {declarations}
            }}"#,
        ),
    );
}

fn write_flat_item_model_and_texture(assets: &Path, model_id: &str, color: &[u8]) {
    write_json(
        &assets
            .join("models")
            .join("item")
            .join(format!("{model_id}.json")),
        &format!(
            r#"{{
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

fn write_item_registry_sources(root: &Path) {
    write_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
                public static final Item TEST_COMBO = registerItem("test_combo", new Item.Properties().equippable(EquipmentSlot.CHEST));
            }"#,
    );
}

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

struct TestSkinPngFetcher {
    bytes: Vec<u8>,
    calls: Arc<AtomicUsize>,
}

impl SkinPngFetcher for TestSkinPngFetcher {
    fn fetch_skin_png(&mut self, _url: &str) -> Result<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.bytes.clone())
    }
}

fn player_skin_png_bytes() -> Vec<u8> {
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

fn player_profile_texture_png_bytes() -> Vec<u8> {
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

fn drain_until_player_skin_download_result(
    runtime: &NativeItemRuntime,
) -> Vec<NativeDynamicPlayerSkinDownload> {
    for _ in 0..100 {
        let downloads = runtime.drain_dynamic_player_skin_download_results();
        if !downloads.is_empty() {
            return downloads;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for player skin download result");
}

fn drain_until_player_texture_download_results(
    runtime: &NativeItemRuntime,
    expected_len: usize,
) -> Vec<NativeDynamicPlayerTextureDownload> {
    let mut downloads = Vec::new();
    for _ in 0..100 {
        downloads.extend(runtime.drain_dynamic_player_texture_download_results());
        if downloads.len() >= expected_len {
            return downloads;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for player profile texture download results");
}

fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8).unwrap();
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}-{id}"))
}
