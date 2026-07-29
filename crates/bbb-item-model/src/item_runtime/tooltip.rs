use bbb_protocol::data_component_types as component_id;
use bbb_protocol::{
    attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
    },
    attribute_types::{
        vanilla_attribute_type, vanilla_attribute_type_for_resource_id, AttributeSentiment,
        AttributeTypeCatalogEntry,
    },
    block_entity_types::vanilla_block_entity_type_only_op_can_set_nbt,
    builtin_enum_component_values::{normalize_vanilla_dye_color_id, vanilla_dye_color_name},
    entity_types::{
        vanilla_entity_type_allowed_in_peaceful, vanilla_entity_type_only_op_can_set_nbt,
    },
    equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUPS,
    mob_effect_types::{vanilla_mob_effect_type, MobEffectCategory},
    packets::{
        AdventureModePredicateSummary, AttributeModifierSummary, PaintingVariantSummary,
        SuspiciousStewEffectSummary, MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
    },
    tropical_fish_patterns::{
        vanilla_tropical_fish_pattern, VANILLA_TROPICAL_FISH_COMMON_VARIANTS,
    },
    ComponentStyle, NbtSummaryValue, StyledTextRun,
};
use bbb_render_types::BannerPatternKind;
#[cfg(test)]
use bbb_world::VANILLA_PAINTING_VARIANTS;
use bbb_world::{
    direct_banner_pattern_presentation, direct_trim_material_presentation,
    direct_trim_pattern_presentation, painting_variant_translation_key, vanilla_painting_variant,
    BannerPatternPresentation, EnchantmentAttributeModifier, EnchantmentTooltipDefinition,
    InstrumentPresentation, JukeboxSongPresentation, PaintingVariantPresentation, RegistrySet,
    TrimMaterialPresentation, TrimPatternPresentation, VanillaEnchantmentIdentity,
    VanillaInstrumentIdentity, VanillaPaintingVariant, VanillaTrimMaterialIdentity,
    VanillaTrimPatternIdentity, VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
};

use super::icon_model::effective_attribute_modifiers;
use super::*;
use bbb_protocol::potion_types::vanilla_potion;
#[cfg(test)]
use bbb_protocol::potion_types::VanillaPotionEffect;

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
    hover_event: None,
    font: None,
};
const OMINOUS_BOTTLE_BAD_OMEN_DURATION_TICKS: i32 = 120_000;
const DEFAULT_TOOLTIP_TICKRATE: f32 = 20.0;
const TOOLTIP_GRAY_TEXT_COLOR: u32 = 0xAA_AA_AA;
const TOOLTIP_BLUE_TEXT_COLOR: u32 = 0x55_55_FF;
const TOOLTIP_DARK_GRAY_TEXT_COLOR: u32 = 0x55_55_55;
const TOOLTIP_DARK_PURPLE_TEXT_COLOR: u32 = 0xAA_00_AA;
const TOOLTIP_RED_TEXT_COLOR: u32 = 0xFF_55_55;
const TOOLTIP_YELLOW_TEXT_COLOR: u32 = 0xFF_FF_55;
const DISC_FRAGMENT_5_DESCRIPTION_KEY: &str = "item.minecraft.disc_fragment_5.desc";
const PAINTING_DIMENSIONS_KEY: &str = "painting.dimensions";
const PAINTING_RANDOM_KEY: &str = "painting.random";
const POTION_WHEN_DRANK_KEY: &str = "potion.whenDrank";
const SPAWNER_DESC1_KEY: &str = "block.minecraft.spawner.desc1";
const SPAWNER_DESC2_KEY: &str = "block.minecraft.spawner.desc2";
const CAN_BREAK_HEADER_KEY: &str = "item.canBreak";
const CAN_PLACE_HEADER_KEY: &str = "item.canPlace";
const CAN_USE_UNKNOWN_KEY: &str = "item.canUse.unknown";
const SPAWN_EGG_PEACEFUL_KEY: &str = "item.spawn_egg.peaceful";
const ARMOR_TRIM_SMITHING_TEMPLATE_SUFFIX: &str = "_armor_trim_smithing_template";
const SMITHING_TEMPLATE_SUFFIX_KEY: &str = "item.minecraft.smithing_template";
const SMITHING_TEMPLATE_APPLIES_TO_TITLE_KEY: &str = "item.minecraft.smithing_template.applies_to";
const SMITHING_TEMPLATE_INGREDIENTS_TITLE_KEY: &str =
    "item.minecraft.smithing_template.ingredients";
const SMITHING_TEMPLATE_ARMOR_TRIM_APPLIES_TO_KEY: &str =
    "item.minecraft.smithing_template.armor_trim.applies_to";
const SMITHING_TEMPLATE_ARMOR_TRIM_INGREDIENTS_KEY: &str =
    "item.minecraft.smithing_template.armor_trim.ingredients";
const SMITHING_TEMPLATE_NETHERITE_UPGRADE_APPLIES_TO_KEY: &str =
    "item.minecraft.smithing_template.netherite_upgrade.applies_to";
const SMITHING_TEMPLATE_NETHERITE_UPGRADE_INGREDIENTS_KEY: &str =
    "item.minecraft.smithing_template.netherite_upgrade.ingredients";

#[cfg(test)]
thread_local! {
    static TOOLTIP_STACK_EXTREMES: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, usize::MAX)) };
}

#[cfg(test)]
const fn painting_variant(
    id: i32,
    key: &'static str,
    width: i32,
    height: i32,
    has_author: bool,
) -> VanillaPaintingVariant {
    VanillaPaintingVariant {
        id,
        resource_id: key,
        width,
        height,
        has_author,
    }
}

