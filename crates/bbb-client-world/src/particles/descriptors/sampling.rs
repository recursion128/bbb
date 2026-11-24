use super::*;

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
    sprite_count: usize,
    selection: ParticleSpriteSelection,
    random: &mut ParticleRandom,
) -> Option<usize> {
    match selection {
        ParticleSpriteSelection::First | ParticleSpriteSelection::Age => {
            (sprite_count != 0).then_some(0)
        }
        ParticleSpriteSelection::Random => random.next_index(sprite_count),
    }
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

pub(super) const WHITE_PARTICLE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub(super) const WHITE_ASH_SMOKE_RGB: [f32; 3] = [186.0 / 255.0, 177.0 / 255.0, 194.0 / 255.0];

pub(super) const DRAGON_BREATH_COLOR_MIN: [f32; 3] = [0.717_647_1, 0.0, 0.823_529_4];

pub(super) const DRAGON_BREATH_COLOR_MAX: [f32; 3] = [0.874_509_8, 0.0, 0.976_470_6];

pub(super) const CRIT_COLOR_SCALE: [f32; 3] = [1.0, 0.96, 0.9];

pub(super) const MAGIC_CRIT_COLOR_SCALE: [f32; 3] = [0.3, 0.768, 0.9];

pub(super) const TERRAIN_PARTICLE_RGB: [f32; 3] = [0.6, 0.6, 0.6];

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

    pub(super) fn next_gaussian(&mut self) -> f64 {
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

pub(super) fn falling_leaves_descriptor_for_kind(kind: ParticleKind) -> FallingLeavesDescriptor {
    match kind {
        ParticleKind::CHERRY_LEAVES => cherry_falling_leaves_descriptor(),
        ParticleKind::PALE_OAK_LEAVES | ParticleKind::TINTED_LEAVES => {
            pale_oak_falling_leaves_descriptor()
        }
        _ => unreachable!("falling leaves descriptor requested for {kind:?}"),
    }
}

pub(super) fn cherry_falling_leaves_descriptor() -> FallingLeavesDescriptor {
    FallingLeavesDescriptor {
        fall_acceleration: 0.25,
        side_acceleration: 2.0,
        swirl: false,
        flow_away: true,
        scale: 1.0,
        start_velocity: 0.0,
    }
}

pub(super) fn pale_oak_falling_leaves_descriptor() -> FallingLeavesDescriptor {
    FallingLeavesDescriptor {
        fall_acceleration: 0.07,
        side_acceleration: 10.0,
        swirl: true,
        flow_away: false,
        scale: 2.0,
        start_velocity: 0.021,
    }
}

pub(super) fn terrain_particle_descriptor(
    provider: Provider,
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

pub(super) fn breaking_item_particle_descriptor(
    provider: Provider,
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

pub(super) fn random_signed_velocity(random: &mut ParticleRandom) -> f64 {
    random_signed_unit(random) * 0.4
}

fn random_signed_unit(random: &mut ParticleRandom) -> f64 {
    f64::from(random.next_f32()) * 2.0 - 1.0
}

pub(super) fn random_centered_offset(random: &mut ParticleRandom, scale: f64) -> f64 {
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
