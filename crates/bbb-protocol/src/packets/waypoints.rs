use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec::{Decoder, ProtocolError, Result};
use crate::waypoint_wire_types::WaypointDataType;
pub use crate::waypoint_wire_types::WaypointOperation;

use super::{read_resource_location, ChunkPos};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointVec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
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
