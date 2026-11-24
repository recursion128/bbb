use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_client_assets::{ItemRegistryCatalog, LanguageCatalog, PackRoots};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::{VanillaTrimPatternIdentity, WorldStore};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID,
    },
    builtin_enum_component_values::{
        VANILLA_DYE_COLOR_BLACK_ID, VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_GRAY_ID,
        VANILLA_DYE_COLOR_ORANGE_ID, VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_WHITE_ID,
    },
    mob_effect_types::{
        VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID,
    },
    packets::{
        AttributeModifierSummary, FireworkExplosionSummary, ItemStackSummary,
        ItemStackTemplateSummary, MobEffectInstanceSummary, NbtSummaryEntry, SoundEventSummary,
    },
    tropical_fish_patterns::{
        VANILLA_TROPICAL_FISH_PATTERN_KOB_PACKED_ID,
        VANILLA_TROPICAL_FISH_PATTERN_STRIPEY_PACKED_ID,
    },
};

use super::*;

mod batch_02;
mod batch_03;

pub(super) trait TooltipRuntimeTestExt {
    fn tooltip_lines_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<Vec<NativeItemTooltipLine>>;
    fn tooltip_lines_for_stack_with_options(
        &self,
        stack: &ItemStackSummary,
        advanced: bool,
    ) -> Option<Vec<NativeItemTooltipLine>>;
    fn tooltip_lines_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        options: NativeItemTooltipOptions<'_>,
    ) -> Option<Vec<NativeItemTooltipLine>>;
    fn styled_hover_name(&self, stack: &ItemStackSummary) -> Option<NativeItemHoverName>;
}

impl TooltipRuntimeTestExt for NativeItemRuntime {
    fn tooltip_lines_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        tooltip_lines_for_stack_with_context(self, stack, NativeItemTooltipOptions::default())
    }

    fn tooltip_lines_for_stack_with_options(
        &self,
        stack: &ItemStackSummary,
        advanced: bool,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        tooltip_lines_for_stack_with_context(
            self,
            stack,
            NativeItemTooltipOptions {
                advanced,
                ..NativeItemTooltipOptions::default()
            },
        )
    }

    fn tooltip_lines_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        options: NativeItemTooltipOptions<'_>,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        tooltip_lines_for_stack_with_context(self, stack, options)
    }

    fn styled_hover_name(&self, stack: &ItemStackSummary) -> Option<NativeItemHoverName> {
        styled_hover_name(self, stack)
    }
}

fn world_with_stack_component_defaults(runtime: &NativeItemRuntime) -> WorldStore {
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
    let mut world = WorldStore::new();
    world.set_default_item_component_codec_values(defaults);
    world.rebuild_item_component_defaults().unwrap();
    world
}

fn bee_occupants(count: usize) -> Vec<bbb_protocol::packets::BeeOccupantSummary> {
    vec![
        bbb_protocol::packets::BeeOccupantSummary {
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
            entity_data_tag: None,
            ticks_in_hive: 0,
            min_ticks_in_hive: 0,
        };
        count
    ]
}

fn container_from_items(
    items: Vec<ItemStackTemplateSummary>,
    slot_count: Option<usize>,
) -> bbb_protocol::packets::data_components::ContainerContentsSummary {
    let slots = items
        .into_iter()
        .enumerate()
        .map(|(slot, item)| bbb_protocol::packets::ItemContainerSlotSummary { slot, item })
        .collect::<Vec<_>>();
    bbb_protocol::packets::data_components::ContainerContentsSummary {
        slot_count: slot_count.or(Some(slots.len())),
        slots,
    }
}

static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

fn tooltip_line(text: &str, tint: [f32; 4]) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun::plain(text).into()],
    }
}

fn colored_tooltip_line(text: &str, tint: [f32; 4], color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle::default(),
            color: Some(color),
        }
        .into()],
    }
}

fn enchantment_tooltip_line(description: &str, level: &str) -> NativeItemTooltipLine {
    let run = |text: &str| HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: HudTextStyle::default(),
        color: Some(0xAA_AA_AA),
    };
    NativeItemTooltipLine {
        text: format!("{description} {level}"),
        tint: TOOLTIP_TEXT_GRAY,
        runs: vec![run(description).into(), run(" ").into(), run(level).into()],
    }
}

fn trim_tooltip_line(text: &str, description_run: HudStyledTextRun) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: format!(" {text}"),
        tint: TOOLTIP_TEXT_WHITE,
        runs: vec![HudStyledTextRun::plain(" ").into(), description_run.into()],
    }
}

fn colored_trim_tooltip_line(text: &str, color: u32) -> NativeItemTooltipLine {
    trim_tooltip_line(
        text,
        HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle::default(),
            color: Some(color),
        },
    )
}

