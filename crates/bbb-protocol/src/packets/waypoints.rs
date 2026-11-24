use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::ids;
use bbb_facts::waypoint_wire_types::WaypointDataType;
pub use bbb_facts::waypoint_wire_types::WaypointOperation;

pub use super::Vec3i as WaypointVec3i;
use super::{normalize_resource_location, read_resource_location, ChunkPos};

const MAX_WAYPOINT_IDENTIFIER_NAME_CHARS: usize = 32_767;
const MAX_WAYPOINT_ICON_STYLE_CHARS: usize = 32_767;
const MAX_WAYPOINT_ICON_COLOR_RGB: u32 = 0x00ff_ffff;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedWaypointPacket {
    pub operation: WaypointOperation,
    pub waypoint: TrackedWaypoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedWaypoint {
    pub identifier: WaypointIdentifier,
    pub icon: WaypointIcon,
    pub data: WaypointData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaypointIdentifier {
    Uuid(Uuid),
    Name(String),
}

impl WaypointIdentifier {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Uuid(_) => "uuid",
            Self::Name(_) => "name",
        }
    }

    pub fn value_string(&self) -> String {
        match self {
            Self::Uuid(id) => id.to_string(),
            Self::Name(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointIcon {
    pub style: String,
    pub color_rgb: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WaypointData {
    Empty,
    Position(WaypointVec3i),
    Chunk(ChunkPos),
    Azimuth(f32),
}

impl WaypointData {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Position(_) => "position",
            Self::Chunk(_) => "chunk",
            Self::Azimuth(_) => "azimuth",
        }
    }
}

/// `ClientboundTrackedWaypointPacket.STREAM_CODEC` writes the operation before the waypoint
/// (`ClientboundTrackedWaypointPacket.java:20-28`,
/// `ClientboundTrackedWaypointPacket.java:71-80`); the waypoint wire order and variants are defined
/// by `TrackedWaypoint.java:39-54`, `TrackedWaypoint.java:85-110`,
/// `TrackedWaypoint.java:141-167`, `TrackedWaypoint.java:199-214`, and
/// `TrackedWaypoint.java:246-285`.
pub fn encode_play_clientbound_tracked_waypoint(
    packet: &TrackedWaypointPacket,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(packet.operation.ordinal());
    encode_tracked_waypoint(&mut encoder, &packet.waypoint)?;
    Ok((ids::play::CLIENTBOUND_WAYPOINT, encoder.into_inner()))
}

fn encode_tracked_waypoint(encoder: &mut Encoder, waypoint: &TrackedWaypoint) -> Result<()> {
    match &waypoint.identifier {
        WaypointIdentifier::Uuid(identifier) => {
            encoder.write_bool(true);
            encoder.write_uuid(*identifier);
        }
        WaypointIdentifier::Name(name) => {
            let actual = name.chars().count();
            if actual > MAX_WAYPOINT_IDENTIFIER_NAME_CHARS {
                return Err(ProtocolError::StringTooLong {
                    actual,
                    max: MAX_WAYPOINT_IDENTIFIER_NAME_CHARS,
                });
            }
            encoder.write_bool(false);
            encoder.write_string(name);
        }
    }
    encode_waypoint_icon(encoder, &waypoint.icon)?;
    match waypoint.data {
        WaypointData::Empty => encoder.write_var_i32(WaypointDataType::Empty as i32),
        WaypointData::Position(position) => {
            encoder.write_var_i32(WaypointDataType::Vec3i as i32);
            encoder.write_var_i32(position.x);
            encoder.write_var_i32(position.y);
            encoder.write_var_i32(position.z);
        }
        WaypointData::Chunk(chunk) => {
            encoder.write_var_i32(WaypointDataType::Chunk as i32);
            encoder.write_var_i32(chunk.x);
            encoder.write_var_i32(chunk.z);
        }
        WaypointData::Azimuth(angle) => {
            encoder.write_var_i32(WaypointDataType::Azimuth as i32);
            encoder.write_f32(angle);
        }
    }
    Ok(())
}

fn encode_waypoint_icon(encoder: &mut Encoder, icon: &WaypointIcon) -> Result<()> {
    let style = normalize_resource_location(icon.style.clone())?;
    let actual = style.chars().count();
    if actual > MAX_WAYPOINT_ICON_STYLE_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual,
            max: MAX_WAYPOINT_ICON_STYLE_CHARS,
        });
    }
    encoder.write_string(&style);
    match icon.color_rgb {
        Some(color) if color <= MAX_WAYPOINT_ICON_COLOR_RGB => {
            encoder.write_bool(true);
            encoder.write_u8((color >> 16) as u8);
            encoder.write_u8((color >> 8) as u8);
            encoder.write_u8(color as u8);
        }
        Some(color) => {
            return Err(ProtocolError::InvalidData(format!(
                "waypoint icon color {color:#010x} exceeds 24-bit RGB"
            )));
        }
        None => encoder.write_bool(false),
    }
    Ok(())
}

pub(crate) fn decode_tracked_waypoint_packet(
    decoder: &mut Decoder<'_>,
) -> Result<TrackedWaypointPacket> {
    let operation = WaypointOperation::from_wire_id(decoder.read_var_i32()?);
    let waypoint = decode_tracked_waypoint(decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after waypoint packet".to_string(),
        ));
    }

    Ok(TrackedWaypointPacket {
        operation,
        waypoint,
    })
}

fn decode_tracked_waypoint(decoder: &mut Decoder<'_>) -> Result<TrackedWaypoint> {
    let identifier = if decoder.read_bool()? {
        WaypointIdentifier::Uuid(decoder.read_uuid()?)
    } else {
        WaypointIdentifier::Name(decoder.read_string(32767)?)
    };
    let icon = decode_waypoint_icon(decoder)?;
    let data_type_ordinal = decoder.read_var_i32()?;
    let data_type = WaypointDataType::from_ordinal(data_type_ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid waypoint type ordinal {data_type_ordinal}"))
    })?;
    let data = match data_type {
        WaypointDataType::Empty => WaypointData::Empty,
        WaypointDataType::Vec3i => WaypointData::Position(decode_vec3i(decoder)?),
        WaypointDataType::Chunk => WaypointData::Chunk(ChunkPos {
            x: decoder.read_var_i32()?,
            z: decoder.read_var_i32()?,
        }),
        WaypointDataType::Azimuth => WaypointData::Azimuth(decoder.read_f32()?),
    };

    Ok(TrackedWaypoint {
        identifier,
        icon,
        data,
    })
}

fn decode_waypoint_icon(decoder: &mut Decoder<'_>) -> Result<WaypointIcon> {
    let style = read_resource_location(decoder)?;
    let color_rgb = if decoder.read_bool()? {
        let red = u32::from(decoder.read_u8()?);
        let green = u32::from(decoder.read_u8()?);
        let blue = u32::from(decoder.read_u8()?);
        Some((red << 16) | (green << 8) | blue)
    } else {
        None
    };

    Ok(WaypointIcon { style, color_rgb })
}

fn decode_vec3i(decoder: &mut Decoder<'_>) -> Result<WaypointVec3i> {
    Ok(WaypointVec3i {
        x: decoder.read_var_i32()?,
        y: decoder.read_var_i32()?,
        z: decoder.read_var_i32()?,
    })
}

#[cfg(test)]
mod tests;