const INSTRUMENT_DESCRIPTION_STYLE: ComponentStyle = ComponentStyle {
    bold: None,
    italic: None,
    underlined: None,
    strikethrough: None,
    obfuscated: None,
    color: Some(TOOLTIP_GRAY_TEXT_COLOR),
    click_event: None,
    hover_event: None,
    font: None,
};
#[derive(Debug, Clone, Copy)]
struct PotionAttributeModifierLine {
    attribute_description_key: &'static str,
    amount: f64,
    operation_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TooltipTextArg {
    text: String,
    runs: Vec<HudComponentTextRun>,
}

impl TooltipTextArg {
    fn plain(text: String) -> Self {
        Self {
            runs: vec![HudComponentTextRun::plain(text.clone())],
            text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeItemTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
    /// Component draw runs for the line; concatenating the visual texts
    /// reproduces `text`. Unstyled lines carry a single default-font run with
    /// no colour override (the renderer then falls back to `tint`).
    pub runs: Vec<HudComponentTextRun>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NativeItemTooltipOptions<'a> {
    pub advanced: bool,
    pub creative: bool,
    pub peaceful: bool,
    pub map_data: Option<NativeItemMapTooltipData>,
    pub trim_material_keys: Option<&'a [String]>,
    pub trim_material_presentations: Option<&'a [Option<TrimMaterialPresentation>]>,
    pub trim_pattern_keys: Option<&'a [String]>,
    pub trim_pattern_presentations: Option<&'a [Option<TrimPatternPresentation>]>,
    pub instrument_keys: Option<&'a [String]>,
    pub instrument_presentations: Option<&'a [Option<InstrumentPresentation>]>,
    pub jukebox_song_keys: Option<&'a [String]>,
    pub jukebox_song_presentations: Option<&'a [Option<JukeboxSongPresentation>]>,
    pub banner_pattern_keys: Option<&'a [String]>,
    pub banner_pattern_presentations: Option<&'a [Option<BannerPatternPresentation>]>,
    pub painting_variant_presentations: Option<&'a [Option<PaintingVariantPresentation>]>,
    pub enchantment_keys: Option<&'a [String]>,
    pub enchantment_definitions: Option<&'a [Option<EnchantmentTooltipDefinition>]>,
    pub enchantment_attribute_modifiers: Option<&'a [EnchantmentAttributeModifier]>,
    pub attribute_keys: Option<&'a [String]>,
    pub player_base_attack_damage: Option<f64>,
    pub player_base_attack_speed: Option<f64>,
    pub gamemaster_permission: bool,
    pub item_disabled: bool,
    pub tooltip_tick_rate: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeItemMapTooltipData {
    pub scale: i8,
    pub locked: bool,
}

impl NativeItemTooltipLine {
    fn plain(text: String, tint: [f32; 4]) -> Self {
        let runs = vec![HudComponentTextRun::plain(text.clone())];
        Self { text, tint, runs }
    }
}

fn effective_tooltip_tick_rate(options: NativeItemTooltipOptions<'_>) -> f32 {
    match options.tooltip_tick_rate {
        Some(tick_rate) if tick_rate.is_finite() && tick_rate > 0.0 => tick_rate,
        _ => DEFAULT_TOOLTIP_TICKRATE,
    }
}

fn tooltip_text_color_for_tint(tint: [f32; 4]) -> Option<u32> {
    if tint == TOOLTIP_TEXT_BLUE {
        Some(TOOLTIP_BLUE_TEXT_COLOR)
    } else if tint == TOOLTIP_TEXT_DARK_GRAY {
        Some(TOOLTIP_DARK_GRAY_TEXT_COLOR)
    } else if tint == TOOLTIP_TEXT_DARK_PURPLE {
        Some(TOOLTIP_DARK_PURPLE_TEXT_COLOR)
    } else if tint == TOOLTIP_TEXT_GRAY {
        Some(TOOLTIP_GRAY_TEXT_COLOR)
    } else if tint == TOOLTIP_TEXT_RED {
        Some(TOOLTIP_RED_TEXT_COLOR)
    } else if tint == TOOLTIP_TEXT_WHITE {
        None
    } else {
        None
    }
}

fn charged_projectile_detail_tooltip_line(line: NativeItemTooltipLine) -> NativeItemTooltipLine {
    let mut runs = Vec::with_capacity(line.runs.len() + 1);
    runs.push(HudComponentTextRun {
        visual: HudStyledTextRun {
            text: "  ".to_string(),
            style: HudTextStyle::default(),
            color: Some(TOOLTIP_GRAY_TEXT_COLOR),
        },
        font: None,
    });
    let fallback_color = tooltip_text_color_for_tint(line.tint).unwrap_or(TOOLTIP_GRAY_TEXT_COLOR);
    runs.extend(line.runs.into_iter().map(|mut run| {
        if run.visual.color.is_none() {
            run.visual.color = Some(fallback_color);
        }
        run
    }));

    NativeItemTooltipLine {
        text: format!("  {}", line.text),
        tint: TOOLTIP_TEXT_GRAY,
        runs,
    }
}

/// Projects one flattened protocol run into the renderer's resolved run type,
/// first merging `base` under the run's own style (vanilla
/// `ComponentUtils.mergeStyles` / wrapper `withStyle` semantics: the run's own
/// keys win, unset keys inherit `base`).
fn hud_run_from_component(run: &StyledTextRun, base: &ComponentStyle) -> HudComponentTextRun {
    let style = run.style.apply_to(base);
    HudComponentTextRun {
        visual: HudStyledTextRun {
            text: run.text.clone(),
            style: HudTextStyle {
                bold: style.bold == Some(true),
                italic: style.italic == Some(true),
                underlined: style.underlined == Some(true),
                strikethrough: style.strikethrough == Some(true),
                obfuscated: style.obfuscated == Some(true),
            },
            color: style.color,
        },
        font: style.font.clone(),
    }
}

/// Styled-run projection with a plain-text fallback for data decoded before
/// the styled fields existed (or synthesized names): the fallback text becomes
/// a single unstyled run that still receives the `base` style merge.
fn hud_runs_from_component(
    runs: &[StyledTextRun],
    plain_fallback: &str,
    base: &ComponentStyle,
) -> Vec<HudComponentTextRun> {
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

fn shield_item_name_override(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> Option<String> {
    if resource_id != "minecraft:shield" {
        return None;
    }
    let color_id = component_patch.base_color_id?;
    let translation_key = format!("item.minecraft.shield.{}", vanilla_dye_color_name(color_id));
    Some(
        language
            .get(&translation_key)
            .unwrap_or(&translation_key)
            .to_string(),
    )
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
    if let Some(name) = shield_item_name_override(language, resource_id, component_patch) {
        return name;
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
    hover_name_source_runs_for_component_patch(language, resource_id, &stack.component_patch)
}

fn hover_name_source_runs_for_component_patch(
    language: &LanguageCatalog,
    resource_id: &str,
    patch: &DataComponentPatchSummary,
) -> (Vec<StyledTextRun>, bool) {
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
    if let Some(name) = shield_item_name_override(language, resource_id, patch) {
        return (plain(name), false);
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

fn display_name_arg_for_component_patch(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> TooltipTextArg {
    let (name_runs, name_is_custom) =
        hover_name_source_runs_for_component_patch(language, resource_id, component_patch);
    let rarity_color = item_rarity_color(item_rarity_for_stack(component_patch));
    let bracket_run = |text: &str| HudComponentTextRun {
        visual: HudStyledTextRun {
            text: text.to_string(),
            style: HudTextStyle::default(),
            color: Some(rarity_color),
        },
        font: None,
    };
    let name_wrapper = ComponentStyle {
        italic: name_is_custom.then_some(true),
        color: Some(rarity_color),
        ..ComponentStyle::default()
    };
    let mut runs = Vec::with_capacity(name_runs.len() + 2);
    runs.push(bracket_run("["));
    runs.extend(
        name_runs
            .iter()
            .map(|run| hud_run_from_component(run, &name_wrapper)),
    );
    runs.push(bracket_run("]"));
    TooltipTextArg {
        text: format!(
            "[{}]",
            hover_name_for_component_patch(language, resource_id, component_patch)
        ),
        runs,
    }
}

fn hover_name_arg_for_component_patch(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> TooltipTextArg {
    let (name_runs, _) =
        hover_name_source_runs_for_component_patch(language, resource_id, component_patch);
    TooltipTextArg {
        text: hover_name_for_component_patch(language, resource_id, component_patch),
        runs: name_runs
            .iter()
            .map(|run| hud_run_from_component(run, &ComponentStyle::default()))
            .collect(),
    }
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
    bees_present: bool,
    bees_count: usize,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !bees_present {
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
    options: NativeItemTooltipOptions<'_>,
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

    let Some(holder_id) = component_patch.instrument_id else {
        return;
    };
    if let Some(instrument_keys) = options.instrument_keys {
        let Some(index) = usize::try_from(holder_id).ok() else {
            return;
        };
        if let Some(Some(presentation)) = options
            .instrument_presentations
            .and_then(|presentations| presentations.get(index))
        {
            lines.push(NativeItemTooltipLine {
                text: presentation.description.clone(),
                tint: TOOLTIP_TEXT_GRAY,
                runs: hud_runs_from_component(
                    &presentation.description_styled,
                    &presentation.description,
                    &INSTRUMENT_DESCRIPTION_STYLE,
                ),
            });
            return;
        }
        let Some(instrument_key) = instrument_keys
            .get(index)
            .filter(|key| VanillaInstrumentIdentity::for_resource_id(key).is_some())
        else {
            return;
        };
        lines.push(NativeItemTooltipLine::plain(
            language
                .get_or_key(&description_key("instrument", instrument_key))
                .to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
        return;
    }

    let Some(instrument_key) = VanillaInstrumentIdentity::for_vanilla_registry_id(holder_id)
        .map(VanillaInstrumentIdentity::resource_id)
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
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for layer in layers.iter().take(6) {
        let translation_key = if let (Some(asset_id), Some(translation_key)) =
            (layer.asset_id.as_deref(), layer.translation_key.as_deref())
        {
            direct_banner_pattern_presentation(asset_id, translation_key)
                .map(|presentation| presentation.translation_key)
        } else if let Some(registry_id) = layer.registry_id {
            let index = usize::try_from(registry_id).ok();
            match options.banner_pattern_presentations {
                Some(presentations) => index
                    .and_then(|index| presentations.get(index))
                    .and_then(Option::as_ref)
                    .map(|presentation| presentation.translation_key.clone()),
                None => match options.banner_pattern_keys {
                    Some(keys) => index
                        .and_then(|index| keys.get(index))
                        .and_then(|resource_id| BannerPatternKind::for_resource_id(resource_id))
                        .map(|pattern| pattern.translation_key().to_string()),
                    None => BannerPatternKind::for_vanilla_registry_id(registry_id)
                        .map(|pattern| pattern.translation_key().to_string()),
                },
            }
        } else {
            None
        };
        let Some(translation_key) = translation_key else {
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

fn italic_gray_tooltip_line(text: String) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_GRAY,
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                text,
                style: HudTextStyle {
                    italic: true,
                    ..HudTextStyle::default()
                },
                color: Some(TOOLTIP_GRAY_TEXT_COLOR),
            },
            font: None,
        }],
    }
}

fn tropical_fish_pattern(packed_id: i32) -> (i32, &'static str) {
    let pattern = vanilla_tropical_fish_pattern(packed_id);
    (pattern.packed_id, pattern.serialized_name)
}

fn tropical_fish_common_variant_index(
    pattern_id: i32,
    base_color_id: i32,
    pattern_color_id: i32,
) -> Option<usize> {
    VANILLA_TROPICAL_FISH_COMMON_VARIANTS
        .iter()
        .position(|variant| {
            (
                variant.pattern_packed_id,
                variant.base_color_id,
                variant.pattern_color_id,
            ) == (pattern_id, base_color_id, pattern_color_id)
        })
}

fn dye_color_id_or_white(color_id: Option<i32>) -> i32 {
    normalize_vanilla_dye_color_id(color_id.unwrap_or_default())
}

fn dye_color_name(color_id: i32) -> &'static str {
    vanilla_dye_color_name(color_id)
}

fn tooltip_display_shows(component_patch: &DataComponentPatchSummary, type_id: i32) -> bool {
    !component_patch.tooltip_hide_tooltip
        && !component_patch
            .tooltip_hidden_component_type_ids
            .contains(&type_id)
}

fn push_item_specific_tooltip_lines(
    language: &LanguageCatalog,
    item_id: &str,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_DISC_FRAGMENT_5_ID,
    ) {
        lines.push(NativeItemTooltipLine::plain(
            language
                .get_or_key(DISC_FRAGMENT_5_DESCRIPTION_KEY)
                .to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    if let Some((applies_to_key, ingredients_key)) = smithing_template_tooltip_keys(item_id) {
        push_smithing_template_tooltip_lines(language, applies_to_key, ingredients_key, lines);
    }
    if item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_PAINTING_ID,
    ) && tooltip_display_shows(
        component_patch,
        component_id::VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    ) {
        push_painting_tooltip_lines(language, component_patch, options, lines);
    }
}

fn push_painting_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(variant) = component_patch.painting_variant_direct.as_ref() {
        push_painting_variant_tooltip_lines(language, variant, lines);
    } else if let Some(variant_id) = component_patch.painting_variant_id {
        if let Some(presentations) = options.painting_variant_presentations {
            let presentation = usize::try_from(variant_id)
                .ok()
                .and_then(|index| presentations.get(index))
                .and_then(Option::as_ref);
            if let Some(presentation) = presentation {
                push_painting_variant_presentation_tooltip_lines(language, presentation, lines);
            }
        } else if let Some(variant) = vanilla_painting_variant(variant_id) {
            push_vanilla_painting_variant_tooltip_lines(language, variant, lines);
        }
    } else if options.creative {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key(PAINTING_RANDOM_KEY).to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }
}

fn push_painting_variant_presentation_tooltip_lines(
    language: &LanguageCatalog,
    variant: &PaintingVariantPresentation,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(title_key) = variant.title_translation_key.as_deref() {
        lines.push(colored_component_tooltip_line(
            language.get_or_key(title_key).to_string(),
            TOOLTIP_YELLOW_TEXT_COLOR,
        ));
    } else {
        push_optional_component_tooltip_line(
            variant.title.as_deref(),
            Some(&variant.title_styled),
            lines,
        );
    }
    if let Some(author_key) = variant.author_translation_key.as_deref() {
        lines.push(colored_component_tooltip_line(
            language.get_or_key(author_key).to_string(),
            TOOLTIP_GRAY_TEXT_COLOR,
        ));
    } else {
        push_optional_component_tooltip_line(
            variant.author.as_deref(),
            Some(&variant.author_styled),
            lines,
        );
    }
    lines.push(NativeItemTooltipLine::plain(
        translate_with_two_args(
            language,
            PAINTING_DIMENSIONS_KEY,
            &variant.width.to_string(),
            &variant.height.to_string(),
        ),
        TOOLTIP_TEXT_WHITE,
    ));
}

fn push_vanilla_painting_variant_tooltip_lines(
    language: &LanguageCatalog,
    variant: &VanillaPaintingVariant,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(title_key) = painting_variant_translation_key(variant.resource_id, "title") {
        lines.push(colored_component_tooltip_line(
            language.get_or_key(&title_key).to_string(),
            TOOLTIP_YELLOW_TEXT_COLOR,
        ));
    }
    if variant.has_author {
        if let Some(author_key) = painting_variant_translation_key(variant.resource_id, "author") {
            lines.push(colored_component_tooltip_line(
                language.get_or_key(&author_key).to_string(),
                TOOLTIP_GRAY_TEXT_COLOR,
            ));
        }
    }
    lines.push(NativeItemTooltipLine::plain(
        translate_with_two_args(
            language,
            PAINTING_DIMENSIONS_KEY,
            &variant.width.to_string(),
            &variant.height.to_string(),
        ),
        TOOLTIP_TEXT_WHITE,
    ));
}

fn push_painting_variant_tooltip_lines(
    language: &LanguageCatalog,
    variant: &PaintingVariantSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    push_optional_component_tooltip_line(
        variant.title.as_deref(),
        variant.title_styled.as_deref(),
        lines,
    );
    push_optional_component_tooltip_line(
        variant.author.as_deref(),
        variant.author_styled.as_deref(),
        lines,
    );
    lines.push(NativeItemTooltipLine::plain(
        translate_with_two_args(
            language,
            PAINTING_DIMENSIONS_KEY,
            &variant.width.to_string(),
            &variant.height.to_string(),
        ),
        TOOLTIP_TEXT_WHITE,
    ));
}

fn push_optional_component_tooltip_line(
    text: Option<&str>,
    runs: Option<&[StyledTextRun]>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(text) = text {
        lines.push(NativeItemTooltipLine {
            text: text.to_string(),
            tint: TOOLTIP_TEXT_WHITE,
            runs: hud_runs_from_component(runs.unwrap_or(&[]), text, &ComponentStyle::default()),
        });
    }
}

fn colored_component_tooltip_line(text: String, color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_WHITE,
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                text,
                style: HudTextStyle::default(),
                color: Some(color),
            },
            font: None,
        }],
    }
}

fn push_operator_nbt_warning_tooltip_lines(
    language: &LanguageCatalog,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for (index, key) in [
        "item.op_warning.line1",
        "item.op_warning.line2",
        "item.op_warning.line3",
    ]
    .into_iter()
    .enumerate()
    {
        let text = language.get_or_key(key).to_string();
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_RED,
            runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun {
                    text,
                    style: HudTextStyle {
                        bold: index == 0,
                        ..HudTextStyle::default()
                    },
                    color: Some(TOOLTIP_RED_TEXT_COLOR),
                },
                font: None,
            }],
        });
    }
}

fn push_spawner_block_entity_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(entity_id) = component_patch.block_entity_spawn_entity_type.as_deref() {
        lines.push(NativeItemTooltipLine::plain(
            localized_entity_name(language, entity_id),
            TOOLTIP_TEXT_GRAY,
        ));
    } else {
        lines.push(NativeItemTooltipLine::plain(
            String::new(),
            TOOLTIP_TEXT_WHITE,
        ));
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key(SPAWNER_DESC1_KEY).to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
        lines.push(NativeItemTooltipLine::plain(
            format!(" {}", language.get_or_key(SPAWNER_DESC2_KEY)),
            TOOLTIP_TEXT_BLUE,
        ));
    }
}

fn localized_entity_name(language: &LanguageCatalog, resource_id: &str) -> String {
    language
        .get_or_key(&description_key("entity", resource_id))
        .to_string()
}

fn push_entity_data_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    peaceful: bool,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(entity_type_id) = component_patch.entity_data_entity_type_id else {
        return;
    };
    if peaceful && !vanilla_entity_type_allowed_in_peaceful(entity_type_id) {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key(SPAWN_EGG_PEACEFUL_KEY).to_string(),
            TOOLTIP_TEXT_RED,
        ));
    }
}

