use serde::{Deserialize, Serialize};

use crate::{
    codec::{Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, BlockPos, Vec3i};

pub const MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS: usize = 128;
pub const MAX_STRUCTURE_BLOCK_OFFSET: i32 = 48;
pub const MAX_STRUCTURE_BLOCK_SIZE: i32 = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureBlockUpdateKind {
    UpdateData,
    SaveArea,
    LoadArea,
    ScanArea,
}

impl StructureBlockUpdateKind {
    fn wire_id(self) -> i32 {
        match self {
            Self::UpdateData => 0,
            Self::SaveArea => 1,
            Self::LoadArea => 2,
            Self::ScanArea => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
}

impl StructureBlockMode {
    fn wire_id(self) -> i32 {
        match self {
            Self::Save => 0,
            Self::Load => 1,
            Self::Corner => 2,
            Self::Data => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureBlockMirror {
    None,
    LeftRight,
    FrontBack,
}

impl StructureBlockMirror {
    fn wire_id(self) -> i32 {
        match self {
            Self::None => 0,
            Self::LeftRight => 1,
            Self::FrontBack => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureBlockRotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

impl StructureBlockRotation {
    fn wire_id(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 1,
            Self::Clockwise180 => 2,
            Self::Counterclockwise90 => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetStructureBlock {
    pub pos: BlockPos,
    pub update: StructureBlockUpdateKind,
    pub mode: StructureBlockMode,
    pub name: String,
    pub offset: BlockPos,
    pub size: Vec3i,
    pub mirror: StructureBlockMirror,
    pub rotation: StructureBlockRotation,
    pub data: String,
    pub ignore_entities: bool,
    pub strict: bool,
    pub show_air: bool,
    pub show_bounding_box: bool,
    pub integrity: f32,
    pub seed: i64,
}

pub fn encode_play_set_structure_block(packet: &SetStructureBlock) -> Result<(i32, Vec<u8>)> {
    validate_text(&packet.name)?;
    validate_text(&packet.data)?;

    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_var_i32(packet.update.wire_id());
    out.write_var_i32(packet.mode.wire_id());
    out.write_string(&packet.name);
    for value in [packet.offset.x, packet.offset.y, packet.offset.z] {
        // FriendlyByteBuf.writeByte narrows Java's int to its low eight bits.
        // The production editor clamps before constructing this typed packet.
        out.write_i8(value as i8);
    }
    for value in [packet.size.x, packet.size.y, packet.size.z] {
        out.write_i8(value as i8);
    }
    out.write_var_i32(packet.mirror.wire_id());
    out.write_var_i32(packet.rotation.wire_id());
    out.write_string(&packet.data);
    out.write_f32(packet.integrity);
    out.write_var_i64(packet.seed);
    let flags = u8::from(packet.ignore_entities)
        | (u8::from(packet.show_air) << 1)
        | (u8::from(packet.show_bounding_box) << 2)
        | (u8::from(packet.strict) << 3);
    out.write_u8(flags);
    Ok((ids::play::SERVERBOUND_SET_STRUCTURE_BLOCK, out.into_inner()))
}

fn validate_text(value: &str) -> Result<()> {
    let len = value.encode_utf16().count();
    if len > MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS {
        return Err(ProtocolError::StringTooLong {
            actual: len,
            max: MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Decoder;

    fn packet() -> SetStructureBlock {
        SetStructureBlock {
            pos: BlockPos {
                x: -12,
                y: 70,
                z: 31,
            },
            update: StructureBlockUpdateKind::LoadArea,
            mode: StructureBlockMode::Load,
            name: "minecraft:village/house".to_string(),
            offset: BlockPos {
                x: -48,
                y: 2,
                z: 48,
            },
            size: Vec3i { x: 0, y: 12, z: 48 },
            mirror: StructureBlockMirror::FrontBack,
            rotation: StructureBlockRotation::Counterclockwise90,
            data: "metadata".to_string(),
            ignore_entities: true,
            strict: true,
            show_air: true,
            show_bounding_box: true,
            integrity: 1.0,
            seed: i64::MIN,
        }
    }

    #[test]
    fn encodes_set_structure_block_wire_order_bounds_and_flags() {
        let (id, payload) = encode_play_set_structure_block(&packet()).unwrap();
        assert_eq!(id, ids::play::SERVERBOUND_SET_STRUCTURE_BLOCK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decoder.read_i64().unwrap(),
            chunks::encode_block_pos(BlockPos {
                x: -12,
                y: 70,
                z: 31
            })
        );
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_string(128).unwrap(), "minecraft:village/house");
        assert_eq!(decoder.read_i8().unwrap(), -48);
        assert_eq!(decoder.read_i8().unwrap(), 2);
        assert_eq!(decoder.read_i8().unwrap(), 48);
        assert_eq!(decoder.read_i8().unwrap(), 0);
        assert_eq!(decoder.read_i8().unwrap(), 12);
        assert_eq!(decoder.read_i8().unwrap(), 48);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 3);
        assert_eq!(decoder.read_string(128).unwrap(), "metadata");
        assert_eq!(decoder.read_f32().unwrap(), 1.0);
        assert_eq!(decoder.read_var_i64().unwrap(), i64::MIN);
        assert_eq!(decoder.read_u8().unwrap(), 0b1111);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_every_structure_enum_ordinal_and_flag_combination() {
        let updates = [
            StructureBlockUpdateKind::UpdateData,
            StructureBlockUpdateKind::SaveArea,
            StructureBlockUpdateKind::LoadArea,
            StructureBlockUpdateKind::ScanArea,
        ];
        let modes = [
            StructureBlockMode::Save,
            StructureBlockMode::Load,
            StructureBlockMode::Corner,
            StructureBlockMode::Data,
        ];
        let mirrors = [
            StructureBlockMirror::None,
            StructureBlockMirror::LeftRight,
            StructureBlockMirror::FrontBack,
        ];
        let rotations = [
            StructureBlockRotation::None,
            StructureBlockRotation::Clockwise90,
            StructureBlockRotation::Clockwise180,
            StructureBlockRotation::Counterclockwise90,
        ];

        for (expected, value) in updates.into_iter().enumerate() {
            assert_eq!(value.wire_id(), expected as i32);
        }
        for (expected, value) in modes.into_iter().enumerate() {
            assert_eq!(value.wire_id(), expected as i32);
        }
        for (expected, value) in mirrors.into_iter().enumerate() {
            assert_eq!(value.wire_id(), expected as i32);
        }
        for (expected, value) in rotations.into_iter().enumerate() {
            assert_eq!(value.wire_id(), expected as i32);
        }
        for flags in 0_u8..=15 {
            let mut value = packet();
            value.ignore_entities = flags & 1 != 0;
            value.show_air = flags & 2 != 0;
            value.show_bounding_box = flags & 4 != 0;
            value.strict = flags & 8 != 0;
            let (_, payload) = encode_play_set_structure_block(&value).unwrap();
            assert_eq!(payload.last().copied(), Some(flags));
        }
    }

    #[test]
    fn encoder_preserves_java_write_byte_and_float_semantics() {
        let mut value = packet();
        value.offset = BlockPos {
            x: 200,
            y: -129,
            z: 256,
        };
        value.size = Vec3i {
            x: 255,
            y: 256,
            z: 257,
        };
        value.integrity = 1.25;
        let (_, payload) = encode_play_set_structure_block(&value).unwrap();
        let mut decoder = Decoder::new(&payload);
        decoder.read_i64().unwrap();
        decoder.read_var_i32().unwrap();
        decoder.read_var_i32().unwrap();
        decoder.read_string(128).unwrap();
        assert_eq!(decoder.read_i8().unwrap(), -56);
        assert_eq!(decoder.read_i8().unwrap(), 127);
        assert_eq!(decoder.read_i8().unwrap(), 0);
        assert_eq!(decoder.read_i8().unwrap(), -1);
        assert_eq!(decoder.read_i8().unwrap(), 0);
        assert_eq!(decoder.read_i8().unwrap(), 1);
        decoder.read_var_i32().unwrap();
        decoder.read_var_i32().unwrap();
        decoder.read_string(128).unwrap();
        assert_eq!(decoder.read_f32().unwrap(), 1.25);
    }

    #[test]
    fn set_structure_block_enforces_editor_utf16_boundaries() {
        let mut value = packet();
        value.name = "\u{1f642}".repeat(MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS / 2);
        value.data = "x".repeat(MAX_STRUCTURE_BLOCK_TEXT_UTF16_UNITS);
        assert!(encode_play_set_structure_block(&value).is_ok());

        value.name.push('x');
        assert!(matches!(
            encode_play_set_structure_block(&value),
            Err(ProtocolError::StringTooLong {
                actual: 129,
                max: 128
            })
        ));
        value.name.clear();
        value.data.push('x');
        assert!(matches!(
            encode_play_set_structure_block(&value),
            Err(ProtocolError::StringTooLong {
                actual: 129,
                max: 128
            })
        ));
    }
}
