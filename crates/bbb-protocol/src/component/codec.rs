use crate::{
    content_registry::ContentRegistry,
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
    command::{
        is_valid_block_position_utf16, is_valid_entity_selector_utf16_with_registry,
        is_valid_nbt_path_utf16,
    },
    parse_text_color, ComponentClickEvent,
};

pub(super) const MAX_COMPONENT_DEPTH: usize = 64;
pub const COMPONENT_DEFAULT_ATLAS: &str = "minecraft:blocks";
pub const COMPONENT_CONTENT_FIELDS: [(&str, &str); 8] = [
    ("text", "text"),
    ("translatable", "translate"),
    ("keybind", "keybind"),
    ("score", "score"),
    ("selector", "selector"),
    ("nbt", "nbt"),
    ("object", "sprite"),
    ("object", "player"),
];
pub const COMPONENT_DATA_SOURCE_FIELDS: [(&str, &str); 3] = [
    ("entity", "entity"),
    ("block", "block"),
    ("storage", "storage"),
];
pub const COMPONENT_OBJECT_FIELDS: [(&str, &str); 2] = [("atlas", "sprite"), ("player", "player")];
pub const COMPONENT_STYLE_FLAG_FIELDS: [&str; 5] = [
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
];
pub const COMPONENT_STYLE_FIELDS: [&str; 11] = [
    "color",
    "shadow_color",
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
    "click_event",
    "hover_event",
    "insertion",
    "font",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentRecordTypeError<'a> {
    NonStringDiscriminator,
    UnknownType(&'a crate::JavaString),
    MissingField,
}

pub fn resolve_component_record_type<'a>(
    explicit: Option<Result<&'a crate::JavaString, ()>>,
    has_field: impl Fn(&str) -> bool,
    types: &[(&'static str, &'static str)],
) -> Result<(&'static str, &'static str), ComponentRecordTypeError<'a>> {
    if let Some(value) = explicit {
        let kind = value.map_err(|()| ComponentRecordTypeError::NonStringDiscriminator)?;
        return types
            .iter()
            .copied()
            .find(|(candidate, _)| kind == *candidate)
            .ok_or(ComponentRecordTypeError::UnknownType(kind));
    }
    types
        .iter()
        .copied()
        .find(|(_, key)| has_field(key))
        .ok_or(ComponentRecordTypeError::MissingField)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStaticClickAction {
    OpenUrl,
    OpenFile,
    RunCommand,
    SuggestCommand,
    ShowDialog,
    ChangePage,
    CopyToClipboard,
    Custom,
}

