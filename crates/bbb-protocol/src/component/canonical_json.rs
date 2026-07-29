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
    let value = canonical_component_codec_value(value)?;
    let mut owner_error = None;
    let mut owner_adapter = |kind, value: &NbtSummaryValue| match owner_value_utf16_len(kind, value)
    {
        Ok(length) => length,
        Err(error) => {
            owner_error.get_or_insert(error);
            None
        }
    };
    if value.has_non_finite_number() {
        return Err("canonical JsonWriter rejects non-finite Float/Double values".to_string());
    }
    let length = value.json_utf16_len(&mut owner_adapter);
    if let Some(error) = owner_error {
        return Err(error);
    }
    Ok(length)
}

/// Canonical value emitted by vanilla 26.1 `ComponentSerialization.CODEC`
/// before a concrete `DynamicOps` chooses its wire representation.
///
/// Numeric variants remain distinct so consumers such as `HashOps` do not
/// lose the primitive type through the JsonOps representation. Owner nodes
/// preserve values whose codecs belong to a surrounding registry-aware
/// subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalComponentCodecValue {
    Boolean(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    String(String),
    List(Vec<CanonicalComponentCodecValue>),
    ByteArray(Vec<i8>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
    Object(Vec<(&'static str, CanonicalComponentCodecValue)>),
    Map(Vec<(String, CanonicalComponentCodecValue)>),
    Owner(CanonicalComponentOwnerValue, NbtSummaryValue),
}

/// Decodes and canonically re-encodes one Component into a typed,
/// `DynamicOps`-independent value tree.
pub fn canonical_component_codec_value(
    value: &NbtSummaryValue,
) -> Result<CanonicalComponentCodecValue, String> {
    validate_component_nbt(value, "component")?;
    canonical_component(value)
        .ok_or_else(|| "validated component could not be canonically encoded".to_string())
}

/// Exact JsonOps encoding length for a `ResolvableProfile` that has already been decoded and
/// normalized by its semantic owner.
pub fn canonical_resolvable_profile_json_utf16_len(
    profile: &ResolvableProfileSummary,
) -> Option<usize> {
    canonical_resolvable_profile(profile).and_then(|value| value.json_utf16_len(&mut |_, _| None))
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
    value
        .json_utf16_len(&mut |_, _| None)
        .ok_or_else(|| "canonical NBT value unexpectedly requires an owner".to_string())
}

impl CanonicalComponentCodecValue {
    fn has_non_finite_number(&self) -> bool {
        match self {
            Self::Float(bits) => !f32::from_bits(*bits).is_finite(),
            Self::Double(bits) => !f64::from_bits(*bits).is_finite(),
            Self::List(values) => values.iter().any(Self::has_non_finite_number),
            Self::Object(fields) => fields
                .iter()
                .any(|(_, value)| value.has_non_finite_number()),
            Self::Map(fields) => fields
                .iter()
                .any(|(_, value)| value.has_non_finite_number()),
            Self::Boolean(_)
            | Self::Byte(_)
            | Self::Short(_)
            | Self::Int(_)
            | Self::Long(_)
            | Self::String(_)
            | Self::ByteArray(_)
            | Self::IntArray(_)
            | Self::LongArray(_)
            | Self::Owner(_, _) => false,
        }
    }

    fn json_utf16_len(
        &self,
        owner_value_utf16_len: &mut dyn FnMut(
            CanonicalComponentOwnerValue,
            &NbtSummaryValue,
        ) -> Option<usize>,
    ) -> Option<usize> {
        Some(match self {
            Self::Boolean(true) => 4,
            Self::Boolean(false) => 5,
            Self::Byte(value) => value.to_string().len(),
            Self::Short(value) => value.to_string().len(),
            Self::Int(value) => value.to_string().len(),
            Self::Long(value) => value.to_string().len(),
            Self::Float(bits) => java_float_string(f32::from_bits(*bits)).len(),
            Self::Double(bits) => java_double_string(f64::from_bits(*bits)).len(),
            Self::String(value) => json_string_utf16_len(value),
            Self::List(values) => collection_utf16_len(
                values
                    .iter()
                    .map(|value| value.json_utf16_len(owner_value_utf16_len))
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
            Self::ByteArray(values) => {
                collection_utf16_len(values.iter().map(|value| value.to_string().len()))
            }
            Self::IntArray(values) => {
                collection_utf16_len(values.iter().map(|value| value.to_string().len()))
            }
            Self::LongArray(values) => {
                collection_utf16_len(values.iter().map(|value| value.to_string().len()))
            }
            Self::Object(fields) => {
                let entries = fields
                    .iter()
                    .map(|(name, value)| {
                        Some(
                            json_string_utf16_len(name)
                                .saturating_add(1)
                                .saturating_add(value.json_utf16_len(owner_value_utf16_len)?),
                        )
                    })
                    .collect::<Option<Vec<_>>>()?;
                collection_utf16_len(entries.into_iter())
            }
            Self::Map(fields) => {
                let entries = fields
                    .iter()
                    .map(|(name, value)| {
                        Some(
                            json_string_utf16_len(name)
                                .saturating_add(1)
                                .saturating_add(value.json_utf16_len(owner_value_utf16_len)?),
                        )
                    })
                    .collect::<Option<Vec<_>>>()?;
                collection_utf16_len(entries.into_iter())
            }
            Self::Owner(kind, value) => owner_value_utf16_len(*kind, value)?,
        })
    }
}

fn collection_utf16_len(entries: impl Iterator<Item = usize>) -> usize {
    entries.enumerate().fold(2usize, |length, (index, entry)| {
        length
            .saturating_add(usize::from(index != 0))
            .saturating_add(entry)
    })
}

fn canonical_component(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue> {
    canonical_component_at(value, 0)
}

type ComponentRef<'a> = (&'a NbtSummaryValue, usize);

fn canonical_component_at(
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<CanonicalComponentCodecValue> {
    canonical_component_with_siblings(value, depth, Vec::new())
}

fn canonical_component_with_siblings<'a>(
    value: &'a NbtSummaryValue,
    depth: usize,
    appended_siblings: Vec<ComponentRef<'a>>,
) -> Option<CanonicalComponentCodecValue> {
    #[cfg(test)]
    crate::test_stack_budget::record();

    match value {
        NbtSummaryValue::String(text) => {
            if appended_siblings.is_empty() {
                Some(CanonicalComponentCodecValue::String(text.clone()))
            } else {
                Some(CanonicalComponentCodecValue::Object(vec![
                    ("text", CanonicalComponentCodecValue::String(text.clone())),
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
) -> Option<CanonicalComponentCodecValue> {
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
        return Some(CanonicalComponentCodecValue::String(text.clone()));
    }
    if !siblings.is_empty() {
        fields.push(("extra", canonical_component_list(&siblings)?));
    }
    fields.extend(style);
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_component_list(values: &[ComponentRef<'_>]) -> Option<CanonicalComponentCodecValue> {
    values
        .iter()
        .map(|(value, depth)| canonical_component_at(value, *depth))
        .collect::<Option<Vec<_>>>()
        .map(CanonicalComponentCodecValue::List)
}

fn canonical_content_fields(
    content_type: &str,
    entries: &[NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, CanonicalComponentCodecValue)>> {
    let mut fields = Vec::new();
    match content_type {
        "text" => fields.push(("text", string_field(entries, "text")?)),
        "translatable" => {
            fields.push(("translate", string_field(entries, "translate")?));
            if let Some(NbtSummaryValue::String(fallback)) = find_entry(entries, "fallback") {
                fields.push((
                    "fallback",
                    CanonicalComponentCodecValue::String(fallback.clone()),
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
                        None => {
                            CanonicalComponentCodecValue::String("minecraft:blocks".to_string())
                        }
                    };
                    if atlas != CanonicalComponentCodecValue::String("minecraft:blocks".to_string())
                    {
                        fields.push(("atlas", atlas));
                    }
                    fields.push(("sprite", normalized_identifier_field(entries, "sprite")?));
                }
                "player" => {
                    let player = find_entry(entries, "player")?;
                    fields.push((
                        "player",
                        CanonicalComponentCodecValue::Owner(
                            CanonicalComponentOwnerValue::PlayerProfile,
                            player.clone(),
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
) -> Option<CanonicalComponentCodecValue> {
    match value {
        NbtSummaryValue::Byte(value) => Some(CanonicalComponentCodecValue::Byte(*value)),
        NbtSummaryValue::Short(value) => Some(CanonicalComponentCodecValue::Short(*value)),
        NbtSummaryValue::Int(value) => Some(CanonicalComponentCodecValue::Int(*value)),
        NbtSummaryValue::Long(value) => Some(CanonicalComponentCodecValue::Long(*value)),
        NbtSummaryValue::Float(bits) => Some(CanonicalComponentCodecValue::Float(*bits)),
        NbtSummaryValue::Double(bits) => Some(CanonicalComponentCodecValue::Double(*bits)),
        NbtSummaryValue::String(value) => Some(CanonicalComponentCodecValue::String(value.clone())),
        _ => canonical_component_at(value, depth),
    }
}

fn canonical_style_fields(
    entries: &[NbtSummaryEntry],
    depth: usize,
) -> Option<Vec<(&'static str, CanonicalComponentCodecValue)>> {
    let mut fields = Vec::new();
    if let Some(NbtSummaryValue::String(color)) = find_entry(entries, "color") {
        let encoded = if color.starts_with('#') {
            format!("#{:06X}", parse_text_color(color)?)
        } else {
            color.clone()
        };
        fields.push(("color", CanonicalComponentCodecValue::String(encoded)));
    }
    if let Some(shadow) = find_entry(entries, "shadow_color") {
        // `ARGB_COLOR_CODEC` always re-encodes through its primary `Codec.INT` branch. Numeric
        // NBT values and the vector4f alternative therefore both normalize to one signed int.
        let shadow = canonical_argb_color(shadow)?;
        fields.push(("shadow_color", CanonicalComponentCodecValue::Int(shadow)));
    }
    for field in [
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
    ] {
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
    if let Some(NbtSummaryValue::String(insertion)) = find_entry(entries, "insertion") {
        fields.push((
            "insertion",
            CanonicalComponentCodecValue::String(insertion.clone()),
        ));
    }
    if find_entry(entries, "font").is_some() {
        fields.push(("font", normalized_identifier_field(entries, "font")?));
    }
    Some(fields)
}

fn canonical_click_event(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![(
        "action",
        CanonicalComponentCodecValue::String(action.to_string()),
    )];
    match action {
        "open_url" => fields.push(("url", string_field(entries, "url")?)),
        "run_command" | "suggest_command" => {
            fields.push(("command", string_field(entries, "command")?));
        }
        "show_dialog" => {
            let dialog = find_entry(entries, "dialog")?;
            fields.push((
                "dialog",
                CanonicalComponentCodecValue::Owner(
                    CanonicalComponentOwnerValue::Dialog,
                    dialog.clone(),
                ),
            ));
        }
        "change_page" => fields.push((
            "page",
            CanonicalComponentCodecValue::Int(component_i32(find_entry(entries, "page")?)?),
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
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_hover_event(
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<CanonicalComponentCodecValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let action = string_value(find_entry(entries, "action")?)?;
    let mut fields = vec![(
        "action",
        CanonicalComponentCodecValue::String(action.to_string()),
    )];
    match action {
        "show_text" => fields.push((
            "value",
            canonical_component_at(find_entry(entries, "value")?, depth + 1)?,
        )),
        "show_item" => {
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
                        components.clone(),
                    ),
                ));
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
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_uuid(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue> {
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
    Some(CanonicalComponentCodecValue::IntArray(values))
}

fn canonical_resolvable_profile(
    profile: &ResolvableProfileSummary,
) -> Option<CanonicalComponentCodecValue> {
    let mut fields = Vec::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            fields.push(("id", canonical_uuid_value(profile.uuid?)));
            fields.push((
                "name",
                CanonicalComponentCodecValue::String(profile.name.clone()?),
            ));
        }
        ResolvableProfileKindSummary::Partial => {
            if let Some(name) = &profile.name {
                fields.push(("name", CanonicalComponentCodecValue::String(name.clone())));
            }
            if let Some(uuid) = profile.uuid {
                fields.push(("id", canonical_uuid_value(uuid)));
            }
        }
    }
    if !profile.properties.is_empty() {
        fields.push((
            "properties",
            CanonicalComponentCodecValue::List(
                profile
                    .properties
                    .iter()
                    .map(canonical_profile_property)
                    .collect(),
            ),
        ));
    }
    append_skin_patch_fields(&mut fields, &profile.skin_patch);
    Some(CanonicalComponentCodecValue::Object(fields))
}

fn canonical_uuid_value(uuid: uuid::Uuid) -> CanonicalComponentCodecValue {
    let bits = uuid.as_u128();
    CanonicalComponentCodecValue::IntArray(vec![
        (bits >> 96) as u32 as i32,
        (bits >> 64) as u32 as i32,
        (bits >> 32) as u32 as i32,
        bits as u32 as i32,
    ])
}

fn canonical_profile_property(
    property: &GameProfilePropertySummary,
) -> CanonicalComponentCodecValue {
    let mut fields = vec![
        (
            "name",
            CanonicalComponentCodecValue::String(property.name.clone()),
        ),
        (
            "value",
            CanonicalComponentCodecValue::String(property.value.clone()),
        ),
    ];
    if let Some(signature) = &property.signature {
        fields.push((
            "signature",
            CanonicalComponentCodecValue::String(signature.clone()),
        ));
    }
    CanonicalComponentCodecValue::Object(fields)
}

fn append_skin_patch_fields(
    fields: &mut Vec<(&'static str, CanonicalComponentCodecValue)>,
    patch: &PlayerSkinPatchSummary,
) {
    for (field, texture) in [
        ("texture", patch.body.as_ref()),
        ("cape", patch.cape.as_ref()),
        ("elytra", patch.elytra.as_ref()),
    ] {
        if let Some(texture) = texture {
            fields.push((
                field,
                CanonicalComponentCodecValue::String(texture.asset_id.clone()),
            ));
        }
    }
    if let Some(model) = patch.model {
        fields.push((
            "model",
            CanonicalComponentCodecValue::String(
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

fn canonical_nbt(value: &NbtSummaryValue) -> Option<CanonicalComponentCodecValue> {
    Some(match value {
        NbtSummaryValue::Byte(value) => CanonicalComponentCodecValue::Byte(*value),
        NbtSummaryValue::Short(value) => CanonicalComponentCodecValue::Short(*value),
        NbtSummaryValue::Int(value) => CanonicalComponentCodecValue::Int(*value),
        NbtSummaryValue::Long(value) => CanonicalComponentCodecValue::Long(*value),
        NbtSummaryValue::Float(bits) => CanonicalComponentCodecValue::Float(*bits),
        NbtSummaryValue::Double(bits) => CanonicalComponentCodecValue::Double(*bits),
        NbtSummaryValue::ByteArray(values) => {
            CanonicalComponentCodecValue::ByteArray(values.clone())
        }
        NbtSummaryValue::String(value) => CanonicalComponentCodecValue::String(value.clone()),
        NbtSummaryValue::List(values) => CanonicalComponentCodecValue::List(
            values
                .iter()
                .map(canonical_nbt)
                .collect::<Option<Vec<_>>>()?,
        ),
        NbtSummaryValue::Compound(entries) => {
            CanonicalComponentCodecValue::Map(canonical_nbt_map(entries)?)
        }
        NbtSummaryValue::IntArray(values) => CanonicalComponentCodecValue::IntArray(values.clone()),
        NbtSummaryValue::LongArray(values) => {
            CanonicalComponentCodecValue::LongArray(values.clone())
        }
    })
}

fn canonical_nbt_map(
    entries: &[NbtSummaryEntry],
) -> Option<Vec<(String, CanonicalComponentCodecValue)>> {
    let mut normalized =
        Vec::<(String, CanonicalComponentCodecValue)>::with_capacity(entries.len());
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

pub(super) fn java_uuid_int_array(value: &str) -> Option<Vec<i32>> {
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

fn normalized_identifier_field(
    entries: &[NbtSummaryEntry],
    field: &str,
) -> Option<CanonicalComponentCodecValue> {
    let value = string_value(find_entry(entries, field)?)?;
    normalize_resource_location(value.to_string())
        .ok()
        .map(CanonicalComponentCodecValue::String)
}

fn string_field(entries: &[NbtSummaryEntry], field: &str) -> Option<CanonicalComponentCodecValue> {
    string_value(find_entry(entries, field)?)
        .map(|value| CanonicalComponentCodecValue::String(value.to_string()))
}

fn string_value(value: &NbtSummaryValue) -> Option<&str> {
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
                    CanonicalComponentCodecValue::String("key".to_string()),
                ),
                (
                    "with",
                    CanonicalComponentCodecValue::List(vec![
                        CanonicalComponentCodecValue::Byte(-1),
                        CanonicalComponentCodecValue::Short(-2),
                        CanonicalComponentCodecValue::Int(-3),
                        CanonicalComponentCodecValue::Long(-4),
                        CanonicalComponentCodecValue::Float(1.5f32.to_bits()),
                        CanonicalComponentCodecValue::Double(2.5f64.to_bits()),
                    ]),
                ),
                (
                    "click_event",
                    CanonicalComponentCodecValue::Object(vec![
                        (
                            "action",
                            CanonicalComponentCodecValue::String("custom".to_string()),
                        ),
                        (
                            "id",
                            CanonicalComponentCodecValue::String("minecraft:test".to_string()),
                        ),
                        (
                            "payload",
                            CanonicalComponentCodecValue::Map(vec![
                                (
                                    "bytes".to_string(),
                                    CanonicalComponentCodecValue::ByteArray(vec![-1, 2]),
                                ),
                                (
                                    "ints".to_string(),
                                    CanonicalComponentCodecValue::IntArray(vec![-3, 4]),
                                ),
                                (
                                    "longs".to_string(),
                                    CanonicalComponentCodecValue::LongArray(vec![-5, 6]),
                                ),
                            ]),
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
                        CanonicalComponentCodecValue::String("show_entity".to_string()),
                    ),
                    (
                        "id",
                        CanonicalComponentCodecValue::String("minecraft:pig".to_string()),
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
                    player,
                ),
            )])
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
