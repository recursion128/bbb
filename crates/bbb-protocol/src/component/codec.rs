use url::Url;

use crate::{
    data_component_types::vanilla_data_component_type_for_resource_id,
    entity_types::vanilla_entity_type_for_resource_id,
    item_types::{
        vanilla_item_resource_id_matches_type, vanilla_item_type_for_resource_id,
        VANILLA_ITEM_AIR_ID,
    },
    nbt_summary::{NbtSummaryEntry, NbtSummaryValue},
    packets::normalize_resource_location,
};

use super::{
    command::{is_valid_block_position, is_valid_entity_selector, is_valid_nbt_path},
    parse_text_color, ComponentClickEvent,
};

pub(super) const MAX_COMPONENT_DEPTH: usize = 64;
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
/// style fields and the registry-independent portion of click/hover events.
/// Command parsers, deep dialog/item payloads, registry-aware values, and
/// canonical JsonOps encoding are intentionally separate stages.
pub fn validate_component_nbt(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    validate_component(value, what, 0)
}

/// Registry- or owner-sensitive payloads nested inside an otherwise validated
/// vanilla Component. Their envelope belongs to the Component codec; decoding
/// the payload value belongs to the higher semantic owner named by the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentPayload<'a> {
    PlayerProfile(&'a NbtSummaryValue),
    ShowDialog(&'a NbtSummaryValue),
    ShowItemComponents(&'a NbtSummaryValue),
}

/// Validates a Component, then visits payloads whose value codecs require a
/// higher-level owner (Dialog or persistent data components).
pub fn visit_component_payloads<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
    mut visitor: impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    validate_component_nbt(value, what)?;
    visit_component_payloads_inner(value, what, 0, &mut visitor)
}

pub(super) fn validate_component_style_nbt(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    validate_style(entries, what, 0)
}

pub(super) fn validate_component(
    value: &NbtSummaryValue,
    what: &str,
    depth: usize,
) -> Result<(), String> {
    #[cfg(test)]
    crate::test_stack_budget::record();

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

fn visit_component_payloads_inner<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
    depth: usize,
    visitor: &mut impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    #[cfg(test)]
    crate::test_stack_budget::record();

    match value {
        NbtSummaryValue::String(_) => Ok(()),
        NbtSummaryValue::List(values) => {
            for (index, value) in values.iter().enumerate() {
                visit_component_payloads_inner(
                    value,
                    &format!("{what}[{index}]"),
                    depth + 1,
                    visitor,
                )?;
            }
            Ok(())
        }
        NbtSummaryValue::Compound(entries) => {
            visit_content_payloads(entries, what, depth, visitor)?;
            if let Some(NbtSummaryValue::List(values)) = find_entry(entries, "extra") {
                for (index, value) in values.iter().enumerate() {
                    visit_component_payloads_inner(
                        value,
                        &format!("{what} field extra[{index}]"),
                        depth + 1,
                        visitor,
                    )?;
                }
            }
            visit_style_payloads(entries, what, depth, visitor)
        }
        _ => unreachable!("component was validated before payload traversal"),
    }
}

fn visit_content_payloads<'a>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    visitor: &mut impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    let content_type = decoded_content_type(entries, what, depth);
    match content_type {
        "translatable" => {
            if let Some(NbtSummaryValue::List(arguments)) = find_entry(entries, "with") {
                for (index, argument) in arguments.iter().enumerate() {
                    if !is_java_primitive(argument) {
                        visit_component_payloads_inner(
                            argument,
                            &format!("{what} field with[{index}]"),
                            depth + 1,
                            visitor,
                        )?;
                    }
                }
            }
        }
        "selector" => {
            if let Some(separator) = find_entry(entries, "separator") {
                visit_component_payloads_inner(
                    separator,
                    &format!("{what} field separator"),
                    depth + 1,
                    visitor,
                )?;
            }
        }
        "nbt" => {
            if let Some(separator) = find_entry(entries, "separator") {
                let nested_what = format!("{what} field separator");
                if validate_component(separator, &nested_what, depth + 1).is_ok() {
                    visit_component_payloads_inner(separator, &nested_what, depth + 1, visitor)?;
                }
            }
        }
        "object" => {
            if decoded_object_type(entries, what) == "player" {
                visitor(
                    ComponentPayload::PlayerProfile(
                        find_entry(entries, "player")
                            .expect("player object payload was validated before traversal"),
                    ),
                    &format!("{what} field player"),
                )?;
            }
            if let Some(fallback) = find_entry(entries, "fallback") {
                visit_component_payloads_inner(
                    fallback,
                    &format!("{what} field fallback"),
                    depth + 1,
                    visitor,
                )?;
            }
        }
        "text" | "keybind" | "score" => {}
        _ => unreachable!("component content type was validated before payload traversal"),
    }
    Ok(())
}

