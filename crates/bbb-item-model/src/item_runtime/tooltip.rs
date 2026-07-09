use bbb_protocol::{ComponentStyle, StyledTextRun};

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

#[derive(Debug, Clone, PartialEq)]
pub struct NativeItemTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
    /// Styled draw runs for the line; concatenating the run texts reproduces
    /// `text`. Unstyled lines carry a single default-style run with no colour
    /// override (the renderer then falls back to `tint`).
    pub runs: Vec<HudStyledTextRun>,
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
    if let Some(name) = &stack.component_patch.custom_name {
        return name.clone();
    }
    if let Some(title) = stack
        .component_patch
        .written_book
        .as_ref()
        .map(|book| book.title.as_str())
        .filter(|title| !title.trim().is_empty())
    {
        return title.to_string();
    }
    if let Some(name) = &stack.component_patch.item_name {
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
        push_bees_tooltip_lines(&self.language, stack.component_patch.bees_count, &mut lines);
        if let Some(book) = &stack.component_patch.written_book {
            push_written_book_tooltip_lines(&self.language, book, &mut lines);
        }
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
        if stack.component_patch.unbreakable {
            lines.push(NativeItemTooltipLine::plain(
                self.language.get_or_key("item.unbreakable").to_string(),
                TOOLTIP_TEXT_BLUE,
            ));
        }
        if advanced {
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