fn charged_projectile_detail_line(text: &str, child_color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: format!("  {text}"),
        tint: TOOLTIP_TEXT_GRAY,
        runs: vec![
            HudStyledTextRun {
                shadow_color: None,
                text: "  ".to_string(),
                style: HudTextStyle::default(),
                color: Some(0xAA_AA_AA),
            }
            .into(),
            HudStyledTextRun {
                shadow_color: None,
                text: text.to_string(),
                style: HudTextStyle::default(),
                color: Some(child_color),
            }
            .into(),
        ],
    }
}

fn charged_projectile_header_line(
    text: &str,
    runs: Vec<HudStyledTextRun>,
) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEXT_WHITE,
        runs: runs.into_iter().map(HudComponentTextRun::from).collect(),
    }
}

fn container_item_count_line(
    text: &str,
    name_runs: Vec<HudStyledTextRun>,
    count: i32,
) -> NativeItemTooltipLine {
    let mut runs = name_runs
        .into_iter()
        .map(HudComponentTextRun::from)
        .collect::<Vec<_>>();
    runs.push(HudComponentTextRun::plain(" x"));
    runs.push(HudComponentTextRun::plain(count.to_string()));
    NativeItemTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEXT_WHITE,
        runs,
    }
}

fn display_name_bracket_run(text: &str) -> HudStyledTextRun {
    HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: HudTextStyle::default(),
        color: Some(0xFF_FF_FF),
    }
}

fn italic_tooltip_line(text: &str, tint: [f32; 4], color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(color),
        }
        .into()],
    }
}

fn italic_plain_tooltip_line(text: &str, tint: [f32; 4]) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: None,
        }
        .into()],
    }
}

/// A hover-name line: vanilla `getStyledHoverName` wraps the name in the
/// rarity colour and italicizes custom names.
fn name_line(text: &str, tint: [f32; 4], color: u32, italic: bool) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle {
                italic,
                ..HudTextStyle::default()
            },
            color: Some(color),
        }
        .into()],
    }
}

/// A lore line carrying vanilla `ItemLore.LORE_STYLE` (DARK_PURPLE + italic).
fn lore_line(text: &str) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEXT_DARK_PURPLE,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(0xAA_00_AA),
        }
        .into()],
    }
}

fn operator_warning_line(text: &str, bold: bool) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.to_string(),
        tint: TOOLTIP_TEXT_RED,
        runs: vec![HudStyledTextRun {
            shadow_color: None,
            text: text.to_string(),
            style: HudTextStyle {
                bold,
                ..HudTextStyle::default()
            },
            color: Some(0xFF_55_55),
        }
        .into()],
    }
}

fn assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
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

fn write_item_registry_fixture(root: &Path, item_ids: &[&str]) {
    write_item_registry_override(root, item_ids);
}

fn write_item_registry_fixture_defaults(root: &Path) {
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

fn item_registry_override_path(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join(".bbb")
        .join("item_registry_catalog.json")
}

fn write_item_registry_override(root: &Path, item_ids: &[&str]) {
    write_item_registry_override_value(root, &item_registry_override_value(item_ids));
}

fn write_item_registry_override_with_profiles(
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

fn write_item_registry_override_value(root: &Path, catalog: &serde_json::Value) {
    write_json(
        &item_registry_override_path(root),
        &serde_json::to_string_pretty(catalog).unwrap(),
    );
}

fn item_registry_override_value(item_ids: &[&str]) -> serde_json::Value {
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

fn copy_item_registry_profile(
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

fn write_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
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
    std::env::temp_dir().join(format!("bbb-client-{label}-{nanos}-{id}"))
}

const fn potion_effect(effect_id: i32, duration: i32, amplifier: i32) -> VanillaPotionEffect {
    VanillaPotionEffect {
        effect_id,
        duration,
        amplifier,
    }
}

#[test]
fn rich_component_item_tooltip_retains_font_and_all_text_styles() {
    let runs = hud_runs_from_component(
        &[StyledTextRun {
            text: "Styled".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                italic: Some(false),
                underlined: Some(true),
                strikethrough: Some(true),
                obfuscated: Some(true),
                color: Some(0x12_34_56),
                font: Some("example:tooltip".to_string()),
                ..ComponentStyle::default()
            },
        }],
        "fallback",
        &ComponentStyle {
            italic: Some(true),
            color: Some(0xAA_00_AA),
            ..ComponentStyle::default()
        },
    );

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].visual.text, "Styled");
    assert_eq!(runs[0].font.as_deref(), Some("example:tooltip"));
    assert_eq!(runs[0].visual.color, Some(0x12_34_56));
    assert_eq!(
        runs[0].visual.style,
        HudTextStyle {
            bold: true,
            italic: false,
            underlined: true,
            strikethrough: true,
            obfuscated: true,
        }
    );
}

#[test]
fn dyed_color_advanced_tooltip_keeps_java_int_hex_width() {
    assert_eq!(dyed_color_tooltip_hex(0), "#000000");
    assert_eq!(dyed_color_tooltip_hex(0x12_3a_bc), "#123ABC");
    assert_eq!(dyed_color_tooltip_hex(-1), "#FFFFFFFF");
    assert_eq!(dyed_color_tooltip_hex(i32::MIN), "#80000000");
}