fn item_is_spawner(resource_id: &str) -> bool {
    item_types::vanilla_item_resource_id_matches_type(
        resource_id,
        item_types::VANILLA_ITEM_SPAWNER_ID,
    ) || item_types::vanilla_item_resource_id_matches_type(
        resource_id,
        item_types::VANILLA_ITEM_TRIAL_SPAWNER_ID,
    )
}

fn push_adventure_mode_predicate_tooltip_lines(
    language: &LanguageCatalog,
    registries: &RegistrySet,
    header_key: &str,
    predicate: &AdventureModePredicateSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    lines.push(NativeItemTooltipLine::plain(
        String::new(),
        TOOLTIP_TEXT_WHITE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key(header_key).to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    if predicate.unknown {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key(CAN_USE_UNKNOWN_KEY).to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
        return;
    }

    for block_id in &predicate.block_registry_ids {
        let Some(block_name) = registries.block_name_by_registry_id(*block_id) else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            localized_block_name(language, block_name),
            TOOLTIP_TEXT_DARK_GRAY,
        ));
    }
}

fn localized_block_name(language: &LanguageCatalog, resource_id: &str) -> String {
    language
        .get_or_key(&description_key("block", resource_id))
        .to_string()
}

fn smithing_template_tooltip_keys(item_id: &str) -> Option<(&'static str, &'static str)> {
    if item_types::vanilla_item_resource_id_matches_type(
        item_id,
        item_types::VANILLA_ITEM_NETHERITE_UPGRADE_SMITHING_TEMPLATE_ID,
    ) {
        return Some((
            SMITHING_TEMPLATE_NETHERITE_UPGRADE_APPLIES_TO_KEY,
            SMITHING_TEMPLATE_NETHERITE_UPGRADE_INGREDIENTS_KEY,
        ));
    }
    item_id
        .strip_prefix("minecraft:")
        .filter(|path| path.ends_with(ARMOR_TRIM_SMITHING_TEMPLATE_SUFFIX))
        .map(|_| {
            (
                SMITHING_TEMPLATE_ARMOR_TRIM_APPLIES_TO_KEY,
                SMITHING_TEMPLATE_ARMOR_TRIM_INGREDIENTS_KEY,
            )
        })
}

