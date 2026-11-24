use std::borrow::Cow;

use uuid::Uuid;

use crate::codec::{Decoder, ProtocolError, Result};

use super::movement::Vec3d;

pub(crate) const MAX_SERVER_ICON_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn decode_nullable_string(
    decoder: &mut Decoder<'_>,
) -> Result<Option<crate::JavaString>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_java_string(32767)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn decode_optional_uuid(decoder: &mut Decoder<'_>) -> Result<Option<Uuid>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_uuid()?))
    } else {
        Ok(None)
    }
}

pub(super) fn decode_optional_rich_component_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<Option<crate::RichTextComponent>> {
    if decoder.read_bool()? {
        Ok(Some(
            crate::component::decode_rich_text_component_from_decoder_with_registry(
                decoder, registry,
            )?,
        ))
    } else {
        Ok(None)
    }
}

pub(crate) fn decode_optional_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Option<Vec<u8>>> {
    if decoder.read_bool()? {
        Ok(Some(decode_byte_array(decoder, max_len, what)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn decode_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Vec<u8>> {
    let len = decoder.read_len()?;
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }
    Ok(decoder.read_exact(len, what)?.to_vec())
}

pub(crate) fn read_resource_location(decoder: &mut Decoder<'_>) -> Result<String> {
    normalize_resource_location(decoder.read_string(32767)?)
}

pub fn normalize_resource_location<'a>(raw: impl Into<Cow<'a, str>>) -> Result<String> {
    let raw = raw.into();
    let (namespace, path) = resource_location_parts(&raw)?;
    if namespace.is_some() {
        return Ok(raw.into_owned());
    }
    let mut normalized = String::with_capacity("minecraft:".len() + path.len());
    normalized.push_str("minecraft:");
    normalized.push_str(path);
    Ok(normalized)
}

pub(crate) fn is_canonical_resource_location(raw: &str) -> Result<bool> {
    resource_location_parts(raw).map(|(namespace, _)| namespace.is_some())
}

fn resource_location_parts(raw: &str) -> Result<(Option<&str>, &str)> {
    let (namespace, path) = match raw.find(':') {
        Some(0) => (None, &raw[1..]),
        Some(separator) => (Some(&raw[..separator]), &raw[separator + 1..]),
        None => (None, raw),
    };
    if namespace
        .is_some_and(|namespace| namespace == ".." || !is_valid_resource_chars::<false>(namespace))
    {
        return Err(ProtocolError::InvalidData(format!(
            "invalid resource location `{raw}`: invalid namespace"
        )));
    }
    if !is_valid_resource_path(path) {
        return Err(ProtocolError::InvalidData(format!(
            "invalid resource location `{raw}`: invalid path"
        )));
    }
    Ok((namespace, path))
}

fn is_valid_resource_chars<const ALLOW_SLASH: bool>(value: &str) -> bool {
    value.bytes().all(|ch| {
        matches!(ch, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.') || (ALLOW_SLASH && ch == b'/')
    })
}

pub fn is_valid_resource_path(path: &str) -> bool {
    is_valid_resource_chars::<true>(path)
}

pub(crate) fn decode_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    })
}

pub(crate) fn decode_optional_vec3d(decoder: &mut Decoder<'_>) -> Result<Option<Vec3d>> {
    if decoder.read_bool()? {
        Ok(Some(decode_vec3d(decoder)?))
    } else {
        Ok(None)
    }
}

pub(crate) fn pack_move_flags(on_ground: bool, horizontal_collision: bool) -> u8 {
    let mut flags = 0;
    if on_ground {
        flags |= 1;
    }
    if horizontal_collision {
        flags |= 2;
    }
    flags
}