impl ComponentStaticClickAction {
    pub fn parse(action: &(impl PartialEq<&'static str> + ?Sized)) -> Option<Self> {
        Some(if *action == "open_url" {
            Self::OpenUrl
        } else if *action == "open_file" {
            Self::OpenFile
        } else if *action == "run_command" {
            Self::RunCommand
        } else if *action == "suggest_command" {
            Self::SuggestCommand
        } else if *action == "show_dialog" {
            Self::ShowDialog
        } else if *action == "change_page" {
            Self::ChangePage
        } else if *action == "copy_to_clipboard" {
            Self::CopyToClipboard
        } else if *action == "custom" {
            Self::Custom
        } else {
            return None;
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentHoverAction {
    ShowText,
    ShowItem,
    ShowEntity,
}

impl ComponentHoverAction {
    pub fn parse(action: &(impl PartialEq<&'static str> + ?Sized)) -> Option<Self> {
        Some(if *action == "show_text" {
            Self::ShowText
        } else if *action == "show_item" {
            Self::ShowItem
        } else if *action == "show_entity" {
            Self::ShowEntity
        } else {
            return None;
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInEntityIdError {
    InvalidUtf8,
    InvalidIdentifier,
    UnknownEntityType,
}

pub fn normalize_built_in_entity_type_id(
    raw_id: &crate::JavaString,
) -> Result<String, BuiltInEntityIdError> {
    normalize_built_in_entity_type_id_with_registry(raw_id, ContentRegistry::vanilla())
}

pub fn normalize_built_in_entity_type_id_with_registry(
    raw_id: &crate::JavaString,
    registry: &ContentRegistry,
) -> Result<String, BuiltInEntityIdError> {
    let text = raw_id
        .try_to_string()
        .map_err(|_| BuiltInEntityIdError::InvalidUtf8)?;
    let id =
        normalize_resource_location(text).map_err(|_| BuiltInEntityIdError::InvalidIdentifier)?;
    let is_registered_mod_entity =
        !id.starts_with("minecraft:") && registry.entity_by_resource_id(&id).is_some();
    if vanilla_entity_type_for_resource_id(&id).is_none() && !is_registered_mod_entity {
        return Err(BuiltInEntityIdError::UnknownEntityType);
    }
    Ok(id)
}

pub fn argb_from_channels(channels: [f32; 4]) -> i32 {
    let channel = |index: usize| ((channels[index] * 255.0).floor() as i32 as u32) & 0xff;
    ((channel(3) << 24) | (channel(0) << 16) | (channel(1) << 8) | channel(2)) as i32
}

pub fn canonical_text_color_string(raw: &crate::JavaString) -> Option<String> {
    let (rgb, name) = raw.with_utf8(parse_text_color).flatten()?;
    Some(name.map_or_else(|| format!("#{rgb:06X}"), |name| name.as_str().to_string()))
}

/// Validates the registry-independent structural portion of vanilla 26.1
/// `ComponentSerialization.CODEC` over a decoded network-NBT value.
///
/// This owns the recursive root alternatives, the inferred/typed content
/// matcher, simple content fields, nested component positions, `extra`, and
/// style fields and the registry-independent portion of click/hover events.
/// Command parsers, deep dialog/item payloads, registry-aware values, and
/// canonical JsonOps encoding are intentionally separate stages.
pub fn validate_component_nbt(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    validate_component_nbt_with_registry(value, what, ContentRegistry::vanilla())
}

pub fn validate_component_nbt_with_registry(
    value: &NbtSummaryValue,
    what: &str,
    registry: &ContentRegistry,
) -> Result<(), String> {
    validate_component(value, what, 0, registry)
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

/// Validates a Component while visiting payloads whose value codecs require a
/// higher-level owner (Dialog or persistent data components).
pub fn visit_component_payloads<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
    visitor: impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    visit_component_payloads_with_registry(value, what, ContentRegistry::vanilla(), visitor)
}

/// Validates a Component against a session `ContentRegistry` while visiting
/// payloads whose value codecs require a higher-level owner.
pub fn visit_component_payloads_with_registry<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
    registry: &ContentRegistry,
    mut visitor: impl FnMut(ComponentPayload<'a>, &str) -> Result<(), String>,
) -> Result<(), String> {
    validate_component_with_payloads::<true>(value, what, 0, registry, &mut Some(&mut visitor))
}

pub(super) fn validate_component_style_nbt_with_registry(
    value: &NbtSummaryValue,
    what: &str,
    registry: &ContentRegistry,
) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    validate_style::<false>(entries, what, 0, registry, &mut None)
}

pub(super) fn validate_component(
    value: &NbtSummaryValue,
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
) -> Result<(), String> {
    validate_component_with_payloads::<false>(value, what, depth, registry, &mut None)
}

type PayloadVisitor<'value, 'visitor> =
    Option<&'visitor mut dyn FnMut(ComponentPayload<'value>, &str) -> Result<(), String>>;

fn validate_component_with_payloads<'a, const VISIT: bool>(
    value: &'a NbtSummaryValue,
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
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
                validate_component_with_payloads::<VISIT>(
                    value,
                    &format!("{what}[{index}]"),
                    depth + 1,
                    registry,
                    visitor,
                )?;
            }
            Ok(())
        }
        NbtSummaryValue::Compound(entries) => {
            validate_full_component::<VISIT>(entries, what, depth, registry, visitor)
        }
        _ => Err(format!("{what} is not a component")),
    }
}

pub fn component_record_type(
    entries: &[NbtSummaryEntry],
    discriminator: &str,
    types: &[(&'static str, &'static str)],
    what: &str,
) -> Result<&'static str, String> {
    let explicit = find_entry(entries, discriminator).map(|value| match value {
        NbtSummaryValue::String(kind) => Ok(kind),
        _ => Err(()),
    });
    resolve_component_record_type(explicit, |key| find_entry(entries, key).is_some(), types)
        .map(|(kind, _)| kind)
        .map_err(|error| match error {
            ComponentRecordTypeError::NonStringDiscriminator => {
                format!("{what} field {discriminator} is not a string")
            }
            ComponentRecordTypeError::UnknownType(kind) => {
                format!("{what} has unknown component {discriminator} {kind}")
            }
            ComponentRecordTypeError::MissingField => {
                format!("{what} has no matching component {discriminator} field")
            }
        })
}

fn validate_full_component<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let kind = component_record_type(entries, "type", &COMPONENT_CONTENT_FIELDS, what)?;
    validate_content_type::<VISIT>(kind, entries, what, depth, registry, visitor)?;
    if let Some(extra) = find_entry(entries, "extra") {
        let NbtSummaryValue::List(values) = extra else {
            return Err(format!("{what} field extra is not a list"));
        };
        if values.is_empty() {
            return Err(format!("{what} field extra is empty"));
        }
        for (index, value) in values.iter().enumerate() {
            validate_component_with_payloads::<VISIT>(
                value,
                &format!("{what} field extra[{index}]"),
                depth + 1,
                registry,
                visitor,
            )?;
        }
    }
    validate_style::<VISIT>(entries, what, depth, registry, visitor)
}

fn validate_content_type<'a, const VISIT: bool>(
    content_type: &str,
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    match content_type {
        "text" | "keybind" => {
            let value = required_entry(entries, content_type, what)?;
            if !matches!(value, NbtSummaryValue::String(_)) {
                return Err(format!("{what} field {content_type} is not a string"));
            }
            Ok(())
        }
        "translatable" => validate_translatable::<VISIT>(entries, what, depth, registry, visitor),
        "score" => validate_score(entries, what),
        "selector" => validate_selector::<VISIT>(entries, what, depth, registry, visitor),
        "nbt" => validate_nbt_contents::<VISIT>(entries, what, depth, registry, visitor),
        "object" => validate_object_contents::<VISIT>(entries, what, depth, registry, visitor),
        _ => unreachable!("content type is checked before dispatch"),
    }
}

fn validate_translatable<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let key = required_entry(entries, "translate", what)?;
    if !matches!(key, NbtSummaryValue::String(_)) {
        return Err(format!("{what} field translate is not a string"));
    }
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
            validate_component_with_payloads::<VISIT>(
                argument,
                &format!("{what} field with[{index}]"),
                depth + 1,
                registry,
                visitor,
            )?;
        }
    }
    Ok(())
}