fn push_smithing_template_tooltip_lines(
    language: &LanguageCatalog,
    applies_to_key: &str,
    ingredients_key: &str,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key(SMITHING_TEMPLATE_SUFFIX_KEY)
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(NativeItemTooltipLine::plain(
        String::new(),
        TOOLTIP_TEXT_WHITE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key(SMITHING_TEMPLATE_APPLIES_TO_TITLE_KEY)
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(NativeItemTooltipLine::plain(
        format!(" {}", language.get_or_key(applies_to_key)),
        TOOLTIP_TEXT_BLUE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key(SMITHING_TEMPLATE_INGREDIENTS_TITLE_KEY)
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(NativeItemTooltipLine::plain(
        format!(" {}", language.get_or_key(ingredients_key)),
        TOOLTIP_TEXT_BLUE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn potion_effect(effect_id: i32, duration: i32, amplifier: i32) -> VanillaPotionEffect {
        VanillaPotionEffect {
            effect_id,
            duration,
            amplifier,
        }
    }

    #[test]
    fn tooltip_does_not_restore_component_type_id_aliases() {
        let source = include_str!("tooltip.rs");
        let aliases = source
            .lines()
            .filter(|line| line.trim_start().starts_with("const COMPONENT_"))
            .collect::<Vec<_>>();
        assert!(
            aliases.is_empty(),
            "tooltip restored component type id aliases:\n{}",
            aliases.join("\n")
        );
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
    fn operator_only_type_membership_stays_delegated_to_generated_catalogs() {
        let source = include_str!("tooltip.rs");
        for old_table in [
            ["const OP_ONLY_BLOCK_", "ENTITY_TYPE_IDS"].concat(),
            ["const OP_ONLY_", "ENTITY_TYPE_IDS"].concat(),
        ] {
            assert!(!source.contains(&old_table));
        }
        assert!(source.contains("vanilla_block_entity_type_only_op_can_set_nbt"));
        assert!(source.contains("vanilla_entity_type_only_op_can_set_nbt"));
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
        assert_eq!(
            enchantment_key_for_holder(13, None),
            Some("minecraft:sharpness")
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
            description_translation_key: Some("enchantment.example.custom".to_string()),
            description_translation_fallback: None,
            max_level: 3,
        })];
        let mut lines = Vec::new();

        push_enchantments_tooltip_lines(
            &language,
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
            &enchantments,
            Some(&keys),
            Some(&definitions),
            None,
            None,
            &mut lines,
        );
        assert_eq!(lines[0].text, "Localized Custom");
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
                sound_event: bbb_world::JukeboxSoundEvent::Reference {
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
                description_nbt: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                    name: "translate".to_string(),
                    value: NbtSummaryValue::String("jukebox_song.example.custom".to_string()),
                }]),
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
                let runtime =
                    NativeItemRuntime::empty_for_test().with_vanilla_item_registry_for_test();
                let mut projectile = ItemStackTemplateSummary {
                    item_id: 1,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                };
                for _ in 1..MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 10 {
                    projectile = ItemStackTemplateSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            charged_projectiles_items: vec![projectile],
                            ..DataComponentPatchSummary::default()
                        },
                    };
                }
                let stack = ItemStackSummary {
                    item_id: Some(1),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        charged_projectiles_items: vec![projectile],
                        ..DataComponentPatchSummary::default()
                    },
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

fn charged_projectile_group_tooltip_line(
    language: &LanguageCatalog,
    projectile_name: TooltipTextArg,
    count: usize,
) -> NativeItemTooltipLine {
    if count == 1 {
        translated_tooltip_line_with_args(
            language,
            "item.minecraft.crossbow.projectile.single",
            &[projectile_name],
            TOOLTIP_TEXT_WHITE,
        )
    } else {
        translated_tooltip_line_with_args(
            language,
            "item.minecraft.crossbow.projectile.multiple",
            &[TooltipTextArg::plain(count.to_string()), projectile_name],
            TOOLTIP_TEXT_WHITE,
        )
    }
}

fn item_container_more_tooltip_line(text: String) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_WHITE,
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                text,
                style: HudTextStyle {
                    italic: true,
                    ..HudTextStyle::default()
                },
                color: None,
            },
            font: None,
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
    tooltip_tick_rate: f32,
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
            1.0,
            tooltip_tick_rate,
        ),
        TOOLTIP_TEXT_BLUE,
    ));
}

fn push_potion_contents_tooltip_lines(
    language: &LanguageCatalog,
    component_patch: &DataComponentPatchSummary,
    duration_scale: f32,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let mut has_effect_entries = false;
    let mut attribute_modifiers = Vec::new();
    if let Some(potion_id) = component_patch.potion_id {
        if let Some(effects) = vanilla_potion(potion_id).map(|potion| potion.effects) {
            has_effect_entries |= !effects.is_empty();
            for effect in effects {
                push_potion_effect_tooltip_line(
                    language,
                    effect.effect_id,
                    effect.amplifier,
                    effect.duration,
                    duration_scale,
                    tooltip_tick_rate,
                    lines,
                );
                collect_potion_effect_attribute_modifiers(
                    effect.effect_id,
                    effect.amplifier,
                    &mut attribute_modifiers,
                );
            }
        }
    }
    has_effect_entries |= !component_patch.potion_custom_effects.is_empty();
    for effect in &component_patch.potion_custom_effects {
        push_potion_effect_tooltip_line(
            language,
            effect.effect_id,
            effect.amplifier,
            effect.duration,
            duration_scale,
            tooltip_tick_rate,
            lines,
        );
        collect_potion_effect_attribute_modifiers(
            effect.effect_id,
            effect.amplifier,
            &mut attribute_modifiers,
        );
    }
    if !has_effect_entries && potion_contents_component_present(component_patch) {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("effect.none").to_string(),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    push_potion_attribute_modifier_tooltip_lines(language, &attribute_modifiers, lines);
}

fn effective_potion_duration_scale(
    component_patch: &DataComponentPatchSummary,
    prototype_scale: Option<f32>,
) -> f32 {
    let type_id = component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID;
    if component_patch.removed_type_ids.contains(&type_id) {
        return 1.0;
    }
    if component_patch.added_type_ids.contains(&type_id) {
        return component_patch
            .potion_duration_scale_bits
            .map(f32::from_bits)
            .unwrap_or(1.0);
    }
    prototype_scale.unwrap_or(1.0)
}

fn potion_contents_component_present(component_patch: &DataComponentPatchSummary) -> bool {
    component_patch.potion_id.is_some()
        || component_patch.potion_custom_color.is_some()
        || component_patch.potion_custom_effect_count.is_some()
        || component_patch.potion_custom_name.is_some()
        || !component_patch.potion_custom_effects.is_empty()
}

fn push_potion_effect_tooltip_line(
    language: &LanguageCatalog,
    effect_id: i32,
    amplifier: i32,
    duration: i32,
    duration_scale: f32,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(effect_key) = vanilla_mob_effect_type(effect_id).map(|effect| effect.resource_id)
    else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        potion_effect_tooltip_text(
            language,
            effect_key,
            amplifier,
            duration,
            duration_scale,
            tooltip_tick_rate,
        ),
        mob_effect_tooltip_tint_for_id(effect_id),
    ));
}

fn collect_potion_effect_attribute_modifiers(
    effect_id: i32,
    amplifier: i32,
    lines: &mut Vec<PotionAttributeModifierLine>,
) {
    let amplifier_scale = f64::from(amplifier) + 1.0;
    lines.extend(
        vanilla_mob_effect_type(effect_id)
            .map(|effect| effect.attribute_modifiers)
            .unwrap_or(&[])
            .iter()
            .filter_map(|modifier| {
                Some(PotionAttributeModifierLine {
                    attribute_description_key: vanilla_attribute_type(modifier.attribute_id)?
                        .description_key,
                    amount: f64::from_bits(modifier.amount_bits) * amplifier_scale,
                    operation_id: modifier.operation_id,
                })
            }),
    );
}

fn push_potion_attribute_modifier_tooltip_lines(
    language: &LanguageCatalog,
    modifiers: &[PotionAttributeModifierLine],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if modifiers.is_empty() {
        return;
    }
    lines.push(NativeItemTooltipLine::plain(
        String::new(),
        TOOLTIP_TEXT_WHITE,
    ));
    lines.push(NativeItemTooltipLine::plain(
        language.get_or_key(POTION_WHEN_DRANK_KEY).to_string(),
        TOOLTIP_TEXT_DARK_PURPLE,
    ));

    for modifier in modifiers {
        push_potion_attribute_modifier_tooltip_line(language, modifier, lines);
    }
}

fn push_potion_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &PotionAttributeModifierLine,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if modifier.amount == 0.0 {
        return;
    }
    let mut display_amount = if modifier.operation_id
        == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
        || modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
    {
        modifier.amount * 100.0
    } else {
        modifier.amount
    };
    let (key_prefix, tint) = if modifier.amount > 0.0 {
        ("attribute.modifier.plus", TOOLTIP_TEXT_BLUE)
    } else {
        display_amount *= -1.0;
        ("attribute.modifier.take", TOOLTIP_TEXT_RED)
    };
    let amount = format_attribute_modifier_amount(display_amount);
    let attribute = language
        .get_or_key(modifier.attribute_description_key)
        .to_string();
    let text = translate_with_two_args(
        language,
        &format!("{key_prefix}.{}", modifier.operation_id),
        &amount,
        &attribute,
    )
    .replace("%%", "%");
    lines.push(NativeItemTooltipLine::plain(text, tint));
}

fn format_attribute_modifier_amount(amount: f64) -> String {
    let mut text = format!("{amount:.2}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        "0".to_string()
    } else {
        text
    }
}

