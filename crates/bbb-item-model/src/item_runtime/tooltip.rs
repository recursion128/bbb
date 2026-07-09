use bbb_protocol::{
    packets::{MobEffectInstanceSummary, SuspiciousStewEffectSummary},
    ComponentStyle, StyledTextRun,
};

use super::mob_effects::{
    vanilla_mob_effect_category, vanilla_mob_effect_key, VanillaMobEffectCategory,
};
use super::*;

/// Vanilla `ItemLore.LORE_STYLE`
/// (`net.minecraft.world.item.component.ItemLore`):
/// `Style.EMPTY.withColor(ChatFormatting.DARK_PURPLE).withItalic(true)`. The
/// canonical `ItemLore` constructor merges it into every lore line via
/// `ComponentUtils.mergeStyles`, i.e. keys the line set itself win and unset
/// keys are filled from this default.
const LORE_STYLE: ComponentStyle = ComponentStyle {
    bold: None,
    italic: Some(true),
    underlined: None,
    strikethrough: None,
    obfuscated: None,
    color: Some(0xAA_00_AA),
    click_event: None,
};
const OMINOUS_BOTTLE_BAD_OMEN_DURATION_TICKS: i32 = 120_000;
const DEFAULT_TOOLTIP_TICKRATE: f32 = 20.0;
const TOOLTIP_GRAY_TEXT_COLOR: u32 = 0xAA_AA_AA;
const COMPONENT_DAMAGE_TYPE_ID: i32 = 3;
const COMPONENT_UNBREAKABLE_TYPE_ID: i32 = 4;
const COMPONENT_LORE_TYPE_ID: i32 = 11;
const COMPONENT_ENCHANTMENTS_TYPE_ID: i32 = 13;
const COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID: i32 = 22;
const COMPONENT_MAP_ID_TYPE_ID: i32 = 46;
const COMPONENT_STORED_ENCHANTMENTS_TYPE_ID: i32 = 42;
const COMPONENT_DYED_COLOR_TYPE_ID: i32 = 44;
const COMPONENT_CHARGED_PROJECTILES_TYPE_ID: i32 = 49;
const COMPONENT_POTION_CONTENTS_TYPE_ID: i32 = 51;
const COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID: i32 = 53;
const COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID: i32 = 55;
const COMPONENT_TRIM_TYPE_ID: i32 = 56;
const COMPONENT_INSTRUMENT_TYPE_ID: i32 = 61;
const COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID: i32 = 63;
const COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID: i32 = 64;
const COMPONENT_FIREWORK_EXPLOSION_TYPE_ID: i32 = 68;
const COMPONENT_FIREWORKS_TYPE_ID: i32 = 69;
const COMPONENT_PROFILE_TYPE_ID: i32 = 70;
const COMPONENT_BANNER_PATTERNS_TYPE_ID: i32 = 72;
const COMPONENT_POT_DECORATIONS_TYPE_ID: i32 = 74;
const COMPONENT_CONTAINER_TYPE_ID: i32 = 75;
const COMPONENT_BLOCK_STATE_TYPE_ID: i32 = 76;
const COMPONENT_BEES_TYPE_ID: i32 = 77;
const COMPONENT_CONTAINER_LOOT_TYPE_ID: i32 = 79;
const COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID: i32 = 88;
const INSTRUMENT_DESCRIPTION_STYLE: ComponentStyle = ComponentStyle {
    bold: None,
    italic: None,
    underlined: None,
    strikethrough: None,
    obfuscated: None,
    color: Some(TOOLTIP_GRAY_TEXT_COLOR),
    click_event: None,
};
const TROPICAL_FISH_PATTERNS: &[(i32, &str)] = &[
    (0, "kob"),
    (256, "sunstreak"),
    (512, "snooper"),
    (768, "dasher"),
    (1024, "brinely"),
    (1280, "spotty"),
    (1, "flopper"),
    (257, "stripey"),
    (513, "glitter"),
    (769, "blockfish"),
    (1025, "betty"),
    (1281, "clayfish"),
];
const TROPICAL_FISH_COMMON_VARIANTS: &[(i32, i32, i32)] = &[
    (257, 1, 7),
    (1, 7, 7),
    (1, 7, 11),
    (1281, 0, 7),
    (256, 11, 7),
    (0, 1, 0),
    (1280, 6, 3),
    (769, 10, 4),
    (1281, 0, 14),
    (1280, 0, 4),
    (513, 0, 7),
    (1281, 0, 1),
    (768, 9, 6),
    (1024, 5, 3),
    (1025, 14, 0),
    (512, 7, 14),
    (769, 14, 0),
    (1, 0, 4),
    (0, 14, 0),
    (256, 7, 0),
    (768, 9, 4),
    (1, 4, 4),
];
const VANILLA_INSTRUMENT_KEYS: &[&str] = &[
    "minecraft:ponder_goat_horn",
    "minecraft:sing_goat_horn",
    "minecraft:seek_goat_horn",
    "minecraft:feel_goat_horn",
    "minecraft:admire_goat_horn",
    "minecraft:call_goat_horn",
    "minecraft:yearn_goat_horn",
    "minecraft:dream_goat_horn",
];
const VANILLA_BANNER_PATTERN_TRANSLATION_KEYS: &[&str] = &[
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
];
const VANILLA_ENCHANTMENT_KEYS_AND_MAX_LEVELS: &[(&str, i32)] = &[
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
    ("minecraft:channeling", 1),
    ("minecraft:multishot", 1),
    ("minecraft:quick_charge", 3),
    ("minecraft:piercing", 4),
    ("minecraft:density", 5),
    ("minecraft:breach", 4),
    ("minecraft:wind_burst", 3),
    ("minecraft:lunge", 3),
    ("minecraft:mending", 1),
    ("minecraft:vanishing_curse", 1),
];
const VANILLA_ENCHANTMENT_TOOLTIP_ORDER: &[&str] = &[
    "minecraft:binding_curse",
    "minecraft:vanishing_curse",
    "minecraft:riptide",
    "minecraft:channeling",
    "minecraft:wind_burst",
    "minecraft:frost_walker",
    "minecraft:lunge",
    "minecraft:sharpness",
    "minecraft:smite",
    "minecraft:bane_of_arthropods",
    "minecraft:impaling",
    "minecraft:power",
    "minecraft:density",
    "minecraft:breach",
    "minecraft:piercing",
    "minecraft:sweeping_edge",
    "minecraft:multishot",
    "minecraft:fire_aspect",
    "minecraft:flame",
    "minecraft:knockback",
    "minecraft:punch",
    "minecraft:protection",
    "minecraft:blast_protection",
    "minecraft:fire_protection",
    "minecraft:projectile_protection",
    "minecraft:feather_falling",
    "minecraft:fortune",
    "minecraft:looting",
    "minecraft:silk_touch",
    "minecraft:luck_of_the_sea",
    "minecraft:efficiency",
    "minecraft:quick_charge",
    "minecraft:lure",
    "minecraft:respiration",
    "minecraft:aqua_affinity",
    "minecraft:soul_speed",
    "minecraft:swift_sneak",
    "minecraft:depth_strider",
    "minecraft:thorns",
    "minecraft:loyalty",
    "minecraft:unbreaking",
    "minecraft:infinity",
    "minecraft:mending",
];

