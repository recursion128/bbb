use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_PARTICLE_RANDOM_SEED: i64 = 0x5EED_2601;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParticleDescriptor {
    pub(crate) provider: &'static str,
    pub(crate) lifetime: ParticleLifetimeDescriptor,
    pub(crate) sprite_selection: ParticleSpriteSelection,
    pub(crate) friction: f32,
    pub(crate) gravity: f32,
    pub(crate) has_physics: bool,
    pub(crate) speed_up_when_y_motion_is_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParticleLifetimeDescriptor {
    BaseParticle,
    Rising,
    PlayerCloud,
    BaseAshSmoke {
        max_lifetime: u32,
        scale_tenths: u32,
    },
    Explode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleSpriteSelection {
    First,
    Random,
    Age,
}

#[derive(Debug, Clone)]
pub(crate) struct ParticleRandom {
    seed: u64,
}

impl ParticleDescriptor {
    pub(crate) fn for_particle(particle_id: &str) -> Self {
        match particle_id {
            "minecraft:cloud" => Self {
                provider: "PlayerCloudParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::PlayerCloud,
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:flame" | "minecraft:soul_fire_flame" | "minecraft:copper_fire_flame" => {
                Self {
                    provider: "FlameParticle.Provider",
                    lifetime: ParticleLifetimeDescriptor::Rising,
                    sprite_selection: ParticleSpriteSelection::Random,
                    friction: 0.96,
                    gravity: 0.0,
                    has_physics: false,
                    speed_up_when_y_motion_is_blocked: false,
                }
            }
            "minecraft:small_flame" => Self {
                provider: "FlameParticle.SmallFlameProvider",
                lifetime: ParticleLifetimeDescriptor::Rising,
                sprite_selection: ParticleSpriteSelection::Random,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:large_smoke" => Self {
                provider: "LargeSmokeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 8,
                    scale_tenths: 25,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:smoke" => Self {
                provider: "SmokeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 8,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:white_smoke" => Self {
                provider: "WhiteSmokeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 8,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:ash" => Self {
                provider: "AshParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: 0.1,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:white_ash" => Self {
                provider: "WhiteAshParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.96,
                gravity: 0.0125,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:poof" => Self {
                provider: "ExplodeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Explode,
                sprite_selection: ParticleSpriteSelection::Age,
                friction: 0.9,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            _ => Self {
                provider: "Particle",
                lifetime: ParticleLifetimeDescriptor::BaseParticle,
                sprite_selection: ParticleSpriteSelection::First,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
        }
    }
}

impl ParticleLifetimeDescriptor {
    pub(crate) fn sample(self, random: &mut ParticleRandom) -> u32 {
        match self {
            Self::BaseParticle => (4.0 / (random.next_f64() * 0.9 + 0.1)) as u32,
            Self::Rising => (8.0 / (random.next_f64() * 0.8 + 0.2)) as u32 + 4,
            Self::PlayerCloud => {
                let base_lifetime = (8.0 / (random.next_f64() * 0.8 + 0.3)) as u32;
                ((base_lifetime as f32 * 2.5).max(1.0)) as u32
            }
            Self::BaseAshSmoke {
                max_lifetime,
                scale_tenths,
            } => {
                let scale = f64::from(scale_tenths) / 10.0;
                ((f64::from(max_lifetime) / (random.next_f64() * 0.8 + 0.2) * scale) as u32).max(1)
            }
            Self::Explode => (16.0 / (random.next_f64() * 0.8 + 0.2)) as u32 + 2,
        }
    }
}

pub(crate) fn select_initial_sprite(
    sprite_ids: &[String],
    selection: ParticleSpriteSelection,
    random: &mut ParticleRandom,
) -> (Option<usize>, Option<String>) {
    let index = match selection {
        ParticleSpriteSelection::First | ParticleSpriteSelection::Age => {
            (!sprite_ids.is_empty()).then_some(0)
        }
        ParticleSpriteSelection::Random => random.next_index(sprite_ids.len()),
    };
    let sprite_id = index.and_then(|index| sprite_ids.get(index).cloned());
    (index, sprite_id)
}

pub(crate) fn sprite_index_for_age(
    sprite_count: usize,
    age_ticks: u32,
    lifetime_ticks: u32,
) -> Option<usize> {
    if sprite_count == 0 {
        return None;
    }
    if sprite_count == 1 || lifetime_ticks == 0 {
        return Some(0);
    }
    let age = age_ticks as usize;
    let lifetime = lifetime_ticks as usize;
    Some(age.saturating_mul(sprite_count - 1) / lifetime).map(|index| index.min(sprite_count - 1))
}

const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;

impl ParticleRandom {
    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
        }
    }

    fn next_f64(&mut self) -> f64 {
        f64::from(self.next_bits(24)) / f64::from(1_u32 << 24)
    }

    fn next_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let bound = i32::try_from(len).ok()?;
        let mut bits = self.next_bits(31) as i32;
        let mut value = bits % bound;
        while bits.wrapping_sub(value).wrapping_add(bound - 1) < 0 {
            bits = self.next_bits(31) as i32;
            value = bits % bound;
        }
        Some(value as usize)
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_descriptor_maps_core_vanilla_providers_and_physics_flags() {
        assert_descriptor(
            "minecraft:cloud",
            "PlayerCloudParticle.Provider",
            ParticleLifetimeDescriptor::PlayerCloud,
            ParticleSpriteSelection::Age,
            0.96,
            0.0,
            false,
            false,
        );
        assert_descriptor(
            "minecraft:flame",
            "FlameParticle.Provider",
            ParticleLifetimeDescriptor::Rising,
            ParticleSpriteSelection::Random,
            0.96,
            0.0,
            false,
            false,
        );
        assert_descriptor(
            "minecraft:small_flame",
            "FlameParticle.SmallFlameProvider",
            ParticleLifetimeDescriptor::Rising,
            ParticleSpriteSelection::Random,
            0.96,
            0.0,
            false,
            false,
        );
        assert_descriptor(
            "minecraft:smoke",
            "SmokeParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 8,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Age,
            0.96,
            -0.1,
            true,
            true,
        );
        assert_descriptor(
            "minecraft:large_smoke",
            "LargeSmokeParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 8,
                scale_tenths: 25,
            },
            ParticleSpriteSelection::Age,
            0.96,
            -0.1,
            true,
            true,
        );
        assert_descriptor(
            "minecraft:white_smoke",
            "WhiteSmokeParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 8,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Age,
            0.96,
            -0.1,
            true,
            true,
        );
        assert_descriptor(
            "minecraft:ash",
            "AshParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Age,
            0.96,
            0.1,
            false,
            true,
        );
        assert_descriptor(
            "minecraft:white_ash",
            "WhiteAshParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Age,
            0.96,
            0.0125,
            false,
            true,
        );
        assert_descriptor(
            "minecraft:poof",
            "ExplodeParticle.Provider",
            ParticleLifetimeDescriptor::Explode,
            ParticleSpriteSelection::Age,
            0.9,
            -0.1,
            true,
            false,
        );
    }

    #[test]
    fn sprite_index_for_age_matches_vanilla_integer_frame_selection() {
        assert_eq!(sprite_index_for_age(8, 0, 20), Some(0));
        assert_eq!(sprite_index_for_age(8, 10, 20), Some(3));
        assert_eq!(sprite_index_for_age(8, 19, 20), Some(6));
        assert_eq!(sprite_index_for_age(8, 20, 20), Some(7));
        assert_eq!(sprite_index_for_age(1, 20, 20), Some(0));
        assert_eq!(sprite_index_for_age(0, 20, 20), None);
    }

    fn assert_descriptor(
        particle_id: &str,
        provider: &'static str,
        lifetime: ParticleLifetimeDescriptor,
        sprite_selection: ParticleSpriteSelection,
        friction: f32,
        gravity: f32,
        has_physics: bool,
        speed_up_when_y_motion_is_blocked: bool,
    ) {
        let descriptor = ParticleDescriptor::for_particle(particle_id);
        assert_eq!(descriptor.provider, provider);
        assert_eq!(descriptor.lifetime, lifetime);
        assert_eq!(descriptor.sprite_selection, sprite_selection);
        assert_close_f32(descriptor.friction, friction);
        assert_close_f32(descriptor.gravity, gravity);
        assert_eq!(descriptor.has_physics, has_physics);
        assert_eq!(
            descriptor.speed_up_when_y_motion_is_blocked,
            speed_up_when_y_motion_is_blocked
        );
    }

    fn assert_close_f32(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }
}