fn push_stack_attribute_modifiers_tooltip_lines(
    language: &LanguageCatalog,
    modifiers: &[AttributeModifierSummary],
    enchantment_modifiers: &[EnchantmentAttributeModifier],
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for slot_group in VANILLA_EQUIPMENT_SLOT_GROUPS {
        let slot_id = slot_group.id;
        let slot_key = slot_group.serialized_name;
        let mut section_lines = Vec::new();
        for modifier in modifiers
            .iter()
            .filter(|modifier| modifier.slot_id == slot_id)
        {
            if modifier.display_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID {
                continue;
            }
            let Some(line) = stack_attribute_modifier_tooltip_line(language, modifier, options)
            else {
                continue;
            };
            section_lines.push(line);
        }
        section_lines.extend(
            enchantment_modifiers
                .iter()
                .filter(|modifier| modifier.slot == slot_key)
                .filter_map(|modifier| {
                    enchantment_attribute_modifier_tooltip_line(language, modifier, options)
                }),
        );
        if !section_lines.is_empty() {
            lines.push(NativeItemTooltipLine::plain(
                String::new(),
                TOOLTIP_TEXT_WHITE,
            ));
            lines.push(NativeItemTooltipLine::plain(
                language
                    .get_or_key(&format!("item.modifiers.{slot_key}"))
                    .to_string(),
                TOOLTIP_TEXT_GRAY,
            ));
            lines.extend(section_lines);
        }
    }
}

fn stack_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &AttributeModifierSummary,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    if modifier.display_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID {
        return modifier
            .display_text
            .as_ref()
            .map(|text| NativeItemTooltipLine {
                text: text.clone(),
                tint: TOOLTIP_TEXT_WHITE,
                runs: hud_runs_from_component(
                    modifier.display_text_styled.as_deref().unwrap_or_default(),
                    text,
                    &ComponentStyle::default(),
                ),
            });
    }

    let attribute = tooltip_attribute(modifier.attribute_id, options.attribute_keys)?;
    numeric_attribute_modifier_tooltip_line(
        language,
        attribute,
        &modifier.modifier_id,
        f64::from_bits(modifier.amount_bits),
        modifier.operation_id,
        options,
    )
}

fn enchantment_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    modifier: &EnchantmentAttributeModifier,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    numeric_attribute_modifier_tooltip_line(
        language,
        vanilla_attribute_type_for_resource_id(&modifier.attribute)?,
        &modifier.modifier_id,
        modifier.amount,
        modifier.operation_id,
        options,
    )
}

fn numeric_attribute_modifier_tooltip_line(
    language: &LanguageCatalog,
    attribute: &'static AttributeTypeCatalogEntry,
    modifier_id: &str,
    raw_amount: f64,
    operation_id: i32,
    options: NativeItemTooltipOptions<'_>,
) -> Option<NativeItemTooltipLine> {
    let (amount, display_with_base) = match modifier_id {
        "minecraft:base_attack_damage" => options
            .player_base_attack_damage
            .map(|base| (raw_amount + base, true))
            .unwrap_or((raw_amount, false)),
        "minecraft:base_attack_speed" => options
            .player_base_attack_speed
            .map(|base| (raw_amount + base, true))
            .unwrap_or((raw_amount, false)),
        _ => (raw_amount, false),
    };
    if amount == 0.0 && !display_with_base {
        return None;
    }
    let mut display_amount = if operation_id
        == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
        || operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
    {
        amount * 100.0
    } else if attribute.resource_id == "minecraft:knockback_resistance" {
        amount * 10.0
    } else {
        amount
    };
    if display_with_base {
        let text = translate_with_two_args(
            language,
            &format!("attribute.modifier.equals.{operation_id}"),
            &format_attribute_modifier_amount(display_amount),
            language.get_or_key(attribute.description_key),
        )
        .replace("%%", "%");
        return Some(NativeItemTooltipLine::plain(
            format!(" {text}"),
            TOOLTIP_TEXT_DARK_GREEN,
        ));
    }

    let (key_prefix, tint) = if amount > 0.0 {
        (
            "attribute.modifier.plus",
            stack_attribute_modifier_tint(attribute.sentiment, true),
        )
    } else {
        display_amount *= -1.0;
        (
            "attribute.modifier.take",
            stack_attribute_modifier_tint(attribute.sentiment, false),
        )
    };
    let amount = format_attribute_modifier_amount(display_amount);
    let attribute_name = language.get_or_key(attribute.description_key).to_string();
    let text = translate_with_two_args(
        language,
        &format!("{key_prefix}.{operation_id}"),
        &amount,
        &attribute_name,
    )
    .replace("%%", "%");
    Some(NativeItemTooltipLine::plain(text, tint))
}

fn tooltip_attribute(
    attribute_id: i32,
    attribute_keys: Option<&[String]>,
) -> Option<&'static AttributeTypeCatalogEntry> {
    match attribute_keys {
        Some(keys) => usize::try_from(attribute_id)
            .ok()
            .and_then(|index| keys.get(index))
            .and_then(|key| vanilla_attribute_type_for_resource_id(key)),
        None => vanilla_attribute_type(attribute_id),
    }
}

fn stack_attribute_modifier_tint(sentiment: AttributeSentiment, value_increase: bool) -> [f32; 4] {
    match sentiment {
        AttributeSentiment::Positive => {
            if value_increase {
                TOOLTIP_TEXT_BLUE
            } else {
                TOOLTIP_TEXT_RED
            }
        }
        AttributeSentiment::Neutral => TOOLTIP_TEXT_GRAY,
        AttributeSentiment::Negative => {
            if value_increase {
                TOOLTIP_TEXT_RED
            } else {
                TOOLTIP_TEXT_BLUE
            }
        }
    }
}

fn push_suspicious_stew_tooltip_lines(
    language: &LanguageCatalog,
    effects: &[SuspiciousStewEffectSummary],
    creative: bool,
    tooltip_tick_rate: f32,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if !creative {
        return;
    }
    for effect in effects {
        let Some(effect_key) =
            vanilla_mob_effect_type(effect.effect_id).map(|effect| effect.resource_id)
        else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            potion_effect_tooltip_text(
                language,
                effect_key,
                0,
                effect.duration,
                1.0,
                tooltip_tick_rate,
            ),
            mob_effect_tooltip_tint_for_id(effect.effect_id),
        ));
    }
}

pub(super) fn potion_effect_tooltip_text(
    language: &LanguageCatalog,
    effect_key: &str,
    amplifier: i32,
    duration_ticks: i32,
    duration_scale: f32,
    tooltip_tick_rate: f32,
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
            format_tick_duration(
                ((duration_ticks as f32) * duration_scale).floor() as i32,
                tooltip_tick_rate,
            )
        };
        effect = translate_with_two_args(language, "potion.withDuration", &effect, &duration);
    }

    effect
}

fn mob_effect_tooltip_tint_for_id(effect_id: i32) -> [f32; 4] {
    match vanilla_mob_effect_type(effect_id).map(|effect| effect.category) {
        Some(MobEffectCategory::Harmful) => TOOLTIP_TEXT_RED,
        Some(MobEffectCategory::Beneficial | MobEffectCategory::Neutral) => TOOLTIP_TEXT_BLUE,
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
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "item.color", &dyed_color_tooltip_hex(dyed_color)),
            TOOLTIP_TEXT_GRAY,
        ));
    } else {
        let text = language.get_or_key("item.dyed").to_string();
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_GRAY,
            runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun {
                    text,
                    style: HudTextStyle {
                        italic: true,
                        ..HudTextStyle::default()
                    },
                    color: Some(0xAA_AA_AA),
                },
                font: None,
            }],
        });
    }
}

fn dyed_color_tooltip_hex(dyed_color: i32) -> String {
    format!("#{:06X}", dyed_color as u32)
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
    language: &LanguageCatalog,
    songs: &JukeboxSongRegistry,
    song_id: Option<i32>,
    song: Option<&JukeboxSongSummary>,
    session_keys: Option<&[String]>,
    session_presentations: Option<&[Option<JukeboxSongPresentation>]>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(song) = song {
        push_jukebox_description(
            language,
            &song.description,
            song.description_styled.as_deref().unwrap_or(&[]),
            song.description_nbt.as_deref(),
            lines,
        );
        return;
    }
    let Some(song_id) = song_id else {
        return;
    };
    let Some(index) = usize::try_from(song_id).ok() else {
        return;
    };
    if let Some(session_presentations) = session_presentations {
        if let Some(Some(song)) = session_presentations.get(index) {
            push_jukebox_description(
                language,
                &song.description,
                &song.description_styled,
                Some(&song.description_nbt),
                lines,
            );
            return;
        }
    }
    let song_key = if session_presentations.is_some() {
        session_keys
            .and_then(|keys| keys.get(index))
            .filter(|key| !key.is_empty())
            .map(String::as_str)
    } else {
        songs.song_id(song_id)
    };
    let Some(song_key) = song_key else {
        return;
    };
    let description = language
        .get_or_key(&description_key("jukebox_song", song_key))
        .to_string();
    lines.push(NativeItemTooltipLine::plain(description, TOOLTIP_TEXT_GRAY));
}

