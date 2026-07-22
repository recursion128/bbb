use serde::{Deserialize, Serialize};

use crate::{
    codec::{Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, normalize_resource_location, BlockPos};

pub const MAX_JIGSAW_RESOURCE_UTF16_UNITS: usize = 128;
pub const MAX_JIGSAW_FINAL_STATE_UTF16_UNITS: usize = 256;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetJigsawBlock {
    pub pos: BlockPos,
    pub name: String,
    pub target: String,
    pub pool: String,
    pub final_state: String,
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
    validate_utf16(&packet.final_state, MAX_JIGSAW_FINAL_STATE_UTF16_UNITS)?;

    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_string(&name);
    out.write_string(&target);
    out.write_string(&pool);
    out.write_string(&packet.final_state);
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
    validate_utf16(value, MAX_JIGSAW_RESOURCE_UTF16_UNITS)?;
    normalize_resource_location(value.to_string())
}

fn validate_utf16(value: &str, max: usize) -> Result<()> {
    let len = value.encode_utf16().count();
    if len > max {
        return Err(ProtocolError::StringTooLong { actual: len, max });
    }
    Ok(())
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
            final_state: "minecraft:oak_planks[axis=y]".to_string(),
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
    fn set_jigsaw_enforces_identifier_validity_and_editor_utf16_bounds() {
        let mut packet = set_packet(JigsawJoint::Aligned);
        packet.name = "a".repeat(MAX_JIGSAW_RESOURCE_UTF16_UNITS);
        packet.final_state = "x".repeat(MAX_JIGSAW_FINAL_STATE_UTF16_UNITS);
        assert!(encode_play_set_jigsaw_block(&packet).is_ok());

        packet.name.push('x');
        assert!(matches!(
            encode_play_set_jigsaw_block(&packet),
            Err(ProtocolError::StringTooLong {
                actual: 129,
                max: 128
            })
        ));
        packet.name = "Bad Name".to_string();
        assert!(matches!(
            encode_play_set_jigsaw_block(&packet),
            Err(ProtocolError::InvalidData(_))
        ));
        packet.name = "valid".to_string();
        packet.final_state.push('x');
        assert!(matches!(
            encode_play_set_jigsaw_block(&packet),
            Err(ProtocolError::StringTooLong {
                actual: 257,
                max: 256
            })
        ));
    }
}
