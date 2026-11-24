use super::*;

struct FireworkSounds {
    blast: i32,
    blast_far: i32,
    large_blast: i32,
    large_blast_far: i32,
    twinkle: i32,
    twinkle_far: i32,
}

static FIREWORK_SOUNDS: LazyLock<FireworkSounds> = LazyLock::new(|| FireworkSounds {
    blast: sound_event("minecraft:entity.firework_rocket.blast"),
    blast_far: sound_event("minecraft:entity.firework_rocket.blast_far"),
    large_blast: sound_event("minecraft:entity.firework_rocket.large_blast"),
    large_blast_far: sound_event("minecraft:entity.firework_rocket.large_blast_far"),
    twinkle: sound_event("minecraft:entity.firework_rocket.twinkle"),
    twinkle_far: sound_event("minecraft:entity.firework_rocket.twinkle_far"),
});

impl NativeParticleRuntime {
    pub(crate) fn spawn_firework_empty_explosion_particles(
        &mut self,
        out: &mut ParticleEngine,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) {
        let count = self.random.next_i32(3) + 2;
        let Some(kind) = self.defined(ParticleKind::POOF) else {
            return;
        };
        let spawn_position = Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        };
        for _ in 0..count {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.05,
                y: 0.005,
                z: self.random.next_gaussian() * 0.05,
            };
            if self.should_spawn_level_particle(
                kind.override_limiter(),
                false,
                spawn_position,
                camera_position,
            ) {
                out.spawn(particle_spawn(kind, spawn_position, velocity));
            }
        }
    }

    /// `FireworkParticles.Starter`: blast sound, sparks and flash per
    /// explosion, then the delayed twinkle.
    pub(crate) fn spawn_firework_explosion_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) {
        let firework = self.defined(ParticleKind::FIREWORK);
        let flash = self.defined(ParticleKind::FLASH);
        let position = state.position;
        let movement = state.delta_movement;

        if !state.explosions.is_empty() {
            let sound = self.firework_blast_sound(state, camera_position);
            out.play_sound(sound);
        }

        for explosion in &state.explosions {
            if let Some(firework) = firework {
                self.spawn_firework_explosion_sparks(out, firework, position, movement, explosion);
            }
            if let Some(flash) = flash {
                let colors = firework_explosion_colors(explosion);
                out.spawn(
                    particle_spawn(flash, position, Vec3d::default())
                        .with_params(ParticleParams::Color(firework_argb_color(colors[0]))),
                );
            }
        }

        if state
            .explosions
            .iter()
            .any(|explosion| explosion.has_twinkle)
        {
            let sound = self.firework_twinkle_sound(state);
            out.schedule_sound(sound);
        }
    }

    pub(super) fn spawn_firework_rocket_trail(
        &mut self,
        out: &mut ParticleEngine,
        state: FireworkRocketTrailParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::FIREWORK) else {
            return;
        };
        let position = state.position;
        let velocity = Vec3d {
            x: self.random.next_gaussian() * 0.05,
            y: -state.delta_movement.y * 0.5,
            z: self.random.next_gaussian() * 0.05,
        };
        out.spawn(particle_spawn(kind, position, velocity));
    }

    pub(crate) fn spawn_ominous_item_spawner_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: OminousItemSpawnerParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::OMINOUS_SPAWNING) else {
            return;
        };
        let position = state.position;
        let particle_count = self.random.next_i32(3) + 1;
        for _ in 0..particle_count {
            let velocity = Vec3d {
                x: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                y: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                z: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
            };
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    fn firework_blast_sound(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSound {
        let far_effect = camera_position.is_some_and(|camera| {
            let dx = state.position.x - camera[0];
            let dy = state.position.y - camera[1];
            let dz = state.position.z - camera[2];
            dx * dx + dy * dy + dz * dz >= 256.0
        });
        let large_explosion = state.explosions.len() >= 3
            || state
                .explosions
                .iter()
                .any(|explosion| explosion.shape == FireworkExplosionShapeSummary::LargeBall);
        let sounds = &*FIREWORK_SOUNDS;
        let sound = match (large_explosion, far_effect) {
            (true, true) => sounds.large_blast_far,
            (true, false) => sounds.large_blast,
            (false, true) => sounds.blast_far,
            (false, false) => sounds.blast,
        };
        ParticleSound {
            sound,
            source: SoundSource::Ambient,
            position: [state.position.x, state.position.y, state.position.z],
            volume: 20.0,
            pitch: 0.95 + self.random.next_float() * 0.1,
            seed: self.particle_level_random.next_i64(),
            distance_delay: true,
        }
    }

    fn firework_twinkle_sound(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
    ) -> ScheduledParticleSound {
        let sounds = &*FIREWORK_SOUNDS;
        ScheduledParticleSound {
            sound: ParticleSound {
                sound: sounds.twinkle,
                source: SoundSource::Ambient,
                position: [state.position.x, state.position.y, state.position.z],
                volume: 20.0,
                pitch: 0.9 + self.random.next_float() * 0.15,
                seed: self.particle_level_random.next_i64(),
                distance_delay: true,
            },
            delay_ticks: firework_twinkle_delay_ticks(state.explosions.len()),
            far_sound: Some((sounds.twinkle_far, 256.0)),
        }
    }

    fn spawn_firework_explosion_sparks(
        &mut self,
        out: &mut ParticleEngine,
        kind: ParticleKind,
        position: Vec3d,
        movement: Vec3d,
        explosion: &FireworkExplosionSummary,
    ) {
        let colors = firework_explosion_colors(explosion);
        let spark = FireworkSpark {
            kind,
            position,
            colors: &colors,
            explosion,
        };
        match explosion.shape {
            FireworkExplosionShapeSummary::SmallBall => {
                self.spawn_firework_particle_ball(out, &spark, 0.25, 2);
            }
            FireworkExplosionShapeSummary::LargeBall => {
                self.spawn_firework_particle_ball(out, &spark, 0.5, 4);
            }
            FireworkExplosionShapeSummary::Star => {
                self.spawn_firework_particle_shape(
                    out,
                    &spark,
                    0.5,
                    FIREWORK_STAR_PARTICLE_COORDS,
                    false,
                );
            }
            FireworkExplosionShapeSummary::Creeper => {
                self.spawn_firework_particle_shape(
                    out,
                    &spark,
                    0.5,
                    FIREWORK_CREEPER_PARTICLE_COORDS,
                    true,
                );
            }
            FireworkExplosionShapeSummary::Burst => {
                self.spawn_firework_particle_burst(out, &spark, movement);
            }
        }
    }

    fn spawn_firework_particle_ball(
        &mut self,
        out: &mut ParticleEngine,
        spark: &FireworkSpark<'_>,
        base_speed: f64,
        steps: i32,
    ) {
        for y_step in -steps..=steps {
            for x_step in -steps..=steps {
                let mut z_step = -steps;
                while z_step <= steps {
                    let xa =
                        f64::from(x_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let ya =
                        f64::from(y_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let za =
                        f64::from(z_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let len = (xa * xa + ya * ya + za * za).sqrt() / base_speed
                        + self.random.next_gaussian() * 0.05;
                    let velocity = Vec3d {
                        x: xa / len,
                        y: ya / len,
                        z: za / len,
                    };
                    self.spawn_firework_spark(out, spark, velocity);
                    if y_step != -steps && y_step != steps && x_step != -steps && x_step != steps {
                        z_step += steps * 2 - 1;
                    }
                    z_step += 1;
                }
            }
        }
    }

    fn spawn_firework_particle_shape(
        &mut self,
        out: &mut ParticleEngine,
        spark: &FireworkSpark<'_>,
        base_speed: f64,
        coords: &[[f64; 2]],
        flat: bool,
    ) {
        let sx = coords[0][0];
        let sy = coords[0][1];
        self.spawn_firework_spark(
            out,
            spark,
            Vec3d {
                x: sx * base_speed,
                y: sy * base_speed,
                z: 0.0,
            },
        );
        let base_angle = f64::from(self.random.next_float()) * std::f64::consts::PI;
        let angle_mod = if flat { 0.034 } else { 0.34 };

        for angle_step in 0..3 {
            let angle = base_angle + f64::from(angle_step) * std::f64::consts::PI * angle_mod;
            let mut ox = sx;
            let mut oy = sy;
            for coord in coords.iter().skip(1) {
                let tx = coord[0];
                let ty = coord[1];
                for sub_step_index in 1..=4 {
                    let sub_step = f64::from(sub_step_index) * 0.25;
                    let mut xa = lerp_f64(sub_step, ox, tx) * base_speed;
                    let ya = lerp_f64(sub_step, oy, ty) * base_speed;
                    let za = xa * angle.sin();
                    xa *= angle.cos();
                    for flip in [-1.0, 1.0] {
                        self.spawn_firework_spark(
                            out,
                            spark,
                            Vec3d {
                                x: xa * flip,
                                y: ya,
                                z: za * flip,
                            },
                        );
                    }
                }
                ox = tx;
                oy = ty;
            }
        }
    }

    fn spawn_firework_particle_burst(
        &mut self,
        out: &mut ParticleEngine,
        spark: &FireworkSpark<'_>,
        movement: Vec3d,
    ) {
        let base_off_x = self.random.next_gaussian() * 0.05;
        let base_off_z = self.random.next_gaussian() * 0.05;
        for _ in 0..70 {
            let velocity = Vec3d {
                x: movement.x * 0.5 + self.random.next_gaussian() * 0.15 + base_off_x,
                y: movement.y * 0.5 + self.random.next_f64() * 0.5,
                z: movement.z * 0.5 + self.random.next_gaussian() * 0.15 + base_off_z,
            };
            self.spawn_firework_spark(out, spark, velocity);
        }
    }

    /// `FireworkParticles.Starter.createParticle`.
    fn spawn_firework_spark(
        &mut self,
        out: &mut ParticleEngine,
        spark: &FireworkSpark<'_>,
        velocity: Vec3d,
    ) {
        let color = firework_spark_color(random_firework_color(spark.colors, &mut self.random));
        let fade_to = (!spark.explosion.fade_colors.is_empty()).then(|| {
            let [red, green, blue, _] = firework_argb_color(random_firework_color(
                &spark.explosion.fade_colors,
                &mut self.random,
            ));
            [red, green, blue]
        });
        out.spawn(
            particle_spawn(spark.kind, spark.position, velocity).with_params(
                ParticleParams::FireworkSpark {
                    color,
                    fade_to,
                    trail: spark.explosion.has_trail,
                    twinkle: spark.explosion.has_twinkle,
                    half_lifetime: false,
                },
            ),
        );
    }
}

/// The per-explosion inputs every spark shares.
struct FireworkSpark<'a> {
    kind: ParticleKind,
    position: Vec3d,
    colors: &'a [i32],
    explosion: &'a FireworkExplosionSummary,
}

fn firework_explosion_colors(explosion: &FireworkExplosionSummary) -> Vec<i32> {
    if explosion.colors.is_empty() {
        vec![FIREWORK_BLACK_COLOR]
    } else {
        explosion.colors.clone()
    }
}

fn firework_twinkle_delay_ticks(explosion_count: usize) -> u32 {
    if explosion_count == 0 {
        0
    } else {
        (explosion_count as u32).saturating_mul(2) - 1 + 15
    }
}

fn random_firework_color(colors: &[i32], random: &mut LegacyRandom) -> i32 {
    let index = random.next_i32(colors.len() as i32) as usize;
    colors[index]
}

fn firework_spark_color(rgb: i32) -> [f32; 4] {
    let [red, green, blue, _] = firework_argb_color(rgb);
    [red, green, blue, 0.99]
}

fn firework_argb_color(argb: i32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xff) as f32 / 255.0,
        ((argb >> 8) & 0xff) as f32 / 255.0,
        (argb & 0xff) as f32 / 255.0,
        ((argb >> 24) & 0xff) as f32 / 255.0,
    ]
}

const FIREWORK_BLACK_COLOR: i32 = 1_973_019;
const FIREWORK_CREEPER_PARTICLE_COORDS: &[[f64; 2]] = &[
    [0.0, 0.2],
    [0.2, 0.2],
    [0.2, 0.6],
    [0.6, 0.6],
    [0.6, 0.2],
    [0.2, 0.2],
    [0.2, 0.0],
    [0.4, 0.0],
    [0.4, -0.6],
    [0.2, -0.6],
    [0.2, -0.4],
    [0.0, -0.4],
];
const FIREWORK_STAR_PARTICLE_COORDS: &[[f64; 2]] = &[
    [0.0, 1.0],
    [0.3455, 0.309],
    [0.9511, 0.309],
    [0.3795918367346939, -0.12653061224489795],
    [0.6122448979591837, -0.8040816326530612],
    [0.0, -0.35918367346938773],
];
