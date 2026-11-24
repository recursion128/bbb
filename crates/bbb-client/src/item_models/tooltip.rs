#[cfg(test)]
use bbb_client_world::VANILLA_PAINTING_VARIANTS;
use bbb_client_world::{
    direct_banner_pattern_presentation, direct_trim_material_presentation,
    direct_trim_pattern_presentation, loader_ordered_resource_id, painting_variant_translation_key,
    vanilla_painting_variant, BannerPatternElement, EnchantmentAttributeModifier,
    EnchantmentTooltipDefinition, InstrumentPresentation, ItemProfiles, JukeboxSongPresentation,
    PaintingVariantPresentation, TrimMaterialPresentation, TrimPatternPresentation,
    VanillaEnchantmentIdentity, VanillaInstrumentIdentity, VanillaPaintingVariant,
    VanillaTrimMaterialIdentity, BANNER_PATTERN_REGISTRY, ENCHANTMENT_REGISTRY,
    INSTRUMENT_REGISTRY, TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY,
    VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
};
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
    block_types::vanilla_block_type,
    builtin_enum_component_values::{normalize_vanilla_dye_color_id, vanilla_dye_color_name},
    chat_formattings::ChatFormatting,
    entity_types::{
        vanilla_entity_type_allowed_in_peaceful, vanilla_entity_type_only_op_can_set_nbt,
    },
    equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUPS,
    mob_effect_types::{vanilla_mob_effect_type, MobEffectCategory},
    packets::{
        data_components::catalog::adventure_mode_predicate_tooltip_summary,
        AdventureModePredicateDataSummary, AttributeModifierSummary, PaintingVariantSummary,
        SuspiciousStewEffectSummary, MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
    },
    tropical_fish_patterns::{
        vanilla_tropical_fish_pattern, VANILLA_TROPICAL_FISH_COMMON_VARIANTS,
    },
    ComponentStyle, JavaString, NbtSummaryValue, StyledTextRun,
};
use bbb_render_types::BannerPatternKind;

use bbb_client_assets::{JukeboxSongRegistry, LanguageCatalog, TagCatalog};
use bbb_client_render::item_model::{description_key, localized_item_name};
use bbb_client_render::{HudComponentTextRun, HudStyledTextRun, HudTextStyle};
use bbb_protocol::item_component_value_types::{
    FireworkExplosionShapeSummary, ItemRaritySummary, MapPostProcessingSummary,
};
use bbb_protocol::packets::{
    BannerPatternLayerSummary, DataComponentPatchSummary, FireworkExplosionSummary,
    ItemEnchantmentSummary, JukeboxSongSummary, ResolvableProfileKindSummary,
    ResolvableProfileSummary, TrimMaterialSummary, TrimPatternSummary,
};
use bbb_protocol::potion_types::vanilla_potion;
#[cfg(test)]
use bbb_protocol::potion_types::VanillaPotionEffect;

use super::*;

fn item_stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

pub(super) const TOOLTIP_TEXT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub(super) const TOOLTIP_TEXT_YELLOW: [f32; 4] = [1.0, 1.0, 85.0 / 255.0, 1.0];
pub(super) const TOOLTIP_TEXT_AQUA: [f32; 4] = [85.0 / 255.0, 1.0, 1.0, 1.0];
pub(super) const TOOLTIP_TEXT_LIGHT_PURPLE: [f32; 4] = [1.0, 85.0 / 255.0, 1.0, 1.0];
pub(super) const TOOLTIP_TEXT_DARK_PURPLE: [f32; 4] = [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
pub(super) const TOOLTIP_TEXT_GRAY: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];
pub(super) const TOOLTIP_TEXT_DARK_GRAY: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];
pub(super) const TOOLTIP_TEXT_DARK_GREEN: [f32; 4] = [0.0, 170.0 / 255.0, 0.0, 1.0];
pub(super) const TOOLTIP_TEXT_BLUE: [f32; 4] = [85.0 / 255.0, 85.0 / 255.0, 1.0, 1.0];
pub(super) const TOOLTIP_TEXT_RED: [f32; 4] = [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];

pub(super) type BoundKeyLookupFn = fn(&[u16]) -> Option<&'static str>;

pub(super) struct TooltipTextLookup<'a> {
    pub language: &'a LanguageCatalog,
    pub bound_key_lookup: Option<BoundKeyLookupFn>,
}

