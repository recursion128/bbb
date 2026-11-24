use std::borrow::Cow;

use super::{
    codec::{
        argb_from_channels, canonical_text_color_string, component_bool, component_i32,
        component_record_type, is_java_primitive, ComponentHoverAction, ComponentStaticClickAction,
        COMPONENT_CONTENT_FIELDS, COMPONENT_DATA_SOURCE_FIELDS, COMPONENT_DEFAULT_ATLAS,
        COMPONENT_OBJECT_FIELDS, COMPONENT_STYLE_FLAG_FIELDS,
    },
    validate_component_nbt_with_registry,
};
use crate::{
    content_registry::ContentRegistry,
    nbt_summary::{NbtSummaryEntry, NbtSummaryValue},
    packets::normalize_resource_location,
};

mod java_number;

use java_number::{java_double_string, java_float_string};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalComponentOwnerValue {
    PlayerProfile,
    Dialog,
    ItemComponentPatch,
}

/// Canonical value emitted by vanilla 26.1 `ComponentSerialization.CODEC`
/// before a concrete `DynamicOps` chooses its wire representation.
///
/// Raw NBT leaves retain primitive tags for `HashOps`; owner nodes borrow
/// values whose codecs belong to a surrounding registry-aware subsystem.
#[derive(Debug, Clone, Eq)]
pub enum CanonicalComponentCodecValue<'a> {
    Boolean(bool),
    Int(i32),
    String(Cow<'a, crate::JavaString>),
    List(Vec<Self>),
    IntArray(Vec<i32>),
    Object(Vec<(&'static str, Self)>),
    Nbt(&'a NbtSummaryValue),
    Owner(CanonicalComponentOwnerValue, &'a NbtSummaryValue),
}

impl PartialEq for CanonicalComponentCodecValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        use CanonicalComponentCodecValue as Value;
        match (self, other) {
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::IntArray(a), Value::IntArray(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Nbt(a), Value::Nbt(b)) => crate::nbt_summary::codec_values_equal(a, b),
            (Value::Owner(a_kind, a_value), Value::Owner(b_kind, b_value)) => {
                a_kind == b_kind && a_value == b_value
            }
            _ => false,
        }
    }
}

/// Decodes and canonically re-encodes one Component into a typed,
/// `DynamicOps`-independent value tree.
pub fn canonical_component_codec_value(
    value: &NbtSummaryValue,
) -> Result<CanonicalComponentCodecValue<'_>, String> {
    canonical_component_codec_value_with_registry(value, ContentRegistry::vanilla())
}

/// Decodes and canonically re-encodes one Component into a typed,
/// `DynamicOps`-independent value tree using `registry` for custom item/entity IDs.
pub fn canonical_component_codec_value_with_registry<'a>(
    value: &'a NbtSummaryValue,
    registry: &ContentRegistry,
) -> Result<CanonicalComponentCodecValue<'a>, String> {
    validate_component_nbt_with_registry(value, "component", registry)?;
    canonical_component_at(value, 0)
        .ok_or_else(|| "validated component could not be canonically encoded".to_string())
}

/// Formats a Java `float` exactly like `Float.toString`.
///
/// This is also the textual value used by vanilla dialog number-range template substitutions.
pub fn java_float_to_string(value: f32) -> String {
    java_float_string(value)
}

/// Formats a Java `double` exactly like `Double.toString`.
pub fn java_double_to_string(value: f64) -> String {
    java_double_string(value)
}

type ComponentRef<'a> = (&'a NbtSummaryValue, usize);

fn canonical_component_at(
    mut value: &NbtSummaryValue,
    mut depth: usize,
) -> Option<CanonicalComponentCodecValue<'_>> {
    #[cfg(test)]
    crate::test_stack_budget::record();

    let mut siblings = Vec::new();
    while let NbtSummaryValue::List(values) = value {
        let (first, rest) = values.split_first()?;
        depth += 1;
        siblings.extend(rest.iter().rev().map(|value| (value, depth)));
        value = first;
    }
    siblings.reverse();
    match value {
        NbtSummaryValue::String(text) => {
            if siblings.is_empty() {
                Some(CanonicalComponentCodecValue::String(Cow::Borrowed(text)))
            } else {
                Some(CanonicalComponentCodecValue::Object(vec![
                    (
                        "text",
                        CanonicalComponentCodecValue::String(Cow::Borrowed(text)),
                    ),
                    ("extra", canonical_component_list(siblings.into_iter())?),
                ]))
            }
        }
        NbtSummaryValue::Compound(entries) => canonical_full_component(entries, depth, siblings),
        _ => None,
    }
}

