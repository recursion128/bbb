use serde::{Deserialize, Serialize};

use crate::{
    codec::{Encoder, ProtocolError, Result},
    ids,
};

use super::{chunks, BlockPos};

pub const MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS: usize = 128;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestBlockMode {
    Start,
    Log,
    #[default]
    Fail,
    Accept,
}

impl TestBlockMode {
    fn wire_id(self) -> i32 {
        match self {
            Self::Start => 0,
            Self::Log => 1,
            Self::Fail => 2,
            Self::Accept => 3,
        }
    }

    pub fn serialized_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Log => "log",
            Self::Fail => "fail",
            Self::Accept => "accept",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTestBlock {
    pub pos: BlockPos,
    pub mode: TestBlockMode,
    pub message: String,
}

pub fn encode_play_set_test_block(packet: &SetTestBlock) -> Result<(i32, Vec<u8>)> {
    let message_len = packet.message.encode_utf16().count();
    if message_len > MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS {
        return Err(ProtocolError::StringTooLong {
            actual: message_len,
            max: MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS,
        });
    }

    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_var_i32(packet.mode.wire_id());
    out.write_string(&packet.message);
    Ok((ids::play::SERVERBOUND_SET_TEST_BLOCK, out.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Decoder;

    #[test]
    fn encodes_set_test_block_wire_order_position_and_all_mode_ids() {
        for (mode_id, mode) in [
            TestBlockMode::Start,
            TestBlockMode::Log,
            TestBlockMode::Fail,
            TestBlockMode::Accept,
        ]
        .into_iter()
        .enumerate()
        {
            let packet = SetTestBlock {
                pos: BlockPos {
                    x: -12,
                    y: 70,
                    z: 31,
                },
                mode,
                message: "message".to_string(),
            };
            let (id, payload) = encode_play_set_test_block(&packet).unwrap();
            assert_eq!(id, ids::play::SERVERBOUND_SET_TEST_BLOCK);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(
                decoder.read_i64().unwrap(),
                chunks::encode_block_pos(packet.pos)
            );
            assert_eq!(decoder.read_var_i32().unwrap(), mode_id as i32);
            assert_eq!(decoder.read_string(128).unwrap(), "message");
            assert!(decoder.is_empty());
        }
    }

    #[test]
    fn set_test_block_enforces_editor_utf16_boundary() {
        let mut packet = SetTestBlock {
            pos: BlockPos { x: 0, y: 0, z: 0 },
            mode: TestBlockMode::Start,
            message: "🙂".repeat(MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS / 2),
        };
        assert!(encode_play_set_test_block(&packet).is_ok());

        packet.message.push('x');
        assert!(matches!(
            encode_play_set_test_block(&packet),
            Err(ProtocolError::StringTooLong {
                actual: 129,
                max: MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS,
            })
        ));
    }
}
