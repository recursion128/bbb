use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::Vec3d;

const MAX_EFFECT_PAYLOAD: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explosion {
    pub center: Vec3d,
    pub radius: f32,
    pub block_count: i32,
    pub player_knockback: Option<Vec3d>,
    pub raw_effect_payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelParticles {
    pub override_limiter: bool,
    pub always_show: bool,
    pub position: Vec3d,
    pub offset: Vec3d,
    pub max_speed: f32,
    pub count: i32,
    pub particle: ParticlePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticlePayload {
    pub particle_type_id: i32,
    pub raw_options: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePower {
    pub entity_id: i32,
    pub acceleration_power: f64,
}

pub(crate) fn decode_explosion(decoder: &mut Decoder<'_>) -> Result<Explosion> {
    let center = decode_vec3d(decoder)?;
    let radius = decoder.read_f32()?;
    let block_count = decoder.read_i32()?;
    let player_knockback = if decoder.read_bool()? {
        Some(decode_vec3d(decoder)?)
    } else {
        None
    };
    let raw_effect_payload = decode_remaining_payload(decoder, "explosion effect payload")?;

    Ok(Explosion {
        center,
        radius,
        block_count,
        player_knockback,
        raw_effect_payload,
    })
}

pub(crate) fn decode_level_particles(decoder: &mut Decoder<'_>) -> Result<LevelParticles> {
    let packet = LevelParticles {
        override_limiter: decoder.read_bool()?,
        always_show: decoder.read_bool()?,
        position: decode_vec3d(decoder)?,
        offset: decode_vec3f_as_vec3d(decoder)?,
        max_speed: decoder.read_f32()?,
        count: decoder.read_i32()?,
        particle: decode_particle_payload(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "particle packet has {} trailing byte(s)",
            decoder.remaining_len()
        )));
    }
    Ok(packet)
}

pub(crate) fn decode_projectile_power(decoder: &mut Decoder<'_>) -> Result<ProjectilePower> {
    Ok(ProjectilePower {
        entity_id: decoder.read_var_i32()?,
        acceleration_power: decoder.read_f64()?,
    })
}

pub(super) fn decode_particle_payload(decoder: &mut Decoder<'_>) -> Result<ParticlePayload> {
    let particle_type_id = decoder.read_var_i32()?;
    let option_start = decoder.position();
    decode_particle_options(decoder, particle_type_id)?;
    let raw_options = decoder.bytes_from(option_start).to_vec();
    Ok(ParticlePayload {
        particle_type_id,
        raw_options,
    })
}

fn decode_particle_options(decoder: &mut Decoder<'_>, particle_type_id: i32) -> Result<()> {
    match particle_type_id {
        1 | 2 | 29 | 111 | 115 => {
            decoder.read_var_i32()?;
        }
        8 | 38 => {
            decoder.read_f32()?;
        }
        14 => {
            decoder.read_i32()?;
            decoder.read_f32()?;
        }
        15 => {
            decoder.read_i32()?;
            decoder.read_i32()?;
            decoder.read_f32()?;
        }
        16 | 46 => {
            decoder.read_i32()?;
            decoder.read_f32()?;
        }
        21 | 36 | 42 => {
            decoder.read_i32()?;
        }
        49 => {
            decode_vec3d(decoder)?;
            decoder.read_i32()?;
            decoder.read_var_i32()?;
        }
        105 => {
            decoder.read_var_i32()?;
        }
        47 => {
            return Err(ProtocolError::InvalidData(
                "unsupported particle options for item particle".to_string(),
            ));
        }
        48 => {
            return Err(ProtocolError::InvalidData(
                "unsupported particle options for vibration particle".to_string(),
            ));
        }
        other if is_simple_particle_type(other) => {}
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unknown particle type id {other}"
            )));
        }
    }
    Ok(())
}

fn is_simple_particle_type(particle_type_id: i32) -> bool {
    matches!(
        particle_type_id,
        0 | 3
            | 4
            | 5
            | 6
            | 7
            | 9
            | 10
            | 11
            | 12
            | 13
            | 17
            | 18
            | 19
            | 20
            | 22
            | 23
            | 24
            | 25
            | 26
            | 27
            | 28
            | 30
            | 31
            | 32
            | 33
            | 34
            | 35
            | 37
            | 39
            | 40
            | 41
            | 43
            | 44
            | 45
            | 50
            | 51
            | 52
            | 53
            | 54
            | 55
            | 56
            | 57
            | 58
            | 59
            | 60
            | 61
            | 62
            | 63
            | 64
            | 65
            | 66
            | 67
            | 68
            | 69
            | 70
            | 71
            | 72
            | 73
            | 74
            | 75
            | 76
            | 77
            | 78
            | 79
            | 80
            | 81
            | 82
            | 83
            | 84
            | 85
            | 86
            | 87
            | 88
            | 89
            | 90
            | 91
            | 92
            | 93
            | 94
            | 95
            | 96
            | 97
            | 98
            | 99
            | 100
            | 101
            | 102
            | 103
            | 104
            | 106
            | 107
            | 108
            | 109
            | 110
            | 112
            | 113
            | 114
            | 116
    )
}

fn decode_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    })
}

fn decode_vec3f_as_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: f64::from(decoder.read_f32()?),
        y: f64::from(decoder.read_f32()?),
        z: f64::from(decoder.read_f32()?),
    })
}

fn decode_remaining_payload(decoder: &mut Decoder<'_>, what: &'static str) -> Result<Vec<u8>> {
    let len = decoder.remaining_len();
    if len > MAX_EFFECT_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(len, MAX_EFFECT_PAYLOAD));
    }
    Ok(decoder.read_exact(len, what)?.to_vec())
}

#[cfg(test)]
mod tests;
