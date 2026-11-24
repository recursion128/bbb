use super::*;

impl NativeParticleRuntime {
    /// `TrialSpawner.addSpawnParticles`.
    pub(super) fn spawn_trial_spawn(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let flame = match event.data {
            1 => ParticleKind::SOUL_FIRE_FLAME,
            _ => ParticleKind::FLAME,
        };
        let smoke = self.defined(ParticleKind::SMOKE);
        let flame = self.defined(flame);
        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
            };
            spawn_kinds(out, [smoke, flame], position, Vec3d::default());
        }
    }

    /// `TrialSpawner.addEjectItemParticles`.
    pub(super) fn spawn_trial_eject_item(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let small_flame = self.defined(ParticleKind::SMALL_FLAME);
        let smoke = self.defined(ParticleKind::SMOKE);
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
            spawn_kinds(
                out,
                [small_flame],
                position,
                Vec3d {
                    z: velocity.z * 0.25,
                    ..velocity
                },
            );
            spawn_kinds(out, [smoke], position, velocity);
        }
    }

    /// `VaultBlockEntity.Client.emitActivationParticles`.
    pub(super) fn spawn_vault_activation(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) {
        if !context.vault_block_entity_at_event_pos {
            return;
        }
        let flame = if event.data == 0 {
            ParticleKind::SMALL_FLAME
        } else {
            ParticleKind::SOUL_FIRE_FLAME
        };
        let smoke = self.defined(ParticleKind::SMOKE);
        let flame = self.defined(flame);
        if let Some(connection) = context.vault_connection_particles.as_ref() {
            self.spawn_vault_connection(out, connection, random);
        }
        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + random_between(random, 0.1, 0.9),
                y: f64::from(event.pos.y) + random_between(random, 0.25, 0.75),
                z: f64::from(event.pos.z) + random_between(random, 0.1, 0.9),
            };
            spawn_kinds(out, [smoke, flame], position, Vec3d::default());
        }
    }

    fn spawn_vault_connection(
        &self,
        out: &mut ParticleEngine,
        connection: &VaultConnectionParticleState,
        random: &mut LevelEventSoundRandomState,
    ) {
        let kind = self.defined(ParticleKind::VAULT_CONNECTION);
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
                spawn_kinds(out, [kind], position, velocity);
            }
        }
    }

    /// `VaultBlockEntity.Client.emitDeactivationParticles`.
    pub(super) fn spawn_vault_deactivation(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let flame = if event.data == 0 {
            ParticleKind::SMALL_FLAME
        } else {
            ParticleKind::SOUL_FIRE_FLAME
        };
        let Some(kind) = self.defined(flame) else {
            return;
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(super) fn spawn_trial_ominous_activate(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        self.spawn_trial_detect_player(
            out,
            event,
            ParticleKind::TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS,
            0,
            random,
        );
        let trial_omen = self.defined(ParticleKind::TRIAL_OMEN);
        let soul_fire_flame = self.defined(ParticleKind::SOUL_FIRE_FLAME);
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
            spawn_kinds(out, [trial_omen, soul_fire_flame], position, velocity);
        }
    }

    /// `TrialSpawner.addDetectPlayerParticles`.
    pub(super) fn spawn_trial_detect_player(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        kind: ParticleKind,
        data: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(kind) else {
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
            out.spawn(particle_spawn(kind, position, Vec3d::default()));
        }
    }
}

/// One particle per defined kind, in order.
fn spawn_kinds<const N: usize>(
    out: &mut ParticleEngine,
    kinds: [Option<ParticleKind>; N],
    position: Vec3d,
    velocity: Vec3d,
) {
    for kind in kinds.into_iter().flatten() {
        out.spawn(particle_spawn(kind, position, velocity));
    }
}
