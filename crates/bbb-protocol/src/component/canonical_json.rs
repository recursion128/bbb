use super::{
    codec::{
        component_bool, component_i32, decoded_content_type, decoded_data_source_type,
        decoded_object_type, validate_component,
    },
    parse_text_color, validate_component_nbt,
};
use crate::{
    nbt_summary::{NbtSummaryEntry, NbtSummaryValue},
    packets::{
        inventory::{
            GameProfilePropertySummary, PlayerModelTypeSummary, PlayerSkinPatchSummary,
            ResolvableProfileKindSummary, ResolvableProfileSummary,
        },
        normalize_resource_location,
    },
};

mod java_number;

use java_number::{java_double_string, java_float_string};

/// Returns the exact UTF-16 length of vanilla 26.1's canonical JsonOps encoding when the
/// registry-independent Component model is sufficient to reproduce it.
///
/// `None` means that decoding succeeded but canonical re-encoding still needs a higher semantic
/// owner (a player profile, dialog, or item component patch). It is deliberately not an estimate:
/// callers enforcing `flatRestrictedCodec` must only act on an exact `Some` result.
pub fn canonical_component_json_utf16_len(
    value: &NbtSummaryValue,
) -> Result<Option<usize>, String> {
    canonical_component_json_utf16_len_with_owner(value, |_, _| Ok(None))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalComponentOwnerValue {
    PlayerProfile,
    Dialog,
    ItemComponentPatch,
}

/// Returns the exact canonical Component JSON length while delegating values whose Codec belongs
/// to a higher semantic owner. The callback receives the selected owner kind and its raw NBT
/// value; `None` keeps the complete Component length unknown rather than estimating it, while an
/// owner error is propagated to the caller.
pub fn canonical_component_json_utf16_len_with_owner(
    value: &NbtSummaryValue,
    mut owner_value_utf16_len: impl FnMut(
        CanonicalComponentOwnerValue,
        &NbtSummaryValue,
    ) -> Result<Option<usize>, String>,
) -> Result<Option<usize>, String> {
    validate_component_nbt(value, "component")?;
    let mut owner_error = None;
    let mut owner_adapter = |kind, value: &NbtSummaryValue| match owner_value_utf16_len(kind, value)
    {
        Ok(length) => length,
        Err(error) => {
            owner_error.get_or_insert(error);
            None
        }
    };
    let value = canonical_component(value, &mut owner_adapter);
    if let Some(error) = owner_error {
        return Err(error);
    }
    let Some(value) = value else {
        return Ok(None);
    };
    if value.has_non_finite_number() {
        return Err("canonical JsonWriter rejects non-finite Float/Double values".to_string());
    }
    Ok(Some(value.utf16_len()))
}

/// Exact JsonOps encoding length for a `ResolvableProfile` that has already been decoded and
/// normalized by its semantic owner.
pub fn canonical_resolvable_profile_json_utf16_len(
    profile: &ResolvableProfileSummary,
) -> Option<usize> {
    canonical_resolvable_profile(profile).map(|value| value.utf16_len())
}

/// Exact JsonOps/Gson length for a finite Java `float` value that has already passed its semantic
/// Codec validation.
pub fn canonical_f32_json_utf16_len(bits: u32) -> Option<usize> {
    let encoded = java_float_string(f32::from_bits(bits));
    (!matches!(encoded.as_str(), "NaN" | "Infinity" | "-Infinity")).then_some(encoded.len())
}

/// Formats a Java `float` exactly like `Float.toString`.
///
/// This is also the textual value used by vanilla dialog number-range template substitutions.
pub fn java_float_to_string(value: f32) -> String {
    java_float_string(value)
}

pub(crate) fn java_double_to_string(value: f64) -> String {
    java_double_string(value)
}

/// Exact JsonOps/Gson length for a finite Java `double` value that has already passed its
/// semantic Codec validation.
pub fn canonical_f64_json_utf16_len(bits: u64) -> Option<usize> {
    let encoded = java_double_string(f64::from_bits(bits));
    (!matches!(encoded.as_str(), "NaN" | "Infinity" | "-Infinity")).then_some(encoded.len())
}

/// Exact Gson string length in UTF-16 units, including quotes and required JSON escaping.
pub fn canonical_json_string_utf16_len(value: &str) -> usize {
    json_string_utf16_len(value)
}

/// Exact JsonOps/Gson length for an NBT value after vanilla's `NbtOps -> JsonOps`
/// conversion. Compound keys follow `CompoundTag` last-wins semantics.
pub fn canonical_nbt_json_utf16_len(value: &NbtSummaryValue) -> Result<usize, String> {
    let value = canonical_nbt(value)
        .ok_or_else(|| "canonical NBT value cannot be converted to JsonOps".to_string())?;
    if value.has_non_finite_number() {
        return Err("canonical JsonWriter rejects non-finite Float/Double values".to_string());
    }
    Ok(value.utf16_len())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(&'static str, JsonValue)>),
    Map(Vec<(String, JsonValue)>),
    EncodedLen(usize),
}

impl JsonValue {
    fn has_non_finite_number(&self) -> bool {
        match self {
            Self::Number(value) => matches!(value.as_str(), "NaN" | "Infinity" | "-Infinity"),
            Self::Array(values) => values.iter().any(Self::has_non_finite_number),
            Self::Object(fields) => fields
                .iter()
                .any(|(_, value)| value.has_non_finite_number()),
            Self::Map(fields) => fields
                .iter()
                .any(|(_, value)| value.has_non_finite_number()),
            Self::Bool(_) | Self::String(_) | Self::EncodedLen(_) => false,
        }
    }

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
            Self::EncodedLen(length) => *length,
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

type OwnerValueLength<'a> =
    dyn FnMut(CanonicalComponentOwnerValue, &NbtSummaryValue) -> Option<usize> + 'a;

fn canonical_component(
    value: &NbtSummaryValue,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    canonical_component_at(value, 0, owner_value_utf16_len)
}

type ComponentRef<'a> = (&'a NbtSummaryValue, usize);

fn canonical_component_at(
    value: &NbtSummaryValue,
    depth: usize,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    canonical_component_with_siblings(value, depth, Vec::new(), owner_value_utf16_len)
}

fn canonical_component_with_siblings<'a>(
    value: &'a NbtSummaryValue,
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    match value {
        NbtSummaryValue::String(text) => {
            if appended_siblings.is_empty() {
                Some(JsonValue::String(text.clone()))
            } else {
                Some(JsonValue::Object(vec![
                    ("text", JsonValue::String(text.clone())),
                    (
                        "extra",
                        canonical_component_list(&appended_siblings, owner_value_utf16_len)?,
                    ),
                ]))
            }
        }
        NbtSummaryValue::List(values) => {
            let (first, rest) = values.split_first()?;
            let mut siblings = Vec::with_capacity(rest.len() + appended_siblings.len());
            siblings.extend(rest.iter().map(|value| (value, depth + 1)));
            siblings.extend(appended_siblings);
            canonical_component_with_siblings(first, depth + 1, siblings, owner_value_utf16_len)
        }
        NbtSummaryValue::Compound(entries) => {
            canonical_full_component(entries, depth, appended_siblings, owner_value_utf16_len)
        }
        _ => None,
    }
}

fn canonical_full_component<'a>(
    entries: &'a [NbtSummaryEntry],
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    let content_type = decoded_content_type(entries, "component", depth);
    let mut fields = canonical_content_fields(content_type, entries, depth, owner_value_utf16_len)?;
    let mut siblings = match find_entry(entries, "extra") {
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(|value| (value, depth + 1))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    siblings.extend(appended_siblings);
    let style = canonical_style_fields(entries, depth, owner_value_utf16_len)?;

    if content_type == "text" && siblings.is_empty() && style.is_empty() {
        let NbtSummaryValue::String(text) = find_entry(entries, "text")? else {
            unreachable!("plain text content was validated before canonical encoding")
        };
        return Some(JsonValue::String(text.clone()));
    }
    if !siblings.is_empty() {
        fields.push((
            "extra",
            canonical_component_list(&siblings, owner_value_utf16_len)?,
        ));
    }
    fields.extend(style);
    Some(JsonValue::Object(fields))
}

fn canonical_component_list(
    values: &[ComponentRef<'_>],
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    values
        .iter()
        .map(|(value, depth)| canonical_component_at(value, *depth, owner_value_utf16_len))
        .collect::<Option<Vec<_>>>()
        .map(JsonValue::Array)
}

fn canonical_content_fields(
    content_type: &str,
    entries: &[NbtSummaryEntry],
    depth: usize,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
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
                        .map(|argument| {
                            canonical_translation_argument(
                                argument,
                                depth + 1,
                                owner_value_utf16_len,
                            )
                        })
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
                fields.push((
                    "separator",
                    canonical_component_at(separator, depth + 1, owner_value_utf16_len)?,
                ));
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
                    fields.push((
                        "separator",
                        canonical_component_at(separator, depth + 1, owner_value_utf16_len)?,
                    ));
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
                "player" => {
                    let player = find_entry(entries, "player")?;
                    let encoded_len =
                        owner_value_utf16_len(CanonicalComponentOwnerValue::PlayerProfile, player)?;
                    fields.push(("player", JsonValue::EncodedLen(encoded_len)));
                    if !find_entry(entries, "hat")
                        .and_then(component_bool)
                        .unwrap_or(true)
                    {
                        fields.push(("hat", JsonValue::Bool(false)));
                    }
                }
                _ => unreachable!("component object was validated before canonical encoding"),
            }
            if let Some(fallback) = find_entry(entries, "fallback") {
                fields.push((
                    "fallback",
                    canonical_component_at(fallback, depth + 1, owner_value_utf16_len)?,
                ));
            }
        }
        _ => unreachable!("component content was validated before canonical encoding"),
    }
    Some(fields)
}