fn validate_score(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let score = required_entry(entries, "score", what)?;
    let NbtSummaryValue::Compound(score_entries) = score else {
        return Err(format!("{what} field score is not a compound"));
    };
    let score_what = format!("{what} field score");
    for field in ["name", "objective"] {
        if !matches!(
            required_entry(score_entries, field, &score_what)?,
            NbtSummaryValue::String(_)
        ) {
            return Err(format!("{score_what} field {field} is not a string"));
        }
    }
    Ok(())
}

fn validate_selector<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let selector = required_string(entries, "selector", what)?;
    if !is_valid_entity_selector_utf16_with_registry(selector.units(), registry) {
        return Err(format!(
            "{what} field selector is not a valid entity selector: {selector}"
        ));
    }
    if let Some(separator) = find_entry(entries, "separator") {
        validate_component_with_payloads::<VISIT>(
            separator,
            &format!("{what} field separator"),
            depth + 1,
            registry,
            visitor,
        )?;
    }
    Ok(())
}

fn validate_nbt_contents<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let nbt_path = required_string(entries, "nbt", what)?;
    if !is_valid_nbt_path_utf16(nbt_path.units()) {
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
    if let Some(separator) = find_entry(entries, "separator") {
        validate_component_with_payloads::<VISIT>(
            separator,
            &format!("{what} field separator"),
            depth + 1,
            registry,
            visitor,
        )?;
    }
    match component_record_type(entries, "source", &COMPONENT_DATA_SOURCE_FIELDS, what)? {
        "entity" => validate_entity_data_source(entries, what, registry),
        "block" => validate_block_data_source(entries, what),
        "storage" => validate_identifier_field(entries, "storage", what).map(|_| ()),
        _ => unreachable!("component data source table"),
    }
}

fn validate_entity_data_source(
    entries: &[NbtSummaryEntry],
    what: &str,
    registry: &ContentRegistry,
) -> Result<(), String> {
    let selector = required_string(entries, "entity", what)?;
    if is_valid_entity_selector_utf16_with_registry(selector.units(), registry) {
        Ok(())
    } else {
        Err(format!(
            "{what} field entity is not a valid entity selector: {selector}"
        ))
    }
}

