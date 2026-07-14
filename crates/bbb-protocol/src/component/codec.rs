use crate::{
    nbt_summary::{NbtSummaryEntry, NbtSummaryValue},
    packets::normalize_resource_location,
};

use super::parse_text_color;

const MAX_COMPONENT_DEPTH: usize = 64;
const CONTENT_TYPES: [&str; 7] = [
    "text",
    "translatable",
    "keybind",
    "score",
    "selector",
    "nbt",
    "object",
];

/// Validates the registry-independent structural portion of vanilla 26.1
/// `ComponentSerialization.CODEC` over a decoded network-NBT value.
///
/// This owns the recursive root alternatives, the legacy fuzzy/typed content
/// matcher, simple content fields, nested component positions, `extra`, and
/// scalar style fields. Command parsers, click/hover payloads, registry-aware
/// values, and canonical JsonOps encoding are intentionally separate stages.
pub fn validate_component_nbt(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    validate_component(value, what, 0)
}

fn validate_component(value: &NbtSummaryValue, what: &str, depth: usize) -> Result<(), String> {
    if depth > MAX_COMPONENT_DEPTH {
        return Err(format!(
            "{what} component nesting exceeds {MAX_COMPONENT_DEPTH}"
        ));
    }
    match value {
        NbtSummaryValue::String(_) => Ok(()),
        NbtSummaryValue::List(values) => {
            if values.is_empty() {
                return Err(format!("{what} component list is empty"));
            }
            for (index, value) in values.iter().enumerate() {
                validate_component(value, &format!("{what}[{index}]"), depth + 1)?;
            }
            Ok(())
        }
        NbtSummaryValue::Compound(entries) => validate_full_component(entries, what, depth),
        _ => Err(format!("{what} is not a component")),
    }
}

fn validate_full_component(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    validate_contents(entries, what, depth)?;
    if let Some(extra) = find_entry(entries, "extra") {
        let NbtSummaryValue::List(values) = extra else {
            return Err(format!("{what} field extra is not a list"));
        };
        if values.is_empty() {
            return Err(format!("{what} field extra is empty"));
        }
        for (index, value) in values.iter().enumerate() {
            validate_component(value, &format!("{what} field extra[{index}]"), depth + 1)?;
        }
    }
    validate_scalar_style(entries, what)
}

fn validate_contents(entries: &[NbtSummaryEntry], what: &str, depth: usize) -> Result<(), String> {
    if let Some(value) = find_entry(entries, "type") {
        let NbtSummaryValue::String(content_type) = value else {
            return Err(format!("{what} field type is not a string"));
        };
        if !CONTENT_TYPES.contains(&content_type.as_str()) {
            return Err(format!(
                "{what} has unknown component content type {content_type}"
            ));
        }
        return validate_content_type(content_type, entries, what, depth);
    }

    for content_type in CONTENT_TYPES {
        if validate_content_type(content_type, entries, what, depth).is_ok() {
            return Ok(());
        }
    }
    Err(format!("{what} has no matching component content codec"))
}

fn validate_content_type(
    content_type: &str,
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    match content_type {
        "text" => {
            required_string(entries, "text", what)?;
            Ok(())
        }
        "translatable" => validate_translatable(entries, what, depth),
        "keybind" => {
            required_string(entries, "keybind", what)?;
            Ok(())
        }
        "score" => validate_score(entries, what),
        "selector" => validate_selector(entries, what, depth),
        "nbt" => validate_nbt_contents(entries, what, depth),
        "object" => validate_object_contents(entries, what, depth),
        _ => unreachable!("content type is checked before dispatch"),
    }
}

fn validate_translatable(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    required_string(entries, "translate", what)?;
    // `fallback` is lenientOptionalFieldOf: an invalid value is treated as
    // absent rather than failing the whole component.
    if let Some(with) = find_entry(entries, "with") {
        let NbtSummaryValue::List(arguments) = with else {
            return Err(format!("{what} field with is not a list"));
        };
        for (index, argument) in arguments.iter().enumerate() {
            if is_java_primitive(argument) {
                continue;
            }
            validate_component(argument, &format!("{what} field with[{index}]"), depth + 1)?;
        }
    }
    Ok(())
}

fn validate_score(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let score = required_entry(entries, "score", what)?;
    let NbtSummaryValue::Compound(score_entries) = score else {
        return Err(format!("{what} field score is not a compound"));
    };
    required_string(score_entries, "name", &format!("{what} field score"))?;
    required_string(score_entries, "objective", &format!("{what} field score"))?;
    Ok(())
}

fn validate_selector(entries: &[NbtSummaryEntry], what: &str, depth: usize) -> Result<(), String> {
    required_string(entries, "selector", what)?;
    if let Some(separator) = find_entry(entries, "separator") {
        validate_component(separator, &format!("{what} field separator"), depth + 1)?;
    }
    Ok(())
}

fn validate_nbt_contents(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    required_string(entries, "nbt", what)?;
    let interpreting = find_entry(entries, "interpret")
        .and_then(component_bool)
        .unwrap_or(false);
    let plain = find_entry(entries, "plain")
        .and_then(component_bool)
        .unwrap_or(false);
    if interpreting && plain {
        return Err(format!(
            "{what} fields interpret and plain cannot both be enabled"
        ));
    }
    // `interpret`, `plain`, and `separator` are lenient optionals. Preserve
    // vanilla's decode behavior by ignoring malformed optional values here.
    if let Some(separator) = find_entry(entries, "separator") {
        let _ = validate_component(separator, &format!("{what} field separator"), depth + 1);
    }
    validate_data_source(entries, what)
}

