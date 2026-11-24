//! Native Component presentation and legacy style projection.

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_protocol::{
    chat_formattings::{ChatFormatting, VANILLA_CHAT_FORMATTINGS},
    ComponentStyle, JavaString, RichTextComponent, StyledTextRun,
};
#[cfg(test)]
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_render_types::{visit_legacy_text, LegacyTextFormat};

pub(crate) struct ClientTextLookup<'a>(pub(crate) Option<&'a NativeItemRuntime>);

impl bbb_client_assets::ComponentTextLookup for ClientTextLookup<'_> {
    fn translation_utf8(&self, key: &str) -> Option<&str> {
        self.0
            .and_then(|runtime| runtime.localized_text_if_present(key))
    }

    fn translation_units(&self, key: &[u16]) -> Option<&JavaString> {
        self.0
            .and_then(|runtime| runtime.localized_text_raw_units_if_present(key))
    }

    fn bound_key(&self, mapping_id: &[u16]) -> Option<&str> {
        crate::options::key_bindings::bound_key_display_key_units(mapping_id)
    }
}

pub(crate) use bbb_client_assets::{visit_localized_template, LocalizedTemplatePart};

/// Resolves one validated Component into localized, fully inherited runs.
///
/// Unlike visual-only projections, this retains click, hover, and font data so
/// each screen can decide how to present or dispatch the exact resolved style.
pub(crate) fn localized_component_runs(
    component: &RichTextComponent,
    inherited: &ComponentStyle,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<StyledTextRun> {
    bbb_client_assets::localized_component_runs(
        component.component(),
        inherited,
        &ClientTextLookup(item_runtime),
    )
}

/// Hands each distinct click/hover/insertion triple its own id.
///
/// Vanilla `Style.equals` compares those three events, so
/// `SubStringSource.substring` cuts a part where
/// only they differ and `Font` restarts the one-pixel effect
/// extension there. World text has no interaction index to borrow, so the id
/// only has to separate the triples; a style carrying none gets no id.
#[derive(Default)]
pub(crate) struct WorldTextEventIds {
    seen: Vec<(
        Option<bbb_protocol::ComponentClickEvent>,
        Option<bbb_protocol::ComponentHoverEvent>,
        Option<bbb_protocol::JavaString>,
    )>,
}

impl WorldTextEventIds {
    pub(crate) fn id_for(&mut self, style: &ComponentStyle) -> Option<usize> {
        if style.click_event.is_none() && style.hover_event.is_none() && style.insertion.is_none() {
            return None;
        }
        let key = (
            style.click_event.clone(),
            style.hover_event.clone(),
            style.insertion.clone(),
        );
        Some(match self.seen.iter().position(|seen| *seen == key) {
            Some(index) => index,
            None => {
                self.seen.push(key);
                self.seen.len() - 1
            }
        })
    }
}

pub(crate) fn localized_world_component_runs(
    component: &RichTextComponent,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<bbb_client_render::WorldTextRun> {
    // Map decoration names reach the atlas through `font.width(Component)` and
    // `getVisualOrderText()`, so the scale the
    // surface is baked at and the glyphs it bakes share this resolved list.
    world_text_runs(&localized_component_runs(
        component,
        &ComponentStyle::default(),
        item_runtime,
    ))
}

/// The world-text projection of resolved component parts: `§` codes consumed
/// ([`legacy_resolved_runs`]), each distinct event triple with its own id.
pub(crate) fn world_text_runs(runs: &[StyledTextRun]) -> Vec<bbb_client_render::WorldTextRun> {
    let mut world = Vec::new();
    push_world_text_runs(&mut world, &mut WorldTextEventIds::default(), runs);
    world
}

/// [`world_text_runs`] appended to `world`, sharing `events` with earlier parts.
pub(crate) fn push_world_text_runs(
    world: &mut Vec<bbb_client_render::WorldTextRun>,
    events: &mut WorldTextEventIds,
    runs: &[StyledTextRun],
) {
    for run in legacy_resolved_runs(runs) {
        let style = bbb_client_render::WorldTextRun {
            text: String::new(),
            style: hud_text_style(&run.style),
            color: run.style.color,
            shadow_color: run.style.shadow_color,
            event_id: events.id_for(&run.style),
            font: run.style.font,
        };
        bbb_client_render::push_world_text(world, &style, &run.text);
    }
}

/// Visits a component part list through the shared legacy scanner, so `§` codes
/// shape the style of the characters instead of reaching the glyph pass.
///
/// Vanilla resolves component content exactly here, in `SubStringSource.create`
/// each part is iterated with its own inherited
/// style as both the current style and the reset style, which is what makes a
/// `§r` return to that part's style. Consumers that split lines first keep their
/// own measurement style; this is the drawn/visual side.
pub(crate) fn visit_legacy_component_runs(
    runs: &[StyledTextRun],
    mut visit: impl FnMut(char, &ComponentStyle, &StyledTextRun),
) {
    for run in runs {
        visit_legacy_text(
            &run.text,
            run.style.clone(),
            &run.style,
            apply_legacy_format,
            |source, style| {
                visit(source.ch, style, run);
                true
            },
        );
    }
}

/// The same `SubStringSource.create` pass as [`visit_legacy_component_runs`],
/// reshaped as resolved runs.
///
/// Consumers that need a run list rather than a per-character callback (a
/// measurement pass, a font-run split, a wrapped line) take this one list so
/// their width and their glyphs cannot disagree. Each input run keeps at least
/// one output run, so the part boundaries the callers derive interactions and
/// per-part resets from survive; a code only adds a run where it changed the
/// style.
pub(crate) fn legacy_resolved_runs(runs: &[StyledTextRun]) -> Vec<StyledTextRun> {
    let mut resolved: Vec<StyledTextRun> = Vec::new();
    for run in runs {
        let part_start = resolved.len();
        visit_legacy_text(
            &run.text,
            run.style.clone(),
            &run.style,
            apply_legacy_format,
            |source, style| {
                let joins_previous = resolved.len() > part_start
                    && resolved.last().is_some_and(|last| last.style == *style);
                if joins_previous {
                    if let Some(last) = resolved.last_mut() {
                        last.text.push(source.ch);
                    }
                } else {
                    resolved.push(StyledTextRun {
                        text: source.ch.to_string(),
                        style: style.clone(),
                    });
                }
                true
            },
        );
    }
    resolved
}

// Style: a colour code replaces the colour and clears the five
// decorations while font and events survive; a format code only sets its flag.
fn apply_legacy_format(style: &mut ComponentStyle, format: LegacyTextFormat) {
    match format {
        LegacyTextFormat::Color(value) => {
            style.color = Some(value);
            style.color_name = legacy_color_name(value);
            style.bold = Some(false);
            style.italic = Some(false);
            style.underlined = Some(false);
            style.strikethrough = Some(false);
            style.obfuscated = Some(false);
        }
        LegacyTextFormat::Obfuscated => style.obfuscated = Some(true),
        LegacyTextFormat::Bold => style.bold = Some(true),
        LegacyTextFormat::Strikethrough => style.strikethrough = Some(true),
        LegacyTextFormat::Underlined => style.underlined = Some(true),
        LegacyTextFormat::Italic => style.italic = Some(true),
    }
}

// `TextColor.fromLegacyFormat` keeps the formatting a colour was spelled with;
// the shared scanner hands back the resolved value on its own.
fn legacy_color_name(value: u32) -> Option<ChatFormatting> {
    VANILLA_CHAT_FORMATTINGS
        .iter()
        .find(|entry| !entry.is_format && entry.color == Some(value))
        .map(|entry| entry.formatting)
}

pub(crate) fn hud_text_style(style: &ComponentStyle) -> bbb_client_render::HudTextStyle {
    bbb_client_render::HudTextStyle {
        bold: style.bold == Some(true),
        italic: style.italic == Some(true),
        underlined: style.underlined == Some(true),
        strikethrough: style.strikethrough == Some(true),
        obfuscated: style.obfuscated == Some(true),
    }
}

pub(crate) fn localized_or_fallback(
    item_runtime: Option<&NativeItemRuntime>,
    key: &str,
    fallback: &str,
) -> String {
    item_runtime
        .and_then(|runtime| runtime.localized_text_if_present(key))
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_component_runs_consume_codes_with_the_part_style_as_reset() {
        use super::*;

        let style = ComponentStyle {
            bold: Some(true),
            color: Some(0xff_ff_55),
            ..ComponentStyle::default()
        };
        let runs = [StyledTextRun {
            text: "A§cB§rC§?D§".to_string(),
            style: style.clone(),
        }];
        let mut seen = Vec::new();
        visit_legacy_component_runs(&runs, |ch, resolved, _part| {
            seen.push((ch, resolved.clone()));
        });

        // The unknown code and the trailing `§` are consumed, not drawn; `A`
        // and everything after `§r` keep the inherited part style.
        assert_eq!(
            seen.iter().map(|(ch, _)| *ch).collect::<Vec<_>>(),
            ['A', 'B', 'C', 'D']
        );
        let inherited = ComponentStyle {
            bold: Some(true),
            color: Some(0xff_ff_55),
            ..ComponentStyle::default()
        };
        assert_eq!(seen[0].1, inherited);
        assert_eq!(
            seen[1].1,
            ComponentStyle {
                bold: Some(false),
                italic: Some(false),
                underlined: Some(false),
                strikethrough: Some(false),
                obfuscated: Some(false),
                color: Some(0xff_55_55),
                color_name: Some(bbb_protocol::chat_formattings::ChatFormatting::Red),
                ..ComponentStyle::default()
            }
        );
        assert_eq!(seen[2].1, inherited);
        assert_eq!(seen[3].1, inherited);
    }

    #[test]
    fn localized_world_component_runs_consume_codes_before_the_map_text_width() {
        use bbb_client_render::{
            item_frame_map_text_width, HudAsciiGlyph, HudFontGlyphBook, HudFontGlyphMap,
        };
        use std::collections::BTreeMap;

        let component = RichTextComponent::try_from_component_nbt(
            bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
                "text": "ab§ccd§ref§?gh§"
            }))
            .unwrap(),
        )
        .unwrap();
        let runs = localized_world_component_runs(&component, None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
            ["ab", "cd", "efgh"]
        );
        assert_eq!(runs[1].color, Some(0xff_55_55));
        assert_eq!(runs[0].color, None);
        assert_eq!(runs[2].color, None);

        // `MapRenderer` scales the decoration by `font.width(name)`;
        // an unconsumed `§` would add the font's `?` missing-glyph advance.
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', '?'] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: 6.0,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        );
        assert_eq!(item_frame_map_text_width(&runs, &fonts), Some(48));
    }

    #[test]
    fn map_decoration_name_runs_separate_the_event_triples_they_carry() {
        use super::*;

        // Vanilla `Style.equals` counts the click, hover and insertion events,
        // so `SubStringSource.substring` cuts a
        // part where only they differ even though nothing visible changes.
        let copy = |value: &str| serde_json::json!({"action": "copy_to_clipboard", "value": value});
        let component = RichTextComponent::try_from_component_nbt(
            bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
                "text": "a",
                "extra": [
                    {"text": "b", "click_event": copy("one")},
                    {"text": "c", "click_event": copy("two")},
                    {"text": "d", "click_event": copy("one")},
                ],
            }))
            .unwrap(),
        )
        .unwrap();
        let runs = localized_world_component_runs(&component, None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c", "d"]
        );
        // An event-free run carries no id; an event triple already seen keeps
        // the id it was given, so only a real difference cuts a part.
        assert_eq!(
            runs.iter().map(|run| run.event_id).collect::<Vec<_>>(),
            [None, Some(0), Some(1), Some(0)]
        );
    }

    #[test]
    fn legacy_component_runs_never_pair_codes_across_parts() {
        use super::*;

        let part = |text: &str| StyledTextRun {
            text: text.to_string(),
            style: ComponentStyle::default(),
        };
        let runs = [part("§"), part("aX")];
        let mut text = String::new();
        visit_legacy_component_runs(&runs, |ch, _style, _part| text.push(ch));
        assert_eq!(text, "aX");
    }

    #[test]
    fn legacy_resolved_runs_split_at_each_code_and_merge_only_equal_styles() {
        use super::*;

        let inherited = ComponentStyle {
            font: Some("example:names".into()),
            ..ComponentStyle::default()
        };
        let runs = [
            StyledTextRun {
                text: "AB§cCD§rEF§?GH§".to_string(),
                style: inherited.clone(),
            },
            StyledTextRun {
                text: "§lIJ".to_string(),
                style: inherited.clone(),
            },
        ];
        let resolved = legacy_resolved_runs(&runs);

        // The unknown code and both trailing `§` are consumed, never drawn, and
        // the second part's `§l` cannot reach `GH`: the parts resolve
        // independently, which is why `IJ` stays its own run even though it
        // ended up with the style the first part already had. `EF` and `GH`
        // merge because `§r` returned them to the same part style.
        assert_eq!(
            resolved
                .iter()
                .map(|run| run.text.as_str())
                .collect::<Vec<_>>(),
            ["AB", "CD", "EFGH", "IJ"]
        );
        let red = ComponentStyle {
            bold: Some(false),
            italic: Some(false),
            underlined: Some(false),
            strikethrough: Some(false),
            obfuscated: Some(false),
            color: Some(0xff_55_55),
            color_name: Some(bbb_protocol::chat_formattings::ChatFormatting::Red),
            font: Some("example:names".into()),
            ..ComponentStyle::default()
        };
        // `AB` and `EFGH` keep the part style; `CD` carries the colour and its
        // cleared decorations; `IJ` only adds bold to the inherited style.
        assert_eq!(resolved[0].style, inherited);
        assert_eq!(resolved[1].style, red);
        assert_eq!(resolved[2].style, inherited);
        assert_eq!(
            resolved[3].style,
            ComponentStyle {
                bold: Some(true),
                font: Some("example:names".into()),
                ..ComponentStyle::default()
            }
        );
    }

    #[test]
    fn raw_keybind_native_displays_unknown_key_without_changing_canonical_units() {
        let nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "keybind".into(),
                value: NbtSummaryValue::string_from_utf16(vec![107, 101, 121, 46, 0xd800]),
            },
            NbtSummaryEntry {
                name: "italic".into(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let component = RichTextComponent::try_from_component_nbt(nbt.clone()).unwrap();
        let runs = localized_component_runs(&component, &ComponentStyle::default(), None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "key.�"
        );
        assert!(runs.iter().all(|run| run.style.italic == Some(true)));
        assert_eq!(component.component(), &nbt);
    }

    #[test]
    fn raw_translatable_native_keeps_text_and_formats_primitive_arguments() {
        let raw = |units| NbtSummaryValue::string_from_utf16(units);
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(
            [
                ("translate", raw(vec![0xd802])),
                (
                    "fallback",
                    raw(vec![0xd800, 37, 115, 47, 37, 115, 47, 37, 115, 0xdc00]),
                ),
                ("bold", NbtSummaryValue::Byte(1)),
                (
                    "with",
                    NbtSummaryValue::List(vec![
                        raw(vec![0xd801]),
                        NbtSummaryValue::Long(42),
                        NbtSummaryValue::Double((-0.0f64).to_bits()),
                    ]),
                ),
            ]
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
        ))
        .unwrap();
        let before = component.component().clone();
        let runs = localized_component_runs(&component, &ComponentStyle::default(), None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "��/42/-0.0�"
        );
        assert!(runs.iter().all(|run| run.style.bold == Some(true)));
        assert_eq!(component.component(), &before);
        let literal = RichTextComponent::try_from_component_nbt(raw(vec![0xd800])).unwrap();
        assert_eq!(
            localized_component_runs(&literal, &ComponentStyle::default(), None)[0].text,
            "�"
        );
        let key_only = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: raw(vec![0xd800]),
            },
        ]))
        .unwrap();
        assert_eq!(
            localized_component_runs(&key_only, &ComponentStyle::default(), None)[0].text,
            "�"
        );
    }

    #[test]
    fn raw_template_scans_utf16_before_display_projection() {
        let render = |template: JavaString| {
            let component =
                RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "translate".into(),
                        value: NbtSummaryValue::String("missing".into()),
                    },
                    NbtSummaryEntry {
                        name: "fallback".into(),
                        value: template.to_nbt(),
                    },
                    NbtSummaryEntry {
                        name: "with".into(),
                        value: NbtSummaryValue::List(vec![NbtSummaryValue::String("ARG".into())]),
                    },
                ]))
                .unwrap();
            localized_component_runs(&component, &ComponentStyle::default(), None)
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
        };
        let template = JavaString::from(vec![
            0xd800,
            b'%' as u16,
            b's' as u16,
            b'\\' as u16,
            b'u' as u16,
            b'D' as u16,
            b'8' as u16,
            b'0' as u16,
            b'0' as u16,
        ]);
        assert_eq!(render(template), "�ARG\\uD800");
        let bad = JavaString::from(vec![0xd801, b'%' as u16, b'q' as u16]);
        assert_eq!(render(bad), "�%q");
    }

    use bbb_protocol::{ComponentClickEvent, ComponentHoverEvent};

    use super::*;

    #[test]
    fn localized_component_lists_inherit_root_without_leaking_member_overrides() {
        let component = RichTextComponent::try_from_component_nbt(
            bbb_protocol::json_value_to_nbt_summary(&serde_json::json!([
                {"text": "A", "bold": true, "font": "test:root", "extra": [
                    {"text": "B", "bold": false}, "C"
                ]},
                {"translate": "missing.test", "fallback": "%s%s", "with": [
                    [{"text": "D", "italic": true}, "E"], "F"
                ]}
            ]))
            .unwrap(),
        )
        .unwrap();
        let runs = localized_component_runs(&component, &ComponentStyle::default(), None);
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            ["A", "B", "C", "D", "E", "F"]
        );
        assert_eq!(
            runs.iter().map(|r| r.style.bold).collect::<Vec<_>>(),
            [
                Some(true),
                Some(false),
                Some(true),
                Some(true),
                Some(true),
                Some(true)
            ]
        );
        assert_eq!(
            runs.iter().map(|r| r.style.italic).collect::<Vec<_>>(),
            [None, None, None, Some(true), Some(true), None]
        );
        assert!(runs
            .iter()
            .all(|r| r.style.font.as_deref() == Some("test:root")));
        let world = localized_world_component_runs(&component, None);
        assert_eq!(
            world
                .iter()
                .flat_map(|r| r.text.chars().map(|_| r.style.bold))
                .collect::<Vec<_>>(),
            [true, false, true, true, true, true]
        );
        assert!(world.iter().all(|r| r.font.as_deref() == Some("test:root")));
    }

    #[test]
    fn localized_template_tokens_cover_sequential_indexed_escape_and_invalid_forms() {
        let mut parts = Vec::new();
        visit_localized_template("<%s> %2$s %%", 2, |part| parts.push(part));
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
            ]
        );
    }

    #[test]
    fn localized_template_errors_fall_back_atomically_to_original_part() {
        for template in [
            "%s then %q",
            "%s then %2$s",
            "%s then %",
            "%s then %0$s",
            "%s then %2147483648$s",
            "%s then %1$%",
            "%s then % s",
            "%s then %S",
        ] {
            let mut parts = Vec::new();
            visit_localized_template(template, 1, |part| parts.push(part));
            assert_eq!(
                parts,
                [LocalizedTemplatePart::Literal(template)],
                "{template}"
            );
        }
        let mut parts = Vec::new();
        visit_localized_template("%2$s%s%1$s%s", 2, |part| parts.push(part));
        assert_eq!(
            parts
                .iter()
                .filter_map(|part| match part {
                    LocalizedTemplatePart::Argument { index, .. } => Some(*index),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [1, 0, 0, 1]
        );
    }

    #[test]
    fn localized_component_keeps_template_and_argument_source_boundaries() {
        let component = RichTextComponent::try_from_component_nbt(
            bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
                "translate": "missing.parts", "fallback": "§%s%%tail", "font": "test:font",
                "with": [{"text": "aX", "extra": ["Y"]}], "extra": ["Z"]
            }))
            .unwrap(),
        )
        .unwrap();
        let runs = localized_component_runs(&component, &ComponentStyle::default(), None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
            ["§", "aX", "Y", "%", "tail", "Z"]
        );
        assert!(runs
            .iter()
            .all(|run| run.style.font.as_deref() == Some("test:font")));
    }

    #[test]
    fn keybind_component_resolves_the_currently_bound_key_with_id_fallback() {
        let options =
            crate::options::ClientOptions::defaults(crate::options::ClientOptionHost::default());
        crate::options::key_bindings::install_bound_key_display_keys(&options);

        let jump = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "keybind".into(),
                value: NbtSummaryValue::string("key.jump"),
            },
        ]))
        .unwrap();
        let runs = localized_component_runs(&jump, &ComponentStyle::default(), None);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "key.keyboard.space");

        let unknown = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "keybind".into(),
                value: NbtSummaryValue::string("key.not.registered"),
            },
        ]))
        .unwrap();
        let runs = localized_component_runs(&unknown, &ComponentStyle::default(), None);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "key.not.registered");
    }

    #[test]
    fn bound_key_display_keys_follow_rebinds_and_defaults() {
        let mut options =
            crate::options::ClientOptions::defaults(crate::options::ClientOptionHost::default());
        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.up");
        let table = crate::options::key_bindings::bound_key_display_keys(&options);
        assert_eq!(
            table.get(bbb_protocol::JavaString::from("key.forward").units()),
            Some(&"key.keyboard.up")
        );
        assert_eq!(
            table.get(bbb_protocol::JavaString::from("key.jump").units()),
            Some(&"key.keyboard.space")
        );
        assert_eq!(
            table.get(bbb_protocol::JavaString::from("key.attack").units()),
            Some(&"key.mouse.left")
        );
        assert_eq!(
            table.get(bbb_protocol::JavaString::from("key.smoothCamera").units()),
            Some(&"key.keyboard.unknown")
        );
    }

    #[test]
    fn localized_world_component_shadow_translates_arguments_without_merging_overrides() {
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            entry(
                "translate",
                NbtSummaryValue::String("missing.shadow".into()),
            ),
            entry("fallback", NbtSummaryValue::String("A%sC".into())),
            entry("shadow_color", NbtSummaryValue::Int(0x8012_3456_u32 as i32)),
            entry(
                "with",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("text", NbtSummaryValue::String("B".into())),
                    entry("shadow_color", NbtSummaryValue::Int(0)),
                ])]),
            ),
        ]))
        .unwrap();
        let runs = localized_world_component_runs(&component, None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "ABC"
        );
        assert_eq!(
            runs.iter().map(|run| run.shadow_color).collect::<Vec<_>>(),
            [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
        );
    }

    #[test]
    fn localized_component_arguments_retain_interactive_and_font_style() {
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string("missing.test"),
            },
            NbtSummaryEntry {
                name: "fallback".into(),
                value: NbtSummaryValue::string("Open %s"),
            },
            NbtSummaryEntry {
                name: "with".into(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::string("guide")]),
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
            shadow_color: Some(0x8034_5678),
            font: Some("example:book".to_string()),
            ..ComponentStyle::default()
        };

        let runs = localized_component_runs(&component, &inherited, None);

        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "Open guide"
        );
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].style.click_event, inherited.click_event);
        assert_eq!(runs[0].style.hover_event, inherited.hover_event);
        assert_eq!(runs[0].style.font, inherited.font);
        assert_eq!(runs[0].style.shadow_color, inherited.shadow_color);
    }
}