fn validate_block_data_source(entries: &[NbtSummaryEntry], what: &str) -> Result<(), String> {
    let coordinates = required_string(entries, "block", what)?;
    if is_valid_block_position_utf16(coordinates.units()) {
        Ok(())
    } else {
        Err(format!(
            "{what} field block is not valid block coordinates: {coordinates}"
        ))
    }
}

fn validate_object_contents<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    match component_record_type(entries, "object", &COMPONENT_OBJECT_FIELDS, what)? {
        "atlas" => validate_atlas_object(entries, what)?,
        "player" => {
            validate_player_object(entries, what)?;
            if VISIT {
                if let Some(visitor) = visitor {
                    visitor(
                        ComponentPayload::PlayerProfile(required_entry(entries, "player", what)?),
                        &format!("{what} field player"),
                    )?;
                }
            }
        }
        _ => unreachable!("component object table"),
    }
    if let Some(fallback) = find_entry(entries, "fallback") {
        validate_component_with_payloads::<VISIT>(
            fallback,
            &format!("{what} field fallback"),
            depth + 1,
            registry,
            visitor,
        )?;
    }
    Ok(())
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

fn validate_style<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    if let Some(color) = find_entry(entries, "color") {
        let NbtSummaryValue::String(color) = color else {
            return Err(format!("{what} field color is not a string"));
        };
        if canonical_text_color_string(color).is_none() {
            return Err(format!("{what} field color is invalid: {color}"));
        }
    }
    if let Some(shadow) = find_entry(entries, "shadow_color") {
        validate_argb_color(shadow, &format!("{what} field shadow_color"))?;
    }
    for field in COMPONENT_STYLE_FLAG_FIELDS {
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
        let click_what = format!("{what} field click_event");
        let is_show_dialog = matches!(
            decode_component_click_event(click_event, &click_what)?,
            ComponentClickEvent::ShowDialog { .. }
        );
        if VISIT {
            if let Some(visitor) = visitor {
                if is_show_dialog {
                    let NbtSummaryValue::Compound(click_entries) = click_event else {
                        unreachable!("click event was decoded as a compound");
                    };
                    visitor(
                        ComponentPayload::ShowDialog(required_entry(
                            click_entries,
                            "dialog",
                            &click_what,
                        )?),
                        &format!("{click_what} field dialog"),
                    )?;
                }
            }
        }
    }
    if let Some(hover_event) = find_entry(entries, "hover_event") {
        validate_hover_event::<VISIT>(
            hover_event,
            &format!("{what} field hover_event"),
            depth,
            registry,
            visitor,
        )?;
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
    match ComponentStaticClickAction::parse(action) {
        Some(ComponentStaticClickAction::OpenUrl) => {
            let url = crate::JavaString::from_nbt(required_entry(entries, "url", what)?)
                .ok_or_else(|| format!("{what} field url is not a string"))?;
            validate_untrusted_uri_units(&url, what)?;
            Ok(ComponentClickEvent::OpenUrl { url })
        }
        Some(ComponentStaticClickAction::OpenFile) => Err(format!(
            "{what} action open_file is not allowed from a server"
        )),
        Some(ComponentStaticClickAction::RunCommand) => {
            let command = crate::JavaString::from_nbt(required_entry(entries, "command", what)?)
                .ok_or_else(|| format!("{what} field command is not a string"))?;
            validate_chat_string_units(&command, what)?;
            Ok(ComponentClickEvent::RunCommand { command })
        }
        Some(ComponentStaticClickAction::SuggestCommand) => {
            let command = crate::JavaString::from_nbt(required_entry(entries, "command", what)?)
                .ok_or_else(|| format!("{what} field command is not a string"))?;
            validate_chat_string_units(&command, what)?;
            Ok(ComponentClickEvent::SuggestCommand { command })
        }
        Some(ComponentStaticClickAction::ShowDialog) => {
            let dialog = required_entry(entries, "dialog", what)?;
            validate_registry_file_holder_shape(dialog, &format!("{what} field dialog"))?;
            Ok(ComponentClickEvent::ShowDialog {
                dialog: dialog.clone(),
            })
        }
        Some(ComponentStaticClickAction::ChangePage) => {
            let page = component_i32(required_entry(entries, "page", what)?)
                .ok_or_else(|| format!("{what} field page is not numeric"))?;
            if page <= 0 {
                return Err(format!("{what} field page must be positive"));
            }
            Ok(ComponentClickEvent::ChangePage { page })
        }
        Some(ComponentStaticClickAction::CopyToClipboard) => {
            Ok(ComponentClickEvent::CopyToClipboard {
                value: crate::JavaString::from_nbt(required_entry(entries, "value", what)?)
                    .ok_or_else(|| format!("{what} field value is not a string"))?,
            })
        }
        Some(ComponentStaticClickAction::Custom) => {
            let id = validate_identifier_field(entries, "id", what)?;
            Ok(ComponentClickEvent::Custom {
                id,
                payload: find_entry(entries, "payload").cloned(),
            })
        }
        None => Err(format!("{what} has unknown click action {action}")),
    }
}