fn canonical_translation_argument(
    value: &NbtSummaryValue,
    depth: usize,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    match value {
        NbtSummaryValue::Byte(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Short(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Int(value) => Some(number(i64::from(*value))),
        NbtSummaryValue::Long(value) => Some(number(*value)),
        NbtSummaryValue::Float(bits) => Some(java_float(f32::from_bits(*bits))),
        NbtSummaryValue::Double(bits) => Some(java_double(f64::from_bits(*bits))),
        NbtSummaryValue::String(value) => Some(JsonValue::String(value.clone())),
        _ => canonical_component_at(value, depth, owner_value_utf16_len),
    }
}

fn canonical_style_fields(
    entries: &[NbtSummaryEntry],
    depth: usize,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
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
        fields.push((
            "click_event",
            canonical_click_event(click_event, owner_value_utf16_len)?,
        ));
    }
    if let Some(hover_event) = find_entry(entries, "hover_event") {
        fields.push((
            "hover_event",
            canonical_hover_event(hover_event, depth, owner_value_utf16_len)?,
        ));
    }
    if let Some(NbtSummaryValue::String(insertion)) = find_entry(entries, "insertion") {
        fields.push(("insertion", JsonValue::String(insertion.clone())));
    }
    if find_entry(entries, "font").is_some() {
        fields.push(("font", normalized_identifier_field(entries, "font")?));
    }
    Some(fields)
}

fn canonical_click_event(
    value: &NbtSummaryValue,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
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
        "show_dialog" => {
            let dialog = find_entry(entries, "dialog")?;
            let encoded_len = owner_value_utf16_len(CanonicalComponentOwnerValue::Dialog, dialog)?;
            fields.push(("dialog", JsonValue::EncodedLen(encoded_len)));
        }
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

fn canonical_hover_event(
    value: &NbtSummaryValue,
    depth: usize,
    owner_value_utf16_len: &mut OwnerValueLength<'_>,
) -> Option<JsonValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![("action", JsonValue::String(action.to_string()))];
    match action {
        "show_text" => fields.push((
            "value",
            canonical_component_at(
                find_entry(entries, "value")?,
                depth + 1,
                owner_value_utf16_len,
            )?,
        )),
        "show_item" => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            if let Some(count) = find_entry(entries, "count").and_then(component_i32) {
                if count != 1 {
                    fields.push(("count", number(i64::from(count))));
                }
            }
            if let Some(components) = find_entry(entries, "components") {
                let encoded_len = owner_value_utf16_len(
                    CanonicalComponentOwnerValue::ItemComponentPatch,
                    components,
                )?;
                fields.push(("components", JsonValue::EncodedLen(encoded_len)));
            }
        }
        "show_entity" => {
            fields.push(("id", normalized_identifier_field(entries, "id")?));
            fields.push(("uuid", canonical_uuid(find_entry(entries, "uuid")?)?));
            if let Some(name) = find_entry(entries, "name") {
                fields.push((
                    "name",
                    canonical_component_at(name, depth + 1, owner_value_utf16_len)?,
                ));
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

fn canonical_resolvable_profile(profile: &ResolvableProfileSummary) -> Option<JsonValue> {
    let mut fields = Vec::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            fields.push(("id", canonical_uuid_value(profile.uuid?)));
            fields.push(("name", JsonValue::String(profile.name.clone()?)));
        }
        ResolvableProfileKindSummary::Partial => {
            if let Some(name) = &profile.name {
                fields.push(("name", JsonValue::String(name.clone())));
            }
            if let Some(uuid) = profile.uuid {
                fields.push(("id", canonical_uuid_value(uuid)));
            }
        }
    }
    if !profile.properties.is_empty() {
        fields.push((
            "properties",
            JsonValue::Array(
                profile
                    .properties
                    .iter()
                    .map(canonical_profile_property)
                    .collect(),
            ),
        ));
    }
    append_skin_patch_fields(&mut fields, &profile.skin_patch);
    Some(JsonValue::Object(fields))
}

fn canonical_uuid_value(uuid: uuid::Uuid) -> JsonValue {
    let bits = uuid.as_u128();
    JsonValue::Array(
        [
            (bits >> 96) as u32 as i32,
            (bits >> 64) as u32 as i32,
            (bits >> 32) as u32 as i32,
            bits as u32 as i32,
        ]
        .into_iter()
        .map(|value| number(i64::from(value)))
        .collect(),
    )
}

fn canonical_profile_property(property: &GameProfilePropertySummary) -> JsonValue {
    let mut fields = vec![
        ("name", JsonValue::String(property.name.clone())),
        ("value", JsonValue::String(property.value.clone())),
    ];
    if let Some(signature) = &property.signature {
        fields.push(("signature", JsonValue::String(signature.clone())));
    }
    JsonValue::Object(fields)
}

fn append_skin_patch_fields(
    fields: &mut Vec<(&'static str, JsonValue)>,
    patch: &PlayerSkinPatchSummary,
) {
    for (field, texture) in [
        ("texture", patch.body.as_ref()),
        ("cape", patch.cape.as_ref()),
        ("elytra", patch.elytra.as_ref()),
    ] {
        if let Some(texture) = texture {
            fields.push((field, JsonValue::String(texture.asset_id.clone())));
        }
    }
    if let Some(model) = patch.model {
        fields.push((
            "model",
            JsonValue::String(
                match model {
                    PlayerModelTypeSummary::Slim => "slim",
                    PlayerModelTypeSummary::Wide => "wide",
                }
                .to_string(),
            ),
        ));
    }
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
        NbtSummaryValue::Float(bits) => java_float(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => java_double(f64::from_bits(*bits)),
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

fn java_float(value: f32) -> JsonValue {
    JsonValue::Number(java_float_string(value))
}

fn java_double(value: f64) -> JsonValue {
    JsonValue::Number(java_double_string(value))
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
    fn canonicalizes_raw_nbt_with_last_wins_and_strict_numbers() {
        let value = compound(vec![
            ("key", NbtSummaryValue::Int(1)),
            ("key", string("last")),
            ("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
            (
                "nested",
                list(vec![
                    NbtSummaryValue::Float(8.0f32.to_bits()),
                    NbtSummaryValue::Long(9),
                ]),
            ),
        ]);
        assert_eq!(
            canonical_nbt_json_utf16_len(&value).unwrap(),
            r#"{"key":"last","bytes":[-1,2],"nested":[8.0,9]}"#.len()
        );

        let non_finite = compound(vec![(
            "value",
            NbtSummaryValue::Double(f64::NEG_INFINITY.to_bits()),
        )]);
        assert!(canonical_nbt_json_utf16_len(&non_finite)
            .unwrap_err()
            .contains("non-finite"));
    }

    #[test]
    fn canonicalizes_java_float_and_double_values() {
        let translation = compound(vec![
            ("translate", string("key")),
            ("with", list(vec![NbtSummaryValue::Float(1)])),
        ]);
        assert_eq!(
            exact(&translation),
            r#"{"translate":"key","with":[1.4E-45]}"#.len()
        );
        assert_eq!(canonical_f32_json_utf16_len(1), Some("1.4E-45".len()));
        assert_eq!(canonical_f32_json_utf16_len(f32::NAN.to_bits()), None);
        assert_eq!(canonical_f64_json_utf16_len(1), Some("4.9E-324".len()));
        assert_eq!(canonical_f64_json_utf16_len(f64::NAN.to_bits()), None);
        assert_eq!(canonical_json_string_utf16_len("\"\u{2028}😀"), 12);
        let custom = compound(vec![
            ("text", string("custom")),
            (
                "click_event",
                compound(vec![
                    ("action", string("custom")),
                    ("id", string("test")),
                    ("payload", NbtSummaryValue::Double(1)),
                ]),
            ),
        ]);
        assert_eq!(
            exact(&custom),
            r#"{"text":"custom","click_event":{"action":"custom","id":"minecraft:test","payload":4.9E-324}}"#.len()
        );

        let non_finite = compound(vec![
            ("translate", string("key")),
            (
                "with",
                list(vec![NbtSummaryValue::Float(f32::NAN.to_bits())]),
            ),
        ]);
        assert!(canonical_component_json_utf16_len(&non_finite)
            .unwrap_err()
            .contains("non-finite"));

        // Fuzzy Component encoding discards unknown fields before Gson's strict writer sees them.
        let ignored = compound(vec![
            ("text", string("ok")),
            ("unknown", NbtSummaryValue::Double(f64::INFINITY.to_bits())),
        ]);
        assert_eq!(exact(&ignored), r#""ok""#.len());
    }

    #[test]
    fn canonicalizes_resolvable_profiles_through_the_semantic_owner() {
        let partial = ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some("Alex".to_string()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        };
        assert_eq!(
            canonical_resolvable_profile_json_utf16_len(&partial),
            Some(r#"{"name":"Alex"}"#.len())
        );

        let complete = ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(uuid::Uuid::from_u128(1)),
            name: Some("Alex".to_string()),
            properties: vec![GameProfilePropertySummary {
                name: "textures".to_string(),
                value: "value".to_string(),
                signature: Some("sig".to_string()),
            }],
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary {
                body: Some(crate::packets::inventory::ResourceTextureSummary {
                    asset_id: "minecraft:entity/player".to_string(),
                    texture_path: "ignored-derived-path".to_string(),
                }),
                cape: Some(crate::packets::inventory::ResourceTextureSummary {
                    asset_id: "example:cape".to_string(),
                    texture_path: "ignored-derived-path".to_string(),
                }),
                elytra: Some(crate::packets::inventory::ResourceTextureSummary {
                    asset_id: "minecraft:elytra".to_string(),
                    texture_path: "ignored-derived-path".to_string(),
                }),
                model: Some(PlayerModelTypeSummary::Wide),
            },
        };
        assert_eq!(
            canonical_resolvable_profile_json_utf16_len(&complete),
            Some(
                concat!(
                    r#"{"id":[0,0,0,1],"name":"Alex","properties":[{"name":"textures","value":"value","signature":"sig"}],"texture":"minecraft:entity/player","cape":"example:cape","elytra":"minecraft:elytra","model":"wide"}"#
                )
                .len()
            )
        );

        let invalid = ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: None,
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        };
        assert_eq!(canonical_resolvable_profile_json_utf16_len(&invalid), None);

        let player = compound(vec![
            ("object", string("player")),
            ("player", string("Alex")),
            ("hat", NbtSummaryValue::Byte(0)),
        ]);
        assert_eq!(canonical_component_json_utf16_len(&player).unwrap(), None);
        assert_eq!(
            canonical_component_json_utf16_len_with_owner(&player, |_, _| {
                Err("owner encoding failed".to_string())
            })
            .unwrap_err(),
            "owner encoding failed"
        );
        assert_eq!(
            canonical_component_json_utf16_len_with_owner(&player, |kind, _| {
                assert_eq!(kind, CanonicalComponentOwnerValue::PlayerProfile);
                Ok(canonical_resolvable_profile_json_utf16_len(&partial))
            })
            .unwrap(),
            Some(r#"{"player":{"name":"Alex"},"hat":false}"#.len())
        );
    }

    #[test]
    fn reports_remaining_owner_values_as_unknown() {
        let value = compound(vec![
            ("text", string("item")),
            (
                "hover_event",
                compound(vec![
                    ("action", string("show_item")),
                    ("id", string("stone")),
                    ("components", compound(Vec::new())),
                ]),
            ),
        ]);
        assert_eq!(canonical_component_json_utf16_len(&value).unwrap(), None);
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
