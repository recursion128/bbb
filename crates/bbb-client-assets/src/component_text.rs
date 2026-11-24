//! Native component localization over the exact component tree.

use bbb_protocol::{
    component_node_style_from_nbt_value, ComponentStyle, JavaString, NbtSummaryEntry,
    NbtSummaryValue, StyledTextRun,
};

pub trait ComponentTextLookup {
    fn translation_utf8(&self, key: &str) -> Option<&str>;
    fn translation_units(&self, key: &[u16]) -> Option<&JavaString>;
    fn bound_key(&self, mapping_id: &[u16]) -> Option<&str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalizedTemplatePart<'a> {
    Literal(&'a str),
    Argument { index: usize, raw: &'a str },
}

enum TemplateRange {
    Literal(std::ops::Range<usize>),
    Argument {
        index: usize,
        raw: std::ops::Range<usize>,
    },
}

// TranslatableContents.java:86-160: a bad token or argument discards the entire decomposition.
pub fn visit_localized_template<'a>(
    template: &'a str,
    argument_count: usize,
    mut visitor: impl FnMut(LocalizedTemplatePart<'a>),
) {
    if let Some(parts) = localized_template_ranges(template.as_bytes(), argument_count) {
        for part in parts {
            match part {
                TemplateRange::Literal(range) => {
                    visitor(LocalizedTemplatePart::Literal(&template[range]))
                }
                TemplateRange::Argument { index, raw } => {
                    visitor(LocalizedTemplatePart::Argument {
                        index,
                        raw: &template[raw],
                    })
                }
            }
        }
    } else {
        visitor(LocalizedTemplatePart::Literal(template));
    }
}

fn localized_template_ranges<T: Copy + Into<u32>>(
    units: &[T],
    argument_count: usize,
) -> Option<Vec<TemplateRange>> {
    let mut parts = Vec::new();
    let mut offset = 0;
    let mut sequential = 0_usize;
    while offset < units.len() {
        let Some(relative_percent) = units[offset..]
            .iter()
            .position(|unit| (*unit).into() == u32::from(b'%'))
        else {
            parts.push(TemplateRange::Literal(offset..units.len()));
            break;
        };
        let percent = offset + relative_percent;
        if percent > offset {
            parts.push(TemplateRange::Literal(offset..percent));
        }
        if units
            .get(percent + 1)
            .is_some_and(|unit| (*unit).into() == u32::from(b'%'))
        {
            parts.push(TemplateRange::Literal(percent..percent + 1));
            offset = percent + 2;
            continue;
        }
        let (index, end) = if units
            .get(percent + 1)
            .is_some_and(|unit| (*unit).into() == u32::from(b's'))
        {
            let index = sequential;
            sequential += 1;
            (index, percent + 2)
        } else {
            let mut cursor = percent + 1;
            while units
                .get(cursor)
                .is_some_and(|unit| (u32::from(b'0')..=u32::from(b'9')).contains(&(*unit).into()))
            {
                cursor += 1;
            }
            if cursor == percent + 1
                || !units
                    .get(cursor)
                    .is_some_and(|unit| (*unit).into() == u32::from(b'$'))
                || !units
                    .get(cursor + 1)
                    .is_some_and(|unit| (*unit).into() == u32::from(b's'))
            {
                return None;
            }
            let index = units[percent + 1..cursor]
                .iter()
                .try_fold(0_i32, |value, digit| {
                    value
                        .checked_mul(10)?
                        .checked_add(((*digit).into() - u32::from(b'0')) as i32)
                })?
                .checked_sub(1)?;
            (usize::try_from(index).ok()?, cursor + 2)
        };
        if index >= argument_count {
            return None;
        }
        parts.push(TemplateRange::Argument {
            index,
            raw: percent..end,
        });
        offset = end;
    }
    Some(parts)
}

const MAX_LOCALIZED_COMPONENT_NODES: usize = 65_536;
const MAX_LOCALIZED_COMPONENT_RUNS: usize = 4_096;
const MAX_LOCALIZED_COMPONENT_BYTES: usize = 256 * 1024;
const MAX_LOCALIZED_COMPONENT_DEPTH: usize = 64;

struct LocalizationBudget {
    remaining_nodes: usize,
    remaining_runs: usize,
    remaining_bytes: usize,
    depth: usize,
}

impl Default for LocalizationBudget {
    fn default() -> Self {
        Self {
            remaining_nodes: MAX_LOCALIZED_COMPONENT_NODES,
            remaining_runs: MAX_LOCALIZED_COMPONENT_RUNS,
            remaining_bytes: MAX_LOCALIZED_COMPONENT_BYTES,
            depth: 0,
        }
    }
}

impl LocalizationBudget {
    #[inline]
    fn is_exhausted(&self) -> bool {
        self.remaining_nodes == 0 || self.remaining_runs == 0 || self.remaining_bytes == 0
    }

    #[inline]
    fn take_node(&mut self) -> bool {
        if self.is_exhausted() {
            return false;
        }
        self.remaining_nodes -= 1;
        true
    }
}

pub fn localized_component_runs(
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    lookup: &impl ComponentTextLookup,
) -> Vec<StyledTextRun> {
    let mut runs = Vec::new();
    let mut budget = LocalizationBudget::default();
    let _ = append_component_value_runs(&mut runs, value, inherited, lookup, &mut budget);
    runs
}

fn append_component_value_runs(
    out: &mut Vec<StyledTextRun>,
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) -> ComponentStyle {
    if budget.depth >= MAX_LOCALIZED_COMPONENT_DEPTH || !budget.take_node() {
        return inherited.clone();
    }
    budget.depth += 1;
    let effective_style = match value {
        NbtSummaryValue::String(text) => {
            push_styled_run(out, &text.to_display_string(), inherited, budget);
            inherited.clone()
        }
        NbtSummaryValue::List(values) => {
            // ComponentSerialization.java:74-82: later entries inherit the first root.
            // Evaluate the first element once so nested `hover_event` subtrees are not
            // traversed twice.
            let Some((first, rest)) = values.split_first() else {
                budget.depth -= 1;
                return inherited.clone();
            };
            let style = append_component_value_runs(out, first, inherited, lookup, budget);
            for value in rest {
                if budget.is_exhausted() {
                    break;
                }
                let _ = append_component_value_runs(out, value, &style, lookup, budget);
            }
            style
        }
        NbtSummaryValue::Compound(entries) => {
            let style = component_node_style_from_nbt_value(value).apply_to(inherited);
            append_primary_component_runs(out, entries, &style, lookup, budget);
            if !budget.is_exhausted() {
                if let Some(extra) = find_entry(entries, "extra") {
                    if let NbtSummaryValue::List(members) = extra {
                        for member in members {
                            if budget.is_exhausted() {
                                break;
                            }
                            let _ =
                                append_component_value_runs(out, member, &style, lookup, budget);
                        }
                    } else {
                        let _ = append_component_value_runs(out, extra, &style, lookup, budget);
                    }
                }
            }
            style
        }
        _ => inherited.clone(),
    };
    budget.depth -= 1;
    effective_style
}

fn append_primary_component_runs(
    out: &mut Vec<StyledTextRun>,
    entries: &[NbtSummaryEntry],
    style: &ComponentStyle,
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) {
    if let Some(text) = find_entry(entries, "text") {
        let _ = append_component_value_runs(out, text, style, lookup, budget);
        return;
    }
    if let Some(key @ NbtSummaryValue::String(_)) = find_entry(entries, "translate") {
        let raw_args = raw_translation_arguments(entries);
        let mut cache: Vec<Option<std::ops::Range<usize>>> = vec![None; raw_args.len()];
        if let NbtSummaryValue::String(text) = key {
            if let Some(template) = lookup.translation_units(text.units()) {
                append_translation_template_units(
                    out,
                    template.units(),
                    style,
                    raw_args,
                    &mut cache,
                    lookup,
                    budget,
                );
                return;
            }
        }
        if find_entry(entries, "fallback").is_some_and(|fallback| {
            append_nbt_string_template(out, fallback, style, raw_args, &mut cache, lookup, budget)
        }) {
            return;
        }
        append_nbt_string_template(out, key, style, raw_args, &mut cache, lookup, budget);
        return;
    }
    if let Some(NbtSummaryValue::String(key)) = find_entry(entries, "keybind") {
        let mut empty_cache = Vec::new();
        if let Some(display_key) = lookup.bound_key(key.units()) {
            let template = lookup.translation_utf8(display_key).unwrap_or(display_key);
            append_translation_template_utf8(
                out,
                template,
                style,
                &[],
                &mut empty_cache,
                lookup,
                budget,
            );
        } else {
            let template = lookup.translation_units(key.units()).unwrap_or(key);
            append_translation_template_units(
                out,
                template.units(),
                style,
                &[],
                &mut empty_cache,
                lookup,
                budget,
            );
        }
        return;
    }
    for key in ["selector", "nbt"] {
        if let Some(NbtSummaryValue::String(text)) = find_entry(entries, key) {
            push_styled_run(out, &text.to_display_string(), style, budget);
            return;
        }
    }
}

fn raw_translation_arguments(entries: &[NbtSummaryEntry]) -> &[NbtSummaryValue] {
    let Some(with) = find_entry(entries, "with") else {
        return &[];
    };
    match with {
        NbtSummaryValue::List(values) => values.as_slice(),
        value => std::slice::from_ref(value),
    }
}

#[allow(clippy::too_many_arguments)]
fn append_translation_argument(
    out: &mut Vec<StyledTextRun>,
    index: usize,
    raw_args: &[NbtSummaryValue],
    cache: &mut [Option<std::ops::Range<usize>>],
    style: &ComponentStyle,
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) -> bool {
    let Some(value) = raw_args.get(index) else {
        return false;
    };
    if let Some(range) = cache[index].clone() {
        for run_index in range {
            if budget.is_exhausted() {
                break;
            }
            let text_len = out[run_index].text.len();
            let copy_len = text_len.min(budget.remaining_bytes);
            let slice_len = truncate_utf8(&out[run_index].text, copy_len).len();
            if slice_len == 0 {
                if text_len > 0 {
                    budget.remaining_bytes = 0;
                }
                break;
            }
            let text = out[run_index].text[..slice_len].to_string();
            let run_style = out[run_index].style.clone();
            budget.remaining_bytes -= slice_len;
            budget.remaining_runs -= 1;
            out.push(StyledTextRun {
                text,
                style: run_style,
            });
        }
        return true;
    }
    let start = out.len();
    let number = match value {
        NbtSummaryValue::Byte(value) => Some(value.to_string()),
        NbtSummaryValue::Short(value) => Some(value.to_string()),
        NbtSummaryValue::Int(value) => Some(value.to_string()),
        NbtSummaryValue::Long(value) => Some(value.to_string()),
        NbtSummaryValue::Float(bits) => {
            Some(bbb_protocol::java_float_to_string(f32::from_bits(*bits)))
        }
        NbtSummaryValue::Double(bits) => {
            Some(bbb_protocol::java_double_to_string(f64::from_bits(*bits)))
        }
        _ => None,
    };
    if let Some(number) = number {
        push_styled_run(out, &number, style, budget);
    } else {
        let _ = append_component_value_runs(out, value, style, lookup, budget);
    }
    cache[index] = Some(start..out.len());
    true
}

#[allow(clippy::too_many_arguments)]
fn append_nbt_string_template(
    out: &mut Vec<StyledTextRun>,
    value: &NbtSummaryValue,
    style: &ComponentStyle,
    raw_args: &[NbtSummaryValue],
    cache: &mut [Option<std::ops::Range<usize>>],
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) -> bool {
    match value {
        NbtSummaryValue::String(template) => {
            append_translation_template_units(
                out,
                template.units(),
                style,
                raw_args,
                cache,
                lookup,
                budget,
            );
            true
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_translation_template_units(
    out: &mut Vec<StyledTextRun>,
    template: &[u16],
    style: &ComponentStyle,
    raw_args: &[NbtSummaryValue],
    cache: &mut [Option<std::ops::Range<usize>>],
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) {
    append_translation_template(
        out,
        template,
        style,
        raw_args,
        cache,
        lookup,
        budget,
        |range| String::from_utf16_lossy(&template[range]),
    );
}

#[allow(clippy::too_many_arguments)]
fn append_translation_template_utf8(
    out: &mut Vec<StyledTextRun>,
    template: &str,
    style: &ComponentStyle,
    raw_args: &[NbtSummaryValue],
    cache: &mut [Option<std::ops::Range<usize>>],
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
) {
    append_translation_template(
        out,
        template.as_bytes(),
        style,
        raw_args,
        cache,
        lookup,
        budget,
        |range| &template[range],
    );
}

#[allow(clippy::too_many_arguments)]
fn append_translation_template<T: Copy + Into<u32>, S: AsRef<str>>(
    out: &mut Vec<StyledTextRun>,
    template: &[T],
    style: &ComponentStyle,
    raw_args: &[NbtSummaryValue],
    cache: &mut [Option<std::ops::Range<usize>>],
    lookup: &impl ComponentTextLookup,
    budget: &mut LocalizationBudget,
    literal: impl Fn(std::ops::Range<usize>) -> S,
) {
    let parts = localized_template_ranges(template, raw_args.len())
        .unwrap_or_else(|| vec![TemplateRange::Literal(0..template.len())]);
    for part in parts {
        if !budget.take_node() {
            break;
        }
        match part {
            TemplateRange::Literal(range) => {
                let text = literal(range);
                push_styled_run(out, text.as_ref(), style, budget);
            }
            TemplateRange::Argument { index, raw } => {
                if !append_translation_argument(out, index, raw_args, cache, style, lookup, budget)
                {
                    let text = literal(raw);
                    push_styled_run(out, text.as_ref(), style, budget);
                }
            }
        }
    }
}

fn push_styled_run(
    runs: &mut Vec<StyledTextRun>,
    text: &str,
    style: &ComponentStyle,
    budget: &mut LocalizationBudget,
) {
    if text.is_empty() || budget.remaining_runs == 0 || budget.remaining_bytes == 0 {
        return;
    }
    let slice = truncate_utf8(text, budget.remaining_bytes);
    if slice.is_empty() {
        budget.remaining_bytes = 0;
        return;
    }
    budget.remaining_bytes -= slice.len();
    budget.remaining_runs -= 1;
    runs.push(StyledTextRun {
        text: slice.to_string(),
        style: style.clone(),
    });
}

fn truncate_utf8(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyLookup;

    impl ComponentTextLookup for EmptyLookup {
        fn translation_utf8(&self, _key: &str) -> Option<&str> {
            None
        }

        fn translation_units(&self, _key: &[u16]) -> Option<&JavaString> {
            None
        }

        fn bound_key(&self, _mapping_id: &[u16]) -> Option<&str> {
            None
        }
    }

    fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: JavaString::from(name),
                    value,
                })
                .collect(),
        )
    }

    fn nbt_str(text: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(JavaString::from(text))
    }

    #[test]
    fn nested_list_with_hover_event_inherits_first_style_without_exponential_retraversal() {
        // Depth-28 list wrapping a compound with hover_event: if List re-traversed items[0],
        // 2^28 work would hang for minutes; single-pass evaluation finishes instantly.
        let mut node = compound(vec![
            ("text", nbt_str("head")),
            ("color", nbt_str("red")),
            (
                "hover_event",
                compound(vec![
                    ("action", nbt_str("show_text")),
                    ("value", nbt_str("tooltip")),
                ]),
            ),
        ]);
        for _ in 0..28 {
            node = NbtSummaryValue::List(vec![node, nbt_str("!")]);
        }

        let runs = localized_component_runs(&node, &ComponentStyle::default(), &EmptyLookup);
        assert_eq!(runs.len(), 29);
        assert_eq!(runs[0].text, "head");
        assert_eq!(runs[0].style.color, Some(0xFF5555));
        assert!(runs[0].style.hover_event.is_some());
        assert_eq!(runs[28].text, "!");
        assert_eq!(runs[28].style.color, Some(0xFF5555));
    }

    #[test]
    fn recursive_translatable_percent_bomb_is_bounded_by_run_and_byte_budgets() {
        // Depth-24 doubling bomb: 2^24 copies of "ABCD" would be 64 MiB and 16M runs unbounded.
        let mut node = compound(vec![("text", nbt_str("ABCD"))]);
        for _ in 0..24 {
            node = compound(vec![
                ("translate", nbt_str("%1$s%1$s")),
                ("with", NbtSummaryValue::List(vec![node])),
            ]);
        }

        let runs = localized_component_runs(&node, &ComponentStyle::default(), &EmptyLookup);
        let total_bytes: usize = runs.iter().map(|run| run.text.len()).sum();
        assert!(!runs.is_empty());
        assert!(runs.len() <= MAX_LOCALIZED_COMPONENT_RUNS);
        assert!(total_bytes <= MAX_LOCALIZED_COMPONENT_BYTES);
    }

    #[test]
    fn utf8_budget_truncation_preserves_char_boundaries() {
        let mut runs = Vec::new();
        let mut budget = LocalizationBudget {
            remaining_nodes: 100,
            remaining_runs: 100,
            remaining_bytes: 5,
            depth: 0,
        };
        // Each CJK char is 3 UTF-8 bytes; 5-byte budget must truncate cleanly to 1 char (3 bytes).
        push_styled_run(
            &mut runs,
            "你好世界",
            &ComponentStyle::default(),
            &mut budget,
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "你");
        assert_eq!(budget.remaining_bytes, 2);

        // Next 3-byte char cannot fit in 2 bytes -> exhausts remaining_bytes without panicking.
        push_styled_run(&mut runs, "好", &ComponentStyle::default(), &mut budget);
        assert_eq!(runs.len(), 1);
        assert_eq!(budget.remaining_bytes, 0);
    }
}