fn validate_data_source(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    if let Some(source) = find_entry(entries, "source") {
        let NbtSummaryValue::String(source) = source else {
            return Err(format!("{what} field source is not a string"));
        };
        return match source.as_str() {
            "block" => required_string(entries, "block", what).map(|_| ()),
            "entity" => required_string(entries, "entity", what).map(|_| ()),
            "storage" => validate_identifier_field(entries, "storage", what),
            _ => Err(format!("{what} has unknown component data source {source}")),
        };
    }
    if matches!(
        find_entry(entries, "block"),
        Some(NbtSummaryValue::String(_))
    ) || matches!(
        find_entry(entries, "entity"),
        Some(NbtSummaryValue::String(_))
    ) || validate_identifier_field(entries, "storage", what).is_ok()
    {
        return Ok(());
    }
    Err(format!("{what} has no matching component data source"))
}

fn validate_object_contents(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    validate_object_info(entries, what)?;
    if let Some(fallback) = find_entry(entries, "fallback") {
        validate_component(fallback, &format!("{what} field fallback"), depth + 1)?;
    }
    Ok(())
}

fn validate_object_info(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    if let Some(object) = find_entry(entries, "object") {
        let NbtSummaryValue::String(object) = object else {
            return Err(format!("{what} field object is not a string"));
        };
        return match object.as_str() {
            "atlas" => validate_atlas_object(entries, what),
            "player" => validate_player_object(entries, what),
            _ => Err(format!("{what} has unknown component object type {object}")),
        };
    }
    if validate_atlas_object(entries, what).is_ok() || validate_player_object(entries, what).is_ok()
    {
        return Ok(());
    }
    Err(format!("{what} has no matching component object codec"))
}

fn validate_atlas_object(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    if find_entry(entries, "atlas").is_some() {
        validate_identifier_field(entries, "atlas", what)?;
    }
    validate_identifier_field(entries, "sprite", what)
}

fn validate_player_object(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    match required_entry(entries, "player", what)? {
        NbtSummaryValue::String(_) | NbtSummaryValue::Compound(_) => {}
        _ => return Err(format!("{what} field player is not a profile")),
    }
    if let Some(hat) = find_entry(entries, "hat") {
        component_bool(hat).ok_or_else(|| format!("{what} field hat is not a boolean"))?;
    }
    Ok(())
}

fn validate_scalar_style(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    if let Some(color) = find_entry(entries, "color") {
        let NbtSummaryValue::String(color) = color else {
            return Err(format!("{what} field color is not a string"));
        };
        if parse_text_color(color).is_none() {
            return Err(format!("{what} field color is invalid: {color}"));
        }
    }
    if let Some(shadow) = find_entry(entries, "shadow_color") {
        validate_argb_color(shadow, &format!("{what} field shadow_color"))?;
    }
    for field in [
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
    ] {
        if let Some(value) = find_entry(entries, field) {
            component_bool(value)
                .ok_or_else(|| format!("{what} field {field} is not a boolean"))?;
        }
    }
    if let Some(insertion) = find_entry(entries, "insertion") {
        if !matches!(insertion, NbtSummaryValue::String(_)) {
            return Err(format!("{what} field insertion is not a string"));
        }
    }
    if find_entry(entries, "font").is_some() {
        validate_identifier_field(entries, "font", what)?;
    }
    Ok(())
}

fn validate_argb_color(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    if is_numeric(value) {
        return Ok(());
    }
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is neither an integer nor a vector4f"));
    };
    if values.len() != 4 || !values.iter().all(is_numeric) {
        return Err(format!("{what} is not a four-number vector4f"));
    }
    Ok(())
}

fn validate_identifier_field(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<(), String> {
    let raw = required_string(entries, field, what)?;
    normalize_resource_location(raw.to_string())
        .map(|_| ())
        .map_err(|_| format!("{what} field {field} is not a valid identifier: {raw}"))
}

fn component_bool(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => (*value as i8) != 0,
        NbtSummaryValue::Int(value) => (*value as i8) != 0,
        NbtSummaryValue::Long(value) => (*value as i8) != 0,
        NbtSummaryValue::Float(bits) => (f32::from_bits(*bits) as i32 as i8) != 0,
        NbtSummaryValue::Double(bits) => (f64::from_bits(*bits) as i32 as i8) != 0,
        _ => return None,
    })
}

fn is_java_primitive(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::String(_)) || is_numeric(value)
}

fn is_numeric(value: &NbtSummaryValue) -> bool {
    matches!(
        value,
        NbtSummaryValue::Byte(_)
            | NbtSummaryValue::Short(_)
            | NbtSummaryValue::Int(_)
            | NbtSummaryValue::Long(_)
            | NbtSummaryValue::Float(_)
            | NbtSummaryValue::Double(_)
    )
}

fn required_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    find_entry(entries, field).ok_or_else(|| format!("{what} is missing field {field}"))
}

fn required_string<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a str, String> {
    let value = required_entry(entries, field, what)?;
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} field {field} is not a string"));
    };
    Ok(value)
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], field: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests;
