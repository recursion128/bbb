use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

use super::Vec3d;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatEnd {
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatKill {
    pub player_id: i32,
    pub message: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityAnchor {
    Feet,
    Eyes,
}

impl EntityAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feet => "feet",
            Self::Eyes => "eyes",
        }
    }

    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Feet,
            1 => Self::Eyes,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid entity anchor ordinal {other}"
                )));
            }
        })
    }
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

pub(crate) fn decode_player_combat_kill(decoder: &mut Decoder<'_>) -> Result<PlayerCombatKill> {
    let update = PlayerCombatKill {
        player_id: decoder.read_var_i32()?,
        message: decode_component_summary_from_decoder(decoder)?,
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
    EntityAnchor::from_ordinal(decoder.read_var_i32()?)
}

#[cfg(test)]
mod tests;