#[derive(Debug, Clone, PartialEq)]
pub struct NativeItemTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
    /// Styled draw runs for the line; concatenating the run texts reproduces
    /// `text`. Unstyled lines carry a single default-style run with no colour
    /// override (the renderer then falls back to `tint`).
    pub runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeItemTooltipOptions<'a> {
    pub advanced: bool,
    pub creative: bool,
    pub map_data: Option<NativeItemMapTooltipData>,
    pub enchantment_keys: Option<&'a [String]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeItemMapTooltipData {
    pub scale: i8,
    pub locked: bool,
}

impl NativeItemTooltipLine {
    fn plain(text: String, tint: [f32; 4]) -> Self {
        let runs = vec![HudStyledTextRun::plain(text.clone())];
        Self { text, tint, runs }
    }
}

/// Projects one flattened protocol run into the renderer's resolved run type,
/// first merging `base` under the run's own style (vanilla
/// `ComponentUtils.mergeStyles` / wrapper `withStyle` semantics: the run's own
/// keys win, unset keys inherit `base`).
fn hud_run_from_component(run: &StyledTextRun, base: &ComponentStyle) -> HudStyledTextRun {
    let style = run.style.apply_to(base);
    HudStyledTextRun {
        text: run.text.clone(),
        style: HudTextStyle {
            bold: style.bold == Some(true),
            italic: style.italic == Some(true),
            underlined: style.underlined == Some(true),
            strikethrough: style.strikethrough == Some(true),
            obfuscated: style.obfuscated == Some(true),
        },
        color: style.color,
    }
}

/// Styled-run projection with a plain-text fallback for data decoded before
/// the styled fields existed (or synthesized names): the fallback text becomes
/// a single unstyled run that still receives the `base` style merge.
fn hud_runs_from_component(
    runs: &[StyledTextRun],
    plain_fallback: &str,
    base: &ComponentStyle,
) -> Vec<HudStyledTextRun> {
    if runs.is_empty() {
        let fallback = StyledTextRun {
            text: plain_fallback.to_string(),
            style: ComponentStyle::default(),
        };
        return vec![hud_run_from_component(&fallback, base)];
    }
    runs.iter()
        .map(|run| hud_run_from_component(run, base))
        .collect()
}

pub(super) fn localized_item_name(language: &LanguageCatalog, resource_id: &str) -> String {
    let item_key = description_key("item", resource_id);
    if let Some(name) = language.get(&item_key) {
        return name.to_string();
    }

    let block_key = description_key("block", resource_id);
    language.get(&block_key).unwrap_or(&item_key).to_string()
}

pub(super) fn hover_name_for_stack(
    language: &LanguageCatalog,
    resource_id: &str,
    stack: &ItemStackSummary,
) -> String {
    hover_name_for_component_patch(language, resource_id, &stack.component_patch)
}

fn hover_name_for_component_patch(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> String {
    if let Some(name) = &component_patch.custom_name {
        return name.clone();
    }
    if let Some(title) = component_patch
        .written_book
        .as_ref()
        .map(|book| book.title.as_str())
        .filter(|title| !title.trim().is_empty())
    {
        return title.to_string();
    }
    if let Some(name) = &component_patch.item_name {
        return name.clone();
    }
    localized_item_name(language, resource_id)
}

/// Source runs of the hover name (same precedence as [`hover_name_for_stack`])
/// plus whether they came from `minecraft:custom_name` (which vanilla
/// `ItemStack.getStyledHoverName` additionally italicizes).
fn hover_name_source_runs(
    language: &LanguageCatalog,
    resource_id: &str,
    stack: &ItemStackSummary,
) -> (Vec<StyledTextRun>, bool) {
    let patch = &stack.component_patch;
    if let Some(name) = &patch.custom_name {
        let runs = match &patch.custom_name_styled {
            Some(runs) if !runs.is_empty() => runs.clone(),
            _ => vec![StyledTextRun {
                text: name.clone(),
                style: ComponentStyle::default(),
            }],
        };
        return (runs, true);
    }
    let plain = |text: String| {
        vec![StyledTextRun {
            text,
            style: ComponentStyle::default(),
        }]
    };
    if let Some(title) = patch
        .written_book
        .as_ref()
        .map(|book| book.title.as_str())
        .filter(|title| !title.trim().is_empty())
    {
        return (plain(title.to_string()), false);
    }
    if let Some(name) = &patch.item_name {
        let runs = match &patch.item_name_styled {
            Some(runs) if !runs.is_empty() => runs.clone(),
            _ => plain(name.clone()),
        };
        return (runs, false);
    }
    (plain(localized_item_name(language, resource_id)), false)
}

pub(super) fn push_written_book_tooltip_lines(
    language: &LanguageCatalog,
    book: &bbb_protocol::packets::WrittenBookContentSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !book.author.trim().is_empty() {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "book.byAuthor", &book.author),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key(&format!("book.generation.{}", book.generation))
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn push_bees_tooltip_lines(
    language: &LanguageCatalog,
    bees_count: usize,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if bees_count == 0 {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        translate_with_two_args(
            language,
            "container.beehive.bees",
            &bees_count.to_string(),
            "3",
        ),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn push_map_id_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(map_id) = component_patch.map_id else {
        return;
    };

    let Some(map_data) = options.map_data else {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("filled_map.unknown").to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
        return;
    };

    if component_patch.custom_name.is_none() && component_patch.map_post_processing.is_none() {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "filled_map.id", &map_id.to_string()),
            TOOLTIP_TEXT_GRAY,
        ));
    }

    if map_data.locked
        || component_patch.map_post_processing == Some(MapPostProcessingSummary::Lock)
    {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("filled_map.locked").to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }

    if options.advanced {
        let scale_to_add =
            i8::from(component_patch.map_post_processing == Some(MapPostProcessingSummary::Scale));
        let scale = (map_data.scale + scale_to_add).clamp(0, 4);
        let map_scale = 1_i32 << u32::from(scale as u8);
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "filled_map.scale", &map_scale.to_string()),
            TOOLTIP_TEXT_GRAY,
        ));
        lines.push(NativeItemTooltipLine::plain(
            translate_with_two_args(language, "filled_map.level", &scale.to_string(), "4"),
            TOOLTIP_TEXT_GRAY,
        ));
    }
}

