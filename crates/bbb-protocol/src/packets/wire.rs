use uuid::Uuid;

use crate::codec::{Decoder, ProtocolError, Result};

use super::movement::Vec3d;

pub(crate) use crate::component::decode_component_summary_from_decoder;

pub(crate) const MAX_SERVER_ICON_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn decode_nullable_string(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_string(32767)?))
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

pub(crate) fn decode_optional_component_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decode_component_summary_from_decoder(decoder)?))
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

pub(crate) fn decode_optional_trailing_number_format(
    decoder: &mut Decoder<'_>,
) -> Result<Option<Vec<u8>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    let len = decoder.remaining_len();
    Ok(Some(
        decoder.read_exact(len, "number format payload")?.to_vec(),
    ))
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

fn normalize_resource_location(raw: String) -> Result<String> {
    let (namespace, path) = match raw.find(':') {
        Some(0) => ("minecraft", &raw[1..]),
        Some(separator) => (&raw[..separator], &raw[separator + 1..]),
        None => ("minecraft", raw.as_str()),
    };

    if !is_valid_resource_namespace(namespace) {
        return Err(ProtocolError::InvalidData(format!(
            "invalid resource location `{raw}`: invalid namespace"
        )));
    }
    if !is_valid_resource_path(path) {
        return Err(ProtocolError::InvalidData(format!(
            "invalid resource location `{raw}`: invalid path"
        )));
    }

    Ok(format!("{namespace}:{path}"))
}

fn is_valid_resource_namespace(namespace: &str) -> bool {
    namespace != ".."
        && namespace.chars().all(|ch| {
            ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_lowercase() || ch.is_ascii_digit()
        })
}

fn is_valid_resource_path(path: &str) -> bool {
    path.chars().all(|ch| {
        ch == '_'
            || ch == '-'
            || ch == '/'
            || ch == '.'
            || ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
    })
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