#[test]
fn vanilla_instrument_keys_follow_26_1_goat_horn_registry_order() {
    assert_eq!(
        (0..8)
            .filter_map(VanillaInstrumentIdentity::for_vanilla_registry_id)
            .map(VanillaInstrumentIdentity::resource_id)
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            "minecraft:ponder_goat_horn",
            "minecraft:sing_goat_horn",
            "minecraft:seek_goat_horn",
            "minecraft:feel_goat_horn",
            "minecraft:admire_goat_horn",
            "minecraft:call_goat_horn",
            "minecraft:yearn_goat_horn",
            "minecraft:dream_goat_horn",
        ]
    );
    assert_eq!(
        VanillaInstrumentIdentity::for_vanilla_registry_id(0)
            .map(VanillaInstrumentIdentity::resource_id),
        Some("minecraft:ponder_goat_horn")
    );
    assert_eq!(
        VanillaInstrumentIdentity::for_vanilla_registry_id(7)
            .map(VanillaInstrumentIdentity::resource_id),
        Some("minecraft:dream_goat_horn")
    );
    assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(8), None);
    assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(-1), None);
}

#[test]
fn tropical_fish_tables_follow_26_1_pattern_and_common_variant_order() {
    assert_eq!(
        bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERNS.len(),
        12
    );
    assert_eq!(tropical_fish_pattern(257), (257, "stripey"));
    assert_eq!(tropical_fish_pattern(9999), (0, "kob"));
    assert_eq!(tropical_fish_common_variant_index(257, 1, 7), Some(0));
    assert_eq!(tropical_fish_common_variant_index(1, 4, 4), Some(21));
    assert_eq!(tropical_fish_common_variant_index(0, 0, 14), None);
    assert_eq!(dye_color_id_or_white(Some(15)), 15);
    assert_eq!(dye_color_id_or_white(Some(16)), 0);
}

#[test]
fn vanilla_banner_pattern_translation_keys_follow_26_1_bootstrap_order() {
    assert_eq!(
        (0..43)
            .filter_map(BannerPatternKind::for_vanilla_registry_id)
            .map(BannerPatternKind::translation_key)
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            "block.minecraft.banner.base",
            "block.minecraft.banner.square_bottom_left",
            "block.minecraft.banner.square_bottom_right",
            "block.minecraft.banner.square_top_left",
            "block.minecraft.banner.square_top_right",
            "block.minecraft.banner.stripe_bottom",
            "block.minecraft.banner.stripe_top",
            "block.minecraft.banner.stripe_left",
            "block.minecraft.banner.stripe_right",
            "block.minecraft.banner.stripe_center",
            "block.minecraft.banner.stripe_middle",
            "block.minecraft.banner.stripe_downright",
            "block.minecraft.banner.stripe_downleft",
            "block.minecraft.banner.small_stripes",
            "block.minecraft.banner.cross",
            "block.minecraft.banner.straight_cross",
            "block.minecraft.banner.triangle_bottom",
            "block.minecraft.banner.triangle_top",
            "block.minecraft.banner.triangles_bottom",
            "block.minecraft.banner.triangles_top",
            "block.minecraft.banner.diagonal_left",
            "block.minecraft.banner.diagonal_up_right",
            "block.minecraft.banner.diagonal_up_left",
            "block.minecraft.banner.diagonal_right",
            "block.minecraft.banner.circle",
            "block.minecraft.banner.rhombus",
            "block.minecraft.banner.half_vertical",
            "block.minecraft.banner.half_horizontal",
            "block.minecraft.banner.half_vertical_right",
            "block.minecraft.banner.half_horizontal_bottom",
            "block.minecraft.banner.border",
            "block.minecraft.banner.gradient",
            "block.minecraft.banner.gradient_up",
            "block.minecraft.banner.bricks",
            "block.minecraft.banner.curly_border",
            "block.minecraft.banner.globe",
            "block.minecraft.banner.creeper",
            "block.minecraft.banner.skull",
            "block.minecraft.banner.flower",
            "block.minecraft.banner.mojang",
            "block.minecraft.banner.piglin",
            "block.minecraft.banner.flow",
            "block.minecraft.banner.guster",
        ]
    );
    assert_eq!(
        BannerPatternKind::for_vanilla_registry_id(5).map(BannerPatternKind::translation_key),
        Some("block.minecraft.banner.stripe_bottom")
    );
    assert_eq!(
        BannerPatternKind::for_vanilla_registry_id(42).map(BannerPatternKind::translation_key),
        Some("block.minecraft.banner.guster")
    );
    assert_eq!(
        BannerPatternKind::for_vanilla_registry_id(43).map(BannerPatternKind::translation_key),
        None
    );
    assert_eq!(
        BannerPatternKind::for_vanilla_registry_id(-1).map(BannerPatternKind::translation_key),
        None
    );
    assert_eq!(
        BannerPatternKind::for_vanilla_registry_id(5).map(BannerPatternKind::resource_id),
        Some("minecraft:stripe_bottom")
    );
    assert_eq!(BannerPatternKind::for_vanilla_registry_id(43), None);
}