impl bbb_client_assets::ComponentTextLookup for TooltipTextLookup<'_> {
    fn translation_utf8(&self, key: &str) -> Option<&str> {
        self.language.get(key)
    }

    fn translation_units(&self, key: &[u16]) -> Option<&JavaString> {
        self.language.get_raw_units(key)
    }

    fn bound_key(&self, mapping_id: &[u16]) -> Option<&str> {
        self.bound_key_lookup.and_then(|lookup| lookup(mapping_id))
    }
}

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
    // `withColor(ChatFormatting)` keeps the name beside the value
    // (`TextColor.fromLegacyFormat`), which is what a
    // re-serialized lore style has to spell.
    color_name: Some(ChatFormatting::DarkPurple),
    shadow_color: None,
    click_event: None,
    hover_event: None,
    insertion: None,
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

/// `InstrumentComponent.addToTooltip` (InstrumentComponent.java:24) merges the
/// description with `Style.EMPTY.withColor(ChatFormatting.GRAY)`, so this colour
/// is the named one rather than a bare `0xAAAAAA`.
const INSTRUMENT_DESCRIPTION_STYLE: ComponentStyle = ComponentStyle {
    bold: None,
    italic: None,
    underlined: None,
    strikethrough: None,
    obfuscated: None,
    color: Some(TOOLTIP_GRAY_TEXT_COLOR),
    color_name: Some(ChatFormatting::Gray),
    shadow_color: None,
    click_event: None,
    hover_event: None,
    insertion: None,
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

/// `ItemStack.getHoverName` flattened plus the `getStyledHoverName` draw runs
/// (rarity colour, italic for a custom name).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeItemHoverName {
    pub text: String,
    pub runs: Vec<HudComponentTextRun>,
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
    pub item_profiles: Option<&'a ItemProfiles>,
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
    pub banner_pattern_presentations: Option<&'a [Option<BannerPatternElement>]>,
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
    pub item_inspector: bool,
    pub furnace_burn_ticks: Option<u32>,
    pub mod_display_name: Option<&'a str>,
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
    } else {
        None
    }
}

