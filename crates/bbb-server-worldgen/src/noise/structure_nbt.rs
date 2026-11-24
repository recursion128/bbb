//! NBT helpers shared by the saved structure-start codecs.
//!
//! Reads return `None` for a missing or mistyped tag; the caller skips that
//! start, as `StructureStart.loadStaticStart` does (`StructureStart.java:39-67`).

use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use super::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use bbb_protocol::packets::BlockPos;

pub trait NbtRead {
    fn compound(&self) -> Option<&[NbtSummaryEntry]>;
    fn list(&self) -> Option<&[NbtSummaryValue]>;
    fn int(&self) -> Option<i32>;
    fn byte(&self) -> Option<i8>;
    fn float(&self) -> Option<f32>;
    fn string(&self) -> Option<String>;
    fn int_array(&self) -> Option<&[i32]>;

    fn boolean(&self) -> Option<bool> {
        self.byte().map(|value| value != 0)
    }

    fn resource_id(&self) -> Option<String> {
        normalize_resource_location(self.string()?).ok()
    }
}

impl NbtRead for NbtSummaryValue {
    fn compound(&self) -> Option<&[NbtSummaryEntry]> {
        match self {
            Self::Compound(fields) => Some(fields),
            _ => None,
        }
    }

    fn list(&self) -> Option<&[NbtSummaryValue]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

    fn int(&self) -> Option<i32> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    fn byte(&self) -> Option<i8> {
        match self {
            Self::Byte(value) => Some(*value),
            _ => None,
        }
    }

    fn float(&self) -> Option<f32> {
        match self {
            Self::Float(bits) => Some(f32::from_bits(*bits)),
            _ => None,
        }
    }

    fn string(&self) -> Option<String> {
        match self {
            Self::String(value) => value.try_to_string().ok(),
            _ => None,
        }
    }

    fn int_array(&self) -> Option<&[i32]> {
        match self {
            Self::IntArray(values) => Some(values),
            _ => None,
        }
    }
}

pub trait NbtFields {
    fn field(&self, name: &str) -> Option<&NbtSummaryValue>;
}

impl NbtFields for [NbtSummaryEntry] {
    fn field(&self, name: &str) -> Option<&NbtSummaryValue> {
        self.iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    }
}

pub fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

pub fn string(name: &str, value: &str) -> NbtSummaryEntry {
    entry(name, NbtSummaryValue::string(value))
}

pub fn int(name: &str, value: i32) -> NbtSummaryEntry {
    entry(name, NbtSummaryValue::Int(value))
}

pub fn byte(name: &str, value: i8) -> NbtSummaryEntry {
    entry(name, NbtSummaryValue::Byte(value))
}

pub fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

pub fn bounding_box_to_nbt(bounds: JigsawBoundingBox) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(vec![
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ])
}

pub fn start_to_nbt(
    structure_id: &str,
    start_chunk: [i32; 2],
    references: i32,
    children: Vec<NbtSummaryValue>,
) -> NbtSummaryValue {
    compound(vec![
        string("id", structure_id),
        int("ChunkX", start_chunk[0]),
        int("ChunkZ", start_chunk[1]),
        int("references", references),
        entry("Children", NbtSummaryValue::List(children)),
    ])
}

/// `BoundingBox.CODEC` int array `[minX, minY, minZ, maxX, maxY, maxZ]`.
pub fn bounding_box(value: &NbtSummaryValue) -> Option<JigsawBoundingBox> {
    let [min_x, min_y, min_z, max_x, max_y, max_z] = *value.int_array()? else {
        return None;
    };
    Some(JigsawBoundingBox {
        min: BlockPos {
            x: min_x,
            y: min_y,
            z: min_z,
        },
        max: BlockPos {
            x: max_x,
            y: max_y,
            z: max_z,
        },
    })
}

pub const fn rotation_name(rotation: JigsawRotation) -> &'static str {
    match rotation {
        JigsawRotation::None => "NONE",
        JigsawRotation::Clockwise90 => "CLOCKWISE_90",
        JigsawRotation::Clockwise180 => "CLOCKWISE_180",
        JigsawRotation::CounterClockwise90 => "COUNTERCLOCKWISE_90",
    }
}

pub fn rotation(value: &NbtSummaryValue) -> Option<JigsawRotation> {
    Some(match value.string()?.as_str() {
        "NONE" => JigsawRotation::None,
        "CLOCKWISE_90" => JigsawRotation::Clockwise90,
        "CLOCKWISE_180" => JigsawRotation::Clockwise180,
        "COUNTERCLOCKWISE_90" => JigsawRotation::CounterClockwise90,
        _ => return None,
    })
}
