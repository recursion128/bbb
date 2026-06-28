use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_PARTICLE_RANDOM_SEED: i64 = 0x5EED_2601;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParticleDescriptor {
    pub(crate) provider: &'static str,
    pub(crate) lifetime: ParticleLifetimeDescriptor,
    pub(crate) sprite_selection: ParticleSpriteSelection,
    pub(crate) visual: ParticleVisualDescriptor,
    pub(crate) initial_velocity: ParticleInitialVelocityDescriptor,
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
    RandomInclusive {
        min: u32,
        max: u32,
    },
    Explode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleSpriteSelection {
    First,
    Random,
    Age,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParticleVisualDescriptor {
    BaseSingleQuad,
    PlayerCloud,
    Flame {
        scale: f32,
    },
    BaseAshSmoke {
        scale: f32,
        color: ParticleColorDescriptor,
    },
    SuspendedTown {
        color: ParticleColorDescriptor,
    },
    Explode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParticleColorDescriptor {
    RandomGray { max: f32 },
    RandomRgbRange { min: [f32; 3], max: [f32; 3] },
    FixedRgb([f32; 3]),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleQuadSizeCurve {
    #[default]
    Constant,
    GrowToBase,
    Flame,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParticleInitialVelocityDescriptor {
    Command,
    ParticleConstructorScaled { scale: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParticleVisualState {
    pub(crate) base_quad_size: f32,
    pub(crate) color: [f32; 4],
    pub(crate) quad_size_curve: ParticleQuadSizeCurve,
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
                visual: ParticleVisualDescriptor::PlayerCloud,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dragon_breath" => Self {
                provider: "DragonBreathParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::RandomRgbRange {
                        min: DRAGON_BREATH_COLOR_MIN,
                        max: DRAGON_BREATH_COLOR_MAX,
                    },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                    visual: ParticleVisualDescriptor::Flame { scale: 1.0 },
                    initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::Flame { scale: 0.5 },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 2.5,
                    color: ParticleColorDescriptor::RandomGray { max: 0.3 },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::RandomGray { max: 0.3 },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb(WHITE_ASH_SMOKE_RGB),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::RandomGray { max: 0.5 },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
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
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb(WHITE_ASH_SMOKE_RGB),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0125,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:happy_villager" => Self {
                provider: "SuspendedTownParticle.HappyVillagerProvider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SuspendedTown {
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.02,
                },
                friction: 0.99,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:composter" => Self {
                provider: "SuspendedTownParticle.ComposterFillProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 3, max: 7 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SuspendedTown {
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.02,
                },
                friction: 0.99,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:poof" => Self {
                provider: "ExplodeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Explode,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::Explode,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.9,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            _ => Self {
                provider: "Particle",
                lifetime: ParticleLifetimeDescriptor::BaseParticle,
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
        }
    }
}

impl ParticleVisualDescriptor {
    pub(crate) fn sample(self, random: &mut ParticleRandom) -> ParticleVisualState {
        let base_quad_size = sample_single_quad_size(random);
        match self {
            Self::BaseSingleQuad => ParticleVisualState::new(
                base_quad_size,
                WHITE_PARTICLE_COLOR,
                ParticleQuadSizeCurve::Constant,
            ),
            Self::PlayerCloud => {
                let color = 1.0 - random.next_f32() * 0.3;
                ParticleVisualState::new(
                    base_quad_size * 1.875,
                    [color, color, color, 1.0],
                    ParticleQuadSizeCurve::GrowToBase,
                )
            }
            Self::Flame { scale } => ParticleVisualState::new(
                base_quad_size * scale,
                WHITE_PARTICLE_COLOR,
                ParticleQuadSizeCurve::Flame,
            ),
            Self::BaseAshSmoke { scale, color } => ParticleVisualState::new(
                base_quad_size * 0.75 * scale,
                color.sample(random),
                ParticleQuadSizeCurve::GrowToBase,
            ),
            Self::SuspendedTown { color } => {
                let _base_tint = random.next_f32() * 0.1 + 0.2;
                let scale = random.next_f32() * 0.6 + 0.5;
                ParticleVisualState::new(
                    base_quad_size * scale,
                    color.sample(random),
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::Explode => {
                let color = random.next_f32() * 0.3 + 0.7;
                let base_quad_size = 0.1 * (random.next_f32() * random.next_f32() * 6.0 + 1.0);
                ParticleVisualState::new(
                    base_quad_size,
                    [color, color, color, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
        }
    }
}

impl ParticleInitialVelocityDescriptor {
    pub(crate) fn sample(
        self,
        command_velocity: [f64; 3],
        random: &mut ParticleRandom,
    ) -> [f64; 3] {
        match self {
            Self::Command => command_velocity,
            Self::ParticleConstructorScaled { scale } => {
                let x = command_velocity[0] + random_signed_velocity(random);
                let y = command_velocity[1] + random_signed_velocity(random);
                let z = command_velocity[2] + random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                if length == 0.0 {
                    return [0.0, 0.1 * scale, 0.0];
                }
                [
                    x / length * speed * 0.4 * scale,
                    (y / length * speed * 0.4 + 0.1) * scale,
                    z / length * speed * 0.4 * scale,
                ]
            }
        }
    }
}

impl ParticleColorDescriptor {
    fn sample(self, random: &mut ParticleRandom) -> [f32; 4] {
        match self {
            Self::RandomGray { max } => {
                let color = random.next_f32() * max;
                [color, color, color, 1.0]
            }
            Self::RandomRgbRange { min, max } => [
                sample_range(random, min[0], max[0]),
                sample_range(random, min[1], max[1]),
                sample_range(random, min[2], max[2]),
                1.0,
            ],
            Self::FixedRgb([red, green, blue]) => [red, green, blue, 1.0],
        }
    }
}

impl ParticleVisualState {
    fn new(base_quad_size: f32, color: [f32; 4], quad_size_curve: ParticleQuadSizeCurve) -> Self {
        Self {
            base_quad_size,
            color,
            quad_size_curve,
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
            Self::RandomInclusive { min, max } => {
                let span = max.saturating_sub(min).saturating_add(1);
                min + random.next_index(span as usize).unwrap_or(0) as u32
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
const WHITE_PARTICLE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const WHITE_ASH_SMOKE_RGB: [f32; 3] = [186.0 / 255.0, 177.0 / 255.0, 194.0 / 255.0];
const DRAGON_BREATH_COLOR_MIN: [f32; 3] = [0.717_647_1, 0.0, 0.823_529_4];
const DRAGON_BREATH_COLOR_MAX: [f32; 3] = [0.874_509_8, 0.0, 0.976_470_6];

impl ParticleRandom {
    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
        }
    }

    fn next_f64(&mut self) -> f64 {
        f64::from(self.next_bits(24)) / f64::from(1_u32 << 24)
    }

    fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
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

fn sample_single_quad_size(random: &mut ParticleRandom) -> f32 {
    0.1 * (random.next_f32() * 0.5 + 0.5) * 2.0
}

fn sample_range(random: &mut ParticleRandom, min: f32, max: f32) -> f32 {
    min + random.next_f32() * (max - min)
}

fn random_signed_velocity(random: &mut ParticleRandom) -> f64 {
    (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4
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
            ParticleVisualDescriptor::PlayerCloud,
            0.96,
            0.0,
            false,
            false,
        );
        assert_descriptor(
            "minecraft:dragon_breath",
            "DragonBreathParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 1.0,
                color: ParticleColorDescriptor::RandomRgbRange {
                    min: DRAGON_BREATH_COLOR_MIN,
                    max: DRAGON_BREATH_COLOR_MAX,
                },
            },
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
            ParticleVisualDescriptor::Flame { scale: 1.0 },
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
            ParticleVisualDescriptor::Flame { scale: 0.5 },
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
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 1.0,
                color: ParticleColorDescriptor::RandomGray { max: 0.3 },
            },
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
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 2.5,
                color: ParticleColorDescriptor::RandomGray { max: 0.3 },
            },
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
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 1.0,
                color: ParticleColorDescriptor::FixedRgb(WHITE_ASH_SMOKE_RGB),
            },
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
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 1.0,
                color: ParticleColorDescriptor::RandomGray { max: 0.5 },
            },
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
            ParticleVisualDescriptor::BaseAshSmoke {
                scale: 1.0,
                color: ParticleColorDescriptor::FixedRgb(WHITE_ASH_SMOKE_RGB),
            },
            0.96,
            0.0125,
            false,
            true,
        );
        assert_descriptor(
            "minecraft:happy_villager",
            "SuspendedTownParticle.HappyVillagerProvider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SuspendedTown {
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            },
            0.99,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:happy_villager").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
        );
        assert_descriptor(
            "minecraft:composter",
            "SuspendedTownParticle.ComposterFillProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 3, max: 7 },
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SuspendedTown {
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            },
            0.99,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:composter").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
        );
        assert_descriptor(
            "minecraft:poof",
            "ExplodeParticle.Provider",
            ParticleLifetimeDescriptor::Explode,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::Explode,
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

    #[test]
    fn visual_descriptors_sample_vanilla_shaped_size_color_and_curves() {
        let mut flame_random = ParticleRandom::new(7);
        let flame = ParticleVisualDescriptor::Flame { scale: 1.0 }.sample(&mut flame_random);
        let mut small_flame_random = ParticleRandom::new(7);
        let small_flame =
            ParticleVisualDescriptor::Flame { scale: 0.5 }.sample(&mut small_flame_random);
        assert_close_f32(small_flame.base_quad_size, flame.base_quad_size * 0.5);
        assert_eq!(flame.color, WHITE_PARTICLE_COLOR);
        assert_eq!(flame.quad_size_curve, ParticleQuadSizeCurve::Flame);

        let mut cloud_random = ParticleRandom::new(8);
        let cloud = ParticleVisualDescriptor::PlayerCloud.sample(&mut cloud_random);
        assert_range_f32(cloud.base_quad_size, 0.1875, 0.375);
        assert_range_f32(cloud.color[0], 0.7, 1.0);
        assert_eq!(cloud.color[0], cloud.color[1]);
        assert_eq!(cloud.color[1], cloud.color[2]);
        assert_eq!(cloud.color[3], 1.0);
        assert_eq!(cloud.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut dragon_random = ParticleRandom::new(12);
        let dragon_breath = ParticleVisualDescriptor::BaseAshSmoke {
            scale: 1.0,
            color: ParticleColorDescriptor::RandomRgbRange {
                min: DRAGON_BREATH_COLOR_MIN,
                max: DRAGON_BREATH_COLOR_MAX,
            },
        }
        .sample(&mut dragon_random);
        assert_range_f32(dragon_breath.base_quad_size, 0.075, 0.15);
        assert_range_f32(
            dragon_breath.color[0],
            DRAGON_BREATH_COLOR_MIN[0],
            DRAGON_BREATH_COLOR_MAX[0],
        );
        assert_eq!(dragon_breath.color[1], 0.0);
        assert_range_f32(
            dragon_breath.color[2],
            DRAGON_BREATH_COLOR_MIN[2],
            DRAGON_BREATH_COLOR_MAX[2],
        );
        assert_eq!(
            dragon_breath.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );

        let mut happy_villager_random = ParticleRandom::new(13);
        let happy_villager = ParticleVisualDescriptor::SuspendedTown {
            color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
        }
        .sample(&mut happy_villager_random);
        assert_range_f32(happy_villager.base_quad_size, 0.05, 0.22);
        assert_eq!(happy_villager.color, WHITE_PARTICLE_COLOR);
        assert_eq!(
            happy_villager.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );

        let mut smoke_random = ParticleRandom::new(9);
        let smoke = ParticleVisualDescriptor::BaseAshSmoke {
            scale: 2.5,
            color: ParticleColorDescriptor::RandomGray { max: 0.3 },
        }
        .sample(&mut smoke_random);
        assert_range_f32(smoke.base_quad_size, 0.1875, 0.375);
        assert_range_f32(smoke.color[0], 0.0, 0.3);
        assert_eq!(smoke.color[0], smoke.color[1]);
        assert_eq!(smoke.color[1], smoke.color[2]);
        assert_eq!(smoke.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut white_smoke_random = ParticleRandom::new(10);
        let white_smoke = ParticleVisualDescriptor::BaseAshSmoke {
            scale: 1.0,
            color: ParticleColorDescriptor::FixedRgb(WHITE_ASH_SMOKE_RGB),
        }
        .sample(&mut white_smoke_random);
        assert_eq!(
            white_smoke.color,
            [
                WHITE_ASH_SMOKE_RGB[0],
                WHITE_ASH_SMOKE_RGB[1],
                WHITE_ASH_SMOKE_RGB[2],
                1.0,
            ]
        );

        let mut poof_random = ParticleRandom::new(11);
        let poof = ParticleVisualDescriptor::Explode.sample(&mut poof_random);
        assert_range_f32(poof.base_quad_size, 0.1, 0.7);
        assert_range_f32(poof.color[0], 0.7, 1.0);
        assert_eq!(poof.quad_size_curve, ParticleQuadSizeCurve::Constant);
    }

    #[test]
    fn initial_velocity_descriptor_matches_vanilla_particle_constructor_scaling() {
        let mut random = ParticleRandom::new(14);
        let velocity = ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
            .sample([0.0, 0.0, 0.0], &mut random);

        assert_range_f64(velocity[0], -0.004, 0.004);
        assert_range_f64(velocity[1], -0.002, 0.006);
        assert_range_f64(velocity[2], -0.004, 0.004);
        assert_ne!(velocity, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn random_inclusive_lifetime_samples_configured_range() {
        let mut random = ParticleRandom::new(15);
        for _ in 0..32 {
            let lifetime =
                ParticleLifetimeDescriptor::RandomInclusive { min: 3, max: 7 }.sample(&mut random);
            assert!((3..=7).contains(&lifetime));
        }
    }

    fn assert_descriptor(
        particle_id: &str,
        provider: &'static str,
        lifetime: ParticleLifetimeDescriptor,
        sprite_selection: ParticleSpriteSelection,
        visual: ParticleVisualDescriptor,
        friction: f32,
        gravity: f32,
        has_physics: bool,
        speed_up_when_y_motion_is_blocked: bool,
    ) {
        let descriptor = ParticleDescriptor::for_particle(particle_id);
        assert_eq!(descriptor.provider, provider);
        assert_eq!(descriptor.lifetime, lifetime);
        assert_eq!(descriptor.sprite_selection, sprite_selection);
        assert_eq!(descriptor.visual, visual);
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

    fn assert_range_f32(actual: f32, min: f32, max: f32) {
        assert!(
            actual >= min && actual <= max,
            "expected {actual} to be in {min}..={max}"
        );
    }

    fn assert_range_f64(actual: f64, min: f64, max: f64) {
        assert!(
            actual >= min && actual <= max,
            "expected {actual} to be in {min}..={max}"
        );
    }
}