fn push_jukebox_description(
    language: &LanguageCatalog,
    description: &str,
    description_styled: &[StyledTextRun],
    description_nbt: Option<&NbtSummaryValue>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let (text, styled) =
        localized_component_description(language, description, description_styled, description_nbt);
    let base = ComponentStyle {
        color: Some(TOOLTIP_GRAY_TEXT_COLOR),
        ..ComponentStyle::default()
    };
    lines.push(NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_GRAY,
        runs: hud_runs_from_component(&styled, &text, &base),
    });
}

fn localized_component_description(
    language: &LanguageCatalog,
    description: &str,
    description_styled: &[StyledTextRun],
    description_nbt: Option<&NbtSummaryValue>,
) -> (String, Vec<StyledTextRun>) {
    let mut text = description.to_string();
    let mut styled = description_styled.to_vec();
    if let Some((translation_key, fallback)) =
        description_nbt.and_then(component_translation_key_and_fallback)
    {
        let localized = language
            .get(translation_key)
            .or(fallback)
            .unwrap_or(translation_key);
        let encoded_text = fallback
            .filter(|fallback| text.contains(fallback))
            .unwrap_or(translation_key);
        if let Some(run) = styled.iter_mut().find(|run| run.text == encoded_text) {
            run.text = localized.to_string();
            text = styled.iter().map(|run| run.text.as_str()).collect();
        } else if text.contains(encoded_text) {
            text = text.replacen(encoded_text, localized, 1);
        }
    }
    (text, styled)
}

fn component_translation_key_and_fallback(value: &NbtSummaryValue) -> Option<(&str, Option<&str>)> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let translation_key = entries.iter().find_map(|entry| {
        (entry.name == "translate")
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(value) => Some(value.as_str()),
                _ => None,
            })
    })?;
    let fallback = entries.iter().find_map(|entry| {
        (entry.name == "fallback")
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(value) => Some(value.as_str()),
                _ => None,
            })
    });
    Some((translation_key, fallback))
}

fn push_armor_trim_tooltip_lines(
    language: &LanguageCatalog,
    material_id: Option<i32>,
    material: Option<&TrimMaterialSummary>,
    pattern_id: Option<i32>,
    pattern: Option<&TrimPatternSummary>,
    material_keys: Option<&[String]>,
    material_presentations: Option<&[Option<TrimMaterialPresentation>]>,
    pattern_keys: Option<&[String]>,
    pattern_presentations: Option<&[Option<TrimPatternPresentation>]>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(material_tooltip) = armor_trim_material_tooltip(
        language,
        material_id,
        material,
        material_keys,
        material_presentations,
    ) else {
        return;
    };
    let Some(pattern_description) = armor_trim_pattern_description(
        language,
        pattern_id,
        pattern,
        pattern_keys,
        pattern_presentations,
    ) else {
        return;
    };
    lines.push(NativeItemTooltipLine::plain(
        language
            .get_or_key("item.minecraft.smithing_template.upgrade")
            .to_string(),
        TOOLTIP_TEXT_GRAY,
    ));
    lines.push(armor_trim_description_line(
        &pattern_description,
        &material_tooltip.style,
    ));
    lines.push(armor_trim_description_line(
        &material_tooltip.description,
        &ComponentStyle::default(),
    ));
}

struct ArmorTrimMaterialTooltip {
    description: ArmorTrimDescription,
    style: ComponentStyle,
}

struct ArmorTrimDescription {
    text: String,
    runs: Vec<StyledTextRun>,
}

fn armor_trim_material_tooltip(
    language: &LanguageCatalog,
    material_id: Option<i32>,
    material: Option<&TrimMaterialSummary>,
    material_keys: Option<&[String]>,
    material_presentations: Option<&[Option<TrimMaterialPresentation>]>,
) -> Option<ArmorTrimMaterialTooltip> {
    if let Some(material) = material {
        let material = direct_trim_material_presentation(material)?;
        let description = armor_trim_component_description(
            language,
            &material.description,
            &material.description_styled,
            material.description_nbt.as_ref(),
        );
        let style = armor_trim_material_style(&description.runs);
        return Some(ArmorTrimMaterialTooltip { description, style });
    }
    let material_id = material_id?;
    let index = usize::try_from(material_id).ok()?;
    if let Some(Some(material)) =
        material_presentations.and_then(|presentations| presentations.get(index))
    {
        let description = armor_trim_component_description(
            language,
            &material.description,
            &material.description_styled,
            material.description_nbt.as_ref(),
        );
        let style = armor_trim_material_style(&description.runs);
        return Some(ArmorTrimMaterialTooltip { description, style });
    }
    let key = match material_keys {
        Some(keys) => keys.get(index)?.as_str(),
        None => VanillaTrimMaterialIdentity::for_vanilla_registry_id(material_id)?.resource_id(),
    };
    let material = VanillaTrimMaterialIdentity::for_resource_id(key)?;
    let text = language
        .get_or_key(&description_key("trim_material", key))
        .to_string();
    let style = ComponentStyle {
        color: Some(material.description_color()),
        ..ComponentStyle::default()
    };
    Some(ArmorTrimMaterialTooltip {
        description: ArmorTrimDescription {
            text: text.clone(),
            runs: vec![StyledTextRun {
                text,
                style: style.clone(),
            }],
        },
        style,
    })
}

fn armor_trim_pattern_description(
    language: &LanguageCatalog,
    pattern_id: Option<i32>,
    pattern: Option<&TrimPatternSummary>,
    pattern_keys: Option<&[String]>,
    pattern_presentations: Option<&[Option<TrimPatternPresentation>]>,
) -> Option<ArmorTrimDescription> {
    if let Some(pattern) = pattern {
        let pattern = direct_trim_pattern_presentation(pattern)?;
        return Some(armor_trim_component_description(
            language,
            &pattern.description,
            &pattern.description_styled,
            pattern.description_nbt.as_ref(),
        ));
    }
    let pattern_id = pattern_id?;
    let index = usize::try_from(pattern_id).ok()?;
    if let Some(Some(pattern)) =
        pattern_presentations.and_then(|presentations| presentations.get(index))
    {
        return Some(armor_trim_component_description(
            language,
            &pattern.description,
            &pattern.description_styled,
            pattern.description_nbt.as_ref(),
        ));
    }
    let key = match pattern_keys {
        Some(keys) => keys.get(index)?.as_str(),
        None => VanillaTrimPatternIdentity::for_vanilla_registry_id(pattern_id)?.resource_id(),
    };
    let text = language
        .get_or_key(&description_key("trim_pattern", key))
        .to_string();
    Some(ArmorTrimDescription {
        text: text.clone(),
        runs: vec![StyledTextRun {
            text,
            style: ComponentStyle::default(),
        }],
    })
}

fn armor_trim_component_description(
    language: &LanguageCatalog,
    text: &str,
    styled: &[StyledTextRun],
    nbt: Option<&NbtSummaryValue>,
) -> ArmorTrimDescription {
    let styled = if styled.is_empty() {
        vec![StyledTextRun {
            text: text.to_string(),
            style: ComponentStyle::default(),
        }]
    } else {
        styled.to_vec()
    };
    let (text, runs) = localized_component_description(language, text, &styled, nbt);
    ArmorTrimDescription { text, runs }
}

fn armor_trim_material_style(runs: &[StyledTextRun]) -> ComponentStyle {
    runs.iter()
        .find(|run| !run.text.is_empty())
        .map(|run| run.style.clone())
        .unwrap_or_default()
}

fn armor_trim_description_line(
    description: &ArmorTrimDescription,
    base: &ComponentStyle,
) -> NativeItemTooltipLine {
    let mut runs = Vec::with_capacity(description.runs.len() + 1);
    runs.push(HudComponentTextRun::plain(" "));
    runs.extend(hud_runs_from_component(
        &description.runs,
        &description.text,
        base,
    ));
    NativeItemTooltipLine {
        text: format!(" {}", description.text),
        tint: TOOLTIP_TEXT_WHITE,
        runs,
    }
}

fn push_enchantments_tooltip_lines(
    language: &LanguageCatalog,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: Option<&[String]>,
    enchantment_definitions: Option<&[Option<EnchantmentTooltipDefinition>]>,
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
            enchantment_key_for_holder(enchantment.holder_id, enchantment_keys).map(|key| {
                (
                    index,
                    enchantment,
                    key,
                    enchantment_definition_for_holder(
                        enchantment.holder_id,
                        enchantment_definitions,
                    ),
                )
            })
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

    for (index, enchantment, key, definition) in resolved {
        if !emitted[index] {
            push_enchantment_tooltip_line(
                language,
                key,
                enchantment.level,
                definition,
                enchantment_tags,
                lines,
            );
        }
    }
}

