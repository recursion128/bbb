use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};
pub use crate::sound_sources::SoundSource;

use super::{read_resource_location, Vec3d};

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
    let packet = SoundEntityEvent {
        sound: decode_sound_event_holder(decoder)?,
        source: decode_sound_source(decoder)?,
        entity_id: decoder.read_var_i32()?,
        volume: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
        seed: decoder.read_i64()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after sound entity packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_stop_sound(decoder: &mut Decoder<'_>) -> Result<StopSound> {
    let flags = decoder.read_u8()?;
    let source = if flags & 1 != 0 {
        Some(decode_sound_source(decoder)?)
    } else {
        None
    };
    let name = if flags & 2 != 0 {
        Some(read_resource_location(decoder)?)
    } else {
        None
    };

    Ok(StopSound { source, name })
}

fn decode_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<SoundEventHolder> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id == 0 {
        let location = read_resource_location(decoder)?;
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
    let ordinal = decoder.read_var_i32()?;
    SoundSource::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid sound source ordinal {ordinal}"))
    })
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