fn validate_hover_event<'a, const VISIT: bool>(
    value: &'a NbtSummaryValue,
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let action = required_string(entries, "action", what)?;
    match ComponentHoverAction::parse(action) {
        Some(ComponentHoverAction::ShowText) => validate_component_with_payloads::<VISIT>(
            required_entry(entries, "value", what)?,
            &format!("{what} field value"),
            depth + 1,
            registry,
            visitor,
        ),
        Some(ComponentHoverAction::ShowItem) => {
            validate_hover_item::<VISIT>(entries, what, registry, visitor)
        }
        Some(ComponentHoverAction::ShowEntity) => {
            validate_hover_entity::<VISIT>(entries, what, depth, registry, visitor)
        }
        None => Err(format!("{what} has unknown hover action {action}")),
    }
}

fn validate_hover_item<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let raw_id = required_string(entries, "id", what)?;
    let resource_id = normalize_resource_location(
        raw_id
            .try_to_string()
            .map_err(|_| format!("{what} is not a valid identifier: {raw_id}"))?,
    )
    .map_err(|_| format!("{what} field id is not a valid item identifier: {raw_id}"))?;
    let is_registered_mod_item = !resource_id.starts_with("minecraft:")
        && registry.item_by_resource_id(&resource_id).is_some();
    if vanilla_item_resource_id_matches_type(&resource_id, VANILLA_ITEM_AIR_ID)
        || (vanilla_item_type_for_resource_id(&resource_id).is_none() && !is_registered_mod_item)
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
        let payload_what = format!("{what} field components");
        validate_data_component_patch(components, &payload_what)?;
        if VISIT {
            if let Some(visitor) = visitor {
                visitor(
                    ComponentPayload::ShowItemComponents(components),
                    &payload_what,
                )?;
            }
        }
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
            .map_or((false, entry.name.clone()), |resource_id| {
                (true, resource_id)
            });
        // A component patch key is a resource location; that grammar owns the bound.
        let raw_resource_id = raw_resource_id.to_jdk_string();
        let resource_id = normalize_resource_location(raw_resource_id.clone())
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

fn validate_hover_entity<'a, const VISIT: bool>(
    entries: &'a [NbtSummaryEntry],
    what: &str,
    depth: usize,
    registry: &ContentRegistry,
    visitor: &mut PayloadVisitor<'a, '_>,
) -> Result<(), String> {
    let raw_id = required_string(entries, "id", what)?;
    normalize_built_in_entity_type_id_with_registry(raw_id, registry).map_err(
        |error| match error {
            BuiltInEntityIdError::InvalidUtf8 => {
                format!("{what} is not a valid identifier: {raw_id}")
            }
            BuiltInEntityIdError::InvalidIdentifier => {
                format!("{what} field id is not a valid entity identifier: {raw_id}")
            }
            BuiltInEntityIdError::UnknownEntityType => {
                format!("{what} field id is not a built-in entity type: {raw_id}")
            }
        },
    )?;
    let uuid_what = format!("{what} field uuid");
    if super::component_uuid_words(required_entry(entries, "uuid", what)?).is_none() {
        return Err(format!("{uuid_what} is not a lenient UUID"));
    }
    if let Some(name) = find_entry(entries, "name") {
        validate_component_with_payloads::<VISIT>(
            name,
            &format!("{what} field name"),
            depth + 1,
            registry,
            visitor,
        )?;
    }
    Ok(())
}