pub(super) fn decoded_content_type<'a>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> &'a str {
    if let Some(NbtSummaryValue::String(content_type)) = find_entry(entries, "type") {
        return content_type;
    }
    CONTENT_TYPES
        .into_iter()
        .find(|content_type| validate_content_type(content_type, entries, what, depth).is_ok())
        .expect("component content type was validated before payload traversal")
}

fn visit_style_payloads<'a>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    visitor: &mut impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(NbtSummaryValue::Compound(click_entries)) = find_entry(entries, "click_event") {
        if matches!(
            find_entry(click_entries, "action"),
            Some(NbtSummaryValue::String(action)) if action == "show_dialog"
        ) {
            let payload_what = format!("{what} field click_event field dialog");
            visitor(
                ComponentPayload::ShowDialog(
                    find_entry(click_entries, "dialog")
                        .expect("show-dialog payload was validated before traversal"),
                ),
                &payload_what,
            )?;
        }
    }
    let Some(NbtSummaryValue::Compound(hover_entries)) = find_entry(entries, "hover_event") else {
        return Ok(());
    };
    match find_entry(hover_entries, "action") {
        Some(NbtSummaryValue::String(action)) if action == "show_text" => {
            visit_component_payloads_inner(
                find_entry(hover_entries, "value")
                    .expect("show-text payload was validated before traversal"),
                &format!("{what} field hover_event field value"),
                depth + 1,
                visitor,
            )?;
        }
        Some(NbtSummaryValue::String(action)) if action == "show_item" => {
            if let Some(components) = find_entry(hover_entries, "components") {
                let payload_what = format!("{what} field hover_event field components");
                visitor(
                    ComponentPayload::ShowItemComponents(components),
                    &payload_what,
                )?;
            }
        }
        Some(NbtSummaryValue::String(action)) if action == "show_entity" => {
            if let Some(name) = find_entry(hover_entries, "name") {
                visit_component_payloads_inner(
                    name,
                    &format!("{what} field hover_event field name"),
                    depth + 1,
                    visitor,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
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
    validate_style(entries, what, depth)
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
    let selector = required_string(entries, "selector", what)?;
    if !is_valid_entity_selector(selector) {
        return Err(format!(
            "{what} field selector is not a valid entity selector: {selector}"
        ));
    }
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
    let nbt_path = required_string(entries, "nbt", what)?;
    if !is_valid_nbt_path(nbt_path) {
        return Err(format!(
            "{what} field nbt is not a valid NBT path: {nbt_path}"
        ));
    }
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
            "block" => validate_block_data_source(entries, what),
            "entity" => validate_entity_data_source(entries, what),
            "storage" => validate_identifier_field(entries, "storage", what).map(|_| ()),
            _ => Err(format!("{what} has unknown component data source {source}")),
        };
    }
    // DataSources registers its legacy fuzzy codecs in entity/block/storage
    // order. Keep that first-success dispatch even though more than one field
    // may be present in the same compound.
    if validate_entity_data_source(entries, what).is_ok()
        || validate_block_data_source(entries, what).is_ok()
        || validate_identifier_field(entries, "storage", what).is_ok()
    {
        return Ok(());
    }
    Err(format!("{what} has no matching component data source"))
}

pub(super) fn decoded_data_source_type<'a>(entries: &'a [NbtSummaryEntry], what: &str) -> &'a str {
    if let Some(NbtSummaryValue::String(source)) = find_entry(entries, "source") {
        return source;
    }
    if validate_entity_data_source(entries, what).is_ok() {
        "entity"
    } else if validate_block_data_source(entries, what).is_ok() {
        "block"
    } else {
        debug_assert!(validate_identifier_field(entries, "storage", what).is_ok());
        "storage"
    }
}

fn validate_entity_data_source(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let selector = required_string(entries, "entity", what)?;
    if is_valid_entity_selector(selector) {
        Ok(())
    } else {
        Err(format!(
            "{what} field entity is not a valid entity selector: {selector}"
        ))
    }
}

fn validate_block_data_source(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let coordinates = required_string(entries, "block", what)?;
    if is_valid_block_position(coordinates) {
        Ok(())
    } else {
        Err(format!(
            "{what} field block is not valid block coordinates: {coordinates}"
        ))
    }
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

pub(super) fn decoded_object_type<'a>(entries: &'a [NbtSummaryEntry], what: &str) -> &'a str {
    if let Some(NbtSummaryValue::String(object)) = find_entry(entries, "object") {
        return object;
    }
    if validate_atlas_object(entries, what).is_ok() {
        "atlas"
    } else {
        debug_assert!(validate_player_object(entries, what).is_ok());
        "player"
    }
}

