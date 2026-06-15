use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::Renderer;

const DEFAULT_MAX_PENDING_PARTICLE_SPAWNS: usize = 16_384;

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
    pub unknown_particle_type_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ParticleRuntimeState {
    pending_spawns: VecDeque<ParticleSpawnCommand>,
    max_pending_spawns: usize,
    dropped_spawns: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ParticleSubmitSummary {
    pub(crate) requested_spawns: usize,
    pub(crate) queued_spawns: usize,
    pub(crate) dropped_spawns: usize,
    pub(crate) missing_definition_count: usize,
    pub(crate) unknown_particle_type_count: usize,
    pub(crate) pending_spawns: usize,
    pub(crate) total_dropped_spawns: u64,
}

impl ParticleSpawnBatch {
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
            && self.missing_definition_count == 0
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
        Self {
            pending_spawns: VecDeque::new(),
            max_pending_spawns,
            dropped_spawns: 0,
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
            unknown_particle_type_count: batch.unknown_particle_type_count,
            pending_spawns: self.pending_spawns.len(),
            total_dropped_spawns: self.dropped_spawns,
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_spawns(&self) -> &VecDeque<ParticleSpawnCommand> {
        &self.pending_spawns
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
        self.counters.particle_unknown_types = self
            .counters
            .particle_unknown_types
            .saturating_add(summary.unknown_particle_type_count as u64);
        self.counters.last_particle_spawn_count = summary.queued_spawns;
        self.counters.pending_particle_spawns = summary.pending_spawns;
        self.counters.dropped_particle_spawns = summary.total_dropped_spawns;
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
            ..ParticleSpawnBatch::default()
        });

        assert_eq!(summary.requested_spawns, 1);
        assert_eq!(summary.queued_spawns, 0);
        assert_eq!(summary.dropped_spawns, 1);
        assert_eq!(summary.missing_definition_count, 2);
        assert_eq!(summary.pending_spawns, 0);
        assert!(particles.pending_spawns().is_empty());
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
