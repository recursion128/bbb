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
    Ok(LevelParticles {
        override_limiter: decoder.read_bool()?,
        always_show: decoder.read_bool()?,
        position: decode_vec3d(decoder)?,
        offset: decode_vec3f_as_vec3d(decoder)?,
        max_speed: decoder.read_f32()?,
        count: decoder.read_i32()?,
        particle: decode_particle_payload(decoder)?,
    })
}

pub(crate) fn decode_projectile_power(decoder: &mut Decoder<'_>) -> Result<ProjectilePower> {
    Ok(ProjectilePower {
        entity_id: decoder.read_var_i32()?,
        acceleration_power: decoder.read_f64()?,
    })
}

fn decode_particle_payload(decoder: &mut Decoder<'_>) -> Result<ParticlePayload> {
    let particle_type_id = decoder.read_var_i32()?;
    let raw_options = decode_remaining_payload(decoder, "particle options")?;
    Ok(ParticlePayload {
        particle_type_id,
        raw_options,
    })
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
