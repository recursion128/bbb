use crate::{
    nbt_summary::{NbtSummaryEntry, NbtSummaryValue},
    packets::normalize_resource_location,
};

use super::{
    codec::{
        component_bool, component_i32, decoded_content_type, decoded_data_source_type,
        decoded_object_type, validate_component,
    },
    parse_text_color, validate_component_nbt,
};

/// Returns the exact UTF-16 length of vanilla 26.1's canonical JsonOps encoding when the
/// registry-independent Component model is sufficient to reproduce it.
///
/// `None` means that decoding succeeded but canonical re-encoding still needs a higher semantic
/// owner (a player profile, dialog, or item component patch) or Java Float/Double formatting. It
/// is deliberately not an estimate: callers enforcing `flatRestrictedCodec` must only act on an
/// exact `Some` result.
pub fn canonical_component_json_utf16_len(
    value: &NbtSummaryValue,
) -> Result<Option<usize>, String> {
    validate_component_nbt(value, "component")?;
    Ok(canonical_component(value).map(|value| value.utf16_len()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(&'static str, JsonValue)>),
    Map(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn utf16_len(&self) -> usize {
        match self {
            Self::Bool(true) => 4,
            Self::Bool(false) => 5,
            Self::Number(value) => value.len(),
            Self::String(value) => json_string_utf16_len(value),
            Self::Array(values) => collection_utf16_len(values.iter().map(Self::utf16_len)),
            Self::Object(fields) => {
                let entries = fields.iter().map(|(name, value)| {
                    json_string_utf16_len(name)
                        .saturating_add(1)
                        .saturating_add(value.utf16_len())
                });
                collection_utf16_len(entries)
            }
            Self::Map(fields) => {
                let entries = fields.iter().map(|(name, value)| {
                    json_string_utf16_len(name)
                        .saturating_add(1)
                        .saturating_add(value.utf16_len())
                });
                collection_utf16_len(entries)
            }
        }
    }
}

fn collection_utf16_len(entries: impl Iterator<Item = usize>) -> usize {
    entries.enumerate().fold(2usize, |length, (index, entry)| {
        length
            .saturating_add(usize::from(index != 0))
            .saturating_add(entry)
    })
}

fn canonical_component(value: &NbtSummaryValue) -> Option<JsonValue> {
    canonical_component_at(value, 0)
}

type ComponentRef<'a> = (&'a NbtSummaryValue, usize);

fn canonical_component_at(value: &NbtSummaryValue, depth: usize) -> Option<JsonValue> {
    canonical_component_with_siblings(value, depth, Vec::new())
}

fn canonical_component_with_siblings<'a>(
    value: &'a NbtSummaryValue,
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
) -> Option<JsonValue> {
    match value {
        NbtSummaryValue::String(text) => {
            if appended_siblings.is_empty() {
                Some(JsonValue::String(text.clone()))
            } else {
                Some(JsonValue::Object(vec![
                    ("text", JsonValue::String(text.clone())),
                    ("extra", canonical_component_list(&appended_siblings)?),
                ]))
            }
        }
        NbtSummaryValue::List(values) => {
            let (first, rest) = values.split_first()?;
            let mut siblings = Vec::with_capacity(rest.len() + appended_siblings.len());
            siblings.extend(rest.iter().map(|value| (value, depth + 1)));
            siblings.extend(appended_siblings);
            canonical_component_with_siblings(first, depth + 1, siblings)
        }
        NbtSummaryValue::Compound(entries) => {
            canonical_full_component(entries, depth, appended_siblings)
        }
        _ => None,
    }
}

fn canonical_full_component<'a>(
    entries: &'a [NbtSummaryEntry],
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
) -> Option<JsonValue> {
    let content_type = decoded_content_type(entries, "component", depth);
    let mut fields = canonical_content_fields(content_type, entries, depth)?;
    let mut siblings = match find_entry(entries, "extra") {
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(|value| (value, depth + 1))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    siblings.extend(appended_siblings);
    let style = canonical_style_fields(entries, depth)?;

    if content_type == "text" && siblings.is_empty() && style.is_empty() {
        let NbtSummaryValue::String(text) = find_entry(entries, "text")? else {
            unreachable!("plain text content was validated before canonical encoding")
        };
        return Some(JsonValue::String(text.clone()));
    }
    if !siblings.is_empty() {
        fields.push(("extra", canonical_component_list(&siblings)?));
    }
    fields.extend(style);
    Some(JsonValue::Object(fields))
}

fn canonical_component_list(values: &[ComponentRef<'_>]) -> Option<JsonValue> {
    values
        .iter()
        .map(|(value, depth)| canonical_component_at(value, *depth))
        .collect::<Option<Vec<_>>>()
        .map(JsonValue::Array)
}

fn canonical_content_fields(
    content_type: &str,
    entries: &[NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, JsonValue)>> {
    let mut fields = Vec::new();
    match content_type {
        "text" => fields.push(("text", string_field(entries, "text")?)),
        "translatable" => {
            fields.push(("translate", string_field(entries, "translate")?));
            if let Some(NbtSummaryValue::String(fallback)) = find_entry(entries, "fallback") {
                fields.push(("fallback", JsonValue::String(fallback.clone())));
            }
            if let Some(NbtSummaryValue::List(arguments)) = find_entry(entries, "with") {
                if !arguments.is_empty() {
                    let arguments = arguments
                        .iter()
                        .map(|argument| canonical_translation_argument(argument, depth + 1))
                        .collect::<Option<Vec<_>>>()?;
                    fields.push(("with", JsonValue::Array(arguments)));
                }
            }
        }
        "keybind" => fields.push(("keybind", string_field(entries, "keybind")?)),
        "score" => {
            let NbtSummaryValue::Compound(score) = find_entry(entries, "score")? else {
                return None;
            };
            fields.push((
                "score",
                JsonValue::Object(vec![
                    ("name", string_field(score, "name")?),
                    ("objective", string_field(score, "objective")?),
                ]),
            ));
        }
        "selector" => {
            fields.push(("selector", string_field(entries, "selector")?));
            if let Some(separator) = find_entry(entries, "separator") {
                fields.push(("separator", canonical_component_at(separator, depth + 1)?));
            }
        }
        "nbt" => {
            fields.push(("nbt", string_field(entries, "nbt")?));
            if find_entry(entries, "interpret")
                .and_then(component_bool)
                .unwrap_or(false)
            {
                fields.push(("interpret", JsonValue::Bool(true)));
            }
            if find_entry(entries, "plain")
                .and_then(component_bool)
                .unwrap_or(false)
            {
                fields.push(("plain", JsonValue::Bool(true)));
            }
            if let Some(separator) = find_entry(entries, "separator") {
                if validate_component(separator, "component separator", depth + 1).is_ok() {
                    fields.push(("separator", canonical_component_at(separator, depth + 1)?));
                }
            }
            match decoded_data_source_type(entries, "component") {
                "entity" => fields.push(("entity", string_field(entries, "entity")?)),
                "block" => fields.push(("block", string_field(entries, "block")?)),
                "storage" => {
                    fields.push(("storage", normalized_identifier_field(entries, "storage")?))
                }
                _ => unreachable!("component data source was validated before canonical encoding"),
            }
        }
        "object" => {
            match decoded_object_type(entries, "component") {
                "atlas" => {
                    let atlas = match find_entry(entries, "atlas") {
                        Some(_) => normalized_identifier_field(entries, "atlas")?,
                        None => JsonValue::String("minecraft:blocks".to_string()),
                    };
                    if atlas != JsonValue::String("minecraft:blocks".to_string()) {
                        fields.push(("atlas", atlas));
                    }
                    fields.push(("sprite", normalized_identifier_field(entries, "sprite")?));
                }
                // `ResolvableProfile.CODEC` is owned by the world layer and may normalize the
                // compact/full alternatives before JsonOps encoding.
                "player" => return None,
                _ => unreachable!("component object was validated before canonical encoding"),
            }
            if let Some(fallback) = find_entry(entries, "fallback") {
                fields.push(("fallback", canonical_component_at(fallback, depth + 1)?));
            }
        }
        _ => unreachable!("component content was validated before canonical encoding"),
    }
    Some(fields)
}

fn canonical_translation_argument(value: &NbtSummaryValue, depth: usize) -> Option<JsonValue> {
    match value {
        NbtSummaryValue::Byte(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Short(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Int(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Long(value) => Some(number(*value)),
        // Java Float/Double.toString has observable shortest-decimal rules that differ from
        // Rust's formatter at exponent and subnormal edges. Keep the result explicitly unknown
        // until that codec-owned representation is implemented exactly.
        NbtSummaryValue::Float(_) | NbtSummaryValue::Double(_) => None,
        NbtSummaryValue::String(value) => Some(JsonValue::String(value.clone())),
        _ => canonical_component_at(value, depth),
    }
}

fn canonical_style_fields(
    entries: &[NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, JsonValue)>> {
    let mut fields = Vec::new();
    if let Some(NbtSummaryValue::String(color)) = find_entry(entries, "color") {
        let encoded = if color.starts_with('#') {
            format!("#{:06X}", parse_text_color(color)?)
        } else {
            color.clone()
        };
        fields.push(("color", JsonValue::String(encoded)));
    }
    if let Some(shadow) = find_entry(entries, "shadow_color") {
        // `ARGB_COLOR_CODEC` always re-encodes through its primary `Codec.INT` branch. Numeric
        // NBT values and the vector4f alternative therefore both normalize to one signed int.
        let shadow = canonical_argb_color(shadow)?;
        fields.push(("shadow_color", number(i64::from(shadow))));
    }
    for field in [
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
    ] {
        if let Some(value) = find_entry(entries, field) {
            fields.push((field, JsonValue::Bool(component_bool(value)?)));
        }
    }
    if let Some(click_event) = find_entry(entries, "click_event") {
        fields.push(("click_event", canonical_click_event(click_event)?));
    }
    if let Some(hover_event) = find_entry(entries, "hover_event") {
        fields.push(("hover_event", canonical_hover_event(hover_event, depth)?));
    }
    if let Some(NbtSummaryValue::String(insertion)) = find_entry(entries, "insertion") {
        fields.push(("insertion", JsonValue::String(insertion.clone())));
    }
    if find_entry(entries, "font").is_some() {
        fields.push(("font", normalized_identifier_field(entries, "font")?));
    }
    Some(fields)
}

fn canonical_click_event(value: &NbtSummaryValue) -> Option<JsonValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![("action", JsonValue::String(action.to_string()))];
    match action {
        "open_url" => fields.push(("url", string_field(entries, "url")?)),
        "run_command" | "suggest_command" => {
            fields.push(("command", string_field(entries, "command")?));
        }
        "show_dialog" => return None,
        "change_page" => fields.push((
            "page",
            number(i64::from(component_i32(find_entry(entries, "page")?)?)),
        )),
        "copy_to_clipboard" => fields.push(("value", string_field(entries, "value")?)),
        "custom" => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            if let Some(payload) = find_entry(entries, "payload") {
                fields.push(("payload", canonical_nbt(payload)?));
            }
        }
        _ => return None,
    }
    Some(JsonValue::Object(fields))
}

fn canonical_hover_event(value: &NbtSummaryValue, depth: usize) -> Option<JsonValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![("action", JsonValue::String(action.to_string()))];
    match action {
        "show_text" => fields.push((
            "value",
            canonical_component_at(find_entry(entries, "value")?, depth + 1)?,
        )),
        "show_item" => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            if let Some(count) = find_entry(entries, "count").and_then(component_i32) {
                if count != 1 {
                    fields.push(("count", number(i64::from(count))));
                }
            }
            if find_entry(entries, "components").is_some() {
                return None;
            }
        }
        "show_entity" => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            fields.push(("uuid", canonical_uuid(find_entry(entries, "uuid")?)?));
            if let Some(name) = find_entry(entries, "name") {
                fields.push(("name", canonical_component_at(name, depth + 1)?));
            }
        }
        _ => return None,
    }
    Some(JsonValue::Object(fields))
}

fn canonical_uuid(value: &NbtSummaryValue) -> Option<JsonValue> {
    let values = match value {
        NbtSummaryValue::IntArray(values) if values.len() == 4 => values.clone(),
        NbtSummaryValue::ByteArray(values) if values.len() == 4 => {
            values.iter().map(|value| i32::from(*value)).collect()
        }
        NbtSummaryValue::LongArray(values) if values.len() == 4 => {
            values.iter().map(|value| *value as i32).collect()
        }
        NbtSummaryValue::List(values) if values.len() == 4 => values
            .iter()
            .map(component_i32)
            .collect::<Option<Vec<_>>>()?,
        NbtSummaryValue::String(value) => java_uuid_int_array(value)?,
        _ => return None,
    };
    Some(JsonValue::Array(
        values
            .into_iter()
            .map(|value| number(i64::from(value)))
            .collect(),
    ))
}

fn canonical_argb_color(value: &NbtSummaryValue) -> Option<i32> {
    if let Some(value) = component_i32(value) {
        return Some(value);
    }
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    let [red, green, blue, alpha] = values.as_slice() else {
        return None;
    };
    let channel = |value| -> Option<u32> {
        Some(((component_f32(value)? * 255.0).floor() as i32 as u32) & 0xff)
    };
    Some(
        ((channel(alpha)? << 24) | (channel(red)? << 16) | (channel(green)? << 8) | channel(blue)?)
            as i32,
    )
}

fn component_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

fn canonical_nbt(value: &NbtSummaryValue) -> Option<JsonValue> {
    Some(match value {
        NbtSummaryValue::Byte(value) => number(i64::from(*value)),
        NbtSummaryValue::Short(value) => number(i64::from(*value)),
        NbtSummaryValue::Int(value) => number(i64::from(*value)),
        NbtSummaryValue::Long(value) => number(*value),
        NbtSummaryValue::Float(_) | NbtSummaryValue::Double(_) => return None,
        NbtSummaryValue::ByteArray(values) => JsonValue::Array(
            values
                .iter()
                .map(|value| number(i64::from(*value)))
                .collect(),
        ),
        NbtSummaryValue::String(value) => JsonValue::String(value.clone()),
        NbtSummaryValue::List(values) => JsonValue::Array(
            values
                .iter()
                .map(canonical_nbt)
                .collect::<Option<Vec<_>>>()?,
        ),
        NbtSummaryValue::Compound(entries) => JsonValue::Map(canonical_nbt_map(entries)?),
        NbtSummaryValue::IntArray(values) => JsonValue::Array(
            values
                .iter()
                .map(|value| number(i64::from(*value)))
                .collect(),
        ),
        NbtSummaryValue::LongArray(values) => {
            JsonValue::Array(values.iter().map(|value| number(*value)).collect())
        }
    })
}

fn canonical_nbt_map(entries: &[NbtSummaryEntry]) -> Option<Vec<(String, JsonValue)>> {
    let mut normalized = Vec::<(String, JsonValue)>::with_capacity(entries.len());
    for entry in entries {
        let value = canonical_nbt(&entry.value)?;
        if let Some((_, existing)) = normalized.iter_mut().find(|(name, _)| name == &entry.name) {
            *existing = value;
        } else {
            normalized.push((entry.name.clone(), value));
        }
    }
    Some(normalized)
}

fn java_uuid_int_array(value: &str) -> Option<Vec<i32>> {
    let parts = value
        .split('-')
        .map(|part| u64::from_str_radix(part.strip_prefix('+').unwrap_or(part), 16).ok())
        .collect::<Option<Vec<_>>>()?;
    let [time_low, time_mid, version, variant, node] = parts.as_slice() else {
        return None;
    };
    let most = (time_low & 0xffff_ffff) << 32 | (time_mid & 0xffff) << 16 | version & 0xffff;
    let least = (variant & 0xffff) << 48 | node & 0xffff_ffff_ffff;
    Some(vec![
        (most >> 32) as u32 as i32,
        most as u32 as i32,
        (least >> 32) as u32 as i32,
        least as u32 as i32,
    ])
}

fn normalized_identifier_field(entries: &[NbtSummaryEntry], field: &str) -> Option<JsonValue> {
    let value = string_value(find_entry(entries, field)?)?;
    normalize_resource_location(value.to_string())
        .ok()
        .map(JsonValue::String)
}

fn string_field(entries: &[NbtSummaryEntry], field: &str) -> Option<JsonValue> {
    string_value(find_entry(entries, field)?).map(|value| JsonValue::String(value.to_string()))
}

fn string_value(value: &NbtSummaryValue) -> Option<&str> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(value)
}

fn number(value: i64) -> JsonValue {
    JsonValue::Number(value.to_string())
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], field: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
}

fn json_string_utf16_len(value: &str) -> usize {
    value.chars().fold(2usize, |length, character| {
        length.saturating_add(match character {
            '"' | '\\' | '\u{0008}' | '\u{0009}' | '\n' | '\u{000c}' | '\r' => 2,
            '\u{0000}'..='\u{001f}' | '\u{2028}' | '\u{2029}' => 6,
            character => character.len_utf16(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_special_roots_defaults_and_unknown_fields() {
        assert_eq!(exact(&string("text")), r#""text""#.len());
        assert_eq!(
            exact(&compound(vec![
                ("type", string("text")),
                ("text", string("text")),
                ("unknown", string("ignored")),
            ])),
            r#""text""#.len()
        );
        assert_eq!(
            exact(&list(vec![string("first"), string("second")])),
            r#"{"text":"first","extra":["second"]}"#.len()
        );
        assert_eq!(
            exact(&compound(vec![
                ("translate", string("key")),
                ("fallback", NbtSummaryValue::Int(1)),
                ("with", list(Vec::new())),
                ("bold", NbtSummaryValue::Byte(0)),
            ])),
            r#"{"translate":"key","bold":false}"#.len()
        );
    }

    #[test]
    fn canonicalizes_identifiers_colors_events_and_uuid() {
        let value = compound(vec![
            ("object", string("atlas")),
            ("atlas", string("blocks")),
            ("sprite", string("item/diamond")),
            ("color", string("#1a2B3c")),
            ("font", string("uniform")),
            (
                "hover_event",
                compound(vec![
                    ("action", string("show_entity")),
                    ("id", string("pig")),
                    ("uuid", string("1-2-3-4-5")),
                ]),
            ),
        ]);
        assert_eq!(
            exact(&value),
            concat!(
                r##"{"sprite":"minecraft:item/diamond","color":"#1A2B3C","hover_event":{"action":"show_entity","id":"minecraft:pig","uuid":[1,131075,262144,5]},"font":"minecraft:uniform"}"##
            )
            .len()
        );
    }

    #[test]
    fn canonicalizes_vector_shadow_and_custom_nbt_payload() {
        let shadow = list(vec![
            NbtSummaryValue::Float(1.0f32.to_bits()),
            NbtSummaryValue::Double(0.5f64.to_bits()),
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Byte(1),
        ]);
        assert_eq!(canonical_argb_color(&shadow), Some(0xffff_7f00u32 as i32));
        let value = compound(vec![
            ("text", string("x")),
            ("shadow_color", shadow),
            (
                "click_event",
                compound(vec![
                    ("action", string("custom")),
                    ("id", string("test")),
                    (
                        "payload",
                        compound(vec![
                            ("a", NbtSummaryValue::Int(1)),
                            ("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
                            ("nested", list(vec![string("z")])),
                        ]),
                    ),
                ]),
            ),
        ]);
        assert_eq!(
            exact(&value),
            r#"{"text":"x","shadow_color":-33024,"click_event":{"action":"custom","id":"minecraft:test","payload":{"a":1,"bytes":[-1,2],"nested":["z"]}}}"#.len()
        );
    }

    #[test]
    fn canonicalizes_duplicate_component_and_payload_keys_last_wins() {
        let value = compound(vec![("text", string("first")), ("text", string("last"))]);
        assert_eq!(exact(&value), r#""last""#.len());

        let custom = compound(vec![
            ("text", string("x")),
            (
                "click_event",
                compound(vec![
                    ("action", string("custom")),
                    ("id", string("test")),
                    (
                        "payload",
                        compound(vec![
                            ("key", NbtSummaryValue::Int(1)),
                            ("key", string("last")),
                        ]),
                    ),
                ]),
            ),
        ]);
        assert_eq!(
            exact(&custom),
            r#"{"text":"x","click_event":{"action":"custom","id":"minecraft:test","payload":{"key":"last"}}}"#.len()
        );
    }

    #[test]
    fn reports_owner_and_non_integral_java_values_as_unknown() {
        for value in [
            compound(vec![
                ("object", string("player")),
                ("player", string("Alex")),
            ]),
            compound(vec![
                ("text", string("item")),
                (
                    "hover_event",
                    compound(vec![
                        ("action", string("show_item")),
                        ("id", string("stone")),
                        ("components", compound(Vec::new())),
                    ]),
                ),
            ]),
            compound(vec![
                ("translate", string("key")),
                ("with", list(vec![NbtSummaryValue::Float(1.0f32.to_bits())])),
            ]),
            compound(vec![
                ("text", string("custom")),
                (
                    "click_event",
                    compound(vec![
                        ("action", string("custom")),
                        ("id", string("test")),
                        ("payload", NbtSummaryValue::Double(1.0f64.to_bits())),
                    ]),
                ),
            ]),
        ] {
            assert_eq!(canonical_component_json_utf16_len(&value).unwrap(), None);
        }
    }

    fn exact(value: &NbtSummaryValue) -> usize {
        canonical_component_json_utf16_len(value)
            .unwrap()
            .expect("component should have an exact canonical encoding")
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string())
    }

    fn list(values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
        NbtSummaryValue::List(values)
    }

    fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.to_string(),
                    value,
                })
                .collect(),
        )
    }
}