fn push_matching_ordered_enchantments(
    language: &LanguageCatalog,
    resolved: &[(
        usize,
        &ItemEnchantmentSummary,
        &str,
        Option<&EnchantmentTooltipDefinition>,
    )],
    emitted: &mut [bool],
    order_key: &str,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    for (index, enchantment, key, definition) in resolved {
        if !emitted[*index] && *key == order_key && enchantment.level > 0 {
            push_enchantment_tooltip_line(
                language,
                key,
                enchantment.level,
                *definition,
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
    definition: Option<&EnchantmentTooltipDefinition>,
    enchantment_tags: Option<&TagCatalog>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let tint = if enchantment_is_curse(enchantment_key, enchantment_tags) {
        TOOLTIP_TEXT_RED
    } else {
        TOOLTIP_TEXT_GRAY
    };
    let base = ComponentStyle {
        color: tooltip_text_color_for_tint(tint),
        ..ComponentStyle::default()
    };
    let (mut text, mut component_runs) =
        enchantment_description(language, enchantment_key, definition);
    let max_level = definition
        .map(|definition| definition.max_level)
        .or_else(|| {
            VanillaEnchantmentIdentity::for_resource_id(enchantment_key)
                .map(VanillaEnchantmentIdentity::max_level)
        })
        .unwrap_or(1);
    if level != 1 || max_level != 1 {
        let appended_style = component_runs
            .first()
            .map(|run| run.style.clone())
            .unwrap_or_default();
        let level_text = language
            .get_or_key(&format!("enchantment.level.{level}"))
            .to_string();
        text.push(' ');
        text.push_str(&level_text);
        component_runs.push(StyledTextRun {
            text: " ".to_string(),
            style: appended_style.clone(),
        });
        component_runs.push(StyledTextRun {
            text: level_text,
            style: appended_style,
        });
    }
    let runs = hud_runs_from_component(&component_runs, &text, &base);
    lines.push(NativeItemTooltipLine { text, tint, runs });
}

fn enchantment_description(
    language: &LanguageCatalog,
    enchantment_key: &str,
    definition: Option<&EnchantmentTooltipDefinition>,
) -> (String, Vec<StyledTextRun>) {
    let Some(definition) = definition else {
        let text = language
            .get_or_key(&description_key("enchantment", enchantment_key))
            .to_string();
        return (
            text.clone(),
            vec![StyledTextRun {
                text,
                style: ComponentStyle::default(),
            }],
        );
    };
    let mut text = definition.description.clone();
    let mut runs = definition.description_styled.clone();
    if let Some(translation_key) = definition.description_translation_key.as_deref() {
        let localized = language
            .get(translation_key)
            .or(definition.description_translation_fallback.as_deref())
            .unwrap_or(translation_key);
        let encoded_text = definition
            .description_translation_fallback
            .as_deref()
            .filter(|fallback| text.contains(fallback))
            .unwrap_or(translation_key);
        if let Some(run) = runs.iter_mut().find(|run| run.text == encoded_text) {
            run.text = localized.to_string();
            text = runs.iter().map(|run| run.text.as_str()).collect();
        } else if text.contains(encoded_text) {
            text = text.replacen(encoded_text, localized, 1);
        }
    }
    if runs.is_empty() {
        runs.push(StyledTextRun {
            text: text.clone(),
            style: ComponentStyle::default(),
        });
    }
    (text, runs)
}

fn enchantment_key_for_holder<'a>(
    holder_id: i32,
    enchantment_keys: Option<&'a [String]>,
) -> Option<&'a str> {
    if let Some(enchantment_keys) = enchantment_keys {
        let index = usize::try_from(holder_id).ok()?;
        return enchantment_keys.get(index).map(String::as_str);
    }
    VanillaEnchantmentIdentity::for_vanilla_registry_id(holder_id)
        .map(VanillaEnchantmentIdentity::resource_id)
}

fn enchantment_definition_for_holder(
    holder_id: i32,
    definitions: Option<&[Option<EnchantmentTooltipDefinition>]>,
) -> Option<&EnchantmentTooltipDefinition> {
    definitions?.get(usize::try_from(holder_id).ok()?)?.as_ref()
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
            shape.as_str()
        ))
        .to_string()
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

fn translated_tooltip_line_with_args(
    language: &LanguageCatalog,
    key: &str,
    args: &[TooltipTextArg],
    tint: [f32; 4],
) -> NativeItemTooltipLine {
    let template = language.get_or_key(key);
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut implicit_arg_index = 0usize;
    let mut index = 0usize;
    let bytes = template.as_bytes();

    while let Some(relative_percent) = template[index..].find('%') {
        let percent = index + relative_percent;
        push_plain_tooltip_template_segment(&mut text, &mut runs, &template[index..percent]);
        let mut cursor = percent + 1;
        if bytes.get(cursor) == Some(&b'%') {
            push_plain_tooltip_template_segment(&mut text, &mut runs, "%");
            index = cursor + 1;
            continue;
        }

        let digits_start = cursor;
        while bytes.get(cursor).is_some_and(|byte| byte.is_ascii_digit()) {
            cursor += 1;
        }
        let explicit_arg_index = if cursor > digits_start && bytes.get(cursor) == Some(&b'$') {
            let index_text = &template[digits_start..cursor];
            cursor += 1;
            index_text
                .parse::<usize>()
                .ok()
                .and_then(|value| value.checked_sub(1))
        } else {
            None
        };

        if bytes.get(cursor) == Some(&b's') {
            let arg_index = explicit_arg_index.unwrap_or_else(|| {
                let current = implicit_arg_index;
                implicit_arg_index += 1;
                current
            });
            if let Some(arg) = args.get(arg_index) {
                text.push_str(&arg.text);
                runs.extend(arg.runs.iter().cloned());
            } else {
                push_plain_tooltip_template_segment(
                    &mut text,
                    &mut runs,
                    &template[percent..=cursor],
                );
            }
            index = cursor + 1;
        } else {
            push_plain_tooltip_template_segment(&mut text, &mut runs, "%");
            index = percent + 1;
        }
    }
    push_plain_tooltip_template_segment(&mut text, &mut runs, &template[index..]);

    NativeItemTooltipLine { text, tint, runs }
}

