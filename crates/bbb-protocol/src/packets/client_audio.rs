use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::Vec3d;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEvent {
    pub sound: SoundEventHolder,
    pub source: SoundSource,
    pub position: Vec3d,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEntityEvent {
    pub sound: SoundEventHolder,
    pub source: SoundSource,
    pub entity_id: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSound {
    pub source: Option<SoundSource>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SoundEventHolder {
    Reference {
        registry_id: i32,
    },
    Direct {
        location: String,
        fixed_range: Option<f32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundSource {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Hostile,
    Neutral,
    Players,
    Ambient,
    Voice,
    Ui,
}

impl SoundSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Records => "record",
            Self::Weather => "weather",
            Self::Blocks => "block",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Players => "player",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
            Self::Ui => "ui",
        }
    }

    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Master,
            1 => Self::Music,
            2 => Self::Records,
            3 => Self::Weather,
            4 => Self::Blocks,
            5 => Self::Hostile,
            6 => Self::Neutral,
            7 => Self::Players,
            8 => Self::Ambient,
            9 => Self::Voice,
            10 => Self::Ui,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid sound source ordinal {other}"
                )))
            }
        })
    }
}

pub(crate) fn decode_sound_event(decoder: &mut Decoder<'_>) -> Result<SoundEvent> {
    Ok(SoundEvent {
        sound: decode_sound_event_holder(decoder)?,
        source: decode_sound_source(decoder)?,
        position: decode_sound_position(decoder)?,
        volume: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
        seed: decoder.read_i64()?,
    })
}

pub(crate) fn decode_sound_entity_event(decoder: &mut Decoder<'_>) -> Result<SoundEntityEvent> {
    Ok(SoundEntityEvent {
        sound: decode_sound_event_holder(decoder)?,
        source: decode_sound_source(decoder)?,
        entity_id: decoder.read_var_i32()?,
        volume: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
        seed: decoder.read_i64()?,
    })
}

pub(crate) fn decode_stop_sound(decoder: &mut Decoder<'_>) -> Result<StopSound> {
    let flags = decoder.read_u8()?;
    let source = if flags & 1 != 0 {
        Some(decode_sound_source(decoder)?)
    } else {
        None
    };
    let name = if flags & 2 != 0 {
        Some(decoder.read_string(32767)?)
    } else {
        None
    };

    Ok(StopSound { source, name })
}

fn decode_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<SoundEventHolder> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id == 0 {
        let location = decoder.read_string(32767)?;
        let fixed_range = if decoder.read_bool()? {
            Some(decoder.read_f32()?)
        } else {
            None
        };
        return Ok(SoundEventHolder::Direct {
            location,
            fixed_range,
        });
    }
    if holder_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid sound event holder id {holder_id}"
        )));
    }

    Ok(SoundEventHolder::Reference {
        registry_id: holder_id - 1,
    })
}

fn decode_sound_source(decoder: &mut Decoder<'_>) -> Result<SoundSource> {
    SoundSource::from_ordinal(decoder.read_var_i32()?)
}

fn decode_sound_position(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: f64::from(decoder.read_i32()?) / 8.0,
        y: f64::from(decoder.read_i32()?) / 8.0,
        z: f64::from(decoder.read_i32()?) / 8.0,
    })
}

#[cfg(test)]
mod tests;