fn validate_registry_file_holder_shape(value: &NbtSummaryValue, what: &str) -> Result<(), String> {
    match value {
        NbtSummaryValue::String(resource_id) => resource_id
            .try_to_string()
            .ok()
            .and_then(|value| normalize_resource_location(value).ok())
            .ok_or_else(|| format!("{what} is not a valid resource identifier: {resource_id}"))
            .map(|_| ()),
        NbtSummaryValue::Compound(_) => Ok(()),
        _ => Err(format!(
            "{what} is neither a resource id nor a direct value"
        )),
    }
}

/// Equality fields of the `URI` created by `ExtraCodecs.UNTRUSTED_URI`.
/// Raw fields normalize escape hex case; scheme and server host use ASCII case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UntrustedUriIdentity {
    pub scheme: String,
    pub fragment: Option<crate::JavaString>,
    pub kind: UntrustedUriKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UntrustedUriKind {
    Opaque {
        scheme_specific: crate::JavaString,
    },
    Hierarchical {
        authority: UntrustedUriAuthority,
        path: crate::JavaString,
        query: Option<crate::JavaString>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UntrustedUriAuthority {
    Absent,
    Registry(crate::JavaString),
    Server {
        user_info: Option<crate::JavaString>,
        host: crate::JavaString,
        port: i32,
    },
}

/// `net.minecraft.network.chat.ClickEvent`'s untrusted-URI check.
pub fn validate_url(value: &str) -> Result<(), &'static str> {
    parse_untrusted_uri(&value.encode_utf16().collect::<Vec<_>>()).map(|_| ())
}

fn parse_untrusted_uri(value: &[u16]) -> Result<UntrustedUriIdentity, &'static str> {
    let Some(colon) = value.iter().position(|&unit| unit == b':' as u16) else {
        return Err("URI is missing a protocol");
    };
    let scheme = String::from_utf16_lossy(&value[..colon]);
    let mut scheme_chars = scheme.chars();
    if !scheme_chars
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic())
        || !scheme_chars
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
    {
        return Err("invalid URI protocol");
    }
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https") {
        return Err("URI protocol is not permitted");
    }
    let tail = &value[colon + 1..];
    let (scheme_specific, fragment) = split_uri_units(tail, b'#');
    if let Some(fragment) = fragment {
        if fragment.contains(&(b'#' as u16)) {
            return Err("URI contains more than one fragment delimiter");
        }
    }
    if scheme_specific.is_empty() {
        return Err("URI has an empty scheme-specific part");
    }
    let kind = if scheme_specific.starts_with(&[b'/' as u16]) {
        let (authority, remainder) = if let Some(after_slashes) =
            scheme_specific.strip_prefix(&[b'/' as u16, b'/' as u16])
        {
            let end = after_slashes
                .iter()
                .position(|&unit| unit == b'/' as u16 || unit == b'?' as u16)
                .unwrap_or(after_slashes.len());
            let authority = &after_slashes[..end];
            let remainder = &after_slashes[end..];
            if authority.is_empty() && remainder.is_empty() && fragment.is_none() {
                return Err("URI is missing an authority");
            }
            (parse_uri_authority(authority)?, remainder)
        } else {
            (UntrustedUriAuthority::Absent, scheme_specific)
        };
        let (path, query) = split_uri_units(remainder, b'?');
        validate_uri_units(path, is_uri_path_character)?;
        if let Some(query) = query {
            validate_uri_units(query, is_uri_character)?;
        }
        UntrustedUriKind::Hierarchical {
            authority,
            path: normalized_uri_component(path),
            query: query.map(normalized_uri_component),
        }
    } else {
        validate_uri_units(scheme_specific, is_uri_character)?;
        UntrustedUriKind::Opaque {
            scheme_specific: normalized_uri_component(scheme_specific),
        }
    };
    if let Some(fragment) = fragment {
        validate_uri_units(fragment, is_uri_character)?;
    }
    Ok(UntrustedUriIdentity {
        scheme,
        fragment: fragment.map(normalized_uri_component),
        kind,
    })
}

fn split_uri_units(value: &[u16], delimiter: u8) -> (&[u16], Option<&[u16]>) {
    value
        .iter()
        .position(|&unit| unit == delimiter as u16)
        .map_or((value, None), |index| {
            (&value[..index], Some(&value[index + 1..]))
        })
}

