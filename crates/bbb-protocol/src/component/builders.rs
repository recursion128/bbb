use super::*;

/// Vanilla `Component.literal(text)` as it leaves `ComponentSerialization.CODEC`.
///
/// The codec collapses a literal that carries no style and no siblings to a
/// bare string (`ComponentSerialization.java:107-114`, via
/// `Component.tryCollapseToString`), so an unstyled literal is a string tag
/// rather than a `{text:...}` compound.
pub fn component_literal_nbt(text: impl Into<crate::JavaString>) -> NbtSummaryValue {
    NbtSummaryValue::string(text)
}

/// One parsed component in the form `ComponentSerialization.CODEC` writes it.
///
/// The codec's `xmap` asks `tryCollapseToString` first
/// (`ComponentSerialization.java:107-113`), and that answers the text of a
/// `PlainTextContents` with no siblings and an empty style
/// (`Component.java:56-58`); everything else keeps the record form. A reader
/// that accepted `{text:"x"}` therefore has to answer `"x"` before the value
/// is stored or saved, or the same component reaches a save in a shape the
/// official server never writes.
///
/// The walk reaches the two fields whose members are components in their own
/// right -- `extra` and a translatable's `with` -- and leaves every other
/// field alone, because nothing else here knows which nested compounds are
/// components.
pub fn collapsed_component_nbt(value: &NbtSummaryValue) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = value else {
        if let NbtSummaryValue::List(items) = value {
            return NbtSummaryValue::List(items.iter().map(collapsed_component_nbt).collect());
        }
        return value.clone();
    };
    if let [only] = entries.as_slice() {
        if only.name == "text" && matches!(&only.value, NbtSummaryValue::String(_)) {
            return only.value.clone();
        }
    }
    NbtSummaryValue::Compound(
        entries
            .iter()
            .map(|entry| {
                let value = match &entry.value {
                    NbtSummaryValue::List(items)
                        if entry.name == "extra" || entry.name == "with" =>
                    {
                        NbtSummaryValue::List(items.iter().map(collapsed_component_nbt).collect())
                    }
                    _ => entry.value.clone(),
                };
                NbtSummaryEntry {
                    name: entry.name.clone(),
                    value,
                }
            })
            .collect(),
    )
}

/// Vanilla `Component.literal(text).withStyle(..)`.
///
/// A styled literal cannot collapse, so the codec writes the `text` field
/// beside the `Style.Serializer` keys.
pub fn component_styled_literal_nbt(
    text: impl Into<crate::JavaString>,
    style: &ComponentStyle,
) -> std::result::Result<NbtSummaryValue, String> {
    let mut entries = vec![nbt_entry("text", NbtSummaryValue::string(text))];
    entries.extend(style_entries_from_projection(style)?);
    Ok(NbtSummaryValue::Compound(entries))
}

/// Vanilla `Component.translatable(key, args)`.
///
/// `TranslatableContents.MAP_CODEC` (`TranslatableContents.java:36-44`) writes
/// `translate` plus an optional `with` list; an empty argument array is
/// omitted entirely (`adjustArgs`, :64-66). Numbers, booleans and strings stay
/// primitive arguments (`isAllowedPrimitiveArgument`, :60-62) — every other
/// argument is a nested Component, which is what the caller passes here.
pub fn component_translatable_nbt(
    key: impl Into<crate::JavaString>,
    arguments: Vec<NbtSummaryValue>,
    style: &ComponentStyle,
) -> std::result::Result<NbtSummaryValue, String> {
    let mut entries = vec![nbt_entry("translate", NbtSummaryValue::string(key))];
    if !arguments.is_empty() {
        entries.push(nbt_entry("with", NbtSummaryValue::List(arguments)));
    }
    entries.extend(style_entries_from_projection(style)?);
    Ok(NbtSummaryValue::Compound(entries))
}

/// Vanilla `Component.keybind(name)`.
///
/// `KeybindContents.MAP_CODEC` (`KeybindContents.java:15-17`) writes the
/// mapping name alone; the content resolves lazily through
/// `KeybindResolver.keyResolver` (`KeybindContents.java:26-31`), which the
/// client installs as `KeyMapping::createNameSupplier`
/// (`Minecraft.java:435`), so what the reader sees is the mapping's currently
/// bound key name.
pub fn component_keybind_nbt(
    name: impl Into<crate::JavaString>,
    style: &ComponentStyle,
) -> std::result::Result<NbtSummaryValue, String> {
    let mut entries = vec![nbt_entry("keybind", NbtSummaryValue::string(name))];
    entries.extend(style_entries_from_projection(style)?);
    Ok(NbtSummaryValue::Compound(entries))
}

