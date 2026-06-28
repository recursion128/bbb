use std::collections::{BTreeMap, VecDeque};

use anyhow::Result;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::Renderer;

mod descriptors;
mod gpu;

use descriptors::{
    select_initial_sprite, sprite_index_for_age, ParticleDescriptor, ParticleQuadSizeCurve,
    ParticleRandom, ParticleSpriteSelection, DEFAULT_PARTICLE_RANDOM_SEED,
};
pub(super) use gpu::{
    create_particle_atlas_gpu, create_particle_pipeline, ParticleAtlasGpu, ParticleVertex,
};

const DEFAULT_MAX_PENDING_PARTICLE_SPAWNS: usize = 16_384;
const DEFAULT_MAX_ACTIVE_PARTICLE_INSTANCES: usize = 16_384;
const DEFAULT_PARTICLE_QUAD_SIZE: f32 = 0.2;
const DEFAULT_PARTICLE_RENDER_PARTIAL_TICK: f32 = 0.5;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleSpriteUv {
    pub id: String,
    pub uv: ParticleUvRect,
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
    instances_expired: u64,
    dropped_active_instances: u64,
    random: ParticleRandom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ParticleInstance {
    pub(crate) particle_type_id: i32,
    pub(crate) particle_id: String,
    pub(crate) sprite_ids: Vec<String>,
    #[serde(default)]
    pub(crate) current_sprite_id: Option<String>,
    #[serde(default)]
    pub(crate) current_sprite_index: Option<usize>,
    pub(crate) previous_position: [f64; 3],
    pub(crate) position: [f64; 3],
    pub(crate) velocity: [f64; 3],
    pub(crate) age_ticks: u32,
    pub(crate) lifetime_ticks: u32,
    #[serde(default = "default_particle_quad_size")]
    pub(crate) base_quad_size: f32,
    #[serde(default = "default_particle_color")]
    pub(crate) color: [f32; 4],
    #[serde(default)]
    pub(crate) quad_size_curve: ParticleQuadSizeCurve,
    pub(crate) provider: String,
    pub(crate) friction: f32,
    pub(crate) gravity: f32,
    pub(crate) has_physics: bool,
    pub(crate) speed_up_when_y_motion_is_blocked: bool,
    pub(crate) sprite_selection: ParticleSpriteSelection,
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
    pub(crate) expired_instances: usize,
    pub(crate) dropped_active_instances: usize,
    pub(crate) pending_spawns: usize,
    pub(crate) active_instances: usize,
    pub(crate) total_instances_created: u64,
    pub(crate) total_instances_expired: u64,
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

fn default_particle_quad_size() -> f32 {
    DEFAULT_PARTICLE_QUAD_SIZE
}

fn default_particle_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
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
        Self::with_random(
            max_pending_spawns,
            max_active_instances,
            ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED),
        )
    }

    #[cfg(test)]
    pub(crate) fn with_capacities_and_seed(
        max_pending_spawns: usize,
        max_active_instances: usize,
        seed: i64,
    ) -> Self {
        Self::with_random(
            max_pending_spawns,
            max_active_instances,
            ParticleRandom::new(seed),
        )
    }

    fn with_random(
        max_pending_spawns: usize,
        max_active_instances: usize,
        random: ParticleRandom,
    ) -> Self {
        Self {
            pending_spawns: VecDeque::new(),
            active_instances: VecDeque::new(),
            max_pending_spawns,
            max_active_instances,
            dropped_spawns: 0,
            instances_created: 0,
            instances_expired: 0,
            dropped_active_instances: 0,
            random,
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
        let mut expired_instances = 0;
        let mut dropped_active_instances = 0;

        if ticks == 0 {
            self.drain_pending_spawns(&mut intaken_instances, &mut dropped_active_instances);
        } else {
            for _ in 0..ticks {
                expired_instances += self.tick_active_instances();
                self.drain_pending_spawns(&mut intaken_instances, &mut dropped_active_instances);
            }
        }

        self.instances_created = self
            .instances_created
            .saturating_add(intaken_instances as u64);
        self.instances_expired = self
            .instances_expired
            .saturating_add(expired_instances as u64);
        self.dropped_active_instances = self
            .dropped_active_instances
            .saturating_add(dropped_active_instances as u64);

        ParticleAdvanceSummary {
            ticks,
            intaken_instances,
            expired_instances,
            dropped_active_instances,
            pending_spawns: self.pending_spawns.len(),
            active_instances: self.active_instances.len(),
            total_instances_created: self.instances_created,
            total_instances_expired: self.instances_expired,
            total_dropped_active_instances: self.dropped_active_instances,
        }
    }

    fn tick_active_instances(&mut self) -> usize {
        let mut expired_instances = 0;
        let mut active_instances = VecDeque::with_capacity(self.active_instances.len());
        while let Some(mut instance) = self.active_instances.pop_front() {
            if instance.age_ticks >= instance.lifetime_ticks {
                expired_instances += 1;
                continue;
            }
            instance.tick_motion_without_collision();
            instance.age_ticks = instance.age_ticks.saturating_add(1);
            instance.update_sprite_from_age();
            active_instances.push_back(instance);
        }
        self.active_instances = active_instances;
        expired_instances
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
                .push_back(ParticleInstance::from_spawn_command(
                    command,
                    &mut self.random,
                ));
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
    fn from_spawn_command(command: ParticleSpawnCommand, random: &mut ParticleRandom) -> Self {
        let descriptor = ParticleDescriptor::for_particle(&command.particle_id);
        let velocity = descriptor.initial_velocity.sample(command.velocity, random);
        let (current_sprite_index, current_sprite_id) =
            select_initial_sprite(&command.sprite_ids, descriptor.sprite_selection, random);
        let visual = descriptor.visual.sample(random);
        Self {
            particle_type_id: command.particle_type_id,
            particle_id: command.particle_id,
            sprite_ids: command.sprite_ids,
            current_sprite_id,
            current_sprite_index,
            previous_position: command.position,
            position: command.position,
            velocity,
            age_ticks: 0,
            lifetime_ticks: descriptor.lifetime.sample(random),
            base_quad_size: visual.base_quad_size,
            color: visual.color,
            quad_size_curve: visual.quad_size_curve,
            provider: descriptor.provider.to_string(),
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            has_physics: descriptor.has_physics,
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            sprite_selection: descriptor.sprite_selection,
            override_limiter: command.override_limiter,
            always_show: command.always_show,
            raw_options_len: command.raw_options_len,
        }
    }

    fn render_quad_size(&self) -> f32 {
        self.quad_size_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
    }

    fn quad_size_at_partial_tick(&self, partial_tick: f32) -> f32 {
        let lifetime = self.lifetime_ticks.max(1) as f32;
        let age = (self.age_ticks as f32 + partial_tick.clamp(0.0, 1.0)).clamp(0.0, lifetime);
        let progress = age / lifetime;
        match self.quad_size_curve {
            ParticleQuadSizeCurve::Constant => self.base_quad_size,
            ParticleQuadSizeCurve::GrowToBase => {
                self.base_quad_size * (progress * 32.0).clamp(0.0, 1.0)
            }
            ParticleQuadSizeCurve::Flame => {
                self.base_quad_size * (1.0 - progress * progress * 0.5).max(0.0)
            }
        }
    }

    fn tick_motion_without_collision(&mut self) {
        self.previous_position = self.position;
        self.velocity[1] -= 0.04 * f64::from(self.gravity);
        self.position[0] += self.velocity[0];
        self.position[1] += self.velocity[1];
        self.position[2] += self.velocity[2];
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
    }

    fn update_sprite_from_age(&mut self) {
        if self.sprite_selection != ParticleSpriteSelection::Age {
            return;
        }
        let Some(index) =
            sprite_index_for_age(self.sprite_ids.len(), self.age_ticks, self.lifetime_ticks)
        else {
            self.current_sprite_index = None;
            self.current_sprite_id = None;
            return;
        };
        self.current_sprite_index = Some(index);
        self.current_sprite_id = self.sprite_ids.get(index).cloned();
    }
}

impl Renderer {
    pub fn upload_particle_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sprite_uvs: Vec<ParticleSpriteUv>,
    ) -> Result<()> {
        self.particle_atlas = Some(create_particle_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
            sprite_uvs,
        )?);
        Ok(())
    }

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
        self.counters.last_particle_expired_count = summary.expired_instances;
        self.counters.last_particle_active_drop_count = summary.dropped_active_instances;
        self.counters.particle_runtime_ticks = self
            .counters
            .particle_runtime_ticks
            .saturating_add(summary.ticks as u64);
        self.counters.particle_instances_created = summary.total_instances_created;
        self.counters.particle_instances_expired = summary.total_instances_expired;
        self.counters.dropped_active_particle_instances = summary.total_dropped_active_instances;
    }

    pub(crate) fn collect_particle_vertices(&self) -> Vec<ParticleVertex> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        let Some(atlas) = &self.particle_atlas else {
            return Vec::new();
        };
        particle_billboard_vertices(
            self.particles.active_instances.iter(),
            &atlas.sprite_uvs,
            camera_billboard_axes(pose),
        )
    }
}