fn push_instrument_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(description) = &component_patch.instrument_description {
        lines.push(NativeItemTooltipLine {
            text: description.clone(),
            tint: TOOLTIP_TEXT_GRAY,
            runs: hud_runs_from_component(
                component_patch
                    .instrument_description_styled
                    .as_deref()
                    .unwrap_or(&[]),
                description,
                &INSTRUMENT_DESCRIPTION_STYLE,
            ),
        });
        return;
    }

    let Some(instrument_key) = component_patch
        .instrument_id
        .and_then(vanilla_instrument_key)
    else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key(&description_key("instrument", instrument_key))
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn vanilla_instrument_key(registry_id: i32) -> Option<&'static str> {
    let index = usize::try_from(registry_id).ok()?;
    VANILLA_INSTRUMENT_KEYS.get(index).copied()
}

fn push_tropical_fish_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(pattern_id) = component_patch.tropical_fish_pattern_id else {
        return;
    };
    let (pattern_id, pattern_name) = tropical_fish_pattern(pattern_id);
    let base_color_id = dye_color_id_or_white(component_patch.tropical_fish_base_color_id);
    let pattern_color_id = dye_color_id_or_white(component_patch.tropical_fish_pattern_color_id);

    if let Some(common_index) =
        tropical_fish_common_variant_index(pattern_id, base_color_id, pattern_color_id)
    {
        lines.push(italic_gray_tooltip_line(
            language
                .get_or_key(&format!(
                    "entity.minecraft.tropical_fish.predefined.{common_index}"
                ))
                .to_string(),
        ));
        return;
    }

    lines.push(italic_gray_tooltip_line(
        language
            .get_or_key(&format!(
                "entity.minecraft.tropical_fish.type.{pattern_name}"
            ))
            .to_string(),
    ));

    let base_color = language
        .get_or_key(&format!(
            "color.minecraft.{}",
            dye_color_name(base_color_id)
        ))
        .to_string();
    let color_text = if base_color_id == pattern_color_id {
        base_color
    } else {
        format!(
            "{}, {}",
            base_color,
            language.get_or_key(&format!(
                "color.minecraft.{}",
                dye_color_name(pattern_color_id)
            ))
        )
    };
    lines.push(italic_gray_tooltip_line(color_text));
}

fn push_banner_pattern_tooltip_lines(
    language: &LanguageCatalog,
    layers: &[BannerPatternLayerSummary],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for layer in layers.iter().take(6) {
        let Some(translation_key) = layer.translation_key.as_deref().or_else(|| {
            layer
                .registry_id
                .and_then(vanilla_banner_pattern_translation_key)
        }) else {
            continue;
        };
        let color_name = dye_color_name(dye_color_id_or_white(Some(layer.color_id)));
        lines.push(NativeItemTooltipLine::plain(
            language
                .get_or_key(&format!("{translation_key}.{color_name}"))
                .to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }
}

fn vanilla_banner_pattern_translation_key(registry_id: i32) -> Option<&'static str> {
    let index = usize::try_from(registry_id).ok()?;
    VANILLA_BANNER_PATTERN_TRANSLATION_KEYS.get(index).copied()
}

fn italic_gray_tooltip_line(text: String) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_GRAY,
        runs: vec![HudStyledTextRun {
            text,
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: Some(TOOLTIP_GRAY_TEXT_COLOR),
        }],
    }
}

fn tropical_fish_pattern(packed_id: i32) -> (i32, &'static str) {
    TROPICAL_FISH_PATTERNS
        .iter()
        .copied()
        .find(|(id, _)| *id == packed_id)
        .unwrap_or((0, "kob"))
}

fn tropical_fish_common_variant_index(
    pattern_id: i32,
    base_color_id: i32,
    pattern_color_id: i32,
) -> Option<usize> {
    TROPICAL_FISH_COMMON_VARIANTS
        .iter()
        .position(|variant| *variant == (pattern_id, base_color_id, pattern_color_id))
}

fn dye_color_id_or_white(color_id: Option<i32>) -> i32 {
    color_id.filter(|id| (0..=15).contains(id)).unwrap_or(0)
}

fn dye_color_name(color_id: i32) -> &'static str {
    match color_id {
        1 => "orange",
        2 => "magenta",
        3 => "light_blue",
        4 => "yellow",
        5 => "lime",
        6 => "pink",
        7 => "gray",
        8 => "light_gray",
        9 => "cyan",
        10 => "purple",
        11 => "blue",
        12 => "brown",
        13 => "green",
        14 => "red",
        15 => "black",
        _ => "white",
    }
}

fn tooltip_display_shows(component_patch: &DataComponentPatchSummary, type_id: i32) -> bool {
    !component_patch.tooltip_hide_tooltip
        && !component_patch
            .tooltip_hidden_component_type_ids
            .contains(&type_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_instrument_keys_follow_26_1_goat_horn_registry_order() {
        assert_eq!(
            VANILLA_INSTRUMENT_KEYS,
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
            vanilla_instrument_key(0),
            Some("minecraft:ponder_goat_horn")
        );
        assert_eq!(vanilla_instrument_key(7), Some("minecraft:dream_goat_horn"));
        assert_eq!(vanilla_instrument_key(8), None);
        assert_eq!(vanilla_instrument_key(-1), None);
    }

    #[test]
    fn tropical_fish_tables_follow_26_1_pattern_and_common_variant_order() {
        assert_eq!(
            TROPICAL_FISH_PATTERNS,
            &[
                (0, "kob"),
                (256, "sunstreak"),
                (512, "snooper"),
                (768, "dasher"),
                (1024, "brinely"),
                (1280, "spotty"),
                (1, "flopper"),
                (257, "stripey"),
                (513, "glitter"),
                (769, "blockfish"),
                (1025, "betty"),
                (1281, "clayfish"),
            ]
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
            VANILLA_BANNER_PATTERN_TRANSLATION_KEYS,
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
            vanilla_banner_pattern_translation_key(5),
            Some("block.minecraft.banner.stripe_bottom")
        );
        assert_eq!(
            vanilla_banner_pattern_translation_key(42),
            Some("block.minecraft.banner.guster")
        );
        assert_eq!(vanilla_banner_pattern_translation_key(43), None);
        assert_eq!(vanilla_banner_pattern_translation_key(-1), None);
    }

    #[test]
    fn vanilla_enchantment_tables_follow_26_1_registry_and_tooltip_order() {
        assert_eq!(
            VANILLA_ENCHANTMENT_KEYS_AND_MAX_LEVELS,
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
                ("minecraft:channeling", 1),
                ("minecraft:multishot", 1),
                ("minecraft:quick_charge", 3),
                ("minecraft:piercing", 4),
                ("minecraft:density", 5),
                ("minecraft:breach", 4),
                ("minecraft:wind_burst", 3),
                ("minecraft:lunge", 3),
                ("minecraft:mending", 1),
                ("minecraft:vanishing_curse", 1),
            ]
        );
        assert_eq!(
            VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
            &[
                "minecraft:binding_curse",
                "minecraft:vanishing_curse",
                "minecraft:riptide",
                "minecraft:channeling",
                "minecraft:wind_burst",
                "minecraft:frost_walker",
                "minecraft:lunge",
                "minecraft:sharpness",
                "minecraft:smite",
                "minecraft:bane_of_arthropods",
                "minecraft:impaling",
                "minecraft:power",
                "minecraft:density",
                "minecraft:breach",
                "minecraft:piercing",
                "minecraft:sweeping_edge",
                "minecraft:multishot",
                "minecraft:fire_aspect",
                "minecraft:flame",
                "minecraft:knockback",
                "minecraft:punch",
                "minecraft:protection",
                "minecraft:blast_protection",
                "minecraft:fire_protection",
                "minecraft:projectile_protection",
                "minecraft:feather_falling",
                "minecraft:fortune",
                "minecraft:looting",
                "minecraft:silk_touch",
                "minecraft:luck_of_the_sea",
                "minecraft:efficiency",
                "minecraft:quick_charge",
                "minecraft:lure",
                "minecraft:respiration",
                "minecraft:aqua_affinity",
                "minecraft:soul_speed",
                "minecraft:swift_sneak",
                "minecraft:depth_strider",
                "minecraft:thorns",
                "minecraft:loyalty",
                "minecraft:unbreaking",
                "minecraft:infinity",
                "minecraft:mending",
            ]
        );
        assert_eq!(
            enchantment_key_for_holder(13, None),
            Some("minecraft:sharpness")
        );
        assert_eq!(
            vanilla_enchantment_max_level("minecraft:sharpness"),
            Some(5)
        );
        assert!(enchantment_is_curse("minecraft:binding_curse", None));
        assert!(!enchantment_is_curse("minecraft:sharpness", None));
    }
}