#[test]
fn vanilla_painting_variants_follow_26_1_bootstrap_order() {
    assert_eq!(VANILLA_PAINTING_VARIANTS.len(), 51);
    assert_eq!(
        vanilla_painting_variant(0).copied(),
        Some(painting_variant(0, "minecraft:kebab", 1, 1, true))
    );
    assert_eq!(
        vanilla_painting_variant(10).copied(),
        Some(painting_variant(10, "minecraft:sunset", 2, 1, true))
    );
    assert_eq!(
        vanilla_painting_variant(19).copied(),
        Some(painting_variant(19, "minecraft:wither", 2, 2, false))
    );
    assert_eq!(
        vanilla_painting_variant(50).copied(),
        Some(painting_variant(50, "minecraft:dennis", 3, 3, true))
    );
    assert_eq!(vanilla_painting_variant(51), None);
    assert_eq!(vanilla_painting_variant(-1), None);
    assert_eq!(
        painting_variant_translation_key("minecraft:skull_and_roses", "author"),
        Some("painting.minecraft.skull_and_roses.author".to_string())
    );
    assert_eq!(
        painting_variant_translation_key("custom:gallery/frame", "title"),
        Some("painting.custom.gallery.frame.title".to_string())
    );
}

#[test]
fn vanilla_trim_catalog_follows_26_1_bootstrap_order() {
    assert_eq!(
        (0..11)
            .filter_map(VanillaTrimMaterialIdentity::for_vanilla_registry_id)
            .map(VanillaTrimMaterialIdentity::resource_id)
            .collect::<Vec<_>>(),
        &[
            "minecraft:quartz",
            "minecraft:iron",
            "minecraft:netherite",
            "minecraft:redstone",
            "minecraft:copper",
            "minecraft:gold",
            "minecraft:emerald",
            "minecraft:diamond",
            "minecraft:lapis",
            "minecraft:amethyst",
            "minecraft:resin",
        ]
    );
    assert_eq!(
        (0..11)
            .filter_map(VanillaTrimMaterialIdentity::for_vanilla_registry_id)
            .map(VanillaTrimMaterialIdentity::description_color)
            .collect::<Vec<_>>(),
        &[
            0xE3_D4_C4, 0xEC_EC_EC, 0x62_58_59, 0x97_16_07, 0xB4_68_4D, 0xDE_B1_2D, 0x11_A0_36,
            0x6E_EC_D2, 0x41_6E_97, 0x9A_5C_C6, 0xFC_78_12,
        ]
    );
    assert_eq!(
        (0..18)
            .filter_map(VanillaTrimPatternIdentity::for_vanilla_registry_id)
            .map(VanillaTrimPatternIdentity::resource_id)
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            "minecraft:sentry",
            "minecraft:dune",
            "minecraft:coast",
            "minecraft:wild",
            "minecraft:ward",
            "minecraft:eye",
            "minecraft:vex",
            "minecraft:tide",
            "minecraft:snout",
            "minecraft:rib",
            "minecraft:spire",
            "minecraft:wayfinder",
            "minecraft:shaper",
            "minecraft:silence",
            "minecraft:raiser",
            "minecraft:host",
            "minecraft:flow",
            "minecraft:bolt",
        ]
    );
    assert_eq!(
        VanillaTrimMaterialIdentity::for_vanilla_registry_id(1)
            .map(VanillaTrimMaterialIdentity::resource_id),
        Some("minecraft:iron")
    );
    assert_eq!(
        VanillaTrimMaterialIdentity::for_vanilla_registry_id(10)
            .map(VanillaTrimMaterialIdentity::resource_id),
        Some("minecraft:resin")
    );
    assert_eq!(
        VanillaTrimMaterialIdentity::for_vanilla_registry_id(11),
        None
    );
    assert_eq!(
        VanillaTrimMaterialIdentity::for_vanilla_registry_id(-1),
        None
    );
    assert_eq!(
        VanillaTrimMaterialIdentity::for_vanilla_registry_id(1)
            .map(VanillaTrimMaterialIdentity::description_color),
        Some(0xEC_EC_EC)
    );
    assert_eq!(
        VanillaTrimPatternIdentity::for_vanilla_registry_id(0)
            .map(VanillaTrimPatternIdentity::resource_id),
        Some("minecraft:sentry")
    );
    assert_eq!(
        VanillaTrimPatternIdentity::for_vanilla_registry_id(17)
            .map(VanillaTrimPatternIdentity::resource_id),
        Some("minecraft:bolt")
    );
    assert_eq!(
        VanillaTrimPatternIdentity::for_vanilla_registry_id(18),
        None
    );
    assert_eq!(
        VanillaTrimPatternIdentity::for_vanilla_registry_id(-1),
        None
    );
}

