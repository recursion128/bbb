use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::decode_rich_text_component_from_decoder_with_registry,
    content_registry::ContentRegistry,
    nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH},
    RichTextComponent,
};

const MAP_SIZE: usize = 128;
const MAX_MAP_DECORATIONS: usize = 4096;
const MAX_MAP_DECORATION_NAME_NBT_BYTES: usize =
    crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;

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
    pub name: Option<RichTextComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapColorPatch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    pub colors: Vec<u8>,
}

/// `ClientboundMapItemDataPacket.STREAM_CODEC`
/// (`ClientboundMapItemDataPacket.java:16-30`).
pub fn encode_play_clientbound_map_item_data(update: &MapItemData) -> Result<(i32, Vec<u8>)> {
    let decorations_len = update.decorations.as_ref().map_or(0, Vec::len);
    if decorations_len > MAX_MAP_DECORATIONS {
        return Err(ProtocolError::PacketTooLarge(
            decorations_len,
            MAX_MAP_DECORATIONS,
        ));
    }
    let decorations_len = i32::try_from(decorations_len).map_err(|_| {
        ProtocolError::InvalidData("map decoration count does not fit a VarInt".to_string())
    })?;
    if let Some(patch) = &update.color_patch {
        validate_map_color_patch(patch)?;
    }

    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.map_id);
    encoder.write_i8(update.scale);
    encoder.write_bool(update.locked);
    match &update.decorations {
        Some(decorations) => {
            encoder.write_bool(true);
            encoder.write_var_i32(decorations_len);
            for decoration in decorations {
                encoder.write_var_i32(decoration.type_id);
                encoder.write_i8(decoration.x);
                encoder.write_i8(decoration.y);
                encoder.write_u8(decoration.rot & 15);
                match &decoration.name {
                    Some(name) => {
                        encoder.write_bool(true);
                        encoder.write_bytes(&encode_nbt_summary_bounded(
                            Some(name.component()),
                            MAX_MAP_DECORATION_NAME_NBT_BYTES,
                            MAX_NBT_DEPTH,
                        )?);
                    }
                    None => encoder.write_bool(false),
                }
            }
        }
        None => encoder.write_bool(false),
    }
    match &update.color_patch {
        Some(patch) => {
            encoder.write_u8(patch.width);
            encoder.write_u8(patch.height);
            encoder.write_u8(patch.start_x);
            encoder.write_u8(patch.start_y);
            encoder.write_var_i32(i32::try_from(patch.colors.len()).map_err(|_| {
                ProtocolError::InvalidData(
                    "map color patch length does not fit a VarInt".to_string(),
                )
            })?);
            encoder.write_bytes(&patch.colors);
        }
        None => encoder.write_u8(0),
    }
    Ok((
        crate::ids::play::CLIENTBOUND_MAP_ITEM_DATA,
        encoder.into_inner(),
    ))
}

fn validate_map_color_patch(patch: &MapColorPatch) -> Result<()> {
    if patch.width == 0 {
        return Err(ProtocolError::InvalidData(
            "present map color patch has zero width".to_string(),
        ));
    }
    let expected_len = usize::from(patch.width) * usize::from(patch.height);
    if usize::from(patch.start_x) + usize::from(patch.width) > MAP_SIZE
        || usize::from(patch.start_y) + usize::from(patch.height) > MAP_SIZE
    {
        return Err(ProtocolError::InvalidData(format!(
            "map color patch {}x{} at {},{} exceeds 128x128 map bounds",
            patch.width, patch.height, patch.start_x, patch.start_y
        )));
    }
    if patch.colors.len() != expected_len {
        return Err(ProtocolError::InvalidData(format!(
            "map color patch has {} colors, expected {expected_len}",
            patch.colors.len()
        )));
    }
    Ok(())
}

pub(crate) fn decode_map_item_data_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<MapItemData> {
    let packet = MapItemData {
        map_id: decoder.read_var_i32()?,
        scale: decoder.read_i8()?,
        locked: decoder.read_bool()?,
        decorations: decode_optional_decorations(decoder, registry)?,
        color_patch: decode_map_color_patch(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after map item data packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_optional_decorations(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Option<Vec<MapDecoration>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    let count = decoder.read_len()?;
    if count > MAX_MAP_DECORATIONS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_MAP_DECORATIONS));
    }
    let mut decorations = Vec::with_capacity(decoder.bounded_capacity(count, 5));
    for _ in 0..count {
        decorations.push(decode_map_decoration(decoder, registry)?);
    }
    Ok(Some(decorations))
}

fn decode_map_decoration(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<MapDecoration> {
    Ok(MapDecoration {
        type_id: decoder.read_var_i32()?,
        x: decoder.read_i8()?,
        y: decoder.read_i8()?,
        rot: decoder.read_u8()? & 15,
        name: decoder
            .read_bool()?
            .then(|| decode_rich_text_component_from_decoder_with_registry(decoder, registry))
            .transpose()?,
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