fn push_container_loot_tooltip_lines(
    language: &LanguageCatalog,
    container_loot: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !container_loot {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key("item.container.loot_table.unknown")
            .to_string(),
        TOOLTIP_TEXT_WHITE,
    ));
}

fn charged_projectile_group_tooltip_text(
    language: &LanguageCatalog,
    projectile_name: &str,
    count: usize,
) -> String {
    if count == 1 {
        translate_with_first_arg(
            language,
            "item.minecraft.crossbow.projectile.single",
            projectile_name,
        )
    } else {
        translate_with_two_args(
            language,
            "item.minecraft.crossbow.projectile.multiple",
            &count.to_string(),
            projectile_name,
        )
    }
}

fn item_container_more_tooltip_line(text: String) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_WHITE,
        runs: vec![HudStyledTextRun {
            text,
            style: HudTextStyle {
                italic: true,
                ..HudTextStyle::default()
            },
            color: None,
        }],
    }
}

fn push_intangible_projectile_tooltip_line(
    language: &LanguageCatalog,
    intangible_projectile: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !intangible_projectile {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key("item.intangible").to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn push_ominous_bottle_tooltip_lines(
    language: &LanguageCatalog,
    amplifier: Option<i32>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(amplifier) = amplifier else {
        return;
    };

    lines.push(NativeItemTooltipLine::plain(
        potion_effect_tooltip_text(
            language,
            "minecraft:bad_omen",
            amplifier,
            OMINOUS_BOTTLE_BAD_OMEN_DURATION_TICKS,
        ),
        TOOLTIP_TEXT_BLUE,
    ));
}

fn push_potion_contents_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for effect in &component_patch.potion_custom_effects {
        let Some(effect_key) = vanilla_mob_effect_key(effect.effect_id) else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            potion_effect_tooltip_text(language, effect_key, effect.amplifier, effect.duration),
            mob_effect_tooltip_tint(effect),
        ));
    }
}

fn push_suspicious_stew_tooltip_lines(
    language: &LanguageCatalog,
    effects: &[SuspiciousStewEffectSummary],
    creative: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !creative {
        return;
    }
    for effect in effects {
        let Some(effect_key) = vanilla_mob_effect_key(effect.effect_id) else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            potion_effect_tooltip_text(language, effect_key, 0, effect.duration),
            mob_effect_tooltip_tint_for_id(effect.effect_id),
        ));
    }
}

fn potion_effect_tooltip_text(
    language: &LanguageCatalog,
    effect_key: &str,
    amplifier: i32,
    duration_ticks: i32,
) -> String {
    let mut effect = language
        .get_or_key(&description_key("effect", effect_key))
        .to_string();
    if amplifier > 0 {
        let potency = language
            .get_or_key(&format!("potion.potency.{amplifier}"))
            .to_string();
        effect = translate_with_two_args(language, "potion.withAmplifier", &effect, &potency);
    }

    if duration_ticks == -1 || duration_ticks > 20 {
        let duration = if duration_ticks == -1 {
            language.get_or_key("effect.duration.infinite").to_string()
        } else {
            format_tick_duration(duration_ticks, DEFAULT_TOOLTIP_TICKRATE)
        };
        effect = translate_with_two_args(language, "potion.withDuration", &effect, &duration);
    }

    effect
}

fn mob_effect_tooltip_tint(effect: &MobEffectInstanceSummary) -> [f32; 4] {
    mob_effect_tooltip_tint_for_id(effect.effect_id)
}

fn mob_effect_tooltip_tint_for_id(effect_id: i32) -> [f32; 4] {
    match vanilla_mob_effect_category(effect_id) {
        Some(VanillaMobEffectCategory::Harmful) => TOOLTIP_TEXT_RED,
        Some(VanillaMobEffectCategory::Beneficial | VanillaMobEffectCategory::Neutral) => {
            TOOLTIP_TEXT_BLUE
        }
        None => TOOLTIP_TEXT_GRAY,
    }
}

fn format_tick_duration(ticks: i32, tickrate: f32) -> String {
    let mut seconds = ((ticks as f32) / tickrate).floor() as i32;
    let mut minutes = seconds / 60;
    seconds %= 60;
    let hours = minutes / 60;
    minutes %= 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn push_dyed_color_tooltip_lines(
    language: &LanguageCatalog,
    dyed_color: Option<i32>,
    advanced: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(dyed_color) = dyed_color else {
        return;
    };
    if advanced {
        let rgb = (dyed_color as u32) & 0x00FF_FFFF;
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "item.color", &format!("#{rgb:06X}")),
            TOOLTIP_TEXT_GRAY,
        ));
    } else {
        let text = language.get_or_key("item.dyed").to_string();
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_GRAY,
            runs: vec![HudStyledTextRun {
                text,
                style: HudTextStyle {
                    italic: true,
                    ..HudTextStyle::default()
                },
                color: Some(0xAA_AA_AA),
            }],
        });
    }
}