fn validate_atlas_object(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    if find_entry(entries, "atlas").is_some() {
        validate_identifier_field(entries, "atlas", what)?;
    }
    validate_identifier_field(entries, "sprite", what).map(|_| ())
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

fn validate_style(entries: &[NbtSummaryEntry], what: &str, depth: usize) -> Result<(), String> {
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
    if let Some(click_event) = find_entry(entries, "click_event") {
        decode_component_click_event(click_event, &format!("{what} field click_event"))?;
    }
    if let Some(hover_event) = find_entry(entries, "hover_event") {
        validate_hover_event(hover_event, &format!("{what} field hover_event"), depth)?;
    }
    Ok(())
}

pub(super) fn decode_component_click_event(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<ComponentClickEvent, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let action = required_string(entries, "action", what)?;
    match action {
        "open_url" => {
            let url = required_string(entries, "url", what)?;
            validate_untrusted_uri(url, what)?;
            Ok(ComponentClickEvent::OpenUrl {
                url: url.to_string(),
            })
        }
        "open_file" => Err(format!(
            "{what} action open_file is not allowed from a server"
        )),
        "run_command" => {
            let command = required_string(entries, "command", what)?;
            validate_chat_string(command, what)?;
            Ok(ComponentClickEvent::RunCommand {
                command: command.to_string(),
            })
        }
        "suggest_command" => {
            let command = required_string(entries, "command", what)?;
            validate_chat_string(command, what)?;
            Ok(ComponentClickEvent::SuggestCommand {
                command: command.to_string(),
            })
        }
        "show_dialog" => {
            let dialog = required_entry(entries, "dialog", what)?;
            validate_registry_file_holder_shape(dialog, &format!("{what} field dialog"))?;
            Ok(ComponentClickEvent::ShowDialog {
                dialog: dialog.clone(),
            })
        }
        "change_page" => {
            let page = component_i32(required_entry(entries, "page", what)?)
                .ok_or_else(|| format!("{what} field page is not numeric"))?;
            if page <= 0 {
                return Err(format!("{what} field page must be positive"));
            }
            Ok(ComponentClickEvent::ChangePage { page })
        }
        "copy_to_clipboard" => Ok(ComponentClickEvent::CopyToClipboard {
            value: required_string(entries, "value", what)?.to_string(),
        }),
        "custom" => {
            let id = validate_identifier_field(entries, "id", what)?;
            Ok(ComponentClickEvent::Custom {
                id,
                payload: find_entry(entries, "payload").cloned(),
            })
        }
        _ => Err(format!("{what} has unknown click action {action}")),
    }
}

fn validate_hover_event(value: &NbtSummaryValue, what: &str, depth: usize) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    match required_string(entries, "action", what)? {
        "show_text" => validate_component(
            required_entry(entries, "value", what)?,
            &format!("{what} field value"),
            depth + 1,
        ),
        "show_item" => validate_hover_item(entries, what),
        "show_entity" => validate_hover_entity(entries, what, depth),
        action => Err(format!("{what} has unknown hover action {action}")),
    }
}

fn validate_hover_item(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let raw_id = required_string(entries, "id", what)?;
    let resource_id = normalize_resource_location(raw_id.to_string())
        .map_err(|_| format!("{what} field id is not a valid item identifier: {raw_id}"))?;
    if vanilla_item_resource_id_matches_type(&resource_id, VANILLA_ITEM_AIR_ID)
        || vanilla_item_type_for_resource_id(&resource_id).is_none()
    {
        return Err(format!(
            "{what} field id is not a non-empty built-in item: {raw_id}"
        ));
    }
    if let Some(count) = find_entry(entries, "count") {
        let count =
            component_i32(count).ok_or_else(|| format!("{what} field count is not numeric"))?;
        if !(1..=99).contains(&count) {
            return Err(format!("{what} field count {count} is outside 1..=99"));
        }
    }
    if let Some(components) = find_entry(entries, "components") {
        validate_data_component_patch(components, &format!("{what} field components"))?;
    }
    Ok(())
}