/// Vanilla `Component.empty().append(..)` with an optional root style.
///
/// `MutableComponent` siblings serialize into `extra`
/// (`ComponentSerialization.java:102`). With no children the empty literal
/// collapses to the empty string exactly as [`component_literal_nbt`] does.
pub fn component_appended_nbt(
    style: &ComponentStyle,
    children: Vec<NbtSummaryValue>,
) -> std::result::Result<NbtSummaryValue, String> {
    let style_entries = style_entries_from_projection(style)?;
    if children.is_empty() && style_entries.is_empty() {
        return Ok(component_literal_nbt(""));
    }
    let mut entries = vec![nbt_entry(
        "text",
        NbtSummaryValue::String(String::new().into()),
    )];
    entries.extend(style_entries);
    if !children.is_empty() {
        entries.push(nbt_entry("extra", NbtSummaryValue::List(children)));
    }
    Ok(NbtSummaryValue::Compound(entries))
}

/// Vanilla `MutableComponent.append` (`MutableComponent.java:52-55`).
///
/// The siblings attach to the component itself rather than to a new empty
/// root, which is what `PlayerList.canPlayerLogin` does when it appends a ban
/// expiry to the ban reason. A collapsed string component grows the `text`
/// field it needs to carry `extra`; an existing `extra` list is extended.
pub fn component_with_siblings_nbt(
    component: NbtSummaryValue,
    siblings: Vec<NbtSummaryValue>,
) -> std::result::Result<NbtSummaryValue, String> {
    if siblings.is_empty() {
        return Ok(component);
    }
    let mut entries = match component {
        NbtSummaryValue::String(text) => {
            vec![nbt_entry("text", NbtSummaryValue::String(text))]
        }
        NbtSummaryValue::Compound(entries) => entries,
        _ => return Err("only a string or compound component can take siblings".to_string()),
    };
    match entries.iter_mut().find(|entry| entry.name == "extra") {
        Some(entry) => {
            let NbtSummaryValue::List(existing) = &mut entry.value else {
                return Err("component extra must be a list".to_string());
            };
            existing.extend(siblings);
        }
        None => entries.push(nbt_entry("extra", NbtSummaryValue::List(siblings))),
    }
    Ok(NbtSummaryValue::Compound(entries))
}

pub(super) fn style_entries_from_projection(
    style: &ComponentStyle,
) -> std::result::Result<Vec<NbtSummaryEntry>, String> {
    let mut entries = Vec::new();
    for (name, value) in [
        ("bold", style.bold),
        ("italic", style.italic),
        ("underlined", style.underlined),
        ("strikethrough", style.strikethrough),
        ("obfuscated", style.obfuscated),
    ] {
        if let Some(value) = value {
            entries.push(NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::Byte(i8::from(value)),
            });
        }
    }
    // `TextColor.serialize` (`TextColor.java:41-46`): a colour that carries a
    // name writes the name, and only an anonymous RGB colour falls back to the
    // uppercase `String.format("#%06X")` literal. Vanilla command feedback is
    // built from `ChatFormatting` constants, so its wire form spells `red`, not
    // `#FF5555`; both parse back to the same colour but the bytes differ.
    if let Some(color) = style.color {
        let value = color & 0xFF_FFFF;
        let serialized = match style.color_name {
            Some(name) => {
                if name.color() != Some(value) {
                    return Err(format!(
                        "component style colour {value:#08X} does not match its name {}",
                        name.as_str()
                    ));
                }
                name.as_str().to_string()
            }
            None => format!("#{value:06X}"),
        };
        entries.push(NbtSummaryEntry {
            name: "color".into(),
            value: NbtSummaryValue::String(serialized.into()),
        });
    } else if let Some(name) = style.color_name {
        return Err(format!(
            "component style names the colour {} without a resolved value",
            name.as_str()
        ));
    }
    if let Some(shadow_color) = style.shadow_color {
        entries.push(NbtSummaryEntry {
            name: "shadow_color".into(),
            value: NbtSummaryValue::Int(shadow_color as i32),
        });
    }
    if let Some(click_event) = style.click_event.as_ref() {
        entries.push(NbtSummaryEntry {
            name: "click_event".into(),
            value: click_event_nbt(click_event)?,
        });
    }
    if let Some(hover_event) = style.hover_event.as_ref() {
        entries.push(NbtSummaryEntry {
            name: "hover_event".into(),
            value: hover_event_nbt(hover_event)?,
        });
    }
    if let Some(insertion) = style.insertion.as_ref() {
        entries.push(NbtSummaryEntry {
            name: "insertion".into(),
            value: insertion.to_nbt(),
        });
    }
    if let Some(font) = style.font.as_ref() {
        entries.push(NbtSummaryEntry {
            name: "font".into(),
            value: NbtSummaryValue::String(font.clone().into()),
        });
    }
    Ok(entries)
}