fn canonical_full_component<'a>(
    entries: &'a [NbtSummaryEntry],
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
) -> Option<CanonicalComponentCodecValue<'a>> {
    let content_type =
        component_record_type(entries, "type", &COMPONENT_CONTENT_FIELDS, "component")
            .expect("component content type was validated before canonical encoding");
    let extra = match find_entry(entries, "extra") {
        Some(NbtSummaryValue::List(values)) => values.as_slice(),
        _ => &[],
    };
    let has_siblings = !extra.is_empty() || !appended_siblings.is_empty();
    let style = canonical_style_fields(entries, depth)?;

    if content_type == "text" && !has_siblings && style.is_empty() {
        return string_field(entries, "text");
    }
    let mut fields = canonical_content_fields(content_type, entries, depth)?;
    if has_siblings {
        let siblings = extra
            .iter()
            .map(|value| (value, depth + 1))
            .chain(appended_siblings);
        fields.push(("extra", canonical_component_list(siblings)?));
    }
    fields.extend(style);
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_component_list<'a>(
    values: impl Iterator<Item = ComponentRef<'a>>,
) -> Option<CanonicalComponentCodecValue<'a>> {
    values
        .map(|(value, depth)| canonical_component_at(value, depth))
        .collect::<Option<Vec<_>>>()
        .map(CanonicalComponentCodecValue::List)
}

