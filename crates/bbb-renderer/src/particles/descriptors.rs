use serde::{Deserialize, Serialize};

use super::ParticleFluidKind;

pub(crate) const DEFAULT_PARTICLE_RANDOM_SEED: i64 = 0x5EED_2601;
pub(crate) const END_ROD_FADE_COLOR: [f32; 3] = [242.0 / 255.0, 222.0 / 255.0, 201.0 / 255.0];
pub(crate) const SQUID_INK_AIR_DOWNWARD_ACCELERATION: f64 = 0.0074;
pub(crate) const VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT: usize = 1000;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FallingLeavesDescriptor {
    pub(crate) fall_acceleration: f32,
    pub(crate) side_acceleration: f32,
    pub(crate) swirl: bool,
    pub(crate) flow_away: bool,
    pub(crate) scale: f32,
    pub(crate) start_velocity: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleTickMotionDescriptor {
    #[default]
    DefaultParticleTick,
    DirectGravityNoFriction,
    NoMotion,
    CurrentDown,
    Snowflake,
    FlyTowardsPosition,
    TrailTarget,
    VibrationSignal,
    FlyStraightTowards,
    CampfireSmoke,
    DripHang,
    CoolingDripHang,
    DripFalling,
    DripFallAndLand,
    DripLand,
    DustPlume,
    WaterDrop,
    Wake,
    Portal,
    ReversePortal,
    DragonBreath,
    Firefly,
    FallingLeaves,
    FallingDust,
    ItemPickup,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleLightEmissionDescriptor {
    #[default]
    World,
    FullBright,
    FullBlock,
    SmoothBlockByAge,
    SmoothBlockByAgeQuartic,
    Firefly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleFacingCameraMode {
    #[default]
    LookAtXyz,
    LookAtY,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleAlphaCurve {
    #[default]
    Constant,
    SimpleAnimatedFade,
    FlashOverlayFade,
    FireworkSparkFade,
    ShriekFade,
    VaultConnectionFade,
    FireflyFade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleChildEmissionDescriptor {
    LavaSmoke,
    HugeExplosionSeed,
    DripHangToFall,
    DripFallAndLand,
    GustSeed {
        scale_tenths: u32,
        vanilla_lifetime: u32,
        tick_delay: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum ParticleLimitDescriptor {
    SporeBlossom,
}

impl ParticleLimitDescriptor {
    pub(crate) const fn limit(self) -> usize {
        match self {
            Self::SporeBlossom => VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT,
        }
    }
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
    SixOverRandom,
    EightOverRandom,
    SixteenOverRandom,
    SixteenOverRandomPlusTwo,
    FortyOverRandom,
    SporeBlossomAir,
    TrialSpawnerDetection,
    Portal,
    ReversePortal,
    RandomFloatSpan {
        min: u32,
        span: u32,
    },
    RandomFloatDivisor {
        numerator: u32,
        min_tenths: u32,
        span_tenths: u32,
    },
    RandomInclusive {
        min: u32,
        max: u32,
    },
    InclusiveTick {
        vanilla_lifetime: u32,
    },
    FallingDust,
    CommandOption {
        fallback: u32,
    },
    DustScale {
        fallback_scale: u32,
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
    AttackSweep,
    SingleQuadRandomScaled {
        min_scale: f32,
        max_scale: f32,
        color: ParticleColorDescriptor,
        quad_size_curve: ParticleQuadSizeCurve,
    },
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
    Snowflake,
    FixedQuad {
        size: f32,
        color: ParticleColorDescriptor,
    },
    HugeExplosion,
    FlyTowardsPosition,
    OminousSpawn,
    VaultConnection,
    Shriek,
    Totem,
    Portal,
    ReversePortal,
    BaseAshSmoke {
        scale: f32,
        color: ParticleColorDescriptor,
    },
    SuspendedTown {
        color: SuspendedTownColorDescriptor,
    },
    Explode,
    FallingLeaves {
        scale: f32,
        color: ParticleColorDescriptor,
    },
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
    FixedRgbMinusRandom {
        rgb: [f32; 3],
        max_subtract: f32,
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
    Lava,
    FlashOverlay,
    Portal,
    ReversePortal,
    Shriek,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParticleInitialVelocityDescriptor {
    Zero,
    Fixed([f64; 3]),
    Command,
    CommandScaledPlusRandom {
        command_scale: f64,
        random_range: f64,
    },
    CommandAxisScaled {
        scale: [f64; 3],
    },
    CommandWithYOffset {
        y_offset: f64,
    },
    RisingParticle,
    ParticleConstructorScaled {
        scale: f64,
    },
    ParticleConstructorZero,
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
    TerrainDustPillar,
    CrimsonSpore,
    WarpedSpore,
    CampfireSmoke,
    WaterDrop,
    SplashWaterDrop,
    Spell,
    GlowSquid,
    Lava,
    Firefly,
    /// `BaseAshSmokeParticle`: the `Particle` 7-arg normalized base spread
    /// multiplied per axis by `dir` (`xd *= dirX; yd *= dirY; zd *= dirZ`),
    /// then the provider velocity added (`xd += xa; yd += ya; zd += za`).
    BaseAshSmokeSpread {
        dir: [f64; 3],
        provider_offset: BaseAshSmokeOffset,
    },
}

/// Provider velocity added after the per-axis `dir` multiply in
/// `BaseAshSmokeParticle` (`this.xd += xa; this.yd += ya; this.zd += za`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BaseAshSmokeOffset {
    /// `AshParticle.Provider.createParticle` passes velocity `(0, 0, 0)`.
    Zero,
    /// `WhiteAshParticle.Provider.createParticle` ignores the command velocity
    /// and draws its own negative-biased `xa/ya/za`.
    WhiteAsh,
    /// `DustPlumeParticle.Provider.createParticle` passes the command velocity as
    /// `xa/ya/za` and `DustPlumeParticle` adds `y_offset` to `ya`
    /// (`super(..., xa, ya + 0.15F, za, ...)`). Draws no RNG.
    CommandWithYOffset { y_offset: f64 },
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
    next_next_gaussian: Option<f64>,
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
            "minecraft:current_down" => Self {
                provider: "WaterCurrentDownParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 30, span: 60 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.2,
                    max_scale: 0.8,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Fixed([0.0, -0.05, 0.0]),
                friction: 0.98,
                gravity: 0.002,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:rain" => Self {
                provider: "WaterDropParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::WaterDrop,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:splash" => Self {
                provider: "SplashParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::SplashWaterDrop,
                friction: 0.98,
                gravity: 0.04,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:fishing" => Self {
                provider: "WakeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:ominous_spawning" => Self {
                provider: "FlyStraightTowardsParticle.OminousSpawnProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 25, span: 5 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::OminousSpawn,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:bubble_pop" => Self {
                provider: "BubblePopParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(4),
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.008,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:firefly" => Self {
                provider: "FireflyParticle.FireflyProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 200, max: 300 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.125,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 0.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Firefly,
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:dust" | "minecraft:dust_color_transition" => Self {
                provider: if particle_id == "minecraft:dust_color_transition" {
                    "DustColorTransitionParticle.Provider"
                } else {
                    "DustParticle.Provider"
                },
                lifetime: ParticleLifetimeDescriptor::DustScale { fallback_scale: 1 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.0, 0.0]),
                    quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorScaled {
                    scale: 0.1,
                },
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:sweep_attack" => Self {
                provider: "AttackSweepParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(4),
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::AttackSweep,
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorZero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:underwater" => Self {
                provider: "SuspendedParticle.UnderwaterProvider",
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.2,
                    max_scale: 0.8,
                    color: ParticleColorDescriptor::FixedRgb([0.4, 0.4, 0.7]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 1.0,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:spore_blossom_air" => Self {
                provider: "SuspendedParticle.SporeBlossomAirProvider",
                lifetime: ParticleLifetimeDescriptor::SporeBlossomAir,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.6,
                    max_scale: 1.2,
                    color: ParticleColorDescriptor::FixedRgb([0.32, 0.5, 0.22]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Fixed([0.0, -0.8, 0.0]),
                friction: 1.0,
                gravity: 0.01,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_nectar" => Self {
                provider: "DripParticle.NectarFallProvider",
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.92, 0.782, 0.72]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.007,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_spore_blossom" => Self {
                provider: "DripParticle.SporeBlossomFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 1,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.32, 0.5, 0.22]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.005,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_honey" => Self {
                provider: "DripParticle.HoneyHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(100),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.622, 0.508, 0.082]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.000_012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_honey" => Self {
                provider: "DripParticle.HoneyFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.582, 0.448, 0.082]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.01,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:landing_honey" => Self {
                provider: "DripParticle.HoneyLandProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 128,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.522, 0.408, 0.082]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_obsidian_tear" => Self {
                provider: "DripParticle.ObsidianTearHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(100),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.511_718_75, 0.031_25, 0.890_625]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.000_012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_obsidian_tear" => Self {
                provider: "DripParticle.ObsidianTearFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.511_718_75, 0.031_25, 0.890_625]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.01,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:landing_obsidian_tear" => Self {
                provider: "DripParticle.ObsidianTearLandProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 28,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.511_718_75, 0.031_25, 0.890_625]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_lava" => Self {
                provider: "DripParticle.LavaHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(40),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_lava" => Self {
                provider: "DripParticle.LavaFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.285_714_3, 0.083_333_336]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:landing_lava" => Self {
                provider: "DripParticle.LavaLandProvider",
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.285_714_3, 0.083_333_336]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_water" => Self {
                provider: "DripParticle.WaterHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(40),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.2, 0.3, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_water" => Self {
                provider: "DripParticle.WaterFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.2, 0.3, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_dripstone_lava" => Self {
                provider: "DripParticle.DripstoneLavaHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(40),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_dripstone_lava" => Self {
                provider: "DripParticle.DripstoneLavaFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 0.285_714_3, 0.083_333_336]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dripping_dripstone_water" => Self {
                provider: "DripParticle.DripstoneWaterHangProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(40),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.2, 0.3, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0012,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:falling_dripstone_water" => Self {
                provider: "DripParticle.DripstoneWaterFallProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatDivisor {
                    numerator: 64,
                    min_tenths: 2,
                    span_tenths: 8,
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgb([0.2, 0.3, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:crimson_spore" | "minecraft:warped_spore" => Self {
                provider: if particle_id == "minecraft:crimson_spore" {
                    "SuspendedParticle.CrimsonSporeProvider"
                } else {
                    "SuspendedParticle.WarpedSporeProvider"
                },
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.6,
                    max_scale: 1.2,
                    color: ParticleColorDescriptor::FixedRgb(if particle_id
                        == "minecraft:crimson_spore"
                    {
                        [0.9, 0.4, 0.5]
                    } else {
                        [0.1, 0.1, 0.3]
                    }),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: if particle_id == "minecraft:crimson_spore" {
                    ParticleInitialVelocityDescriptor::CrimsonSpore
                } else {
                    ParticleInitialVelocityDescriptor::WarpedSpore
                },
                friction: 1.0,
                gravity: 0.0,
                has_physics: false,
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
            "minecraft:snowflake" => Self {
                provider: "SnowflakeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandomPlusTwo,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::Snowflake,
                initial_velocity: ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                    command_scale: 1.0,
                    random_range: 0.05,
                },
                friction: 1.0,
                gravity: 0.225,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:squid_ink" | "minecraft:glow_squid_ink" => Self {
                provider: if particle_id == "minecraft:glow_squid_ink" {
                    "SquidInkParticle.GlowInkProvider"
                } else {
                    "SquidInkParticle.Provider"
                },
                lifetime: ParticleLifetimeDescriptor::SixOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.5,
                    color: if particle_id == "minecraft:glow_squid_ink" {
                        ParticleColorDescriptor::FixedRgba([0.2, 0.8, 0.6, 1.0])
                    } else {
                        ParticleColorDescriptor::FixedRgba([0.0, 0.0, 0.0, 1.0])
                    },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.92,
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
            "minecraft:enchant" | "minecraft:nautilus" => Self {
                provider: if particle_id == "minecraft:nautilus" {
                    "FlyTowardsPositionParticle.NautilusProvider"
                } else {
                    "FlyTowardsPositionParticle.EnchantProvider"
                },
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 30, span: 10 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::FlyTowardsPosition,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:vault_connection" => Self {
                provider: "FlyTowardsPositionParticle.VaultConnectionProvider",
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 30, span: 10 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::VaultConnection,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:totem_of_undying" => Self {
                provider: "TotemParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 60, max: 71 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::Totem,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.6,
                gravity: 1.25,
                has_physics: true,
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
            "minecraft:end_rod" => Self {
                provider: "EndRodParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 60, max: 71 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.91,
                gravity: 0.0125,
                has_physics: true,
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
            "minecraft:explosion_emitter" => Self {
                provider: "HugeExplosionSeedParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(8),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:elder_guardian" => Self {
                provider: "ElderGuardianParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(30),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
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
            "minecraft:sculk_charge" => Self {
                provider: "SculkChargeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 8, max: 19 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.5,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:sculk_charge_pop" => Self {
                provider: "SculkChargePopParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 6, max: 9 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:shriek" => Self {
                provider: "ShriekParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(30),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Shriek,
                initial_velocity: ParticleInitialVelocityDescriptor::Fixed([0.0, 0.1, 0.0]),
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:trial_spawner_detection"
            | "minecraft:trial_spawner_detection_ominous" => Self {
                provider: "TrialSpawnerDetectionParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::TrialSpawnerDetection,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.125,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
                },
                // TrialSpawnerDetectionParticle: `super(..., 0, 0, 0, ...)` base
                // spread, then `xd *= 0.0; yd *= 0.9; zd *= 0.0` before adding the
                // command velocity. The provider passes `xAux/yAux/zAux` straight
                // through with no y offset, so reuse the shared BaseAshSmoke shape
                // with a `CommandWithYOffset { y_offset: 0.0 }` passthrough.
                initial_velocity: ParticleInitialVelocityDescriptor::BaseAshSmokeSpread {
                    dir: [0.0, 0.9, 0.0],
                    provider_offset: BaseAshSmokeOffset::CommandWithYOffset { y_offset: 0.0 },
                },
                friction: 0.96,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: true,
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
            "minecraft:gust_emitter_large" => Self {
                provider: "GustSeedParticle.Provider(3.0,7,0)",
                lifetime: ParticleLifetimeDescriptor::InclusiveTick {
                    vanilla_lifetime: 7,
                },
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:gust_emitter_small" => Self {
                provider: "GustSeedParticle.Provider(1.0,3,2)",
                lifetime: ParticleLifetimeDescriptor::InclusiveTick {
                    vanilla_lifetime: 3,
                },
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
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
            "minecraft:flame" | "minecraft:soul_fire_flame" | "minecraft:copper_fire_flame" => Self {
                provider: "FlameParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Rising,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Flame { scale: 1.0 },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:lava" => Self {
                provider: "LavaParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.2,
                    max_scale: 2.2,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Lava,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Lava,
                friction: 0.999,
                gravity: 0.75,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
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
            "minecraft:flash" => Self {
                provider: "FireworkParticles.FlashProvider",
                lifetime: ParticleLifetimeDescriptor::Fixed(4),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::FlashOverlay,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:firework" => Self {
                provider: "FireworkParticles.SparkProvider",
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 48, max: 59 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.75,
                    color: ParticleColorDescriptor::FixedRgba([1.0, 1.0, 1.0, 0.99]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.91,
                gravity: 0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:trail" => Self {
                provider: "TrailParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::CommandOption { fallback: 1 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.26,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:vibration" => Self {
                provider: "VibrationSignalParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::CommandOption { fallback: 1 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.3,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:effect"
            | "minecraft:instant_effect"
            | "minecraft:entity_effect"
            | "minecraft:infested"
            | "minecraft:raid_omen"
            | "minecraft:trial_omen" => Self {
                provider: match particle_id {
                    "minecraft:effect" | "minecraft:instant_effect" => {
                        "SpellParticle.InstantProvider"
                    }
                    "minecraft:entity_effect" => "SpellParticle.MobEffectProvider",
                    _ => "SpellParticle.Provider",
                },
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
            "minecraft:cherry_leaves"
            | "minecraft:pale_oak_leaves"
            | "minecraft:tinted_leaves" => {
                let falling = falling_leaves_descriptor_for_particle(particle_id);
                Self {
                    provider: match particle_id {
                        "minecraft:cherry_leaves" => "FallingLeavesParticle.CherryProvider",
                        "minecraft:pale_oak_leaves" => "FallingLeavesParticle.PaleOakProvider",
                        _ => "FallingLeavesParticle.TintedLeavesProvider",
                    },
                    lifetime: ParticleLifetimeDescriptor::Fixed(300),
                    sprite_selection: ParticleSpriteSelection::Random,
                    visual: ParticleVisualDescriptor::FallingLeaves {
                        scale: falling.scale,
                        color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    },
                    initial_velocity: ParticleInitialVelocityDescriptor::Fixed([
                        0.0,
                        -falling.start_velocity,
                        0.0,
                    ]),
                    friction: 1.0,
                    gravity: falling.fall_acceleration * 1.2 * 0.0025,
                    has_physics: true,
                    speed_up_when_y_motion_is_blocked: false,
                }
            }
            "minecraft:pause_mob_growth" | "minecraft:reset_mob_growth" => Self {
                provider: if particle_id == "minecraft:reset_mob_growth" {
                    "SimpleVerticalParticle.ResetMobGrowthProvider"
                } else {
                    "SimpleVerticalParticle.PauseMobGrowthProvider"
                },
                lifetime: ParticleLifetimeDescriptor::Fixed(8),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.5,
                    max_scale: 1.1,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CommandWithYOffset {
                    y_offset: if particle_id == "minecraft:reset_mob_growth" {
                        0.03
                    } else {
                        -0.03
                    },
                },
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
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
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:campfire_cosy_smoke" | "minecraft:campfire_signal_smoke" => Self {
                provider: if particle_id == "minecraft:campfire_signal_smoke" {
                    "CampfireSmokeParticle.SignalProvider"
                } else {
                    "CampfireSmokeParticle.CosyProvider"
                },
                lifetime: if particle_id == "minecraft:campfire_signal_smoke" {
                    ParticleLifetimeDescriptor::RandomInclusive { min: 280, max: 329 }
                } else {
                    ParticleLifetimeDescriptor::RandomInclusive { min: 80, max: 129 }
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 3.0,
                    color: ParticleColorDescriptor::FixedRgba(if particle_id
                        == "minecraft:campfire_signal_smoke"
                    {
                        [1.0, 1.0, 1.0, 0.95]
                    } else {
                        [1.0, 1.0, 1.0, 0.9]
                    }),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::CampfireSmoke,
                friction: 0.98,
                gravity: 3.0E-6,
                has_physics: true,
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
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                        scale: 0.1,
                    },
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
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                        scale: 0.1,
                    },
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
                initial_velocity:
                    ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                        scale: 0.1,
                    },
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
                initial_velocity: ParticleInitialVelocityDescriptor::BaseAshSmokeSpread {
                    dir: [0.1, -0.1, 0.1],
                    provider_offset: BaseAshSmokeOffset::Zero,
                },
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
                initial_velocity: ParticleInitialVelocityDescriptor::BaseAshSmokeSpread {
                    dir: [0.1, -0.1, 0.1],
                    provider_offset: BaseAshSmokeOffset::WhiteAsh,
                },
                friction: 0.96,
                gravity: 0.0125,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            "minecraft:dust_plume" => Self {
                provider: "DustPlumeParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::BaseAshSmoke {
                    max_lifetime: 7,
                    scale_tenths: 10,
                },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::BaseAshSmoke {
                    scale: 1.0,
                    color: ParticleColorDescriptor::FixedRgbMinusRandom {
                        rgb: WHITE_ASH_SMOKE_RGB,
                        max_subtract: 0.2,
                    },
                },
                initial_velocity: ParticleInitialVelocityDescriptor::BaseAshSmokeSpread {
                    dir: [0.7, 0.6, 0.7],
                    provider_offset: BaseAshSmokeOffset::CommandWithYOffset { y_offset: 0.15 },
                },
                friction: 0.96,
                gravity: 0.5,
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
                initial_velocity: ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                    command_scale: 1.0,
                    random_range: 0.05,
                },
                friction: 0.9,
                gravity: -0.1,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:portal" => Self {
                provider: "PortalParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Portal,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Portal,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:reverse_portal" => Self {
                provider: "ReversePortalParticle.ReversePortalProvider",
                lifetime: ParticleLifetimeDescriptor::ReversePortal,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::ReversePortal,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:spit" => Self {
                provider: "SpitParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::Explode,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::Explode,
                initial_velocity: ParticleInitialVelocityDescriptor::CommandScaledPlusRandom {
                    command_scale: 1.0,
                    random_range: 0.05,
                },
                friction: 0.9,
                gravity: 0.5,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:block" => terrain_particle_descriptor(
                "TerrainParticle.Provider",
                ParticleLifetimeDescriptor::BaseParticle,
                ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 1.0 },
            ),
            "minecraft:block_marker" => Self {
                provider: "BlockMarker.Provider",
                lifetime: ParticleLifetimeDescriptor::Fixed(80),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.5,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:dust_pillar" => terrain_particle_descriptor(
                "TerrainParticle.DustPillarProvider",
                ParticleLifetimeDescriptor::RandomInclusive { min: 20, max: 39 },
                ParticleInitialVelocityDescriptor::TerrainDustPillar,
            ),
            "minecraft:block_crumble" => terrain_particle_descriptor(
                "TerrainParticle.CrumblingProvider",
                ParticleLifetimeDescriptor::RandomInclusive { min: 1, max: 10 },
                ParticleInitialVelocityDescriptor::Zero,
            ),
            "minecraft:falling_dust" => Self {
                provider: "FallingDustParticle.Provider",
                lifetime: ParticleLifetimeDescriptor::FallingDust,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 0.674_999_95,
                    color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
                    quad_size_curve: ParticleQuadSizeCurve::GrowToBase,
                },
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            "minecraft:item" => breaking_item_particle_descriptor(
                "BreakingItemParticle.Provider",
                ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                    scale: 0.1,
                },
            ),
            "minecraft:item_slime" => breaking_item_particle_descriptor(
                "BreakingItemParticle.SlimeProvider",
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            "minecraft:item_cobweb" => breaking_item_particle_descriptor(
                "BreakingItemParticle.CobwebProvider",
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            "minecraft:item_snowball" => breaking_item_particle_descriptor(
                "BreakingItemParticle.SnowballProvider",
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            "minecraft:item_pickup" => Self {
                provider: "ItemPickupParticle",
                lifetime: ParticleLifetimeDescriptor::Fixed(3),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
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
            "SuspendedParticle.UnderwaterProvider"
            | "SuspendedParticle.SporeBlossomAirProvider"
            | "SuspendedParticle.CrimsonSporeProvider"
            | "SuspendedParticle.WarpedSporeProvider" => [
                command_position[0],
                command_position[1] - 0.125,
                command_position[2],
            ],
            _ => command_position,
        }
    }

    pub(crate) fn tick_motion(self) -> ParticleTickMotionDescriptor {
        match self.provider {
            "BubblePopParticle.Provider" => ParticleTickMotionDescriptor::DirectGravityNoFriction,
            "AttackSweepParticle.Provider" => ParticleTickMotionDescriptor::NoMotion,
            "WaterCurrentDownParticle.Provider" => ParticleTickMotionDescriptor::CurrentDown,
            "SnowflakeParticle.Provider" => ParticleTickMotionDescriptor::Snowflake,
            "FlyTowardsPositionParticle.EnchantProvider"
            | "FlyTowardsPositionParticle.NautilusProvider"
            | "FlyTowardsPositionParticle.VaultConnectionProvider" => {
                ParticleTickMotionDescriptor::FlyTowardsPosition
            }
            "TrailParticle.Provider" => ParticleTickMotionDescriptor::TrailTarget,
            "VibrationSignalParticle.Provider" => ParticleTickMotionDescriptor::VibrationSignal,
            "FlyStraightTowardsParticle.OminousSpawnProvider" => {
                ParticleTickMotionDescriptor::FlyStraightTowards
            }
            "CampfireSmokeParticle.CosyProvider" | "CampfireSmokeParticle.SignalProvider" => {
                ParticleTickMotionDescriptor::CampfireSmoke
            }
            "DripParticle.HoneyHangProvider"
            | "DripParticle.ObsidianTearHangProvider"
            | "DripParticle.WaterHangProvider"
            | "DripParticle.DripstoneWaterHangProvider" => ParticleTickMotionDescriptor::DripHang,
            "DripParticle.LavaHangProvider" | "DripParticle.DripstoneLavaHangProvider" => {
                ParticleTickMotionDescriptor::CoolingDripHang
            }
            "DustPlumeParticle.Provider" => ParticleTickMotionDescriptor::DustPlume,
            "WaterDropParticle.Provider" | "SplashParticle.Provider" => {
                ParticleTickMotionDescriptor::WaterDrop
            }
            "DripParticle.NectarFallProvider" | "DripParticle.SporeBlossomFallProvider" => {
                ParticleTickMotionDescriptor::DripFalling
            }
            "DripParticle.HoneyFallProvider"
            | "DripParticle.ObsidianTearFallProvider"
            | "DripParticle.LavaFallProvider"
            | "DripParticle.WaterFallProvider"
            | "DripParticle.DripstoneLavaFallProvider"
            | "DripParticle.DripstoneWaterFallProvider" => {
                ParticleTickMotionDescriptor::DripFallAndLand
            }
            "DripParticle.HoneyLandProvider"
            | "DripParticle.ObsidianTearLandProvider"
            | "DripParticle.LavaLandProvider" => ParticleTickMotionDescriptor::DripLand,
            "WakeParticle.Provider" => ParticleTickMotionDescriptor::Wake,
            "PortalParticle.Provider" => ParticleTickMotionDescriptor::Portal,
            "ReversePortalParticle.ReversePortalProvider" => {
                ParticleTickMotionDescriptor::ReversePortal
            }
            "DragonBreathParticle.Provider" => ParticleTickMotionDescriptor::DragonBreath,
            "FireflyParticle.FireflyProvider" => ParticleTickMotionDescriptor::Firefly,
            "FallingLeavesParticle.CherryProvider"
            | "FallingLeavesParticle.PaleOakProvider"
            | "FallingLeavesParticle.TintedLeavesProvider" => {
                ParticleTickMotionDescriptor::FallingLeaves
            }
            "FallingDustParticle.Provider" => ParticleTickMotionDescriptor::FallingDust,
            "ItemPickupParticle" => ParticleTickMotionDescriptor::ItemPickup,
            _ => ParticleTickMotionDescriptor::DefaultParticleTick,
        }
    }

    pub(crate) fn collision_size(self) -> Option<[f32; 2]> {
        match self.provider {
            "CampfireSmokeParticle.CosyProvider" | "CampfireSmokeParticle.SignalProvider" => {
                Some([0.25, 0.25])
            }
            "WakeParticle.Provider" => Some([0.01, 0.01]),
            _ => None,
        }
    }

    pub(crate) fn drip_fluid(self) -> Option<ParticleFluidKind> {
        match self.provider {
            "DripParticle.WaterHangProvider"
            | "DripParticle.WaterFallProvider"
            | "DripParticle.DripstoneWaterHangProvider"
            | "DripParticle.DripstoneWaterFallProvider" => Some(ParticleFluidKind::Water),
            "DripParticle.LavaHangProvider"
            | "DripParticle.LavaFallProvider"
            | "DripParticle.LavaLandProvider"
            | "DripParticle.DripstoneLavaHangProvider"
            | "DripParticle.DripstoneLavaFallProvider" => Some(ParticleFluidKind::Lava),
            _ => None,
        }
    }

    pub(crate) fn required_fluid(self) -> Option<ParticleFluidKind> {
        match self.provider {
            "BubbleParticle.Provider"
            | "BubbleColumnUpParticle.Provider"
            | "WaterCurrentDownParticle.Provider" => Some(ParticleFluidKind::Water),
            _ => None,
        }
    }

    pub(crate) fn moves_without_collision(self) -> bool {
        matches!(
            self.provider,
            "EndRodParticle.Provider"
                | "FlameParticle.Provider"
                | "FlameParticle.SmallFlameProvider"
                | "PortalParticle.Provider"
                | "ReversePortalParticle.ReversePortalProvider"
                | "SuspendedTownParticle.HappyVillagerProvider"
                | "SuspendedTownParticle.ComposterFillProvider"
                | "SuspendedTownParticle.DolphinSpeedProvider"
                | "SuspendedTownParticle.EggCrackProvider"
                | "SuspendedTownParticle.Provider"
        )
    }

    pub(crate) fn air_downward_acceleration(self) -> f64 {
        match self.provider {
            "SquidInkParticle.Provider" | "SquidInkParticle.GlowInkProvider" => {
                SQUID_INK_AIR_DOWNWARD_ACCELERATION
            }
            _ => 0.0,
        }
    }

    pub(crate) fn falling_leaves(self) -> Option<FallingLeavesDescriptor> {
        match self.provider {
            "FallingLeavesParticle.CherryProvider" => Some(cherry_falling_leaves_descriptor()),
            "FallingLeavesParticle.PaleOakProvider"
            | "FallingLeavesParticle.TintedLeavesProvider" => {
                Some(pale_oak_falling_leaves_descriptor())
            }
            _ => None,
        }
    }

    pub(crate) fn light_emission(self) -> ParticleLightEmissionDescriptor {
        match self.provider {
            // Vanilla overrides `getLightCoords` to return `LightCoordsUtil.FULL_BRIGHT`.
            "AttackSweepParticle.Provider"
            | "TotemParticle.Provider"
            | "SquidInkParticle.Provider"
            | "SquidInkParticle.GlowInkProvider"
            | "EndRodParticle.Provider"
            | "FireworkParticles.SparkProvider"
            | "HugeExplosionParticle.Provider"
            | "SonicBoomParticle.Provider"
            | "GustParticle.Provider"
            | "GustParticle.SmallProvider"
            | "TrailParticle.Provider" => ParticleLightEmissionDescriptor::FullBright,
            // Vanilla keeps sky light from the world sample and forces block light to 15.
            "LavaParticle.Provider"
            | "SoulParticle.EmissiveProvider"
            | "SculkChargeParticle.Provider"
            | "SculkChargePopParticle.Provider"
            | "DripParticle.ObsidianTearHangProvider"
            | "DripParticle.ObsidianTearFallProvider"
            | "DripParticle.ObsidianTearLandProvider"
            | "ShriekParticle.Provider"
            | "VibrationSignalParticle.Provider"
            | "FlyTowardsPositionParticle.VaultConnectionProvider"
            | "TrialSpawnerDetectionParticle.Provider"
            | "FlyStraightTowardsParticle.OminousSpawnProvider" => {
                ParticleLightEmissionDescriptor::FullBlock
            }
            // Vanilla uses `LightCoordsUtil.addSmoothBlockEmission(..., (age + partialTick) / lifetime)`.
            "FlameParticle.Provider"
            | "FlameParticle.SmallFlameProvider"
            | "GlowParticle.GlowSquidProvider"
            | "GlowParticle.ElectricSparkProvider"
            | "GlowParticle.ScrapeProvider"
            | "GlowParticle.WaxOffProvider"
            | "GlowParticle.WaxOnProvider" => ParticleLightEmissionDescriptor::SmoothBlockByAge,
            // Vanilla portal particles add `(age / lifetime)^4` smooth block emission.
            "FlyTowardsPositionParticle.EnchantProvider"
            | "FlyTowardsPositionParticle.NautilusProvider"
            | "PortalParticle.Provider"
            | "ReversePortalParticle.ReversePortalProvider" => {
                ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
            }
            "FireflyParticle.FireflyProvider" => ParticleLightEmissionDescriptor::Firefly,
            _ => ParticleLightEmissionDescriptor::World,
        }
    }

    pub(crate) fn facing_camera_mode(self) -> ParticleFacingCameraMode {
        match self.provider {
            "TrialSpawnerDetectionParticle.Provider" => ParticleFacingCameraMode::LookAtY,
            _ => ParticleFacingCameraMode::LookAtXyz,
        }
    }

    pub(crate) fn alpha_curve(self) -> ParticleAlphaCurve {
        match self.provider {
            "TotemParticle.Provider"
            | "SquidInkParticle.Provider"
            | "SquidInkParticle.GlowInkProvider"
            | "EndRodParticle.Provider" => ParticleAlphaCurve::SimpleAnimatedFade,
            "FireworkParticles.FlashProvider" => ParticleAlphaCurve::FlashOverlayFade,
            "FireworkParticles.SparkProvider" => ParticleAlphaCurve::FireworkSparkFade,
            "ShriekParticle.Provider" => ParticleAlphaCurve::ShriekFade,
            "FlyTowardsPositionParticle.VaultConnectionProvider" => {
                ParticleAlphaCurve::VaultConnectionFade
            }
            "FireflyParticle.FireflyProvider" => ParticleAlphaCurve::FireflyFade,
            _ => ParticleAlphaCurve::Constant,
        }
    }

    pub(crate) fn color_fade_target(self) -> Option<[f32; 3]> {
        match self.provider {
            "EndRodParticle.Provider" => Some(END_ROD_FADE_COLOR),
            _ => None,
        }
    }

    pub(crate) fn child_emission(self) -> Option<ParticleChildEmissionDescriptor> {
        match self.provider {
            "LavaParticle.Provider" => Some(ParticleChildEmissionDescriptor::LavaSmoke),
            "DripParticle.HoneyHangProvider"
            | "DripParticle.ObsidianTearHangProvider"
            | "DripParticle.LavaHangProvider"
            | "DripParticle.WaterHangProvider"
            | "DripParticle.DripstoneLavaHangProvider"
            | "DripParticle.DripstoneWaterHangProvider" => {
                Some(ParticleChildEmissionDescriptor::DripHangToFall)
            }
            "DripParticle.HoneyFallProvider"
            | "DripParticle.ObsidianTearFallProvider"
            | "DripParticle.LavaFallProvider"
            | "DripParticle.WaterFallProvider"
            | "DripParticle.DripstoneLavaFallProvider"
            | "DripParticle.DripstoneWaterFallProvider" => {
                Some(ParticleChildEmissionDescriptor::DripFallAndLand)
            }
            "HugeExplosionSeedParticle.Provider" => {
                Some(ParticleChildEmissionDescriptor::HugeExplosionSeed)
            }
            "GustSeedParticle.Provider(3.0,7,0)" => {
                Some(ParticleChildEmissionDescriptor::GustSeed {
                    scale_tenths: 30,
                    vanilla_lifetime: 7,
                    tick_delay: 0,
                })
            }
            "GustSeedParticle.Provider(1.0,3,2)" => {
                Some(ParticleChildEmissionDescriptor::GustSeed {
                    scale_tenths: 10,
                    vanilla_lifetime: 3,
                    tick_delay: 2,
                })
            }
            _ => None,
        }
    }
}

pub(crate) fn particle_limit_for_particle(particle_id: &str) -> Option<ParticleLimitDescriptor> {
    match particle_id {
        "minecraft:spore_blossom_air" => Some(ParticleLimitDescriptor::SporeBlossom),
        _ => None,
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
            Self::AttackSweep => {
                let color = random.next_f32() * 0.6 + 0.4;
                let size = 1.0 - command_velocity[0] as f32 * 0.5;
                ParticleVisualState::new(
                    size,
                    [color, color, color, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::SingleQuadRandomScaled {
                min_scale,
                max_scale,
                color,
                quad_size_curve,
            } => {
                let scale = sample_range(random, min_scale, max_scale);
                ParticleVisualState::new(
                    base_quad_size * scale,
                    color.sample(random),
                    quad_size_curve,
                )
            }
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
            Self::Snowflake => {
                let size = 0.1 * (random.next_f32() * random.next_f32() + 1.0);
                ParticleVisualState::new(
                    size,
                    [0.923, 0.964, 0.999, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
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
            Self::FlyTowardsPosition => {
                let size = 0.1 * (random.next_f32() * 0.5 + 0.2);
                let brightness = random.next_f32() * 0.6 + 0.4;
                ParticleVisualState::new(
                    size,
                    [brightness * 0.9, brightness * 0.9, brightness, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::OminousSpawn => {
                let size = 0.1 * (random.next_f32() * 0.5 + 0.2) * sample_range(random, 3.0, 5.0);
                ParticleVisualState::new(
                    size,
                    [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0, 1.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::VaultConnection => {
                let size = 0.1 * (random.next_f32() * 0.5 + 0.2) * 1.5;
                let brightness = random.next_f32() * 0.6 + 0.4;
                ParticleVisualState::new(
                    size,
                    [brightness * 0.9, brightness * 0.9, brightness, 0.0],
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::Shriek => {
                ParticleVisualState::new(0.85, WHITE_PARTICLE_COLOR, ParticleQuadSizeCurve::Shriek)
            }
            Self::Totem => {
                let color = if random.next_index(4) == Some(0) {
                    [
                        0.6 + random.next_f32() * 0.2,
                        0.6 + random.next_f32() * 0.3,
                        random.next_f32() * 0.2,
                        1.0,
                    ]
                } else {
                    [
                        0.1 + random.next_f32() * 0.2,
                        0.4 + random.next_f32() * 0.3,
                        random.next_f32() * 0.2,
                        1.0,
                    ]
                };
                ParticleVisualState::new(
                    base_quad_size * 0.75,
                    color,
                    ParticleQuadSizeCurve::Constant,
                )
            }
            Self::Portal => {
                let portal_size = base_quad_size * 0.2 + 0.03;
                let brightness = random.next_f32() * 0.6 + 0.4;
                ParticleVisualState::new(
                    portal_size,
                    [brightness * 0.9, brightness * 0.3, brightness, 1.0],
                    ParticleQuadSizeCurve::Portal,
                )
            }
            Self::ReversePortal => {
                let portal_size = (base_quad_size * 0.2 + 0.03) * 1.5;
                let brightness = random.next_f32() * 0.6 + 0.4;
                ParticleVisualState::new(
                    portal_size,
                    [brightness * 0.9, brightness * 0.3, brightness, 1.0],
                    ParticleQuadSizeCurve::ReversePortal,
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
            Self::FallingLeaves { scale, color } => {
                let size = scale * if random.next_bool() { 0.05 } else { 0.075 };
                ParticleVisualState::new(
                    size,
                    color.sample(random),
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
            Self::Fixed(velocity) => velocity,
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
            Self::CommandWithYOffset { y_offset } => [
                command_velocity[0],
                command_velocity[1] + y_offset,
                command_velocity[2],
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
            Self::ParticleConstructorZero => {
                let x = random_signed_velocity(random);
                let y = random_signed_velocity(random);
                let z = random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                if length == 0.0 {
                    return [0.0, 0.1, 0.0];
                }
                [
                    x / length * speed * 0.4,
                    y / length * speed * 0.4 + 0.1,
                    z / length * speed * 0.4,
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
            Self::TerrainDustPillar => [
                random.next_gaussian() / 30.0,
                command_velocity[1] + random.next_gaussian() / 2.0,
                random.next_gaussian() / 30.0,
            ],
            Self::CampfireSmoke => [
                command_velocity[0],
                command_velocity[1] + f64::from(random.next_f32()) / 500.0,
                command_velocity[2],
            ],
            Self::WaterDrop => sample_water_drop_velocity(random),
            Self::SplashWaterDrop => {
                if command_velocity[1] == 0.0
                    && (command_velocity[0] != 0.0 || command_velocity[2] != 0.0)
                {
                    [command_velocity[0], 0.1, command_velocity[2]]
                } else {
                    sample_water_drop_velocity(random)
                }
            }
            Self::CrimsonSpore => [
                random.next_gaussian() * 1.0E-6,
                random.next_gaussian() * 1.0E-4,
                random.next_gaussian() * 1.0E-6,
            ],
            Self::WarpedSpore => [
                0.0,
                f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
                0.0,
            ],
            Self::Spell | Self::GlowSquid => {
                sample_random_horizontal_y_velocity(command_velocity, random)
            }
            Self::Lava => {
                let x = random_signed_velocity(random);
                let y = random_signed_velocity(random);
                let z = random_signed_velocity(random);
                let speed =
                    (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
                let length = (x * x + y * y + z * z).sqrt();
                let [x, _, z] = if length == 0.0 {
                    [0.0, 0.1, 0.0]
                } else {
                    [
                        x / length * speed * 0.4,
                        y / length * speed * 0.4 + 0.1,
                        z / length * speed * 0.4,
                    ]
                };
                [x * 0.8, f64::from(random.next_f32()) * 0.4 + 0.05, z * 0.8]
            }
            Self::Firefly => sample_firefly_velocity(command_velocity, random),
            Self::BaseAshSmokeSpread {
                dir,
                provider_offset,
            } => {
                // BaseAshSmokeParticle: `super(..., 0.0, 0.0, 0.0, ...)` runs the
                // Particle 7-arg zero-aux base spread, then per-axis
                // `xd *= dirX; yd *= dirY; zd *= dirZ` and finally
                // `xd += xa; yd += ya; zd += za` (the provider velocity).
                let base = ParticleInitialVelocityDescriptor::ParticleConstructorZero
                    .sample([0.0; 3], random);
                let offset = match provider_offset {
                    // AshParticle.Provider forces provider velocity to (0, 0, 0).
                    BaseAshSmokeOffset::Zero => [0.0, 0.0, 0.0],
                    // WhiteAshParticle.Provider ignores the command velocity and
                    // draws its own negative-biased xa/ya/za.
                    BaseAshSmokeOffset::WhiteAsh => [
                        f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
                        f64::from(random.next_f32())
                            * -0.5
                            * f64::from(random.next_f32())
                            * 0.1
                            * 5.0,
                        f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
                    ],
                    // DustPlumeParticle.Provider passes the command velocity as
                    // xa/ya/za and the ctor adds y_offset to ya. Draws no RNG.
                    BaseAshSmokeOffset::CommandWithYOffset { y_offset } => [
                        command_velocity[0],
                        command_velocity[1] + y_offset,
                        command_velocity[2],
                    ],
                };
                [
                    base[0] * dir[0] + offset[0],
                    base[1] * dir[1] + offset[1],
                    base[2] * dir[2] + offset[2],
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
            Self::FixedRgbMinusRandom {
                rgb: [red, green, blue],
                max_subtract,
            } => {
                let shift = random.next_f32() * max_subtract;
                [red - shift, green - shift, blue - shift, 1.0]
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
            Self::SixOverRandom => ((6.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
            Self::EightOverRandom => ((8.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
            Self::SixteenOverRandom => ((16.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
            Self::SixteenOverRandomPlusTwo => {
                ((16.0 / (random.next_f32() * 0.8 + 0.2)) as u32 + 2).max(1)
            }
            Self::FortyOverRandom => ((40.0 / (random.next_f32() * 0.8 + 0.2)) as u32).max(1),
            Self::SporeBlossomAir => {
                random.next_f32();
                500 + random.next_index(501).unwrap_or(0) as u32
            }
            Self::TrialSpawnerDetection => (12.0 / (random.next_f32() * 0.5 + 0.5)) as u32,
            Self::Portal => (random.next_f32() * 10.0) as u32 + 40,
            Self::ReversePortal => {
                random.next_f32();
                (random.next_f32() * 2.0) as u32 + 60
            }
            Self::RandomFloatSpan { min, span } => min + (random.next_f32() * span as f32) as u32,
            Self::RandomFloatDivisor {
                numerator,
                min_tenths,
                span_tenths,
            } => {
                let divisor = (min_tenths as f32 + random.next_f32() * span_tenths as f32) / 10.0;
                (numerator as f32 / divisor) as u32
            }
            Self::RandomInclusive { min, max } => {
                let span = max.saturating_sub(min).saturating_add(1);
                min + random.next_index(span as usize).unwrap_or(0) as u32
            }
            Self::InclusiveTick { vanilla_lifetime } => vanilla_lifetime.saturating_add(1),
            Self::FallingDust => {
                let base_lifetime = (32.0 / (random.next_f32() * 0.8 + 0.2)) as u32;
                ((base_lifetime as f32 * 0.9).max(1.0)) as u32
            }
            Self::CommandOption { fallback } => fallback,
            Self::DustScale { fallback_scale } => dust_lifetime(random, fallback_scale as f32),
            Self::Explode => (16.0 / (random.next_f64() * 0.8 + 0.2)) as u32 + 2,
        }
    }
}

pub(crate) fn dust_lifetime(random: &mut ParticleRandom, scale: f32) -> u32 {
    let scale = scale.clamp(0.01, 4.0);
    let base_lifetime = (8.0 / (random.next_f64() * 0.8 + 0.2)) as u32;
    ((base_lifetime as f32 * scale).max(1.0)) as u32
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
const TERRAIN_PARTICLE_RGB: [f32; 3] = [0.6, 0.6, 0.6];

impl ParticleRandom {
    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
            next_next_gaussian: None,
        }
    }

    pub(crate) fn next_f64(&mut self) -> f64 {
        f64::from(self.next_bits(24)) / f64::from(1_u32 << 24)
    }

    pub(crate) fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }

    pub(crate) fn next_bool(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    pub(crate) fn next_i64(&mut self) -> i64 {
        let high = (self.next_bits(32) as i32 as i64) << 32;
        let low = self.next_bits(32) as i32 as i64;
        high.wrapping_add(low)
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(next) = self.next_next_gaussian.take() {
            return next;
        }

        let (v1, v2, s) = loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                break (v1, v2, s);
            }
        };
        let multiplier = (-2.0 * s.ln() / s).sqrt();
        self.next_next_gaussian = Some(v2 * multiplier);
        v1 * multiplier
    }

    pub(crate) fn next_double(&mut self) -> f64 {
        let high = u64::from(self.next_bits(26));
        let low = u64::from(self.next_bits(27));
        ((high << 27) | low) as f64 / (1_u64 << 53) as f64
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

fn falling_leaves_descriptor_for_particle(particle_id: &str) -> FallingLeavesDescriptor {
    match particle_id {
        "minecraft:cherry_leaves" => cherry_falling_leaves_descriptor(),
        "minecraft:pale_oak_leaves" | "minecraft:tinted_leaves" => {
            pale_oak_falling_leaves_descriptor()
        }
        _ => unreachable!("falling leaves descriptor requested for {particle_id}"),
    }
}

fn cherry_falling_leaves_descriptor() -> FallingLeavesDescriptor {
    FallingLeavesDescriptor {
        fall_acceleration: 0.25,
        side_acceleration: 2.0,
        swirl: false,
        flow_away: true,
        scale: 1.0,
        start_velocity: 0.0,
    }
}

fn pale_oak_falling_leaves_descriptor() -> FallingLeavesDescriptor {
    FallingLeavesDescriptor {
        fall_acceleration: 0.07,
        side_acceleration: 10.0,
        swirl: true,
        flow_away: false,
        scale: 2.0,
        start_velocity: 0.021,
    }
}

fn terrain_particle_descriptor(
    provider: &'static str,
    lifetime: ParticleLifetimeDescriptor,
    initial_velocity: ParticleInitialVelocityDescriptor,
) -> ParticleDescriptor {
    ParticleDescriptor {
        provider,
        lifetime,
        sprite_selection: ParticleSpriteSelection::First,
        visual: ParticleVisualDescriptor::SingleQuadScaled {
            scale: 0.5,
            color: ParticleColorDescriptor::FixedRgb(TERRAIN_PARTICLE_RGB),
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        },
        initial_velocity,
        friction: 0.98,
        gravity: 1.0,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
    }
}

fn breaking_item_particle_descriptor(
    provider: &'static str,
    initial_velocity: ParticleInitialVelocityDescriptor,
) -> ParticleDescriptor {
    ParticleDescriptor {
        provider,
        lifetime: ParticleLifetimeDescriptor::BaseParticle,
        sprite_selection: ParticleSpriteSelection::Random,
        visual: ParticleVisualDescriptor::SingleQuadScaled {
            scale: 0.5,
            color: ParticleColorDescriptor::FixedRgb([1.0, 1.0, 1.0]),
            quad_size_curve: ParticleQuadSizeCurve::Constant,
        },
        initial_velocity,
        friction: 0.98,
        gravity: 1.0,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
    }
}

fn sample_water_drop_velocity(random: &mut ParticleRandom) -> [f64; 3] {
    let velocity =
        ParticleInitialVelocityDescriptor::ParticleConstructorZero.sample([0.0; 3], random);
    [
        velocity[0] * 0.3,
        f64::from(random.next_f32()) * 0.2 + 0.1,
        velocity[2] * 0.3,
    ]
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

fn sample_firefly_velocity(command_velocity: [f64; 3], random: &mut ParticleRandom) -> [f64; 3] {
    let x = 0.5 - random.next_double();
    let y = if random.next_bool() {
        command_velocity[1]
    } else {
        -command_velocity[1]
    };
    let z = 0.5 - random.next_double();
    let x = x + random_signed_velocity(random);
    let y = y + random_signed_velocity(random);
    let z = z + random_signed_velocity(random);
    let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
    let length = (x * x + y * y + z * z).sqrt();
    let velocity = if length == 0.0 {
        [0.0, 0.1, 0.0]
    } else {
        [
            x / length * speed * 0.4,
            y / length * speed * 0.4 + 0.1,
            z / length * speed * 0.4,
        ]
    };
    [velocity[0] * 0.8, velocity[1] * 0.8, velocity[2] * 0.8]
}

#[cfg(test)]
mod tests;