fn push_plain_tooltip_template_segment(
    text: &mut String,
    runs: &mut Vec<HudComponentTextRun>,
    segment: &str,
) {
    if segment.is_empty() {
        return;
    }
    text.push_str(segment);
    runs.push(HudComponentTextRun::plain(segment));
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
    /// Formats `Enchantment.getFullname` for a synchronized registry holder.
    ///
    /// The caller supplies the holder resource id and decoded definition so
    /// registry ownership remains outside the item runtime; localization,
    /// curse tint, Component fonts, and level suffixes reuse the canonical
    /// item-tooltip path.
    pub fn enchantment_name_tooltip_line(
        &self,
        resource_id: &str,
        level: i32,
        definition: Option<&EnchantmentTooltipDefinition>,
    ) -> NativeItemTooltipLine {
        let mut lines = Vec::with_capacity(1);
        push_enchantment_tooltip_line(
            &self.language,
            resource_id,
            level,
            definition,
            self.enchantment_tags.as_ref(),
            &mut lines,
        );
        lines
            .pop()
            .expect("enchantment formatter always emits one line")
    }

    fn push_stack_detail_tooltip_lines(
        &self,
        item_id: &str,
        protocol_id: i32,
        component_patch: &DataComponentPatchSummary,
        options: NativeItemTooltipOptions<'_>,
        item_stack_template_depth: usize,
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        #[cfg(test)]
        {
            let marker = 0u8;
            let address = std::ptr::from_ref(&marker) as usize;
            TOOLTIP_STACK_EXTREMES.with(|cell| {
                let (top, bottom) = cell.get();
                cell.set((top.max(address), bottom.min(address)));
            });
        }
        let shows = |type_id| tooltip_display_shows(component_patch, type_id);
        let tooltip_tick_rate = effective_tooltip_tick_rate(options);
        push_item_specific_tooltip_lines(&self.language, item_id, component_patch, options, lines);
        if shows(component_id::VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID) {
            push_tropical_fish_tooltip_lines(&self.language, component_patch, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID) {
            push_instrument_tooltip_lines(&self.language, component_patch, options, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID) {
            push_map_id_tooltip_lines(&self.language, component_patch, options, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_BEES_TYPE_ID) {
            push_bees_tooltip_lines(
                &self.language,
                component_patch.bees_present,
                component_patch.bees_count,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID) {
            push_container_loot_tooltip_lines(
                &self.language,
                component_patch.container_loot,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID) {
            self.push_container_items_tooltip_lines(&component_patch.container_items, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID) {
            push_banner_pattern_tooltip_lines(
                &self.language,
                &component_patch.banner_pattern_layers,
                options,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID) {
            self.push_pot_decorations_tooltip_lines(
                &component_patch.pot_decorations_item_ids,
                lines,
            );
        }
        if let Some(book) = component_patch
            .written_book
            .as_ref()
            .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID))
        {
            push_written_book_tooltip_lines(&self.language, book, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID) {
            self.push_charged_projectiles_tooltip_lines(
                &component_patch.charged_projectiles_items,
                options,
                item_stack_template_depth,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID) {
            push_fireworks_tooltip_lines(
                &self.language,
                component_patch.fireworks_flight_duration,
                &component_patch.fireworks_explosions,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID) {
            push_firework_explosion_tooltip_lines(&self.language, component_patch, lines);
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID) {
            let duration_scale = effective_potion_duration_scale(
                component_patch,
                self.default_potion_duration_scale_for_resource_id(item_id),
            );
            push_potion_contents_tooltip_lines(
                &self.language,
                component_patch,
                duration_scale,
                tooltip_tick_rate,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID) {
            push_jukebox_playable_tooltip_lines(
                &self.language,
                &self.jukebox_songs,
                component_patch.jukebox_song_id,
                component_patch.jukebox_direct_song.as_deref(),
                options.jukebox_song_keys,
                options.jukebox_song_presentations,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID) {
            push_armor_trim_tooltip_lines(
                &self.language,
                component_patch.armor_trim_material_id,
                component_patch.armor_trim_material_direct.as_deref(),
                component_patch.armor_trim_pattern_id,
                component_patch.armor_trim_pattern_direct.as_deref(),
                options.trim_material_keys,
                options.trim_material_presentations,
                options.trim_pattern_keys,
                options.trim_pattern_presentations,
                lines,
            );
        }
        let enchantment_tags = self.enchantment_tags.as_ref();
        let tooltip_order =
            enchantment_tags.and_then(|tags| tags.values("minecraft:tooltip_order"));
        if shows(component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID) {
            push_enchantments_tooltip_lines(
                &self.language,
                &component_patch.stored_enchantments,
                options.enchantment_keys,
                options.enchantment_definitions,
                tooltip_order,
                enchantment_tags,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID) {
            push_enchantments_tooltip_lines(
                &self.language,
                &component_patch.enchantments,
                options.enchantment_keys,
                options.enchantment_definitions,
                tooltip_order,
                enchantment_tags,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID) {
            push_dyed_color_tooltip_lines(
                &self.language,
                component_patch.dyed_color,
                options.advanced,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID) {
            push_profile_tooltip_lines(&self.language, component_patch.profile.as_ref(), lines);
        }
        // Vanilla `ItemLore.styledLines`: every lore line gets `LORE_STYLE`
        // (DARK_PURPLE + italic) merged under its own style keys.
        if shows(component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID) {
            lines.extend(
                component_patch
                    .lore
                    .iter()
                    .enumerate()
                    .map(|(index, text)| NativeItemTooltipLine {
                        text: text.clone(),
                        tint: TOOLTIP_TEXT_DARK_PURPLE,
                        runs: hud_runs_from_component(
                            component_patch
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
        if shows(component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID) {
            let default_modifiers =
                self.default_attribute_modifiers_for_resource_id(item_id, options.attribute_keys);
            let modifiers =
                effective_attribute_modifiers(Some(component_patch), default_modifiers.as_slice())
                    .unwrap_or_default();
            push_stack_attribute_modifiers_tooltip_lines(
                &self.language,
                modifiers,
                options.enchantment_attribute_modifiers.unwrap_or_default(),
                options,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID) {
            push_intangible_projectile_tooltip_line(
                &self.language,
                component_patch.intangible_projectile,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID)
            && component_patch.unbreakable
        {
            lines.push(NativeItemTooltipLine::plain(
                self.language.get_or_key("item.unbreakable").to_string(),
                TOOLTIP_TEXT_BLUE,
            ));
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID) {
            push_ominous_bottle_tooltip_lines(
                &self.language,
                component_patch.ominous_bottle_amplifier,
                tooltip_tick_rate,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID) {
            push_suspicious_stew_tooltip_lines(
                &self.language,
                &component_patch.suspicious_stew_effects,
                options.creative,
                tooltip_tick_rate,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID) {
            push_block_state_tooltip_lines(
                &self.language,
                &component_patch.block_state_properties,
                lines,
            );
        }
        if shows(component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID) {
            push_entity_data_tooltip_lines(
                &self.language,
                component_patch,
                options.peaceful,
                lines,
            );
        }
        if item_is_spawner(item_id)
            && shows(component_id::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID)
        {
            push_spawner_block_entity_tooltip_lines(&self.language, component_patch, lines);
        }
        if (shows(component_id::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID)
            && component_patch.can_break.is_some())
            || (shows(component_id::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID)
                && component_patch.can_place_on.is_some())
        {
            let block_registries = RegistrySet::vanilla_26_1();
            if let Some(predicate) = component_patch
                .can_break
                .as_ref()
                .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID))
            {
                push_adventure_mode_predicate_tooltip_lines(
                    &self.language,
                    &block_registries,
                    CAN_BREAK_HEADER_KEY,
                    predicate,
                    lines,
                );
            }
            if let Some(predicate) = component_patch
                .can_place_on
                .as_ref()
                .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID))
            {
                push_adventure_mode_predicate_tooltip_lines(
                    &self.language,
                    &block_registries,
                    CAN_PLACE_HEADER_KEY,
                    predicate,
                    lines,
                );
            }
        }
        if options.advanced {
            push_advanced_tooltip_lines(
                &self.language,
                item_id,
                component_patch,
                self.default_max_damage_for_protocol_id(protocol_id),
                self.default_component_type_ids_for_resource_id(item_id),
                shows(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID),
                lines,
            );
        }
        if options.item_disabled {
            lines.push(NativeItemTooltipLine::plain(
                self.language.get_or_key("item.disabled").to_string(),
                TOOLTIP_TEXT_RED,
            ));
        }
        if self.should_print_operator_nbt_warning(item_id, component_patch, options) {
            push_operator_nbt_warning_tooltip_lines(&self.language, lines);
        }
    }

    fn should_print_operator_nbt_warning(
        &self,
        item_id: &str,
        component_patch: &DataComponentPatchSummary,
        options: NativeItemTooltipOptions<'_>,
    ) -> bool {
        if !options.gamemaster_permission {
            return false;
        }
        let Some(registry) = self.registry.as_ref() else {
            return false;
        };
        if registry.is_block_item(item_id)
            && !component_patch
                .removed_type_ids
                .contains(&component_id::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID)
            && component_patch
                .block_entity_data_type_id
                .is_some_and(vanilla_block_entity_type_only_op_can_set_nbt)
        {
            return true;
        }
        registry.is_spawn_egg_item(item_id)
            && !component_patch
                .removed_type_ids
                .contains(&component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)
            && component_patch
                .entity_data_entity_type_id
                .is_some_and(vanilla_entity_type_only_op_can_set_nbt)
    }

    fn template_hover_name_arg(&self, item: &ItemStackTemplateSummary) -> Option<TooltipTextArg> {
        let resource_id = self.registry.as_ref()?.resource_id(item.item_id)?;
        Some(hover_name_arg_for_component_patch(
            &self.language,
            resource_id,
            &item.component_patch,
        ))
    }

    fn push_charged_projectile_group_tooltip_line(
        &self,
        projectile: &ItemStackTemplateSummary,
        count: usize,
        options: NativeItemTooltipOptions<'_>,
        item_stack_template_depth: usize,
        lines: &mut Vec<NativeItemTooltipLine>,
    ) {
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(resource_id) = registry.resource_id(projectile.item_id) else {
            return;
        };
        let projectile_name = display_name_arg_for_component_patch(
            &self.language,
            resource_id,
            &projectile.component_patch,
        );
        lines.push(charged_projectile_group_tooltip_line(
            &self.language,
            projectile_name,
            count,
        ));
        if item_stack_template_depth >= MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
            return;
        }
        let mut detail_lines = Vec::new();
        self.push_stack_detail_tooltip_lines(
            resource_id,
            projectile.item_id,
            &projectile.component_patch,
            NativeItemTooltipOptions {
                advanced: false,
                creative: false,
                map_data: None,
                ..options
            },
            item_stack_template_depth + 1,
            &mut detail_lines,
        );
        lines.extend(
            detail_lines
                .into_iter()
                .map(charged_projectile_detail_tooltip_line),
        );
    }

    fn push_charged_projectiles_tooltip_lines(
        &self,
        projectiles: &[ItemStackTemplateSummary],
        options: NativeItemTooltipOptions<'_>,
        item_stack_template_depth: usize,
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
                    self.push_charged_projectile_group_tooltip_line(
                        previous,
                        count,
                        options,
                        item_stack_template_depth,
                        lines,
                    );
                    current = Some(projectile);
                    count = 1;
                }
            }
        }
        if let Some(projectile) = current {
            self.push_charged_projectile_group_tooltip_line(
                projectile,
                count,
                options,
                item_stack_template_depth,
                lines,
            );
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
            let Some(item_name) = self.template_hover_name_arg(item) else {
                continue;
            };
            lines.push(translated_tooltip_line_with_args(
                &self.language,
                "item.container.item_count",
                &[item_name, TooltipTextArg::plain(item.count.to_string())],
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
                ..NativeItemTooltipOptions::default()
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
            let mut lines = Vec::new();
            if self.should_print_operator_nbt_warning(item_id, &stack.component_patch, options) {
                push_operator_nbt_warning_tooltip_lines(&self.language, &mut lines);
            }
            return (!lines.is_empty()).then_some(lines);
        }

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
        self.push_stack_detail_tooltip_lines(
            item_id,
            protocol_id,
            &stack.component_patch,
            options,
            0,
            &mut lines,
        );
        Some(lines)
    }
}