fn canonical_content_fields<'a>(
    content_type: &str,
    entries: &'a [NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, CanonicalComponentCodecValue<'a>)>> {
    let mut fields = Vec::new();
    match content_type {
        "text" => fields.push(("text", string_field(entries, "text")?)),
        "translatable" => {
            fields.push(("translate", string_field(entries, "translate")?));
            if let Some(NbtSummaryValue::String(fallback)) = find_entry(entries, "fallback") {
                fields.push((
                    "fallback",
                    CanonicalComponentCodecValue::String(Cow::Borrowed(fallback)),
                ));
            }
            if let Some(NbtSummaryValue::List(arguments)) = find_entry(entries, "with") {
                if !arguments.is_empty() {
                    let arguments = arguments
                        .iter()
                        .map(|argument| canonical_translation_argument(argument, depth + 1))
                        .collect::<Option<Vec<_>>>()?;
                    fields.push(("with", CanonicalComponentCodecValue::List(arguments)));
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
                CanonicalComponentCodecValue::Object(vec![
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
                fields.push(("interpret", CanonicalComponentCodecValue::Boolean(true)));
            }
            if find_entry(entries, "plain")
                .and_then(component_bool)
                .unwrap_or(false)
            {
                fields.push(("plain", CanonicalComponentCodecValue::Boolean(true)));
            }
            if let Some(separator) = find_entry(entries, "separator") {
                fields.push(("separator", canonical_component_at(separator, depth + 1)?));
            }
            match component_record_type(
                entries,
                "source",
                &COMPONENT_DATA_SOURCE_FIELDS,
                "component",
            )
            .expect("component data source was validated before canonical encoding")
            {
                "entity" => fields.push(("entity", string_field(entries, "entity")?)),
                "block" => fields.push(("block", string_field(entries, "block")?)),
                "storage" => {
                    fields.push(("storage", normalized_identifier_field(entries, "storage")?))
                }
                _ => unreachable!("component data source was validated before canonical encoding"),
            }
        }
        "object" => {
            match component_record_type(entries, "object", &COMPONENT_OBJECT_FIELDS, "component")
                .expect("component object type was validated before canonical encoding")
            {
                "atlas" => {
                    if find_entry(entries, "atlas").is_some() {
                        let atlas = normalized_identifier_field(entries, "atlas")?;
                        if !matches!(
                            &atlas,
                            CanonicalComponentCodecValue::String(value)
                                if value.as_ref() == COMPONENT_DEFAULT_ATLAS
                        ) {
                            fields.push(("atlas", atlas));
                        }
                    }
                    fields.push(("sprite", normalized_identifier_field(entries, "sprite")?));
                }
                "player" => {
                    let player = find_entry(entries, "player")?;
                    fields.push((
                        "player",
                        CanonicalComponentCodecValue::Owner(
                            CanonicalComponentOwnerValue::PlayerProfile,
                            player,
                        ),
                    ));
                    if !find_entry(entries, "hat")
                        .and_then(component_bool)
                        .unwrap_or(true)
                    {
                        fields.push(("hat", CanonicalComponentCodecValue::Boolean(false)));
                    }
                }
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

fn canonical_translation_argument(
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<CanonicalComponentCodecValue<'_>> {
    match value {
        NbtSummaryValue::String(value) => {
            Some(CanonicalComponentCodecValue::String(Cow::Borrowed(value)))
        }
        value if is_java_primitive(value) => Some(CanonicalComponentCodecValue::Nbt(value)),
        _ => canonical_component_at(value, depth),
    }
}

fn canonical_style_fields(
    entries: &[NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, CanonicalComponentCodecValue<'_>)>> {
    let mut fields = Vec::new();
    if let Some(NbtSummaryValue::String(color)) = find_entry(entries, "color") {
        let encoded = if color.units().first() == Some(&(b'#' as u16)) {
            Cow::Owned(canonical_text_color_string(color)?.into())
        } else {
            Cow::Borrowed(color)
        };
        fields.push(("color", CanonicalComponentCodecValue::String(encoded)));
    }
    if let Some(shadow) = find_entry(entries, "shadow_color") {
        // `ARGB_COLOR_CODEC` always re-encodes through its primary `Codec.INT` branch. Numeric
        // NBT values and the vector4f alternative therefore both normalize to one signed int.
        let shadow = canonical_argb_color(shadow)?;
        fields.push(("shadow_color", CanonicalComponentCodecValue::Int(shadow)));
    }
    for field in COMPONENT_STYLE_FLAG_FIELDS {
        if let Some(value) = find_entry(entries, field) {
            fields.push((
                field,
                CanonicalComponentCodecValue::Boolean(component_bool(value)?),
            ));
        }
    }
    if let Some(click_event) = find_entry(entries, "click_event") {
        fields.push(("click_event", canonical_click_event(click_event)?));
    }
    if let Some(hover_event) = find_entry(entries, "hover_event") {
        fields.push(("hover_event", canonical_hover_event(hover_event, depth)?));
    }
    if let Some(insertion) = string_field(entries, "insertion") {
        fields.push(("insertion", insertion));
    }
    if find_entry(entries, "font").is_some() {
        fields.push(("font", normalized_identifier_field(entries, "font")?));
    }
    Some(fields)
}

fn canonical_click_event(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue<'_>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![(
        "action",
        CanonicalComponentCodecValue::String(Cow::Borrowed(action)),
    )];
    match ComponentStaticClickAction::parse(action)? {
        ComponentStaticClickAction::OpenUrl => fields.push(("url", string_field(entries, "url")?)),
        ComponentStaticClickAction::RunCommand | ComponentStaticClickAction::SuggestCommand => {
            fields.push(("command", string_field(entries, "command")?));
        }
        ComponentStaticClickAction::ShowDialog => {
            let dialog = find_entry(entries, "dialog")?;
            fields.push((
                "dialog",
                CanonicalComponentCodecValue::Owner(CanonicalComponentOwnerValue::Dialog, dialog),
            ));
        }
        ComponentStaticClickAction::ChangePage => fields.push((
            "page",
            CanonicalComponentCodecValue::Int(component_i32(find_entry(entries, "page")?)?),
        )),
        ComponentStaticClickAction::CopyToClipboard => {
            fields.push(("value", string_field(entries, "value")?))
        }
        ComponentStaticClickAction::Custom => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            if let Some(payload) = find_entry(entries, "payload") {
                fields.push(("payload", CanonicalComponentCodecValue::Nbt(payload)));
            }
        }
        ComponentStaticClickAction::OpenFile => return None,
    }
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_hover_event(
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<CanonicalComponentCodecValue<'_>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![(
        "action",
        CanonicalComponentCodecValue::String(Cow::Borrowed(action)),
    )];
    match ComponentHoverAction::parse(action)? {
        ComponentHoverAction::ShowText => fields.push((
            "value",
            canonical_component_at(find_entry(entries, "value")?, depth + 1)?,
        )),
        ComponentHoverAction::ShowItem => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            if let Some(count) = find_entry(entries, "count").and_then(component_i32) {
                if count != 1 {
                    fields.push(("count", CanonicalComponentCodecValue::Int(count)));
                }
            }
            if let Some(components) = find_entry(entries, "components") {
                fields.push((
                    "components",
                    CanonicalComponentCodecValue::Owner(
                        CanonicalComponentOwnerValue::ItemComponentPatch,
                        components,
                    ),
                ));
            }
        }
        ComponentHoverAction::ShowEntity => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            fields.push(("uuid", canonical_uuid(find_entry(entries, "uuid")?)?));
            if let Some(name) = find_entry(entries, "name") {
                fields.push(("name", canonical_component_at(name, depth + 1)?));
            }
        }
    }
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_uuid(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue<'static>> {
    super::component_uuid_words(value)
        .map(|words| CanonicalComponentCodecValue::IntArray(words.into()))
}

pub(super) fn canonical_argb_color(value: &NbtSummaryValue) -> Option<i32> {
    if let Some(value) = component_i32(value) {
        return Some(value);
    }
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    let [red, green, blue, alpha] = values.as_slice() else {
        return None;
    };
    Some(argb_from_channels([
        component_f32(red)?,
        component_f32(green)?,
        component_f32(blue)?,
        component_f32(alpha)?,
    ]))
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

fn normalized_identifier_field(
    entries: &[NbtSummaryEntry],
    field: &str,
) -> Option<CanonicalComponentCodecValue<'static>> {
    let value = string_value(find_entry(entries, field)?)?;
    normalize_resource_location(value.try_to_string().ok()?)
        .ok()
        .map(|value| CanonicalComponentCodecValue::String(Cow::Owned(value.into())))
}

fn string_field<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
) -> Option<CanonicalComponentCodecValue<'a>> {
    string_value(find_entry(entries, field)?)
        .map(|value| CanonicalComponentCodecValue::String(Cow::Borrowed(value)))
}

fn string_value(value: &NbtSummaryValue) -> Option<&crate::JavaString> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(value)
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], field: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_typed_numbers_and_native_arrays_for_dynamic_ops() {
        let value = compound(vec![
            ("type", string("translatable")),
            ("translate", string("key")),
            (
                "with",
                list(vec![
                    NbtSummaryValue::Byte(-1),
                    NbtSummaryValue::Short(-2),
                    NbtSummaryValue::Int(-3),
                    NbtSummaryValue::Long(-4),
                    NbtSummaryValue::Float(1.5f32.to_bits()),
                    NbtSummaryValue::Double(2.5f64.to_bits()),
                ]),
            ),
            (
                "click_event",
                compound(vec![
                    ("action", string("custom")),
                    ("id", string("test")),
                    (
                        "payload",
                        compound(vec![
                            ("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
                            ("ints", NbtSummaryValue::IntArray(vec![-3, 4])),
                            ("longs", NbtSummaryValue::LongArray(vec![-5, 6])),
                        ]),
                    ),
                ]),
            ),
        ]);
        assert_eq!(
            canonical_component_codec_value(&value).unwrap(),
            CanonicalComponentCodecValue::Object(vec![
                (
                    "translate",
                    CanonicalComponentCodecValue::String(Cow::Owned("key".into())),
                ),
                (
                    "with",
                    CanonicalComponentCodecValue::List(vec![
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Byte(-1)),
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Short(-2)),
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Int(-3)),
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Long(-4)),
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Float(
                            1.5f32.to_bits()
                        )),
                        CanonicalComponentCodecValue::Nbt(&NbtSummaryValue::Double(
                            2.5f64.to_bits()
                        )),
                    ]),
                ),
                (
                    "click_event",
                    CanonicalComponentCodecValue::Object(vec![
                        (
                            "action",
                            CanonicalComponentCodecValue::String(Cow::Owned("custom".into())),
                        ),
                        (
                            "id",
                            CanonicalComponentCodecValue::String(Cow::Owned(
                                "minecraft:test".into()
                            )),
                        ),
                        (
                            "payload",
                            CanonicalComponentCodecValue::Nbt(&compound(vec![
                                ("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
                                ("ints", NbtSummaryValue::IntArray(vec![-3, 4])),
                                ("longs", NbtSummaryValue::LongArray(vec![-5, 6])),
                            ])),
                        ),
                    ]),
                ),
            ])
        );
    }

    #[test]
    fn retains_uuid_array_and_semantic_owner_nodes() {
        let entity = compound(vec![
            ("text", string("entity")),
            (
                "hover_event",
                compound(vec![
                    ("action", string("show_entity")),
                    ("id", string("pig")),
                    ("uuid", string("1-2-3-4-5")),
                ]),
            ),
        ]);
        let encoded = canonical_component_codec_value(&entity).unwrap();
        let CanonicalComponentCodecValue::Object(fields) = encoded else {
            panic!("styled text component should encode as a map");
        };
        assert_eq!(
            fields[1],
            (
                "hover_event",
                CanonicalComponentCodecValue::Object(vec![
                    (
                        "action",
                        CanonicalComponentCodecValue::String(Cow::Owned("show_entity".into())),
                    ),
                    (
                        "id",
                        CanonicalComponentCodecValue::String(Cow::Owned("minecraft:pig".into())),
                    ),
                    (
                        "uuid",
                        CanonicalComponentCodecValue::IntArray(vec![1, 131_075, 262_144, 5]),
                    ),
                ]),
            )
        );

        let player = string("Alex");
        let object = compound(vec![
            ("object", string("player")),
            ("player", player.clone()),
        ]);
        assert_eq!(
            canonical_component_codec_value(&object).unwrap(),
            CanonicalComponentCodecValue::Object(vec![(
                "player",
                CanonicalComponentCodecValue::Owner(
                    CanonicalComponentOwnerValue::PlayerProfile,
                    &player,
                ),
            )])
        );
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string().into())
    }

    fn list(values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
        NbtSummaryValue::List(values)
    }

    fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.into(),
                    value,
                })
                .collect(),
        )
    }
}