fn normalized_uri_component(value: &[u16]) -> crate::JavaString {
    let mut value = value.to_vec();
    let mut index = 0;
    while index < value.len() {
        if value[index] == b'%' as u16 {
            for unit in &mut value[index + 1..index + 3] {
                *unit = ascii_lower_unit(*unit);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    value.into()
}

fn ascii_lower_unit(unit: u16) -> u16 {
    if (b'A' as u16..=b'Z' as u16).contains(&unit) {
        unit + 32
    } else {
        unit
    }
}

fn validate_uri_units(value: &[u16], allowed_ascii: fn(char) -> bool) -> Result<(), &'static str> {
    // URI scanEscape admits surrogate units; projection is only used to check classes.
    validate_uri_chars(&String::from_utf16_lossy(value), allowed_ascii)
}

fn parse_uri_authority(value: &[u16]) -> Result<UntrustedUriAuthority, &'static str> {
    if value.is_empty() {
        return Ok(UntrustedUriAuthority::Absent);
    }
    let bracketed = value
        .iter()
        .any(|&unit| unit == b'[' as u16 || unit == b']' as u16);
    if !bracketed {
        validate_uri_units(value, is_uri_registry_name_character)?;
    }
    let (user_info, host_and_port) = match value.iter().position(|&unit| unit == b'@' as u16) {
        Some(index) => (Some(&value[..index]), &value[index + 1..]),
        None => (None, value),
    };
    let server = (|| {
        if let Some(user_info) = user_info {
            validate_uri_units(user_info, is_uri_user_info_character).ok()?;
        }
        let (host, port) = if host_and_port.starts_with(&[b'[' as u16]) {
            let close = host_and_port.iter().position(|&unit| unit == b']' as u16)?;
            let literal = String::from_utf16(&host_and_port[1..close]).ok()?;
            let (address, scope) = literal
                .split_once('%')
                .map_or((literal.as_str(), None), |(address, scope)| {
                    (address, Some(scope))
                });
            if !valid_uri_ipv6(address)
                || scope.is_some_and(|scope| {
                    scope.is_empty()
                        || !scope
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
                })
            {
                return None;
            }
            let port = &host_and_port[close + 1..];
            (&host_and_port[..close + 1], parse_uri_port(port)?)
        } else {
            let (host, port) = split_uri_units(host_and_port, b':');
            let spelling = String::from_utf16(host).ok()?;
            if !valid_uri_ipv4(&spelling) && !valid_uri_hostname(&spelling) {
                return None;
            }
            let port = match port {
                None => -1,
                Some(port) => parse_uri_port_digits(port)?,
            };
            (host, port)
        };
        Some(UntrustedUriAuthority::Server {
            user_info: user_info.map(normalized_uri_component),
            host: host
                .iter()
                .copied()
                .map(ascii_lower_unit)
                .collect::<Vec<_>>()
                .into(),
            port,
        })
    })();
    if let Some(server) = server {
        Ok(server)
    } else if bracketed {
        Err("URI contains a malformed IPv6 authority")
    } else {
        // URI(String) uses parse(false): failed server syntax remains a legal registry authority.
        Ok(UntrustedUriAuthority::Registry(normalized_uri_component(
            value,
        )))
    }
}

fn parse_uri_port(value: &[u16]) -> Option<i32> {
    if value.is_empty() {
        Some(-1)
    } else {
        parse_uri_port_digits(value.strip_prefix(&[b':' as u16])?)
    }
}

fn parse_uri_port_digits(value: &[u16]) -> Option<i32> {
    if value.is_empty() {
        return Some(-1);
    }
    value.iter().try_fold(0_i32, |port, &unit| {
        if !(b'0' as u16..=b'9' as u16).contains(&unit) {
            return None;
        }
        port.checked_mul(10)?
            .checked_add(i32::from(unit - b'0' as u16))
    })
}

fn valid_uri_hostname(value: &str) -> bool {
    let value = value.strip_suffix('.').unwrap_or(value);
    let mut labels = value.split('.').peekable();
    let mut index = 0;
    while let Some(label) = labels.next() {
        if label.is_empty()
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || !label.as_bytes()[0].is_ascii_alphanumeric()
            || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            || (index > 0 && labels.peek().is_none() && !label.as_bytes()[0].is_ascii_alphabetic())
        {
            return false;
        }
        index += 1;
    }
    index > 0
}

fn valid_uri_ipv4(value: &str) -> bool {
    let mut count = 0;
    for part in value.split('.') {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        let significant = part.trim_start_matches('0');
        if significant.len() > 3
            || (!significant.is_empty()
                && significant.parse::<u16>().map_or(true, |part| part > 255))
        {
            return false;
        }
        count += 1;
    }
    count == 4
}

fn valid_uri_ipv6(value: &str) -> bool {
    // JDK URI.parseIPv6Reference counts words without canonicalizing the host spelling.
    // Its embedded IPv4 scanByte accepts arbitrarily many leading zeroes.
    let (left, right) = value
        .split_once("::")
        .map_or((value, None), |(left, right)| (left, Some(right)));
    if right.is_some_and(|right| right.contains("::")) {
        return false;
    }
    let mut words = 0;
    let sides: Vec<&str> = if let Some(right) = right {
        vec![left, right]
    } else {
        vec![left]
    };
    for (side_index, side) in sides.iter().enumerate() {
        if side.is_empty() {
            if right.is_none() {
                return false;
            }
            continue;
        }
        let mut parts = side.split(':').peekable();
        while let Some(part) = parts.next() {
            if part.contains('.') {
                if parts.peek().is_some() || side_index + 1 != sides.len() || !valid_uri_ipv4(part)
                {
                    return false;
                }
                words += 2;
            } else {
                if part.is_empty()
                    || part.len() > 4
                    || !part.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return false;
                }
                words += 1;
            }
        }
    }
    if right.is_some() {
        words < 8
    } else {
        words == 8
    }
}

