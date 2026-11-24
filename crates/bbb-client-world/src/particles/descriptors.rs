use super::ParticleKind;
use crate::TerrainFluidKind;

mod sampling;
pub(crate) use sampling::*;

pub(crate) const DEFAULT_PARTICLE_RANDOM_SEED: i64 = 0x5EED_2601;
pub(crate) const END_ROD_FADE_COLOR: [f32; 3] = [242.0 / 255.0, 222.0 / 255.0, 201.0 / 255.0];
pub(crate) const SQUID_INK_AIR_DOWNWARD_ACCELERATION: f64 = 0.0074;
pub(crate) const VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT: usize = 1000;

/// Vanilla particle provider classes (`ParticleResources.registerProviders`); several
/// types share one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Ash,
    AttackSweep,
    Base,
    BlockMarker,
    BreakingItem,
    BreakingItemCobweb,
    BreakingItemSlime,
    BreakingItemSnowball,
    Bubble,
    BubbleColumnUp,
    BubblePop,
    CampfireSmokeCosy,
    CampfireSmokeSignal,
    Crit,
    CritDamageIndicator,
    CritMagic,
    DragonBreath,
    DripHoneyFall,
    DripHoneyHang,
    DripHoneyLand,
    DripLavaFall,
    DripLavaHang,
    DripLavaLand,
    DripNectarFall,
    DripObsidianTearFall,
    DripObsidianTearHang,
    DripObsidianTearLand,
    DripSporeBlossomFall,
    DripWaterFall,
    DripWaterHang,
    DripstoneLavaFall,
    DripstoneLavaHang,
    DripstoneWaterFall,
    DripstoneWaterHang,
    Dust,
    DustColorTransition,
    DustPlume,
    ElderGuardian,
    EndRod,
    Explode,
    FallingDust,
    FallingLeavesCherry,
    FallingLeavesPaleOak,
    FallingLeavesTintedLeaves,
    Firefly,
    FireworkFlash,
    FireworkSpark,
    Flame,
    FlameSmallFlame,
    FlyStraightTowardsOminousSpawn,
    FlyTowardsPositionEnchant,
    FlyTowardsPositionNautilus,
    FlyTowardsPositionVaultConnection,
    GlowElectricSpark,
    GlowScrape,
    GlowSquid,
    GlowWaxOff,
    GlowWaxOn,
    Gust,
    GustSeedLarge,
    GustSeedSmall,
    GustSmall,
    Heart,
    HeartAngryVillager,
    HugeExplosion,
    HugeExplosionSeed,
    ItemPickup,
    LargeSmoke,
    Lava,
    Note,
    PlayerCloud,
    PlayerCloudSneeze,
    Portal,
    ReversePortal,
    SculkCharge,
    SculkChargePop,
    Shriek,
    SimpleVerticalPauseMobGrowth,
    SimpleVerticalResetMobGrowth,
    Smoke,
    Snowflake,
    SonicBoom,
    Soul,
    SoulEmissive,
    Spell,
    SpellInstant,
    SpellMobEffect,
    SpellWitch,
    Spit,
    Splash,
    SquidInk,
    SquidInkGlowInk,
    SuspendedCrimsonSpore,
    SuspendedSporeBlossomAir,
    SuspendedTown,
    SuspendedTownComposterFill,
    SuspendedTownDolphinSpeed,
    SuspendedTownEggCrack,
    SuspendedTownHappyVillager,
    SuspendedUnderwater,
    SuspendedWarpedSpore,
    Terrain,
    TerrainCrumbling,
    TerrainDustPillar,
    Totem,
    Trail,
    TrialSpawnerDetection,
    VibrationSignal,
    Wake,
    WaterCurrentDown,
    WaterDrop,
    WhiteAsh,
    WhiteSmoke,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParticleDescriptor {
    pub(crate) provider: Provider,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ParticleLightEmissionDescriptor {
    #[default]
    World,
    FullBright,
    FullBlock,
    SmoothBlockByAge,
    SmoothBlockByAgeQuartic,
    Firefly,
}

pub use bbb_render_types::ParticleFacingCameraMode;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ParticleLimitDescriptor {
    SporeBlossom,
}

impl ParticleLimitDescriptor {
    pub(crate) const COUNT: usize = 1;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    pub(crate) fn for_kind(kind: ParticleKind) -> Self {
        match kind {
            ParticleKind::ANGRY_VILLAGER => Self {
                provider: Provider::HeartAngryVillager,
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
            ParticleKind::BUBBLE => Self {
                provider: Provider::Bubble,
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
            ParticleKind::BUBBLE_COLUMN_UP => Self {
                provider: Provider::BubbleColumnUp,
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
            ParticleKind::CURRENT_DOWN => Self {
                provider: Provider::WaterCurrentDown,
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
            ParticleKind::RAIN => Self {
                provider: Provider::WaterDrop,
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::WaterDrop,
                friction: 0.98,
                gravity: 0.06,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::SPLASH => Self {
                provider: Provider::Splash,
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::SplashWaterDrop,
                friction: 0.98,
                gravity: 0.04,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::FISHING => Self {
                provider: Provider::Wake,
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::OMINOUS_SPAWNING => Self {
                provider: Provider::FlyStraightTowardsOminousSpawn,
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 25, span: 5 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::OminousSpawn,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::BUBBLE_POP => Self {
                provider: Provider::BubblePop,
                lifetime: ParticleLifetimeDescriptor::Fixed(4),
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.008,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::FIREFLY => Self {
                provider: Provider::Firefly,
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
            ParticleKind::DUST | ParticleKind::DUST_COLOR_TRANSITION => Self {
                provider: if kind == ParticleKind::DUST_COLOR_TRANSITION {
                    Provider::DustColorTransition
                } else {
                    Provider::Dust
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
            ParticleKind::SWEEP_ATTACK => Self {
                provider: Provider::AttackSweep,
                lifetime: ParticleLifetimeDescriptor::Fixed(4),
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::AttackSweep,
                initial_velocity: ParticleInitialVelocityDescriptor::ParticleConstructorZero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::UNDERWATER => Self {
                provider: Provider::SuspendedUnderwater,
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
            ParticleKind::SPORE_BLOSSOM_AIR => Self {
                provider: Provider::SuspendedSporeBlossomAir,
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
            ParticleKind::FALLING_NECTAR => Self {
                provider: Provider::DripNectarFall,
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
            ParticleKind::FALLING_SPORE_BLOSSOM => Self {
                provider: Provider::DripSporeBlossomFall,
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
            ParticleKind::DRIPPING_HONEY => Self {
                provider: Provider::DripHoneyHang,
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
            ParticleKind::FALLING_HONEY => Self {
                provider: Provider::DripHoneyFall,
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
            ParticleKind::LANDING_HONEY => Self {
                provider: Provider::DripHoneyLand,
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
            ParticleKind::DRIPPING_OBSIDIAN_TEAR => Self {
                provider: Provider::DripObsidianTearHang,
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
            ParticleKind::FALLING_OBSIDIAN_TEAR => Self {
                provider: Provider::DripObsidianTearFall,
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
            ParticleKind::LANDING_OBSIDIAN_TEAR => Self {
                provider: Provider::DripObsidianTearLand,
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
            ParticleKind::DRIPPING_LAVA => Self {
                provider: Provider::DripLavaHang,
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
            ParticleKind::FALLING_LAVA => Self {
                provider: Provider::DripLavaFall,
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
            ParticleKind::LANDING_LAVA => Self {
                provider: Provider::DripLavaLand,
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
            ParticleKind::DRIPPING_WATER => Self {
                provider: Provider::DripWaterHang,
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
            ParticleKind::FALLING_WATER => Self {
                provider: Provider::DripWaterFall,
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
            ParticleKind::DRIPPING_DRIPSTONE_LAVA => Self {
                provider: Provider::DripstoneLavaHang,
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
            ParticleKind::FALLING_DRIPSTONE_LAVA => Self {
                provider: Provider::DripstoneLavaFall,
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
            ParticleKind::DRIPPING_DRIPSTONE_WATER => Self {
                provider: Provider::DripstoneWaterHang,
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
            ParticleKind::FALLING_DRIPSTONE_WATER => Self {
                provider: Provider::DripstoneWaterFall,
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
            ParticleKind::CRIMSON_SPORE | ParticleKind::WARPED_SPORE => Self {
                provider: if kind == ParticleKind::CRIMSON_SPORE {
                    Provider::SuspendedCrimsonSpore
                } else {
                    Provider::SuspendedWarpedSpore
                },
                lifetime: ParticleLifetimeDescriptor::SixteenOverRandom,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadRandomScaled {
                    min_scale: 0.6,
                    max_scale: 1.2,
                    color: ParticleColorDescriptor::FixedRgb(if kind
                        == ParticleKind::CRIMSON_SPORE
                    {
                        [0.9, 0.4, 0.5]
                    } else {
                        [0.1, 0.1, 0.3]
                    }),
                    quad_size_curve: ParticleQuadSizeCurve::Constant,
                },
                initial_velocity: if kind == ParticleKind::CRIMSON_SPORE {
                    ParticleInitialVelocityDescriptor::CrimsonSpore
                } else {
                    ParticleInitialVelocityDescriptor::WarpedSpore
                },
                friction: 1.0,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::CLOUD => Self {
                provider: Provider::PlayerCloud,
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
            ParticleKind::SNEEZE => Self {
                provider: Provider::PlayerCloudSneeze,
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
            ParticleKind::SNOWFLAKE => Self {
                provider: Provider::Snowflake,
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
            ParticleKind::SQUID_INK | ParticleKind::GLOW_SQUID_INK => Self {
                provider: if kind == ParticleKind::GLOW_SQUID_INK {
                    Provider::SquidInkGlowInk
                } else {
                    Provider::SquidInk
                },
                lifetime: ParticleLifetimeDescriptor::SixOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::FixedQuad {
                    size: 0.5,
                    color: if kind == ParticleKind::GLOW_SQUID_INK {
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
            ParticleKind::CRIT => Self {
                provider: Provider::Crit,
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
            ParticleKind::DAMAGE_INDICATOR => Self {
                provider: Provider::CritDamageIndicator,
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
            ParticleKind::ENCHANTED_HIT => Self {
                provider: Provider::CritMagic,
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
            ParticleKind::ENCHANT | ParticleKind::NAUTILUS => Self {
                provider: if kind == ParticleKind::NAUTILUS {
                    Provider::FlyTowardsPositionNautilus
                } else {
                    Provider::FlyTowardsPositionEnchant
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
            ParticleKind::VAULT_CONNECTION => Self {
                provider: Provider::FlyTowardsPositionVaultConnection,
                lifetime: ParticleLifetimeDescriptor::RandomFloatSpan { min: 30, span: 10 },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::VaultConnection,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::TOTEM_OF_UNDYING => Self {
                provider: Provider::Totem,
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 60, max: 71 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::Totem,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.6,
                gravity: 1.25,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::DRAGON_BREATH => Self {
                provider: Provider::DragonBreath,
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
            ParticleKind::END_ROD => Self {
                provider: Provider::EndRod,
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
            ParticleKind::EXPLOSION => Self {
                provider: Provider::HugeExplosion,
                lifetime: ParticleLifetimeDescriptor::RandomInclusive { min: 6, max: 9 },
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::HugeExplosion,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::EXPLOSION_EMITTER => Self {
                provider: Provider::HugeExplosionSeed,
                lifetime: ParticleLifetimeDescriptor::Fixed(8),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::ELDER_GUARDIAN => Self {
                provider: Provider::ElderGuardian,
                lifetime: ParticleLifetimeDescriptor::Fixed(30),
                sprite_selection: ParticleSpriteSelection::First,
                visual: ParticleVisualDescriptor::BaseSingleQuad,
                initial_velocity: ParticleInitialVelocityDescriptor::Zero,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::SONIC_BOOM => Self {
                provider: Provider::SonicBoom,
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
            ParticleKind::SCULK_CHARGE => Self {
                provider: Provider::SculkCharge,
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
            ParticleKind::SCULK_CHARGE_POP => Self {
                provider: Provider::SculkChargePop,
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
            ParticleKind::SHRIEK => Self {
                provider: Provider::Shriek,
                lifetime: ParticleLifetimeDescriptor::Fixed(30),
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Shriek,
                initial_velocity: ParticleInitialVelocityDescriptor::Fixed([0.0, 0.1, 0.0]),
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::TRIAL_SPAWNER_DETECTED_PLAYER
            | ParticleKind::TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS => Self {
                provider: Provider::TrialSpawnerDetection,
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
            ParticleKind::GUST => Self {
                provider: Provider::Gust,
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
            ParticleKind::GUST_EMITTER_LARGE => Self {
                provider: Provider::GustSeedLarge,
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
            ParticleKind::GUST_EMITTER_SMALL => Self {
                provider: Provider::GustSeedSmall,
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
            ParticleKind::SMALL_GUST => Self {
                provider: Provider::GustSmall,
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
            ParticleKind::DOLPHIN => Self {
                provider: Provider::SuspendedTownDolphinSpeed,
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
            ParticleKind::FLAME | ParticleKind::SOUL_FIRE_FLAME | ParticleKind::COPPER_FIRE_FLAME => Self {
                provider: Provider::Flame,
                lifetime: ParticleLifetimeDescriptor::Rising,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Flame { scale: 1.0 },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::LAVA => Self {
                provider: Provider::Lava,
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
            ParticleKind::SOUL | ParticleKind::SCULK_SOUL => Self {
                provider: if kind == ParticleKind::SCULK_SOUL {
                    Provider::SoulEmissive
                } else {
                    Provider::Soul
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
            ParticleKind::GLOW => Self {
                provider: Provider::GlowSquid,
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
            ParticleKind::ELECTRIC_SPARK => Self {
                provider: Provider::GlowElectricSpark,
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
            ParticleKind::SCRAPE => Self {
                provider: Provider::GlowScrape,
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
            ParticleKind::WAX_OFF => Self {
                provider: Provider::GlowWaxOff,
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
            ParticleKind::WAX_ON => Self {
                provider: Provider::GlowWaxOn,
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
            ParticleKind::HEART => Self {
                provider: Provider::Heart,
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
            ParticleKind::FLASH => Self {
                provider: Provider::FireworkFlash,
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
            ParticleKind::FIREWORK => Self {
                provider: Provider::FireworkSpark,
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
            ParticleKind::TRAIL => Self {
                provider: Provider::Trail,
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
            ParticleKind::VIBRATION => Self {
                provider: Provider::VibrationSignal,
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
            ParticleKind::EFFECT
            | ParticleKind::INSTANT_EFFECT
            | ParticleKind::ENTITY_EFFECT
            | ParticleKind::INFESTED
            | ParticleKind::RAID_OMEN
            | ParticleKind::TRIAL_OMEN => Self {
                provider: match kind {
                    ParticleKind::EFFECT | ParticleKind::INSTANT_EFFECT => {
                        Provider::SpellInstant
                    }
                    ParticleKind::ENTITY_EFFECT => Provider::SpellMobEffect,
                    _ => Provider::Spell,
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
            ParticleKind::CHERRY_LEAVES
            | ParticleKind::PALE_OAK_LEAVES
            | ParticleKind::TINTED_LEAVES => {
                let falling = falling_leaves_descriptor_for_kind(kind);
                Self {
                    provider: match kind {
                        ParticleKind::CHERRY_LEAVES => Provider::FallingLeavesCherry,
                        ParticleKind::PALE_OAK_LEAVES => Provider::FallingLeavesPaleOak,
                        _ => Provider::FallingLeavesTintedLeaves,
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
            ParticleKind::PAUSE_MOB_GROWTH | ParticleKind::RESET_MOB_GROWTH => Self {
                provider: if kind == ParticleKind::RESET_MOB_GROWTH {
                    Provider::SimpleVerticalResetMobGrowth
                } else {
                    Provider::SimpleVerticalPauseMobGrowth
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
                    y_offset: if kind == ParticleKind::RESET_MOB_GROWTH {
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
            ParticleKind::WITCH => Self {
                provider: Provider::SpellWitch,
                lifetime: ParticleLifetimeDescriptor::EightOverRandom,
                sprite_selection: ParticleSpriteSelection::Age,
                visual: ParticleVisualDescriptor::WitchSpell,
                initial_velocity: ParticleInitialVelocityDescriptor::Spell,
                friction: 0.96,
                gravity: -0.1,
                has_physics: false,
                speed_up_when_y_motion_is_blocked: true,
            },
            ParticleKind::SMALL_FLAME => Self {
                provider: Provider::FlameSmallFlame,
                lifetime: ParticleLifetimeDescriptor::Rising,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Flame { scale: 0.5 },
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.96,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::CAMPFIRE_COSY_SMOKE | ParticleKind::CAMPFIRE_SIGNAL_SMOKE => Self {
                provider: if kind == ParticleKind::CAMPFIRE_SIGNAL_SMOKE {
                    Provider::CampfireSmokeSignal
                } else {
                    Provider::CampfireSmokeCosy
                },
                lifetime: if kind == ParticleKind::CAMPFIRE_SIGNAL_SMOKE {
                    ParticleLifetimeDescriptor::RandomInclusive { min: 280, max: 329 }
                } else {
                    ParticleLifetimeDescriptor::RandomInclusive { min: 80, max: 129 }
                },
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::SingleQuadScaled {
                    scale: 3.0,
                    color: ParticleColorDescriptor::FixedRgba(if kind
                        == ParticleKind::CAMPFIRE_SIGNAL_SMOKE
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
            ParticleKind::LARGE_SMOKE => Self {
                provider: Provider::LargeSmoke,
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
            ParticleKind::SMOKE => Self {
                provider: Provider::Smoke,
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
            ParticleKind::WHITE_SMOKE => Self {
                provider: Provider::WhiteSmoke,
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
            ParticleKind::ASH => Self {
                provider: Provider::Ash,
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
            ParticleKind::WHITE_ASH => Self {
                provider: Provider::WhiteAsh,
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
            ParticleKind::DUST_PLUME => Self {
                provider: Provider::DustPlume,
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
            ParticleKind::HAPPY_VILLAGER => Self {
                provider: Provider::SuspendedTownHappyVillager,
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
            ParticleKind::COMPOSTER => Self {
                provider: Provider::SuspendedTownComposterFill,
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
            ParticleKind::MYCELIUM => Self {
                provider: Provider::SuspendedTown,
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
            ParticleKind::NOTE => Self {
                provider: Provider::Note,
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
            ParticleKind::EGG_CRACK => Self {
                provider: Provider::SuspendedTownEggCrack,
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
            ParticleKind::POOF => Self {
                provider: Provider::Explode,
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
            ParticleKind::PORTAL => Self {
                provider: Provider::Portal,
                lifetime: ParticleLifetimeDescriptor::Portal,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::Portal,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::REVERSE_PORTAL => Self {
                provider: Provider::ReversePortal,
                lifetime: ParticleLifetimeDescriptor::ReversePortal,
                sprite_selection: ParticleSpriteSelection::Random,
                visual: ParticleVisualDescriptor::ReversePortal,
                initial_velocity: ParticleInitialVelocityDescriptor::Command,
                friction: 0.98,
                gravity: 0.0,
                has_physics: true,
                speed_up_when_y_motion_is_blocked: false,
            },
            ParticleKind::SPIT => Self {
                provider: Provider::Spit,
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
            ParticleKind::BLOCK => terrain_particle_descriptor(
                Provider::Terrain,
                ParticleLifetimeDescriptor::BaseParticle,
                ParticleInitialVelocityDescriptor::ParticleConstructorScaled { scale: 1.0 },
            ),
            ParticleKind::BLOCK_MARKER => Self {
                provider: Provider::BlockMarker,
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
            ParticleKind::DUST_PILLAR => terrain_particle_descriptor(
                Provider::TerrainDustPillar,
                ParticleLifetimeDescriptor::RandomInclusive { min: 20, max: 39 },
                ParticleInitialVelocityDescriptor::TerrainDustPillar,
            ),
            ParticleKind::BLOCK_CRUMBLE => terrain_particle_descriptor(
                Provider::TerrainCrumbling,
                ParticleLifetimeDescriptor::RandomInclusive { min: 1, max: 10 },
                ParticleInitialVelocityDescriptor::Zero,
            ),
            ParticleKind::FALLING_DUST => Self {
                provider: Provider::FallingDust,
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
            ParticleKind::ITEM => breaking_item_particle_descriptor(
                Provider::BreakingItem,
                ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                    scale: 0.1,
                },
            ),
            ParticleKind::ITEM_SLIME => breaking_item_particle_descriptor(
                Provider::BreakingItemSlime,
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            ParticleKind::ITEM_COBWEB => breaking_item_particle_descriptor(
                Provider::BreakingItemCobweb,
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            ParticleKind::ITEM_SNOWBALL => breaking_item_particle_descriptor(
                Provider::BreakingItemSnowball,
                ParticleInitialVelocityDescriptor::ParticleConstructorZero,
            ),
            ParticleKind::ITEM_PICKUP => Self {
                provider: Provider::ItemPickup,
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
                provider: Provider::Base,
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
            Provider::HeartAngryVillager => [
                command_position[0],
                command_position[1] + 0.5,
                command_position[2],
            ],
            Provider::Soul | Provider::SoulEmissive => [
                command_position[0] + random_centered_offset(random, 0.05),
                command_position[1] + random_centered_offset(random, 0.05),
                command_position[2] + random_centered_offset(random, 0.05),
            ],
            Provider::SuspendedUnderwater
            | Provider::SuspendedSporeBlossomAir
            | Provider::SuspendedCrimsonSpore
            | Provider::SuspendedWarpedSpore => [
                command_position[0],
                command_position[1] - 0.125,
                command_position[2],
            ],
            _ => command_position,
        }
    }

    pub(crate) fn tick_motion(self) -> ParticleTickMotionDescriptor {
        match self.provider {
            Provider::BubblePop => ParticleTickMotionDescriptor::DirectGravityNoFriction,
            Provider::AttackSweep => ParticleTickMotionDescriptor::NoMotion,
            Provider::WaterCurrentDown => ParticleTickMotionDescriptor::CurrentDown,
            Provider::Snowflake => ParticleTickMotionDescriptor::Snowflake,
            Provider::FlyTowardsPositionEnchant
            | Provider::FlyTowardsPositionNautilus
            | Provider::FlyTowardsPositionVaultConnection => {
                ParticleTickMotionDescriptor::FlyTowardsPosition
            }
            Provider::Trail => ParticleTickMotionDescriptor::TrailTarget,
            Provider::VibrationSignal => ParticleTickMotionDescriptor::VibrationSignal,
            Provider::FlyStraightTowardsOminousSpawn => {
                ParticleTickMotionDescriptor::FlyStraightTowards
            }
            Provider::CampfireSmokeCosy | Provider::CampfireSmokeSignal => {
                ParticleTickMotionDescriptor::CampfireSmoke
            }
            Provider::DripHoneyHang
            | Provider::DripObsidianTearHang
            | Provider::DripWaterHang
            | Provider::DripstoneWaterHang => ParticleTickMotionDescriptor::DripHang,
            Provider::DripLavaHang | Provider::DripstoneLavaHang => {
                ParticleTickMotionDescriptor::CoolingDripHang
            }
            Provider::DustPlume => ParticleTickMotionDescriptor::DustPlume,
            Provider::WaterDrop | Provider::Splash => ParticleTickMotionDescriptor::WaterDrop,
            Provider::DripNectarFall | Provider::DripSporeBlossomFall => {
                ParticleTickMotionDescriptor::DripFalling
            }
            Provider::DripHoneyFall
            | Provider::DripObsidianTearFall
            | Provider::DripLavaFall
            | Provider::DripWaterFall
            | Provider::DripstoneLavaFall
            | Provider::DripstoneWaterFall => ParticleTickMotionDescriptor::DripFallAndLand,
            Provider::DripHoneyLand | Provider::DripObsidianTearLand | Provider::DripLavaLand => {
                ParticleTickMotionDescriptor::DripLand
            }
            Provider::Wake => ParticleTickMotionDescriptor::Wake,
            Provider::Portal => ParticleTickMotionDescriptor::Portal,
            Provider::ReversePortal => ParticleTickMotionDescriptor::ReversePortal,
            Provider::DragonBreath => ParticleTickMotionDescriptor::DragonBreath,
            Provider::Firefly => ParticleTickMotionDescriptor::Firefly,
            Provider::FallingLeavesCherry
            | Provider::FallingLeavesPaleOak
            | Provider::FallingLeavesTintedLeaves => ParticleTickMotionDescriptor::FallingLeaves,
            Provider::FallingDust => ParticleTickMotionDescriptor::FallingDust,
            Provider::ItemPickup => ParticleTickMotionDescriptor::ItemPickup,
            _ => ParticleTickMotionDescriptor::DefaultParticleTick,
        }
    }

    pub(crate) fn collision_size(self) -> Option<[f32; 2]> {
        match self.provider {
            Provider::CampfireSmokeCosy | Provider::CampfireSmokeSignal => Some([0.25, 0.25]),
            Provider::Wake => Some([0.01, 0.01]),
            // Every `DripParticle.*` provider goes through the shared
            // `DripParticle` constructor `this.setSize(0.01F, 0.01F)`
            // (DripParticle.java:25).
            Provider::DripLavaHang
            | Provider::DripLavaFall
            | Provider::DripLavaLand
            | Provider::DripWaterHang
            | Provider::DripWaterFall
            | Provider::DripHoneyHang
            | Provider::DripHoneyFall
            | Provider::DripHoneyLand
            | Provider::DripNectarFall
            | Provider::DripSporeBlossomFall
            | Provider::DripObsidianTearHang
            | Provider::DripObsidianTearFall
            | Provider::DripObsidianTearLand
            | Provider::DripstoneWaterHang
            | Provider::DripstoneWaterFall
            | Provider::DripstoneLavaHang
            | Provider::DripstoneLavaFall => Some([0.01, 0.01]),
            // `WaterDropParticle` (`minecraft:rain`) `this.setSize(0.01F, 0.01F)`
            // (WaterDropParticle.java:16); `SplashParticle` extends
            // `WaterDropParticle` and inherits it through `super(...)`.
            Provider::WaterDrop | Provider::Splash => Some([0.01, 0.01]),
            // `BubbleParticle` `this.setSize(0.02F, 0.02F)` (BubbleParticle.java:22)
            // and `BubbleColumnUpParticle` `this.setSize(0.02F, 0.02F)`
            // (BubbleColumnUpParticle.java:24).
            Provider::Bubble | Provider::BubbleColumnUp => Some([0.02, 0.02]),
            // `scale(1.5F)` resolves through `Particle.scale`
            // (`this.setSize(0.2F * scale, 0.2F * scale)`, Particle.java:77-80) to
            // `setSize(0.3F, 0.3F)` (`0.2F * 1.5F == 0.3F` exactly in `f32`).
            // Soul (SoulParticle.java:17, both `Provider` and `EmissiveProvider`
            // build a `SoulParticle`) and firefly (FireflyParticle.java:94).
            Provider::Soul | Provider::SoulEmissive | Provider::Firefly => Some([0.3, 0.3]),
            _ => None,
        }
    }

    pub(crate) fn drip_fluid(self) -> Option<TerrainFluidKind> {
        match self.provider {
            Provider::DripWaterHang
            | Provider::DripWaterFall
            | Provider::DripstoneWaterHang
            | Provider::DripstoneWaterFall => Some(TerrainFluidKind::Water),
            Provider::DripLavaHang
            | Provider::DripLavaFall
            | Provider::DripLavaLand
            | Provider::DripstoneLavaHang
            | Provider::DripstoneLavaFall => Some(TerrainFluidKind::Lava),
            _ => None,
        }
    }

    pub(crate) fn required_fluid(self) -> Option<TerrainFluidKind> {
        match self.provider {
            Provider::Bubble | Provider::BubbleColumnUp | Provider::WaterCurrentDown => {
                Some(TerrainFluidKind::Water)
            }
            _ => None,
        }
    }

    pub(crate) fn moves_without_collision(self) -> bool {
        matches!(
            self.provider,
            Provider::EndRod
                | Provider::Flame
                | Provider::FlameSmallFlame
                | Provider::Portal
                | Provider::ReversePortal
                | Provider::SuspendedTownHappyVillager
                | Provider::SuspendedTownComposterFill
                | Provider::SuspendedTownDolphinSpeed
                | Provider::SuspendedTownEggCrack
                | Provider::SuspendedTown
        )
    }

    pub(crate) fn air_downward_acceleration(self) -> f64 {
        match self.provider {
            Provider::SquidInk | Provider::SquidInkGlowInk => SQUID_INK_AIR_DOWNWARD_ACCELERATION,
            _ => 0.0,
        }
    }

    pub(crate) fn falling_leaves(self) -> Option<FallingLeavesDescriptor> {
        match self.provider {
            Provider::FallingLeavesCherry => Some(cherry_falling_leaves_descriptor()),
            Provider::FallingLeavesPaleOak | Provider::FallingLeavesTintedLeaves => {
                Some(pale_oak_falling_leaves_descriptor())
            }
            _ => None,
        }
    }

    pub(crate) fn light_emission(self) -> ParticleLightEmissionDescriptor {
        match self.provider {
            // Vanilla overrides `getLightCoords` to return `LightCoordsUtil.FULL_BRIGHT`.
            Provider::AttackSweep
            | Provider::Totem
            | Provider::SquidInk
            | Provider::SquidInkGlowInk
            | Provider::EndRod
            | Provider::FireworkSpark
            | Provider::HugeExplosion
            | Provider::SonicBoom
            | Provider::Gust
            | Provider::GustSmall
            | Provider::Trail => ParticleLightEmissionDescriptor::FullBright,
            // Vanilla keeps sky light from the world sample and forces block light to 15.
            Provider::Lava
            | Provider::SoulEmissive
            | Provider::SculkCharge
            | Provider::SculkChargePop
            | Provider::DripObsidianTearHang
            | Provider::DripObsidianTearFall
            | Provider::DripObsidianTearLand
            | Provider::Shriek
            | Provider::VibrationSignal
            | Provider::FlyTowardsPositionVaultConnection
            | Provider::TrialSpawnerDetection
            | Provider::FlyStraightTowardsOminousSpawn => {
                ParticleLightEmissionDescriptor::FullBlock
            }
            // Vanilla uses `LightCoordsUtil.addSmoothBlockEmission(..., (age + partialTick) / lifetime)`.
            Provider::Flame
            | Provider::FlameSmallFlame
            | Provider::GlowSquid
            | Provider::GlowElectricSpark
            | Provider::GlowScrape
            | Provider::GlowWaxOff
            | Provider::GlowWaxOn => ParticleLightEmissionDescriptor::SmoothBlockByAge,
            // Vanilla portal particles add `(age / lifetime)^4` smooth block emission.
            Provider::FlyTowardsPositionEnchant
            | Provider::FlyTowardsPositionNautilus
            | Provider::Portal
            | Provider::ReversePortal => ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic,
            Provider::Firefly => ParticleLightEmissionDescriptor::Firefly,
            _ => ParticleLightEmissionDescriptor::World,
        }
    }

    pub(crate) fn facing_camera_mode(self) -> ParticleFacingCameraMode {
        match self.provider {
            Provider::TrialSpawnerDetection => ParticleFacingCameraMode::LookAtY,
            _ => ParticleFacingCameraMode::LookAtXyz,
        }
    }

    pub(crate) fn alpha_curve(self) -> ParticleAlphaCurve {
        match self.provider {
            Provider::Totem | Provider::SquidInk | Provider::SquidInkGlowInk | Provider::EndRod => {
                ParticleAlphaCurve::SimpleAnimatedFade
            }
            Provider::FireworkFlash => ParticleAlphaCurve::FlashOverlayFade,
            Provider::FireworkSpark => ParticleAlphaCurve::FireworkSparkFade,
            Provider::Shriek => ParticleAlphaCurve::ShriekFade,
            Provider::FlyTowardsPositionVaultConnection => ParticleAlphaCurve::VaultConnectionFade,
            Provider::Firefly => ParticleAlphaCurve::FireflyFade,
            _ => ParticleAlphaCurve::Constant,
        }
    }

    pub(crate) fn color_fade_target(self) -> Option<[f32; 3]> {
        match self.provider {
            Provider::EndRod => Some(END_ROD_FADE_COLOR),
            _ => None,
        }
    }

    pub(crate) fn child_emission(self) -> Option<ParticleChildEmissionDescriptor> {
        match self.provider {
            Provider::Lava => Some(ParticleChildEmissionDescriptor::LavaSmoke),
            Provider::DripHoneyHang
            | Provider::DripObsidianTearHang
            | Provider::DripLavaHang
            | Provider::DripWaterHang
            | Provider::DripstoneLavaHang
            | Provider::DripstoneWaterHang => Some(ParticleChildEmissionDescriptor::DripHangToFall),
            Provider::DripHoneyFall
            | Provider::DripObsidianTearFall
            | Provider::DripLavaFall
            | Provider::DripWaterFall
            | Provider::DripstoneLavaFall
            | Provider::DripstoneWaterFall => {
                Some(ParticleChildEmissionDescriptor::DripFallAndLand)
            }
            Provider::HugeExplosionSeed => Some(ParticleChildEmissionDescriptor::HugeExplosionSeed),
            Provider::GustSeedLarge => Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths: 30,
                vanilla_lifetime: 7,
                tick_delay: 0,
            }),
            Provider::GustSeedSmall => Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths: 10,
                vanilla_lifetime: 3,
                tick_delay: 2,
            }),
            _ => None,
        }
    }
}

pub(crate) fn particle_limit_for_kind(kind: ParticleKind) -> Option<ParticleLimitDescriptor> {
    match kind {
        ParticleKind::SPORE_BLOSSOM_AIR => Some(ParticleLimitDescriptor::SporeBlossom),
        _ => None,
    }
}
