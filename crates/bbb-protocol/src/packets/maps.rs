use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::decode_optional_component_summary_from_decoder;

const MAP_SIZE: usize = 128;
const MAX_MAP_DECORATIONS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapItemData {
    pub map_id: i32,
    pub scale: i8,
    pub locked: bool,
    pub decorations: Option<Vec<MapDecoration>>,
    pub color_patch: Option<MapColorPatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDecoration {
    pub type_id: i32,
    pub x: i8,
    pub y: i8,
    pub rot: u8,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapColorPatch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    pub colors: Vec<u8>,
}

pub(crate) fn decode_map_item_data(decoder: &mut Decoder<'_>) -> Result<MapItemData> {
    let packet = MapItemData {
        map_id: decoder.read_var_i32()?,
        scale: decoder.read_i8()?,
        locked: decoder.read_bool()?,
        decorations: decode_optional_decorations(decoder)?,
        color_patch: decode_map_color_patch(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after map item data packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_optional_decorations(decoder: &mut Decoder<'_>) -> Result<Option<Vec<MapDecoration>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    let count = decoder.read_len()?;
    if count > MAX_MAP_DECORATIONS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_MAP_DECORATIONS));
    }
    let mut decorations = Vec::with_capacity(count);
    for _ in 0..count {
        decorations.push(decode_map_decoration(decoder)?);
    }
    Ok(Some(decorations))
}

fn decode_map_decoration(decoder: &mut Decoder<'_>) -> Result<MapDecoration> {
    Ok(MapDecoration {
        type_id: decoder.read_var_i32()?,
        x: decoder.read_i8()?,
        y: decoder.read_i8()?,
        rot: decoder.read_u8()? & 15,
        name: decode_optional_component_summary_from_decoder(decoder)?,
    })
}

fn decode_map_color_patch(decoder: &mut Decoder<'_>) -> Result<Option<MapColorPatch>> {
    let width = decoder.read_u8()?;
    if width == 0 {
        return Ok(None);
    }

    let height = decoder.read_u8()?;
    let start_x = decoder.read_u8()?;
    let start_y = decoder.read_u8()?;
    let expected_len = usize::from(width) * usize::from(height);
    if usize::from(start_x) + usize::from(width) > MAP_SIZE
        || usize::from(start_y) + usize::from(height) > MAP_SIZE
    {
        return Err(ProtocolError::InvalidData(format!(
            "map color patch {width}x{height} at {start_x},{start_y} exceeds 128x128 map bounds"
        )));
    }

    let color_len = decoder.read_len()?;
    if color_len != expected_len {
        return Err(ProtocolError::InvalidData(format!(
            "map color patch has {color_len} colors, expected {expected_len}"
        )));
    }
    let colors = decoder
        .read_exact(color_len, "map color patch bytes")?
        .to_vec();

    Ok(Some(MapColorPatch {
        start_x,
        start_y,
        width,
        height,
        colors,
    }))
}

#[cfg(test)]
mod tests;