fn push_fireworks_tooltip_lines(
    language: &LanguageCatalog,
    flight_duration: Option<i32>,
    explosions: &[FireworkExplosionSummary],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(flight_duration) = flight_duration.filter(|flight_duration| *flight_duration > 0) {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{} {}",
                language.get_or_key("item.minecraft.firework_rocket.flight"),
                flight_duration
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }

    let mut current = None;
    let mut count = 0;
    for explosion in explosions {
        match current {
            None => {
                current = Some(explosion);
                count = 1;
            }
            Some(previous) if previous == explosion => {
                count += 1;
            }
            Some(previous) => {
                push_fireworks_explosion_group_tooltip_lines(language, previous, count, lines);
                current = Some(explosion);
                count = 1;
            }
        }
    }
    if let Some(explosion) = current {
        push_fireworks_explosion_group_tooltip_lines(language, explosion, count, lines);
    }
}

fn push_fireworks_explosion_group_tooltip_lines(
    language: &LanguageCatalog,
    explosion: &FireworkExplosionSummary,
    count: usize,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let shape_name = firework_explosion_shape_text(language, explosion.shape);
    let text = if count == 1 {
        translate_with_first_arg(
            language,
            "item.minecraft.firework_rocket.single_star",
            &shape_name,
        )
    } else {
        translate_with_two_args(
            language,
            "item.minecraft.firework_rocket.multiple_stars",
            &count.to_string(),
            &shape_name,
        )
    };
    lines.push(NativeItemTooltipLine::plain(text, TOOLTIP_TEXT_GRAY));
    push_firework_explosion_additional_tooltip_lines(
        language,
        &explosion.colors,
        &explosion.fade_colors,
        explosion.has_trail,
        explosion.has_twinkle,
        "  ",
        lines,
    );
}

fn push_firework_explosion_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(shape) = component_patch.firework_explosion_shape else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        firework_explosion_shape_text(language, shape),
        TOOLTIP_TEXT_GRAY,
    ));
    push_firework_explosion_additional_tooltip_lines(
        language,
        &component_patch.firework_explosion_colors,
        &component_patch.firework_explosion_fade_colors,
        component_patch
            .firework_explosion_has_trail
            .unwrap_or_default(),
        component_patch
            .firework_explosion_has_twinkle
            .unwrap_or_default(),
        "",
        lines,
    );
}

