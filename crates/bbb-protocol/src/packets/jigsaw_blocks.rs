use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, normalize_resource_location, BlockPos};

pub const MAX_JIGSAW_RESOURCE_UTF16_UNITS: usize = 128;
pub const MAX_JIGSAW_FINAL_STATE_UTF16_UNITS: usize = 256;
const WIRE_STRING_MAX_UTF16_UNITS: usize = 32_767;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JigsawJoint {
    Rollable,
    Aligned,
}

impl JigsawJoint {
    pub fn serialized_name(self) -> &'static str {
        match self {
            Self::Rollable => "rollable",
            Self::Aligned => "aligned",
        }
    }

    /// `JigsawBlockEntity.Joint.byName` (JigsawBlockEntity.java), whose
    /// `Optional` the packet turns into `ALIGNED`.
    fn by_name(name: &str) -> Self {
        match name {
            "rollable" => Self::Rollable,
            _ => Self::Aligned,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetJigsawBlock {
    pub pos: BlockPos,
    pub name: String,
    pub target: String,
    pub pool: String,
    pub final_state: crate::JavaString,
    pub joint: JigsawJoint,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JigsawGenerate {
    pub pos: BlockPos,
    pub levels: i32,
    pub keep_jigsaws: bool,
}

pub fn encode_play_set_jigsaw_block(packet: &SetJigsawBlock) -> Result<(i32, Vec<u8>)> {
    let name = normalized_bounded_resource(&packet.name)?;
    let target = normalized_bounded_resource(&packet.target)?;
    let pool = normalized_bounded_resource(&packet.pool)?;

    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_string(&name);
    out.write_string(&target);
    out.write_string(&pool);
    out.write_java_string(&packet.final_state, WIRE_STRING_MAX_UTF16_UNITS)?;
    out.write_string(packet.joint.serialized_name());
    out.write_var_i32(packet.selection_priority);
    out.write_var_i32(packet.placement_priority);
    Ok((ids::play::SERVERBOUND_SET_JIGSAW_BLOCK, out.into_inner()))
}

pub fn encode_play_jigsaw_generate(packet: JigsawGenerate) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_var_i32(packet.levels);
    out.write_bool(packet.keep_jigsaws);
    (ids::play::SERVERBOUND_JIGSAW_GENERATE, out.into_inner())
}

fn normalized_bounded_resource(value: &str) -> Result<String> {
    validate_utf16(value, WIRE_STRING_MAX_UTF16_UNITS)?;
    normalize_resource_location(value.to_string())
}

fn validate_utf16(value: &str, max: usize) -> Result<()> {
    let len = value.encode_utf16().count();
    if len > max {
        return Err(ProtocolError::StringTooLong { actual: len, max });
    }
    Ok(())
}

/// `ServerboundSetJigsawBlockPacket(FriendlyByteBuf)`
/// (ServerboundSetJigsawBlockPacket.java:41-50).
pub(super) fn decode_set_jigsaw_block(decoder: &mut Decoder<'_>) -> Result<SetJigsawBlock> {
    Ok(SetJigsawBlock {
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        name: super::read_resource_location(decoder)?,
        target: super::read_resource_location(decoder)?,
        pool: super::read_resource_location(decoder)?,
        final_state: decoder.read_java_string(WIRE_STRING_MAX_UTF16_UNITS)?,
        joint: JigsawJoint::by_name(&decoder.read_string(WIRE_STRING_MAX_UTF16_UNITS)?),
        selection_priority: decoder.read_var_i32()?,
        placement_priority: decoder.read_var_i32()?,
    })
}

/// `ServerboundJigsawGeneratePacket(FriendlyByteBuf)`
/// (ServerboundJigsawGeneratePacket.java:26-30).
pub(super) fn decode_jigsaw_generate(decoder: &mut Decoder<'_>) -> Result<JigsawGenerate> {
    Ok(JigsawGenerate {
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        levels: decoder.read_var_i32()?,
        keep_jigsaws: decoder.read_bool()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Decoder;

    fn set_packet(joint: JigsawJoint) -> SetJigsawBlock {
        SetJigsawBlock {
            pos: BlockPos {
                x: -12,
                y: 70,
                z: 31,
            },
            name: "entry".to_string(),
            target: "example:target".to_string(),
            pool: "example:village/houses".to_string(),
            final_state: "minecraft:oak_planks[axis=y]".into(),
            joint,
            selection_priority: i32::MIN,
            placement_priority: i32::MAX,
        }
    }

    #[test]
    fn encodes_set_jigsaw_wire_order_resources_joint_and_priorities() {
        for (joint, expected_joint) in [
            (JigsawJoint::Rollable, "rollable"),
            (JigsawJoint::Aligned, "aligned"),
        ] {
            let (id, payload) = encode_play_set_jigsaw_block(&set_packet(joint)).unwrap();
            assert_eq!(id, ids::play::SERVERBOUND_SET_JIGSAW_BLOCK);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(
                decoder.read_i64().unwrap(),
                chunks::encode_block_pos(BlockPos {
                    x: -12,
                    y: 70,
                    z: 31
                })
            );
            assert_eq!(decoder.read_string(128).unwrap(), "minecraft:entry");
            assert_eq!(decoder.read_string(128).unwrap(), "example:target");
            assert_eq!(decoder.read_string(128).unwrap(), "example:village/houses");
            assert_eq!(
                decoder.read_string(256).unwrap(),
                "minecraft:oak_planks[axis=y]"
            );
            assert_eq!(decoder.read_string(16).unwrap(), expected_joint);
            assert_eq!(decoder.read_var_i32().unwrap(), i32::MIN);
            assert_eq!(decoder.read_var_i32().unwrap(), i32::MAX);
            assert!(decoder.is_empty());
        }
    }

    #[test]
    fn encodes_jigsaw_generate_extremes_without_narrowing() {
        for packet in [
            JigsawGenerate {
                pos: BlockPos { x: 1, y: 2, z: 3 },
                levels: i32::MIN,
                keep_jigsaws: false,
            },
            JigsawGenerate {
                pos: BlockPos {
                    x: -1,
                    y: -2,
                    z: -3,
                },
                levels: i32::MAX,
                keep_jigsaws: true,
            },
        ] {
            let (id, payload) = encode_play_jigsaw_generate(packet);
            assert_eq!(id, ids::play::SERVERBOUND_JIGSAW_GENERATE);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(
                decoder.read_i64().unwrap(),
                chunks::encode_block_pos(packet.pos)
            );
            assert_eq!(decoder.read_var_i32().unwrap(), packet.levels);
            assert_eq!(decoder.read_bool().unwrap(), packet.keep_jigsaws);
            assert!(decoder.is_empty());
        }
    }

    #[test]
    fn set_jigsaw_rejects_invalid_identifiers() {
        let mut packet = set_packet(JigsawJoint::Aligned);
        packet.name = "Bad Name".to_string();
        assert!(matches!(
            encode_play_set_jigsaw_block(&packet),
            Err(ProtocolError::InvalidData(_))
        ));
    }
}
