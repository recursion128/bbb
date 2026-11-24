use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, BlockPos};

pub const MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS: usize = 32_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandBlockMode {
    Sequence,
    Auto,
    Redstone,
}

impl CommandBlockMode {
    fn wire_id(self) -> i32 {
        match self {
            Self::Sequence => 0,
            Self::Auto => 1,
            Self::Redstone => 2,
        }
    }

    fn from_wire_id(id: i32) -> Result<Self> {
        match id {
            0 => Ok(Self::Sequence),
            1 => Ok(Self::Auto),
            2 => Ok(Self::Redstone),
            other => Err(ProtocolError::InvalidData(format!(
                "invalid command block mode ordinal {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCommandBlock {
    pub pos: BlockPos,
    pub command: crate::JavaString,
    pub mode: CommandBlockMode,
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCommandMinecart {
    pub entity_id: i32,
    pub command: crate::JavaString,
    pub track_output: bool,
}

pub fn encode_play_set_command_block(packet: &SetCommandBlock) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_java_string(&packet.command, MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS)?;
    out.write_var_i32(packet.mode.wire_id());
    let flags = u8::from(packet.track_output)
        | (u8::from(packet.conditional) << 1)
        | (u8::from(packet.automatic) << 2);
    out.write_u8(flags);
    Ok((ids::play::SERVERBOUND_SET_COMMAND_BLOCK, out.into_inner()))
}

pub fn encode_play_set_command_minecart(packet: &SetCommandMinecart) -> Result<(i32, Vec<u8>)> {
    let mut out = Encoder::new();
    out.write_var_i32(packet.entity_id);
    out.write_java_string(&packet.command, MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS)?;
    out.write_bool(packet.track_output);
    Ok((
        ids::play::SERVERBOUND_SET_COMMAND_MINECART,
        out.into_inner(),
    ))
}

/// `ServerboundSetCommandBlockPacket(FriendlyByteBuf)`
/// (ServerboundSetCommandBlockPacket.java:36-46).
pub(super) fn decode_set_command_block(decoder: &mut Decoder<'_>) -> Result<SetCommandBlock> {
    let pos = chunks::decode_block_pos(decoder.read_i64()?);
    let command = decoder.read_java_string(MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS)?;
    let mode = CommandBlockMode::from_wire_id(decoder.read_var_i32()?)?;
    let flags = decoder.read_u8()?;
    Ok(SetCommandBlock {
        pos,
        command,
        mode,
        track_output: flags & 1 != 0,
        conditional: flags & 2 != 0,
        automatic: flags & 4 != 0,
    })
}

/// `ServerboundSetCommandMinecartPacket(FriendlyByteBuf)`
/// (ServerboundSetCommandMinecartPacket.java:29-33).
pub(super) fn decode_set_command_minecart(decoder: &mut Decoder<'_>) -> Result<SetCommandMinecart> {
    Ok(SetCommandMinecart {
        entity_id: decoder.read_var_i32()?,
        command: decoder.read_java_string(MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS)?,
        track_output: decoder.read_bool()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Decoder;

    fn packet(mode: CommandBlockMode, flags: u8) -> SetCommandBlock {
        SetCommandBlock {
            pos: BlockPos {
                x: 12,
                y: 64,
                z: -7,
            },
            command: "say hello".into(),
            mode,
            track_output: flags & 1 != 0,
            conditional: flags & 2 != 0,
            automatic: flags & 4 != 0,
        }
    }

    #[test]
    fn encodes_set_command_block_wire_order_modes_and_flags() {
        for (mode, mode_id) in [
            (CommandBlockMode::Sequence, 0),
            (CommandBlockMode::Auto, 1),
            (CommandBlockMode::Redstone, 2),
        ] {
            for flags in 0..=7 {
                let (id, payload) = encode_play_set_command_block(&packet(mode, flags)).unwrap();
                assert_eq!(id, ids::play::SERVERBOUND_SET_COMMAND_BLOCK);
                let mut decoder = Decoder::new(&payload);
                assert_eq!(
                    decoder.read_i64().unwrap(),
                    chunks::encode_block_pos(BlockPos {
                        x: 12,
                        y: 64,
                        z: -7
                    })
                );
                assert_eq!(decoder.read_string(32_500).unwrap(), "say hello");
                assert_eq!(decoder.read_var_i32().unwrap(), mode_id);
                assert_eq!(decoder.read_u8().unwrap(), flags);
                assert!(decoder.is_empty());
            }
        }
    }

    #[test]
    fn set_command_block_enforces_editor_utf16_boundary() {
        let mut boundary = packet(CommandBlockMode::Redstone, 1);
        boundary.command = "\u{1f642}"
            .repeat(MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS / 2)
            .into();
        assert!(encode_play_set_command_block(&boundary).is_ok());

        boundary.command.push_str("x");
        assert!(matches!(
            encode_play_set_command_block(&boundary),
            Err(ProtocolError::StringTooLong {
                actual: 32_501,
                max: MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
            })
        ));
    }

    #[test]
    fn encodes_set_command_minecart_wire_order_and_multibyte_entity_id() {
        let packet = SetCommandMinecart {
            entity_id: 300,
            command: "say minecart".into(),
            track_output: false,
        };
        let (id, payload) = encode_play_set_command_minecart(&packet).unwrap();
        assert_eq!(id, ids::play::SERVERBOUND_SET_COMMAND_MINECART);

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 300);
        assert_eq!(decoder.read_string(32_767).unwrap(), "say minecart");
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn set_command_minecart_enforces_editor_utf16_boundary() {
        let mut packet = SetCommandMinecart {
            entity_id: 1,
            command: "\u{1f642}"
                .repeat(MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS / 2)
                .into(),
            track_output: true,
        };
        assert!(encode_play_set_command_minecart(&packet).is_ok());

        packet.command.push_str("x");
        assert!(matches!(
            encode_play_set_command_minecart(&packet),
            Err(ProtocolError::StringTooLong {
                actual: 32_501,
                max: MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
            })
        ));
    }
}
