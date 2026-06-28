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
    Fixed(u32),
    Rising,
    PlayerCloud,
    BaseAshSmoke {
        max_lifetime: u32,
        scale_tenths: u32,
    },
    BaseAshSmokeDivided {
        max_lifetime: u32,
        scale_tenths: u32,
        divisor: u32,
    },
    Crit,
    EightOverRandom,
    FortyOverRandom,
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
    PlayerCloudTint {
        color: ParticleColorDescriptor,
    },
    Bubble,
    Note,
    SingleQuadScaled {
        scale: f32,
        color: ParticleColorDescriptor,
        quad_size_curve: ParticleQuadSizeCurve,
    },
    WitchSpell,
    Crit {
        color_scale: [f32; 3],
    },
    Flame {
        scale: f32,
    },
    FixedQuad {
        size: f32,
        color: ParticleColorDescriptor,
    },
    HugeExplosion,
    BaseAshSmoke {
        scale: f32,
        color: ParticleColorDescriptor,
    },
    SuspendedTown {
        color: SuspendedTownColorDescriptor,
    },
    Explode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParticleColorDescriptor {
    RandomGray {
        max: f32,
    },
    RandomGrayRange {
        min: f32,
        max: f32,
    },
    RandomRgbRange {
        min: [f32; 3],
        max: [f32; 3],
    },
    FixedRgbRandomAlpha {
        rgb: [f32; 3],
        min_alpha: f32,
        max_alpha: f32,
    },
    FixedRgbChoice {
        first: [f32; 3],
        second: [f32; 3],
    },
    FixedRgba([f32; 4]),
    FixedRgb([f32; 3]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SuspendedTownColorDescriptor {
    BaseGray,
    Override(ParticleColorDescriptor),
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
    Zero,
    Command,
    CommandScaledPlusRandom {
        command_scale: f64,
        random_range: f64,
    },
    CommandAxisScaled {
        scale: [f64; 3],
    },
    RisingParticle,
    ParticleConstructorScaled {
        scale: f64,
    },
    ParticleConstructorZeroScaledPlusCommand {
        scale: f64,
    },
    ParticleConstructorZeroScaledPlusScaledCommand {
        random_scale: f64,
        command_scale: f64,
        command_y_offset: f64,
    },
    ParticleConstructorZeroScaledWithYOffset {
        scale: f64,
        y_offset: f64,
    },
    Spell,
    GlowSquid,
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
            "minecraft:angry_villager" => Self {
                provider: "HeartParticle.AngryVillagerProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(16),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.5,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                        scale: 0.01,
                        y_offset: 0.1,
                    },
                friction: 0.86,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:bubble" => Self {
                provider: "BubbleParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Bubble,
                initial_velocity: ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                    command_scale: 0.2,
                    random_range: 0.02,
                },
                friction: 0.85,
                gravity: -0.05,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:bubble_column_up" => Self {
                provider: "BubbleColumnUpParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::FortyOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Bubble,
                initial_velocity: ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                    command_scale: 0.2,
                    random_range: 0.02,
                },
                friction: 0.85,
                gravity: -0.125,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:cloud" => Self {
                provider: "PlayerCloudParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::PlayerCloud,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::PlayerCloud,
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                        scale: 0.1,
                    },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:sneeze" => Self {
                provider: "PlayerCloudParticle.SneezeProvider",
                lifetime: ParticleLifetimeDescriptor::PlayerCloud,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::PlayerCloudTint {
                    color: ParticleColorDescriptor::FixedRgba([0.22, 1.0, 0.53, 0.4]),
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                        scale: 0.1,
                    },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:crit" => Self {
                provider: "CritParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Crit,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Crit {
                    color_scale: CRIT_COLOR_SCALE,
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                        random_scale: 0.1,
                        command_scale: 0.4,
                        command_y_offset: 0.0,
                    },
                friction: 0.7,
                gravity: 0.5,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:damage_indicator" => Self {
                provider: "CritParticle.DamageIndicatorProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(20),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Crit {
                    color_scale: CRIT_COLOR_SCALE,
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                        random_scale: 0.1,
                        command_scale: 0.4,
                        command_y_offset: 1.0,
                    },
                friction: 0.7,
                gravity: 0.5,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:enchanted_hit" => Self {
                provider: "CritParticle.MagicProvider",
                lifetime: ParticleLifetimeDescriptor::Crit,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Crit {
                    color_scale: MAGIC_CRIT_COLOR_SCALE,
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                        random_scale: 0.1,
                        command_scale: 0.4,
                        command_y_offset: 0.0,
                    },
                friction: 0.7,
                gravity: 0.5,
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
            "minecraft:explosion" => Self {
                provider: "HugeExplosionParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 6, max: 9 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::HugeExplosion,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:sonic_boom" => Self {
                provider: "SonicBoomParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(16),
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 1.5,
                    color: ParticleColorDescriptor::RandomGrayRange { min: 0.4, max: 1.0 },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:gust" => Self {
                provider: "GustParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 12, max: 15 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:small_gust" => Self {
                provider: "GustParticle.SmallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 12, max: 15 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.15,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dolphin" => Self {
                provider: "SuspendedTownParticle.DolphinSpeedProvider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmokeDivided {
                    max_lifetime: 20,
                    scale_tenths: 10,
                    divisor: 2,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SuspendedTown {
                    color: SuspendedTownColorDescriptor::Override(
                        ParticleColorDescriptor::FixedRgbRandomAlpha {
                            rgb: [0.3, 0.5, 1.0],
                            min_alpha: 0.3,
                            max_alpha: 1.0,
                        },
                    ),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.02,
                },
                friction: 0.99,
                gravity: 0.0,
                has_physics: true,
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
            "minecraft:soul" | "minecraft:sculk_soul" => Self {
                provider: if particle_id == "minecraft:sculk_soul" {
                    "SoulParticle.EmissiveProvider"
                } else {
                    "SoulParticle.Provider"
                },
                lifetime: ParticleLifetimeDescriptor::Rising,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.5,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::RisingParticle,
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:glow" => Self {
                provider: "GlowParticle.GlowSquidProvider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgbChoice {
                        first: [0.6, 1.0, 0.8],
                        second: [0.08, 0.4, 0.4],
                    },
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::GlowSquid,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:electric_spark" => Self {
                provider: "GlowParticle.ElectricSparkProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 2, max: 3 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.9, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CommandAxisScaled {
                    scale: [0.25, 0.25, 0.25],
                },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:scrape" => Self {
                provider: "GlowParticle.ScrapeProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgbChoice {
                        first: [0.29, 0.58, 0.51],
                        second: [0.43, 0.77, 0.62],
                    },
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CommandAxisScaled {
                    scale: [0.01, 0.01, 0.01],
                },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:wax_off" => Self {
                provider: "GlowParticle.WaxOffProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.9, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CommandAxisScaled {
                    scale: [0.005, 0.01, 0.005],
                },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:wax_on" => Self {
                provider: "GlowParticle.WaxOnProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([0.91, 0.55, 0.08]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CommandAxisScaled {
                    scale: [0.005, 0.01, 0.005],
                },
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:heart" => Self {
                provider: "HeartParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(16),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.5,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
                },
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                        scale: 0.01,
                        y_offset: 0.1,
                    },
                friction: 0.86,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:infested" | "minecraft:raid_omen" | "minecraft:trial_omen" => Self {
                provider: "SpellParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Spell,
                friction: 0.96,
                gravity: -0.1,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:witch" => Self {
                provider: "SpellParticle.WitchProvider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::WitchSpell,
                initial_velocity: ParticleInitialVelocityDescriptor::Spell,
                friction: 0.96,
                gravity: -0.1,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
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
                    color: SuspendedTownColorDescriptor::Override(
                        ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    ),
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
                    color: SuspendedTownColorDescriptor::Override(
                        ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    ),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.02,
                },
                friction: 0.99,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:mycelium" => Self {
                provider: "SuspendedTownParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SuspendedTown {
                    color: SuspendedTownColorDescriptor::BaseGray,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.02,
                },
                friction: 0.99,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:note" => Self {
                provider: "NoteParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(6),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Note,
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                        scale: 0.01,
                        y_offset: 0.2,
                    },
                friction: 0.66,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:egg_crack" => Self {
                provider: "SuspendedTownParticle.EggCrackProvider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 20,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SuspendedTown {
                    color: SuspendedTownColorDescriptor::Override(
                        ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    ),
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

    pub(crate) fn initial_position(
        self,
        command_position: [f64; 3],
        random: &mut ParticleRandom,
    ) -> [f64; 3] {
        match self.provider {
            "HeartParticle.AngryVillagerProvider" => [
                command_position[0],
                command_position[1] + 0.5,
                command_position[2],
            ],
            "SoulParticle.Provider" | "SoulParticle.EmissiveProvider" => [
                command_position[0] + random_centered_offset(random, 0.05),
                command_position[1] + random_centered_offset(random, 0.05),
                command_position[2] + random_centered_offset(random, 0.05),
            ],
            _ => command_position,
        }
    }
}