fn push_firework_explosion_additional_tooltip_lines(
    language: &LanguageCatalog,
    colors: &[i32],
    fade_colors: &[i32],
    has_trail: bool,
    has_twinkle: bool,
    prefix: &str,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !colors.is_empty() {
        lines.push(NativeItemTooltipLine::plain(
            format!("{prefix}{}", firework_color_names(language, colors)),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if !fade_colors.is_empty() {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{} {}",
                language.get_or_key("item.minecraft.firework_star.fade_to"),
                firework_color_names(language, fade_colors)
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if has_trail {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{}",
                language.get_or_key("item.minecraft.firework_star.trail")
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if has_twinkle {
        lines.push(NativeItemTooltipLine::plain(
            format!(
                "{prefix}{}",
                language.get_or_key("item.minecraft.firework_star.flicker")
            ),
            TOOLTIP_TEXT_GRAY,
        ));
    }
}

fn push_jukebox_playable_tooltip_lines(
    song: Option<&JukeboxSongSummary>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(song) = song else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        song.description.clone(),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn push_armor_trim_tooltip_lines(
    language: &LanguageCatalog,
    material: Option<&TrimMaterialSummary>,
    pattern: Option<&TrimPatternSummary>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let (Some(material), Some(pattern)) = (material, pattern) else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key("item.minecraft.smithing_template.upgrade")
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(NativeItemTooltipLine::plain(
        format!(" {}", pattern.description),
        TOOLTIP_TEXT_WHITE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        format!(" {}", material.description),
        TOOLTIP_TEXT_WHITE,
    ));
}

fn push_enchantments_tooltip_lines(
    language: &LanguageCatalog,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: Option<&[String]>,
    tooltip_order: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if enchantments.is_empty() {
        return;
    }

    let resolved = enchantments
        .iter()
        .enumerate()
        .filter_map(|(index, enchantment)| {
            enchantment_key_for_holder(enchantment.holder_id, enchantment_keys)
                .map(|key| (index, enchantment, key))
        })
        .collect::<Vec<_>>();
    let mut emitted = vec![false; enchantments.len()];
    if let Some(tooltip_order) = tooltip_order {
        for order_key in tooltip_order {
            push_matching_ordered_enchantments(
                language,
                &resolved,
                &mut emitted,
                order_key,
                enchantment_tags,
                lines,
            );
        }
    } else {
        for order_key in VANILLA_ENCHANTMENT_TOOLTIP_ORDER {
            push_matching_ordered_enchantments(
                language,
                &resolved,
                &mut emitted,
                order_key,
                enchantment_tags,
                lines,
            );
        }
    }

    for (index, enchantment, key) in resolved {
        if !emitted[index] {
            push_enchantment_tooltip_line(
                language,
                key,
                enchantment.level,
                enchantment_tags,
                lines,
            );
        }
    }
}

fn push_matching_ordered_enchantments(
    language: &LanguageCatalog,
    resolved: &[(usize, &ItemEnchantmentSummary, &str)],
    emitted: &mut [bool],
    order_key: &str,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for (index, enchantment, key) in resolved {
        if !emitted[*index] && *key == order_key && enchantment.level > 0 {
            push_enchantment_tooltip_line(
                language,
                key,
                enchantment.level,
                enchantment_tags,
                lines,
            );
            emitted[*index] = true;
        }
    }
}

fn push_enchantment_tooltip_line(
    language: &LanguageCatalog,
    enchantment_key: &str,
    level: i32,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let text = enchantment_tooltip_text(language, enchantment_key, level);
    let tint = if enchantment_is_curse(enchantment_key, enchantment_tags) {
        TOOLTIP_TEXT_RED
    } else {
        TOOLTIP_TEXT_GRAY
    };
    lines.push(NativeItemTooltipLine::plain(text, tint));
}

fn enchantment_tooltip_text(
    language: &LanguageCatalog,
    enchantment_key: &str,
    level: i32,
) -> String {
    let mut text = language
        .get_or_key(&description_key("enchantment", enchantment_key))
        .to_string();
    let max_level = vanilla_enchantment_max_level(enchantment_key).unwrap_or(1);
    if level != 1 || max_level != 1 {
        text.push(' ');
        text.push_str(language.get_or_key(&format!("enchantment.level.{level}")));
    }
    text
}

fn enchantment_key_for_holder<'a>(
    holder_id: i32,
    enchantment_keys: Option<&'a [String]>,
) -> Option<&'a str> {
    let index = usize::try_from(holder_id).ok()?;
    if let Some(enchantment_keys) = enchantment_keys {
        return enchantment_keys.get(index).map(String::as_str);
    }
    VANILLA_ENCHANTMENT_KEYS_AND_MAX_LEVELS
        .get(index)
        .map(|(key, _)| *key)
}

fn vanilla_enchantment_max_level(enchantment_key: &str) -> Option<i32> {
    VANILLA_ENCHANTMENT_KEYS_AND_MAX_LEVELS
        .iter()
        .find(|(key, _)| *key == enchantment_key)
        .map(|(_, max_level)| *max_level)
}

fn enchantment_is_curse(enchantment_key: &str, enchantment_tags: Option<&TagCatalog>) -> bool {
    enchantment_tags.is_some_and(|tags| tags.contains("minecraft:curse", enchantment_key))
        || matches!(
            enchantment_key,
            "minecraft:binding_curse" | "minecraft:vanishing_curse"
        )
}

fn push_profile_tooltip_lines(
    language: &LanguageCatalog,
    profile: Option<&ResolvableProfileSummary>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !profile.is_some_and(is_dynamic_profile) {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key("component.profile.dynamic").to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
}

fn is_dynamic_profile(profile: &ResolvableProfileSummary) -> bool {
    profile.kind == ResolvableProfileKindSummary::Partial
        && profile.properties.is_empty()
        && (profile.name.is_some() != profile.uuid.is_some())
}

fn firework_explosion_shape_text(
    language: &LanguageCatalog,
    shape: FireworkExplosionShapeSummary,
) -> String {
    language
        .get_or_key(&format!(
            "item.minecraft.firework_star.shape.{}",
            firework_explosion_shape_name(shape)
        ))
        .to_string()
}

fn firework_explosion_shape_name(shape: FireworkExplosionShapeSummary) -> &'static str {
    match shape {
        FireworkExplosionShapeSummary::SmallBall => "small_ball",
        FireworkExplosionShapeSummary::LargeBall => "large_ball",
        FireworkExplosionShapeSummary::Star => "star",
        FireworkExplosionShapeSummary::Creeper => "creeper",
        FireworkExplosionShapeSummary::Burst => "burst",
    }
}

fn firework_color_names(language: &LanguageCatalog, colors: &[i32]) -> String {
    colors
        .iter()
        .map(|color| {
            language
                .get_or_key(firework_color_translation_key(*color))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn firework_color_translation_key(color: i32) -> &'static str {
    match color {
        15_790_320 => "item.minecraft.firework_star.white",
        15_435_844 => "item.minecraft.firework_star.orange",
        12_801_229 => "item.minecraft.firework_star.magenta",
        6_719_955 => "item.minecraft.firework_star.light_blue",
        14_602_026 => "item.minecraft.firework_star.yellow",
        4_312_372 => "item.minecraft.firework_star.lime",
        14_188_952 => "item.minecraft.firework_star.pink",
        4_408_131 => "item.minecraft.firework_star.gray",
        11_250_603 => "item.minecraft.firework_star.light_gray",
        2_651_799 => "item.minecraft.firework_star.cyan",
        8_073_150 => "item.minecraft.firework_star.purple",
        2_437_522 => "item.minecraft.firework_star.blue",
        5_320_730 => "item.minecraft.firework_star.brown",
        3_887_386 => "item.minecraft.firework_star.green",
        11_743_532 => "item.minecraft.firework_star.red",
        1_973_019 => "item.minecraft.firework_star.black",
        _ => "item.minecraft.firework_star.custom_color",
    }
}

fn push_block_state_tooltip_lines(
    language: &LanguageCatalog,
    block_state_properties: &BTreeMap<String, String>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(honey_level) = block_state_properties
        .get("honey_level")
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|honey_level| (0..=5).contains(honey_level))
    else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        translate_with_two_args(
            language,
            "container.beehive.honey",
            &honey_level.to_string(),
            "5",
        ),
        TOOLTIP_TEXT_GRAY,
    ));
}

pub(super) fn translate_with_first_arg(language: &LanguageCatalog, key: &str, arg: &str) -> String {
    let template = language.get_or_key(key);
    if template.contains("%1$s") {
        template.replace("%1$s", arg)
    } else {
        template.replacen("%s", arg, 1)
    }
}

pub(super) fn translate_with_two_args(
    language: &LanguageCatalog,
    key: &str,
    first: &str,
    second: &str,
) -> String {
    let mut translated = translate_with_first_arg(language, key, first);
    if translated.contains("%2$s") {
        translated = translated.replace("%2$s", second);
    }
    translated.replacen("%s", second, 1)
}

pub(super) fn item_rarity_for_stack(
    component_patch: &DataComponentPatchSummary,
) -> ItemRaritySummary {
    let base = component_patch.rarity.unwrap_or(ItemRaritySummary::Common);
    if component_patch.enchantments.is_empty() {
        return base;
    }
    match base {
        ItemRaritySummary::Common | ItemRaritySummary::Uncommon => ItemRaritySummary::Rare,
        ItemRaritySummary::Rare => ItemRaritySummary::Epic,
        ItemRaritySummary::Epic => ItemRaritySummary::Epic,
    }
}

pub(super) fn item_rarity_tint(rarity: ItemRaritySummary) -> [f32; 4] {
    match rarity {
        ItemRaritySummary::Common => TOOLTIP_TEXT_WHITE,
        ItemRaritySummary::Uncommon => TOOLTIP_TEXT_YELLOW,
        ItemRaritySummary::Rare => TOOLTIP_TEXT_AQUA,
        ItemRaritySummary::Epic => TOOLTIP_TEXT_LIGHT_PURPLE,
    }
}

/// Vanilla `Rarity.color()` as a text colour value (`ChatFormatting` colour
/// ints); the `[f32; 4]` twin of [`item_rarity_tint`].
fn item_rarity_color(rarity: ItemRaritySummary) -> u32 {
    match rarity {
        ItemRaritySummary::Common => 0xFF_FF_FF,   // WHITE
        ItemRaritySummary::Uncommon => 0xFF_FF_55, // YELLOW
        ItemRaritySummary::Rare => 0x55_FF_FF,     // AQUA
        ItemRaritySummary::Epic => 0xFF_55_FF,     // LIGHT_PURPLE
    }
}

fn effective_damage_state(
    component_patch: &DataComponentPatchSummary,
    default_max_damage: Option<i32>,
) -> Option<(i32, i32)> {
    let max_damage = component_patch
        .max_damage
        .or(default_max_damage)
        .filter(|max_damage| *max_damage > 0)?;
    let damage = component_patch.damage.unwrap_or(0).clamp(0, max_damage);
    Some((damage, max_damage))
}

fn advanced_component_count(
    component_patch: &DataComponentPatchSummary,
    default_component_type_ids: Option<&BTreeSet<i32>>,
) -> Option<usize> {
    let mut type_ids = default_component_type_ids?.clone();
    for type_id in &component_patch.removed_type_ids {
        type_ids.remove(type_id);
    }
    for type_id in &component_patch.added_type_ids {
        type_ids.insert(*type_id);
    }
    Some(type_ids.len())
}

fn push_advanced_tooltip_lines(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
    default_max_damage: Option<i32>,
    default_component_type_ids: Option<&BTreeSet<i32>>,
    show_damage: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some((damage, max_damage)) = show_damage
        .then(|| effective_damage_state(component_patch, default_max_damage))
        .flatten()
        .filter(|(damage, _)| *damage > 0)
    {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_two_args(
                language,
                "item.durability",
                &(max_damage - damage).to_string(),
                &max_damage.to_string(),
            ),
            TOOLTIP_TEXT_WHITE,
        ));
    }

    lines.push(NativeItemTooltipLine::plain(
        resource_id.to_string(),
        TOOLTIP_TEXT_DARK_GRAY,
    ));
    if let Some(component_count) =
        advanced_component_count(component_patch, default_component_type_ids)
            .filter(|component_count| *component_count > 0)
    {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "item.components", &component_count.to_string()),
            TOOLTIP_TEXT_DARK_GRAY,
        ));
    }
}

