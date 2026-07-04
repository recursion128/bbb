use super::*;

impl ParticleCommandResolver {
    pub(super) fn trial_spawn_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let flame_particle_type_id = match event.data {
            1 => SOUL_FIRE_FLAME_PARTICLE_TYPE_ID,
            _ => FLAME_PARTICLE_TYPE_ID,
        };
        let mut batch = ParticleSpawnBatch::default();
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let flame = self.simple_particle_template(flame_particle_type_id);
        let smoke = self.append_template_result(&mut batch, smoke);
        let flame = self.append_template_result(&mut batch, flame);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
            };
            if let Some(smoke) = smoke.as_ref() {
                batch.commands.push(self.command_from_template(
                    smoke,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
            if let Some(flame) = flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    flame,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
        }

        batch
    }

    pub(super) fn trial_eject_item_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        let small_flame = self.simple_particle_template(SMALL_FLAME_PARTICLE_TYPE_ID);
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let small_flame = self.append_template_result(&mut batch, small_flame);
        let smoke = self.append_template_result(&mut batch, smoke);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.4 + random.next_double() * 0.2,
                y: f64::from(event.pos.y) + 0.4 + random.next_double() * 0.2,
                z: f64::from(event.pos.z) + 0.4 + random.next_double() * 0.2,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            if let Some(small_flame) = small_flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    small_flame,
                    position,
                    Vec3d {
                        z: velocity.z * 0.25,
                        ..velocity
                    },
                    false,
                ));
            }
            if let Some(smoke) = smoke.as_ref() {
                batch
                    .commands
                    .push(self.command_from_template(smoke, position, velocity, false));
            }
        }

        batch
    }

    pub(super) fn trial_detect_player_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        data: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_trial_detect_player_particles(
            &mut batch,
            event,
            particle_type_id,
            data,
            random,
        );
        batch
    }

    pub(super) fn vault_activation_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if !context.vault_block_entity_at_event_pos {
            return ParticleSpawnBatch::default();
        }

        let flame_particle_type_id = if event.data == 0 {
            SMALL_FLAME_PARTICLE_TYPE_ID
        } else {
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
        };
        let mut batch = ParticleSpawnBatch::default();
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let flame = self.simple_particle_template(flame_particle_type_id);
        let vault_connection = context
            .vault_connection_particles
            .as_ref()
            .map(|_| self.simple_particle_template(VAULT_CONNECTION_PARTICLE_TYPE_ID));
        let smoke = self.append_template_result(&mut batch, smoke);
        let flame = self.append_template_result(&mut batch, flame);
        let vault_connection =
            vault_connection.and_then(|template| self.append_template_result(&mut batch, template));

        if let Some(connection) = context.vault_connection_particles.as_ref() {
            self.append_vault_connection_particles(
                &mut batch,
                connection,
                vault_connection.as_ref(),
                random,
            );
        }

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + random_between(random, 0.1, 0.9),
                y: f64::from(event.pos.y) + random_between(random, 0.25, 0.75),
                z: f64::from(event.pos.z) + random_between(random, 0.1, 0.9),
            };
            if let Some(smoke) = smoke.as_ref() {
                batch.commands.push(self.command_from_template(
                    smoke,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
            if let Some(flame) = flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    flame,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
        }

        batch
    }

    pub(super) fn append_vault_connection_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        connection: &VaultConnectionParticleState,
        template: Option<&SimpleParticleTemplate>,
        random: &mut LevelEventSoundRandomState,
    ) {
        let position = Vec3d {
            x: connection.origin[0],
            y: connection.origin[1],
            z: connection.origin[2],
        };
        for target in &connection.targets {
            let direction = [
                target.target_position[0] - connection.origin[0],
                target.target_position[1] - connection.origin[1],
                target.target_position[2] - connection.origin[2],
            ];
            let particle_count = random.next_int_bound(4) + 2;
            for _ in 0..particle_count {
                let velocity = Vec3d {
                    x: direction[0] + f64::from(random.next_float() - 0.5),
                    y: direction[1] + f64::from(random.next_float() - 0.5),
                    z: direction[2] + f64::from(random.next_float() - 0.5),
                };
                if let Some(template) = template {
                    batch
                        .commands
                        .push(self.command_from_template(template, position, velocity, false));
                }
            }
        }
    }

    pub(super) fn vault_deactivation_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let flame_particle_type_id = if event.data == 0 {
            SMALL_FLAME_PARTICLE_TYPE_ID
        } else {
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
        };
        let template = match self.simple_particle_template(flame_particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + random_between(random, 0.4, 0.6),
                y: f64::from(event.pos.y) + random_between(random, 0.4, 0.6),
                z: f64::from(event.pos.z) + random_between(random, 0.4, 0.6),
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    pub(super) fn trial_ominous_activate_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_trial_detect_player_particles(
            &mut batch,
            event,
            TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID,
            0,
            random,
        );
        self.append_trial_become_ominous_particles(&mut batch, event, random);
        batch
    }

    pub(super) fn append_trial_detect_player_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        event: &LevelEvent,
        particle_type_id: i32,
        data: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(template) =
            self.append_template_result(batch, self.simple_particle_template(particle_type_id))
        else {
            return;
        };
        let count = 30_i64 + i64::from(data.min(10)) * 5;
        for _ in 0..count.max(0) {
            let spread_x = (2.0 * f64::from(random.next_float()) - 1.0) * 0.65;
            let spread_z = (2.0 * f64::from(random.next_float()) - 1.0) * 0.65;
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + spread_x,
                y: f64::from(event.pos.y) + 0.1 + f64::from(random.next_float()) * 0.8,
                z: f64::from(event.pos.z) + 0.5 + spread_z,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d::default(),
                false,
            ));
        }
    }

    pub(super) fn append_trial_become_ominous_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let trial_omen = self.simple_particle_template(TRIAL_OMEN_PARTICLE_TYPE_ID);
        let soul_fire_flame = self.simple_particle_template(SOUL_FIRE_FLAME_PARTICLE_TYPE_ID);
        let trial_omen = self.append_template_result(batch, trial_omen);
        let soul_fire_flame = self.append_template_result(batch, soul_fire_flame);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            if let Some(trial_omen) = trial_omen.as_ref() {
                batch
                    .commands
                    .push(self.command_from_template(trial_omen, position, velocity, false));
            }
            if let Some(soul_fire_flame) = soul_fire_flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    soul_fire_flame,
                    position,
                    velocity,
                    false,
                ));
            }
        }
    }
}