#[test]
fn vanilla_potion_effects_follow_26_1_bootstrap_order() {
    assert_eq!(
        vanilla_potion(0).map(|potion| potion.effects),
        Some(&[][..])
    );
    assert_eq!(
        vanilla_potion(4).map(|potion| potion.effects),
        Some(&[potion_effect(15, 3600, 0)][..])
    );
    assert_eq!(
        vanilla_potion(19).map(|potion| potion.effects),
        Some(&[potion_effect(1, 400, 3), potion_effect(10, 400, 2)][..])
    );
    assert_eq!(
        vanilla_potion(24).map(|potion| potion.effects),
        Some(&[potion_effect(5, 1, 0)][..])
    );
    assert_eq!(
        vanilla_potion(45).map(|potion| potion.effects),
        Some(
            &[potion_effect(
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INFESTED_ID,
                3600,
                0
            )][..]
        )
    );
    assert_eq!(vanilla_potion(46).map(|potion| potion.effects), None);
    assert_eq!(vanilla_potion(-1).map(|potion| potion.effects), None);
    assert_eq!(format_attribute_modifier_amount(40.0), "40");
    assert_eq!(format_attribute_modifier_amount(1.5), "1.5");
    assert_eq!(format_attribute_modifier_amount(1.234), "1.23");
    assert_eq!(
        effective_tooltip_tick_rate(NativeItemTooltipOptions {
            tooltip_tick_rate: Some(10.0),
            ..NativeItemTooltipOptions::default()
        }),
        10.0
    );
    assert_eq!(
        effective_tooltip_tick_rate(NativeItemTooltipOptions {
            tooltip_tick_rate: Some(0.0),
            ..NativeItemTooltipOptions::default()
        }),
        DEFAULT_TOOLTIP_TICKRATE
    );
}

#[test]
fn vanilla_enchantment_tables_follow_26_1_registry_and_tooltip_order() {
    assert_eq!(
        (0..43)
            .map(|registry_id| {
                let enchantment =
                    VanillaEnchantmentIdentity::for_vanilla_registry_id(registry_id).unwrap();
                (enchantment.resource_id(), enchantment.max_level())
            })
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            ("minecraft:protection", 4),
            ("minecraft:fire_protection", 4),
            ("minecraft:feather_falling", 4),
            ("minecraft:blast_protection", 4),
            ("minecraft:projectile_protection", 4),
            ("minecraft:respiration", 3),
            ("minecraft:aqua_affinity", 1),
            ("minecraft:thorns", 3),
            ("minecraft:depth_strider", 3),
            ("minecraft:frost_walker", 2),
            ("minecraft:binding_curse", 1),
            ("minecraft:soul_speed", 3),
            ("minecraft:swift_sneak", 3),
            ("minecraft:sharpness", 5),
            ("minecraft:smite", 5),
            ("minecraft:bane_of_arthropods", 5),
            ("minecraft:knockback", 2),
            ("minecraft:fire_aspect", 2),
            ("minecraft:looting", 3),
            ("minecraft:sweeping_edge", 3),
            ("minecraft:efficiency", 5),
            ("minecraft:silk_touch", 1),
            ("minecraft:unbreaking", 3),
            ("minecraft:fortune", 3),
            ("minecraft:power", 5),
            ("minecraft:punch", 2),
            ("minecraft:flame", 1),
            ("minecraft:infinity", 1),
            ("minecraft:luck_of_the_sea", 3),
            ("minecraft:lure", 3),
            ("minecraft:loyalty", 3),
            ("minecraft:impaling", 5),
            ("minecraft:riptide", 3),
            ("minecraft:lunge", 3),
            ("minecraft:channeling", 1),
            ("minecraft:multishot", 1),
            ("minecraft:quick_charge", 3),
            ("minecraft:piercing", 4),
            ("minecraft:density", 5),
            ("minecraft:breach", 4),
            ("minecraft:wind_burst", 3),
            ("minecraft:mending", 1),
            ("minecraft:vanishing_curse", 1),
        ]
    );
    assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.len(), 43);
    // Without an enchantment registry the holder id is the loader-order
    // index (loads the registry from data files),
    // so `minecraft:sharpness` answers to 33, not to its bootstrap index 13.
    assert_eq!(
        enchantment_key_for_holder(33, None),
        Some("minecraft:sharpness")
    );
    assert_eq!(
        enchantment_key_for_holder(13, None),
        Some("minecraft:fortune")
    );
    assert_eq!(
        VanillaEnchantmentIdentity::for_vanilla_registry_id(42)
            .map(VanillaEnchantmentIdentity::resource_id),
        Some("minecraft:vanishing_curse")
    );
    assert_eq!(
        VanillaEnchantmentIdentity::for_vanilla_registry_id(43),
        None
    );
    assert_eq!(
        VanillaEnchantmentIdentity::for_vanilla_registry_id(-1),
        None
    );
    assert_eq!(
        VanillaEnchantmentIdentity::for_resource_id("minecraft:sharpness")
            .map(VanillaEnchantmentIdentity::max_level),
        Some(5)
    );
    assert!(enchantment_is_curse("minecraft:binding_curse", None));
    assert!(!enchantment_is_curse("minecraft:sharpness", None));
}