pub(super) fn description_key(prefix: &str, resource_id: &str) -> String {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    format!("{prefix}.{namespace}.{}", path.replace('/', "."))
}

impl NativeItemRuntime {
    fn template_hover_name(&self, item: &ItemStackTemplateSummary) -> Option<String> {
        let resource_id = self.registry.as_ref()?.resource_id(item.item_id)?;
        Some(hover_name_for_component_patch(
            &self.language,
            resource_id,
            &item.component_patch,
        ))
    }

    fn push_charged_projectile_group_tooltip_line(
        &self,
        projectile: &ItemStackTemplateSummary,
        count: usize,
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        let Some(projectile_name) = self.template_hover_name(projectile) else {
            return;
        };
        lines.push(NativeItemTooltipLine::plain(
            charged_projectile_group_tooltip_text(&self.language, &projectile_name, count),
            TOOLTIP_TEXT_WHITE,
        ));
    }

    fn push_charged_projectiles_tooltip_lines(
        &self,
        projectiles: &[ItemStackTemplateSummary],
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        let mut current = None;
        let mut count = 0;
        for projectile in projectiles {
            match current {
                None => {
                    current = Some(projectile);
                    count = 1;
                }
                Some(previous) if previous == projectile => {
                    count += 1;
                }
                Some(previous) => {
                    self.push_charged_projectile_group_tooltip_line(previous, count, lines);
                    current = Some(projectile);
                    count = 1;
                }
            }
        }
        if let Some(projectile) = current {
            self.push_charged_projectile_group_tooltip_line(projectile, count, lines);
        }
    }

    fn push_container_items_tooltip_lines(
        &self,
        items: &[ItemStackTemplateSummary],
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        let mut line_count = 0;
        let item_count = items.len();
        for item in items {
            if line_count > 4 {
                continue;
            }
            line_count += 1;
            let Some(item_name) = self.template_hover_name(item) else {
                continue;
            };
            lines.push(NativeItemTooltipLine::plain(
                translate_with_two_args(
                    &self.language,
                    "item.container.item_count",
                    &item_name,
                    &item.count.to_string(),
                ),
                TOOLTIP_TEXT_WHITE,
            ));
        }

        let hidden_count = item_count.saturating_sub(line_count);
        if hidden_count > 0 {
            lines.push(item_container_more_tooltip_line(translate_with_first_arg(
                &self.language,
                "item.container.more_items",
                &hidden_count.to_string(),
            )));
        }
    }

    fn push_pot_decorations_tooltip_lines(
        &self,
        item_ids: &[i32],
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        if item_ids.is_empty() {
            return;
        }

        let brick_id = self
            .registry
            .as_ref()
            .and_then(|registry| registry.protocol_id("minecraft:brick"));
        let side_ids = [
            item_ids.get(3).copied().or(brick_id),
            item_ids.get(1).copied().or(brick_id),
            item_ids.get(2).copied().or(brick_id),
            item_ids.first().copied().or(brick_id),
        ];
        if brick_id.is_some_and(|brick_id| side_ids.iter().all(|id| *id == Some(brick_id))) {
            return;
        }

        lines.push(NativeItemTooltipLine::plain(
            String::new(),
            TOOLTIP_TEXT_WHITE,
        ));
        for item_id in side_ids.into_iter().flatten() {
            let Some(item_name) = self.item_hover_name_for_protocol_id(item_id) else {
                continue;
            };
            lines.push(NativeItemTooltipLine::plain(item_name, TOOLTIP_TEXT_GRAY));
        }
    }

    fn item_hover_name_for_protocol_id(&self, item_id: i32) -> Option<String> {
        Some(localized_item_name(
            &self.language,
            self.item_resource_id(item_id)?,
        ))
    }