fn particle_billboard_vertices<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    sprite_uvs: &BTreeMap<String, ParticleUvRect>,
    axes: ParticleBillboardAxes,
) -> Vec<ParticleVertex> {
    let mut vertices = Vec::new();
    for instance in instances {
        let Some(sprite_id) = instance.current_sprite_id.as_deref() else {
            continue;
        };
        let Some(uv) = sprite_uvs.get(sprite_id).copied() else {
            continue;
        };
        vertices.extend(particle_instance_vertices(instance, uv, axes));
    }
    vertices
}

#[derive(Debug, Clone, Copy)]
struct ParticleBillboardAxes {
    right: Vec3,
    up: Vec3,
}

fn camera_billboard_axes(pose: crate::CameraPose) -> ParticleBillboardAxes {
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let right = Vec3::Y.cross(forward).normalize_or_zero();
    let right = if right.length_squared() > 0.0 {
        right
    } else {
        Vec3::X
    };
    let up = forward.cross(right).normalize_or_zero();
    ParticleBillboardAxes {
        right,
        up: if up.length_squared() > 0.0 {
            up
        } else {
            Vec3::Y
        },
    }
}

fn particle_instance_vertices(
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    axes: ParticleBillboardAxes,
) -> [ParticleVertex; 6] {
    let center = Vec3::new(
        instance.position[0] as f32,
        instance.position[1] as f32,
        instance.position[2] as f32,
    );
    let half_size = instance.render_quad_size() * 0.5;
    let right = axes.right * half_size;
    let up = axes.up * half_size;
    let bottom_left = center - right - up;
    let bottom_right = center + right - up;
    let top_right = center + right + up;
    let top_left = center - right + up;
    let tint = instance.color;

    [
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint),
        particle_vertex(bottom_right, [uv.max[0], uv.max[1]], tint),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint),
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint),
        particle_vertex(top_left, [uv.min[0], uv.min[1]], tint),
    ]
}