impl ParticleVisualDescriptor {
    pub(crate) fn sample_for_command(
        self,
        random: &mut ParticleRandom,
        command_velocity: [f64; 3],
    ) -> ParticleVisualState {
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
            Self::PlayerCloudTint { color } => ParticleVisualState::new(
                base_quad_size * 1.875,
                color.sample(random),
                ParticleQuadSizeCurve::GrowToBase,
            ),
            Self::Bubble => {
                let scale = random.next_f32() * 0.6 + 0.2;
                ParticleVisualState::new(
                    base_quad_size * scale,
                    WHITE_PARTICLE_COLOR,
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::Note => ParticleVisualState::new(
                base_quad_size * 1.5,
                note_color(command_velocity[0] as f32),
                ParticleQuadSizeCurve::GrowToBase,
            ),
            Self::SingleQuadScaled {
                scale,
                color,
                quad_size_curve,
            } => ParticleVisualState::new(
                base_quad_size * scale,
                color.sample(random),
                quad_size_curve,
            ),
            Self::WitchSpell => {
                let brightness = random.next_f32() * 0.5 + 0.35;
                ParticleVisualState::new(
                    base_quad_size * 0.75,
                    [brightness, 0.0, brightness, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::Crit { color_scale } => {
                let color = random.next_f32() * 0.3 + 0.6;
                ParticleVisualState::new(
                    base_quad_size * 0.75,
                    [
                        color * color_scale[0],
                        color * color_scale[1],
                        color * color_scale[2],
                        1.0,
                    ],
                    ParticleQuadSizeCurve::GrowToBase,
                )
            }
            Self::Flame { scale } => ParticleVisualState::new(
                base_quad_size * scale,
                WHITE_PARTICLE_COLOR,
                ParticleQuadSizeCurve::Flame,
            ),
            Self::FixedQuad { size, color } => ParticleVisualState::new(
                size,
                color.sample(random),
                ParticleQuadSizeCurve::Constant,
            ),
            Self::HugeExplosion => {
                let color = random.next_f32() * 0.6 + 0.4;
                let size = 2.0 * (1.0 - command_velocity[0] as f32 * 0.5);
                ParticleVisualState::new(
                    size,
                    [color, color, color, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::BaseAshSmoke { scale, color } => ParticleVisualState::new(
                base_quad_size * 0.75 * scale,
                color.sample(random),
                ParticleQuadSizeCurve::GrowToBase,
            ),
            Self::SuspendedTown { color } => {
                let base_tint = random.next_f32() * 0.1 + 0.2;
                let scale = random.next_f32() * 0.6 + 0.5;
                ParticleVisualState::new(
                    base_quad_size * scale,
                    color.sample(base_tint, random),
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

impl SuspendedTownColorDescriptor {
    fn sample(self, base_tint: f32, random: &mut ParticleRandom) -> [f32; 4] {
        match self {
            Self::BaseGray => [base_tint, base_tint, base_tint, 1.0],
            Self::Override(color) => color.sample(random),
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
            Self::Zero => [0.0, 0.0, 0.0],
            Self::Command => command_velocity,
            Self::CommandScaledPlusRandom {
                command_scale,
                random_range,
            } => [
                command_velocity[0] * command_scale + random_signed_unit(random) * random_range,
                command_velocity[1] * command_scale + random_signed_unit(random) * random_range,
                command_velocity[2] * command_scale + random_signed_unit(random) * random_range,
            ],
            Self::CommandAxisScaled { scale } => [
                command_velocity[0] * scale[0],
                command_velocity[1] * scale[1],
                command_velocity[2] * scale[2],
            ],
            Self::RisingParticle => {
                let x = command_velocity[0] + random_signed_velocity(random);
                let y = command_velocity[1] + random_signed_velocity(random);
                let z = command_velocity[2] + random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                let random_velocity = if length == 0.0 {
                    [0.0, 0.1, 0.0]
                } else {
                    [
                        x / length * speed * 0.4,
                        y / length * speed * 0.4 + 0.1,
                        z / length * speed * 0.4,
                    ]
                };
                [
                    random_velocity[0] * 0.01 + command_velocity[0],
                    random_velocity[1] * 0.01 + command_velocity[1],
                    random_velocity[2] * 0.01 + command_velocity[2],
                ]
            }
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
            Self::ParticleConstructorZeroScaledPlusCommand { scale } => {
                let x = random_signed_velocity(random);
                let y = random_signed_velocity(random);
                let z = random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                if length == 0.0 {
                    return [
                        command_velocity[0],
                        command_velocity[1] + 0.1 * scale,
                        command_velocity[2],
                    ];
                }
                [
                    command_velocity[0] + x / length * speed * 0.4 * scale,
                    command_velocity[1] + (y / length * speed * 0.4 + 0.1) * scale,
                    command_velocity[2] + z / length * speed * 0.4 * scale,
                ]
            }
            Self::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale,
                command_scale,
                command_y_offset,
            } => {
                let x = random_signed_velocity(random);
                let y = random_signed_velocity(random);
                let z = random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                let random_velocity = if length == 0.0 {
                    [0.0, 0.1, 0.0]
                } else {
                    [
                        x / length * speed * 0.4,
                        y / length * speed * 0.4 + 0.1,
                        z / length * speed * 0.4,
                    ]
                };
                [
                    random_velocity[0] * random_scale + command_velocity[0] * command_scale,
                    random_velocity[1] * random_scale
                        + (command_velocity[1] + command_y_offset) * command_scale,
                    random_velocity[2] * random_scale + command_velocity[2] * command_scale,
                ]
            }
            Self::ParticleConstructorZeroScaledWithYOffset { scale, y_offset } => {
                let x = random_signed_velocity(random);
                let y = random_signed_velocity(random);
                let z = random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                if length == 0.0 {
                    return [0.0, y_offset, 0.0];
                }
                [
                    x / length * speed * 0.4 * scale,
                    y / length * speed * 0.4 * scale + y_offset,
                    z / length * speed * 0.4 * scale,
                ]
            }
            Self::Spell | Self::GlowSquid => {
                sample_random_horizontal_y_velocity(command_velocity, random)
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
            Self::RandomGrayRange { min, max } => {
                let color = sample_range(random, min, max);
                [color, color, color, 1.0]
            }
            Self::RandomRgbRange { min, max } => [
                sample_range(random, min[0], max[0]),
                sample_range(random, min[1], max[1]),
                sample_range(random, min[2], max[2]),
                1.0,
            ],
            Self::FixedRgbRandomAlpha {
                rgb: [red, green, blue],
                min_alpha,
                max_alpha,
            } => [red, green, blue, sample_range(random, min_alpha, max_alpha)],
            Self::FixedRgbChoice { first, second } => {
                let [red, green, blue] = if random.next_bool() { first } else { second };
                [red, green, blue, 1.0]
            }
            Self::FixedRgba(rgba) => rgba,
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
            Self::Fixed(lifetime) => lifetime,
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
            Self::BaseAshSmokeDivided {
                max_lifetime,
                scale_tenths,
                divisor,
            } => {
                let scale = f64::from(scale_tenths) / 10.0;
                let lifetime =
                    (f64::from(max_lifetime) / (random.next_f64() * 0.8 + 0.2) * scale) as u32;
                lifetime.max(1) / divisor.max(1)
            }
            Self::Crit => ((6.0 / (random.next_f32() * 0.8 + 0.6)) as u32).max(1),
            Self::EightOverRandom => ((8.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
            Self::FortyOverRandom => ((40.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
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
const CRIT_COLOR_SCALE: [f32; 3] = [1.0, 0.96, 0.9];
const MAGIC_CRIT_COLOR_SCALE: [f32; 3] = [0.3, 0.768, 0.9];

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

    fn next_bool(&mut self) -> bool {
        self.next_bits(1) != 0
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

fn note_color(color: f32) -> [f32; 4] {
    [
        note_color_component(color, 0.0),
        note_color_component(color, 1.0 / 3.0),
        note_color_component(color, 2.0 / 3.0),
        1.0,
    ]
}

fn note_color_component(color: f32, offset: f32) -> f32 {
    ((color + offset) * std::f32::consts::TAU)
        .sin()
        .mul_add(0.65, 0.35)
        .max(0.0)
}

fn random_signed_velocity(random: &mut ParticleRandom) -> f64 {
    random_signed_unit(random) * 0.4
}

fn random_signed_unit(random: &mut ParticleRandom) -> f64 {
    f64::from(random.next_f32()) * 2.0 - 1.0
}

fn random_centered_offset(random: &mut ParticleRandom, scale: f64) -> f64 {
    (f64::from(random.next_f32()) - f64::from(random.next_f32())) * scale
}

fn sample_random_horizontal_y_velocity(
    command_velocity: [f64; 3],
    random: &mut ParticleRandom,
) -> [f64; 3] {
    let x = 0.5 - random.next_f64();
    let y = command_velocity[1];
    let z = 0.5 - random.next_f64();
    let x = x + random_signed_velocity(random);
    let y = y + random_signed_velocity(random);
    let z = z + random_signed_velocity(random);
    let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
    let length = (x * x + y * y + z * z).sqrt();
    let mut velocity = if length == 0.0 {
        [0.0, 0.1, 0.0]
    } else {
        [
            x / length * speed * 0.4,
            y / length * speed * 0.4 + 0.1,
            z / length * speed * 0.4,
        ]
    };
    velocity[1] *= 0.2;
    if command_velocity[0] == 0.0 && command_velocity[2] == 0.0 {
        velocity[0] *= 0.1;
        velocity[2] *= 0.1;
    }
    velocity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_descriptor_maps_core_vanilla_providers_and_physics_flags() {
        assert_descriptor(
            "minecraft:angry_villager",
            "HeartParticle.AngryVillagerProvider",
            ParticleLifetimeDescriptor::Fixed(16),
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 1.5,
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
            },
            0.86,
            0.0,
            false,
            true,
        );
        let angry_villager = ParticleDescriptor::for_particle("minecraft:angry_villager");
        assert_eq!(
            angry_villager.initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                scale: 0.01,
                y_offset: 0.1
            }
        );
        assert_eq!(
            angry_villager.initial_position([1.0, 2.0, 3.0], &mut ParticleRandom::new(1)),
            [1.0, 2.5, 3.0]
        );

        assert_descriptor(
            "minecraft:bubble",
            "BubbleParticle.Provider",
            ParticleLifetimeDescriptor::EightOverRandom,
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Bubble,
            0.85,
            -0.05,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:bubble").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                command_scale: 0.2,
                random_range: 0.02,
            }
        );
        assert_descriptor(
            "minecraft:bubble_column_up",
            "BubbleColumnUpParticle.Provider",
            ParticleLifetimeDescriptor::FortyOverRandom,
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Bubble,
            0.85,
            -0.125,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:bubble_column_up").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                command_scale: 0.2,
                random_range: 0.02,
            }
        );

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
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:cloud").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                scale: 0.1
            }
        );
        assert_descriptor(
            "minecraft:sneeze",
            "PlayerCloudParticle.SneezeProvider",
            ParticleLifetimeDescriptor::PlayerCloud,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::PlayerCloudTint {
                color: ParticleColorDescriptor::FixedRgba([0.22, 1.0, 0.53, 0.4]),
            },
            0.96,
            0.0,
            false,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:sneeze").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                scale: 0.1
            }
        );
        assert_descriptor(
            "minecraft:crit",
            "CritParticle.Provider",
            ParticleLifetimeDescriptor::Crit,
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Crit {
                color_scale: CRIT_COLOR_SCALE,
            },
            0.7,
            0.5,
            false,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:crit").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale: 0.1,
                command_scale: 0.4,
                command_y_offset: 0.0,
            }
        );
        assert_descriptor(
            "minecraft:damage_indicator",
            "CritParticle.DamageIndicatorProvider",
            ParticleLifetimeDescriptor::Fixed(20),
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Crit {
                color_scale: CRIT_COLOR_SCALE,
            },
            0.7,
            0.5,
            false,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:damage_indicator").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale: 0.1,
                command_scale: 0.4,
                command_y_offset: 1.0,
            }
        );
        assert_descriptor(
            "minecraft:enchanted_hit",
            "CritParticle.MagicProvider",
            ParticleLifetimeDescriptor::Crit,
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Crit {
                color_scale: MAGIC_CRIT_COLOR_SCALE,
            },
            0.7,
            0.5,
            false,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:enchanted_hit").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale: 0.1,
                command_scale: 0.4,
                command_y_offset: 0.0,
            }
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
            "minecraft:explosion",
            "HugeExplosionParticle.Provider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 6, max: 9 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::HugeExplosion,
            0.98,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:explosion").initial_velocity,
            ParticleInitialVelocityDescriptor::Zero
        );
        assert_descriptor(
            "minecraft:sonic_boom",
            "SonicBoomParticle.Provider",
            ParticleLifetimeDescriptor::Fixed(16),
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::FixedQuad {
                size: 1.5,
                color: ParticleColorDescriptor::RandomGrayRange { min: 0.4, max: 1.0 },
            },
            0.98,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:sonic_boom").initial_velocity,
            ParticleInitialVelocityDescriptor::Zero
        );
        assert_descriptor(
            "minecraft:gust",
            "GustParticle.Provider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 12, max: 15 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::FixedQuad {
                size: 1.0,
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            },
            0.98,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:gust").initial_velocity,
            ParticleInitialVelocityDescriptor::Zero
        );
        assert_descriptor(
            "minecraft:small_gust",
            "GustParticle.SmallProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 12, max: 15 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::FixedQuad {
                size: 0.15,
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            },
            0.98,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:small_gust").initial_velocity,
            ParticleInitialVelocityDescriptor::Zero
        );
        assert_descriptor(
            "minecraft:dolphin",
            "SuspendedTownParticle.DolphinSpeedProvider",
            ParticleLifetimeDescriptor::BaseAshSmokeDivided {
                max_lifetime: 20,
                scale_tenths: 10,
                divisor: 2,
            },
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SuspendedTown {
                color: SuspendedTownColorDescriptor::Override(
                    ParticleColorDescriptor::FixedRgbRandomAlpha {
                        rgb: [0.3, 0.5, 1.0],
                        min_alpha: 0.3,
                        max_alpha: 1.0,
                    },
                ),
            },
            0.99,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:dolphin").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
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
            "minecraft:soul",
            "SoulParticle.Provider",
            ParticleLifetimeDescriptor::Rising,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 1.5,
                color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:soul").initial_velocity,
            ParticleInitialVelocityDescriptor::RisingParticle
        );
        assert_descriptor(
            "minecraft:sculk_soul",
            "SoulParticle.EmissiveProvider",
            ParticleLifetimeDescriptor::Rising,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 1.5,
                color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:sculk_soul").initial_velocity,
            ParticleInitialVelocityDescriptor::RisingParticle
        );
        let soul_position = ParticleDescriptor::for_particle("minecraft:soul")
            .initial_position([1.0, 2.0, 3.0], &mut ParticleRandom::new(35));
        assert_range_f64(soul_position[0], 0.95, 1.05);
        assert_range_f64(soul_position[1], 1.95, 2.05);
        assert_range_f64(soul_position[2], 2.95, 3.05);
        assert_descriptor(
            "minecraft:heart",
            "HeartParticle.Provider",
            ParticleLifetimeDescriptor::Fixed(16),
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 1.5,
                color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
            },
            0.86,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:heart").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                scale: 0.01,
                y_offset: 0.1
            }
        );
        assert_descriptor(
            "minecraft:glow",
            "GlowParticle.GlowSquidProvider",
            ParticleLifetimeDescriptor::EightOverRandom,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 0.75,
                color: ParticleColorDescriptor::FixedRgbChoice {
                    first: [0.6, 1.0, 0.8],
                    second: [0.08, 0.4, 0.4],
                },
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:glow").initial_velocity,
            ParticleInitialVelocityDescriptor::GlowSquid
        );
        assert_descriptor(
            "minecraft:electric_spark",
            "GlowParticle.ElectricSparkProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 2, max: 3 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 0.75,
                color: ParticleColorDescriptor::FixedRgb([1.0, 0.9, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:electric_spark").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandAxisScaled {
                scale: [0.25, 0.25, 0.25],
            }
        );
        assert_descriptor(
            "minecraft:scrape",
            "GlowParticle.ScrapeProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 0.75,
                color: ParticleColorDescriptor::FixedRgbChoice {
                    first: [0.29, 0.58, 0.51],
                    second: [0.43, 0.77, 0.62],
                },
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:scrape").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandAxisScaled {
                scale: [0.01, 0.01, 0.01],
            }
        );
        assert_descriptor(
            "minecraft:wax_off",
            "GlowParticle.WaxOffProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 0.75,
                color: ParticleColorDescriptor::FixedRgb([1.0, 0.9, 1.0]),
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:wax_off").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandAxisScaled {
                scale: [0.005, 0.01, 0.005],
            }
        );
        assert_descriptor(
            "minecraft:wax_on",
            "GlowParticle.WaxOnProvider",
            ParticleLifetimeDescriptor::RandomInclusive { min: 10, max: 39 },
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::SingleQuadScaled {
                scale: 0.75,
                color: ParticleColorDescriptor::FixedRgb([0.91, 0.55, 0.08]),
                quad_size_curve: ParticleQuadSizeCurve::Constant,
            },
            0.96,
            0.0,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:wax_on").initial_velocity,
            ParticleInitialVelocityDescriptor::CommandAxisScaled {
                scale: [0.005, 0.01, 0.005],
            }
        );
        for particle_id in [
            "minecraft:infested",
            "minecraft:raid_omen",
            "minecraft:trial_omen",
        ] {
            assert_descriptor(
                particle_id,
                "SpellParticle.Provider",
                ParticleLifetimeDescriptor::EightOverRandom,
                ParticleSpriteSelection::Age,
                ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                0.96,
                -0.1,
                false,
                true,
            );
            assert_eq!(
                ParticleDescriptor::for_particle(particle_id).initial_velocity,
                ParticleInitialVelocityDescriptor::Spell
            );
        }
        assert_descriptor(
            "minecraft:witch",
            "SpellParticle.WitchProvider",
            ParticleLifetimeDescriptor::EightOverRandom,
            ParticleSpriteSelection::Age,
            ParticleVisualDescriptor::WitchSpell,
            0.96,
            -0.1,
            false,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:witch").initial_velocity,
            ParticleInitialVelocityDescriptor::Spell
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
                color: SuspendedTownColorDescriptor::Override(ParticleColorDescriptor::FixedRgb([
                    1.0, 1.0, 1.0,
                ])),
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
                color: SuspendedTownColorDescriptor::Override(ParticleColorDescriptor::FixedRgb([
                    1.0, 1.0, 1.0,
                ])),
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
            "minecraft:mycelium",
            "SuspendedTownParticle.Provider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SuspendedTown {
                color: SuspendedTownColorDescriptor::BaseGray,
            },
            0.99,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:mycelium").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
        );
        assert_descriptor(
            "minecraft:note",
            "NoteParticle.Provider",
            ParticleLifetimeDescriptor::Fixed(6),
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::Note,
            0.66,
            0.0,
            true,
            true,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:note").initial_velocity,
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                scale: 0.01,
                y_offset: 0.2
            }
        );
        assert_descriptor(
            "minecraft:egg_crack",
            "SuspendedTownParticle.EggCrackProvider",
            ParticleLifetimeDescriptor::BaseAshSmoke {
                max_lifetime: 20,
                scale_tenths: 10,
            },
            ParticleSpriteSelection::Random,
            ParticleVisualDescriptor::SuspendedTown {
                color: SuspendedTownColorDescriptor::Override(ParticleColorDescriptor::FixedRgb([
                    1.0, 1.0, 1.0,
                ])),
            },
            0.99,
            0.0,
            true,
            false,
        );
        assert_eq!(
            ParticleDescriptor::for_particle("minecraft:egg_crack").initial_velocity,
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
        let flame = ParticleVisualDescriptor::Flame { scale: 1.0 }
            .sample_for_command(&mut flame_random, [0.0, 0.0, 0.0]);
        let mut small_flame_random = ParticleRandom::new(7);
        let small_flame = ParticleVisualDescriptor::Flame { scale: 0.5 }
            .sample_for_command(&mut small_flame_random, [0.0, 0.0, 0.0]);
        assert_close_f32(small_flame.base_quad_size, flame.base_quad_size * 0.5);
        assert_eq!(flame.color, WHITE_PARTICLE_COLOR);
        assert_eq!(flame.quad_size_curve, ParticleQuadSizeCurve::Flame);

        let mut explosion_random = ParticleRandom::new(36);
        let explosion = ParticleVisualDescriptor::HugeExplosion
            .sample_for_command(&mut explosion_random, [0.5, 0.0, 0.0]);
        assert_close_f32(explosion.base_quad_size, 1.5);
        assert_range_f32(explosion.color[0], 0.4, 1.0);
        assert_eq!(explosion.color[0], explosion.color[1]);
        assert_eq!(explosion.color[1], explosion.color[2]);
        assert_eq!(explosion.color[3], 1.0);
        assert_eq!(explosion.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut sonic_boom_random = ParticleRandom::new(39);
        let sonic_boom = ParticleVisualDescriptor::FixedQuad {
            size: 1.5,
            color: ParticleColorDescriptor::RandomGrayRange { min: 0.4, max: 1.0 },
        }
        .sample_for_command(&mut sonic_boom_random, [9.0, 9.0, 9.0]);
        assert_close_f32(sonic_boom.base_quad_size, 1.5);
        assert_range_f32(sonic_boom.color[0], 0.4, 1.0);
        assert_eq!(sonic_boom.color[0], sonic_boom.color[1]);
        assert_eq!(sonic_boom.color[1], sonic_boom.color[2]);
        assert_eq!(sonic_boom.color[3], 1.0);
        assert_eq!(sonic_boom.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut gust_random = ParticleRandom::new(37);
        let gust = ParticleVisualDescriptor::FixedQuad {
            size: 1.0,
            color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
        }
        .sample_for_command(&mut gust_random, [9.0, 9.0, 9.0]);
        assert_close_f32(gust.base_quad_size, 1.0);
        assert_eq!(gust.color, WHITE_PARTICLE_COLOR);
        assert_eq!(gust.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut small_gust_random = ParticleRandom::new(38);
        let small_gust = ParticleVisualDescriptor::FixedQuad {
            size: 0.15,
            color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
        }
        .sample_for_command(&mut small_gust_random, [9.0, 9.0, 9.0]);
        assert_close_f32(small_gust.base_quad_size, 0.15);
        assert_eq!(small_gust.color, WHITE_PARTICLE_COLOR);
        assert_eq!(small_gust.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut soul_random = ParticleRandom::new(35);
        let soul = ParticleVisualDescriptor::SingleQuadScaled {
            scale: 1.5,
            color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        }
        .sample_for_command(&mut soul_random, [0.0, 0.0, 0.0]);
        assert_range_f32(soul.base_quad_size, 0.15, 0.3);
        assert_eq!(soul.color, WHITE_PARTICLE_COLOR);
        assert_eq!(soul.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut cloud_random = ParticleRandom::new(8);
        let cloud = ParticleVisualDescriptor::PlayerCloud
            .sample_for_command(&mut cloud_random, [0.0, 0.0, 0.0]);
        assert_range_f32(cloud.base_quad_size, 0.1875, 0.375);
        assert_range_f32(cloud.color[0], 0.7, 1.0);
        assert_eq!(cloud.color[0], cloud.color[1]);
        assert_eq!(cloud.color[1], cloud.color[2]);
        assert_eq!(cloud.color[3], 1.0);
        assert_eq!(cloud.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut bubble_random = ParticleRandom::new(27);
        let bubble = ParticleVisualDescriptor::Bubble
            .sample_for_command(&mut bubble_random, [0.0, 0.0, 0.0]);
        assert_range_f32(bubble.base_quad_size, 0.02, 0.16);
        assert_eq!(bubble.color, WHITE_PARTICLE_COLOR);
        assert_eq!(bubble.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut sneeze_random = ParticleRandom::new(22);
        let sneeze = ParticleVisualDescriptor::PlayerCloudTint {
            color: ParticleColorDescriptor::FixedRgba([0.22, 1.0, 0.53, 0.4]),
        }
        .sample_for_command(&mut sneeze_random, [0.0, 0.0, 0.0]);
        assert_range_f32(sneeze.base_quad_size, 0.1875, 0.375);
        assert_close_f32(sneeze.color[0], 0.22);
        assert_close_f32(sneeze.color[1], 1.0);
        assert_close_f32(sneeze.color[2], 0.53);
        assert_close_f32(sneeze.color[3], 0.4);
        assert_eq!(sneeze.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut note_random = ParticleRandom::new(21);
        let note =
            ParticleVisualDescriptor::Note.sample_for_command(&mut note_random, [0.0, 0.0, 0.0]);
        assert_range_f32(note.base_quad_size, 0.15, 0.3);
        assert_close_f32(note.color[0], 0.35);
        assert_close_f32(note.color[1], 0.912_916_5);
        assert_close_f32(note.color[2], 0.0);
        assert_eq!(note.color[3], 1.0);
        assert_eq!(note.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut witch_random = ParticleRandom::new(31);
        let witch = ParticleVisualDescriptor::WitchSpell
            .sample_for_command(&mut witch_random, [0.0, 0.0, 0.0]);
        assert_range_f32(witch.base_quad_size, 0.075, 0.15);
        assert_range_f32(witch.color[0], 0.35, 0.85);
        assert_close_f32(witch.color[1], 0.0);
        assert_close_f32(witch.color[2], witch.color[0]);
        assert_eq!(witch.color[3], 1.0);
        assert_eq!(witch.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut glow_random = ParticleRandom::new(34);
        let glow = ParticleVisualDescriptor::SingleQuadScaled {
            scale: 0.75,
            color: ParticleColorDescriptor::FixedRgbChoice {
                first: [0.6, 1.0, 0.8],
                second: [0.08, 0.4, 0.4],
            },
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        }
        .sample_for_command(&mut glow_random, [0.0, 0.0, 0.0]);
        assert_range_f32(glow.base_quad_size, 0.075, 0.15);
        assert!(glow.color == [0.6, 1.0, 0.8, 1.0] || glow.color == [0.08, 0.4, 0.4, 1.0]);
        assert_eq!(glow.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut wax_on_random = ParticleRandom::new(32);
        let wax_on = ParticleVisualDescriptor::SingleQuadScaled {
            scale: 0.75,
            color: ParticleColorDescriptor::FixedRgb([0.91, 0.55, 0.08]),
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        }
        .sample_for_command(&mut wax_on_random, [0.0, 0.0, 0.0]);
        assert_range_f32(wax_on.base_quad_size, 0.075, 0.15);
        assert_eq!(wax_on.color, [0.91, 0.55, 0.08, 1.0]);
        assert_eq!(wax_on.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut scrape_random = ParticleRandom::new(33);
        let scrape = ParticleVisualDescriptor::SingleQuadScaled {
            scale: 0.75,
            color: ParticleColorDescriptor::FixedRgbChoice {
                first: [0.29, 0.58, 0.51],
                second: [0.43, 0.77, 0.62],
            },
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        }
        .sample_for_command(&mut scrape_random, [0.0, 0.0, 0.0]);
        assert!(scrape.color == [0.29, 0.58, 0.51, 1.0] || scrape.color == [0.43, 0.77, 0.62, 1.0]);
        assert_eq!(scrape.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut crit_random = ParticleRandom::new(24);
        let crit = ParticleVisualDescriptor::Crit {
            color_scale: CRIT_COLOR_SCALE,
        }
        .sample_for_command(&mut crit_random, [0.0, 0.0, 0.0]);
        assert_range_f32(crit.base_quad_size, 0.075, 0.15);
        assert_range_f32(crit.color[0], 0.6, 0.9);
        assert_close_f32(crit.color[1], crit.color[0] * 0.96);
        assert_close_f32(crit.color[2], crit.color[0] * 0.9);
        assert_eq!(crit.color[3], 1.0);
        assert_eq!(crit.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut magic_crit_random = ParticleRandom::new(24);
        let magic_crit = ParticleVisualDescriptor::Crit {
            color_scale: MAGIC_CRIT_COLOR_SCALE,
        }
        .sample_for_command(&mut magic_crit_random, [0.0, 0.0, 0.0]);
        assert_close_f32(magic_crit.base_quad_size, crit.base_quad_size);
        assert_close_f32(magic_crit.color[0], crit.color[0] * 0.3);
        assert_close_f32(magic_crit.color[1], crit.color[1] * 0.8);
        assert_close_f32(magic_crit.color[2], crit.color[2]);
        assert_eq!(
            magic_crit.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );

        let mut heart_random = ParticleRandom::new(17);
        let heart = ParticleVisualDescriptor::SingleQuadScaled {
            scale: 1.5,
            color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
        }
        .sample_for_command(&mut heart_random, [0.0, 0.0, 0.0]);
        assert_range_f32(heart.base_quad_size, 0.15, 0.3);
        assert_eq!(heart.color, WHITE_PARTICLE_COLOR);
        assert_eq!(heart.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);

        let mut dragon_random = ParticleRandom::new(12);
        let dragon_breath = ParticleVisualDescriptor::BaseAshSmoke {
            scale: 1.0,
            color: ParticleColorDescriptor::RandomRgbRange {
                min: DRAGON_BREATH_COLOR_MIN,
                max: DRAGON_BREATH_COLOR_MAX,
            },
        }
        .sample_for_command(&mut dragon_random, [0.0, 0.0, 0.0]);
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

        let mut dolphin_random = ParticleRandom::new(19);
        let dolphin = ParticleVisualDescriptor::SuspendedTown {
            color: SuspendedTownColorDescriptor::Override(
                ParticleColorDescriptor::FixedRgbRandomAlpha {
                    rgb: [0.3, 0.5, 1.0],
                    min_alpha: 0.3,
                    max_alpha: 1.0,
                },
            ),
        }
        .sample_for_command(&mut dolphin_random, [0.0, 0.0, 0.0]);
        assert_range_f32(dolphin.base_quad_size, 0.05, 0.22);
        assert_close_f32(dolphin.color[0], 0.3);
        assert_close_f32(dolphin.color[1], 0.5);
        assert_close_f32(dolphin.color[2], 1.0);
        assert_range_f32(dolphin.color[3], 0.3, 1.0);
        assert_eq!(dolphin.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut happy_villager_random = ParticleRandom::new(13);
        let happy_villager = ParticleVisualDescriptor::SuspendedTown {
            color: SuspendedTownColorDescriptor::Override(ParticleColorDescriptor::FixedRgb([
                1.0, 1.0, 1.0,
            ])),
        }
        .sample_for_command(&mut happy_villager_random, [0.0, 0.0, 0.0]);
        assert_range_f32(happy_villager.base_quad_size, 0.05, 0.22);
        assert_eq!(happy_villager.color, WHITE_PARTICLE_COLOR);
        assert_eq!(
            happy_villager.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );

        let mut mycelium_random = ParticleRandom::new(16);
        let mycelium = ParticleVisualDescriptor::SuspendedTown {
            color: SuspendedTownColorDescriptor::BaseGray,
        }
        .sample_for_command(&mut mycelium_random, [0.0, 0.0, 0.0]);
        assert_range_f32(mycelium.base_quad_size, 0.05, 0.22);
        assert_range_f32(mycelium.color[0], 0.2, 0.3);
        assert_eq!(mycelium.color[0], mycelium.color[1]);
        assert_eq!(mycelium.color[1], mycelium.color[2]);
        assert_eq!(mycelium.color[3], 1.0);
        assert_eq!(mycelium.quad_size_curve, ParticleQuadSizeCurve::Constant);

        let mut smoke_random = ParticleRandom::new(9);
        let smoke = ParticleVisualDescriptor::BaseAshSmoke {
            scale: 2.5,
            color: ParticleColorDescriptor::RandomGray { max: 0.3 },
        }
        .sample_for_command(&mut smoke_random, [0.0, 0.0, 0.0]);
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
        .sample_for_command(&mut white_smoke_random, [0.0, 0.0, 0.0]);
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
        let poof =
            ParticleVisualDescriptor::Explode.sample_for_command(&mut poof_random, [0.0, 0.0, 0.0]);
        assert_range_f32(poof.base_quad_size, 0.1, 0.7);
        assert_range_f32(poof.color[0], 0.7, 1.0);
        assert_eq!(poof.quad_size_curve, ParticleQuadSizeCurve::Constant);
    }

    #[test]
    fn initial_velocity_descriptor_matches_vanilla_particle_constructor_scaling() {
        let zero_velocity = ParticleInitialVelocityDescriptor::Zero
            .sample([1.0, 2.0, 3.0], &mut ParticleRandom::new(36));
        assert_eq!(zero_velocity, [0.0, 0.0, 0.0]);

        let mut random = ParticleRandom::new(14);
        let velocity = ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 0.02 }
            .sample([0.0, 0.0, 0.0], &mut random);

        assert_range_f64(velocity[0], -0.004, 0.004);
        assert_range_f64(velocity[1], -0.002, 0.006);
        assert_range_f64(velocity[2], -0.004, 0.004);
        assert_ne!(velocity, [0.0, 0.0, 0.0]);

        let mut heart_random = ParticleRandom::new(18);
        let heart_velocity =
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledWithYOffset {
                scale: 0.01,
                y_offset: 0.1,
            }
            .sample([8.0, 8.0, 8.0], &mut heart_random);
        assert_range_f64(heart_velocity[0], -0.002, 0.002);
        assert_range_f64(heart_velocity[1], 0.098, 0.102);
        assert_range_f64(heart_velocity[2], -0.002, 0.002);

        let mut cloud_random = ParticleRandom::new(23);
        let cloud_velocity =
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                scale: 0.1,
            }
            .sample([1.0, 2.0, 3.0], &mut cloud_random);
        assert_range_f64(cloud_velocity[0], 0.98, 1.02);
        assert_range_f64(cloud_velocity[1], 1.99, 2.03);
        assert_range_f64(cloud_velocity[2], 2.98, 3.02);

        let mut bubble_random = ParticleRandom::new(27);
        let bubble_velocity = ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
            command_scale: 0.2,
            random_range: 0.02,
        }
        .sample([1.0, 2.0, 3.0], &mut bubble_random);
        assert_range_f64(bubble_velocity[0], 0.18, 0.22);
        assert_range_f64(bubble_velocity[1], 0.38, 0.42);
        assert_range_f64(bubble_velocity[2], 0.58, 0.62);

        let axis_scaled = ParticleInitialVelocityDescriptor::CommandAxisScaled {
            scale: [0.005, 0.01, 0.005],
        }
        .sample([2.0, 3.0, 4.0], &mut ParticleRandom::new(29));
        assert_close_f64(axis_scaled[0], 0.01);
        assert_close_f64(axis_scaled[1], 0.03);
        assert_close_f64(axis_scaled[2], 0.02);

        let rising_velocity = ParticleInitialVelocityDescriptor::RisingParticle
            .sample([1.0, 2.0, 3.0], &mut ParticleRandom::new(34));
        assert_range_f64(rising_velocity[0], 0.998, 1.002);
        assert_range_f64(rising_velocity[1], 2.000, 2.003);
        assert_range_f64(rising_velocity[2], 2.998, 3.002);

        let mut crit_random = ParticleRandom::new(24);
        let crit_velocity =
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale: 0.1,
                command_scale: 0.4,
                command_y_offset: 0.0,
            }
            .sample([0.5, 0.25, -0.5], &mut crit_random);
        assert_range_f64(crit_velocity[0], 0.19, 0.21);
        assert_range_f64(crit_velocity[1], 0.10, 0.12);
        assert_range_f64(crit_velocity[2], -0.21, -0.19);

        let mut damage_random = ParticleRandom::new(25);
        let damage_velocity =
            ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusScaledCommand {
                random_scale: 0.1,
                command_scale: 0.4,
                command_y_offset: 1.0,
            }
            .sample([0.0, 0.0, 0.0], &mut damage_random);
        assert_range_f64(damage_velocity[0], -0.012, 0.012);
        assert_range_f64(damage_velocity[1], 0.40, 0.43);
        assert_range_f64(damage_velocity[2], -0.012, 0.012);

        let mut still_spell_random = ParticleRandom::new(30);
        let still_spell_velocity = ParticleInitialVelocityDescriptor::Spell
            .sample([0.0, 1.0, 0.0], &mut still_spell_random);
        let mut moving_spell_random = ParticleRandom::new(30);
        let moving_spell_velocity = ParticleInitialVelocityDescriptor::Spell
            .sample([1.0, 1.0, 0.0], &mut moving_spell_random);
        assert_range_f64(still_spell_velocity[0].abs(), 0.0, 0.008);
        assert_range_f64(still_spell_velocity[1], 0.0, 0.06);
        assert_range_f64(still_spell_velocity[2].abs(), 0.0, 0.008);
        assert_close_f64(still_spell_velocity[0], moving_spell_velocity[0] * 0.1);
        assert_close_f64(still_spell_velocity[2], moving_spell_velocity[2] * 0.1);

        let mut still_glow_random = ParticleRandom::new(31);
        let still_glow_velocity = ParticleInitialVelocityDescriptor::GlowSquid
            .sample([0.0, 1.0, 0.0], &mut still_glow_random);
        let mut moving_glow_random = ParticleRandom::new(31);
        let moving_glow_velocity = ParticleInitialVelocityDescriptor::GlowSquid
            .sample([1.0, 1.0, 0.0], &mut moving_glow_random);
        assert_range_f64(still_glow_velocity[0].abs(), 0.0, 0.008);
        assert_range_f64(still_glow_velocity[1], 0.0, 0.06);
        assert_range_f64(still_glow_velocity[2].abs(), 0.0, 0.008);
        assert_close_f64(still_glow_velocity[0], moving_glow_velocity[0] * 0.1);
        assert_close_f64(still_glow_velocity[2], moving_glow_velocity[2] * 0.1);
    }

    #[test]
    fn random_inclusive_lifetime_samples_configured_range() {
        let mut random = ParticleRandom::new(15);
        for _ in 0..32 {
            let lifetime =
                ParticleLifetimeDescriptor::RandomInclusive { min: 3, max: 7 }.sample(&mut random);
            assert!((3..=7).contains(&lifetime));
        }

        let mut random = ParticleRandom::new(20);
        for _ in 0..32 {
            let lifetime = ParticleLifetimeDescriptor::BaseAshSmokeDivided {
                max_lifetime: 20,
                scale_tenths: 10,
                divisor: 2,
            }
            .sample(&mut random);
            assert!((10..=50).contains(&lifetime));
        }

        let mut random = ParticleRandom::new(26);
        for _ in 0..32 {
            let lifetime = ParticleLifetimeDescriptor::Crit.sample(&mut random);
            assert!((4..=10).contains(&lifetime));
        }

        let mut random = ParticleRandom::new(28);
        for _ in 0..32 {
            let lifetime = ParticleLifetimeDescriptor::EightOverRandom.sample(&mut random);
            assert!((8..=40).contains(&lifetime));
        }

        let mut random = ParticleRandom::new(29);
        for _ in 0..32 {
            let lifetime = ParticleLifetimeDescriptor::FortyOverRandom.sample(&mut random);
            assert!((40..=200).contains(&lifetime));
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

    fn assert_close_f64(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
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