fn validate_uri_chars(value: &str, allowed_ascii: fn(char) -> bool) -> Result<(), &'static str> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err("URI contains an invalid percent escape");
            }
            index += 3;
            continue;
        }
        let character = value[index..]
            .chars()
            .next()
            .expect("index remains on a UTF-8 boundary");
        if character.is_ascii() {
            if !allowed_ascii(character) {
                return Err("URI contains a character that must be escaped");
            }
        } else if character.is_whitespace() || character.is_control() {
            return Err("URI contains whitespace or a control character");
        }
        index += character.len_utf8();
    }
    Ok(())
}

fn is_uri_unreserved(value: char) -> bool {
    value.is_ascii_alphanumeric()
        || matches!(value, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
}

fn is_uri_character(value: char) -> bool {
    is_uri_unreserved(value)
        || matches!(
            value,
            ';' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | ',' | '[' | ']'
        )
}

fn is_uri_path_character(value: char) -> bool {
    is_uri_unreserved(value) || matches!(value, ':' | '@' | '&' | '=' | '+' | '$' | ',' | ';' | '/')
}

fn is_uri_registry_name_character(value: char) -> bool {
    is_uri_unreserved(value) || matches!(value, '$' | ',' | ';' | ':' | '@' | '&' | '=' | '+')
}

fn is_uri_user_info_character(value: char) -> bool {
    is_uri_unreserved(value) || matches!(value, ';' | ':' | '&' | '=' | '+' | '$' | ',')
}

/// Parses once and preserves every raw UTF-16 unit in URI equality fields.
pub fn parse_untrusted_uri_units(
    value: &crate::JavaString,
    what: &str,
) -> Result<UntrustedUriIdentity, String> {
    parse_untrusted_uri(value.units())
        .map_err(|reason| format!("{what} field url is not a valid untrusted URI: {reason}"))
}

pub fn validate_untrusted_uri_units(value: &crate::JavaString, what: &str) -> Result<(), String> {
    parse_untrusted_uri_units(value, what).map(|_| ())
}

pub fn validate_chat_string_units(value: &crate::JavaString, what: &str) -> Result<(), String> {
    if let Some(&unit) = value
        .units()
        .iter()
        .find(|&&unit| unit == 167 || unit < 32 || unit == 127)
    {
        return Err(format!(
            "{what} contains disallowed chat character {:?}",
            char::from(unit as u8)
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
    normalize_resource_location(
        raw.try_to_string()
            .map_err(|_| format!("{what} is not a valid identifier: {raw}"))?,
    )
    .map_err(|_| format!("{what} field {field} is not a valid identifier: {raw}"))
}

pub fn component_bool(value: &NbtSummaryValue) -> Option<bool> {
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
    crate::nbt_summary::codec_int_value(value)
}

pub(super) fn is_java_primitive(value: &NbtSummaryValue) -> bool {
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
) -> Result<&'a crate::JavaString, String> {
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