fn particle_vertex(position: Vec3, uv: [f32; 2], color: [f32; 4]) -> ParticleVertex {
    ParticleVertex {
        position: position.to_array(),
        uv,
        color,
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
    fn particle_descriptor_falls_back_without_blocking_unknown_particles() {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:unknown_test_particle", 1.0)],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(0);

        assert_eq!(summary.intaken_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.provider, "Particle");
        assert!(instance.lifetime_ticks > 0);
        assert_eq!(instance.current_sprite_index, Some(0));
        assert_eq!(
            instance.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
        assert_close_f32(instance.friction, 0.98);
        assert_close_f32(instance.gravity, 0.0);
        assert!(instance.has_physics);
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
    fn particle_runtime_advances_motion_with_gravity_before_friction() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:smoke", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.5, 0.25, -0.5];
        instance.gravity = 0.5;
        instance.friction = 0.8;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.5, 2.23, 2.5]);
        assert_close3(instance.velocity, [0.4, 0.184, -0.4]);
    }

    #[test]
    fn particle_runtime_negative_gravity_increases_y_velocity_before_friction() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:poof", 20);
        instance.velocity = [0.0, 0.0, 0.0];
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_close3(instance.position, [0.0, 0.004, 0.0]);
        assert_close3(instance.velocity, [0.0, 0.0036, 0.0]);
    }

    #[test]
    fn particle_runtime_moves_particles_even_when_physics_is_disabled() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:flame", 20);
        assert!(!instance.has_physics);
        instance.velocity = [0.25, 0.5, 0.75];
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_close3(instance.position, [0.25, 0.5, 0.75]);
        assert_close3(instance.velocity, [0.24, 0.48, 0.72]);
    }

    #[test]
    fn particle_runtime_expires_after_vanilla_post_increment_lifetime_boundary() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles
            .active_instances
            .push_back(test_instance_with_lifetime("minecraft:poof", 2));

        let summary = particles.advance(2);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        assert_eq!(particles.active_instances()[0].age_ticks, 2);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 1);
        assert_eq!(summary.total_instances_expired, 1);
        assert_eq!(summary.active_instances, 0);
        assert!(particles.active_instances().is_empty());
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
        assert_eq!(particles.active_instances()[1].position, [2.0, 0.0, 0.0]);
    }

    #[test]
    fn particle_runtime_updates_age_based_sprite_frames_after_tick() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:smoke", 4);
        instance.sprite_ids = vec![
            "minecraft:smoke_0".to_string(),
            "minecraft:smoke_1".to_string(),
            "minecraft:smoke_2".to_string(),
        ];
        instance.current_sprite_index = Some(0);
        instance.current_sprite_id = Some("minecraft:smoke_0".to_string());
        particles.active_instances.push_back(instance);

        particles.advance(2);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 2);
        assert_eq!(instance.current_sprite_index, Some(1));
        assert_eq!(
            instance.current_sprite_id.as_deref(),
            Some("minecraft:smoke_1")
        );
    }

    #[test]
    fn particle_runtime_age_based_sprite_reaches_last_frame_at_lifetime_boundary() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:poof", 2);
        instance.sprite_ids = vec![
            "minecraft:poof_0".to_string(),
            "minecraft:poof_1".to_string(),
            "minecraft:poof_2".to_string(),
        ];
        instance.current_sprite_index = Some(0);
        instance.current_sprite_id = Some("minecraft:poof_0".to_string());
        particles.active_instances.push_back(instance);

        particles.advance(2);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 2);
        assert_eq!(instance.current_sprite_index, Some(2));
        assert_eq!(
            instance.current_sprite_id.as_deref(),
            Some("minecraft:poof_2")
        );
    }

    #[test]
    fn particle_runtime_keeps_random_sprite_selection_stable_after_tick() {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
        let mut command = spawn_command("minecraft:flame", 1.0);
        command.sprite_ids = vec![
            "minecraft:flame_0".to_string(),
            "minecraft:flame_1".to_string(),
            "minecraft:flame_2".to_string(),
        ];
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![command],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);
        let initial_sprite = particles.active_instances()[0].current_sprite_id.clone();
        assert!(initial_sprite.is_some());

        particles.advance(3);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(instance.current_sprite_id, initial_sprite);
    }

    #[test]
    fn particle_runtime_sets_initial_sprite_from_spawn_command_sprites() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:smoke", 1.0)],
            ..ParticleSpawnBatch::default()
        });

        particles.advance(0);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.current_sprite_index, Some(0));
        assert_eq!(
            instance.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
    }

    #[test]
    fn particle_runtime_handles_empty_sprite_sets_without_blocking_spawn() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut command = spawn_command("minecraft:smoke", 1.0);
        command.sprite_ids.clear();
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![command],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(0);

        assert_eq!(summary.intaken_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.current_sprite_index, None);
        assert_eq!(instance.current_sprite_id, None);
    }

    #[test]
    fn particle_runtime_uses_age_selection_for_ash_family_particles() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![
                spawn_command("minecraft:ash", 1.0),
                spawn_command("minecraft:white_ash", 2.0),
                spawn_command("minecraft:white_smoke", 3.0),
            ],
            ..ParticleSpawnBatch::default()
        });

        particles.advance(0);

        let selections: Vec<_> = particles
            .active_instances()
            .iter()
            .map(|instance| (instance.particle_id.as_str(), instance.sprite_selection))
            .collect();
        assert_eq!(
            selections,
            vec![
                ("minecraft:ash", ParticleSpriteSelection::Age),
                ("minecraft:white_ash", ParticleSpriteSelection::Age),
                ("minecraft:white_smoke", ParticleSpriteSelection::Age),
            ]
        );
    }

    #[test]
    fn particle_instances_sample_provider_visual_state() {
        let mut flame_random = ParticleRandom::new(42);
        let flame = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:flame", 1.0),
            &mut flame_random,
        );
        let mut small_flame_random = ParticleRandom::new(42);
        let small_flame = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:small_flame", 1.0),
            &mut small_flame_random,
        );
        assert_close_f32(small_flame.base_quad_size, flame.base_quad_size * 0.5);
        assert_eq!(flame.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(flame.quad_size_curve, ParticleQuadSizeCurve::Flame);

        let mut cloud_random = ParticleRandom::new(43);
        let cloud = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:cloud", 1.0),
            &mut cloud_random,
        );
        assert_eq!(cloud.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(cloud.base_quad_size, 0.1875, 0.375);
        assert_range_f32(cloud.color[0], 0.7, 1.0);
        assert_eq!(cloud.color[0], cloud.color[1]);
        assert_eq!(cloud.color[1], cloud.color[2]);

        let mut dragon_random = ParticleRandom::new(46);
        let dragon_breath = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dragon_breath", 1.0),
            &mut dragon_random,
        );
        assert_eq!(dragon_breath.provider, "DragonBreathParticle.Provider");
        assert_eq!(
            dragon_breath.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );
        assert_range_f32(dragon_breath.base_quad_size, 0.075, 0.15);
        assert_range_f32(dragon_breath.color[0], 0.717_647_1, 0.874_509_8);
        assert_close_f32(dragon_breath.color[1], 0.0);
        assert_range_f32(dragon_breath.color[2], 0.823_529_4, 0.976_470_6);
        assert_eq!(dragon_breath.friction, 0.96);
        assert_eq!(dragon_breath.gravity, 0.0);
        assert!(!dragon_breath.has_physics);

        let mut happy_villager_random = ParticleRandom::new(47);
        let happy_villager = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:happy_villager", 1.0),
            &mut happy_villager_random,
        );
        assert_eq!(
            happy_villager.provider,
            "SuspendedTownParticle.HappyVillagerProvider"
        );
        assert_eq!(
            happy_villager.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            happy_villager.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(happy_villager.base_quad_size, 0.05, 0.22);
        assert_eq!(happy_villager.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(happy_villager.friction, 0.99);
        assert_eq!(happy_villager.gravity, 0.0);
        assert!(happy_villager.has_physics);
        assert_ne!(happy_villager.velocity, [0.0, 0.0, 0.0]);

        let mut smoke_random = ParticleRandom::new(44);
        let smoke = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:white_smoke", 1.0),
            &mut smoke_random,
        );
        assert_eq!(smoke.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_close_f32(smoke.color[0], 186.0 / 255.0);
        assert_close_f32(smoke.color[1], 177.0 / 255.0);
        assert_close_f32(smoke.color[2], 194.0 / 255.0);

        let mut poof_random = ParticleRandom::new(45);
        let poof = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:poof", 1.0),
            &mut poof_random,
        );
        assert_eq!(poof.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(poof.base_quad_size, 0.1, 0.7);
        assert_range_f32(poof.color[0], 0.7, 1.0);
    }

    #[test]
    fn particle_quad_size_curves_follow_vanilla_shapes() {
        let mut cloud = test_instance_with_lifetime("minecraft:cloud", 64);
        cloud.base_quad_size = 0.4;
        cloud.quad_size_curve = ParticleQuadSizeCurve::GrowToBase;
        assert_close_f32(cloud.quad_size_at_partial_tick(0.0), 0.0);
        assert_close_f32(cloud.quad_size_at_partial_tick(0.5), 0.1);
        cloud.age_ticks = 2;
        assert_close_f32(cloud.quad_size_at_partial_tick(0.0), 0.4);

        let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
        flame.base_quad_size = 0.2;
        flame.quad_size_curve = ParticleQuadSizeCurve::Flame;
        assert_close_f32(flame.quad_size_at_partial_tick(0.0), 0.2);
        flame.age_ticks = 20;
        assert_close_f32(flame.quad_size_at_partial_tick(0.0), 0.1);
    }

    #[test]
    fn particle_runtime_expires_existing_active_before_intaking_pending_spawns() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        particles
            .active_instances
            .push_back(test_instance_with_lifetime("minecraft:poof", 0));
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:flame", 2.0)],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 1);
        assert_eq!(summary.intaken_instances, 1);
        assert_eq!(summary.active_instances, 1);
        assert_eq!(
            particles.active_instances()[0].particle_id,
            "minecraft:flame"
        );
        assert_eq!(particles.active_instances()[0].age_ticks, 0);
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

    #[test]
    fn particle_billboard_vertices_emit_camera_facing_textured_quad() {
        let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.4;
        instance.color = [0.25, 0.5, 0.75, 0.8];
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.25, 0.125],
                max: [0.5, 0.375],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
        );

        assert_eq!(vertices.len(), 6);
        assert_close3_f32(vertices[0].position, [0.8, 1.8, 3.0]);
        assert_eq!(vertices[0].uv, [0.25, 0.375]);
        assert_eq!(vertices[0].color, [0.25, 0.5, 0.75, 0.8]);
        assert_close3_f32(vertices[2].position, [1.2, 2.2, 3.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.125]);
        assert_eq!(vertices[2].color, [0.25, 0.5, 0.75, 0.8]);
        assert_close3_f32(vertices[5].position, [0.8, 2.2, 3.0]);
        assert_eq!(vertices[5].uv, [0.25, 0.125]);
        assert_eq!(vertices[5].color, [0.25, 0.5, 0.75, 0.8]);
    }

    #[test]
    fn particle_billboard_vertices_skip_instances_without_uploaded_sprite_uv() {
        let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
        instance.current_sprite_id = Some("minecraft:missing".to_string());

        let vertices = particle_billboard_vertices(
            [&instance],
            &BTreeMap::new(),
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
        );

        assert!(vertices.is_empty());
    }

    fn test_instance_with_lifetime(particle_id: &str, lifetime_ticks: u32) -> ParticleInstance {
        let descriptor = ParticleDescriptor::for_particle(particle_id);
        ParticleInstance {
            particle_type_id: 0,
            particle_id: particle_id.to_string(),
            sprite_ids: Vec::new(),
            current_sprite_id: None,
            current_sprite_index: None,
            previous_position: [0.0, 0.0, 0.0],
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            age_ticks: 0,
            lifetime_ticks,
            base_quad_size: DEFAULT_PARTICLE_QUAD_SIZE,
            color: [1.0, 1.0, 1.0, 1.0],
            quad_size_curve: ParticleQuadSizeCurve::Constant,
            provider: descriptor.provider.to_string(),
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            has_physics: descriptor.has_physics,
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            sprite_selection: descriptor.sprite_selection,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
        }
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

    fn assert_close_f32(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_range_f32(actual: f32, min: f32, max: f32) {
        assert!(
            actual >= min && actual <= max,
            "expected {actual} to be in {min}..={max}"
        );
    }

    fn assert_close3(actual: [f64; 3], expected: [f64; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-6,
                "expected {expected}, got {actual}"
            );
        }
    }

    fn assert_close3_f32(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-6,
                "expected {expected}, got {actual}"
            );
        }
    }
}