fn click_event_nbt(
    click_event: &ComponentClickEvent,
) -> std::result::Result<NbtSummaryValue, String> {
    let (action, name, value) = match click_event {
        ComponentClickEvent::OpenFile { .. } => {
            return Err("open_file is not valid in a server-backed Style".to_string())
        }
        ComponentClickEvent::OpenUrl { url } => ("open_url", "url", url.to_nbt()),
        ComponentClickEvent::RunCommand { command } => ("run_command", "command", command.to_nbt()),
        ComponentClickEvent::SuggestCommand { command } => {
            ("suggest_command", "command", command.to_nbt())
        }
        ComponentClickEvent::ShowDialog { dialog } => ("show_dialog", "dialog", dialog.clone()),
        ComponentClickEvent::ChangePage { page } => {
            ("change_page", "page", NbtSummaryValue::Int(*page))
        }
        ComponentClickEvent::CopyToClipboard { value } => {
            ("copy_to_clipboard", "value", value.to_nbt())
        }
        ComponentClickEvent::Custom { id, payload } => {
            let mut entries = vec![
                nbt_entry("action", NbtSummaryValue::String("custom".into())),
                nbt_entry("id", NbtSummaryValue::String(id.clone().into())),
            ];
            if let Some(payload) = payload {
                entries.push(nbt_entry("payload", payload.clone()));
            }
            return Ok(NbtSummaryValue::Compound(entries));
        }
    };
    Ok(NbtSummaryValue::Compound(vec![
        nbt_entry("action", NbtSummaryValue::String(action.to_string().into())),
        nbt_entry(name, value),
    ]))
}

fn hover_event_nbt(
    hover_event: &ComponentHoverEvent,
) -> std::result::Result<NbtSummaryValue, String> {
    let mut entries = Vec::new();
    match hover_event {
        ComponentHoverEvent::ShowText { value, component } => {
            entries.push(nbt_entry(
                "action",
                NbtSummaryValue::String("show_text".into()),
            ));
            entries.push(nbt_entry(
                "value",
                component
                    .clone()
                    .map_or_else(|| styled_runs_component_nbt(value), Ok)?,
            ));
        }
        ComponentHoverEvent::ShowItem {
            id,
            count,
            components,
        } => {
            entries.push(nbt_entry(
                "action",
                NbtSummaryValue::String("show_item".into()),
            ));
            entries.push(nbt_entry("id", NbtSummaryValue::String(id.clone().into())));
            entries.push(nbt_entry("count", NbtSummaryValue::Int(*count)));
            if let Some(components) = components {
                entries.push(nbt_entry("components", components.clone()));
            }
        }
        ComponentHoverEvent::ShowEntity {
            id,
            uuid,
            name,
            name_component,
        } => {
            entries.push(nbt_entry(
                "action",
                NbtSummaryValue::String("show_entity".into()),
            ));
            entries.push(nbt_entry("id", NbtSummaryValue::String(id.clone().into())));
            entries.push(nbt_entry("uuid", uuid.clone()));
            if let Some(name_component) = name_component {
                entries.push(nbt_entry("name", name_component.clone()));
            } else if let Some(name) = name {
                entries.push(nbt_entry("name", styled_runs_component_nbt(name)?));
            }
        }
    }
    Ok(NbtSummaryValue::Compound(entries))
}

fn styled_runs_component_nbt(
    runs: &[StyledTextRun],
) -> std::result::Result<NbtSummaryValue, String> {
    if runs.is_empty() {
        return Ok(NbtSummaryValue::String(String::new().into()));
    }
    let mut components = Vec::with_capacity(runs.len());
    for run in runs {
        let mut entries = vec![nbt_entry(
            "text",
            NbtSummaryValue::String(run.text.clone().into()),
        )];
        entries.extend(style_entries_from_projection(&run.style)?);
        components.push(NbtSummaryValue::Compound(entries));
    }
    if components.len() == 1 {
        Ok(components.pop().expect("one component remains"))
    } else {
        Ok(NbtSummaryValue::List(components))
    }
}

pub(super) fn nbt_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