#[test]
fn synced_enchantment_definition_controls_description_style_and_level_suffix() {
    let language = LanguageCatalog::from_json_bytes(
        br#"{
            "enchantment.example.custom": "Localized Custom",
            "\ud800": "Raw enchant",
            "enchantment.level.1": "I"
        }"#,
    )
    .unwrap();
    let enchantments = [ItemEnchantmentSummary {
        holder_id: 0,
        level: 1,
    }];
    let keys = ["example:custom".to_string()];
    let mut definitions = [Some(EnchantmentTooltipDefinition {
        description: "enchantment.example.custom".to_string(),
        description_styled: vec![StyledTextRun {
            text: "enchantment.example.custom".to_string(),
            style: ComponentStyle {
                color: Some(0x12_34_56),
                ..ComponentStyle::default()
            },
        }],
        description_nbt: Some(NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::String("enchantment.example.custom".into()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::String("#123456".into()),
            },
        ])),
        description_translation_key: Some("enchantment.example.custom".to_string()),
        description_translation_fallback: None,
        max_level: 3,
    })];
    let mut lines = Vec::new();

    push_enchantments_tooltip_lines(
        &language,
        None,
        &enchantments,
        Some(&keys),
        Some(&definitions),
        None,
        None,
        &mut lines,
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Localized Custom I");
    assert_eq!(lines[0].tint, TOOLTIP_TEXT_GRAY);
    assert!(lines[0]
        .runs
        .iter()
        .all(|run| run.visual.color == Some(0x12_34_56)));

    definitions[0].as_mut().unwrap().max_level = 1;
    lines.clear();
    push_enchantments_tooltip_lines(
        &language,
        None,
        &enchantments,
        Some(&keys),
        Some(&definitions),
        None,
        None,
        &mut lines,
    );
    assert_eq!(lines[0].text, "Localized Custom");
    definitions[0].as_mut().unwrap().description_nbt = Some(NbtSummaryValue::Compound(vec![
        bbb_protocol::NbtSummaryEntry {
            name: "translate".into(),
            value: NbtSummaryValue::string_from_utf16(vec![0xd800]),
        },
    ]));
    lines.clear();
    push_enchantments_tooltip_lines(
        &language,
        None,
        &enchantments,
        Some(&keys),
        Some(&definitions),
        None,
        None,
        &mut lines,
    );
    assert_eq!(lines[0].text, "Raw enchant");
}

#[test]
fn synchronized_jukebox_tooltip_uses_typed_description_and_safe_stub_fallback() {
    let language = LanguageCatalog::from_json_bytes(
        br#"{
            "jukebox_song.example.custom": "Custom song",
            "jukebox_song.minecraft.cat": "C418 - cat"
        }"#,
    )
    .unwrap();
    let keys = [
        "example:custom".to_string(),
        "minecraft:cat".to_string(),
        String::new(),
    ];
    let presentations = [
        Some(JukeboxSongPresentation {
            sound_event: bbb_client_world::RegistrySoundEvent::Reference {
                resource_id: "example:music.custom".to_string(),
            },
            description: "jukebox_song.example.custom".to_string(),
            description_styled: vec![StyledTextRun {
                text: "jukebox_song.example.custom".to_string(),
                style: ComponentStyle {
                    bold: Some(true),
                    ..ComponentStyle::default()
                },
            }],
            description_nbt: NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "translate".into(),
                    value: NbtSummaryValue::string("jukebox_song.example.custom"),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "bold".into(),
                    value: NbtSummaryValue::Byte(1),
                },
            ]),
            length_in_seconds: 20.0,
            comparator_output: 3,
        }),
        None,
        None,
    ];
    let registry = JukeboxSongRegistry::default();

    let mut lines = Vec::new();
    push_jukebox_playable_tooltip_lines(
        &language,
        None,
        &registry,
        Some(0),
        None,
        Some(&keys),
        Some(&presentations),
        &mut lines,
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Custom song");
    assert!(lines[0].runs[0].visual.style.bold);

    lines.clear();
    push_jukebox_playable_tooltip_lines(
        &language,
        None,
        &registry,
        Some(1),
        None,
        Some(&keys),
        Some(&presentations),
        &mut lines,
    );
    assert_eq!(lines[0].text, "C418 - cat");

    lines.clear();
    push_jukebox_playable_tooltip_lines(
        &language,
        None,
        &registry,
        Some(2),
        None,
        Some(&keys),
        Some(&presentations),
        &mut lines,
    );
    assert!(lines.is_empty());
}

