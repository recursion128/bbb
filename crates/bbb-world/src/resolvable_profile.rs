use std::borrow::Cow;

use bbb_protocol::packets::inventory::{
    decode_profile_textures_from_properties, GameProfilePropertySummary, PlayerModelTypeSummary,
    PlayerSkinPatchSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    ResourceTextureSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::registries::normalize_codec_identifier;

pub(crate) fn decode_profile(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<ResolvableProfileSummary, String> {
    if let NbtSummaryValue::String(name) = value {
        validate_player_name(name, what)?;
        return Ok(ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some(name.clone()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        });
    }

    let entries = compound_entries(value, what)?;
    let uuid = find_entry(entries, "id")
        .map(|value| decode_uuid(value, &format!("{what} field id")))
        .transpose()?;
    let name = find_entry(entries, "name")
        .map(|value| decode_player_name(value, &format!("{what} field name")))
        .transpose()?;
    let properties = find_entry(entries, "properties")
        .map(|value| decode_property_map(value, &format!("{what} field properties")))
        .transpose()?
        .unwrap_or_default();
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    let skin_patch = decode_skin_patch(entries, what)?;
    let kind = if uuid.is_some() && name.is_some() {
        ResolvableProfileKindSummary::GameProfile
    } else {
        ResolvableProfileKindSummary::Partial
    };
    Ok(ResolvableProfileSummary {
        kind,
        uuid,
        name,
        properties,
        profile_textures,
        skin_patch,
    })
}

fn decode_uuid(value: &NbtSummaryValue, what: &str) -> Result<Uuid, String> {
    let values = component_list(value).map_err(|_| format!("{what} is not an int stream"))?;
    if values.len() != 4 {
        return Err(format!("{what} has {} integers instead of 4", values.len()));
    }
    let mut decoded = [0i32; 4];
    for (index, value) in values.iter().enumerate() {
        decoded[index] = component_i32(value, &format!("{what}[{index}]"))?;
    }
    let raw = (u128::from(decoded[0] as u32) << 96)
        | (u128::from(decoded[1] as u32) << 64)
        | (u128::from(decoded[2] as u32) << 32)
        | u128::from(decoded[3] as u32);
    Ok(Uuid::from_u128(raw))
}

fn decode_player_name(value: &NbtSummaryValue, what: &str) -> Result<String, String> {
    let NbtSummaryValue::String(name) = value else {
        return Err(format!("{what} is not a string"));
    };
    validate_player_name(name, what)?;
    Ok(name.clone())
}

fn validate_player_name(name: &str, what: &str) -> Result<(), String> {
    let utf16 = name.encode_utf16().collect::<Vec<_>>();
    if utf16.len() > 16 {
        return Err(format!(
            "{what} has {} UTF-16 units, maximum is 16",
            utf16.len()
        ));
    }
    if utf16.iter().any(|unit| *unit <= 32 || *unit >= 127) {
        return Err(format!(
            "{what} contains a disallowed player-name character"
        ));
    }
    Ok(())
}

fn decode_property_map(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<GameProfilePropertySummary>, String> {
    match value {
        NbtSummaryValue::Compound(entries) => decode_compound_property_map(entries, what),
        _ => decode_property_list(value, what),
    }
}

fn decode_compound_property_map(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<Vec<GameProfilePropertySummary>, String> {
    if entries.len() > 16 {
        return Err(format!(
            "{what} has {} property names, maximum is 16",
            entries.len()
        ));
    }
    let mut properties = Vec::new();
    for entry in entries {
        let values = component_list(&entry.value)
            .map_err(|_| format!("{what} field {} is not a list", entry.name))?;
        for (index, value) in values.iter().enumerate() {
            let NbtSummaryValue::String(value) = value else {
                return Err(format!(
                    "{what} field {}[{index}] is not a string",
                    entry.name
                ));
            };
            properties.push(GameProfilePropertySummary {
                name: entry.name.clone(),
                value: value.clone(),
                signature: None,
            });
        }
    }
    Ok(properties)
}

fn decode_property_list(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<GameProfilePropertySummary>, String> {
    let values =
        component_list(value).map_err(|_| format!("{what} is neither a map nor a list"))?;
    if values.len() > 16 {
        return Err(format!(
            "{what} has {} property entries, maximum is 16",
            values.len()
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_property(value, &format!("{what}[{index}]")))
        .collect()
}

fn decode_property(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<GameProfilePropertySummary, String> {
    let entries = compound_entries(value, what)?;
    let name = required_bounded_string(entries, "name", 64, what)?;
    let value = required_bounded_string(entries, "value", 32_767, what)?;
    let signature = find_entry(entries, "signature")
        .map(|value| bounded_string(value, 1_024, &format!("{what} field signature")))
        .transpose()?;
    Ok(GameProfilePropertySummary {
        name,
        value,
        signature,
    })
}

fn required_bounded_string(
    entries: &[NbtSummaryEntry],
    key: &str,
    max_utf16: usize,
    what: &str,
) -> Result<String, String> {
    let value = find_entry(entries, key).ok_or_else(|| format!("{what} is missing field {key}"))?;
    bounded_string(value, max_utf16, &format!("{what} field {key}"))
}

fn bounded_string(value: &NbtSummaryValue, max_utf16: usize, what: &str) -> Result<String, String> {
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} is not a string"));
    };
    let len = value.encode_utf16().count();
    if len > max_utf16 {
        return Err(format!(
            "{what} has {len} UTF-16 units, maximum is {max_utf16}"
        ));
    }
    Ok(value.clone())
}

fn decode_skin_patch(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<PlayerSkinPatchSummary, String> {
    Ok(PlayerSkinPatchSummary {
        body: decode_resource_texture(entries, "texture", what)?,
        cape: decode_resource_texture(entries, "cape", what)?,
        elytra: decode_resource_texture(entries, "elytra", what)?,
        model: match find_entry(entries, "model") {
            None => None,
            Some(NbtSummaryValue::String(name)) => Some(match name.as_str() {
                "slim" => PlayerModelTypeSummary::Slim,
                "wide" => PlayerModelTypeSummary::Wide,
                _ => return Err(format!("unknown {what} model {name}")),
            }),
            Some(_) => return Err(format!("{what} field model is not an enum name")),
        },
    })
}

fn decode_resource_texture(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<ResourceTextureSummary>, String> {
    let Some(value) = find_entry(entries, key) else {
        return Ok(None);
    };
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} field {key} is not a resource id"));
    };
    let asset_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} field {key} resource id {raw}"))?;
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("normalized resource ids include a namespace");
    Ok(Some(ResourceTextureSummary {
        texture_path: format!("{namespace}:textures/{path}.png"),
        asset_id,
    }))
}

fn compound_entries<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
) -> Result<&'a [NbtSummaryEntry], String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(entries)
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn component_list(value: &NbtSummaryValue) -> Result<Cow<'_, [NbtSummaryValue]>, ()> {
    match value {
        NbtSummaryValue::List(values) => Ok(Cow::Borrowed(values)),
        NbtSummaryValue::ByteArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Byte).collect(),
        )),
        NbtSummaryValue::IntArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Int).collect(),
        )),
        NbtSummaryValue::LongArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Long).collect(),
        )),
        _ => Err(()),
    }
}

fn component_i32(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Int(value) => Ok(*value),
        NbtSummaryValue::Long(value) => Ok(*value as i32),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i32),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i32),
        _ => Err(format!("{what} is not numeric")),
    }
}
