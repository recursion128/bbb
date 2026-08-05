//! Shared language-template tokenization for native Component presenters.

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    component_node_style_from_nbt_value, ComponentStyle, NbtSummaryEntry, NbtSummaryValue,
    RichTextComponent, StyledTextRun,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalizedTemplatePart<'a> {
    Literal(&'a str),
    Argument { index: usize, raw: &'a str },
}

/// Visits vanilla `%s`, `%1$s`, and `%%` language-template parts in order.
/// Unknown format syntax remains literal, and callers decide how to present a
/// syntactically valid placeholder whose argument is unavailable.
pub(crate) fn visit_localized_template<'a>(
    template: &'a str,
    mut visitor: impl FnMut(LocalizedTemplatePart<'a>),
) {
    let bytes = template.as_bytes();
    let mut offset = 0;
    let mut sequential = 0_usize;
    while offset < bytes.len() {
        let Some(relative_percent) = bytes[offset..].iter().position(|byte| *byte == b'%') else {
            visitor(LocalizedTemplatePart::Literal(&template[offset..]));
            break;
        };
        let percent = offset + relative_percent;
        if percent > offset {
            visitor(LocalizedTemplatePart::Literal(&template[offset..percent]));
        }
        if bytes.get(percent + 1) == Some(&b'%') {
            visitor(LocalizedTemplatePart::Literal("%"));
            offset = percent + 2;
            continue;
        }
        if bytes.get(percent + 1) == Some(&b's') {
            visitor(LocalizedTemplatePart::Argument {
                index: sequential,
                raw: &template[percent..percent + 2],
            });
            sequential += 1;
            offset = percent + 2;
            continue;
        }

        let mut cursor = percent + 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let argument_index = (cursor > percent + 1
            && bytes.get(cursor) == Some(&b'$')
            && bytes.get(cursor + 1) == Some(&b's'))
        .then(|| template[percent + 1..cursor].parse::<usize>().ok())
        .flatten()
        .and_then(|index| index.checked_sub(1));
        if let Some(index) = argument_index {
            let next = cursor + 2;
            visitor(LocalizedTemplatePart::Argument {
                index,
                raw: &template[percent..next],
            });
            offset = next;
        } else {
            visitor(LocalizedTemplatePart::Literal("%"));
            offset = percent + 1;
        }
    }
}

/// Resolves one validated Component into localized, fully inherited runs.
///
/// Unlike visual-only projections, this retains click, hover, and font data so
/// each screen can decide how to present or dispatch the exact resolved style.
pub(crate) fn localized_component_runs(
    component: &RichTextComponent,
    inherited: &ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<StyledTextRun> {
    let mut runs = Vec::new();
    append_component_value_runs(&mut runs, component.component(), inherited, item_runtime);
    runs
}

pub(crate) fn localized_or_fallback(
    item_runtime: Option<&NativeItemRuntime>,
    key: &str,
    fallback: &str,
) -> String {
    item_runtime
        .map(|runtime| runtime.localized_text(key))
        .filter(|text| *text != key)
        .unwrap_or(fallback)
        .to_string()
}

fn append_component_value_runs(
    out: &mut Vec<StyledTextRun>,
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) {
    match value {
        NbtSummaryValue::String(text) => push_styled_run(out, text, inherited),
        NbtSummaryValue::List(values) => {
            for value in values {
                append_component_value_runs(out, value, inherited, item_runtime);
            }
        }
        NbtSummaryValue::Compound(entries) => {
            let style = component_node_style_from_nbt_value(value).apply_to(inherited);
            append_primary_component_runs(out, entries, &style, item_runtime);
            if let Some(extra) = find_entry(entries, "extra") {
                append_component_value_runs(out, extra, &style, item_runtime);
            }
        }
        _ => {}
    }
}

fn append_primary_component_runs(
    out: &mut Vec<StyledTextRun>,
    entries: &[NbtSummaryEntry],
    style: &ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) {
    if let Some(text) = find_entry(entries, "text") {
        append_component_value_runs(out, text, style, item_runtime);
        return;
    }
    if let Some(NbtSummaryValue::String(key)) = find_entry(entries, "translate") {
        let fallback = match find_entry(entries, "fallback") {
            Some(NbtSummaryValue::String(fallback)) => fallback.as_str(),
            _ => key.as_str(),
        };
        let template = localized_or_fallback(item_runtime, key, fallback);
        let arguments = translation_arguments(entries, style, item_runtime);
        append_translation_template(out, &template, style, &arguments);
        return;
    }
    if let Some(NbtSummaryValue::String(key)) = find_entry(entries, "keybind") {
        // `KeyMapping.createNameSupplier`: a registered mapping resolves to its
        // currently bound key's display name; an unknown id falls back to the
        // translatable id itself.
        let text = match crate::client_options::key_bindings::bound_key_display_key(key) {
            Some(bound) => localized_or_fallback(item_runtime, bound, bound),
            None => localized_or_fallback(item_runtime, key, key),
        };
        push_styled_run(out, &text, style);
        return;
    }
    for key in ["selector", "nbt"] {
        if let Some(NbtSummaryValue::String(text)) = find_entry(entries, key) {
            push_styled_run(out, text, style);
            return;
        }
    }
}

fn translation_arguments(
    entries: &[NbtSummaryEntry],
    style: &ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<Vec<StyledTextRun>> {
    let Some(with) = find_entry(entries, "with") else {
        return Vec::new();
    };
    let values = match with {
        NbtSummaryValue::List(values) => values.as_slice(),
        value => std::slice::from_ref(value),
    };
    values
        .iter()
        .map(|value| {
            let mut runs = Vec::new();
            append_component_value_runs(&mut runs, value, style, item_runtime);
            runs
        })
        .collect()
}

fn append_translation_template(
    out: &mut Vec<StyledTextRun>,
    template: &str,
    style: &ComponentStyle,
    arguments: &[Vec<StyledTextRun>],
) {
    visit_localized_template(template, |part| match part {
        LocalizedTemplatePart::Literal(text) => push_styled_run(out, text, style),
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                for run in argument {
                    push_styled_run(out, &run.text, &run.style);
                }
            } else {
                push_styled_run(out, raw, style);
            }
        }
    });
}

fn push_styled_run(runs: &mut Vec<StyledTextRun>, text: &str, style: &ComponentStyle) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut().filter(|previous| previous.style == *style) {
        previous.text.push_str(text);
    } else {
        runs.push(StyledTextRun {
            text: text.to_string(),
            style: style.clone(),
        });
    }
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{ComponentClickEvent, ComponentHoverEvent};

    use super::*;

    #[test]
    fn localized_template_tokens_cover_sequential_indexed_escape_and_invalid_forms() {
        let mut parts = Vec::new();
        visit_localized_template("<%s> %2$s %% %q", |part| parts.push(part));
        assert_eq!(
            parts,
            vec![
                LocalizedTemplatePart::Literal("<"),
                LocalizedTemplatePart::Argument {
                    index: 0,
                    raw: "%s"
                },
                LocalizedTemplatePart::Literal("> "),
                LocalizedTemplatePart::Argument {
                    index: 1,
                    raw: "%2$s"
                },
                LocalizedTemplatePart::Literal(" "),
                LocalizedTemplatePart::Literal("%"),
                LocalizedTemplatePart::Literal(" "),
                LocalizedTemplatePart::Literal("%"),
                LocalizedTemplatePart::Literal("q"),
            ]
        );
    }

    #[test]
    fn keybind_component_resolves_the_currently_bound_key_with_id_fallback() {
        let options = crate::client_options::ClientOptions::defaults(
            crate::client_options::ClientOptionHost::default(),
        );
        crate::client_options::key_bindings::install_bound_key_display_keys(&options);

        let jump = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "keybind".to_string(),
                value: NbtSummaryValue::String("key.jump".to_string()),
            },
        ]))
        .unwrap();
        let runs = localized_component_runs(&jump, &ComponentStyle::default(), None);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "key.keyboard.space");

        let unknown = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "keybind".to_string(),
                value: NbtSummaryValue::String("key.not.registered".to_string()),
            },
        ]))
        .unwrap();
        let runs = localized_component_runs(&unknown, &ComponentStyle::default(), None);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "key.not.registered");
    }

    #[test]
    fn bound_key_display_keys_follow_rebinds_and_defaults() {
        let mut options = crate::client_options::ClientOptions::defaults(
            crate::client_options::ClientOptionHost::default(),
        );
        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.up");
        let table = crate::client_options::key_bindings::bound_key_display_keys(&options);
        assert_eq!(table.get("key.forward"), Some(&"key.keyboard.up"));
        assert_eq!(table.get("key.jump"), Some(&"key.keyboard.space"));
        assert_eq!(table.get("key.attack"), Some(&"key.mouse.left"));
        assert_eq!(table.get("key.smoothCamera"), Some(&"key.keyboard.unknown"));
    }

    #[test]
    fn localized_component_arguments_retain_interactive_and_font_style() {
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".to_string(),
                value: NbtSummaryValue::String("missing.test".to_string()),
            },
            NbtSummaryEntry {
                name: "fallback".to_string(),
                value: NbtSummaryValue::String("Open %s".to_string()),
            },
            NbtSummaryEntry {
                name: "with".to_string(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::String("guide".to_string())]),
            },
        ]))
        .unwrap();
        let inherited = ComponentStyle {
            click_event: Some(ComponentClickEvent::ChangePage { page: 4 }),
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: vec![StyledTextRun {
                    text: "Page four".to_string(),
                    style: ComponentStyle::default(),
                }],
                component: None,
            }),
            font: Some("example:book".to_string()),
            ..ComponentStyle::default()
        };

        let runs = localized_component_runs(&component, &inherited, None);

        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "Open guide"
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].style.click_event, inherited.click_event);
        assert_eq!(runs[0].style.hover_event, inherited.hover_event);
        assert_eq!(runs[0].style.font, inherited.font);
    }
}
