use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::Renderer;

const DEFAULT_MAX_PENDING_PARTICLE_SPAWNS: usize = 16_384;
const DEFAULT_MAX_ACTIVE_PARTICLE_INSTANCES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleSpawnCommand {
    pub particle_type_id: i32,
    pub particle_id: String,
    #[serde(default)]
    pub sprite_ids: Vec<String>,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub override_limiter: bool,
    pub always_show: bool,
    pub raw_options_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParticleSpawnBatch {
    #[serde(default)]
    pub commands: Vec<ParticleSpawnCommand>,
    #[serde(default)]
    pub missing_definition_count: usize,
    #[serde(default)]
    pub missing_sprite_count: usize,
    #[serde(default)]
    pub unknown_particle_type_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ParticleRuntimeState {
    pending_spawns: VecDeque<ParticleSpawnCommand>,
    active_instances: VecDeque<ParticleInstance>,
    max_pending_spawns: usize,
    max_active_instances: usize,
    dropped_spawns: u64,
    instances_created: u64,
    dropped_active_instances: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ParticleInstance {
    pub(crate) particle_type_id: i32,
    pub(crate) particle_id: String,
    pub(crate) sprite_ids: Vec<String>,
    pub(crate) position: [f64; 3],
    pub(crate) velocity: [f64; 3],
    pub(crate) age_ticks: u32,
    pub(crate) override_limiter: bool,
    pub(crate) always_show: bool,
    pub(crate) raw_options_len: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ParticleSubmitSummary {
    pub(crate) requested_spawns: usize,
    pub(crate) queued_spawns: usize,
    pub(crate) dropped_spawns: usize,
    pub(crate) missing_definition_count: usize,
    pub(crate) missing_sprite_count: usize,
    pub(crate) unknown_particle_type_count: usize,
    pub(crate) pending_spawns: usize,
    pub(crate) total_dropped_spawns: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ParticleAdvanceSummary {
    pub(crate) ticks: u32,
    pub(crate) intaken_instances: usize,
    pub(crate) dropped_active_instances: usize,
    pub(crate) pending_spawns: usize,
    pub(crate) active_instances: usize,
    pub(crate) total_instances_created: u64,
    pub(crate) total_dropped_active_instances: u64,
}

impl ParticleSpawnBatch {
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
            && self.missing_definition_count == 0
            && self.missing_sprite_count == 0
            && self.unknown_particle_type_count == 0
    }
}

impl Default for ParticleRuntimeState {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_PENDING_PARTICLE_SPAWNS)
    }
}

impl ParticleRuntimeState {
    pub(crate) fn with_capacity(max_pending_spawns: usize) -> Self {
        Self::with_capacities(max_pending_spawns, DEFAULT_MAX_ACTIVE_PARTICLE_INSTANCES)
    }

    pub(crate) fn with_capacities(max_pending_spawns: usize, max_active_instances: usize) -> Self {
        Self {
            pending_spawns: VecDeque::new(),
            active_instances: VecDeque::new(),
            max_pending_spawns,
            max_active_instances,
            dropped_spawns: 0,
            instances_created: 0,
            dropped_active_instances: 0,
        }
    }

    pub(crate) fn submit_batch(&mut self, batch: ParticleSpawnBatch) -> ParticleSubmitSummary {
        if batch.is_empty() {
            return ParticleSubmitSummary {
                pending_spawns: self.pending_spawns.len(),
                total_dropped_spawns: self.dropped_spawns,
                ..ParticleSubmitSummary::default()
            };
        }

        let requested_spawns = batch.commands.len();
        let mut queued_spawns = 0;
        let mut dropped_spawns = 0;

        for command in batch.commands {
            if self.max_pending_spawns == 0 {
                dropped_spawns += 1;
                continue;
            }
            if self.pending_spawns.len() == self.max_pending_spawns {
                self.pending_spawns.pop_front();
                dropped_spawns += 1;
            }
            self.pending_spawns.push_back(command);
            queued_spawns += 1;
        }

        self.dropped_spawns = self.dropped_spawns.saturating_add(dropped_spawns as u64);

        ParticleSubmitSummary {
            requested_spawns,
            queued_spawns,
            dropped_spawns,
            missing_definition_count: batch.missing_definition_count,
            missing_sprite_count: batch.missing_sprite_count,
            unknown_particle_type_count: batch.unknown_particle_type_count,
            pending_spawns: self.pending_spawns.len(),
            total_dropped_spawns: self.dropped_spawns,
        }
    }

    pub(crate) fn advance(&mut self, ticks: u32) -> ParticleAdvanceSummary {
        let mut intaken_instances = 0;
        let mut dropped_active_instances = 0;

        if ticks == 0 {
            self.drain_pending_spawns(&mut intaken_instances, &mut dropped_active_instances);
        } else {
            for _ in 0..ticks {
                self.tick_active_instances();
                self.drain_pending_spawns(&mut intaken_instances, &mut dropped_active_instances);
            }
        }

        self.instances_created = self
            .instances_created
            .saturating_add(intaken_instances as u64);
        self.dropped_active_instances = self
            .dropped_active_instances
            .saturating_add(dropped_active_instances as u64);

        ParticleAdvanceSummary {
            ticks,
            intaken_instances,
            dropped_active_instances,
            pending_spawns: self.pending_spawns.len(),
            active_instances: self.active_instances.len(),
            total_instances_created: self.instances_created,
            total_dropped_active_instances: self.dropped_active_instances,
        }
    }

    fn tick_active_instances(&mut self) {
        for instance in &mut self.active_instances {
            instance.age_ticks = instance.age_ticks.saturating_add(1);
        }
    }

    fn drain_pending_spawns(
        &mut self,
        intaken_instances: &mut usize,
        dropped_active_instances: &mut usize,
    ) {
        while let Some(command) = self.pending_spawns.pop_front() {
            if self.max_active_instances == 0 {
                *dropped_active_instances += 1;
                continue;
            }
            if self.active_instances.len() == self.max_active_instances {
                self.active_instances.pop_front();
                *dropped_active_instances += 1;
            }
            self.active_instances
                .push_back(ParticleInstance::from_spawn_command(command));
            *intaken_instances += 1;
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_spawns(&self) -> &VecDeque<ParticleSpawnCommand> {
        &self.pending_spawns
    }

    #[cfg(test)]
    pub(crate) fn active_instances(&self) -> &VecDeque<ParticleInstance> {
        &self.active_instances
    }
}

impl ParticleInstance {
    fn from_spawn_command(command: ParticleSpawnCommand) -> Self {
        Self {
            particle_type_id: command.particle_type_id,
            particle_id: command.particle_id,
            sprite_ids: command.sprite_ids,
            position: command.position,
            velocity: command.velocity,
            age_ticks: 0,
            override_limiter: command.override_limiter,
            always_show: command.always_show,
            raw_options_len: command.raw_options_len,
        }
    }
}

impl Renderer {
    pub fn submit_particle_spawns(&mut self, batch: ParticleSpawnBatch) {
        let is_empty = batch.is_empty();
        let summary = self.particles.submit_batch(batch);
        if is_empty {
            return;
        }

        self.counters.particle_spawn_batches =
            self.counters.particle_spawn_batches.saturating_add(1);
        self.counters.particle_spawn_commands = self
            .counters
            .particle_spawn_commands
            .saturating_add(summary.requested_spawns as u64);
        self.counters.particle_missing_definitions = self
            .counters
            .particle_missing_definitions
            .saturating_add(summary.missing_definition_count as u64);
        self.counters.particle_missing_sprites = self
            .counters
            .particle_missing_sprites
            .saturating_add(summary.missing_sprite_count as u64);
        self.counters.particle_unknown_types = self
            .counters
            .particle_unknown_types
            .saturating_add(summary.unknown_particle_type_count as u64);
        self.counters.last_particle_spawn_count = summary.queued_spawns;
        self.counters.pending_particle_spawns = summary.pending_spawns;
        self.counters.dropped_particle_spawns = summary.total_dropped_spawns;
    }

    pub fn advance_particles(&mut self, ticks: u32) {
        let summary = self.particles.advance(ticks);
        self.counters.pending_particle_spawns = summary.pending_spawns;
        self.counters.active_particle_instances = summary.active_instances;
        self.counters.last_particle_intake_count = summary.intaken_instances;
        self.counters.last_particle_tick_count = summary.ticks as usize;
        self.counters.last_particle_active_drop_count = summary.dropped_active_instances;
        self.counters.particle_runtime_ticks = self
            .counters
            .particle_runtime_ticks
            .saturating_add(summary.ticks as u64);
        self.counters.particle_instances_created = summary.total_instances_created;
        self.counters.dropped_active_particle_instances = summary.total_dropped_active_instances;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_spawn_batch_empty_tracks_diagnostics() {
        assert!(ParticleSpawnBatch::default().is_empty());
        assert!(!ParticleSpawnBatch {
            unknown_particle_type_count: 1,
            ..ParticleSpawnBatch::default()
        }
        .is_empty());
        assert!(!ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..ParticleSpawnBatch::default()
        }
        .is_empty());
    }

    #[test]
    fn particle_runtime_queues_spawns_and_keeps_newest_on_overflow() {
        let mut particles = ParticleRuntimeState::with_capacity(2);

        let summary = particles.submit_batch(ParticleSpawnBatch {
            commands: vec![
                spawn_command("minecraft:cloud", 1.0),
                spawn_command("minecraft:flame", 2.0),
                spawn_command("minecraft:smoke", 3.0),
            ],
            ..ParticleSpawnBatch::default()
        });

        assert_eq!(summary.requested_spawns, 3);
        assert_eq!(summary.queued_spawns, 3);
        assert_eq!(summary.dropped_spawns, 1);
        assert_eq!(summary.pending_spawns, 2);
        assert_eq!(summary.total_dropped_spawns, 1);
        let ids: Vec<_> = particles
            .pending_spawns()
            .iter()
            .map(|command| command.particle_id.as_str())
            .collect();
        assert_eq!(ids, vec!["minecraft:flame", "minecraft:smoke"]);
    }

    #[test]
    fn particle_runtime_zero_capacity_counts_drops_without_queueing() {
        let mut particles = ParticleRuntimeState::with_capacity(0);

        let summary = particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:cloud", 1.0)],
            missing_definition_count: 2,
            missing_sprite_count: 3,
            ..ParticleSpawnBatch::default()
        });

        assert_eq!(summary.requested_spawns, 1);
        assert_eq!(summary.queued_spawns, 0);
        assert_eq!(summary.dropped_spawns, 1);
        assert_eq!(summary.missing_definition_count, 2);
        assert_eq!(summary.missing_sprite_count, 3);
        assert_eq!(summary.pending_spawns, 0);
        assert!(particles.pending_spawns().is_empty());
    }

    #[test]
    fn particle_runtime_advances_pending_spawns_into_active_instances() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![
                spawn_command("minecraft:cloud", 1.0),
                spawn_command("minecraft:flame", 2.0),
            ],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(0);

        assert_eq!(summary.ticks, 0);
        assert_eq!(summary.intaken_instances, 2);
        assert_eq!(summary.dropped_active_instances, 0);
        assert_eq!(summary.pending_spawns, 0);
        assert_eq!(summary.active_instances, 2);
        assert!(particles.pending_spawns().is_empty());
        assert_eq!(
            particles.active_instances()[0].particle_id,
            "minecraft:cloud"
        );
        assert_eq!(particles.active_instances()[0].position, [1.0, 0.0, 0.0]);
        assert_eq!(particles.active_instances()[0].age_ticks, 0);
    }

    #[test]
    fn particle_runtime_ages_active_instances_on_client_ticks() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:cloud", 1.0)],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        let summary = particles.advance(3);

        assert_eq!(summary.ticks, 3);
        assert_eq!(summary.intaken_instances, 0);
        assert_eq!(summary.active_instances, 1);
        assert_eq!(particles.active_instances()[0].age_ticks, 3);
    }

    #[test]
    fn particle_runtime_ticks_existing_active_before_intaking_pending_spawns() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:cloud", 1.0)],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:flame", 2.0)],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(1);

        assert_eq!(summary.ticks, 1);
        assert_eq!(summary.intaken_instances, 1);
        assert_eq!(summary.active_instances, 2);
        assert_eq!(
            particles.active_instances()[0].particle_id,
            "minecraft:cloud"
        );
        assert_eq!(particles.active_instances()[0].age_ticks, 1);
        assert_eq!(
            particles.active_instances()[1].particle_id,
            "minecraft:flame"
        );
        assert_eq!(particles.active_instances()[1].age_ticks, 0);
    }

    #[test]
    fn particle_runtime_limits_active_instances_and_keeps_newest() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 2);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![
                spawn_command("minecraft:cloud", 1.0),
                spawn_command("minecraft:flame", 2.0),
                spawn_command("minecraft:smoke", 3.0),
            ],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(0);

        assert_eq!(summary.intaken_instances, 3);
        assert_eq!(summary.dropped_active_instances, 1);
        assert_eq!(summary.active_instances, 2);
        assert_eq!(summary.total_instances_created, 3);
        assert_eq!(summary.total_dropped_active_instances, 1);
        let ids: Vec<_> = particles
            .active_instances()
            .iter()
            .map(|instance| instance.particle_id.as_str())
            .collect();
        assert_eq!(ids, vec!["minecraft:flame", "minecraft:smoke"]);
    }

    fn spawn_command(particle_id: &str, x: f64) -> ParticleSpawnCommand {
        ParticleSpawnCommand {
            particle_type_id: 4,
            particle_id: particle_id.to_string(),
            sprite_ids: vec!["minecraft:generic_0".to_string()],
            position: [x, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
        }
    }
}
