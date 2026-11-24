use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::content_registry::ContentRegistry;
use crate::ids;
use crate::sound_event_types::vanilla_sound_event_resource_id;
pub use bbb_facts::sound_sources::SoundSource;

use super::{normalize_resource_location, read_resource_location, Vec3d};

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

/// `ClientboundSoundPacket.write` (`ClientboundSoundPacket.java:57-65`).
///
/// The packet constructor is where vanilla loses precision: it stores
/// `(int)(x * 8.0)` and hands the client back `x / 8.0F`
/// (`ClientboundSoundPacket.java:20-41, 88-98`). Keeping the quantisation here
/// rather than at the call site makes this encoder the exact inverse of
/// `decode_sound_event`, and leaves the emitting server free to pass the
/// entity position it actually has.
pub fn encode_play_clientbound_sound(update: &SoundEvent) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encode_sound_event_holder(&mut encoder, &update.sound)?;
    encoder.write_var_i32(update.source.ordinal());
    encoder.write_i32(sound_position_eighths(update.position.x));
    encoder.write_i32(sound_position_eighths(update.position.y));
    encoder.write_i32(sound_position_eighths(update.position.z));
    encoder.write_f32(update.volume);
    encoder.write_f32(update.pitch);
    encoder.write_i64(update.seed);
    Ok((ids::play::CLIENTBOUND_SOUND, encoder.into_inner()))
}

/// `ClientboundSoundEntityPacket.write` (`ClientboundSoundEntityPacket.java:43-50`).
pub fn encode_play_clientbound_sound_entity(update: &SoundEntityEvent) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encode_sound_event_holder(&mut encoder, &update.sound)?;
    encoder.write_var_i32(update.source.ordinal());
    encoder.write_var_i32(update.entity_id);
    encoder.write_f32(update.volume);
    encoder.write_f32(update.pitch);
    encoder.write_i64(update.seed);
    Ok((ids::play::CLIENTBOUND_SOUND_ENTITY, encoder.into_inner()))
}

/// `LOCATION_ACCURACY` (`ClientboundSoundPacket.java:15`) applied by the same
/// narrowing cast Java uses, which truncates towards zero on both signs.
fn sound_position_eighths(coordinate: f64) -> i32 {
    (coordinate * 8.0) as i32
}

/// `ClientboundStopSoundPacket.write` (`ClientboundStopSoundPacket.java:40-56`):
/// a flags byte (`HAS_SOURCE` = 1, `HAS_SOUND` = 2) followed by whichever of
/// the two optionals it announced, in that order.
pub fn encode_play_clientbound_stop_sound(update: &StopSound) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    let mut flags = 0u8;
    if update.source.is_some() {
        flags |= 1;
    }
    if update.name.is_some() {
        flags |= 2;
    }
    encoder.write_u8(flags);
    if let Some(source) = update.source {
        encoder.write_var_i32(source.ordinal());
    }
    if let Some(name) = &update.name {
        encoder.write_string(&normalize_resource_location(name.clone())?);
    }
    Ok((ids::play::CLIENTBOUND_STOP_SOUND, encoder.into_inner()))
}

/// `ByteBufCodecs.holderRegistry` (`ByteBufCodecs.java`): a registry entry is
/// written as its id plus one, and zero introduces an inline definition.
pub(super) fn encode_sound_event_holder(
    encoder: &mut Encoder,
    holder: &SoundEventHolder,
) -> Result<()> {
    match holder {
        SoundEventHolder::Reference { registry_id } => {
            encoder.write_var_i32(registry_id + 1);
        }
        SoundEventHolder::Direct {
            location,
            fixed_range,
        } => {
            encoder.write_var_i32(0);
            encoder.write_string(&normalize_resource_location(location.clone())?);
            match fixed_range {
                Some(range) => {
                    encoder.write_bool(true);
                    encoder.write_f32(*range);
                }
                None => encoder.write_bool(false),
            }
        }
    }
    Ok(())
}

pub(crate) fn decode_sound_event_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SoundEvent> {
    let packet = SoundEvent {
        sound: decode_sound_event_holder_with_registry(decoder, registry)?,
        source: decode_sound_source(decoder)?,
        position: decode_sound_position(decoder)?,
        volume: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
        seed: decoder.read_i64()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after sound packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_sound_entity_event_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SoundEntityEvent> {
    let packet = SoundEntityEvent {
        sound: decode_sound_event_holder_with_registry(decoder, registry)?,
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

    let packet = StopSound { source, name };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after stop sound packet".to_string(),
        ));
    }
    Ok(packet)
}

pub fn validate_sound_event_registry_id(
    registry_id: i32,
    registry: &ContentRegistry,
) -> Result<i32> {
    if vanilla_sound_event_resource_id(registry_id).is_some()
        || registry.sound_event_by_id(registry_id).is_some()
    {
        return Ok(registry_id);
    }
    Err(ProtocolError::InvalidData(format!(
        "unknown sound event registry id {registry_id}"
    )))
}

pub(super) fn decode_sound_event_holder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SoundEventHolder> {
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

    let registry_id = validate_sound_event_registry_id(holder_id - 1, registry)?;
    Ok(SoundEventHolder::Reference { registry_id })
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
