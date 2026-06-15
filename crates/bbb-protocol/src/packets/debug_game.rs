use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

use super::{chunks, read_resource_location, BlockPos, ChunkPos};

const MAX_DEBUG_PAYLOAD: usize = 2 * 1024 * 1024;
const MAX_DEBUG_SAMPLE_LONGS: usize = 1_000_000;
const MAX_GAME_RULE_VALUES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBlockValue {
    pub pos: BlockPos,
    pub raw_update_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugChunkValue {
    pub pos: ChunkPos,
    pub raw_update_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEntityValue {
    pub entity_id: i32,
    pub raw_update_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEvent {
    pub raw_event_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSample {
    pub sample: Vec<i64>,
    pub sample_type: RemoteDebugSampleType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteDebugSampleType {
    TickTime,
}

impl RemoteDebugSampleType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TickTime => "tick_time",
        }
    }

    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::TickTime,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid remote debug sample type ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValues {
    pub values: Vec<GameRuleValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValue {
    pub rule: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightPos {
    pub absolute_pos: BlockPos,
    pub relative_pos: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockStatus {
    pub status: String,
    pub size: Option<Vec3i>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub(crate) fn decode_debug_block_value(decoder: &mut Decoder<'_>) -> Result<DebugBlockValue> {
    let pos = chunks::decode_block_pos(decoder.read_i64()?);
    let raw_update_payload = decode_remaining_payload(decoder, "debug block update payload")?;
    Ok(DebugBlockValue {
        pos,
        raw_update_payload,
    })
}

pub(crate) fn decode_debug_chunk_value(decoder: &mut Decoder<'_>) -> Result<DebugChunkValue> {
    let pos = chunks::decode_chunk_pos(decoder.read_i64()?);
    let raw_update_payload = decode_remaining_payload(decoder, "debug chunk update payload")?;
    Ok(DebugChunkValue {
        pos,
        raw_update_payload,
    })
}

pub(crate) fn decode_debug_entity_value(decoder: &mut Decoder<'_>) -> Result<DebugEntityValue> {
    let entity_id = decoder.read_var_i32()?;
    let raw_update_payload = decode_remaining_payload(decoder, "debug entity update payload")?;
    Ok(DebugEntityValue {
        entity_id,
        raw_update_payload,
    })
}

pub(crate) fn decode_debug_event(decoder: &mut Decoder<'_>) -> Result<DebugEvent> {
    let raw_event_payload = decode_remaining_payload(decoder, "debug event payload")?;
    Ok(DebugEvent { raw_event_payload })
}

pub(crate) fn decode_debug_sample(decoder: &mut Decoder<'_>) -> Result<DebugSample> {
    let count = decoder.read_len()?;
    if count > MAX_DEBUG_SAMPLE_LONGS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_DEBUG_SAMPLE_LONGS));
    }

    let mut sample = Vec::with_capacity(count);
    for _ in 0..count {
        sample.push(decoder.read_i64()?);
    }

    Ok(DebugSample {
        sample,
        sample_type: RemoteDebugSampleType::from_ordinal(decoder.read_var_i32()?)?,
    })
}

pub(crate) fn decode_game_rule_values(decoder: &mut Decoder<'_>) -> Result<GameRuleValues> {
    let count = decoder.read_len()?;
    if count > MAX_GAME_RULE_VALUES {
        return Err(ProtocolError::PacketTooLarge(count, MAX_GAME_RULE_VALUES));
    }

    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(GameRuleValue {
            rule: read_resource_location(decoder)?,
            value: decoder.read_string(32767)?,
        });
    }

    Ok(GameRuleValues { values })
}

pub(crate) fn decode_game_test_highlight_pos(
    decoder: &mut Decoder<'_>,
) -> Result<GameTestHighlightPos> {
    Ok(GameTestHighlightPos {
        absolute_pos: chunks::decode_block_pos(decoder.read_i64()?),
        relative_pos: chunks::decode_block_pos(decoder.read_i64()?),
    })
}

pub(crate) fn decode_test_instance_block_status(
    decoder: &mut Decoder<'_>,
) -> Result<TestInstanceBlockStatus> {
    let status = decode_component_summary_from_decoder(decoder)?;
    let size = if decoder.read_bool()? {
        Some(decode_vec3i(decoder)?)
    } else {
        None
    };

    Ok(TestInstanceBlockStatus { status, size })
}

fn decode_vec3i(decoder: &mut Decoder<'_>) -> Result<Vec3i> {
    Ok(Vec3i {
        x: decoder.read_var_i32()?,
        y: decoder.read_var_i32()?,
        z: decoder.read_var_i32()?,
    })
}

fn decode_remaining_payload(decoder: &mut Decoder<'_>, what: &'static str) -> Result<Vec<u8>> {
    let len = decoder.remaining_len();
    if len > MAX_DEBUG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(len, MAX_DEBUG_PAYLOAD));
    }

    Ok(decoder.read_exact(len, what)?.to_vec())
}

#[cfg(test)]
mod tests;