#[test]
fn charged_projectile_tooltip_bounds_deep_programmatic_templates() {
    const STACK_BUDGET_BYTES: usize = 4 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let runtime = NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
            let mut projectile = ItemStackTemplateSummary {
                item_id: 1,
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            };
            for _ in 1..MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 10 {
                projectile = ItemStackTemplateSummary {
                    item_id: 1,
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        charged_projectiles_items: vec![projectile],
                        ..DataComponentPatchSummary::default()
                    }
                    .into(),
                };
            }
            let stack = ItemStackSummary {
                item_id: Some(1),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    charged_projectiles_items: vec![projectile],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            };

            TOOLTIP_STACK_EXTREMES.with(|cell| cell.set((0, usize::MAX)));
            let lines = runtime.tooltip_lines_for_stack(&stack).unwrap();
            let (top, bottom) = TOOLTIP_STACK_EXTREMES.with(std::cell::Cell::get);
            let used = top - bottom;
            assert_eq!(lines.len(), MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 2);
            assert!(
                used <= STACK_BUDGET_BYTES,
                "depth-{MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH} charged-projectile tooltip used \
                 {used} bytes of stack, over the {STACK_BUDGET_BYTES}-byte budget"
            );
        })
        .expect("spawn deep charged-projectile tooltip test thread")
        .join()
        .expect("deep charged-projectile tooltip test thread");
}

#[test]
fn tooltip_component_shadow_inherits_base_and_keeps_zero_override() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::String("A".into())),
        entry("shadow_color", NbtSummaryValue::Int(0x8012_3456_u32 as i32)),
        entry(
            "extra",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![
                    entry("text", NbtSummaryValue::String("B".into())),
                    entry("shadow_color", NbtSummaryValue::Int(0)),
                ]),
                NbtSummaryValue::String("C".into()),
            ]),
        ),
    ]))
    .unwrap();
    let base = ComponentStyle {
        shadow_color: Some(0xffab_cdef),
        ..Default::default()
    };
    let runs = hud_runs_from_component(component.styled(), "", &base);
    assert_eq!(
        runs.iter()
            .map(|run| run.visual.shadow_color)
            .collect::<Vec<_>>(),
        [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
    );
    assert_eq!(
        hud_runs_from_component(&[], "fallback", &base)[0]
            .visual
            .shadow_color,
        base.shadow_color
    );
    let language = LanguageCatalog::from_json_bytes(br#"{"shadow.wrapper":"[%s]"}"#).unwrap();
    let translated = translated_tooltip_line_with_args(
        &language,
        "shadow.wrapper",
        &[TooltipTextArg {
            text: "ABC".into(),
            runs,
        }],
        TOOLTIP_TEXT_WHITE,
    );
    assert_eq!(translated.text, "[ABC]");
    assert_eq!(
        translated
            .runs
            .iter()
            .map(|run| run.visual.shadow_color)
            .collect::<Vec<_>>(),
        [None, Some(0x8012_3456), Some(0), Some(0x8012_3456), None]
    );
}

#[test]
fn potion_tooltip_uses_prototype_explicit_and_removed_duration_scales() {
    let runtime = NativeItemRuntime::empty_for_test()
        .with_vanilla_item_registry_for_test()
        .with_language(
            LanguageCatalog::from_json_bytes(
                br#"{
            "item.minecraft.tipped_arrow": "Tipped Arrow",
            "effect.minecraft.night_vision": "Night Vision",
            "potion.withDuration": "%s (%s)"
        }"#,
            )
            .unwrap(),
        );
    let tipped_arrow_id = runtime.item_protocol_id("minecraft:tipped_arrow").unwrap();
    let defaults = runtime.default_component_codec_values_by_protocol_id();
    let scale_id = component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID;
    assert_eq!(
        defaults
            .values()
            .filter(|values| values.contains_key(&scale_id))
            .count(),
        2
    );
    assert_eq!(
        defaults[&tipped_arrow_id][&scale_id],
        serde_json::json!(0.125)
    );
    let mut world = WorldStore::new();
    world.set_default_item_component_codec_values(BTreeMap::from([(
        tipped_arrow_id,
        BTreeMap::from([(scale_id, defaults[&tipped_arrow_id][&scale_id].clone())]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    let options = NativeItemTooltipOptions {
        item_profiles: Some(world.item_profiles()),
        ..NativeItemTooltipOptions::default()
    };

    let stack = |component_patch| ItemStackSummary {
        item_id: Some(tipped_arrow_id),
        count: 1,
        component_patch,
    };
    let potion = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ],
        potion_contents: bbb_protocol::packets::PotionContentsSummary {
            potion_id: Some(4),
            ..Default::default()
        },
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&stack(potion.clone().into()), options),
        Some(vec![
            name_line(
                "item.minecraft.tipped_arrow.effect.night_vision",
                TOOLTIP_TEXT_WHITE,
                0xFF_FF_FF,
                false,
            ),
            tooltip_line("Night Vision (00:22)", TOOLTIP_TEXT_BLUE),
        ])
    );

    let explicit = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        ],
        potion_duration_scale_bits: Some(0.5_f32.to_bits()),
        ..potion.clone()
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&stack(explicit.into()), options),
        Some(vec![
            name_line(
                "item.minecraft.tipped_arrow.effect.night_vision",
                TOOLTIP_TEXT_WHITE,
                0xFF_FF_FF,
                false,
            ),
            tooltip_line("Night Vision (01:30)", TOOLTIP_TEXT_BLUE),
        ])
    );

    let removed = DataComponentPatchSummary {
        removed_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        ],
        ..potion
    };
    assert_eq!(
        runtime.tooltip_lines_for_stack_with_context(&stack(removed.into()), options),
        Some(vec![
            name_line(
                "item.minecraft.tipped_arrow.effect.night_vision",
                TOOLTIP_TEXT_WHITE,
                0xFF_FF_FF,
                false,
            ),
            tooltip_line("Night Vision (03:00)", TOOLTIP_TEXT_BLUE),
        ])
    );
}