fn validate_data_component_patch(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let mut decoded_keys = Vec::<(i32, bool)>::new();
    for entry in entries {
        let (removed, raw_resource_id) = entry
            .name
            .strip_prefix('!')
            .map_or((false, entry.name.as_str()), |resource_id| {
                (true, resource_id)
            });
        let resource_id = normalize_resource_location(raw_resource_id.to_string())
            .map_err(|_| format!("{what} has invalid component identifier {raw_resource_id}"))?;
        let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
            .ok_or_else(|| format!("{what} has unknown component {resource_id}"))?;
        if !component_type.persistent {
            return Err(format!("{what} component {resource_id} is not persistent"));
        }
        if removed && !matches!(entry.value, NbtSummaryValue::Compound(_)) {
            return Err(format!(
                "{what} removed component {resource_id} payload is not a compound"
            ));
        }
        if decoded_keys.iter().any(|(type_id, existing_removed)| {
            *type_id == component_type.type_id && *existing_removed == removed
        }) {
            return Err(format!(
                "{what} has duplicate component patch key {}{}",
                if removed { "!" } else { "" },
                resource_id
            ));
        }
        if let Some(existing) = decoded_keys
            .iter_mut()
            .find(|(type_id, _)| *type_id == component_type.type_id)
        {
            *existing = (component_type.type_id, removed);
        } else {
            decoded_keys.push((component_type.type_id, removed));
        }
    }
    Ok(())
}

fn validate_hover_entity(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<(), String> {
    let raw_id = required_string(entries, "id", what)?;
    let resource_id = normalize_resource_location(raw_id.to_string())
        .map_err(|_| format!("{what} field id is not a valid entity identifier: {raw_id}"))?;
    if vanilla_entity_type_for_resource_id(&resource_id).is_none() {
        return Err(format!(
            "{what} field id is not a built-in entity type: {raw_id}"
        ));
    }
    validate_uuid(
        required_entry(entries, "uuid", what)?,
        &format!("{what} field uuid"),
    )?;
    if let Some(name) = find_entry(entries, "name") {
        validate_component(name, &format!("{what} field name"), depth + 1)?;
    }
    Ok(())
}

fn validate_uuid(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    match value {
        NbtSummaryValue::IntArray(values) if values.len() == 4 => Ok(()),
        NbtSummaryValue::ByteArray(values) if values.len() == 4 => Ok(()),
        NbtSummaryValue::LongArray(values) if values.len() == 4 => Ok(()),
        NbtSummaryValue::List(values)
            if values.len() == 4 && values.iter().all(|value| component_i32(value).is_some()) =>
        {
            Ok(())
        }
        NbtSummaryValue::String(value) if is_java_uuid_string(value) => Ok(()),
        _ => Err(format!("{what} is not a lenient UUID")),
    }
}

fn is_java_uuid_string(value: &str) -> bool {
    if value.len() > 36 {
        return false;
    }
    let mut parts = value.split('-');
    (0..5).all(|_| parts.next().is_some_and(is_java_positive_hex_long)) && parts.next().is_none()
}

fn is_java_positive_hex_long(part: &str) -> bool {
    let digits = part.strip_prefix('+').unwrap_or(part);
    !digits.is_empty()
        && digits.len() <= 16
        && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
        && (digits.len() < 16 || digits.as_bytes()[0] <= b'7')
}

fn validate_registry_file_holder_shape(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    match value {
        NbtSummaryValue::String(resource_id) => normalize_resource_location(resource_id.clone())
            .map(|_| ())
            .map_err(|_| format!("{what} is not a valid resource identifier: {resource_id}")),
        NbtSummaryValue::Compound(_) => Ok(()),
        _ => Err(format!(
            "{what} is neither a resource id nor a direct value"
        )),
    }
}

fn validate_untrusted_uri(value: &str, what: &str) -> Result<(), String> {
    if value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(format!("{what} field url is not a valid untrusted URI"));
    }
    let url = Url::parse(value)
        .map_err(|error| format!("{what} field url is not a valid URI: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "{what} field url uses unsupported scheme {}",
            url.scheme()
        ));
    }
    Ok(())
}

fn validate_chat_string(value: &str, what: &str) -> Result<(), String> {
    if let Some(character) = value
        .chars()
        .find(|character| *character == '§' || (*character as u32) < 32 || *character == '\u{7f}')
    {
        return Err(format!(
            "{what} contains disallowed chat character {character:?}"
        ));
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
) -> Result<String, String> {
    let raw = required_string(entries, field, what)?;
    normalize_resource_location(raw.to_string())
        .map_err(|_| format!("{what} field {field} is not a valid identifier: {raw}"))
}

pub(super) fn component_bool(value: &NbtSummaryValue) -> Option<bool> {
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

pub(super) fn component_i32(value: &NbtSummaryValue) -> Option<i32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i32,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as i32,
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
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests;
