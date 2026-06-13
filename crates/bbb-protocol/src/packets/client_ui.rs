use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::{chunks, BlockPos, InteractionHand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountScreenOpen {
    pub container_id: i32,
    pub inventory_columns: i32,
    pub entity_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenBook {
    pub hand: InteractionHand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSignEditor {
    pub pos: BlockPos,
    pub is_front_text: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PongResponse {
    pub time: i64,
}

pub(crate) fn decode_mount_screen_open(decoder: &mut Decoder<'_>) -> Result<MountScreenOpen> {
    Ok(MountScreenOpen {
        container_id: decoder.read_var_i32()?,
        inventory_columns: decoder.read_var_i32()?,
        entity_id: decoder.read_i32()?,
    })
}

pub(crate) fn decode_open_book(decoder: &mut Decoder<'_>) -> Result<OpenBook> {
    Ok(OpenBook {
        hand: decode_interaction_hand(decoder.read_var_i32()?)?,
    })
}

pub(crate) fn decode_open_sign_editor(decoder: &mut Decoder<'_>) -> Result<OpenSignEditor> {
    Ok(OpenSignEditor {
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        is_front_text: decoder.read_bool()?,
    })
}

pub(crate) fn decode_play_pong_response(decoder: &mut Decoder<'_>) -> Result<PongResponse> {
    Ok(PongResponse {
        time: decoder.read_i64()?,
    })
}

fn decode_interaction_hand(id: i32) -> Result<InteractionHand> {
    match id {
        0 => Ok(InteractionHand::MainHand),
        1 => Ok(InteractionHand::OffHand),
        other => Err(ProtocolError::InvalidData(format!(
            "invalid interaction hand ordinal {other}"
        ))),
    }
}

#[cfg(test)]
mod tests;