#[test]
fn potion_tooltip_checks_the_unscaled_duration_before_formatting_the_scaled_value() {
    let language = LanguageCatalog::from_json_bytes(
        br#"{
            "effect.minecraft.speed": "Speed",
            "potion.withDuration": "%s (%s)"
        }"#,
    )
    .unwrap();

    assert_eq!(
        potion_effect_tooltip_text(&language, "minecraft:speed", 0, 20, 2.0, 20.0),
        "Speed"
    );
    assert_eq!(
        potion_effect_tooltip_text(&language, "minecraft:speed", 0, 21, 0.125, 20.0),
        "Speed (00:00)"
    );
}

#[test]
fn shield_hover_name_uses_base_color_before_item_name() {
    let runtime = NativeItemRuntime::empty_for_test()
        .with_vanilla_item_registry_for_test()
        .with_language(
            LanguageCatalog::from_json_bytes(
                br#"{
            "item.minecraft.shield": "Shield",
            "item.minecraft.shield.red": "Red Shield",
            "item.minecraft.stone": "Stone"
        }"#,
            )
            .unwrap(),
        );
    let shield_id = runtime.item_protocol_id("minecraft:shield").unwrap();
    let stone_id = runtime.item_protocol_id("minecraft:stone").unwrap();
    let stack = |item_id, component_patch| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch,
    };

    assert_eq!(
        runtime.tooltip_lines_for_stack(&stack(
            shield_id,
            DataComponentPatchSummary {
                base_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                item_name: Some("Ignored Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        )),
        Some(vec![name_line(
            "Red Shield",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&stack(
            shield_id,
            DataComponentPatchSummary {
                custom_name: Some("Custom Shield".to_string()),
                base_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                item_name: Some("Ignored Item Name".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        )),
        Some(vec![name_line(
            "Custom Shield",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            true
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&stack(
            shield_id,
            DataComponentPatchSummary {
                item_name: Some("Patched Shield".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        )),
        Some(vec![name_line(
            "Patched Shield",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
    assert_eq!(
        runtime.tooltip_lines_for_stack(&stack(
            stone_id,
            DataComponentPatchSummary {
                base_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        )),
        Some(vec![name_line(
            "Stone",
            TOOLTIP_TEXT_WHITE,
            0xFF_FF_FF,
            false
        )])
    );
}

#[test]
fn item_inspector_appends_food_fuel_and_namespace_lines_only_when_enabled() {
    let runtime = NativeItemRuntime::empty_for_test()
        .with_vanilla_item_registry_for_test()
        .with_language(
            LanguageCatalog::from_json_bytes(
                br#"{
            "item.minecraft.apple": "Apple",
            "item.minecraft.coal": "Coal"
        }"#,
            )
            .unwrap(),
        );
    let apple_id = runtime.item_protocol_id("minecraft:apple").unwrap();
    let coal_id = runtime.item_protocol_id("minecraft:coal").unwrap();

    let apple_stack = ItemStackSummary {
        item_id: Some(apple_id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            food_nutrition: Some(4),
            food_saturation_bits: Some(2.4_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let baseline_apple = runtime.tooltip_lines_for_stack(&apple_stack).unwrap();
    assert_eq!(baseline_apple.len(), 1);

    let inspected_apple = runtime
        .tooltip_lines_for_stack_with_context(
            &apple_stack,
            NativeItemTooltipOptions {
                item_inspector: true,
                ..NativeItemTooltipOptions::default()
            },
        )
        .unwrap();
    let apple_texts: Vec<&str> = inspected_apple.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        apple_texts,
        vec!["Apple", "Food: +4 (2.4 sat)", "@minecraft"]
    );

    let coal_stack = ItemStackSummary {
        item_id: Some(coal_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let inspected_coal = runtime
        .tooltip_lines_for_stack_with_context(
            &coal_stack,
            NativeItemTooltipOptions {
                item_inspector: true,
                furnace_burn_ticks: Some(1600),
                mod_display_name: Some("bbb_demo"),
                ..NativeItemTooltipOptions::default()
            },
        )
        .unwrap();
    let coal_texts: Vec<&str> = inspected_coal.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        coal_texts,
        vec!["Coal", "Fuel: 1600t (8.0 items)", "@bbb_demo"]
    );
}
