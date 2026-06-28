use std::collections::{BTreeMap, VecDeque};

use anyhow::Result;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::Renderer;

mod descriptors;
mod gpu;

use descriptors::{
    select_initial_sprite, sprite_index_for_age, ParticleDescriptor, ParticleQuadSizeCurve,
    ParticleRandom, ParticleSpriteSelection, ParticleTickMotionDescriptor,
    DEFAULT_PARTICLE_RANDOM_SEED,
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
    #[serde(default)]
    pub(crate) tick_motion: ParticleTickMotionDescriptor,
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
        let position = descriptor.initial_position(command.position, random);
        let velocity = descriptor.initial_velocity.sample(command.velocity, random);
        let (current_sprite_index, current_sprite_id) =
            select_initial_sprite(&command.sprite_ids, descriptor.sprite_selection, random);
        let visual = descriptor
            .visual
            .sample_for_command(random, command.velocity);
        Self {
            particle_type_id: command.particle_type_id,
            particle_id: command.particle_id,
            sprite_ids: command.sprite_ids,
            current_sprite_id,
            current_sprite_index,
            previous_position: position,
            position,
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
            tick_motion: descriptor.tick_motion(),
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
        match self.tick_motion {
            ParticleTickMotionDescriptor::DefaultParticleTick => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
            }
            ParticleTickMotionDescriptor::DirectGravityNoFriction => {
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
            }
            ParticleTickMotionDescriptor::NoMotion => {}
        }
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
    fn particle_runtime_bubble_pop_subtracts_full_gravity_without_friction() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:bubble_pop", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.5, 0.25, -0.5];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.5, 2.242, 2.5]);
        assert_close3(instance.velocity, [0.5, 0.242, -0.5]);
    }

    #[test]
    fn particle_runtime_no_motion_tick_preserves_attack_sweep_position_and_velocity() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:sweep_attack", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = [0.0, 0.0, 0.0];
        instance.velocity = [0.5, 0.25, -0.5];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.0, 2.0, 3.0]);
        assert_close3(instance.velocity, [0.5, 0.25, -0.5]);
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

        let mut lava_random = ParticleRandom::new(44);
        let lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:lava", 1.0),
            &mut lava_random,
        );
        assert_eq!(lava.provider, "LavaParticle.Provider");
        assert_eq!(lava.sprite_selection, ParticleSpriteSelection::Random);
        assert_range_f32(lava.base_quad_size, 0.02, 0.44);
        assert_eq!(lava.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(lava.quad_size_curve, ParticleQuadSizeCurve::Flame);
        assert!((16..=80).contains(&lava.lifetime_ticks));
        assert_range_f64(lava.velocity[0], -0.15, 0.15);
        assert_range_f64(lava.velocity[1], 0.05, 0.45);
        assert_range_f64(lava.velocity[2], -0.15, 0.15);
        assert_eq!(lava.friction, 0.999);
        assert_eq!(lava.gravity, 0.75);
        assert!(lava.has_physics);

        let mut soul_random = ParticleRandom::new(68);
        let mut soul_command = spawn_command("minecraft:soul", 1.0);
        soul_command.position = [1.0, 2.0, 3.0];
        soul_command.velocity = [1.0, 2.0, 3.0];
        let soul = ParticleInstance::from_spawn_command(soul_command, &mut soul_random);
        assert_eq!(soul.provider, "SoulParticle.Provider");
        assert_eq!(soul.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(soul.base_quad_size, 0.15, 0.3);
        assert_eq!(soul.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(soul.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert!((12..=44).contains(&soul.lifetime_ticks));
        assert_eq!(soul.friction, 0.96);
        assert_eq!(soul.gravity, 0.0);
        assert!(soul.has_physics);
        assert!(!soul.speed_up_when_y_motion_is_blocked);
        assert_range_f64(soul.position[0], 0.95, 1.05);
        assert_range_f64(soul.position[1], 1.95, 2.05);
        assert_range_f64(soul.position[2], 2.95, 3.05);
        assert_range_f64(soul.velocity[0], 0.998, 1.002);
        assert_range_f64(soul.velocity[1], 2.0, 2.003);
        assert_range_f64(soul.velocity[2], 2.998, 3.002);

        let mut sculk_soul_random = ParticleRandom::new(69);
        let sculk_soul = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:sculk_soul", 1.0),
            &mut sculk_soul_random,
        );
        assert_eq!(sculk_soul.provider, "SoulParticle.EmissiveProvider");
        assert_eq!(sculk_soul.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(sculk_soul.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((12..=44).contains(&sculk_soul.lifetime_ticks));
        assert!(sculk_soul.has_physics);

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
        assert_ne!(cloud.velocity, [0.0, 0.0, 0.0]);

        let mut bubble_random = ParticleRandom::new(59);
        let mut bubble_command = spawn_command("minecraft:bubble", 1.0);
        bubble_command.velocity = [1.0, 2.0, 3.0];
        let bubble = ParticleInstance::from_spawn_command(bubble_command, &mut bubble_random);
        assert_eq!(bubble.provider, "BubbleParticle.Provider");
        assert_eq!(bubble.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(bubble.base_quad_size, 0.02, 0.16);
        assert_eq!(bubble.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((8..=40).contains(&bubble.lifetime_ticks));
        assert_eq!(bubble.friction, 0.85);
        assert_eq!(bubble.gravity, -0.05);
        assert!(bubble.has_physics);
        assert_range_f64(bubble.velocity[0], 0.18, 0.22);
        assert_range_f64(bubble.velocity[1], 0.38, 0.42);
        assert_range_f64(bubble.velocity[2], 0.58, 0.62);

        let mut column_bubble_random = ParticleRandom::new(60);
        let mut column_bubble_command = spawn_command("minecraft:bubble_column_up", 1.0);
        column_bubble_command.velocity = [1.0, 2.0, 3.0];
        let column_bubble =
            ParticleInstance::from_spawn_command(column_bubble_command, &mut column_bubble_random);
        assert_eq!(column_bubble.provider, "BubbleColumnUpParticle.Provider");
        assert_eq!(
            column_bubble.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(column_bubble.base_quad_size, 0.02, 0.16);
        assert!((40..=200).contains(&column_bubble.lifetime_ticks));
        assert_eq!(column_bubble.friction, 0.85);
        assert_eq!(column_bubble.gravity, -0.125);
        assert!(column_bubble.has_physics);
        assert_range_f64(column_bubble.velocity[0], 0.18, 0.22);
        assert_range_f64(column_bubble.velocity[1], 0.38, 0.42);
        assert_range_f64(column_bubble.velocity[2], 0.58, 0.62);

        let mut bubble_pop_random = ParticleRandom::new(75);
        let mut bubble_pop_command = spawn_command("minecraft:bubble_pop", 1.0);
        bubble_pop_command.velocity = [1.0, 2.0, 3.0];
        let bubble_pop =
            ParticleInstance::from_spawn_command(bubble_pop_command, &mut bubble_pop_random);
        assert_eq!(bubble_pop.provider, "BubblePopParticle.Provider");
        assert_eq!(bubble_pop.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(bubble_pop.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(bubble_pop.base_quad_size, 0.1, 0.2);
        assert_eq!(bubble_pop.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(bubble_pop.lifetime_ticks, 4);
        assert_eq!(bubble_pop.friction, 0.98);
        assert_eq!(bubble_pop.gravity, 0.008);
        assert!(bubble_pop.has_physics);
        assert_eq!(bubble_pop.velocity, [1.0, 2.0, 3.0]);
        assert_eq!(
            bubble_pop.tick_motion,
            ParticleTickMotionDescriptor::DirectGravityNoFriction
        );

        let mut sweep_random = ParticleRandom::new(76);
        let mut sweep_command = spawn_command("minecraft:sweep_attack", 1.0);
        sweep_command.velocity = [0.5, 0.0, 0.0];
        let sweep = ParticleInstance::from_spawn_command(sweep_command, &mut sweep_random);
        assert_eq!(sweep.provider, "AttackSweepParticle.Provider");
        assert_eq!(sweep.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(sweep.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_close_f32(sweep.base_quad_size, 0.75);
        assert_range_f32(sweep.color[0], 0.4, 1.0);
        assert_eq!(sweep.color[0], sweep.color[1]);
        assert_eq!(sweep.color[1], sweep.color[2]);
        assert_eq!(sweep.color[3], 1.0);
        assert_eq!(sweep.lifetime_ticks, 4);
        assert_eq!(sweep.friction, 0.98);
        assert_eq!(sweep.gravity, 0.0);
        assert!(sweep.has_physics);
        assert_ne!(sweep.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(sweep.tick_motion, ParticleTickMotionDescriptor::NoMotion);

        let mut underwater_random = ParticleRandom::new(77);
        let underwater = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:underwater", 1.0),
            &mut underwater_random,
        );
        assert_eq!(underwater.provider, "SuspendedParticle.UnderwaterProvider");
        assert_eq!(underwater.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(underwater.current_sprite_index, Some(0));
        assert_eq!(
            underwater.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
        assert_eq!(underwater.previous_position, [1.0, -0.125, 0.0]);
        assert_eq!(underwater.position, [1.0, -0.125, 0.0]);
        assert_eq!(underwater.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(underwater.base_quad_size, 0.02, 0.16);
        assert_eq!(underwater.color, [0.4, 0.4, 0.7, 1.0]);
        assert!((8..=40).contains(&underwater.lifetime_ticks));
        assert_eq!(underwater.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(underwater.friction, 1.0);
        assert_eq!(underwater.gravity, 0.0);
        assert!(!underwater.has_physics);

        let mut glow_random = ParticleRandom::new(67);
        let mut glow_command = spawn_command("minecraft:glow", 1.0);
        glow_command.velocity = [0.0, 1.0, 0.0];
        let glow = ParticleInstance::from_spawn_command(glow_command, &mut glow_random);
        assert_eq!(glow.provider, "GlowParticle.GlowSquidProvider");
        assert_eq!(glow.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(glow.base_quad_size, 0.075, 0.15);
        assert!(glow.color == [0.6, 1.0, 0.8, 1.0] || glow.color == [0.08, 0.4, 0.4, 1.0]);
        assert!((8..=40).contains(&glow.lifetime_ticks));
        assert_eq!(glow.friction, 0.96);
        assert!(!glow.has_physics);
        assert!(glow.speed_up_when_y_motion_is_blocked);
        assert_range_f64(glow.velocity[0].abs(), 0.0, 0.02);
        assert_range_f64(glow.velocity[1], 0.015, 0.08);
        assert_range_f64(glow.velocity[2].abs(), 0.0, 0.02);

        let mut electric_random = ParticleRandom::new(63);
        let mut electric_command = spawn_command("minecraft:electric_spark", 1.0);
        electric_command.velocity = [2.0, 3.0, 4.0];
        let electric = ParticleInstance::from_spawn_command(electric_command, &mut electric_random);
        assert_eq!(electric.provider, "GlowParticle.ElectricSparkProvider");
        assert_eq!(electric.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(electric.base_quad_size, 0.075, 0.15);
        assert_eq!(electric.color, [1.0, 0.9, 1.0, 1.0]);
        assert!((2..=3).contains(&electric.lifetime_ticks));
        assert_eq!(electric.friction, 0.96);
        assert!(!electric.has_physics);
        assert!(electric.speed_up_when_y_motion_is_blocked);
        assert_range_f64(electric.velocity[0], 0.499, 0.501);
        assert_range_f64(electric.velocity[1], 0.749, 0.751);
        assert_range_f64(electric.velocity[2], 0.999, 1.001);

        let mut scrape_random = ParticleRandom::new(64);
        let mut scrape_command = spawn_command("minecraft:scrape", 1.0);
        scrape_command.velocity = [2.0, 3.0, 4.0];
        let scrape = ParticleInstance::from_spawn_command(scrape_command, &mut scrape_random);
        assert_eq!(scrape.provider, "GlowParticle.ScrapeProvider");
        assert!(scrape.color == [0.29, 0.58, 0.51, 1.0] || scrape.color == [0.43, 0.77, 0.62, 1.0]);
        assert!((10..=39).contains(&scrape.lifetime_ticks));
        assert_range_f64(scrape.velocity[0], 0.019, 0.021);
        assert_range_f64(scrape.velocity[1], 0.029, 0.031);
        assert_range_f64(scrape.velocity[2], 0.039, 0.041);

        let mut wax_on_random = ParticleRandom::new(65);
        let mut wax_on_command = spawn_command("minecraft:wax_on", 1.0);
        wax_on_command.velocity = [2.0, 3.0, 4.0];
        let wax_on = ParticleInstance::from_spawn_command(wax_on_command, &mut wax_on_random);
        assert_eq!(wax_on.provider, "GlowParticle.WaxOnProvider");
        assert_eq!(wax_on.color, [0.91, 0.55, 0.08, 1.0]);
        assert!((10..=39).contains(&wax_on.lifetime_ticks));
        assert_range_f64(wax_on.velocity[0], 0.009, 0.011);
        assert_range_f64(wax_on.velocity[1], 0.029, 0.031);
        assert_range_f64(wax_on.velocity[2], 0.019, 0.021);

        let mut wax_off_random = ParticleRandom::new(66);
        let mut wax_off_command = spawn_command("minecraft:wax_off", 1.0);
        wax_off_command.velocity = [2.0, 3.0, 4.0];
        let wax_off = ParticleInstance::from_spawn_command(wax_off_command, &mut wax_off_random);
        assert_eq!(wax_off.provider, "GlowParticle.WaxOffProvider");
        assert_eq!(wax_off.color, [1.0, 0.9, 1.0, 1.0]);
        assert!((10..=39).contains(&wax_off.lifetime_ticks));
        assert_range_f64(wax_off.velocity[0], 0.009, 0.011);
        assert_range_f64(wax_off.velocity[1], 0.029, 0.031);
        assert_range_f64(wax_off.velocity[2], 0.019, 0.021);

        let mut sneeze_random = ParticleRandom::new(55);
        let sneeze = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:sneeze", 1.0),
            &mut sneeze_random,
        );
        assert_eq!(sneeze.provider, "PlayerCloudParticle.SneezeProvider");
        assert_eq!(sneeze.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(sneeze.base_quad_size, 0.1875, 0.375);
        assert_eq!(sneeze.color, [0.22, 1.0, 0.53, 0.4]);
        assert_eq!(sneeze.friction, 0.96);
        assert!(!sneeze.has_physics);
        assert_ne!(sneeze.velocity, [0.0, 0.0, 0.0]);

        let mut snowflake_random = ParticleRandom::new(56);
        let mut snowflake_command = spawn_command("minecraft:snowflake", 1.0);
        snowflake_command.velocity = [1.0, 2.0, 3.0];
        let snowflake =
            ParticleInstance::from_spawn_command(snowflake_command, &mut snowflake_random);
        assert_eq!(snowflake.provider, "SnowflakeParticle.Provider");
        assert_eq!(snowflake.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(snowflake.base_quad_size, 0.1, 0.2);
        assert_eq!(snowflake.color, [0.923, 0.964, 0.999, 1.0]);
        assert_eq!(snowflake.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert!((18..=82).contains(&snowflake.lifetime_ticks));
        assert_range_f64(snowflake.velocity[0], 0.95, 1.05);
        assert_range_f64(snowflake.velocity[1], 1.95, 2.05);
        assert_range_f64(snowflake.velocity[2], 2.95, 3.05);
        assert_eq!(snowflake.friction, 1.0);
        assert_eq!(snowflake.gravity, 0.225);
        assert!(snowflake.has_physics);

        let mut note_random = ParticleRandom::new(54);
        let mut note_command = spawn_command("minecraft:note", 1.0);
        note_command.velocity = [0.0, 0.0, 0.0];
        let note = ParticleInstance::from_spawn_command(note_command, &mut note_random);
        assert_eq!(note.provider, "NoteParticle.Provider");
        assert_eq!(note.lifetime_ticks, 6);
        assert_eq!(note.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(note.base_quad_size, 0.15, 0.3);
        assert_close_f32(note.color[0], 0.35);
        assert_close_f32(note.color[1], 0.912_916_5);
        assert_close_f32(note.color[2], 0.0);
        assert_eq!(note.color[3], 1.0);
        assert_eq!(note.friction, 0.66);
        assert!(note.has_physics);
        assert!(note.speed_up_when_y_motion_is_blocked);
        assert_range_f64(note.velocity[1], 0.198, 0.202);

        let mut spell_random = ParticleRandom::new(61);
        let mut spell_command = spawn_command("minecraft:infested", 1.0);
        spell_command.velocity = [0.0, 1.0, 0.0];
        let spell = ParticleInstance::from_spawn_command(spell_command, &mut spell_random);
        assert_eq!(spell.provider, "SpellParticle.Provider");
        assert_eq!(spell.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(spell.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(spell.base_quad_size, 0.075, 0.15);
        assert_eq!(spell.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((8..=40).contains(&spell.lifetime_ticks));
        assert_eq!(spell.friction, 0.96);
        assert_eq!(spell.gravity, -0.1);
        assert!(!spell.has_physics);
        assert!(spell.speed_up_when_y_motion_is_blocked);
        assert_range_f64(spell.velocity[0].abs(), 0.0, 0.008);
        assert_range_f64(spell.velocity[1], 0.0, 0.06);
        assert_range_f64(spell.velocity[2].abs(), 0.0, 0.008);

        let mut witch_random = ParticleRandom::new(62);
        let witch = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:witch", 1.0),
            &mut witch_random,
        );
        assert_eq!(witch.provider, "SpellParticle.WitchProvider");
        assert_eq!(witch.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(witch.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(witch.base_quad_size, 0.075, 0.15);
        assert_range_f32(witch.color[0], 0.35, 0.85);
        assert_eq!(witch.color[1], 0.0);
        assert_eq!(witch.color[2], witch.color[0]);
        assert_eq!(witch.color[3], 1.0);
        assert!((8..=40).contains(&witch.lifetime_ticks));
        assert_eq!(witch.friction, 0.96);
        assert_eq!(witch.gravity, -0.1);
        assert!(!witch.has_physics);
        assert!(witch.speed_up_when_y_motion_is_blocked);

        let mut crit_random = ParticleRandom::new(56);
        let mut crit_command = spawn_command("minecraft:crit", 1.0);
        crit_command.velocity = [0.5, 0.25, -0.5];
        let crit = ParticleInstance::from_spawn_command(crit_command, &mut crit_random);
        assert_eq!(crit.provider, "CritParticle.Provider");
        assert_eq!(crit.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(crit.base_quad_size, 0.075, 0.15);
        assert_range_f32(crit.color[0], 0.6, 0.9);
        assert_close_f32(crit.color[1], crit.color[0] * 0.96);
        assert_close_f32(crit.color[2], crit.color[0] * 0.9);
        assert_eq!(crit.color[3], 1.0);
        assert!((4..=10).contains(&crit.lifetime_ticks));
        assert_eq!(crit.friction, 0.7);
        assert_eq!(crit.gravity, 0.5);
        assert!(!crit.has_physics);
        assert_range_f64(crit.velocity[0], 0.19, 0.21);
        assert_range_f64(crit.velocity[1], 0.10, 0.12);
        assert_range_f64(crit.velocity[2], -0.21, -0.19);

        let mut damage_random = ParticleRandom::new(57);
        let mut damage_command = spawn_command("minecraft:damage_indicator", 1.0);
        damage_command.velocity = [0.0, 0.0, 0.0];
        let damage = ParticleInstance::from_spawn_command(damage_command, &mut damage_random);
        assert_eq!(damage.provider, "CritParticle.DamageIndicatorProvider");
        assert_eq!(damage.lifetime_ticks, 20);
        assert_range_f64(damage.velocity[1], 0.40, 0.43);

        let mut magic_random = ParticleRandom::new(58);
        let magic = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:enchanted_hit", 1.0),
            &mut magic_random,
        );
        assert_eq!(magic.provider, "CritParticle.MagicProvider");
        assert_range_f32(magic.color[0], 0.18, 0.27);
        assert!(magic.color[1] > magic.color[0]);
        assert!(magic.color[2] > magic.color[1]);
        assert!((4..=10).contains(&magic.lifetime_ticks));

        let mut angry_villager_random = ParticleRandom::new(52);
        let angry_villager = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:angry_villager", 1.0),
            &mut angry_villager_random,
        );
        assert_eq!(
            angry_villager.provider,
            "HeartParticle.AngryVillagerProvider"
        );
        assert_eq!(angry_villager.previous_position, [1.0, 0.5, 0.0]);
        assert_eq!(angry_villager.position, [1.0, 0.5, 0.0]);
        assert_eq!(angry_villager.lifetime_ticks, 16);
        assert_eq!(
            angry_villager.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );
        assert_eq!(angry_villager.color, [1.0, 1.0, 1.0, 1.0]);

        let mut heart_random = ParticleRandom::new(51);
        let heart = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:heart", 1.0),
            &mut heart_random,
        );
        assert_eq!(heart.provider, "HeartParticle.Provider");
        assert_eq!(heart.lifetime_ticks, 16);
        assert_eq!(heart.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(heart.base_quad_size, 0.15, 0.3);
        assert_eq!(heart.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(heart.friction, 0.86);
        assert!(!heart.has_physics);
        assert!(heart.speed_up_when_y_motion_is_blocked);
        assert_range_f64(heart.velocity[1], 0.098, 0.102);

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

        let mut end_rod_random = ParticleRandom::new(79);
        let mut end_rod_command = spawn_command("minecraft:end_rod", 1.0);
        end_rod_command.velocity = [1.0, 2.0, 3.0];
        let end_rod = ParticleInstance::from_spawn_command(end_rod_command, &mut end_rod_random);
        assert_eq!(end_rod.provider, "EndRodParticle.Provider");
        assert_eq!(end_rod.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(end_rod.base_quad_size, 0.075, 0.15);
        assert_eq!(end_rod.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(end_rod.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert!((60..=71).contains(&end_rod.lifetime_ticks));
        assert_eq!(end_rod.velocity, [1.0, 2.0, 3.0]);
        assert_eq!(end_rod.friction, 0.91);
        assert_eq!(end_rod.gravity, 0.0125);
        assert!(end_rod.has_physics);

        let mut dolphin_random = ParticleRandom::new(53);
        let dolphin = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dolphin", 1.0),
            &mut dolphin_random,
        );
        assert_eq!(
            dolphin.provider,
            "SuspendedTownParticle.DolphinSpeedProvider"
        );
        assert!((10..=50).contains(&dolphin.lifetime_ticks));
        assert_close_f32(dolphin.color[0], 0.3);
        assert_close_f32(dolphin.color[1], 0.5);
        assert_close_f32(dolphin.color[2], 1.0);
        assert_range_f32(dolphin.color[3], 0.3, 1.0);
        assert_eq!(dolphin.friction, 0.99);
        assert!(dolphin.has_physics);
        assert_ne!(dolphin.velocity, [0.0, 0.0, 0.0]);

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

        let mut composter_random = ParticleRandom::new(48);
        let composter = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:composter", 1.0),
            &mut composter_random,
        );
        assert_eq!(
            composter.provider,
            "SuspendedTownParticle.ComposterFillProvider"
        );
        assert!((3..=7).contains(&composter.lifetime_ticks));
        assert_eq!(composter.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_eq!(composter.color, [1.0, 1.0, 1.0, 1.0]);
        assert_ne!(composter.velocity, [0.0, 0.0, 0.0]);

        let mut mycelium_random = ParticleRandom::new(49);
        let mycelium = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:mycelium", 1.0),
            &mut mycelium_random,
        );
        assert_eq!(mycelium.provider, "SuspendedTownParticle.Provider");
        assert_range_f32(mycelium.color[0], 0.2, 0.3);
        assert_eq!(mycelium.color[0], mycelium.color[1]);
        assert_eq!(mycelium.color[1], mycelium.color[2]);
        assert_eq!(mycelium.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_ne!(mycelium.velocity, [0.0, 0.0, 0.0]);

        let mut egg_crack_random = ParticleRandom::new(50);
        let egg_crack = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:egg_crack", 1.0),
            &mut egg_crack_random,
        );
        assert_eq!(egg_crack.provider, "SuspendedTownParticle.EggCrackProvider");
        assert_eq!(egg_crack.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(egg_crack.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_ne!(egg_crack.velocity, [0.0, 0.0, 0.0]);

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

        let mut explosion_random = ParticleRandom::new(70);
        let mut explosion_command = spawn_command("minecraft:explosion", 1.0);
        explosion_command.velocity = [0.5, 2.0, 3.0];
        let explosion =
            ParticleInstance::from_spawn_command(explosion_command, &mut explosion_random);
        assert_eq!(explosion.provider, "HugeExplosionParticle.Provider");
        assert_eq!(explosion.sprite_selection, ParticleSpriteSelection::Age);
        assert_close_f32(explosion.base_quad_size, 1.5);
        assert_range_f32(explosion.color[0], 0.4, 1.0);
        assert_eq!(explosion.color[0], explosion.color[1]);
        assert_eq!(explosion.color[1], explosion.color[2]);
        assert_eq!(explosion.color[3], 1.0);
        assert!((6..=9).contains(&explosion.lifetime_ticks));
        assert_eq!(explosion.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(explosion.friction, 0.98);
        assert!(explosion.has_physics);

        let mut sonic_boom_random = ParticleRandom::new(73);
        let mut sonic_boom_command = spawn_command("minecraft:sonic_boom", 1.0);
        sonic_boom_command.velocity = [1.0, 2.0, 3.0];
        let sonic_boom =
            ParticleInstance::from_spawn_command(sonic_boom_command, &mut sonic_boom_random);
        assert_eq!(sonic_boom.provider, "SonicBoomParticle.Provider");
        assert_eq!(sonic_boom.sprite_selection, ParticleSpriteSelection::Age);
        assert_close_f32(sonic_boom.base_quad_size, 1.5);
        assert_range_f32(sonic_boom.color[0], 0.4, 1.0);
        assert_eq!(sonic_boom.color[0], sonic_boom.color[1]);
        assert_eq!(sonic_boom.color[1], sonic_boom.color[2]);
        assert_eq!(sonic_boom.lifetime_ticks, 16);
        assert_eq!(sonic_boom.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(sonic_boom.friction, 0.98);
        assert!(sonic_boom.has_physics);

        let mut sculk_charge_random = ParticleRandom::new(78);
        let mut sculk_charge_command = spawn_command("minecraft:sculk_charge", 1.0);
        sculk_charge_command.velocity = [1.0, 2.0, 3.0];
        let sculk_charge =
            ParticleInstance::from_spawn_command(sculk_charge_command, &mut sculk_charge_random);
        assert_eq!(sculk_charge.provider, "SculkChargeParticle.Provider");
        assert_eq!(sculk_charge.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(sculk_charge.base_quad_size, 0.15, 0.3);
        assert_eq!(sculk_charge.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            sculk_charge.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert!((8..=19).contains(&sculk_charge.lifetime_ticks));
        assert_eq!(sculk_charge.velocity, [1.0, 2.0, 3.0]);
        assert_eq!(sculk_charge.friction, 0.96);
        assert_eq!(sculk_charge.gravity, 0.0);
        assert!(!sculk_charge.has_physics);

        let mut sculk_charge_pop_random = ParticleRandom::new(74);
        let mut sculk_charge_pop_command = spawn_command("minecraft:sculk_charge_pop", 1.0);
        sculk_charge_pop_command.velocity = [1.0, 2.0, 3.0];
        let sculk_charge_pop = ParticleInstance::from_spawn_command(
            sculk_charge_pop_command,
            &mut sculk_charge_pop_random,
        );
        assert_eq!(sculk_charge_pop.provider, "SculkChargePopParticle.Provider");
        assert_eq!(
            sculk_charge_pop.sprite_selection,
            ParticleSpriteSelection::Age
        );
        assert_range_f32(sculk_charge_pop.base_quad_size, 0.1, 0.2);
        assert_eq!(sculk_charge_pop.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((6..=9).contains(&sculk_charge_pop.lifetime_ticks));
        assert_eq!(sculk_charge_pop.velocity, [1.0, 2.0, 3.0]);
        assert_eq!(sculk_charge_pop.friction, 0.96);
        assert!(!sculk_charge_pop.has_physics);

        let mut gust_random = ParticleRandom::new(71);
        let mut gust_command = spawn_command("minecraft:gust", 1.0);
        gust_command.velocity = [1.0, 2.0, 3.0];
        let gust = ParticleInstance::from_spawn_command(gust_command, &mut gust_random);
        assert_eq!(gust.provider, "GustParticle.Provider");
        assert_eq!(gust.sprite_selection, ParticleSpriteSelection::Age);
        assert_close_f32(gust.base_quad_size, 1.0);
        assert_eq!(gust.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((12..=15).contains(&gust.lifetime_ticks));
        assert_eq!(gust.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(gust.friction, 0.98);
        assert!(gust.has_physics);

        let mut small_gust_random = ParticleRandom::new(72);
        let small_gust = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:small_gust", 1.0),
            &mut small_gust_random,
        );
        assert_eq!(small_gust.provider, "GustParticle.SmallProvider");
        assert_eq!(small_gust.sprite_selection, ParticleSpriteSelection::Age);
        assert_close_f32(small_gust.base_quad_size, 0.15);
        assert_eq!(small_gust.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((12..=15).contains(&small_gust.lifetime_ticks));
        assert_eq!(small_gust.velocity, [0.0, 0.0, 0.0]);
        assert!(small_gust.has_physics);
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
            tick_motion: descriptor.tick_motion(),
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

    fn assert_range_f64(actual: f64, min: f64, max: f64) {
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