fn charged_projectile_detail_tooltip_line(line: NativeItemTooltipLine) -> NativeItemTooltipLine {
    let mut runs = Vec::with_capacity(line.runs.len() + 1);
    runs.push(HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
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
            shadow_color: style.shadow_color,
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

fn format_item_name_template(template: String) -> String {
    if !template.contains('%') {
        return template;
    }
    let mut localized = String::new();
    bbb_client_assets::visit_localized_template(&template, 0, |part| {
        if let bbb_client_assets::LocalizedTemplatePart::Literal(literal) = part {
            localized.push_str(literal);
        }
    });
    localized
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
    Some(format_item_name_template(
        language
            .get(&translation_key)
            .unwrap_or(&translation_key)
            .to_string(),
    ))
}

fn potion_item_name_override(
    language: &LanguageCatalog,
    resource_id: &str,
    patch: &DataComponentPatchSummary,
) -> Option<String> {
    let prefix = match resource_id {
        "minecraft:potion" => "item.minecraft.potion.effect.",
        "minecraft:splash_potion" => "item.minecraft.splash_potion.effect.",
        "minecraft:lingering_potion" => "item.minecraft.lingering_potion.effect.",
        "minecraft:tipped_arrow" => "item.minecraft.tipped_arrow.effect.",
        _ => return None,
    };
    if patch
        .removed_type_ids
        .contains(&component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
    {
        return None;
    }
    let known_suffix = patch
        .potion_contents
        .potion_id
        .and_then(vanilla_potion)
        .map(|potion| potion.name)
        .unwrap_or("empty");
    let custom_suffix = patch
        .potion_contents
        .custom_name
        .as_ref()
        .map(JavaString::units);
    let suffix_len = custom_suffix
        .map(<[u16]>::len)
        .unwrap_or_else(|| known_suffix.encode_utf16().count());
    let key_len = prefix.len().checked_add(suffix_len)?;
    let mut stack_units = [0_u16; 128];
    let mut heap_units = Vec::new();
    let key_units = if key_len <= stack_units.len() {
        &mut stack_units[..key_len]
    } else {
        heap_units.resize(key_len, 0);
        heap_units.as_mut_slice()
    };
    for (target, unit) in key_units.iter_mut().zip(prefix.encode_utf16()) {
        *target = unit;
    }
    if let Some(custom_suffix) = custom_suffix {
        key_units[prefix.len()..].copy_from_slice(custom_suffix);
    } else {
        for (target, unit) in key_units[prefix.len()..]
            .iter_mut()
            .zip(known_suffix.encode_utf16())
        {
            *target = unit;
        }
    }
    let value = language
        .get_raw_units(key_units)
        .map(JavaString::to_display_string)
        .unwrap_or_else(|| String::from_utf16_lossy(key_units));
    Some(format_item_name_template(value))
}

/// Source runs of the hover name
/// plus whether they came from `minecraft:custom_name` (which vanilla
/// `ItemStack.getStyledHoverName` additionally italicizes).
fn hover_name_source_runs(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    resource_id: &str,
    stack: &ItemStackSummary,
) -> (Vec<StyledTextRun>, bool) {
    hover_name_source_runs_for_component_patch(
        language,
        bound_key_lookup,
        resource_id,
        &stack.component_patch,
    )
}

fn hover_name_source_runs_for_component_patch(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    resource_id: &str,
    patch: &DataComponentPatchSummary,
) -> (Vec<StyledTextRun>, bool) {
    if let Some(name) = &patch.custom_name {
        if patch.custom_name_nbt.is_some() {
            let runs = localized_component_description_with_binding(
                language,
                name,
                patch.custom_name_styled.as_deref().unwrap_or(&[]),
                patch.custom_name_nbt.as_ref(),
                bound_key_lookup,
            )
            .1;
            return (runs, true);
        }
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
        .map(|book| &book.title)
        .filter(|title| !title.is_minecraft_blank())
    {
        return (plain(title.to_display_string()), false);
    }
    if let Some(name) = shield_item_name_override(language, resource_id, patch) {
        return (plain(name), false);
    }
    if let Some(name) = potion_item_name_override(language, resource_id, patch) {
        return (plain(name), false);
    }
    if let Some(name) = &patch.item_name {
        if patch.item_name_nbt.is_some() {
            let runs = localized_component_description_with_binding(
                language,
                name,
                patch.item_name_styled.as_deref().unwrap_or(&[]),
                patch.item_name_nbt.as_ref(),
                bound_key_lookup,
            )
            .1;
            return (runs, false);
        }
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> TooltipTextArg {
    let (name_runs, name_is_custom) = hover_name_source_runs_for_component_patch(
        language,
        bound_key_lookup,
        resource_id,
        component_patch,
    );
    let rarity_color = item_rarity_color(item_rarity_for_stack(component_patch));
    let bracket_run = |text: &str| HudComponentTextRun {
        visual: HudStyledTextRun {
            shadow_color: None,
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
            name_runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
        ),
        runs,
    }
}

fn hover_name_arg_for_component_patch(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
) -> TooltipTextArg {
    let (name_runs, _) = hover_name_source_runs_for_component_patch(
        language,
        bound_key_lookup,
        resource_id,
        component_patch,
    );
    TooltipTextArg {
        text: name_runs.iter().map(|run| run.text.as_str()).collect(),
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
    if !book.author.is_minecraft_blank() {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg_into(language, "book.byAuthor", |output| {
                book.author.write_display_to(output)
            }),
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(instrument) = component_patch.instrument.direct.as_deref() {
        let (text, styled) = localized_component_description_with_binding(
            language,
            &instrument.description,
            instrument.description_styled.as_deref().unwrap_or(&[]),
            instrument.description_nbt.as_deref(),
            bound_key_lookup,
        );
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_GRAY,
            runs: hud_runs_from_component(&styled, &text, &INSTRUMENT_DESCRIPTION_STYLE),
        });
        return;
    }

    let Some(holder_id) = component_patch.instrument.registry_id else {
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
            let (text, styled) = localized_component_description_with_binding(
                language,
                &presentation.description,
                &presentation.description_styled,
                Some(&presentation.description_nbt),
                bound_key_lookup,
            );
            lines.push(NativeItemTooltipLine {
                text: text.clone(),
                tint: TOOLTIP_TEXT_GRAY,
                runs: hud_runs_from_component(&styled, &text, &INSTRUMENT_DESCRIPTION_STYLE),
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

    // `minecraft:instrument` is loaded from data files
    // and registered in sorted `Identifier` order
    // so an unsynchronized holder id
    // indexes the sorted resource ids, not the `Instruments` bootstrap order.
    let Some(instrument_key) = loader_ordered_resource_id(INSTRUMENT_REGISTRY, holder_id) else {
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
            (layer.asset_id.as_deref(), layer.translation_key.as_ref())
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
                        .map(|pattern| JavaString::from(pattern.translation_key())),
                    // `minecraft:banner_pattern` is loaded from data files
                    // so the unsynchronized holder
                    // id is a sorted-resource-id index.
                    None => loader_ordered_resource_id(BANNER_PATTERN_REGISTRY, registry_id)
                        .and_then(BannerPatternKind::for_resource_id)
                        .map(|pattern| JavaString::from(pattern.translation_key())),
                },
            }
        } else {
            None
        };
        let Some(translation_key) = translation_key else {
            continue;
        };
        let color_name = dye_color_name(dye_color_id_or_white(Some(layer.color_id)));
        let mut key = translation_key;
        key.push_str(".");
        key.push_str(color_name);
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key_raw(&key).to_display_string(),
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
                shadow_color: None,
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
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
        push_painting_tooltip_lines(language, bound_key_lookup, component_patch, options, lines);
    }
}

fn push_painting_tooltip_lines(
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(variant) = component_patch.painting_variant_direct.as_ref() {
        push_painting_variant_tooltip_lines(language, bound_key_lookup, variant, lines);
    } else if let Some(variant_id) = component_patch.painting_variant_id {
        if let Some(presentations) = options.painting_variant_presentations {
            let presentation = usize::try_from(variant_id)
                .ok()
                .and_then(|index| presentations.get(index))
                .and_then(Option::as_ref);
            if let Some(presentation) = presentation {
                push_painting_variant_presentation_tooltip_lines(
                    language,
                    bound_key_lookup,
                    presentation,
                    lines,
                );
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
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
            language,
            bound_key_lookup,
            variant.title.as_deref(),
            Some(&variant.title_styled),
            variant.title_nbt.as_ref(),
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
            language,
            bound_key_lookup,
            variant.author.as_deref(),
            Some(&variant.author_styled),
            variant.author_nbt.as_ref(),
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
    bound_key_lookup: Option<BoundKeyLookupFn>,
    variant: &PaintingVariantSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    push_optional_component_tooltip_line(
        language,
        bound_key_lookup,
        variant.title.as_deref(),
        variant.title_styled.as_deref(),
        variant.title_nbt.as_deref(),
        lines,
    );
    push_optional_component_tooltip_line(
        language,
        bound_key_lookup,
        variant.author.as_deref(),
        variant.author_styled.as_deref(),
        variant.author_nbt.as_deref(),
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
    language: &LanguageCatalog,
    bound_key_lookup: Option<BoundKeyLookupFn>,
    text: Option<&str>,
    runs: Option<&[StyledTextRun]>,
    nbt: Option<&NbtSummaryValue>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some(text) = text {
        let (text, styled) = localized_component_description_with_binding(
            language,
            text,
            runs.unwrap_or(&[]),
            nbt,
            bound_key_lookup,
        );
        lines.push(NativeItemTooltipLine {
            text: text.clone(),
            tint: TOOLTIP_TEXT_WHITE,
            runs: hud_runs_from_component(&styled, &text, &ComponentStyle::default()),
        });
    }
}

fn colored_component_tooltip_line(text: String, color: u32) -> NativeItemTooltipLine {
    NativeItemTooltipLine {
        text: text.clone(),
        tint: TOOLTIP_TEXT_WHITE,
        runs: vec![HudComponentTextRun {
            visual: HudStyledTextRun {
                shadow_color: None,
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
                    shadow_color: None,
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
    header_key: &str,
    predicate: &AdventureModePredicateDataSummary,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let predicate = adventure_mode_predicate_tooltip_summary(predicate);
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
        let Some(block) = vanilla_block_type(*block_id) else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(
            localized_block_name(language, block.resource_id),
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
mod tests;

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
                shadow_color: None,
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

mod component_lines;
pub(crate) use component_lines::*;

fn push_block_state_tooltip_lines(
    language: &LanguageCatalog,
    block_state_properties: &bbb_protocol::packets::data_components::BlockStateProperties,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(honey_level) = block_state_properties
        .get_str("honey_level")
        .and_then(|value| String::from_utf16(value.units()).ok()?.parse::<i32>().ok())
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
    translate_with_first_arg_into(language, key, |output| output.push_str(arg))
}

fn translate_with_first_arg_into(
    language: &LanguageCatalog,
    key: &str,
    mut write_arg: impl FnMut(&mut String),
) -> String {
    let template = language.get_or_key(key);
    let mut output = String::with_capacity(template.len());
    if template.contains("%1$s") {
        for (index, part) in template.split("%1$s").enumerate() {
            if index > 0 {
                write_arg(&mut output);
            }
            output.push_str(part);
        }
    } else if let Some((before, after)) = template.split_once("%s") {
        output.push_str(before);
        write_arg(&mut output);
        output.push_str(after);
    } else {
        output.push_str(template);
    }
    output
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

fn advanced_component_count(
    component_patch: &DataComponentPatchSummary,
    default_component_codec_values: Option<&BTreeMap<i32, serde_json::Value>>,
) -> Option<usize> {
    let defaults = default_component_codec_values?;
    let removed = component_patch
        .removed_type_ids
        .iter()
        .filter(|id| defaults.contains_key(id))
        .count();
    let added = component_patch
        .added_type_ids
        .iter()
        .filter(|id| component_patch.removed_type_ids.contains(id) || !defaults.contains_key(id))
        .count();
    Some(defaults.len() - removed + added)
}

fn push_advanced_tooltip_lines(
    language: &LanguageCatalog,
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
    damage_state: Option<(i32, i32)>,
    default_component_codec_values: Option<&BTreeMap<i32, serde_json::Value>>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some((damage, max_damage)) = damage_state.filter(|(damage, _)| *damage > 0) {
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
        advanced_component_count(component_patch, default_component_codec_values)
            .filter(|component_count| *component_count > 0)
    {
        lines.push(NativeItemTooltipLine::plain(
            translate_with_first_arg(language, "item.components", &component_count.to_string()),
            TOOLTIP_TEXT_DARK_GRAY,
        ));
    }
}

/// Formats `Enchantment.getFullname` for a synchronized registry holder.
///
/// The caller supplies the holder resource id and decoded definition so
/// registry ownership remains outside the item runtime; localization,
/// curse tint, Component fonts, and level suffixes reuse the canonical
/// item-tooltip path.
pub(crate) fn enchantment_name_tooltip_line(
    runtime: &NativeItemRuntime,
    resource_id: &str,
    level: i32,
    definition: Option<&EnchantmentTooltipDefinition>,
) -> NativeItemTooltipLine {
    let mut lines = Vec::with_capacity(1);
    push_enchantment_tooltip_line(
        runtime.language(),
        runtime.bound_key_lookup(),
        resource_id,
        level,
        definition,
        runtime.enchantment_tags(),
        &mut lines,
    );
    lines
        .pop()
        .expect("enchantment formatter always emits one line")
}

fn push_stack_detail_tooltip_lines(
    runtime: &NativeItemRuntime,
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
    let language = runtime.language();
    let bound_key_lookup = runtime.bound_key_lookup();
    let shows = |type_id| tooltip_display_shows(component_patch, type_id);
    let tooltip_tick_rate = effective_tooltip_tick_rate(options);
    push_item_specific_tooltip_lines(
        language,
        bound_key_lookup,
        item_id,
        component_patch,
        options,
        lines,
    );
    if shows(component_id::VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID) {
        push_tropical_fish_tooltip_lines(language, component_patch, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID) {
        push_instrument_tooltip_lines(language, bound_key_lookup, component_patch, options, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID) {
        push_map_id_tooltip_lines(language, component_patch, options, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_BEES_TYPE_ID) {
        push_bees_tooltip_lines(
            language,
            component_patch.bees_present,
            component_patch.bees.len(),
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID) {
        push_container_loot_tooltip_lines(language, component_patch.container_loot, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID) {
        push_container_items_tooltip_lines(
            runtime,
            component_patch.container.items(),
            options.item_profiles,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID) {
        push_banner_pattern_tooltip_lines(
            language,
            &component_patch.banner_pattern_layers,
            options,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID) {
        push_pot_decorations_tooltip_lines(
            runtime,
            &component_patch.pot_decorations_item_ids,
            lines,
        );
    }
    if let Some(book) = component_patch
        .written_book
        .as_ref()
        .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID))
    {
        push_written_book_tooltip_lines(language, book, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID) {
        push_charged_projectiles_tooltip_lines(
            runtime,
            &component_patch.charged_projectiles_items,
            options,
            item_stack_template_depth,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID) {
        push_fireworks_tooltip_lines(
            language,
            component_patch.fireworks_flight_duration,
            &component_patch.fireworks_explosions,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID) {
        push_firework_explosion_tooltip_lines(language, component_patch, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID) {
        let scale_id = component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID;
        let source = options.item_profiles.map_or(component_patch, |profiles| {
            profiles.component_source(Some(protocol_id), component_patch, scale_id)
        });
        let duration_scale = if source.removed_type_ids.contains(&scale_id) {
            1.0
        } else {
            source
                .potion_duration_scale_bits
                .map(f32::from_bits)
                .unwrap_or(1.0)
        };
        push_potion_contents_tooltip_lines(
            language,
            component_patch,
            duration_scale,
            tooltip_tick_rate,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID) {
        push_jukebox_playable_tooltip_lines(
            language,
            bound_key_lookup,
            runtime.jukebox_songs(),
            component_patch.jukebox_song_id,
            component_patch.jukebox_direct_song.as_deref(),
            options.jukebox_song_keys,
            options.jukebox_song_presentations,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID) {
        push_armor_trim_tooltip_lines(
            language,
            bound_key_lookup,
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
    let enchantment_tags = runtime.enchantment_tags();
    let tooltip_order = enchantment_tags.and_then(|tags| tags.values("minecraft:tooltip_order"));
    if shows(component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID) {
        push_enchantments_tooltip_lines(
            language,
            bound_key_lookup,
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
            language,
            bound_key_lookup,
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
            language,
            component_patch.dyed_color,
            options.advanced,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID) {
        push_profile_tooltip_lines(language, component_patch.profile.as_ref(), lines);
    }
    // Vanilla `ItemLore.styledLines`: every lore line gets `LORE_STYLE`
    // (DARK_PURPLE + italic) merged under its own style keys.
    if shows(component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID) {
        lines.extend(
            component_patch
                .lore
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let (localized, styled) = localized_component_description_with_binding(
                        language,
                        text,
                        component_patch
                            .lore_styled
                            .get(index)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                        component_patch.lore_nbt.get(index),
                        bound_key_lookup,
                    );
                    NativeItemTooltipLine {
                        text: localized.clone(),
                        tint: TOOLTIP_TEXT_DARK_PURPLE,
                        runs: hud_runs_from_component(&styled, &localized, &LORE_STYLE),
                    }
                }),
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID) {
        let type_id = component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID;
        let source = options.item_profiles.map_or(component_patch, |profiles| {
            profiles.component_source(Some(protocol_id), component_patch, type_id)
        });
        let modifiers = if source.removed_type_ids.contains(&type_id) {
            &[][..]
        } else {
            source.attribute_modifiers.as_slice()
        };
        push_stack_attribute_modifiers_tooltip_lines(
            language,
            modifiers,
            options.enchantment_attribute_modifiers.unwrap_or_default(),
            options,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID) {
        push_intangible_projectile_tooltip_line(
            language,
            component_patch.intangible_projectile,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID)
        && component_patch.unbreakable
    {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("item.unbreakable").to_string(),
            TOOLTIP_TEXT_BLUE,
        ));
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID) {
        push_ominous_bottle_tooltip_lines(
            language,
            component_patch.ominous_bottle_amplifier,
            tooltip_tick_rate,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID) {
        push_suspicious_stew_tooltip_lines(
            language,
            &component_patch.suspicious_stew_effects,
            options.creative,
            tooltip_tick_rate,
            lines,
        );
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID) {
        push_block_state_tooltip_lines(language, &component_patch.block_state_properties, lines);
    }
    if shows(component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID) {
        push_entity_data_tooltip_lines(language, component_patch, options.peaceful, lines);
    }
    if item_is_spawner(item_id)
        && shows(component_id::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID)
    {
        push_spawner_block_entity_tooltip_lines(language, component_patch, lines);
    }
    if let Some(predicate) = component_patch
        .can_break_persistent
        .as_ref()
        .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID))
    {
        push_adventure_mode_predicate_tooltip_lines(
            language,
            CAN_BREAK_HEADER_KEY,
            predicate,
            lines,
        );
    }
    if let Some(predicate) = component_patch
        .can_place_on_persistent
        .as_ref()
        .filter(|_| shows(component_id::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID))
    {
        push_adventure_mode_predicate_tooltip_lines(
            language,
            CAN_PLACE_HEADER_KEY,
            predicate,
            lines,
        );
    }
    if options.item_inspector {
        push_item_inspector_tooltip_lines(item_id, protocol_id, component_patch, options, lines);
    }
    if options.advanced {
        push_advanced_tooltip_lines(
            language,
            item_id,
            component_patch,
            shows(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
                .then(|| {
                    ItemProfiles::damage_state(
                        options.item_profiles,
                        Some(protocol_id),
                        component_patch,
                    )
                })
                .flatten(),
            runtime.default_component_codec_values_for_resource_id(item_id),
            lines,
        );
    }
    if options.item_disabled {
        lines.push(NativeItemTooltipLine::plain(
            language.get_or_key("item.disabled").to_string(),
            TOOLTIP_TEXT_RED,
        ));
    }
    if should_print_operator_nbt_warning(runtime, item_id, component_patch, options) {
        push_operator_nbt_warning_tooltip_lines(language, lines);
    }
}

fn push_item_inspector_tooltip_lines(
    item_id: &str,
    protocol_id: i32,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let food_type_id = component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID;
    let food_source = options.item_profiles.map_or(component_patch, |profiles| {
        profiles.component_source(Some(protocol_id), component_patch, food_type_id)
    });
    if !food_source.removed_type_ids.contains(&food_type_id) {
        if let Some(nutrition) = food_source.food_nutrition {
            let saturation = food_source.food_saturation_bits.map_or(0.0, f32::from_bits);
            lines.push(NativeItemTooltipLine::plain(
                format!("Food: +{nutrition} ({saturation:.1} sat)"),
                TOOLTIP_TEXT_GRAY,
            ));
        }
    }
    if let Some(ticks) = options.furnace_burn_ticks.filter(|&t| t > 0) {
        let smelts = f64::from(ticks) / 200.0;
        lines.push(NativeItemTooltipLine::plain(
            format!("Fuel: {ticks}t ({smelts:.1} items)"),
            TOOLTIP_TEXT_GRAY,
        ));
    }
    let namespace = item_id.split_once(':').map_or("minecraft", |(ns, _)| ns);
    let source_label = options.mod_display_name.unwrap_or(namespace);
    lines.push(NativeItemTooltipLine::plain(
        format!("@{source_label}"),
        TOOLTIP_TEXT_DARK_GRAY,
    ));
}

fn should_print_operator_nbt_warning(
    runtime: &NativeItemRuntime,
    item_id: &str,
    component_patch: &DataComponentPatchSummary,
    options: NativeItemTooltipOptions<'_>,
) -> bool {
    if !options.gamemaster_permission {
        return false;
    }
    let Some(registry) = runtime.item_registry() else {
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

fn effective_template_component_patch<'a>(
    item_profiles: Option<&ItemProfiles>,
    item_id: i32,
    patch: &'a DataComponentPatchSummary,
) -> std::borrow::Cow<'a, DataComponentPatchSummary> {
    let Some(defaults) = item_profiles.and_then(|profiles| {
        profiles
            .component_defaults_view()
            .default_item_component_patches
            .get(&item_id)
    }) else {
        return std::borrow::Cow::Borrowed(patch);
    };
    let mut effective: Option<DataComponentPatchSummary> = None;
    for &component_id in &defaults.added_type_ids {
        if patch.removed_type_ids.contains(&component_id)
            || patch.added_type_ids.contains(&component_id)
        {
            continue;
        }
        let target = effective.get_or_insert_with(|| patch.clone());
        if bbb_protocol::packets::data_components::fields::copy_component_value(
            component_id,
            defaults,
            target,
        )
        .is_ok()
        {
            target.added_type_ids.push(component_id);
        }
    }
    match effective {
        Some(merged) => std::borrow::Cow::Owned(merged),
        None => std::borrow::Cow::Borrowed(patch),
    }
}

fn template_hover_name_arg(
    runtime: &NativeItemRuntime,
    item_profiles: Option<&ItemProfiles>,
    item: &bbb_protocol::packets::ItemStackTemplateSummary,
) -> Option<TooltipTextArg> {
    let resource_id = runtime.item_resource_id(item.item_id)?;
    let patch =
        effective_template_component_patch(item_profiles, item.item_id, &item.component_patch);
    Some(hover_name_arg_for_component_patch(
        runtime.language(),
        runtime.bound_key_lookup(),
        resource_id,
        patch.as_ref(),
    ))
}

fn push_charged_projectile_group_tooltip_line(
    runtime: &NativeItemRuntime,
    projectile: &bbb_protocol::packets::ItemStackTemplateSummary,
    count: usize,
    options: NativeItemTooltipOptions<'_>,
    item_stack_template_depth: usize,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let Some(resource_id) = runtime.item_resource_id(projectile.item_id) else {
        return;
    };
    let patch = effective_template_component_patch(
        options.item_profiles,
        projectile.item_id,
        &projectile.component_patch,
    );
    let projectile_name = display_name_arg_for_component_patch(
        runtime.language(),
        runtime.bound_key_lookup(),
        resource_id,
        patch.as_ref(),
    );
    lines.push(charged_projectile_group_tooltip_line(
        runtime.language(),
        projectile_name,
        count,
    ));
    if item_stack_template_depth >= MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        return;
    }
    let mut detail_lines = Vec::new();
    push_stack_detail_tooltip_lines(
        runtime,
        resource_id,
        projectile.item_id,
        patch.as_ref(),
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
    runtime: &NativeItemRuntime,
    projectiles: &[bbb_protocol::packets::ItemStackTemplateSummary],
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
                push_charged_projectile_group_tooltip_line(
                    runtime,
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
        push_charged_projectile_group_tooltip_line(
            runtime,
            projectile,
            count,
            options,
            item_stack_template_depth,
            lines,
        );
    }
}

fn push_container_items_tooltip_lines<'a>(
    runtime: &NativeItemRuntime,
    items: impl ExactSizeIterator<Item = &'a bbb_protocol::packets::ItemStackTemplateSummary>,
    item_profiles: Option<&ItemProfiles>,
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    let mut line_count = 0;
    let item_count = items.len();
    for item in items {
        if line_count > 4 {
            continue;
        }
        line_count += 1;
        let Some(item_name) = template_hover_name_arg(runtime, item_profiles, item) else {
            continue;
        };
        lines.push(translated_tooltip_line_with_args(
            runtime.language(),
            "item.container.item_count",
            &[item_name, TooltipTextArg::plain(item.count.to_string())],
            TOOLTIP_TEXT_WHITE,
        ));
    }

    let hidden_count = item_count.saturating_sub(line_count);
    if hidden_count > 0 {
        lines.push(item_container_more_tooltip_line(translate_with_first_arg(
            runtime.language(),
            "item.container.more_items",
            &hidden_count.to_string(),
        )));
    }
}

fn push_pot_decorations_tooltip_lines(
    runtime: &NativeItemRuntime,
    item_ids: &[i32],
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if item_ids.is_empty() {
        return;
    }

    let brick_id = runtime
        .item_registry()
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
        let Some(item_name) = item_hover_name_for_protocol_id(runtime, item_id) else {
            continue;
        };
        lines.push(NativeItemTooltipLine::plain(item_name, TOOLTIP_TEXT_GRAY));
    }
}

fn item_hover_name_for_protocol_id(runtime: &NativeItemRuntime, item_id: i32) -> Option<String> {
    Some(localized_item_name(
        runtime.language(),
        runtime.item_resource_id(item_id)?,
    ))
}

pub(crate) fn styled_hover_name(
    runtime: &NativeItemRuntime,
    stack: &ItemStackSummary,
) -> Option<NativeItemHoverName> {
    if item_stack_is_empty(stack) {
        return None;
    }
    let item_id = runtime.item_resource_id(stack.item_id?)?;
    let rarity = item_rarity_for_stack(&stack.component_patch);
    let (name_runs, name_is_custom) = hover_name_source_runs(
        runtime.language(),
        runtime.bound_key_lookup(),
        item_id,
        stack,
    );
    let name_wrapper = ComponentStyle {
        italic: name_is_custom.then_some(true),
        color: Some(item_rarity_color(rarity)),
        ..ComponentStyle::default()
    };
    Some(NativeItemHoverName {
        text: name_runs.iter().map(|run| run.text.as_str()).collect(),
        runs: name_runs
            .iter()
            .map(|run| hud_run_from_component(run, &name_wrapper))
            .collect(),
    })
}

pub(crate) fn tooltip_lines_for_stack_with_context(
    runtime: &NativeItemRuntime,
    stack: &ItemStackSummary,
    options: NativeItemTooltipOptions<'_>,
) -> Option<Vec<NativeItemTooltipLine>> {
    if item_stack_is_empty(stack) {
        return None;
    }
    let protocol_id = stack.item_id?;
    let item_id = runtime.item_resource_id(protocol_id)?;
    if stack.component_patch.tooltip_hide_tooltip && !options.creative {
        let mut lines = Vec::new();
        if should_print_operator_nbt_warning(runtime, item_id, &stack.component_patch, options) {
            push_operator_nbt_warning_tooltip_lines(runtime.language(), &mut lines);
        }
        return (!lines.is_empty()).then_some(lines);
    }

    // Vanilla `ItemStack.getStyledHoverName`: the hover name is wrapped in
    // the rarity colour, plus ITALIC when the stack carries a custom name;
    // the name component's own style keys win over the wrapper.
    let rarity = item_rarity_for_stack(&stack.component_patch);
    let (name_runs, name_is_custom) = hover_name_source_runs(
        runtime.language(),
        runtime.bound_key_lookup(),
        item_id,
        stack,
    );
    let name_wrapper = ComponentStyle {
        italic: name_is_custom.then_some(true),
        color: Some(item_rarity_color(rarity)),
        ..ComponentStyle::default()
    };
    let mut lines = vec![NativeItemTooltipLine {
        text: name_runs.iter().map(|run| run.text.as_str()).collect(),
        tint: item_rarity_tint(rarity),
        runs: name_runs
            .iter()
            .map(|run| hud_run_from_component(run, &name_wrapper))
            .collect(),
    }];
    push_stack_detail_tooltip_lines(
        runtime,
        item_id,
        protocol_id,
        &stack.component_patch,
        options,
        0,
        &mut lines,
    );
    Some(lines)
}