    pub fn tooltip_lines_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        self.tooltip_lines_for_stack_with_options(stack, false)
    }

    pub fn tooltip_lines_for_stack_with_options(
        &self,
        stack: &ItemStackSummary,
        advanced: bool,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        self.tooltip_lines_for_stack_with_context(
            stack,
            NativeItemTooltipOptions {
                advanced,
                creative: false,
                map_data: None,
                enchantment_keys: None,
            },
        )
    }

    pub fn tooltip_lines_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        options: NativeItemTooltipOptions<'_>,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        if item_stack_is_empty(stack) {
            return None;
        }
        let protocol_id = stack.item_id?;
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        if stack.component_patch.tooltip_hide_tooltip && !options.creative {
            return None;
        }
        let shows = |type_id| tooltip_display_shows(&stack.component_patch, type_id);

        // Vanilla `ItemStack.getStyledHoverName`: the hover name is wrapped in
        // the rarity colour, plus ITALIC when the stack carries a custom name;
        // the name component's own style keys win over the wrapper.
        let rarity = item_rarity_for_stack(&stack.component_patch);
        let (name_runs, name_is_custom) = hover_name_source_runs(&self.language, item_id, stack);
        let name_wrapper = ComponentStyle {
            italic: name_is_custom.then_some(true),
            color: Some(item_rarity_color(rarity)),
            ..ComponentStyle::default()
        };
        let mut lines = vec![NativeItemTooltipLine {
            text: hover_name_for_stack(&self.language, item_id, stack),
            tint: item_rarity_tint(rarity),
            runs: name_runs
                .iter()
                .map(|run| hud_run_from_component(run, &name_wrapper))
                .collect(),
        }];
        if shows(COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID) {
            push_tropical_fish_tooltip_lines(&self.language, &stack.component_patch, &mut lines);
        }
        if shows(COMPONENT_INSTRUMENT_TYPE_ID) {
            push_instrument_tooltip_lines(&self.language, &stack.component_patch, &mut lines);
        }
        if shows(COMPONENT_MAP_ID_TYPE_ID) {
            push_map_id_tooltip_lines(&self.language, &stack.component_patch, options, &mut lines);
        }
        if shows(COMPONENT_BEES_TYPE_ID) {
            push_bees_tooltip_lines(&self.language, stack.component_patch.bees_count, &mut lines);
        }
        if shows(COMPONENT_CONTAINER_LOOT_TYPE_ID) {
            push_container_loot_tooltip_lines(
                &self.language,
                stack.component_patch.container_loot,
                &mut lines,
            );
        }
        if shows(COMPONENT_CONTAINER_TYPE_ID) {
            self.push_container_items_tooltip_lines(
                &stack.component_patch.container_items,
                &mut lines,
            );
        }
        if shows(COMPONENT_BANNER_PATTERNS_TYPE_ID) {
            push_banner_pattern_tooltip_lines(
                &self.language,
                &stack.component_patch.banner_pattern_layers,
                &mut lines,
            );
        }
        if shows(COMPONENT_POT_DECORATIONS_TYPE_ID) {
            self.push_pot_decorations_tooltip_lines(
                &stack.component_patch.pot_decorations_item_ids,
                &mut lines,
            );
        }
        if let Some(book) = stack
            .component_patch
            .written_book
            .as_ref()
            .filter(|_| shows(COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID))
        {
            push_written_book_tooltip_lines(&self.language, book, &mut lines);
        }
        if shows(COMPONENT_CHARGED_PROJECTILES_TYPE_ID) {
            self.push_charged_projectiles_tooltip_lines(
                &stack.component_patch.charged_projectiles_items,
                &mut lines,
            );
        }
        if shows(COMPONENT_FIREWORKS_TYPE_ID) {
            push_fireworks_tooltip_lines(
                &self.language,
                stack.component_patch.fireworks_flight_duration,
                &stack.component_patch.fireworks_explosions,
                &mut lines,
            );
        }
        if shows(COMPONENT_FIREWORK_EXPLOSION_TYPE_ID) {
            push_firework_explosion_tooltip_lines(
                &self.language,
                &stack.component_patch,
                &mut lines,
            );
        }
        if shows(COMPONENT_POTION_CONTENTS_TYPE_ID) {
            push_potion_contents_tooltip_lines(&self.language, &stack.component_patch, &mut lines);
        }
        if shows(COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID) {
            push_jukebox_playable_tooltip_lines(
                stack.component_patch.jukebox_direct_song.as_ref(),
                &mut lines,
            );
        }
        if shows(COMPONENT_TRIM_TYPE_ID) {
            push_armor_trim_tooltip_lines(
                &self.language,
                stack.component_patch.armor_trim_material_direct.as_ref(),
                stack.component_patch.armor_trim_pattern_direct.as_ref(),
                &mut lines,
            );
        }
        let enchantment_tags = self.enchantment_tags.as_ref();
        let tooltip_order =
            enchantment_tags.and_then(|tags| tags.values("minecraft:tooltip_order"));
        if shows(COMPONENT_STORED_ENCHANTMENTS_TYPE_ID) {
            push_enchantments_tooltip_lines(
                &self.language,
                &stack.component_patch.stored_enchantments,
                options.enchantment_keys,
                tooltip_order,
                enchantment_tags,
                &mut lines,
            );
        }
        if shows(COMPONENT_ENCHANTMENTS_TYPE_ID) {
            push_enchantments_tooltip_lines(
                &self.language,
                &stack.component_patch.enchantments,
                options.enchantment_keys,
                tooltip_order,
                enchantment_tags,
                &mut lines,
            );
        }
        if shows(COMPONENT_DYED_COLOR_TYPE_ID) {
            push_dyed_color_tooltip_lines(
                &self.language,
                stack.component_patch.dyed_color,
                options.advanced,
                &mut lines,
            );
        }
        if shows(COMPONENT_PROFILE_TYPE_ID) {
            push_profile_tooltip_lines(
                &self.language,
                stack.component_patch.profile.as_ref(),
                &mut lines,
            );
        }
        // Vanilla `ItemLore.styledLines`: every lore line gets `LORE_STYLE`
        // (DARK_PURPLE + italic) merged under its own style keys.
        if shows(COMPONENT_LORE_TYPE_ID) {
            lines.extend(
                stack
                    .component_patch
                    .lore
                    .iter()
                    .enumerate()
                    .map(|(index, text)| NativeItemTooltipLine {
                        text: text.clone(),
                        tint: TOOLTIP_TEXT_DARK_PURPLE,
                        runs: hud_runs_from_component(
                            stack
                                .component_patch
                                .lore_styled
                                .get(index)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            text,
                            &LORE_STYLE,
                        ),
                    }),
            );
        }
        if shows(COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID) {
            push_intangible_projectile_tooltip_line(
                &self.language,
                stack.component_patch.intangible_projectile,
                &mut lines,
            );
        }
        if shows(COMPONENT_UNBREAKABLE_TYPE_ID) && stack.component_patch.unbreakable {
            lines.push(NativeItemTooltipLine::plain(
                self.language.get_or_key("item.unbreakable").to_string(),
                TOOLTIP_TEXT_BLUE,
            ));
        }
        if shows(COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID) {
            push_ominous_bottle_tooltip_lines(
                &self.language,
                stack.component_patch.ominous_bottle_amplifier,
                &mut lines,
            );
        }
        if shows(COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID) {
            push_suspicious_stew_tooltip_lines(
                &self.language,
                &stack.component_patch.suspicious_stew_effects,
                options.creative,
                &mut lines,
            );
        }
        if shows(COMPONENT_BLOCK_STATE_TYPE_ID) {
            push_block_state_tooltip_lines(
                &self.language,
                &stack.component_patch.block_state_properties,
                &mut lines,
            );
        }
        if options.advanced {
            push_advanced_tooltip_lines(
                &self.language,
                item_id,
                &stack.component_patch,
                self.default_max_damage_for_protocol_id(protocol_id),
                self.default_component_type_ids_for_resource_id(item_id),
                shows(COMPONENT_DAMAGE_TYPE_ID),
                &mut lines,
            );
        }
        Some(lines)
    }
}
