use serde::{Deserialize, Serialize};

pub use crate::entity_anchors::EntityAnchor;
use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::decode_rich_text_component_from_decoder_with_registry,
    content_registry::ContentRegistry,
    ids, RichTextComponent,
};

use super::Vec3d;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatEnd {
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatKill {
    pub player_id: i32,
    pub message: RichTextComponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerLookAt {
    pub from_anchor: EntityAnchor,
    pub position: Vec3d,
    pub target: Option<PlayerLookAtTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerLookAtTarget {
    pub entity_id: i32,
    pub to_anchor: EntityAnchor,
}

pub fn encode_play_clientbound_player_combat_enter() -> (i32, Vec<u8>) {
    (ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER, Vec::new())
}

pub fn encode_play_clientbound_player_combat_end(update: PlayerCombatEnd) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.duration);
    (
        ids::play::CLIENTBOUND_PLAYER_COMBAT_END,
        encoder.into_inner(),
    )
}

pub(crate) fn decode_player_combat_enter(decoder: &Decoder<'_>) -> Result<()> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player combat enter packet".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn decode_player_combat_end(decoder: &mut Decoder<'_>) -> Result<PlayerCombatEnd> {
    let update = PlayerCombatEnd {
        duration: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player combat end packet".to_string(),
        ));
    }
    Ok(update)
}

pub(crate) fn decode_player_combat_kill_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PlayerCombatKill> {
    let update = PlayerCombatKill {
        player_id: decoder.read_var_i32()?,
        message: decode_rich_text_component_from_decoder_with_registry(decoder, registry)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player combat kill packet".to_string(),
        ));
    }
    Ok(update)
}

pub(crate) fn decode_player_look_at(decoder: &mut Decoder<'_>) -> Result<PlayerLookAt> {
    let from_anchor = decode_entity_anchor(decoder)?;
    let position = Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    };
    let target = if decoder.read_bool()? {
        Some(PlayerLookAtTarget {
            entity_id: decoder.read_var_i32()?,
            to_anchor: decode_entity_anchor(decoder)?,
        })
    } else {
        None
    };

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after player look at packet".to_string(),
        ));
    }
    Ok(PlayerLookAt {
        from_anchor,
        position,
        target,
    })
}

fn decode_entity_anchor(decoder: &mut Decoder<'_>) -> Result<EntityAnchor> {
    let ordinal = decoder.read_var_i32()?;
    EntityAnchor::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid entity anchor ordinal {ordinal}"))
    })
}

#[cfg(test)]
mod tests;
