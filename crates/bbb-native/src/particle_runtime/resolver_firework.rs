use super::*;

impl ParticleCommandResolver {
    pub(super) fn firework_empty_explosion_particle_batch(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        let count = self.random.next_i32(3) + 2;
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..count {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.05,
                y: 0.005,
                z: self.random.next_gaussian() * 0.05,
            };
            let command_position = Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            };
            if self.should_spawn_level_particle(
                template.particle_type.override_limiter,
                false,
                command_position,
                camera_position,
            ) {
                batch.commands.push(self.command_from_template(
                    &template,
                    command_position,
                    velocity,
                    false,
                ));
            }
        }

        batch
    }

    pub(super) fn firework_explosion_particle_batch(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        let firework = self.simple_particle_template(FIREWORK_PARTICLE_TYPE_ID);
        let flash = self.simple_particle_template(FLASH_PARTICLE_TYPE_ID);
        let mut batch = ParticleSpawnBatch::default();
        let firework = self.append_template_result(&mut batch, firework);
        let flash = self.append_template_result(&mut batch, flash);
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let movement = Vec3d {
            x: state.delta_movement.x,
            y: state.delta_movement.y,
            z: state.delta_movement.z,
        };

        if !state.explosions.is_empty() {
            batch
                .sound_events
                .push(self.firework_blast_sound_event(state, camera_position));
        }

        for explosion in &state.explosions {
            if let Some(firework) = firework.as_ref() {
                self.append_firework_explosion_sparks(
                    &mut batch, firework, position, movement, explosion,
                );
            }
            if let Some(flash) = flash.as_ref() {
                let colors = firework_explosion_colors(explosion);
                let mut command =
                    self.command_from_template(flash, position, Vec3d::default(), false);
                command.option_color = Some(firework_flash_color(colors[0]));
                batch.commands.push(command);
            }
        }

        if state
            .explosions
            .iter()
            .any(|explosion| explosion.has_twinkle)
        {
            batch
                .scheduled_sound_events
                .push(self.firework_twinkle_sound_event(state));
        }

        batch
    }

    pub(super) fn firework_rocket_trail_particle_batch(
        &mut self,
        state: FireworkRocketTrailParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(FIREWORK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let velocity = Vec3d {
            x: self.random.next_gaussian() * 0.05,
            y: -state.delta_movement.y * 0.5,
            z: self.random.next_gaussian() * 0.05,
        };
        ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            commands: vec![self.command_from_template(&template, position, velocity, false)],
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn ominous_item_spawner_particle_batch(
        &mut self,
        state: OminousItemSpawnerParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(OMINOUS_SPAWNING_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let particle_count = self.random.next_i32(3) + 1;
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..particle_count {
            let velocity = Vec3d {
                x: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                y: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                z: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    pub(super) fn firework_blast_sound_event(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSoundEvent {
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
        let sound_event_id = match (large_explosion, far_effect) {
            (true, true) => FIREWORK_ROCKET_LARGE_BLAST_FAR_SOUND_EVENT_ID,
            (true, false) => FIREWORK_ROCKET_LARGE_BLAST_SOUND_EVENT_ID,
            (false, true) => FIREWORK_ROCKET_BLAST_FAR_SOUND_EVENT_ID,
            (false, false) => FIREWORK_ROCKET_BLAST_SOUND_EVENT_ID,
        };
        ParticleSoundEvent {
            sound_event_id: sound_event_id.to_string(),
            source: "ambient".to_string(),
            position: [state.position.x, state.position.y, state.position.z],
            volume: 20.0,
            pitch: 0.95 + self.random.next_float() * 0.1,
            seed: self.particle_level_random.next_i64(),
            distance_delay: true,
        }
    }

    pub(super) fn firework_twinkle_sound_event(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
    ) -> ParticleScheduledSoundEvent {
        ParticleScheduledSoundEvent {
            event: ParticleSoundEvent {
                sound_event_id: FIREWORK_ROCKET_TWINKLE_SOUND_EVENT_ID.to_string(),
                source: "ambient".to_string(),
                position: [state.position.x, state.position.y, state.position.z],
                volume: 20.0,
                pitch: 0.9 + self.random.next_float() * 0.15,
                seed: self.particle_level_random.next_i64(),
                distance_delay: true,
            },
            delay_ticks: firework_twinkle_delay_ticks(state.explosions.len()),
            far_sound_event_id: Some(FIREWORK_ROCKET_TWINKLE_FAR_SOUND_EVENT_ID.to_string()),
            far_distance_squared: Some(256.0),
        }
    }

    pub(super) fn append_firework_explosion_sparks(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        movement: Vec3d,
        explosion: &FireworkExplosionSummary,
    ) {
        let colors = firework_explosion_colors(explosion);
        match explosion.shape {
            FireworkExplosionShapeSummary::SmallBall => {
                self.append_firework_particle_ball(
                    batch, template, position, 0.25, 2, &colors, explosion,
                );
            }
            FireworkExplosionShapeSummary::LargeBall => {
                self.append_firework_particle_ball(
                    batch, template, position, 0.5, 4, &colors, explosion,
                );
            }
            FireworkExplosionShapeSummary::Star => {
                self.append_firework_particle_shape(
                    batch,
                    template,
                    position,
                    0.5,
                    FIREWORK_STAR_PARTICLE_COORDS,
                    &colors,
                    explosion,
                    false,
                );
            }
            FireworkExplosionShapeSummary::Creeper => {
                self.append_firework_particle_shape(
                    batch,
                    template,
                    position,
                    0.5,
                    FIREWORK_CREEPER_PARTICLE_COORDS,
                    &colors,
                    explosion,
                    true,
                );
            }
            FireworkExplosionShapeSummary::Burst => {
                self.append_firework_particle_burst(
                    batch, template, position, movement, &colors, explosion,
                );
            }
        }
    }

    pub(super) fn append_firework_particle_ball(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        base_speed: f64,
        steps: i32,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
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
                    self.append_firework_spark(
                        batch, template, position, velocity, colors, explosion,
                    );
                    if y_step != -steps && y_step != steps && x_step != -steps && x_step != steps {
                        z_step += steps * 2 - 1;
                    }
                    z_step += 1;
                }
            }
        }
    }

    pub(super) fn append_firework_particle_shape(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        base_speed: f64,
        coords: &[[f64; 2]],
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
        flat: bool,
    ) {
        let sx = coords[0][0];
        let sy = coords[0][1];
        self.append_firework_spark(
            batch,
            template,
            position,
            Vec3d {
                x: sx * base_speed,
                y: sy * base_speed,
                z: 0.0,
            },
            colors,
            explosion,
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
                        self.append_firework_spark(
                            batch,
                            template,
                            position,
                            Vec3d {
                                x: xa * flip,
                                y: ya,
                                z: za * flip,
                            },
                            colors,
                            explosion,
                        );
                    }
                }
                ox = tx;
                oy = ty;
            }
        }
    }

    pub(super) fn append_firework_particle_burst(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        movement: Vec3d,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
    ) {
        let base_off_x = self.random.next_gaussian() * 0.05;
        let base_off_z = self.random.next_gaussian() * 0.05;
        for _ in 0..70 {
            let velocity = Vec3d {
                x: movement.x * 0.5 + self.random.next_gaussian() * 0.15 + base_off_x,
                y: movement.y * 0.5 + self.random.next_f64() * 0.5,
                z: movement.z * 0.5 + self.random.next_gaussian() * 0.15 + base_off_z,
            };
            self.append_firework_spark(batch, template, position, velocity, colors, explosion);
        }
    }

    pub(super) fn append_firework_spark(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        velocity: Vec3d,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
    ) {
        let mut command = self.command_from_template(template, position, velocity, false);
        command.option_color = Some(firework_spark_color(random_firework_color(
            colors,
            &mut self.random,
        )));
        command.option_firework_trail = explosion.has_trail;
        command.option_firework_twinkle = explosion.has_twinkle;
        if !explosion.fade_colors.is_empty() {
            command.option_color_to = Some(firework_spark_fade_color(random_firework_color(
                &explosion.fade_colors,
                &mut self.random,
            )));
        }
        batch.commands.push(command);
    }
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

fn firework_spark_fade_color(rgb: i32) -> [f32; 4] {
    let [red, green, blue, _] = firework_argb_color(rgb);
    [red, green, blue, 1.0]
}

fn firework_flash_color(argb: i32) -> [f32; 4] {
    firework_argb_color(argb)
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
