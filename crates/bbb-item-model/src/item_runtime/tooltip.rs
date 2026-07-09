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
pub struct NativeItemTooltipOptions {
    pub advanced: bool,
    pub creative: bool,
    pub map_data: Option<NativeItemMapTooltipData>,
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
    lines: &mut Vec<NativeItemTooltipLine>,
) {
    if let Some((damage, max_damage)) = effective_damage_state(component_patch, default_max_damage)
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
            },
        )
    }

    pub fn tooltip_lines_for_stack_with_context(
        &self,
        stack: &ItemStackSummary,
        options: NativeItemTooltipOptions,
    ) -> Option<Vec<NativeItemTooltipLine>> {
        if item_stack_is_empty(stack) {
            return None;
        }
        let protocol_id = stack.item_id?;
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;

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
        push_map_id_tooltip_lines(&self.language, &stack.component_patch, options, &mut lines);
        push_bees_tooltip_lines(&self.language, stack.component_patch.bees_count, &mut lines);
        push_container_loot_tooltip_lines(
            &self.language,
            stack.component_patch.container_loot,
            &mut lines,
        );
        self.push_container_items_tooltip_lines(&stack.component_patch.container_items, &mut lines);
        if let Some(book) = &stack.component_patch.written_book {
            push_written_book_tooltip_lines(&self.language, book, &mut lines);
        }
        self.push_charged_projectiles_tooltip_lines(
            &stack.component_patch.charged_projectiles_items,
            &mut lines,
        );
        push_fireworks_tooltip_lines(
            &self.language,
            stack.component_patch.fireworks_flight_duration,
            &stack.component_patch.fireworks_explosions,
            &mut lines,
        );
        push_firework_explosion_tooltip_lines(&self.language, &stack.component_patch, &mut lines);
        push_potion_contents_tooltip_lines(&self.language, &stack.component_patch, &mut lines);
        push_jukebox_playable_tooltip_lines(
            stack.component_patch.jukebox_direct_song.as_ref(),
            &mut lines,
        );
        push_armor_trim_tooltip_lines(
            &self.language,
            stack.component_patch.armor_trim_material_direct.as_ref(),
            stack.component_patch.armor_trim_pattern_direct.as_ref(),
            &mut lines,
        );
        push_dyed_color_tooltip_lines(
            &self.language,
            stack.component_patch.dyed_color,
            options.advanced,
            &mut lines,
        );
        push_profile_tooltip_lines(
            &self.language,
            stack.component_patch.profile.as_ref(),
            &mut lines,
        );
        // Vanilla `ItemLore.styledLines`: every lore line gets `LORE_STYLE`
        // (DARK_PURPLE + italic) merged under its own style keys.
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
        push_intangible_projectile_tooltip_line(
            &self.language,
            stack.component_patch.intangible_projectile,
            &mut lines,
        );
        if stack.component_patch.unbreakable {
            lines.push(NativeItemTooltipLine::plain(
                self.language.get_or_key("item.unbreakable").to_string(),
                TOOLTIP_TEXT_BLUE,
            ));
        }
        push_ominous_bottle_tooltip_lines(
            &self.language,
            stack.component_patch.ominous_bottle_amplifier,
            &mut lines,
        );
        push_suspicious_stew_tooltip_lines(
            &self.language,
            &stack.component_patch.suspicious_stew_effects,
            options.creative,
            &mut lines,
        );
        push_block_state_tooltip_lines(
            &self.language,
            &stack.component_patch.block_state_properties,
            &mut lines,
        );
        if options.advanced {
            push_advanced_tooltip_lines(
                &self.language,
                item_id,
                &stack.component_patch,
                self.default_max_damage_for_protocol_id(protocol_id),
                self.default_component_type_ids_for_resource_id(item_id),
                &mut lines,
            );
        }
        Some(lines)
    }
}
