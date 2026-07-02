use std::collections::{BTreeMap, VecDeque};

use anyhow::Result;
use glam::{EulerRot, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::Renderer;

mod descriptors;
mod gpu;

use descriptors::{
    dust_lifetime, particle_limit_for_particle, select_initial_sprite, sprite_index_for_age,
    FallingLeavesDescriptor, ParticleAlphaCurve, ParticleChildEmissionDescriptor,
    ParticleDescriptor, ParticleFacingCameraMode, ParticleLightEmissionDescriptor,
    ParticleLimitDescriptor, ParticleQuadSizeCurve, ParticleRandom, ParticleSpriteSelection,
    ParticleTickMotionDescriptor, DEFAULT_PARTICLE_RANDOM_SEED,
};
pub(super) use gpu::{
    create_particle_atlas_gpu, create_particle_pipeline, ParticleAtlasGpu, ParticlePipelineKind,
    ParticleVertex,
};

const DEFAULT_MAX_PENDING_PARTICLE_SPAWNS: usize = 16_384;
const DEFAULT_MAX_ACTIVE_PARTICLE_INSTANCES: usize = 16_384;
const DEFAULT_PARTICLE_QUAD_SIZE: f32 = 0.2;
const DEFAULT_PARTICLE_RENDER_PARTIAL_TICK: f32 = 0.5;
const DEFAULT_PARTICLE_LIGHT: [f32; 2] = [1.0, 1.0];
const SHRIEK_MAGICAL_X_ROT: f32 = 1.0472;
const LAVA_CHILD_SMOKE_PARTICLE_ID: &str = "minecraft:smoke";
const HUGE_EXPLOSION_CHILD_PARTICLE_ID: &str = "minecraft:explosion";
const GUST_CHILD_PARTICLE_ID: &str = "minecraft:gust";
const OMINOUS_SPAWN_START_ARGB: u32 = 0xFF45_AEFE;
const OMINOUS_SPAWN_END_ARGB: u32 = 0xFFFF_FFFF;
const FALLING_LEAVES_ACCELERATION_SCALE: f64 = 0.0025;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleChildSpawnTemplate {
    pub particle_type_id: i32,
    pub particle_id: String,
    #[serde(default)]
    pub sprite_ids: Vec<String>,
}

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
    #[serde(default)]
    pub initial_delay_ticks: u32,
    #[serde(default)]
    pub child_spawn_templates: Vec<ParticleChildSpawnTemplate>,
    #[serde(default)]
    pub option_color: Option<[f32; 4]>,
    #[serde(default)]
    pub option_color_to: Option<[f32; 4]>,
    #[serde(default)]
    pub option_scale: Option<f32>,
    #[serde(default)]
    pub option_power: Option<f32>,
    #[serde(default)]
    pub option_target: Option<[f64; 3]>,
    #[serde(default)]
    pub option_duration_ticks: Option<u32>,
    #[serde(default)]
    pub option_roll: Option<f32>,
    #[serde(default)]
    pub option_block: Option<ParticleBlockOptionState>,
    #[serde(default)]
    pub option_item: Option<ParticleItemOptionState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleBlockOptionState {
    pub block_state_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleItemOptionState {
    pub item_id: i32,
    pub count: i32,
    pub component_patch_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct ParticleAtlasUvSubRect {
    pub(crate) u_offset: f32,
    pub(crate) v_offset: f32,
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
    limited_particle_drops: u64,
    limited_particle_counts: BTreeMap<ParticleLimitDescriptor, usize>,
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
    #[serde(default)]
    pub(crate) start_position: [f64; 3],
    pub(crate) previous_position: [f64; 3],
    pub(crate) position: [f64; 3],
    pub(crate) velocity: [f64; 3],
    pub(crate) age_ticks: u32,
    pub(crate) lifetime_ticks: u32,
    #[serde(default)]
    pub(crate) previous_roll: f32,
    #[serde(default)]
    pub(crate) roll: f32,
    #[serde(default)]
    pub(crate) roll_speed: f32,
    #[serde(default)]
    pub(crate) previous_yaw: f32,
    #[serde(default)]
    pub(crate) yaw: f32,
    #[serde(default)]
    pub(crate) previous_pitch: f32,
    #[serde(default)]
    pub(crate) pitch: f32,
    #[serde(default = "default_particle_quad_size")]
    pub(crate) base_quad_size: f32,
    #[serde(default = "default_particle_color")]
    pub(crate) color: [f32; 4],
    #[serde(default)]
    pub(crate) color_fade_target: Option<[f32; 3]>,
    #[serde(default)]
    pub(crate) color_transition_target: Option<[f32; 3]>,
    #[serde(default = "default_particle_light")]
    pub(crate) light: [f32; 2],
    #[serde(default)]
    pub(crate) light_emission: ParticleLightEmissionDescriptor,
    #[serde(default)]
    pub(crate) alpha_curve: ParticleAlphaCurve,
    #[serde(default)]
    pub(crate) quad_size_curve: ParticleQuadSizeCurve,
    pub(crate) provider: String,
    #[serde(default)]
    pub(crate) render_group: ParticleRenderGroup,
    #[serde(default)]
    pub(crate) render_layer: ParticleRenderLayer,
    #[serde(default)]
    pub(crate) texture_atlas: ParticleTextureAtlasKind,
    #[serde(default)]
    pub(crate) facing_camera_mode: ParticleFacingCameraMode,
    pub(crate) friction: f32,
    pub(crate) gravity: f32,
    pub(crate) has_physics: bool,
    pub(crate) speed_up_when_y_motion_is_blocked: bool,
    #[serde(default)]
    pub(crate) tick_motion: ParticleTickMotionDescriptor,
    #[serde(default)]
    pub(crate) tick_angle: f32,
    #[serde(default)]
    pub(crate) particle_limit: Option<ParticleLimitDescriptor>,
    #[serde(default)]
    pub(crate) child_emission: Option<ParticleChildEmissionDescriptor>,
    #[serde(default)]
    pub(crate) child_spawn_templates: Vec<ParticleChildSpawnTemplate>,
    #[serde(default)]
    falling_leaves_motion: Option<FallingLeavesRuntimeState>,
    pub(crate) sprite_selection: ParticleSpriteSelection,
    pub(crate) override_limiter: bool,
    pub(crate) always_show: bool,
    pub(crate) raw_options_len: usize,
    #[serde(default)]
    pub(crate) delay_ticks: u32,
    #[serde(default)]
    pub(crate) option_color: Option<[f32; 4]>,
    #[serde(default)]
    pub(crate) option_color_to: Option<[f32; 4]>,
    #[serde(default)]
    pub(crate) option_scale: Option<f32>,
    #[serde(default)]
    pub(crate) option_power: Option<f32>,
    #[serde(default)]
    pub(crate) option_target: Option<[f64; 3]>,
    #[serde(default)]
    pub(crate) option_duration_ticks: Option<u32>,
    #[serde(default)]
    pub(crate) option_roll: Option<f32>,
    #[serde(default)]
    pub(crate) option_block: Option<ParticleBlockOptionState>,
    #[serde(default)]
    pub(crate) option_item: Option<ParticleItemOptionState>,
    #[serde(default)]
    pub(crate) atlas_uv_sub_rect: Option<ParticleAtlasUvSubRect>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleRenderGroup {
    #[default]
    SingleQuads,
    ItemPickup,
    ElderGuardians,
    NoRender,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleRenderLayer {
    OpaqueTerrain,
    TranslucentTerrain,
    OpaqueItems,
    TranslucentItems,
    #[default]
    Opaque,
    Translucent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ParticleTextureAtlasKind {
    #[default]
    Particles,
    Terrain,
    Items,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
struct FallingLeavesRuntimeState {
    rot_speed: f32,
    spin_acceleration: f32,
    wind_big: f32,
    swirl: bool,
    flow_away: bool,
    xa_flow_scale: f64,
    za_flow_scale: f64,
    swirl_period: f64,
}

impl ParticleRenderGroup {
    fn vanilla_order(self) -> u8 {
        match self {
            Self::SingleQuads => 0,
            Self::ItemPickup => 1,
            Self::ElderGuardians => 2,
            Self::NoRender => 3,
        }
    }
}

impl ParticleRenderLayer {
    fn vanilla_solid_translucent_order(self) -> u8 {
        match self {
            Self::OpaqueTerrain => 0,
            Self::OpaqueItems => 1,
            Self::Opaque => 2,
            Self::TranslucentTerrain => 3,
            Self::TranslucentItems => 4,
            Self::Translucent => 5,
        }
    }

    fn pipeline_kind(self) -> ParticlePipelineKind {
        match self {
            Self::OpaqueTerrain | Self::OpaqueItems | Self::Opaque => ParticlePipelineKind::Opaque,
            Self::TranslucentTerrain | Self::TranslucentItems | Self::Translucent => {
                ParticlePipelineKind::Translucent
            }
        }
    }

    fn texture_atlas_kind(self) -> ParticleTextureAtlasKind {
        match self {
            Self::OpaqueTerrain | Self::TranslucentTerrain => ParticleTextureAtlasKind::Terrain,
            Self::OpaqueItems | Self::TranslucentItems => ParticleTextureAtlasKind::Items,
            Self::Opaque | Self::Translucent => ParticleTextureAtlasKind::Particles,
        }
    }
}

impl FallingLeavesRuntimeState {
    fn sample_angles(settings: FallingLeavesDescriptor, random: &mut ParticleRandom) -> Self {
        let rot_speed = (if random.next_bool() {
            -30.0_f32
        } else {
            30.0_f32
        })
        .to_radians();
        let spin_acceleration = (if random.next_bool() {
            -5.0_f32
        } else {
            5.0_f32
        })
        .to_radians();
        Self {
            rot_speed,
            spin_acceleration,
            wind_big: settings.side_acceleration,
            swirl: settings.swirl,
            flow_away: settings.flow_away,
            xa_flow_scale: 0.0,
            za_flow_scale: 0.0,
            swirl_period: 0.0,
        }
    }

    fn sample_flow(&mut self, settings: FallingLeavesDescriptor, random: &mut ParticleRandom) {
        let particle_random = random.next_f32();
        let flow_angle = f64::from((particle_random * 60.0).to_radians());
        self.xa_flow_scale = flow_angle.cos() * f64::from(settings.side_acceleration);
        self.za_flow_scale = flow_angle.sin() * f64::from(settings.side_acceleration);
        self.swirl_period = f64::from((1000.0 + particle_random * 3000.0).to_radians());
    }
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
    pub(crate) limited_particle_drops: usize,
    pub(crate) total_limited_particle_drops: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ParticleVertexBatches {
    pub(crate) opaque: ParticlePipelineVertexBatch,
    pub(crate) translucent: ParticlePipelineVertexBatch,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ParticlePipelineVertexBatch {
    pub(crate) vertices: Vec<ParticleVertex>,
    pub(crate) draws: Vec<ParticleAtlasDrawRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParticleAtlasDrawRange {
    pub(crate) texture_atlas: ParticleTextureAtlasKind,
    pub(crate) vertex_start: u32,
    pub(crate) vertex_count: u32,
}

impl ParticleAtlasDrawRange {
    pub(crate) fn vertex_end(self) -> u32 {
        self.vertex_start.saturating_add(self.vertex_count)
    }
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

fn default_particle_light() -> [f32; 2] {
    DEFAULT_PARTICLE_LIGHT
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
            limited_particle_drops: 0,
            limited_particle_counts: BTreeMap::new(),
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
        let dropped_spawns_before = self.dropped_spawns;

        for command in batch.commands {
            if self.queue_pending_spawn(command) {
                queued_spawns += 1;
            }
        }

        let dropped_spawns = self.dropped_spawns.saturating_sub(dropped_spawns_before) as usize;

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

    fn queue_pending_spawn(&mut self, command: ParticleSpawnCommand) -> bool {
        if self.max_pending_spawns == 0 {
            self.dropped_spawns = self.dropped_spawns.saturating_add(1);
            return false;
        }
        if self.pending_spawns.len() == self.max_pending_spawns {
            self.pending_spawns.pop_front();
            self.dropped_spawns = self.dropped_spawns.saturating_add(1);
        }
        self.pending_spawns.push_back(command);
        true
    }

    pub(crate) fn advance(&mut self, ticks: u32) -> ParticleAdvanceSummary {
        let mut intaken_instances = 0;
        let mut expired_instances = 0;
        let mut dropped_active_instances = 0;
        let mut limited_particle_drops = 0;

        if ticks == 0 {
            self.drain_pending_spawns(
                &mut intaken_instances,
                &mut dropped_active_instances,
                &mut limited_particle_drops,
            );
        } else {
            for _ in 0..ticks {
                expired_instances += self.tick_active_instances();
                self.drain_pending_spawns(
                    &mut intaken_instances,
                    &mut dropped_active_instances,
                    &mut limited_particle_drops,
                );
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
        self.limited_particle_drops = self
            .limited_particle_drops
            .saturating_add(limited_particle_drops as u64);

        ParticleAdvanceSummary {
            ticks,
            intaken_instances,
            expired_instances,
            dropped_active_instances,
            limited_particle_drops,
            pending_spawns: self.pending_spawns.len(),
            active_instances: self.active_instances.len(),
            total_instances_created: self.instances_created,
            total_instances_expired: self.instances_expired,
            total_dropped_active_instances: self.dropped_active_instances,
            total_limited_particle_drops: self.limited_particle_drops,
        }
    }

    fn tick_active_instances(&mut self) -> usize {
        let mut expired_instances = 0;
        let mut active_instances = VecDeque::with_capacity(self.active_instances.len());
        while let Some(mut instance) = self.active_instances.pop_front() {
            if instance.delay_ticks > 0 {
                instance.delay_ticks = instance.delay_ticks.saturating_sub(1);
                active_instances.push_back(instance);
                continue;
            }
            if instance.age_ticks >= instance.lifetime_ticks {
                self.decrement_particle_limit(instance.particle_limit);
                expired_instances += 1;
                continue;
            }
            instance.tick_motion_without_collision(&mut self.random);
            instance.age_ticks = instance.age_ticks.saturating_add(1);
            instance.update_sprite_from_age();
            instance.update_alpha_from_age();
            instance.update_color_fade_from_age();
            self.enqueue_child_spawns_from(&instance);
            active_instances.push_back(instance);
        }
        self.active_instances = active_instances;
        expired_instances
    }

    fn enqueue_child_spawns_from(&mut self, instance: &ParticleInstance) {
        for command in instance.child_spawn_commands(&mut self.random) {
            self.queue_pending_spawn(command);
        }
    }

    fn drain_pending_spawns(
        &mut self,
        intaken_instances: &mut usize,
        dropped_active_instances: &mut usize,
        limited_particle_drops: &mut usize,
    ) {
        while let Some(command) = self.pending_spawns.pop_front() {
            if self.max_active_instances == 0 {
                *dropped_active_instances += 1;
                continue;
            }
            let instance = ParticleInstance::from_spawn_command(command, &mut self.random);
            if !self.has_space_in_particle_limit(instance.particle_limit) {
                *limited_particle_drops += 1;
                continue;
            }
            if self.active_instances.len() == self.max_active_instances {
                if let Some(evicted) = self.active_instances.pop_front() {
                    self.decrement_particle_limit(evicted.particle_limit);
                }
                *dropped_active_instances += 1;
            }
            self.increment_particle_limit(instance.particle_limit);
            self.active_instances.push_back(instance);
            *intaken_instances += 1;
        }
    }

    fn has_space_in_particle_limit(&self, limit: Option<ParticleLimitDescriptor>) -> bool {
        let Some(limit) = limit else {
            return true;
        };
        self.limited_particle_counts
            .get(&limit)
            .copied()
            .unwrap_or_default()
            < limit.limit()
    }

    fn increment_particle_limit(&mut self, limit: Option<ParticleLimitDescriptor>) {
        if let Some(limit) = limit {
            *self.limited_particle_counts.entry(limit).or_default() += 1;
        }
    }

    fn decrement_particle_limit(&mut self, limit: Option<ParticleLimitDescriptor>) {
        if let Some(limit) = limit {
            let Some(count) = self.limited_particle_counts.get_mut(&limit) else {
                return;
            };
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.limited_particle_counts.remove(&limit);
            }
        }
    }

    fn refresh_lights<F>(&mut self, mut light_at_position: F)
    where
        F: FnMut([f64; 3]) -> [f32; 2],
    {
        for instance in &mut self.active_instances {
            let sampled_light = sanitize_particle_light(light_at_position(instance.position));
            instance.light = particle_light_with_emission(instance, sampled_light);
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
        let child_emission = descriptor.child_emission();
        let particle_limit = particle_limit_for_particle(&command.particle_id);
        let render_group = if (descriptor.provider == "VibrationSignalParticle.Provider"
            && command.option_target.is_none())
            || matches!(
                child_emission,
                Some(
                    ParticleChildEmissionDescriptor::HugeExplosionSeed
                        | ParticleChildEmissionDescriptor::GustSeed { .. }
                )
            ) {
            ParticleRenderGroup::NoRender
        } else {
            particle_render_group_for_particle(&command.particle_id)
        };
        let render_layer = particle_render_layer_for_particle(&command.particle_id);
        let texture_atlas = render_layer.texture_atlas_kind();
        let mut position = descriptor.initial_position(command.position, random);
        let mut velocity = descriptor.initial_velocity.sample(command.velocity, random);
        if descriptor.provider == "SpellParticle.InstantProvider" {
            if let Some(power) = command.option_power {
                velocity = apply_particle_power(velocity, power);
            }
        }
        let starts_at_velocity_position = matches!(
            descriptor.provider,
            "FlyTowardsPositionParticle.EnchantProvider"
                | "FlyTowardsPositionParticle.NautilusProvider"
                | "FlyTowardsPositionParticle.VaultConnectionProvider"
                | "FlyStraightTowardsParticle.OminousSpawnProvider"
        );
        if starts_at_velocity_position {
            position = [
                command.position[0] + velocity[0],
                command.position[1] + velocity[1],
                command.position[2] + velocity[2],
            ];
        }
        let start_position = if starts_at_velocity_position {
            command.position
        } else {
            position
        };
        let (current_sprite_index, current_sprite_id) =
            select_initial_sprite(&command.sprite_ids, descriptor.sprite_selection, random);
        let falling_leaves = descriptor.falling_leaves();
        let mut falling_leaves_motion = falling_leaves
            .map(|settings| FallingLeavesRuntimeState::sample_angles(settings, random));
        let mut visual = descriptor
            .visual
            .sample_for_command(random, command.velocity);
        if let (Some(settings), Some(motion)) = (falling_leaves, falling_leaves_motion.as_mut()) {
            motion.sample_flow(settings, random);
        }
        let option_scale = command.option_scale.map(clamp_particle_option_scale);
        if matches!(
            descriptor.provider,
            "DustParticle.Provider" | "DustColorTransitionParticle.Provider"
        ) {
            visual.base_quad_size *= option_scale.unwrap_or(1.0);
        }
        let mut color = if descriptor.provider == "FallingLeavesParticle.TintedLeavesProvider" {
            command.option_color.map_or(visual.color, |option_color| {
                [
                    option_color[0],
                    option_color[1],
                    option_color[2],
                    visual.color[3],
                ]
            })
        } else {
            command.option_color.unwrap_or(visual.color)
        };
        let mut color_transition_target = None;
        let mut sampled_lifetime_ticks = None;
        if matches!(
            descriptor.provider,
            "DustParticle.Provider" | "DustColorTransitionParticle.Provider"
        ) {
            let scale = option_scale.unwrap_or(1.0);
            sampled_lifetime_ticks = Some(dust_lifetime(random, scale));
            let base_factor = random.next_f32() * 0.4 + 0.6;
            color = dust_particle_color(color, base_factor, random);
            if descriptor.provider == "DustColorTransitionParticle.Provider" {
                let to_color = command.option_color_to.unwrap_or(color);
                let to_color = dust_particle_color(to_color, base_factor, random);
                color_transition_target = Some([to_color[0], to_color[1], to_color[2]]);
            }
        }
        if descriptor.provider == "TrailParticle.Provider" {
            if let Some(option_color) = command.option_color {
                color = trail_particle_color(option_color, random);
            }
        }
        let lifetime_ticks = if let Some(lifetime_ticks) = sampled_lifetime_ticks {
            lifetime_ticks
        } else {
            match descriptor.lifetime {
                descriptors::ParticleLifetimeDescriptor::CommandOption { .. } => command
                    .option_duration_ticks
                    .unwrap_or_else(|| descriptor.lifetime.sample(random)),
                _ => descriptor.lifetime.sample(random),
            }
        };
        let (previous_roll, roll, roll_speed) = match descriptor.provider {
            "SculkChargeParticle.Provider" => {
                let roll = command.option_roll.unwrap_or(0.0);
                (roll, roll, 0.0)
            }
            "FallingDustParticle.Provider" => {
                let roll_speed = (random.next_f32() - 0.5) * 0.1;
                let roll = random.next_f32() * std::f32::consts::PI * 2.0;
                (roll, roll, roll_speed)
            }
            _ => (0.0, 0.0, 0.0),
        };
        let (previous_yaw, yaw, previous_pitch, pitch) =
            if descriptor.provider == "VibrationSignalParticle.Provider" {
                command
                    .option_target
                    .map(|target| {
                        let (yaw, pitch) = vibration_particle_angles(position, target);
                        (yaw, yaw, pitch, pitch)
                    })
                    .unwrap_or((0.0, 0.0, 0.0, 0.0))
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
        let atlas_uv_sub_rect =
            particle_atlas_uv_sub_rect_for_particle(&command.particle_id, random);
        Self {
            particle_type_id: command.particle_type_id,
            particle_id: command.particle_id,
            sprite_ids: command.sprite_ids,
            current_sprite_id,
            current_sprite_index,
            start_position,
            previous_position: position,
            position,
            velocity,
            age_ticks: 0,
            lifetime_ticks,
            previous_roll,
            roll,
            roll_speed,
            previous_yaw,
            yaw,
            previous_pitch,
            pitch,
            base_quad_size: visual.base_quad_size,
            color,
            color_fade_target: descriptor.color_fade_target(),
            color_transition_target,
            light: DEFAULT_PARTICLE_LIGHT,
            light_emission: descriptor.light_emission(),
            alpha_curve: descriptor.alpha_curve(),
            quad_size_curve: visual.quad_size_curve,
            provider: descriptor.provider.to_string(),
            render_group,
            render_layer,
            texture_atlas,
            facing_camera_mode: descriptor.facing_camera_mode(),
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            has_physics: descriptor.has_physics,
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            tick_motion: descriptor.tick_motion(),
            tick_angle: 0.0,
            particle_limit,
            child_emission,
            child_spawn_templates: command.child_spawn_templates,
            falling_leaves_motion,
            sprite_selection: descriptor.sprite_selection,
            override_limiter: command.override_limiter,
            always_show: command.always_show,
            raw_options_len: command.raw_options_len,
            delay_ticks: command.initial_delay_ticks,
            option_color: command.option_color,
            option_color_to: command.option_color_to,
            option_scale,
            option_power: command.option_power,
            option_target: command.option_target,
            option_duration_ticks: command.option_duration_ticks,
            option_roll: command.option_roll,
            option_block: command.option_block,
            option_item: command.option_item,
            atlas_uv_sub_rect,
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
            ParticleQuadSizeCurve::Lava => {
                self.base_quad_size * (1.0 - progress * progress).max(0.0)
            }
            ParticleQuadSizeCurve::FlashOverlay => {
                7.1 * ((age - 1.0) * 0.25 * std::f32::consts::PI).sin()
            }
            ParticleQuadSizeCurve::Portal => {
                self.base_quad_size * (1.0 - (1.0 - progress) * (1.0 - progress))
            }
            ParticleQuadSizeCurve::ReversePortal => {
                self.base_quad_size * (1.0 - progress / 1.5).max(0.0)
            }
            ParticleQuadSizeCurve::Shriek => {
                self.base_quad_size * (progress * 0.75).clamp(0.0, 1.0)
            }
        }
    }

    fn tick_motion_without_collision(&mut self, random: &mut ParticleRandom) {
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
            ParticleTickMotionDescriptor::CurrentDown => {
                let angle = f64::from(self.tick_angle);
                self.velocity[0] = (self.velocity[0] + 0.6 * angle.cos()) * 0.07;
                self.velocity[2] = (self.velocity[2] + 0.6 * angle.sin()) * 0.07;
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                self.tick_angle += 0.08;
            }
            ParticleTickMotionDescriptor::Snowflake => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction * 0.95;
                self.velocity[1] *= friction * 0.9;
                self.velocity[2] *= friction * 0.95;
            }
            ParticleTickMotionDescriptor::FlyTowardsPosition => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let pos = 1.0 - (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let pp = (1.0 - pos).powi(4);
                self.position[0] = self.start_position[0] + self.velocity[0] * f64::from(pos);
                self.position[1] = self.start_position[1] + self.velocity[1] * f64::from(pos)
                    - f64::from(pp * 1.2);
                self.position[2] = self.start_position[2] + self.velocity[2] * f64::from(pos);
            }
            ParticleTickMotionDescriptor::TrailTarget => {
                let Some(target) = self.option_target else {
                    return;
                };
                let next_age = self.age_ticks.saturating_add(1);
                let ticks_remaining = self.lifetime_ticks.saturating_sub(next_age);
                if ticks_remaining == 0 {
                    self.position = target;
                    return;
                }
                let alpha = 1.0 / f64::from(ticks_remaining);
                self.position = [
                    lerp_f64(alpha, self.position[0], target[0]),
                    lerp_f64(alpha, self.position[1], target[1]),
                    lerp_f64(alpha, self.position[2], target[2]),
                ];
            }
            ParticleTickMotionDescriptor::VibrationSignal => {
                let Some(target) = self.option_target else {
                    return;
                };
                let next_age = self.age_ticks.saturating_add(1);
                let ticks_remaining = self.lifetime_ticks.saturating_sub(next_age);
                if ticks_remaining == 0 {
                    self.position = target;
                } else {
                    let alpha = 1.0 / f64::from(ticks_remaining);
                    self.position = [
                        lerp_f64(alpha, self.position[0], target[0]),
                        lerp_f64(alpha, self.position[1], target[1]),
                        lerp_f64(alpha, self.position[2], target[2]),
                    ];
                }
                let (yaw, pitch) = vibration_particle_angles(self.position, target);
                self.previous_yaw = self.yaw;
                self.yaw = yaw;
                self.previous_pitch = self.pitch;
                self.pitch = pitch;
            }
            ParticleTickMotionDescriptor::FlyStraightTowards => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let normalized_age = (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let pos_alpha = 1.0 - normalized_age;
                self.position[0] = self.start_position[0] + self.velocity[0] * f64::from(pos_alpha);
                self.position[1] = self.start_position[1] + self.velocity[1] * f64::from(pos_alpha);
                self.position[2] = self.start_position[2] + self.velocity[2] * f64::from(pos_alpha);
                self.color = argb_srgb_lerp_color(
                    normalized_age,
                    OMINOUS_SPAWN_START_ARGB,
                    OMINOUS_SPAWN_END_ARGB,
                );
            }
            ParticleTickMotionDescriptor::CampfireSmoke => {
                self.velocity[0] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[2] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let next_age = self.age_ticks.saturating_add(1);
                if next_age >= self.lifetime_ticks.saturating_sub(60) && self.color[3] > 0.01 {
                    self.color[3] -= 0.015;
                }
            }
            ParticleTickMotionDescriptor::DripHang => {
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= 0.02 * friction;
                self.velocity[1] *= 0.02 * friction;
                self.velocity[2] *= 0.02 * friction;
            }
            ParticleTickMotionDescriptor::CoolingDripHang => {
                let cooling_age = self.age_ticks as f32;
                self.color[0] = 1.0;
                self.color[1] = 16.0 / (cooling_age + 16.0);
                self.color[2] = 4.0 / (cooling_age + 8.0);
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= 0.02 * friction;
                self.velocity[1] *= 0.02 * friction;
                self.velocity[2] *= 0.02 * friction;
            }
            ParticleTickMotionDescriptor::DustPlume => {
                self.gravity *= 0.88;
                self.friction *= 0.92;
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
            }
            ParticleTickMotionDescriptor::WaterDrop => {
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
            }
            ParticleTickMotionDescriptor::Wake => {
                let life =
                    60_u32.saturating_sub(self.lifetime_ticks.saturating_sub(self.age_ticks));
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if let Some(index) = sprite_index_for_age(self.sprite_ids.len(), life % 4, 4) {
                    self.current_sprite_index = Some(index);
                    self.current_sprite_id = self.sprite_ids.get(index).cloned();
                }
            }
            ParticleTickMotionDescriptor::Portal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let progress = (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let position_scale = 1.0 - (-progress + progress * progress * 2.0);
                self.previous_position = self.position;
                self.position[0] =
                    self.start_position[0] + self.velocity[0] * f64::from(position_scale);
                self.position[1] = self.start_position[1]
                    + self.velocity[1] * f64::from(position_scale)
                    + f64::from(1.0 - progress);
                self.position[2] =
                    self.start_position[2] + self.velocity[2] * f64::from(position_scale);
            }
            ParticleTickMotionDescriptor::ReversePortal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let speed_multiplier = f64::from((next_age as f32 / lifetime).clamp(0.0, 1.0));
                self.previous_position = self.position;
                self.position[0] += self.velocity[0] * speed_multiplier;
                self.position[1] += self.velocity[1] * speed_multiplier;
                self.position[2] += self.velocity[2] * speed_multiplier;
            }
            ParticleTickMotionDescriptor::Firefly => {
                let next_age = self.age_ticks.saturating_add(1);
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;

                if random.next_f32() > 0.95 || next_age == 1 {
                    self.velocity = [
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                    ];
                }
            }
            ParticleTickMotionDescriptor::FallingLeaves => {
                self.tick_falling_leaves_without_collision();
            }
            ParticleTickMotionDescriptor::FallingDust => {
                self.previous_roll = self.roll;
                self.roll += std::f32::consts::PI * self.roll_speed * 2.0;
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                self.velocity[1] = (self.velocity[1] - 0.003).max(-0.14);
            }
        }
    }

    fn tick_falling_leaves_without_collision(&mut self) {
        let Some(motion) = self.falling_leaves_motion.as_mut() else {
            return;
        };
        let alive_ticks = self.age_ticks.saturating_add(1);
        let relative_age = (alive_ticks as f64 / 300.0).min(1.0);
        let mut xa = 0.0;
        let mut za = 0.0;
        if motion.flow_away {
            let flow = relative_age.powf(1.25);
            xa += motion.xa_flow_scale * flow;
            za += motion.za_flow_scale * flow;
        }
        if motion.swirl {
            xa += relative_age
                * (relative_age * motion.swirl_period).cos()
                * f64::from(motion.wind_big);
            za += relative_age
                * (relative_age * motion.swirl_period).sin()
                * f64::from(motion.wind_big);
        }

        self.velocity[0] += xa * FALLING_LEAVES_ACCELERATION_SCALE;
        self.velocity[2] += za * FALLING_LEAVES_ACCELERATION_SCALE;
        self.velocity[1] -= f64::from(self.gravity);
        motion.rot_speed += motion.spin_acceleration / 20.0;
        self.previous_roll = self.roll;
        self.roll += motion.rot_speed / 20.0;
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

    fn update_alpha_from_age(&mut self) {
        match self.alpha_curve {
            ParticleAlphaCurve::Constant => {}
            ParticleAlphaCurve::SimpleAnimatedFade => {
                self.color[3] = simple_animated_alpha(self.age_ticks, self.lifetime_ticks);
            }
            ParticleAlphaCurve::FlashOverlayFade => {}
            ParticleAlphaCurve::FireworkSparkFade => {
                self.color[3] = firework_spark_alpha(self.age_ticks, self.lifetime_ticks);
            }
            ParticleAlphaCurve::ShriekFade => {
                let lifetime = self.lifetime_ticks.max(1) as f32;
                self.color[3] = 1.0 - (self.age_ticks as f32 / lifetime).clamp(0.0, 1.0);
            }
            ParticleAlphaCurve::VaultConnectionFade => {
                self.color[3] = vault_connection_alpha(self.age_ticks, self.lifetime_ticks, 0.0);
            }
            ParticleAlphaCurve::FireflyFade => {
                let progress = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
                self.color[3] = firefly_fade_amount(progress, 0.3, 0.5);
            }
        }
    }

    fn update_color_fade_from_age(&mut self) {
        let Some(target) = self.color_fade_target else {
            return;
        };
        if self.age_ticks <= self.lifetime_ticks / 2 {
            return;
        }
        self.color[0] += (target[0] - self.color[0]) * 0.2;
        self.color[1] += (target[1] - self.color[1]) * 0.2;
        self.color[2] += (target[2] - self.color[2]) * 0.2;
    }

    fn child_spawn_commands(&self, random: &mut ParticleRandom) -> Vec<ParticleSpawnCommand> {
        match self.child_emission {
            Some(ParticleChildEmissionDescriptor::LavaSmoke) => self
                .lava_child_smoke_spawn_command(random)
                .into_iter()
                .collect(),
            Some(ParticleChildEmissionDescriptor::HugeExplosionSeed) => {
                self.huge_explosion_seed_child_spawn_commands(random)
            }
            Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths,
                vanilla_lifetime,
                tick_delay,
            }) => self.gust_seed_child_spawn_commands(
                random,
                scale_tenths,
                vanilla_lifetime,
                tick_delay,
            ),
            None => Vec::new(),
        }
    }

    fn lava_child_smoke_spawn_command(
        &self,
        random: &mut ParticleRandom,
    ) -> Option<ParticleSpawnCommand> {
        let template = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == LAVA_CHILD_SMOKE_PARTICLE_ID)?;
        let odds = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
        if random.next_f32() <= odds {
            return None;
        }
        Some(ParticleSpawnCommand {
            particle_type_id: template.particle_type_id,
            particle_id: template.particle_id.clone(),
            sprite_ids: template.sprite_ids.clone(),
            position: self.position,
            velocity: self.velocity,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: Vec::new(),
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
        })
    }

    fn huge_explosion_seed_child_spawn_commands(
        &self,
        random: &mut ParticleRandom,
    ) -> Vec<ParticleSpawnCommand> {
        let Some(template) = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == HUGE_EXPLOSION_CHILD_PARTICLE_ID)
        else {
            return Vec::new();
        };
        let vanilla_age = self.age_ticks.saturating_sub(1);
        let velocity = [
            f64::from(vanilla_age) / f64::from(self.lifetime_ticks.max(1)),
            0.0,
            0.0,
        ];
        (0..6)
            .map(|_| {
                let position = [
                    self.position[0] + (random.next_double() - random.next_double()) * 4.0,
                    self.position[1] + (random.next_double() - random.next_double()) * 4.0,
                    self.position[2] + (random.next_double() - random.next_double()) * 4.0,
                ];
                ParticleSpawnCommand {
                    particle_type_id: template.particle_type_id,
                    particle_id: template.particle_id.clone(),
                    sprite_ids: template.sprite_ids.clone(),
                    position,
                    velocity,
                    override_limiter: false,
                    always_show: false,
                    raw_options_len: 0,
                    initial_delay_ticks: 0,
                    child_spawn_templates: Vec::new(),
                    option_color: None,
                    option_color_to: None,
                    option_scale: None,
                    option_power: None,
                    option_target: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                }
            })
            .collect()
    }

    fn gust_seed_child_spawn_commands(
        &self,
        random: &mut ParticleRandom,
        scale_tenths: u32,
        vanilla_lifetime: u32,
        tick_delay: u32,
    ) -> Vec<ParticleSpawnCommand> {
        let vanilla_age = self.age_ticks.saturating_sub(1);
        if vanilla_age % tick_delay.saturating_add(1) != 0 {
            return Vec::new();
        }
        let Some(template) = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == GUST_CHILD_PARTICLE_ID)
        else {
            return Vec::new();
        };
        let scale = f64::from(scale_tenths) / 10.0;
        let velocity = [
            f64::from(vanilla_age) / f64::from(vanilla_lifetime.max(1)),
            0.0,
            0.0,
        ];
        (0..3)
            .map(|_| {
                let position = [
                    self.position[0] + (random.next_double() - random.next_double()) * scale,
                    self.position[1] + (random.next_double() - random.next_double()) * scale,
                    self.position[2] + (random.next_double() - random.next_double()) * scale,
                ];
                ParticleSpawnCommand {
                    particle_type_id: template.particle_type_id,
                    particle_id: template.particle_id.clone(),
                    sprite_ids: template.sprite_ids.clone(),
                    position,
                    velocity,
                    override_limiter: false,
                    always_show: false,
                    raw_options_len: 0,
                    initial_delay_ticks: 0,
                    child_spawn_templates: Vec::new(),
                    option_color: None,
                    option_color_to: None,
                    option_scale: None,
                    option_power: None,
                    option_target: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                }
            })
            .collect()
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

    pub fn set_terrain_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        self.terrain_particle_sprite_uvs = particle_sprite_uv_map(sprite_uvs);
    }

    pub fn set_item_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        self.item_particle_sprite_uvs = particle_sprite_uv_map(sprite_uvs);
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
        self.counters.last_particle_limited_drop_count = summary.limited_particle_drops;
        self.counters.particle_runtime_ticks = self
            .counters
            .particle_runtime_ticks
            .saturating_add(summary.ticks as u64);
        self.counters.particle_instances_created = summary.total_instances_created;
        self.counters.particle_instances_expired = summary.total_instances_expired;
        self.counters.dropped_active_particle_instances = summary.total_dropped_active_instances;
        self.counters.dropped_limited_particle_instances = summary.total_limited_particle_drops;
    }

    pub fn refresh_particle_lights<F>(&mut self, mut light_at_position: F)
    where
        F: FnMut([f64; 3]) -> [f32; 2],
    {
        self.particles
            .refresh_lights(|position| light_at_position(position));
    }

    pub(crate) fn collect_particle_vertex_batches(&self) -> ParticleVertexBatches {
        let Some(pose) = self.camera_pose else {
            return ParticleVertexBatches::default();
        };
        let particle_sprite_uvs = self.particle_atlas.as_ref().map(|atlas| &atlas.sprite_uvs);
        let item_sprite_uvs = self
            .item_entity_atlas
            .as_ref()
            .map(|_| &self.item_particle_sprite_uvs);
        let atlas_uvs = ParticleAtlasUvSets {
            particles: particle_sprite_uvs,
            terrain: Some(&self.terrain_particle_sprite_uvs),
            items: item_sprite_uvs,
        };
        let axes = camera_billboard_axes(pose);
        ParticleVertexBatches {
            opaque: particle_pipeline_vertex_batch(
                self.particles.active_instances.iter(),
                atlas_uvs,
                axes,
                ParticlePipelineKind::Opaque,
            ),
            translucent: particle_pipeline_vertex_batch(
                self.particles.active_instances.iter(),
                atlas_uvs,
                axes,
                ParticlePipelineKind::Translucent,
            ),
        }
    }
}

fn particle_sprite_uv_map(sprite_uvs: Vec<ParticleSpriteUv>) -> BTreeMap<String, ParticleUvRect> {
    sprite_uvs
        .into_iter()
        .map(|sprite| (sprite.id, sprite.uv))
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ParticleAtlasUvSets<'a> {
    particles: Option<&'a BTreeMap<String, ParticleUvRect>>,
    terrain: Option<&'a BTreeMap<String, ParticleUvRect>>,
    items: Option<&'a BTreeMap<String, ParticleUvRect>>,
}

impl<'a> ParticleAtlasUvSets<'a> {
    fn for_texture_atlas(
        self,
        texture_atlas: ParticleTextureAtlasKind,
    ) -> Option<&'a BTreeMap<String, ParticleUvRect>> {
        match texture_atlas {
            ParticleTextureAtlasKind::Particles => self.particles,
            ParticleTextureAtlasKind::Terrain => self.terrain,
            ParticleTextureAtlasKind::Items => self.items,
        }
    }
}

fn particle_pipeline_vertex_batch<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    atlas_uvs: ParticleAtlasUvSets<'_>,
    axes: ParticleBillboardAxes,
    pipeline_kind: ParticlePipelineKind,
) -> ParticlePipelineVertexBatch {
    let mut batch = ParticlePipelineVertexBatch::default();
    let mut current_draw_start = 0_u32;
    let mut current_texture_atlas = None;
    let mut instances: Vec<_> = instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::SingleQuads)
        .filter(|instance| instance.delay_ticks == 0)
        .filter(|instance| instance.render_layer.pipeline_kind() == pipeline_kind)
        .collect();
    instances.sort_by_key(|instance| {
        (
            instance.render_group.vanilla_order(),
            instance.render_layer.vanilla_solid_translucent_order(),
        )
    });
    for instance in instances {
        let Some(sprite_id) = instance.current_sprite_id.as_deref() else {
            continue;
        };
        let Some(sprite_uvs) = atlas_uvs.for_texture_atlas(instance.texture_atlas) else {
            continue;
        };
        let Some(uv) = sprite_uvs.get(sprite_id).copied() else {
            continue;
        };
        if current_texture_atlas != Some(instance.texture_atlas) {
            push_particle_atlas_draw_range(&mut batch, current_texture_atlas, current_draw_start);
            current_texture_atlas = Some(instance.texture_atlas);
            current_draw_start = batch.vertices.len() as u32;
        }
        let uv = particle_uv_rect_for_instance(instance, uv);
        append_particle_instance_vertices(&mut batch.vertices, instance, uv, axes);
    }
    push_particle_atlas_draw_range(&mut batch, current_texture_atlas, current_draw_start);
    batch
}

fn push_particle_atlas_draw_range(
    batch: &mut ParticlePipelineVertexBatch,
    texture_atlas: Option<ParticleTextureAtlasKind>,
    vertex_start: u32,
) {
    let Some(texture_atlas) = texture_atlas else {
        return;
    };
    let vertex_end = batch.vertices.len() as u32;
    if vertex_end <= vertex_start {
        return;
    }
    batch.draws.push(ParticleAtlasDrawRange {
        texture_atlas,
        vertex_start,
        vertex_count: vertex_end - vertex_start,
    });
}

#[cfg(test)]
fn particle_billboard_vertices<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    sprite_uvs: &BTreeMap<String, ParticleUvRect>,
    axes: ParticleBillboardAxes,
    pipeline_kind: Option<ParticlePipelineKind>,
) -> Vec<ParticleVertex> {
    let mut vertices = Vec::new();
    let mut instances: Vec<_> = instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::SingleQuads)
        .filter(|instance| instance.delay_ticks == 0)
        .filter(|instance| match pipeline_kind {
            Some(kind) => instance.render_layer.pipeline_kind() == kind,
            None => true,
        })
        .collect();
    instances.sort_by_key(|instance| {
        (
            instance.render_group.vanilla_order(),
            instance.render_layer.vanilla_solid_translucent_order(),
        )
    });
    for instance in instances {
        let Some(sprite_id) = instance.current_sprite_id.as_deref() else {
            continue;
        };
        let Some(uv) = sprite_uvs.get(sprite_id).copied() else {
            continue;
        };
        let uv = particle_uv_rect_for_instance(instance, uv);
        append_particle_instance_vertices(&mut vertices, instance, uv, axes);
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

fn particle_axes_for_instance(
    axes: ParticleBillboardAxes,
    facing_camera_mode: ParticleFacingCameraMode,
) -> ParticleBillboardAxes {
    match facing_camera_mode {
        ParticleFacingCameraMode::LookAtXyz => axes,
        ParticleFacingCameraMode::LookAtY => ParticleBillboardAxes {
            right: axes.right,
            up: Vec3::Y,
        },
    }
}

fn particle_uv_rect_for_instance(
    instance: &ParticleInstance,
    uv: ParticleUvRect,
) -> ParticleUvRect {
    let Some(sub_rect) = instance.atlas_uv_sub_rect else {
        return uv;
    };
    let u_span = uv.max[0] - uv.min[0];
    let v_span = uv.max[1] - uv.min[1];
    ParticleUvRect {
        min: [
            uv.min[0] + u_span * ((sub_rect.u_offset + 1.0) / 4.0),
            uv.min[1] + v_span * (sub_rect.v_offset / 4.0),
        ],
        max: [
            uv.min[0] + u_span * (sub_rect.u_offset / 4.0),
            uv.min[1] + v_span * ((sub_rect.v_offset + 1.0) / 4.0),
        ],
    }
}

fn append_particle_instance_vertices(
    vertices: &mut Vec<ParticleVertex>,
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    axes: ParticleBillboardAxes,
) {
    if instance.provider == "ShriekParticle.Provider" {
        for rotation in shriek_particle_rotations() {
            append_rotated_particle_quad(vertices, instance, uv, rotation);
        }
        return;
    }

    if instance.provider == "VibrationSignalParticle.Provider" {
        for rotation in vibration_particle_rotations(instance) {
            append_rotated_particle_quad(vertices, instance, uv, rotation);
        }
        return;
    }

    vertices.extend(particle_instance_vertices(
        instance,
        uv,
        particle_axes_for_instance(axes, instance.facing_camera_mode),
    ));
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
    let roll = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_roll,
        instance.roll,
    );
    let (right_axis, up_axis) = rotated_billboard_axes(axes, roll);
    let right = right_axis * half_size;
    let up = up_axis * half_size;
    let bottom_left = center - right - up;
    let bottom_right = center + right - up;
    let top_right = center + right + up;
    let top_left = center - right + up;
    let tint = particle_render_color(instance);

    [
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(bottom_right, [uv.max[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(top_left, [uv.min[0], uv.min[1]], tint, instance.light),
    ]
}

fn append_rotated_particle_quad(
    vertices: &mut Vec<ParticleVertex>,
    instance: &ParticleInstance,
    uv: ParticleUvRect,
    rotation: Quat,
) {
    let center = Vec3::new(
        instance.position[0] as f32,
        instance.position[1] as f32,
        instance.position[2] as f32,
    );
    let half_size = instance.render_quad_size() * 0.5;
    let bottom_left = center + rotation * Vec3::new(-half_size, -half_size, 0.0);
    let bottom_right = center + rotation * Vec3::new(half_size, -half_size, 0.0);
    let top_right = center + rotation * Vec3::new(half_size, half_size, 0.0);
    let top_left = center + rotation * Vec3::new(-half_size, half_size, 0.0);
    let tint = particle_render_color(instance);

    vertices.extend([
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(bottom_right, [uv.max[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(bottom_left, [uv.min[0], uv.max[1]], tint, instance.light),
        particle_vertex(top_right, [uv.max[0], uv.min[1]], tint, instance.light),
        particle_vertex(top_left, [uv.min[0], uv.min[1]], tint, instance.light),
    ]);
}

fn shriek_particle_rotations() -> [Quat; 2] {
    [
        Quat::from_rotation_x(-SHRIEK_MAGICAL_X_ROT),
        Quat::from_euler(
            EulerRot::YXZ,
            -std::f32::consts::PI,
            SHRIEK_MAGICAL_X_ROT,
            0.0,
        ),
    ]
}

fn vibration_particle_rotations(instance: &ParticleInstance) -> [Quat; 2] {
    let random_sway =
        vibration_particle_sway(instance.age_ticks, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
    let yaw = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_yaw,
        instance.yaw,
    );
    let pitch = lerp_f32(
        DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
        instance.previous_pitch,
        instance.pitch,
    ) + std::f32::consts::FRAC_PI_2;
    [
        Quat::from_rotation_y(yaw)
            * Quat::from_rotation_x(-pitch)
            * Quat::from_rotation_y(random_sway),
        Quat::from_rotation_y(-std::f32::consts::PI + yaw)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_y(random_sway),
    ]
}

fn particle_render_color(instance: &ParticleInstance) -> [f32; 4] {
    let mut color = instance.color;
    if let Some(target) = instance.color_transition_target {
        let alpha = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
            / (instance.lifetime_ticks as f32 + 1.0).max(1.0);
        color[0] = lerp_f32(alpha, color[0], target[0]);
        color[1] = lerp_f32(alpha, color[1], target[1]);
        color[2] = lerp_f32(alpha, color[2], target[2]);
    }
    match instance.alpha_curve {
        ParticleAlphaCurve::Constant => {}
        ParticleAlphaCurve::SimpleAnimatedFade => {}
        ParticleAlphaCurve::FlashOverlayFade => {
            color[3] =
                flash_overlay_alpha(instance.age_ticks, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
        }
        ParticleAlphaCurve::FireworkSparkFade => {}
        ParticleAlphaCurve::ShriekFade => {
            let lifetime = instance.lifetime_ticks.max(1) as f32;
            color[3] = 1.0
                - ((instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK) / lifetime)
                    .clamp(0.0, 1.0);
        }
        ParticleAlphaCurve::VaultConnectionFade => {
            color[3] = vault_connection_alpha(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
        }
        ParticleAlphaCurve::FireflyFade => {
            let progress = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
                / instance.lifetime_ticks.max(1) as f32;
            color[3] = firefly_fade_amount(progress, 0.3, 0.5);
        }
    }
    color
}

fn simple_animated_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        1.0
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

fn firework_spark_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        0.99
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

fn apply_particle_power(velocity: [f64; 3], power: f32) -> [f64; 3] {
    let power = f64::from(power);
    [
        velocity[0] * power,
        (velocity[1] - 0.1) * power + 0.1,
        velocity[2] * power,
    ]
}

fn trail_particle_color(color: [f32; 4], random: &mut ParticleRandom) -> [f32; 4] {
    [
        color[0] * (0.875 + random.next_f32() * 0.25),
        color[1] * (0.875 + random.next_f32() * 0.25),
        color[2] * (0.875 + random.next_f32() * 0.25),
        color[3],
    ]
}

fn random_sign(random: &mut ParticleRandom) -> f64 {
    if random.next_bool() {
        1.0
    } else {
        -1.0
    }
}

fn dust_particle_color(color: [f32; 4], base_factor: f32, random: &mut ParticleRandom) -> [f32; 4] {
    [
        (random.next_f32() * 0.2 + 0.8) * color[0] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[1] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[2] * base_factor,
        color[3],
    ]
}

fn clamp_particle_option_scale(scale: f32) -> f32 {
    scale.clamp(0.01, 4.0)
}

fn flash_overlay_alpha(age_ticks: u32, partial_tick: f32) -> f32 {
    0.6 - (age_ticks as f32 + partial_tick.clamp(0.0, 1.0) - 1.0) * 0.25 * 0.5
}

fn vibration_particle_angles(position: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = position[0] - target[0];
    let dy = position[1] - target[1];
    let dz = position[2] - target[2];
    let yaw = dx.atan2(dz) as f32;
    let pitch = dy.atan2((dx * dx + dz * dz).sqrt()) as f32;
    (yaw, pitch)
}

fn vibration_particle_sway(age_ticks: u32, partial_tick: f32) -> f32 {
    ((age_ticks as f32 + partial_tick.clamp(0.0, 1.0) - std::f32::consts::TAU) * 0.05).sin() * 2.0
}

fn lerp_f64(alpha: f64, start: f64, end: f64) -> f64 {
    start + alpha * (end - start)
}

fn lerp_f32(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

fn argb_srgb_lerp_color(alpha: f32, start: u32, end: u32) -> [f32; 4] {
    let lerp_channel = |shift: u32| -> f32 {
        let from = ((start >> shift) & 0xFF) as i32;
        let to = ((end >> shift) & 0xFF) as i32;
        (from + (alpha * (to - from) as f32).floor() as i32) as f32 / 255.0
    };
    [
        lerp_channel(16),
        lerp_channel(8),
        lerp_channel(0),
        lerp_channel(24),
    ]
}

fn rotated_billboard_axes(axes: ParticleBillboardAxes, roll: f32) -> (Vec3, Vec3) {
    if roll == 0.0 {
        return (axes.right, axes.up);
    }
    let (sin, cos) = roll.sin_cos();
    (
        axes.right * cos + axes.up * sin,
        -axes.right * sin + axes.up * cos,
    )
}

fn vault_connection_alpha(age_ticks: u32, lifetime_ticks: u32, partial_tick: f32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let normalized = (age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime;
    let time = ((normalized - 0.25) / 0.75).clamp(0.0, 1.0);
    time * 0.6
}

fn particle_render_group_for_particle(particle_id: &str) -> ParticleRenderGroup {
    match particle_id {
        "minecraft:elder_guardian" => ParticleRenderGroup::ElderGuardians,
        _ => ParticleRenderGroup::SingleQuads,
    }
}

fn particle_render_layer_for_particle(particle_id: &str) -> ParticleRenderLayer {
    match particle_id {
        "minecraft:block"
        | "minecraft:block_marker"
        | "minecraft:dust_pillar"
        | "minecraft:block_crumble" => ParticleRenderLayer::OpaqueTerrain,
        "minecraft:item"
        | "minecraft:item_slime"
        | "minecraft:item_cobweb"
        | "minecraft:item_snowball" => ParticleRenderLayer::OpaqueItems,
        "minecraft:cloud"
        | "minecraft:campfire_cosy_smoke"
        | "minecraft:campfire_signal_smoke"
        | "minecraft:sneeze"
        | "minecraft:totem_of_undying"
        | "minecraft:squid_ink"
        | "minecraft:glow_squid_ink"
        | "minecraft:end_rod"
        | "minecraft:soul"
        | "minecraft:sculk_soul"
        | "minecraft:sculk_charge"
        | "minecraft:sculk_charge_pop"
        | "minecraft:shriek"
        | "minecraft:vibration"
        | "minecraft:vault_connection"
        | "minecraft:effect"
        | "minecraft:instant_effect"
        | "minecraft:entity_effect"
        | "minecraft:flash"
        | "minecraft:firework"
        | "minecraft:firefly"
        | "minecraft:elder_guardian"
        | "minecraft:infested"
        | "minecraft:raid_omen"
        | "minecraft:trial_omen"
        | "minecraft:witch" => ParticleRenderLayer::Translucent,
        _ => ParticleRenderLayer::Opaque,
    }
}

fn particle_atlas_uv_sub_rect_for_particle(
    particle_id: &str,
    random: &mut ParticleRandom,
) -> Option<ParticleAtlasUvSubRect> {
    matches!(
        particle_id,
        "minecraft:block"
            | "minecraft:dust_pillar"
            | "minecraft:block_crumble"
            | "minecraft:item"
            | "minecraft:item_slime"
            | "minecraft:item_cobweb"
            | "minecraft:item_snowball"
    )
    .then(|| ParticleAtlasUvSubRect {
        u_offset: random.next_f32() * 3.0,
        v_offset: random.next_f32() * 3.0,
    })
}

fn particle_vertex(
    position: Vec3,
    uv: [f32; 2],
    color: [f32; 4],
    light: [f32; 2],
) -> ParticleVertex {
    ParticleVertex {
        position: position.to_array(),
        uv,
        color,
        light,
    }
}

fn sanitize_particle_light(light: [f32; 2]) -> [f32; 2] {
    [
        if light[0].is_finite() {
            light[0].clamp(0.0, 1.0)
        } else {
            DEFAULT_PARTICLE_LIGHT[0]
        },
        if light[1].is_finite() {
            light[1].clamp(0.0, 1.0)
        } else {
            DEFAULT_PARTICLE_LIGHT[1]
        },
    ]
}

fn particle_light_with_emission(instance: &ParticleInstance, sampled_light: [f32; 2]) -> [f32; 2] {
    match instance.light_emission {
        ParticleLightEmissionDescriptor::World => sampled_light,
        ParticleLightEmissionDescriptor::FullBright => [1.0, 1.0],
        ParticleLightEmissionDescriptor::FullBlock => [1.0, sampled_light[1]],
        ParticleLightEmissionDescriptor::SmoothBlockByAge => {
            let emission = particle_light_emission_progress(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
            [(sampled_light[0] + emission).min(1.0), sampled_light[1]]
        }
        ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic => {
            let age = instance.age_ticks as f32 / instance.lifetime_ticks.max(1) as f32;
            let emission = age * age * age * age;
            [(sampled_light[0] + emission).min(1.0), sampled_light[1]]
        }
        ParticleLightEmissionDescriptor::Firefly => {
            let progress = (instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
                / instance.lifetime_ticks.max(1) as f32;
            [firefly_fade_amount(progress, 0.1, 0.3), 0.0]
        }
    }
}

fn particle_light_emission_progress(age_ticks: u32, lifetime_ticks: u32, partial_tick: f32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    ((age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime).clamp(0.0, 1.0)
}

fn firefly_fade_amount(lifetime_progress: f32, fade_in_time: f32, fade_out_time: f32) -> f32 {
    let lifetime_progress = lifetime_progress.clamp(0.0, 1.0);
    if lifetime_progress >= 1.0 - fade_in_time {
        (1.0 - lifetime_progress) / fade_in_time
    } else if lifetime_progress <= fade_out_time {
        lifetime_progress / fade_out_time
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::descriptors::VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT;
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
    fn particle_runtime_delays_shriek_tick_until_vanilla_delay_clears() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut command = spawn_command("minecraft:shriek", 1.0);
        command.initial_delay_ticks = 2;
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![command],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.delay_ticks, 2);
        assert_eq!(instance.age_ticks, 0);
        assert_eq!(instance.position, [1.0, 0.0, 0.0]);
        assert_eq!(instance.velocity, [0.0, 0.1, 0.0]);

        particles.advance(1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.delay_ticks, 1);
        assert_eq!(instance.age_ticks, 0);
        assert_eq!(instance.position, [1.0, 0.0, 0.0]);

        particles.advance(1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.delay_ticks, 0);
        assert_eq!(instance.age_ticks, 0);
        assert_eq!(instance.position, [1.0, 0.0, 0.0]);

        particles.advance(1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.position, [1.0, 0.1, 0.0]);
        assert_close3(instance.velocity, [0.0, 0.098, 0.0]);
        assert_close_f32(instance.color[3], 1.0 - 1.0 / 30.0);
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
    fn particle_runtime_dust_plume_decays_gravity_and_friction_before_motion() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:dust_plume", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.2, 0.3, -0.4];
        instance.gravity = 0.5;
        instance.friction = 0.96;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close_f32(instance.gravity, 0.44);
        assert_close_f32(instance.friction, 0.8832);
        assert_close3(instance.position, [1.2, 2.2824, 2.6]);
        assert_close3(instance.velocity, [0.17664, 0.249_415_68, -0.35328]);
    }

    #[test]
    fn particle_runtime_falling_dust_rotates_and_clamps_downward_motion() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:falling_dust", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.2, -0.139, -0.4];
        instance.roll = 0.3;
        instance.previous_roll = 0.2;
        instance.roll_speed = 0.02;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.2, 1.861, 2.6]);
        assert_close3(instance.velocity, [0.2, -0.14, -0.4]);
        assert_close_f32(instance.previous_roll, 0.3);
        assert_close_f32(instance.roll, 0.3 + std::f32::consts::PI * 0.02 * 2.0);
    }

    #[test]
    fn particle_runtime_water_drop_uses_direct_gravity_and_friction() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:rain", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.2, 0.3, -0.4];
        instance.gravity = 0.06;
        instance.friction = 0.98;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.2, 2.24, 2.6]);
        assert_close3(instance.velocity, [0.196, 0.2352, -0.392]);
    }

    #[test]
    fn particle_runtime_drip_hang_applies_post_move_damping_before_friction() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:dripping_honey", 100);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.1, 0.0, -0.2];
        instance.gravity = 0.000_012;
        instance.friction = 0.98;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.1, 1.999_988, 2.8]);
        assert_close3(instance.velocity, [0.001_96, -0.000_000_235_2, -0.003_92]);
    }

    #[test]
    fn particle_runtime_lava_drip_hang_updates_cooling_color_before_motion() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:dripping_lava", 40);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.1, 0.0, -0.2];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_eq!(instance.color, [1.0, 1.0, 0.5, 1.0]);
        assert_close3(instance.position, [1.1, 1.9988, 2.8]);
        assert_close3(instance.velocity, [0.001_96, -0.000_023_52, -0.003_92]);
    }

    #[test]
    fn particle_runtime_wake_uses_command_motion_and_vanilla_sprite_cycle() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:fishing", 39);
        instance.sprite_ids = vec![
            "minecraft:wake_0".to_string(),
            "minecraft:wake_1".to_string(),
            "minecraft:wake_2".to_string(),
            "minecraft:wake_3".to_string(),
            "minecraft:wake_4".to_string(),
        ];
        instance.current_sprite_index = Some(0);
        instance.current_sprite_id = Some("minecraft:wake_0".to_string());
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.2, 0.3, -0.4];
        instance.gravity = 0.0;
        instance.friction = 0.98;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.active_instances, 1);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.2, 2.3, 2.6]);
        assert_close3(instance.velocity, [0.196, 0.294, -0.392]);
        assert_eq!(instance.current_sprite_index, Some(1));
        assert_eq!(
            instance.current_sprite_id.as_deref(),
            Some("minecraft:wake_1")
        );
    }

    #[test]
    fn particle_runtime_campfire_smoke_drifts_up_and_fades_near_lifetime_end() {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
        let mut instance = test_instance_with_lifetime("minecraft:campfire_cosy_smoke", 100);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.0, 0.002, 0.0];
        instance.age_ticks = 39;
        instance.color = [1.0, 1.0, 1.0, 0.9];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 40);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_range_f64(instance.velocity[0].abs(), 0.0, 0.0002);
        assert_range_f64(instance.velocity[2].abs(), 0.0, 0.0002);
        assert_close_f64(instance.velocity[1], 0.002 - 3.0E-6);
        assert_close_f64(instance.position[1], 2.0 + 0.002 - 3.0E-6);
        assert_close_f32(instance.color[3], 0.885);
    }

    #[test]
    fn particle_runtime_lava_emits_child_smoke_after_tick_when_vanilla_odds_pass() {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(8, 8, 0);
        let mut lava = test_instance_with_lifetime("minecraft:lava", 20);
        lava.position = [1.0, 2.0, 3.0];
        lava.previous_position = lava.position;
        lava.velocity = [0.1, 0.2, 0.3];
        lava.child_spawn_templates = vec![ParticleChildSpawnTemplate {
            particle_type_id: 62,
            particle_id: "minecraft:smoke".to_string(),
            sprite_ids: vec!["minecraft:generic_7".to_string()],
        }];
        particles.active_instances.push_back(lava);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        assert_eq!(summary.intaken_instances, 1);
        assert_eq!(summary.active_instances, 2);
        let lava = &particles.active_instances()[0];
        assert_eq!(lava.age_ticks, 1);
        assert_close3(lava.position, [1.1, 2.17, 3.3]);
        assert_close3(lava.velocity, [0.0999, 0.16983, 0.2997]);

        let smoke = &particles.active_instances()[1];
        assert_eq!(smoke.particle_type_id, 62);
        assert_eq!(smoke.particle_id, "minecraft:smoke");
        assert_eq!(smoke.provider, "SmokeParticle.Provider");
        assert_eq!(
            smoke.current_sprite_id.as_deref(),
            Some("minecraft:generic_7")
        );
        assert_close3(smoke.position, lava.position);
        // The lava particle spawns a `minecraft:smoke` child with its post-tick
        // velocity as the command velocity. Vanilla `SmokeParticle` (via
        // `BaseAshSmokeParticle` -> the base `Particle` 6-arg constructor) then
        // adds the constructor-random spread scaled by `0.1` on intake, so the
        // child velocity is the lava velocity plus a small deterministic spread
        // rather than an exact copy of it.
        assert_close3(
            smoke.velocity,
            [0.10881595316538636, 0.17285028287621526, 0.3025607498781397],
        );
        assert!(smoke.velocity[0] > lava.velocity[0]);
        assert!(smoke.velocity[1] > lava.velocity[1]);
        assert!(smoke.velocity[2] > lava.velocity[2]);
        assert!(smoke.child_spawn_templates.is_empty());
    }

    #[test]
    fn particle_runtime_smoke_intake_applies_vanilla_base_particle_spread() {
        // Vanilla `SmokeParticle` (via `BaseAshSmokeParticle` -> the base
        // `Particle` 6-arg constructor) seeds a constructor-random velocity,
        // scales it by `0.1`, then adds the command velocity, matching the
        // player-cloud velocity model. The intake path must therefore offset
        // the command velocity by the deterministic base spread instead of
        // copying the command velocity verbatim.
        let command_velocity = [0.3, 0.4, 0.5];
        for particle_id in [
            "minecraft:smoke",
            "minecraft:large_smoke",
            "minecraft:white_smoke",
        ] {
            let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
            let mut command = spawn_command(particle_id, 1.0);
            command.velocity = command_velocity;
            particles.submit_batch(ParticleSpawnBatch {
                commands: vec![command],
                ..ParticleSpawnBatch::default()
            });
            particles.advance(0);

            let instance = &particles.active_instances()[0];
            assert_eq!(instance.particle_id, particle_id);
            let expected =
                descriptors::ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                    scale: 0.1,
                }
                .sample(command_velocity, &mut ParticleRandom::new(0));
            assert_close3(instance.velocity, expected);
            assert_ne!(instance.velocity, command_velocity, "{particle_id}");
        }
    }

    #[test]
    fn particle_runtime_huge_explosion_seed_emits_vanilla_child_explosions() {
        let explosion_template = ParticleChildSpawnTemplate {
            particle_type_id: 23,
            particle_id: "minecraft:explosion".to_string(),
            sprite_ids: vec!["minecraft:explosion_0".to_string()],
        };
        let mut seed_for_command = test_instance_with_lifetime("minecraft:explosion_emitter", 8);
        seed_for_command.position = [1.0, 0.0, 0.0];
        seed_for_command.age_ticks = 3;
        seed_for_command.child_spawn_templates = vec![explosion_template.clone()];
        let commands = seed_for_command.child_spawn_commands(&mut ParticleRandom::new(0));
        assert_eq!(commands.len(), 6);
        for command in &commands {
            assert_eq!(command.particle_type_id, 23);
            assert_eq!(command.particle_id, "minecraft:explosion");
            assert_close3(command.velocity, [2.0 / 8.0, 0.0, 0.0]);
            assert_range_f64(command.position[0], -3.0, 5.0);
            assert_range_f64(command.position[1], -4.0, 4.0);
            assert_range_f64(command.position[2], -4.0, 4.0);
        }

        let mut particles = ParticleRuntimeState::with_capacities_and_seed(32, 32, 0);
        let mut seed = spawn_command("minecraft:explosion_emitter", 1.0);
        seed.child_spawn_templates = vec![explosion_template];
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![seed],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        let seed = &particles.active_instances()[0];
        assert_eq!(seed.provider, "HugeExplosionSeedParticle.Provider");
        assert_eq!(seed.render_group, ParticleRenderGroup::NoRender);
        assert_eq!(seed.lifetime_ticks, 8);

        let summary = particles.advance(1);

        assert_eq!(summary.intaken_instances, 6);
        assert_eq!(summary.active_instances, 7);
        let seed = &particles.active_instances()[0];
        assert_eq!(seed.age_ticks, 1);
        for child in particles.active_instances().iter().skip(1) {
            assert_eq!(child.particle_type_id, 23);
            assert_eq!(child.particle_id, "minecraft:explosion");
            assert_eq!(child.provider, "HugeExplosionParticle.Provider");
            assert_eq!(
                child.current_sprite_id.as_deref(),
                Some("minecraft:explosion_0")
            );
            assert_close_f32(child.base_quad_size, 2.0);
            assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
            assert!(child.child_spawn_templates.is_empty());
        }

        let summary = particles.advance(1);

        assert_eq!(summary.intaken_instances, 6);
        assert_eq!(summary.active_instances, 13);
        for child in particles.active_instances().iter().skip(7) {
            assert_close_f32(child.base_quad_size, 1.875);
        }
    }

    #[test]
    fn particle_runtime_gust_seed_emits_vanilla_child_gusts() {
        let gust_template = ParticleChildSpawnTemplate {
            particle_type_id: 24,
            particle_id: "minecraft:gust".to_string(),
            sprite_ids: vec!["minecraft:gust_0".to_string()],
        };
        let mut seed_for_command = test_instance_with_lifetime("minecraft:gust_emitter_large", 8);
        seed_for_command.position = [1.0, 0.0, 0.0];
        seed_for_command.age_ticks = 2;
        seed_for_command.child_spawn_templates = vec![gust_template.clone()];
        let commands = seed_for_command.child_spawn_commands(&mut ParticleRandom::new(0));
        assert_eq!(commands.len(), 3);
        for command in &commands {
            assert_eq!(command.particle_type_id, 24);
            assert_eq!(command.particle_id, "minecraft:gust");
            assert_close3(command.velocity, [1.0 / 7.0, 0.0, 0.0]);
        }

        let mut particles = ParticleRuntimeState::with_capacities_and_seed(16, 16, 0);
        let mut large = spawn_command("minecraft:gust_emitter_large", 1.0);
        large.child_spawn_templates = vec![gust_template.clone()];
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![large],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        let seed = &particles.active_instances()[0];
        assert_eq!(seed.provider, "GustSeedParticle.Provider(3.0,7,0)");
        assert_eq!(seed.render_group, ParticleRenderGroup::NoRender);
        assert_eq!(seed.lifetime_ticks, 8);

        let summary = particles.advance(1);

        assert_eq!(summary.intaken_instances, 3);
        assert_eq!(summary.active_instances, 4);
        let seed = &particles.active_instances()[0];
        assert_eq!(seed.age_ticks, 1);
        for child in particles.active_instances().iter().skip(1) {
            assert_eq!(child.particle_type_id, 24);
            assert_eq!(child.particle_id, "minecraft:gust");
            assert_eq!(child.provider, "GustParticle.Provider");
            assert_eq!(child.current_sprite_id.as_deref(), Some("minecraft:gust_0"));
            assert_range_f64(child.position[0], -2.0, 4.0);
            assert_range_f64(child.position[1], -3.0, 3.0);
            assert_range_f64(child.position[2], -3.0, 3.0);
            assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
            assert!(child.child_spawn_templates.is_empty());
        }

        let summary = particles.advance(1);

        assert_eq!(summary.intaken_instances, 3);
        assert_eq!(summary.active_instances, 7);
        for child in particles.active_instances().iter().skip(4) {
            assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
        }

        let mut particles = ParticleRuntimeState::with_capacities_and_seed(16, 16, 0);
        let mut small = spawn_command("minecraft:gust_emitter_small", 1.0);
        small.child_spawn_templates = vec![gust_template];
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![small],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        assert_eq!(particles.advance(1).intaken_instances, 3);
        assert_eq!(particles.advance(1).intaken_instances, 0);
        assert_eq!(particles.advance(1).intaken_instances, 0);
        assert_eq!(particles.advance(1).intaken_instances, 3);
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
    fn particle_runtime_firefly_first_tick_rerolls_speed_and_fades_alpha() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:firefly", 100);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.1, 0.2, -0.3];
        instance.color[3] = 0.0;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.1, 2.2, 2.7]);
        assert_range_f64(instance.velocity[0], -0.05, 0.05);
        assert_range_f64(instance.velocity[1], -0.05, 0.05);
        assert_range_f64(instance.velocity[2], -0.05, 0.05);
        assert_close_f32(instance.color[3], firefly_fade_amount(0.01, 0.3, 0.5));
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
    fn particle_runtime_current_down_tick_uses_vanilla_swirl_motion() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:current_down", 30);
        instance.position = [1.0, 2.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.0, -0.05, 0.0];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close3(instance.position, [1.042, 1.95, 3.0]);
        assert_close3(instance.velocity, [0.042, -0.05, 0.0]);
        assert_close_f32(instance.tick_angle, 0.08);
    }

    #[test]
    fn particle_runtime_fly_towards_position_tick_uses_vanilla_curve() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:enchant", 40);
        instance.start_position = [10.0, 64.0, -2.0];
        instance.position = [6.0, 65.0, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [-4.0, 1.0, 5.0];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        let pos = 1.0_f64 - 1.0 / 40.0;
        let pp = (1.0 - pos).powi(4);
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [6.0, 65.0, 3.0]);
        assert_close3(
            instance.position,
            [
                10.0 - 4.0 * pos,
                64.0 + 1.0 * pos - pp * 1.2,
                -2.0 + 5.0 * pos,
            ],
        );
        assert_close3(instance.velocity, [-4.0, 1.0, 5.0]);
    }

    #[test]
    fn particle_runtime_fly_straight_towards_uses_vanilla_curve_and_srgb_lerp() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:ominous_spawning", 25);
        instance.start_position = [1.0, 2.0, 3.0];
        instance.position = [1.25, 2.5, 2.25];
        instance.previous_position = instance.position;
        instance.velocity = [0.25, 0.5, -0.75];
        instance.color = [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0, 1.0];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [1.25, 2.5, 2.25]);
        assert_close3(instance.position, [1.24, 2.48, 2.28]);
        assert_close3(instance.velocity, [0.25, 0.5, -0.75]);
        assert_close_f32(instance.color[0], 76.0 / 255.0);
        assert_close_f32(instance.color[1], 177.0 / 255.0);
        assert_close_f32(instance.color[2], 254.0 / 255.0);
        assert_close_f32(instance.color[3], 1.0);
    }

    #[test]
    fn particle_runtime_trail_tick_interpolates_toward_option_target() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:trail", 5);
        instance.position = [0.0, 0.0, 0.0];
        instance.previous_position = instance.position;
        instance.option_target = Some([4.0, 8.0, 12.0]);
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [0.0, 0.0, 0.0]);
        assert_close3(instance.position, [1.0, 2.0, 3.0]);

        let mut terminal = test_instance_with_lifetime("minecraft:trail", 5);
        terminal.age_ticks = 4;
        terminal.position = [3.0, 6.0, 9.0];
        terminal.previous_position = terminal.position;
        terminal.option_target = Some([4.0, 8.0, 12.0]);
        terminal.tick_motion_without_collision(&mut ParticleRandom::new(0));

        assert_close3(terminal.previous_position, [3.0, 6.0, 9.0]);
        assert_close3(terminal.position, [4.0, 8.0, 12.0]);
        assert!(terminal
            .position
            .iter()
            .all(|coordinate| coordinate.is_finite()));
    }

    #[test]
    fn particle_runtime_vibration_tick_interpolates_toward_option_target_and_rotation() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:vibration", 5);
        instance.position = [0.0, 0.0, 0.0];
        instance.previous_position = instance.position;
        instance.option_target = Some([4.0, 8.0, 12.0]);
        instance.previous_yaw = 8.0;
        instance.yaw = 9.0;
        instance.previous_pitch = 6.0;
        instance.pitch = 7.0;
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        let (yaw, pitch) = vibration_particle_angles([1.0, 2.0, 3.0], [4.0, 8.0, 12.0]);
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [0.0, 0.0, 0.0]);
        assert_close3(instance.position, [1.0, 2.0, 3.0]);
        assert_close_f32(instance.previous_yaw, 9.0);
        assert_close_f32(instance.yaw, yaw);
        assert_close_f32(instance.previous_pitch, 7.0);
        assert_close_f32(instance.pitch, pitch);

        let mut terminal = test_instance_with_lifetime("minecraft:vibration", 5);
        terminal.age_ticks = 4;
        terminal.position = [3.0, 6.0, 9.0];
        terminal.previous_position = terminal.position;
        terminal.option_target = Some([4.0, 8.0, 12.0]);
        terminal.tick_motion_without_collision(&mut ParticleRandom::new(0));

        assert_close3(terminal.previous_position, [3.0, 6.0, 9.0]);
        assert_close3(terminal.position, [4.0, 8.0, 12.0]);
        assert!(terminal
            .position
            .iter()
            .all(|coordinate| coordinate.is_finite()));
    }

    #[test]
    fn particle_runtime_portal_tick_uses_vanilla_start_position_curve() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:portal", 40);
        instance.start_position = [10.0, 64.0, -2.0];
        instance.position = instance.start_position;
        instance.previous_position = instance.position;
        instance.velocity = [-5.0, 0.5, 5.0];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        let progress = 1.0_f64 / 40.0;
        let position_scale = 1.0 - (-progress + progress * progress * 2.0);
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [10.0, 64.0, -2.0]);
        assert_close3(
            instance.position,
            [
                10.0 - 5.0 * position_scale,
                64.0 + 0.5 * position_scale + (1.0 - progress),
                -2.0 + 5.0 * position_scale,
            ],
        );
        assert_close3(instance.velocity, [-5.0, 0.5, 5.0]);
    }

    #[test]
    fn particle_runtime_reverse_portal_tick_uses_incremental_age_scaled_velocity() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:reverse_portal", 60);
        instance.position = [10.0, 64.0, -2.0];
        instance.previous_position = instance.position;
        instance.velocity = [-6.0, 0.6, 6.0];
        particles.active_instances.push_back(instance);

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 0);
        let instance = &particles.active_instances()[0];
        let speed_multiplier = 1.0_f64 / 60.0;
        assert_eq!(instance.age_ticks, 1);
        assert_close3(instance.previous_position, [10.0, 64.0, -2.0]);
        assert_close3(
            instance.position,
            [
                10.0 - 6.0 * speed_multiplier,
                64.0 + 0.6 * speed_multiplier,
                -2.0 + 6.0 * speed_multiplier,
            ],
        );
        assert_close3(instance.velocity, [-6.0, 0.6, 6.0]);
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
    fn particle_runtime_simple_animated_alpha_fades_after_half_lifetime() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:totem_of_undying", 60);
        instance.age_ticks = 30;
        instance.color[3] = 1.0;
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 31);
        assert_close_f32(instance.color[3], 1.0 - 1.0 / 60.0);
    }

    #[test]
    fn particle_runtime_squid_ink_alpha_fades_after_half_lifetime() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:squid_ink", 20);
        instance.age_ticks = 10;
        instance.color[3] = 1.0;
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 11);
        assert_close_f32(instance.color[3], 0.95);
    }

    #[test]
    fn particle_runtime_end_rod_alpha_and_rgb_fade_after_half_lifetime() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:end_rod", 60);
        instance.age_ticks = 30;
        instance.color = [1.0, 1.0, 1.0, 1.0];
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 31);
        let fade = descriptors::END_ROD_FADE_COLOR;
        assert_close_f32(instance.color[0], 1.0 + (fade[0] - 1.0) * 0.2);
        assert_close_f32(instance.color[1], 1.0 + (fade[1] - 1.0) * 0.2);
        assert_close_f32(instance.color[2], 1.0 + (fade[2] - 1.0) * 0.2);
        assert_close_f32(instance.color[3], 1.0 - 1.0 / 60.0);
    }

    #[test]
    fn particle_runtime_vault_connection_alpha_follows_vanilla_lifetime_window() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:vault_connection", 40);
        instance.age_ticks = 20;
        instance.color[3] = 0.0;
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 21);
        assert_close_f32(instance.color[3], 0.22);
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

    // Independent witness for `BaseAshSmokeParticle`: `Particle.java` 7-arg base
    // spread (super called with xa=ya=za=0), reconstructed straight from the
    // vanilla source lines so it does not lean on the descriptor under test.
    fn base_ash_smoke_base_spread(seed: i64) -> [f64; 3] {
        let mut random = ParticleRandom::new(seed);
        let x = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let y = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let z = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
        let length = (x * x + y * y + z * z).sqrt();
        [
            x / length * speed * 0.4,
            y / length * speed * 0.4 + 0.1,
            z / length * speed * 0.4,
        ]
    }

    // Full `BaseAshSmokeParticle` velocity: base spread times per-axis `dir`
    // (`xd *= dirX; yd *= dirY; zd *= dirZ`) plus the provider velocity
    // (`xd += xa; yd += ya; zd += za`). `white_ash` draws the same negative-biased
    // xa/ya/za as `WhiteAshParticle.Provider`; `ash` adds `(0, 0, 0)`.
    fn expected_base_ash_smoke_velocity(seed: i64, dir: [f64; 3], white_ash: bool) -> [f64; 3] {
        let mut random = ParticleRandom::new(seed);
        let x = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let y = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let z = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
        let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
        let length = (x * x + y * y + z * z).sqrt();
        let base = [
            x / length * speed * 0.4,
            y / length * speed * 0.4 + 0.1,
            z / length * speed * 0.4,
        ];
        let offset = if white_ash {
            [
                f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
                f64::from(random.next_f32()) * -0.5 * f64::from(random.next_f32()) * 0.1 * 5.0,
                f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
            ]
        } else {
            [0.0, 0.0, 0.0]
        };
        [
            base[0] * dir[0] + offset[0],
            base[1] * dir[1] + offset[1],
            base[2] * dir[2] + offset[2],
        ]
    }

    #[test]
    fn ash_provider_applies_per_axis_dir_to_base_spread_and_ignores_command_velocity() {
        // AshParticle.Provider.createParticle forces provider velocity (0, 0, 0);
        // the incoming command velocity must be dropped.
        let mut command = spawn_command("minecraft:ash", 5.0);
        command.velocity = [3.0, 4.0, 5.0];
        let mut random = ParticleRandom::new(0);

        let ash = ParticleInstance::from_spawn_command(command, &mut random);

        let dir = [0.1, -0.1, 0.1];
        let expected = expected_base_ash_smoke_velocity(0, dir, false);
        assert_close_f64(ash.velocity[0], expected[0]);
        assert_close_f64(ash.velocity[1], expected[1]);
        assert_close_f64(ash.velocity[2], expected[2]);

        // Per-axis dir: x/z scaled by 0.1, y negated and damped by 0.1.
        let base = base_ash_smoke_base_spread(0);
        assert_close_f64(ash.velocity[0], base[0] * 0.1);
        assert_close_f64(ash.velocity[1], base[1] * -0.1);
        assert_close_f64(ash.velocity[2], base[2] * 0.1);

        // Command velocity is fully ignored: nowhere near [3, 4, 5].
        assert_ne!(ash.velocity, [3.0, 4.0, 5.0]);
        assert!(ash.velocity[0].abs() < 0.02);
        assert!(ash.velocity[1].abs() < 0.03);
        assert!(ash.velocity[2].abs() < 0.02);
    }

    #[test]
    fn white_ash_provider_adds_negative_biased_offset_to_per_axis_base_spread() {
        // WhiteAshParticle.Provider.createParticle ignores the command velocity and
        // adds its own negative-biased xa/ya/za on top of the dir-scaled spread.
        let mut command = spawn_command("minecraft:white_ash", 5.0);
        command.velocity = [3.0, 4.0, 5.0];
        let mut random = ParticleRandom::new(0);

        let white_ash = ParticleInstance::from_spawn_command(command, &mut random);

        let dir = [0.1, -0.1, 0.1];
        let expected = expected_base_ash_smoke_velocity(0, dir, true);
        assert_close_f64(white_ash.velocity[0], expected[0]);
        assert_close_f64(white_ash.velocity[1], expected[1]);
        assert_close_f64(white_ash.velocity[2], expected[2]);

        // Command velocity is ignored.
        assert_ne!(white_ash.velocity, [3.0, 4.0, 5.0]);

        // The provider offset makes white_ash diverge from the ash zero-offset
        // branch at the same seed. Because `ya = rand*-0.5*rand*0.1*5.0 <= 0`, the
        // extra provider velocity always biases the y component downward.
        let ash_only = expected_base_ash_smoke_velocity(0, dir, false);
        assert_ne!(white_ash.velocity, ash_only);
        assert!(white_ash.velocity[1] < ash_only[1]);
    }

    #[test]
    fn dust_plume_provider_adds_command_velocity_and_y_offset_to_per_axis_base_spread() {
        // DustPlumeParticle.Provider passes the command velocity xAux/yAux/zAux as
        // xa/ya/za; DustPlumeParticle calls
        // super(..., 0.7F, 0.6F, 0.7F, xa, ya + 0.15F, za, ...), so the Particle
        // base spread is scaled per axis by (0.7, 0.6, 0.7) and the command
        // velocity (with +0.15 on y) is added on top.
        let mut command = spawn_command("minecraft:dust_plume", 1.0);
        command.velocity = [0.25, 0.5, -0.75];
        let mut random = ParticleRandom::new(86);

        let dust_plume = ParticleInstance::from_spawn_command(command, &mut random);

        let dir = [0.7, 0.6, 0.7];
        let spread = expected_base_ash_smoke_velocity(86, dir, false);
        assert_close_f64(dust_plume.velocity[0], spread[0] + 0.25);
        assert_close_f64(dust_plume.velocity[1], spread[1] + 0.5 + 0.15);
        assert_close_f64(dust_plume.velocity[2], spread[2] - 0.75);

        // Unlike the old CommandWithYOffset path, the per-axis base spread is now
        // applied, so the result is not exactly command velocity + 0.15 on y.
        assert_ne!(dust_plume.velocity, [0.25, 0.65, -0.75]);
    }

    #[test]
    fn firework_spark_provider_uses_vanilla_simple_animated_state() {
        let mut random = ParticleRandom::new(71);
        let mut command = spawn_command("minecraft:firework", 1.0);
        command.sprite_ids = vec![
            "minecraft:firework_0".to_string(),
            "minecraft:firework_1".to_string(),
            "minecraft:firework_2".to_string(),
        ];
        command.velocity = [0.1, 0.2, 0.3];

        let firework = ParticleInstance::from_spawn_command(command, &mut random);

        assert_eq!(firework.provider, "FireworkParticles.SparkProvider");
        assert_eq!(firework.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(
            firework.current_sprite_id.as_deref(),
            Some("minecraft:firework_0")
        );
        assert_range_f32(firework.base_quad_size, 0.075, 0.15);
        assert!((48..=59).contains(&firework.lifetime_ticks));
        assert_eq!(firework.color, [1.0, 1.0, 1.0, 0.99]);
        assert_eq!(firework.velocity, [0.1, 0.2, 0.3]);
        assert_eq!(firework.friction, 0.91);
        assert_eq!(firework.gravity, 0.1);
        assert!(firework.has_physics);
        assert_eq!(firework.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(
            firework.light_emission,
            ParticleLightEmissionDescriptor::FullBright
        );
    }

    #[test]
    fn particle_runtime_firework_spark_alpha_preserves_initial_then_fades() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:firework", 48);
        instance.color[3] = 0.99;
        instance.age_ticks = 23;
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &mut particles.active_instances[0];
        assert_eq!(instance.age_ticks, 24);
        assert_close_f32(instance.color[3], 0.99);
        instance.age_ticks = 24;
        instance.color[3] = 0.99;

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.age_ticks, 25);
        assert_close_f32(instance.color[3], 1.0 - 1.0 / 48.0);
    }

    #[test]
    fn particle_runtime_mirrors_falling_leaves_provider_state_and_motion() {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(8, 8, 7);
        let mut cherry = spawn_command("minecraft:cherry_leaves", 1.0);
        cherry.sprite_ids = vec![
            "minecraft:cherry_leaf_0".to_string(),
            "minecraft:cherry_leaf_1".to_string(),
        ];
        let mut pale_oak = spawn_command("minecraft:pale_oak_leaves", 1.0);
        pale_oak.sprite_ids = vec![
            "minecraft:pale_oak_leaf_0".to_string(),
            "minecraft:pale_oak_leaf_1".to_string(),
        ];
        let mut tinted = spawn_command("minecraft:tinted_leaves", 1.0);
        tinted.sprite_ids = vec![
            "minecraft:tinted_leaf_0".to_string(),
            "minecraft:tinted_leaf_1".to_string(),
        ];
        tinted.option_color = Some([0.25, 0.5, 0.75, 0.25]);
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![cherry, pale_oak, tinted],
            ..ParticleSpawnBatch::default()
        });

        particles.advance(0);

        let initial_sprites: Vec<_> = particles
            .active_instances()
            .iter()
            .map(|instance| instance.current_sprite_id.clone())
            .collect();
        let cherry = &particles.active_instances()[0];
        assert_eq!(cherry.provider, "FallingLeavesParticle.CherryProvider");
        assert_eq!(cherry.sprite_selection, ParticleSpriteSelection::Random);
        assert!(matches!(
            cherry.current_sprite_id.as_deref(),
            Some("minecraft:cherry_leaf_0" | "minecraft:cherry_leaf_1")
        ));
        assert_eq!(cherry.lifetime_ticks, 300);
        assert_range_f32(cherry.base_quad_size, 0.05, 0.075);
        assert_eq!(cherry.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(cherry.velocity, [0.0, -0.0, 0.0]);
        assert_eq!(cherry.friction, 1.0);
        assert_close_f32(cherry.gravity, 0.00075);
        assert_eq!(
            cherry.tick_motion,
            ParticleTickMotionDescriptor::FallingLeaves
        );
        assert_eq!(cherry.render_layer, ParticleRenderLayer::Opaque);
        assert!(cherry.has_physics);
        let cherry_motion = cherry.falling_leaves_motion.expect("falling leaves motion");
        assert!(cherry_motion.flow_away);
        assert!(!cherry_motion.swirl);
        assert_close_f32(cherry_motion.wind_big, 2.0);

        let pale_oak = &particles.active_instances()[1];
        assert_eq!(pale_oak.provider, "FallingLeavesParticle.PaleOakProvider");
        assert!(matches!(
            pale_oak.current_sprite_id.as_deref(),
            Some("minecraft:pale_oak_leaf_0" | "minecraft:pale_oak_leaf_1")
        ));
        assert_range_f32(pale_oak.base_quad_size, 0.1, 0.15);
        assert_eq!(pale_oak.velocity, [0.0, -0.021, 0.0]);
        assert_close_f32(pale_oak.gravity, 0.00021);
        let pale_motion = pale_oak
            .falling_leaves_motion
            .expect("falling leaves motion");
        assert!(!pale_motion.flow_away);
        assert!(pale_motion.swirl);
        assert_close_f32(pale_motion.wind_big, 10.0);

        let tinted = &particles.active_instances()[2];
        assert_eq!(
            tinted.provider,
            "FallingLeavesParticle.TintedLeavesProvider"
        );
        assert!(matches!(
            tinted.current_sprite_id.as_deref(),
            Some("minecraft:tinted_leaf_0" | "minecraft:tinted_leaf_1")
        ));
        assert_range_f32(tinted.base_quad_size, 0.1, 0.15);
        assert_eq!(tinted.color, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(tinted.velocity, [0.0, -0.021, 0.0]);
        assert_close_f32(tinted.gravity, 0.00021);
        assert_eq!(tinted.render_layer, ParticleRenderLayer::Opaque);

        particles.advance(1);

        for (instance, initial_sprite) in particles.active_instances().iter().zip(initial_sprites) {
            assert_eq!(instance.age_ticks, 1);
            assert_eq!(instance.current_sprite_id, initial_sprite);
            assert_ne!(instance.roll, 0.0);
            assert_eq!(instance.previous_roll, 0.0);
        }
        let cherry = &particles.active_instances()[0];
        assert_close_f64(cherry.position[1], -0.00075);
        assert_close_f64(cherry.velocity[1], -0.00075);
        assert!(cherry.position[0] != 0.0 || cherry.position[2] != 0.0);
        let pale_oak = &particles.active_instances()[1];
        assert_close_f64(pale_oak.position[1], -0.02121);
        assert_close_f64(pale_oak.velocity[1], -0.02121);
        assert!(pale_oak.position[0] != 0.0 || pale_oak.position[2] != 0.0);
        let tinted = &particles.active_instances()[2];
        assert_close_f64(tinted.position[1], -0.02121);
        assert_close_f64(tinted.velocity[1], -0.02121);
        assert!(tinted.position[0] != 0.0 || tinted.position[2] != 0.0);
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

        let mut cosy_random = ParticleRandom::new(46);
        let mut cosy_command = spawn_command("minecraft:campfire_cosy_smoke", 1.0);
        cosy_command.velocity = [0.1, 0.2, 0.3];
        let cosy = ParticleInstance::from_spawn_command(cosy_command, &mut cosy_random);
        assert_eq!(cosy.provider, "CampfireSmokeParticle.CosyProvider");
        assert_eq!(cosy.sprite_selection, ParticleSpriteSelection::Random);
        assert_range_f32(cosy.base_quad_size, 0.3, 0.6);
        assert_eq!(cosy.color, [1.0, 1.0, 1.0, 0.9]);
        assert!((80..=129).contains(&cosy.lifetime_ticks));
        assert_eq!(cosy.velocity[0], 0.1);
        assert_range_f64(cosy.velocity[1], 0.2, 0.202);
        assert_eq!(cosy.velocity[2], 0.3);
        assert_eq!(cosy.gravity, 3.0E-6);
        assert_eq!(
            cosy.tick_motion,
            ParticleTickMotionDescriptor::CampfireSmoke
        );
        assert_eq!(cosy.render_layer, ParticleRenderLayer::Translucent);

        let mut signal_random = ParticleRandom::new(47);
        let signal = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:campfire_signal_smoke", 1.0),
            &mut signal_random,
        );
        assert_eq!(signal.provider, "CampfireSmokeParticle.SignalProvider");
        assert_range_f32(signal.base_quad_size, 0.3, 0.6);
        assert_eq!(signal.color, [1.0, 1.0, 1.0, 0.95]);
        assert!((280..=329).contains(&signal.lifetime_ticks));
        assert_eq!(signal.render_layer, ParticleRenderLayer::Translucent);

        let mut lava_random = ParticleRandom::new(44);
        let lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:lava", 1.0),
            &mut lava_random,
        );
        assert_eq!(lava.provider, "LavaParticle.Provider");
        assert_eq!(lava.sprite_selection, ParticleSpriteSelection::Random);
        assert_range_f32(lava.base_quad_size, 0.02, 0.44);
        assert_eq!(lava.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(lava.quad_size_curve, ParticleQuadSizeCurve::Lava);
        assert!((16..=80).contains(&lava.lifetime_ticks));
        assert_range_f64(lava.velocity[0], -0.15, 0.15);
        assert_range_f64(lava.velocity[1], 0.05, 0.45);
        assert_range_f64(lava.velocity[2], -0.15, 0.15);
        assert_eq!(lava.friction, 0.999);
        assert_eq!(lava.gravity, 0.75);
        assert!(lava.has_physics);
        assert_eq!(
            lava.child_emission,
            Some(ParticleChildEmissionDescriptor::LavaSmoke)
        );

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

        let mut rain_random = ParticleRandom::new(62);
        let rain = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:rain", 1.0),
            &mut rain_random,
        );
        assert_eq!(rain.provider, "WaterDropParticle.Provider");
        assert_eq!(rain.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(rain.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(rain.base_quad_size, 0.1, 0.2);
        assert_eq!(rain.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((8..=40).contains(&rain.lifetime_ticks));
        assert_eq!(rain.friction, 0.98);
        assert_eq!(rain.gravity, 0.06);
        assert!(rain.has_physics);
        assert!(!rain.speed_up_when_y_motion_is_blocked);
        assert_eq!(rain.tick_motion, ParticleTickMotionDescriptor::WaterDrop);
        assert_eq!(rain.render_layer, ParticleRenderLayer::Opaque);
        assert_range_f64(rain.velocity[0], -0.06, 0.06);
        assert_range_f64(rain.velocity[1], 0.1, 0.3);
        assert_range_f64(rain.velocity[2], -0.06, 0.06);

        let mut splash_random = ParticleRandom::new(63);
        let mut splash_command = spawn_command("minecraft:splash", 1.0);
        splash_command.velocity = [0.25, 0.0, -0.75];
        let splash = ParticleInstance::from_spawn_command(splash_command, &mut splash_random);
        assert_eq!(splash.provider, "SplashParticle.Provider");
        assert_eq!(splash.sprite_selection, ParticleSpriteSelection::Random);
        assert_range_f32(splash.base_quad_size, 0.1, 0.2);
        assert!((8..=40).contains(&splash.lifetime_ticks));
        assert_eq!(splash.velocity, [0.25, 0.1, -0.75]);
        assert_eq!(splash.friction, 0.98);
        assert_eq!(splash.gravity, 0.04);
        assert!(splash.has_physics);
        assert_eq!(splash.tick_motion, ParticleTickMotionDescriptor::WaterDrop);
        assert_eq!(splash.render_layer, ParticleRenderLayer::Opaque);

        let mut wake_random = ParticleRandom::new(64);
        let mut wake_command = spawn_command("minecraft:fishing", 1.0);
        wake_command.velocity = [0.25, 0.5, -0.75];
        wake_command.sprite_ids = vec![
            "minecraft:wake_0".to_string(),
            "minecraft:wake_1".to_string(),
            "minecraft:wake_2".to_string(),
            "minecraft:wake_3".to_string(),
            "minecraft:wake_4".to_string(),
        ];
        let wake = ParticleInstance::from_spawn_command(wake_command, &mut wake_random);
        assert_eq!(wake.provider, "WakeParticle.Provider");
        assert_eq!(wake.sprite_selection, ParticleSpriteSelection::First);
        assert_eq!(wake.current_sprite_index, Some(0));
        assert_eq!(wake.current_sprite_id.as_deref(), Some("minecraft:wake_0"));
        assert_eq!(wake.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert_range_f32(wake.base_quad_size, 0.1, 0.2);
        assert!((8..=40).contains(&wake.lifetime_ticks));
        assert_eq!(wake.velocity, [0.25, 0.5, -0.75]);
        assert_eq!(wake.friction, 0.98);
        assert_eq!(wake.gravity, 0.0);
        assert!(wake.has_physics);
        assert_eq!(wake.tick_motion, ParticleTickMotionDescriptor::Wake);
        assert_eq!(wake.render_layer, ParticleRenderLayer::Opaque);

        let mut ominous_spawn_random = ParticleRandom::new(65);
        let mut ominous_spawn_command = spawn_command("minecraft:ominous_spawning", 1.0);
        ominous_spawn_command.position = [1.0, 2.0, 3.0];
        ominous_spawn_command.velocity = [0.25, 0.5, -0.75];
        ominous_spawn_command.sprite_ids = vec![
            "minecraft:ominous_spawn_0".to_string(),
            "minecraft:ominous_spawn_1".to_string(),
        ];
        let ominous_spawn =
            ParticleInstance::from_spawn_command(ominous_spawn_command, &mut ominous_spawn_random);
        assert_eq!(
            ominous_spawn.provider,
            "FlyStraightTowardsParticle.OminousSpawnProvider"
        );
        assert_eq!(
            ominous_spawn.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert!(matches!(ominous_spawn.current_sprite_index, Some(0 | 1)));
        assert!(matches!(
            ominous_spawn.current_sprite_id.as_deref(),
            Some("minecraft:ominous_spawn_0" | "minecraft:ominous_spawn_1")
        ));
        assert_eq!(ominous_spawn.start_position, [1.0, 2.0, 3.0]);
        assert_eq!(ominous_spawn.previous_position, [1.25, 2.5, 2.25]);
        assert_eq!(ominous_spawn.position, [1.25, 2.5, 2.25]);
        assert_eq!(ominous_spawn.velocity, [0.25, 0.5, -0.75]);
        assert_range_f32(ominous_spawn.base_quad_size, 0.06, 0.35);
        assert_eq!(
            ominous_spawn.color,
            [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0, 1.0]
        );
        assert!((25..=29).contains(&ominous_spawn.lifetime_ticks));
        assert_eq!(ominous_spawn.friction, 0.98);
        assert_eq!(ominous_spawn.gravity, 0.0);
        assert!(!ominous_spawn.has_physics);
        assert_eq!(
            ominous_spawn.tick_motion,
            ParticleTickMotionDescriptor::FlyStraightTowards
        );
        assert_eq!(ominous_spawn.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(
            ominous_spawn.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );

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

        let mut current_down_random = ParticleRandom::new(82);
        let mut current_down_command = spawn_command("minecraft:current_down", 1.0);
        current_down_command.velocity = [9.0, 9.0, 9.0];
        let current_down =
            ParticleInstance::from_spawn_command(current_down_command, &mut current_down_random);
        assert_eq!(current_down.provider, "WaterCurrentDownParticle.Provider");
        assert_eq!(
            current_down.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            current_down.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(current_down.base_quad_size, 0.02, 0.16);
        assert_eq!(current_down.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((30..=89).contains(&current_down.lifetime_ticks));
        assert_eq!(current_down.velocity, [0.0, -0.05, 0.0]);
        assert_eq!(current_down.friction, 0.98);
        assert_eq!(current_down.gravity, 0.002);
        assert!(!current_down.has_physics);
        assert_eq!(
            current_down.tick_motion,
            ParticleTickMotionDescriptor::CurrentDown
        );
        assert_eq!(current_down.render_layer, ParticleRenderLayer::Opaque);

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

        let mut dust_random = ParticleRandom::new(79);
        let mut dust_command = spawn_command("minecraft:dust", 1.0);
        dust_command.velocity = [1.0, 2.0, 3.0];
        dust_command.option_color = Some([0.25, 0.5, 0.75, 1.0]);
        dust_command.option_scale = Some(2.0);
        let dust = ParticleInstance::from_spawn_command(dust_command, &mut dust_random);
        assert_eq!(dust.provider, "DustParticle.Provider");
        assert_eq!(dust.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(
            dust.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
        assert_eq!(dust.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_range_f32(dust.base_quad_size, 0.15, 0.3);
        assert_range_f32(dust.color[0], 0.25 * 0.48, 0.25);
        assert_range_f32(dust.color[1], 0.5 * 0.48, 0.5);
        assert_range_f32(dust.color[2], 0.75 * 0.48, 0.75);
        assert_eq!(dust.color[3], 1.0);
        assert!((16..=80).contains(&dust.lifetime_ticks));
        assert_eq!(dust.option_scale, Some(2.0));
        assert_eq!(dust.render_layer, ParticleRenderLayer::Opaque);
        assert!(dust.speed_up_when_y_motion_is_blocked);

        let mut transition_random = ParticleRandom::new(80);
        let mut transition_command = spawn_command("minecraft:dust_color_transition", 1.0);
        transition_command.option_color = Some([0.0, 0.0, 1.0, 1.0]);
        transition_command.option_color_to = Some([1.0, 0.0, 0.0, 1.0]);
        transition_command.option_scale = Some(1.0);
        let mut transition =
            ParticleInstance::from_spawn_command(transition_command, &mut transition_random);
        assert_eq!(transition.provider, "DustColorTransitionParticle.Provider");
        assert!(transition.color_transition_target.is_some());
        transition.age_ticks = 10;
        transition.lifetime_ticks = 20;
        let target = transition.color_transition_target.unwrap();
        let tint = particle_render_color(&transition);
        let alpha = 10.5 / 21.0;
        assert_close_f32(tint[0], lerp_f32(alpha, transition.color[0], target[0]));
        assert_close_f32(tint[1], lerp_f32(alpha, transition.color[1], target[1]));
        assert_close_f32(tint[2], lerp_f32(alpha, transition.color[2], target[2]));

        let mut firefly_tint = test_instance_with_lifetime("minecraft:firefly", 100);
        firefly_tint.color = [1.0, 1.0, 1.0, 1.0];
        firefly_tint.age_ticks = 90;
        let tint = particle_render_color(&firefly_tint);
        assert_close_f32(tint[3], firefly_fade_amount(90.5 / 100.0, 0.3, 0.5));

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

        let mut spore_random = ParticleRandom::new(78);
        let spore_blossom_air = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:spore_blossom_air", 1.0),
            &mut spore_random,
        );
        assert_eq!(
            spore_blossom_air.provider,
            "SuspendedParticle.SporeBlossomAirProvider"
        );
        assert_eq!(
            spore_blossom_air.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(spore_blossom_air.previous_position, [1.0, -0.125, 0.0]);
        assert_eq!(spore_blossom_air.position, [1.0, -0.125, 0.0]);
        assert_eq!(
            spore_blossom_air.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(spore_blossom_air.base_quad_size, 0.06, 0.24);
        assert_eq!(spore_blossom_air.color, [0.32, 0.5, 0.22, 1.0]);
        assert!((500..=1000).contains(&spore_blossom_air.lifetime_ticks));
        assert_eq!(spore_blossom_air.velocity, [0.0, -0.8, 0.0]);
        assert_eq!(spore_blossom_air.friction, 1.0);
        assert_eq!(spore_blossom_air.gravity, 0.01);
        assert!(!spore_blossom_air.has_physics);
        assert_eq!(
            spore_blossom_air.particle_limit,
            Some(ParticleLimitDescriptor::SporeBlossom)
        );
        assert_eq!(spore_blossom_air.render_layer, ParticleRenderLayer::Opaque);

        let mut nectar_random = ParticleRandom::new(79);
        let falling_nectar = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_nectar", 1.0),
            &mut nectar_random,
        );
        assert_eq!(falling_nectar.provider, "DripParticle.NectarFallProvider");
        assert_eq!(
            falling_nectar.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            falling_nectar.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(falling_nectar.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_nectar.color, [0.92, 0.782, 0.72, 1.0]);
        assert!((16..=80).contains(&falling_nectar.lifetime_ticks));
        assert_eq!(falling_nectar.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_nectar.friction, 0.98);
        assert_eq!(falling_nectar.gravity, 0.007);
        assert!(falling_nectar.has_physics);
        assert_eq!(
            falling_nectar.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(falling_nectar.render_layer, ParticleRenderLayer::Opaque);

        let mut falling_spore_random = ParticleRandom::new(80);
        let falling_spore_blossom = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_spore_blossom", 1.0),
            &mut falling_spore_random,
        );
        assert_eq!(
            falling_spore_blossom.provider,
            "DripParticle.SporeBlossomFallProvider"
        );
        assert_eq!(
            falling_spore_blossom.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            falling_spore_blossom.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(falling_spore_blossom.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_spore_blossom.color, [0.32, 0.5, 0.22, 1.0]);
        assert!((71..=640).contains(&falling_spore_blossom.lifetime_ticks));
        assert_eq!(falling_spore_blossom.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_spore_blossom.friction, 0.98);
        assert_eq!(falling_spore_blossom.gravity, 0.005);
        assert!(falling_spore_blossom.has_physics);
        assert_eq!(
            falling_spore_blossom.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_spore_blossom.render_layer,
            ParticleRenderLayer::Opaque
        );

        let mut dripping_honey_random = ParticleRandom::new(81);
        let dripping_honey = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_honey", 1.0),
            &mut dripping_honey_random,
        );
        assert_eq!(dripping_honey.provider, "DripParticle.HoneyHangProvider");
        assert_eq!(
            dripping_honey.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            dripping_honey.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_range_f32(dripping_honey.base_quad_size, 0.1, 0.2);
        assert_eq!(dripping_honey.color, [0.622, 0.508, 0.082, 1.0]);
        assert_eq!(dripping_honey.lifetime_ticks, 100);
        assert_eq!(dripping_honey.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_honey.friction, 0.98);
        assert_eq!(dripping_honey.gravity, 0.000_012);
        assert!(dripping_honey.has_physics);
        assert_eq!(
            dripping_honey.tick_motion,
            ParticleTickMotionDescriptor::DripHang
        );
        assert_eq!(dripping_honey.render_layer, ParticleRenderLayer::Opaque);

        let mut falling_honey_random = ParticleRandom::new(82);
        let falling_honey = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_honey", 1.0),
            &mut falling_honey_random,
        );
        assert_eq!(falling_honey.provider, "DripParticle.HoneyFallProvider");
        assert_range_f32(falling_honey.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_honey.color, [0.582, 0.448, 0.082, 1.0]);
        assert!((64..=320).contains(&falling_honey.lifetime_ticks));
        assert_eq!(falling_honey.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_honey.friction, 0.98);
        assert_eq!(falling_honey.gravity, 0.01);
        assert_eq!(
            falling_honey.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(falling_honey.render_layer, ParticleRenderLayer::Opaque);

        let mut landing_honey_random = ParticleRandom::new(83);
        let landing_honey = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:landing_honey", 1.0),
            &mut landing_honey_random,
        );
        assert_eq!(landing_honey.provider, "DripParticle.HoneyLandProvider");
        assert_range_f32(landing_honey.base_quad_size, 0.1, 0.2);
        assert_eq!(landing_honey.color, [0.522, 0.408, 0.082, 1.0]);
        assert!((128..=640).contains(&landing_honey.lifetime_ticks));
        assert_eq!(landing_honey.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(landing_honey.friction, 0.98);
        assert_eq!(landing_honey.gravity, 0.06);
        assert_eq!(
            landing_honey.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(landing_honey.render_layer, ParticleRenderLayer::Opaque);

        let mut dripping_obsidian_random = ParticleRandom::new(84);
        let dripping_obsidian = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_obsidian_tear", 1.0),
            &mut dripping_obsidian_random,
        );
        assert_eq!(
            dripping_obsidian.provider,
            "DripParticle.ObsidianTearHangProvider"
        );
        assert_range_f32(dripping_obsidian.base_quad_size, 0.1, 0.2);
        assert_eq!(
            dripping_obsidian.color,
            [0.511_718_75, 0.031_25, 0.890_625, 1.0]
        );
        assert_eq!(dripping_obsidian.lifetime_ticks, 100);
        assert_eq!(dripping_obsidian.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_obsidian.friction, 0.98);
        assert_eq!(dripping_obsidian.gravity, 0.000_012);
        assert_eq!(
            dripping_obsidian.tick_motion,
            ParticleTickMotionDescriptor::DripHang
        );
        assert_eq!(
            dripping_obsidian.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(dripping_obsidian.render_layer, ParticleRenderLayer::Opaque);

        let mut falling_obsidian_random = ParticleRandom::new(85);
        let falling_obsidian = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_obsidian_tear", 1.0),
            &mut falling_obsidian_random,
        );
        assert_eq!(
            falling_obsidian.provider,
            "DripParticle.ObsidianTearFallProvider"
        );
        assert_range_f32(falling_obsidian.base_quad_size, 0.1, 0.2);
        assert_eq!(
            falling_obsidian.color,
            [0.511_718_75, 0.031_25, 0.890_625, 1.0]
        );
        assert!((64..=320).contains(&falling_obsidian.lifetime_ticks));
        assert_eq!(falling_obsidian.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_obsidian.friction, 0.98);
        assert_eq!(falling_obsidian.gravity, 0.01);
        assert_eq!(
            falling_obsidian.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_obsidian.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(falling_obsidian.render_layer, ParticleRenderLayer::Opaque);

        let mut landing_obsidian_random = ParticleRandom::new(86);
        let landing_obsidian = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:landing_obsidian_tear", 1.0),
            &mut landing_obsidian_random,
        );
        assert_eq!(
            landing_obsidian.provider,
            "DripParticle.ObsidianTearLandProvider"
        );
        assert_range_f32(landing_obsidian.base_quad_size, 0.1, 0.2);
        assert_eq!(
            landing_obsidian.color,
            [0.511_718_75, 0.031_25, 0.890_625, 1.0]
        );
        assert!((28..=140).contains(&landing_obsidian.lifetime_ticks));
        assert_eq!(landing_obsidian.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(landing_obsidian.friction, 0.98);
        assert_eq!(landing_obsidian.gravity, 0.06);
        assert_eq!(
            landing_obsidian.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            landing_obsidian.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(landing_obsidian.render_layer, ParticleRenderLayer::Opaque);

        let mut dripping_lava_random = ParticleRandom::new(87);
        let dripping_lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_lava", 1.0),
            &mut dripping_lava_random,
        );
        assert_eq!(dripping_lava.provider, "DripParticle.LavaHangProvider");
        assert_range_f32(dripping_lava.base_quad_size, 0.1, 0.2);
        assert_eq!(dripping_lava.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(dripping_lava.lifetime_ticks, 40);
        assert_eq!(dripping_lava.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_lava.friction, 0.98);
        assert_eq!(dripping_lava.gravity, 0.0012);
        assert_eq!(
            dripping_lava.tick_motion,
            ParticleTickMotionDescriptor::CoolingDripHang
        );
        assert_eq!(
            dripping_lava.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(dripping_lava.render_layer, ParticleRenderLayer::Opaque);

        let mut falling_lava_random = ParticleRandom::new(88);
        let falling_lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_lava", 1.0),
            &mut falling_lava_random,
        );
        assert_eq!(falling_lava.provider, "DripParticle.LavaFallProvider");
        assert_range_f32(falling_lava.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_lava.color, [1.0, 0.285_714_3, 0.083_333_336, 1.0]);
        assert!((64..=320).contains(&falling_lava.lifetime_ticks));
        assert_eq!(falling_lava.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_lava.friction, 0.98);
        assert_eq!(falling_lava.gravity, 0.06);
        assert_eq!(
            falling_lava.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_lava.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(falling_lava.render_layer, ParticleRenderLayer::Opaque);

        let mut landing_lava_random = ParticleRandom::new(89);
        let landing_lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:landing_lava", 1.0),
            &mut landing_lava_random,
        );
        assert_eq!(landing_lava.provider, "DripParticle.LavaLandProvider");
        assert_range_f32(landing_lava.base_quad_size, 0.1, 0.2);
        assert_eq!(landing_lava.color, [1.0, 0.285_714_3, 0.083_333_336, 1.0]);
        assert!((16..=80).contains(&landing_lava.lifetime_ticks));
        assert_eq!(landing_lava.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(landing_lava.friction, 0.98);
        assert_eq!(landing_lava.gravity, 0.06);
        assert_eq!(
            landing_lava.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            landing_lava.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(landing_lava.render_layer, ParticleRenderLayer::Opaque);

        let mut dripping_water_random = ParticleRandom::new(90);
        let dripping_water = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_water", 1.0),
            &mut dripping_water_random,
        );
        assert_eq!(dripping_water.provider, "DripParticle.WaterHangProvider");
        assert_range_f32(dripping_water.base_quad_size, 0.1, 0.2);
        assert_eq!(dripping_water.color, [0.2, 0.3, 1.0, 1.0]);
        assert_eq!(dripping_water.lifetime_ticks, 40);
        assert_eq!(dripping_water.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_water.friction, 0.98);
        assert_eq!(dripping_water.gravity, 0.0012);
        assert_eq!(
            dripping_water.tick_motion,
            ParticleTickMotionDescriptor::DripHang
        );
        assert_eq!(
            dripping_water.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(dripping_water.render_layer, ParticleRenderLayer::Opaque);

        let mut falling_water_random = ParticleRandom::new(91);
        let falling_water = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_water", 1.0),
            &mut falling_water_random,
        );
        assert_eq!(falling_water.provider, "DripParticle.WaterFallProvider");
        assert_range_f32(falling_water.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_water.color, [0.2, 0.3, 1.0, 1.0]);
        assert!((64..=320).contains(&falling_water.lifetime_ticks));
        assert_eq!(falling_water.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_water.friction, 0.98);
        assert_eq!(falling_water.gravity, 0.06);
        assert_eq!(
            falling_water.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_water.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(falling_water.render_layer, ParticleRenderLayer::Opaque);

        let mut dripping_dripstone_lava_random = ParticleRandom::new(92);
        let dripping_dripstone_lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_dripstone_lava", 1.0),
            &mut dripping_dripstone_lava_random,
        );
        assert_eq!(
            dripping_dripstone_lava.provider,
            "DripParticle.DripstoneLavaHangProvider"
        );
        assert_range_f32(dripping_dripstone_lava.base_quad_size, 0.1, 0.2);
        assert_eq!(dripping_dripstone_lava.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(dripping_dripstone_lava.lifetime_ticks, 40);
        assert_eq!(dripping_dripstone_lava.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_dripstone_lava.friction, 0.98);
        assert_eq!(dripping_dripstone_lava.gravity, 0.0012);
        assert_eq!(
            dripping_dripstone_lava.tick_motion,
            ParticleTickMotionDescriptor::CoolingDripHang
        );
        assert_eq!(
            dripping_dripstone_lava.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(
            dripping_dripstone_lava.render_layer,
            ParticleRenderLayer::Opaque
        );

        let mut falling_dripstone_lava_random = ParticleRandom::new(93);
        let falling_dripstone_lava = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_dripstone_lava", 1.0),
            &mut falling_dripstone_lava_random,
        );
        assert_eq!(
            falling_dripstone_lava.provider,
            "DripParticle.DripstoneLavaFallProvider"
        );
        assert_range_f32(falling_dripstone_lava.base_quad_size, 0.1, 0.2);
        assert_eq!(
            falling_dripstone_lava.color,
            [1.0, 0.285_714_3, 0.083_333_336, 1.0]
        );
        assert!((64..=320).contains(&falling_dripstone_lava.lifetime_ticks));
        assert_eq!(falling_dripstone_lava.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_dripstone_lava.friction, 0.98);
        assert_eq!(falling_dripstone_lava.gravity, 0.06);
        assert_eq!(
            falling_dripstone_lava.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_dripstone_lava.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(
            falling_dripstone_lava.render_layer,
            ParticleRenderLayer::Opaque
        );

        let mut dripping_dripstone_water_random = ParticleRandom::new(94);
        let dripping_dripstone_water = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dripping_dripstone_water", 1.0),
            &mut dripping_dripstone_water_random,
        );
        assert_eq!(
            dripping_dripstone_water.provider,
            "DripParticle.DripstoneWaterHangProvider"
        );
        assert_range_f32(dripping_dripstone_water.base_quad_size, 0.1, 0.2);
        assert_eq!(dripping_dripstone_water.color, [0.2, 0.3, 1.0, 1.0]);
        assert_eq!(dripping_dripstone_water.lifetime_ticks, 40);
        assert_eq!(dripping_dripstone_water.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(dripping_dripstone_water.friction, 0.98);
        assert_eq!(dripping_dripstone_water.gravity, 0.0012);
        assert_eq!(
            dripping_dripstone_water.tick_motion,
            ParticleTickMotionDescriptor::DripHang
        );
        assert_eq!(
            dripping_dripstone_water.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(
            dripping_dripstone_water.render_layer,
            ParticleRenderLayer::Opaque
        );

        let mut falling_dripstone_water_random = ParticleRandom::new(95);
        let falling_dripstone_water = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_dripstone_water", 1.0),
            &mut falling_dripstone_water_random,
        );
        assert_eq!(
            falling_dripstone_water.provider,
            "DripParticle.DripstoneWaterFallProvider"
        );
        assert_range_f32(falling_dripstone_water.base_quad_size, 0.1, 0.2);
        assert_eq!(falling_dripstone_water.color, [0.2, 0.3, 1.0, 1.0]);
        assert!((64..=320).contains(&falling_dripstone_water.lifetime_ticks));
        assert_eq!(falling_dripstone_water.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(falling_dripstone_water.friction, 0.98);
        assert_eq!(falling_dripstone_water.gravity, 0.06);
        assert_eq!(
            falling_dripstone_water.tick_motion,
            ParticleTickMotionDescriptor::WaterDrop
        );
        assert_eq!(
            falling_dripstone_water.light_emission,
            ParticleLightEmissionDescriptor::World
        );
        assert_eq!(
            falling_dripstone_water.render_layer,
            ParticleRenderLayer::Opaque
        );

        let mut crimson_random = ParticleRandom::new(46);
        let crimson_spore = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:crimson_spore", 1.0),
            &mut crimson_random,
        );
        assert_eq!(
            crimson_spore.provider,
            "SuspendedParticle.CrimsonSporeProvider"
        );
        assert_eq!(
            crimson_spore.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(crimson_spore.previous_position, [1.0, -0.125, 0.0]);
        assert_eq!(crimson_spore.position, [1.0, -0.125, 0.0]);
        assert_range_f32(crimson_spore.base_quad_size, 0.06, 0.24);
        assert_eq!(crimson_spore.color, [0.9, 0.4, 0.5, 1.0]);
        assert!((16..=80).contains(&crimson_spore.lifetime_ticks));
        assert_close_f64(crimson_spore.velocity[0], 1.3558214650566454E-6);
        assert_close_f64(crimson_spore.velocity[1], -0.8270729973920494E-4);
        assert_close_f64(crimson_spore.velocity[2], 1.6065611415614136E-6);
        assert_eq!(crimson_spore.friction, 1.0);
        assert_eq!(crimson_spore.gravity, 0.0);
        assert!(!crimson_spore.has_physics);
        assert_eq!(crimson_spore.render_layer, ParticleRenderLayer::Opaque);

        let mut warped_random = ParticleRandom::new(47);
        let warped_spore = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:warped_spore", 1.0),
            &mut warped_random,
        );
        assert_eq!(
            warped_spore.provider,
            "SuspendedParticle.WarpedSporeProvider"
        );
        assert_eq!(warped_spore.previous_position, [1.0, -0.125, 0.0]);
        assert_eq!(warped_spore.position, [1.0, -0.125, 0.0]);
        assert_range_f32(warped_spore.base_quad_size, 0.06, 0.24);
        assert_eq!(warped_spore.color, [0.1, 0.1, 0.3, 1.0]);
        assert!((16..=80).contains(&warped_spore.lifetime_ticks));
        assert_close_f64(warped_spore.velocity[0], 0.0);
        assert_close_f64(warped_spore.velocity[1], -0.055236806630186874);
        assert_close_f64(warped_spore.velocity[2], 0.0);
        assert_eq!(warped_spore.friction, 1.0);
        assert_eq!(warped_spore.gravity, 0.0);
        assert!(!warped_spore.has_physics);
        assert_eq!(warped_spore.render_layer, ParticleRenderLayer::Opaque);

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
        assert_eq!(snowflake.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(
            snowflake.tick_motion,
            ParticleTickMotionDescriptor::Snowflake
        );

        let mut squid_ink_random = ParticleRandom::new(57);
        let mut squid_ink_command = spawn_command("minecraft:squid_ink", 1.0);
        squid_ink_command.velocity = [1.0, 2.0, 3.0];
        let squid_ink =
            ParticleInstance::from_spawn_command(squid_ink_command, &mut squid_ink_random);
        assert_eq!(squid_ink.provider, "SquidInkParticle.Provider");
        assert_eq!(squid_ink.sprite_selection, ParticleSpriteSelection::Age);
        assert_close_f32(squid_ink.base_quad_size, 0.5);
        assert_eq!(squid_ink.color, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(squid_ink.quad_size_curve, ParticleQuadSizeCurve::Constant);
        assert!((6..=30).contains(&squid_ink.lifetime_ticks));
        assert_eq!(squid_ink.velocity, [1.0, 2.0, 3.0]);
        assert_eq!(squid_ink.friction, 0.92);
        assert_eq!(squid_ink.gravity, 0.0);
        assert!(!squid_ink.has_physics);
        assert_eq!(squid_ink.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(
            squid_ink.alpha_curve,
            ParticleAlphaCurve::SimpleAnimatedFade
        );

        let mut glow_ink_random = ParticleRandom::new(58);
        let glow_ink = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:glow_squid_ink", 1.0),
            &mut glow_ink_random,
        );
        assert_eq!(glow_ink.provider, "SquidInkParticle.GlowInkProvider");
        assert_close_f32(glow_ink.base_quad_size, 0.5);
        assert_eq!(glow_ink.color, [0.2, 0.8, 0.6, 1.0]);
        assert!((6..=30).contains(&glow_ink.lifetime_ticks));
        assert!(!glow_ink.has_physics);
        assert_eq!(glow_ink.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(glow_ink.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);

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

        let mut flash_random = ParticleRandom::new(66);
        let mut flash_command = spawn_command("minecraft:flash", 1.0);
        flash_command.option_color = Some([0.1, 0.2, 0.3, 0.4]);
        let flash = ParticleInstance::from_spawn_command(flash_command, &mut flash_random);
        assert_eq!(flash.provider, "FireworkParticles.FlashProvider");
        assert_eq!(flash.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(flash.lifetime_ticks, 4);
        assert_eq!(flash.color, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(flash.quad_size_curve, ParticleQuadSizeCurve::FlashOverlay);
        assert_eq!(flash.alpha_curve, ParticleAlphaCurve::FlashOverlayFade);
        assert_eq!(flash.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(flash.render_layer, ParticleRenderLayer::Translucent);

        let mut trail_random = ParticleRandom::new(67);
        let mut trail_command = spawn_command("minecraft:trail", 1.0);
        trail_command.velocity = [0.1, 0.2, 0.3];
        trail_command.option_color = Some([0.2, 0.4, 0.8, 1.0]);
        trail_command.option_target = Some([4.0, 6.0, 8.0]);
        trail_command.option_duration_ticks = Some(12);
        let mut expected_trail_random = ParticleRandom::new(67);
        let _ = select_initial_sprite(
            &trail_command.sprite_ids,
            ParticleSpriteSelection::Random,
            &mut expected_trail_random,
        );
        let _ = expected_trail_random.next_f32();
        let expected_trail_color = [
            0.2 * (0.875 + expected_trail_random.next_f32() * 0.25),
            0.4 * (0.875 + expected_trail_random.next_f32() * 0.25),
            0.8 * (0.875 + expected_trail_random.next_f32() * 0.25),
            1.0,
        ];
        let trail = ParticleInstance::from_spawn_command(trail_command, &mut trail_random);
        assert_eq!(trail.provider, "TrailParticle.Provider");
        assert_eq!(
            trail.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
        assert_eq!(trail.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(trail.lifetime_ticks, 12);
        assert_close_f32(trail.base_quad_size, 0.26);
        assert_close3_f32(
            [trail.color[0], trail.color[1], trail.color[2]],
            [
                expected_trail_color[0],
                expected_trail_color[1],
                expected_trail_color[2],
            ],
        );
        assert_eq!(trail.color[3], expected_trail_color[3]);
        assert_eq!(trail.option_target, Some([4.0, 6.0, 8.0]));
        assert_eq!(trail.option_duration_ticks, Some(12));
        assert_eq!(trail.velocity, [0.1, 0.2, 0.3]);
        assert_eq!(trail.tick_motion, ParticleTickMotionDescriptor::TrailTarget);
        assert_eq!(
            trail.light_emission,
            ParticleLightEmissionDescriptor::FullBright
        );
        assert_eq!(particle_light_with_emission(&trail, [0.2, 0.3]), [1.0, 1.0]);
        assert_eq!(trail.render_layer, ParticleRenderLayer::Opaque);

        let mut vibration_random = ParticleRandom::new(68);
        let mut vibration_command = spawn_command("minecraft:vibration", 1.0);
        vibration_command.position = [1.0, 2.0, 3.0];
        vibration_command.velocity = [9.0, 9.0, 9.0];
        vibration_command.option_target = Some([4.0, 6.0, 8.0]);
        vibration_command.option_duration_ticks = Some(20);
        let vibration =
            ParticleInstance::from_spawn_command(vibration_command, &mut vibration_random);
        let (yaw, pitch) = vibration_particle_angles([1.0, 2.0, 3.0], [4.0, 6.0, 8.0]);
        assert_eq!(vibration.provider, "VibrationSignalParticle.Provider");
        assert_eq!(
            vibration.current_sprite_id.as_deref(),
            Some("minecraft:generic_0")
        );
        assert_eq!(vibration.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(vibration.lifetime_ticks, 20);
        assert_close_f32(vibration.base_quad_size, 0.3);
        assert_eq!(vibration.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(vibration.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(
            vibration.tick_motion,
            ParticleTickMotionDescriptor::VibrationSignal
        );
        assert_eq!(vibration.option_target, Some([4.0, 6.0, 8.0]));
        assert_eq!(vibration.option_duration_ticks, Some(20));
        assert_close_f32(vibration.previous_yaw, yaw);
        assert_close_f32(vibration.yaw, yaw);
        assert_close_f32(vibration.previous_pitch, pitch);
        assert_close_f32(vibration.pitch, pitch);
        assert_eq!(
            vibration.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(
            particle_light_with_emission(&vibration, [0.2, 0.3]),
            [1.0, 0.3]
        );
        assert_eq!(vibration.render_layer, ParticleRenderLayer::Translucent);

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

        let mut base_effect_random = ParticleRandom::new(63);
        let mut base_effect_command = spawn_command("minecraft:effect", 1.0);
        base_effect_command.velocity = [1.0, 1.0, 0.0];
        let base_effect =
            ParticleInstance::from_spawn_command(base_effect_command, &mut base_effect_random);
        let mut powered_effect_random = ParticleRandom::new(63);
        let mut powered_effect_command = spawn_command("minecraft:effect", 1.0);
        powered_effect_command.velocity = [1.0, 1.0, 0.0];
        powered_effect_command.option_color = Some([0.2, 0.4, 0.6, 1.0]);
        powered_effect_command.option_power = Some(0.5);
        let powered_effect = ParticleInstance::from_spawn_command(
            powered_effect_command,
            &mut powered_effect_random,
        );
        assert_eq!(powered_effect.provider, "SpellParticle.InstantProvider");
        assert_eq!(
            powered_effect.render_layer,
            ParticleRenderLayer::Translucent
        );
        assert_eq!(powered_effect.color, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(powered_effect.option_power, Some(0.5));
        assert_close_f64(powered_effect.velocity[0], base_effect.velocity[0] * 0.5);
        assert_close_f64(
            powered_effect.velocity[1],
            (base_effect.velocity[1] - 0.1) * 0.5 + 0.1,
        );
        assert_close_f64(powered_effect.velocity[2], base_effect.velocity[2] * 0.5);

        let mut entity_effect_random = ParticleRandom::new(64);
        let mut entity_effect_command = spawn_command("minecraft:entity_effect", 1.0);
        entity_effect_command.option_color = Some([0.1, 0.2, 0.3, 0.4]);
        let entity_effect =
            ParticleInstance::from_spawn_command(entity_effect_command, &mut entity_effect_random);
        assert_eq!(entity_effect.provider, "SpellParticle.MobEffectProvider");
        assert_eq!(entity_effect.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(entity_effect.color, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(entity_effect.option_power, None);

        let mut instant_effect_random = ParticleRandom::new(65);
        let mut instant_effect_command = spawn_command("minecraft:instant_effect", 1.0);
        instant_effect_command.option_color = Some([0.9, 0.8, 0.7, 1.0]);
        instant_effect_command.option_power = Some(1.25);
        let instant_effect = ParticleInstance::from_spawn_command(
            instant_effect_command,
            &mut instant_effect_random,
        );
        assert_eq!(instant_effect.provider, "SpellParticle.InstantProvider");
        assert_eq!(instant_effect.color, [0.9, 0.8, 0.7, 1.0]);

        let mut pause_random = ParticleRandom::new(59);
        let mut pause_command = spawn_command("minecraft:pause_mob_growth", 1.0);
        pause_command.velocity = [1.0, 2.0, 3.0];
        let pause_growth = ParticleInstance::from_spawn_command(pause_command, &mut pause_random);
        assert_eq!(
            pause_growth.provider,
            "SimpleVerticalParticle.PauseMobGrowthProvider"
        );
        assert_eq!(
            pause_growth.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_range_f32(pause_growth.base_quad_size, 0.05, 0.22);
        assert_eq!(pause_growth.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            pause_growth.quad_size_curve,
            ParticleQuadSizeCurve::Constant
        );
        assert_eq!(pause_growth.lifetime_ticks, 8);
        assert_eq!(pause_growth.velocity, [1.0, 1.97, 3.0]);
        assert_eq!(pause_growth.friction, 0.98);
        assert_eq!(pause_growth.gravity, 0.0);
        assert!(pause_growth.has_physics);
        assert_eq!(pause_growth.render_layer, ParticleRenderLayer::Opaque);

        let mut reset_random = ParticleRandom::new(60);
        let mut reset_command = spawn_command("minecraft:reset_mob_growth", 1.0);
        reset_command.velocity = [1.0, 2.0, 3.0];
        let reset_growth = ParticleInstance::from_spawn_command(reset_command, &mut reset_random);
        assert_eq!(
            reset_growth.provider,
            "SimpleVerticalParticle.ResetMobGrowthProvider"
        );
        assert_eq!(reset_growth.lifetime_ticks, 8);
        assert_eq!(reset_growth.velocity, [1.0, 2.03, 3.0]);
        assert!(reset_growth.has_physics);
        assert_eq!(reset_growth.render_layer, ParticleRenderLayer::Opaque);

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

        let mut enchant_random = ParticleRandom::new(83);
        let mut enchant_command = spawn_command("minecraft:enchant", 1.0);
        enchant_command.position = [1.0, 2.0, 3.0];
        enchant_command.velocity = [0.5, 1.0, -0.25];
        let enchant = ParticleInstance::from_spawn_command(enchant_command, &mut enchant_random);
        assert_eq!(
            enchant.provider,
            "FlyTowardsPositionParticle.EnchantProvider"
        );
        assert_eq!(enchant.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(enchant.start_position, [1.0, 2.0, 3.0]);
        assert_eq!(enchant.previous_position, [1.5, 3.0, 2.75]);
        assert_eq!(enchant.position, [1.5, 3.0, 2.75]);
        assert_eq!(enchant.velocity, [0.5, 1.0, -0.25]);
        assert_range_f32(enchant.base_quad_size, 0.02, 0.07);
        assert_close_f32(enchant.color[0], enchant.color[2] * 0.9);
        assert_close_f32(enchant.color[1], enchant.color[2] * 0.9);
        assert_eq!(enchant.color[3], 1.0);
        assert!((30..=39).contains(&enchant.lifetime_ticks));
        assert!(!enchant.has_physics);
        assert_eq!(
            enchant.tick_motion,
            ParticleTickMotionDescriptor::FlyTowardsPosition
        );
        assert_eq!(
            enchant.light_emission,
            ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
        );

        let mut nautilus_random = ParticleRandom::new(84);
        let mut nautilus_command = spawn_command("minecraft:nautilus", 1.0);
        nautilus_command.position = [1.0, 2.0, 3.0];
        nautilus_command.velocity = [-0.25, 0.5, 1.25];
        let nautilus = ParticleInstance::from_spawn_command(nautilus_command, &mut nautilus_random);
        assert_eq!(
            nautilus.provider,
            "FlyTowardsPositionParticle.NautilusProvider"
        );
        assert_eq!(nautilus.start_position, [1.0, 2.0, 3.0]);
        assert_eq!(nautilus.previous_position, [0.75, 2.5, 4.25]);
        assert_eq!(nautilus.position, [0.75, 2.5, 4.25]);
        assert_range_f32(nautilus.base_quad_size, 0.02, 0.07);
        assert!((30..=39).contains(&nautilus.lifetime_ticks));

        let mut vault_random = ParticleRandom::new(86);
        let mut vault_command = spawn_command("minecraft:vault_connection", 1.0);
        vault_command.position = [1.0, 2.0, 3.0];
        vault_command.velocity = [0.25, -0.5, 0.75];
        let vault = ParticleInstance::from_spawn_command(vault_command, &mut vault_random);
        assert_eq!(
            vault.provider,
            "FlyTowardsPositionParticle.VaultConnectionProvider"
        );
        assert_eq!(vault.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(vault.start_position, [1.0, 2.0, 3.0]);
        assert_eq!(vault.previous_position, [1.25, 1.5, 3.75]);
        assert_eq!(vault.position, [1.25, 1.5, 3.75]);
        assert_eq!(vault.velocity, [0.25, -0.5, 0.75]);
        assert_range_f32(vault.base_quad_size, 0.03, 0.105);
        assert_close_f32(vault.color[0], vault.color[2] * 0.9);
        assert_close_f32(vault.color[1], vault.color[2] * 0.9);
        assert_eq!(vault.color[3], 0.0);
        assert!((30..=39).contains(&vault.lifetime_ticks));
        assert!(!vault.has_physics);
        assert_eq!(
            vault.tick_motion,
            ParticleTickMotionDescriptor::FlyTowardsPosition
        );
        assert_eq!(vault.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(vault.alpha_curve, ParticleAlphaCurve::VaultConnectionFade);
        assert_eq!(
            vault.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );

        let mut totem_random = ParticleRandom::new(85);
        let mut totem_command = spawn_command("minecraft:totem_of_undying", 1.0);
        totem_command.velocity = [0.25, 0.5, -0.75];
        let totem = ParticleInstance::from_spawn_command(totem_command, &mut totem_random);
        assert_eq!(totem.provider, "TotemParticle.Provider");
        assert_eq!(totem.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(totem.current_sprite_index, Some(0));
        assert_range_f32(totem.base_quad_size, 0.075, 0.15);
        assert!((0.1..=0.3).contains(&totem.color[0]) || (0.6..=0.8).contains(&totem.color[0]));
        assert_range_f32(totem.color[1], 0.4, 0.9);
        assert_range_f32(totem.color[2], 0.0, 0.2);
        assert_eq!(totem.color[3], 1.0);
        assert!((60..=71).contains(&totem.lifetime_ticks));
        assert_eq!(totem.velocity, [0.25, 0.5, -0.75]);
        assert_eq!(totem.friction, 0.6);
        assert_eq!(totem.gravity, 1.25);
        assert!(totem.has_physics);
        assert_eq!(totem.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(
            totem.light_emission,
            ParticleLightEmissionDescriptor::FullBright
        );
        assert_eq!(totem.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);

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
        assert_eq!(end_rod.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(end_rod.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);
        assert_eq!(
            end_rod.color_fade_target,
            Some(descriptors::END_ROD_FADE_COLOR)
        );

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

        let mut portal_random = ParticleRandom::new(80);
        let mut portal_command = spawn_command("minecraft:portal", 1.0);
        portal_command.velocity = [-5.0, 0.0, 5.0];
        let portal = ParticleInstance::from_spawn_command(portal_command, &mut portal_random);
        assert_eq!(portal.provider, "PortalParticle.Provider");
        assert_eq!(portal.sprite_selection, ParticleSpriteSelection::Random);
        assert_eq!(portal.quad_size_curve, ParticleQuadSizeCurve::Portal);
        assert_range_f32(portal.base_quad_size, 0.05, 0.07);
        assert_close_f32(portal.color[0], portal.color[2] * 0.9);
        assert_close_f32(portal.color[1], portal.color[2] * 0.3);
        assert_eq!(portal.color[3], 1.0);
        assert!((40..=49).contains(&portal.lifetime_ticks));
        assert_eq!(portal.velocity, [-5.0, 0.0, 5.0]);
        assert_eq!(portal.friction, 0.98);
        assert_eq!(portal.gravity, 0.0);
        assert!(!portal.has_physics);
        assert_eq!(portal.tick_motion, ParticleTickMotionDescriptor::Portal);
        assert_eq!(
            portal.light_emission,
            ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
        );

        let mut reverse_portal_random = ParticleRandom::new(81);
        let mut reverse_portal_command = spawn_command("minecraft:reverse_portal", 1.0);
        reverse_portal_command.velocity = [-5.0, 0.0, 5.0];
        let reverse_portal = ParticleInstance::from_spawn_command(
            reverse_portal_command,
            &mut reverse_portal_random,
        );
        assert_eq!(
            reverse_portal.provider,
            "ReversePortalParticle.ReversePortalProvider"
        );
        assert_eq!(
            reverse_portal.sprite_selection,
            ParticleSpriteSelection::Random
        );
        assert_eq!(
            reverse_portal.quad_size_curve,
            ParticleQuadSizeCurve::ReversePortal
        );
        assert_range_f32(reverse_portal.base_quad_size, 0.075, 0.105);
        assert_close_f32(reverse_portal.color[0], reverse_portal.color[2] * 0.9);
        assert_close_f32(reverse_portal.color[1], reverse_portal.color[2] * 0.3);
        assert_eq!(reverse_portal.color[3], 1.0);
        assert!((60..=61).contains(&reverse_portal.lifetime_ticks));
        assert_eq!(reverse_portal.velocity, [-5.0, 0.0, 5.0]);
        assert_eq!(
            reverse_portal.tick_motion,
            ParticleTickMotionDescriptor::ReversePortal
        );
        assert_eq!(
            reverse_portal.light_emission,
            ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
        );

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

        let mut explosion_emitter_command = spawn_command("minecraft:explosion_emitter", 1.0);
        explosion_emitter_command.child_spawn_templates = vec![ParticleChildSpawnTemplate {
            particle_type_id: 23,
            particle_id: "minecraft:explosion".to_string(),
            sprite_ids: vec!["minecraft:explosion_0".to_string()],
        }];
        let explosion_emitter =
            ParticleInstance::from_spawn_command(explosion_emitter_command, &mut explosion_random);
        assert_eq!(
            explosion_emitter.provider,
            "HugeExplosionSeedParticle.Provider"
        );
        assert_eq!(
            explosion_emitter.render_group,
            ParticleRenderGroup::NoRender
        );
        assert_eq!(explosion_emitter.lifetime_ticks, 8);
        assert_eq!(explosion_emitter.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(
            explosion_emitter.child_emission,
            Some(ParticleChildEmissionDescriptor::HugeExplosionSeed)
        );

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
        sculk_charge_command.option_roll = Some(0.75);
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
        assert_eq!(sculk_charge.option_roll, Some(0.75));
        assert_eq!(sculk_charge.previous_roll, 0.75);
        assert_eq!(sculk_charge.roll, 0.75);

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

        let mut firefly_random = ParticleRandom::new(75);
        let mut firefly_command = spawn_command("minecraft:firefly", 1.0);
        firefly_command.velocity = [0.0, 0.25, 0.0];
        let firefly = ParticleInstance::from_spawn_command(firefly_command, &mut firefly_random);
        assert_eq!(firefly.provider, "FireflyParticle.FireflyProvider");
        assert_eq!(firefly.sprite_selection, ParticleSpriteSelection::Random);
        assert_range_f32(firefly.base_quad_size, 0.1125, 0.225);
        assert_eq!(firefly.color, [1.0, 1.0, 1.0, 0.0]);
        assert!((200..=300).contains(&firefly.lifetime_ticks));
        assert_range_f64(firefly.velocity[0], -0.15, 0.15);
        assert_range_f64(firefly.velocity[1], -0.07, 0.23);
        assert_range_f64(firefly.velocity[2], -0.15, 0.15);
        assert_eq!(firefly.friction, 0.96);
        assert!(firefly.has_physics);
        assert!(firefly.speed_up_when_y_motion_is_blocked);
        assert_eq!(firefly.tick_motion, ParticleTickMotionDescriptor::Firefly);
        assert_eq!(firefly.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(firefly.alpha_curve, ParticleAlphaCurve::FireflyFade);
        assert_eq!(
            firefly.light_emission,
            ParticleLightEmissionDescriptor::Firefly
        );

        let mut shriek_random = ParticleRandom::new(76);
        let mut shriek_command = spawn_command("minecraft:shriek", 1.0);
        shriek_command.velocity = [1.0, 2.0, 3.0];
        shriek_command.initial_delay_ticks = 15;
        let shriek = ParticleInstance::from_spawn_command(shriek_command, &mut shriek_random);
        assert_eq!(shriek.provider, "ShriekParticle.Provider");
        assert_eq!(shriek.sprite_selection, ParticleSpriteSelection::Random);
        assert_close_f32(shriek.base_quad_size, 0.85);
        assert_eq!(shriek.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(shriek.quad_size_curve, ParticleQuadSizeCurve::Shriek);
        assert_eq!(shriek.alpha_curve, ParticleAlphaCurve::ShriekFade);
        assert_eq!(
            shriek.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(shriek.lifetime_ticks, 30);
        assert_eq!(shriek.velocity, [0.0, 0.1, 0.0]);
        assert_eq!(shriek.friction, 0.98);
        assert!(shriek.has_physics);
        assert_eq!(shriek.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(shriek.delay_ticks, 15);

        let mut detection_random = ParticleRandom::new(82);
        let mut detection_command = spawn_command("minecraft:trial_spawner_detection", 1.0);
        detection_command.velocity = [0.25, 0.5, -0.75];
        let detection =
            ParticleInstance::from_spawn_command(detection_command, &mut detection_random);
        assert_eq!(detection.provider, "TrialSpawnerDetectionParticle.Provider");
        assert_eq!(detection.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(detection.base_quad_size, 0.1125, 0.225);
        assert_eq!(detection.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(detection.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
        assert_eq!(
            detection.light_emission,
            ParticleLightEmissionDescriptor::FullBlock
        );
        assert_eq!(
            detection.facing_camera_mode,
            ParticleFacingCameraMode::LookAtY
        );
        assert!((12..=24).contains(&detection.lifetime_ticks));
        // TrialSpawnerDetectionParticle scales the base spread per axis by
        // (0.0, 0.9, 0.0) and threads the command velocity through with no offset,
        // so x/z drop the base spread and pass straight through while y keeps the
        // 0.9-scaled upward drift on top of the command y.
        let detection_spread = expected_base_ash_smoke_velocity(82, [0.0, 0.9, 0.0], false);
        assert_close_f64(detection.velocity[0], detection_spread[0] + 0.25);
        assert_close_f64(detection.velocity[1], detection_spread[1] + 0.5);
        assert_close_f64(detection.velocity[2], detection_spread[2] - 0.75);
        assert_close_f64(detection.velocity[0], 0.25);
        assert_close_f64(detection.velocity[2], -0.75);
        assert_eq!(detection.friction, 0.96);
        assert_eq!(detection.gravity, -0.1);
        assert!(detection.has_physics);
        assert!(detection.speed_up_when_y_motion_is_blocked);
        assert_eq!(detection.render_layer, ParticleRenderLayer::Opaque);

        let mut dust_plume_random = ParticleRandom::new(86);
        let mut dust_plume_command = spawn_command("minecraft:dust_plume", 1.0);
        dust_plume_command.velocity = [0.25, 0.5, -0.75];
        let dust_plume =
            ParticleInstance::from_spawn_command(dust_plume_command, &mut dust_plume_random);
        assert_eq!(dust_plume.provider, "DustPlumeParticle.Provider");
        assert_eq!(dust_plume.sprite_selection, ParticleSpriteSelection::Age);
        assert_range_f32(dust_plume.base_quad_size, 0.075, 0.15);
        assert_range_f32(dust_plume.color[0], 186.0 / 255.0 - 0.2, 186.0 / 255.0);
        assert_eq!(
            dust_plume.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );
        assert!((7..=35).contains(&dust_plume.lifetime_ticks));
        // DustPlumeParticle scales the base spread per axis by (0.7, 0.6, 0.7) and
        // adds the command velocity with +0.15 on y.
        let dust_plume_spread = expected_base_ash_smoke_velocity(86, [0.7, 0.6, 0.7], false);
        assert_close_f64(dust_plume.velocity[0], dust_plume_spread[0] + 0.25);
        assert_close_f64(dust_plume.velocity[1], dust_plume_spread[1] + 0.65);
        assert_close_f64(dust_plume.velocity[2], dust_plume_spread[2] - 0.75);
        assert_eq!(dust_plume.friction, 0.96);
        assert_eq!(dust_plume.gravity, 0.5);
        assert!(!dust_plume.has_physics);
        assert!(dust_plume.speed_up_when_y_motion_is_blocked);
        assert_eq!(
            dust_plume.tick_motion,
            ParticleTickMotionDescriptor::DustPlume
        );
        assert_eq!(dust_plume.render_layer, ParticleRenderLayer::Opaque);

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

        let mut gust_emitter_large_command = spawn_command("minecraft:gust_emitter_large", 1.0);
        gust_emitter_large_command.child_spawn_templates = vec![ParticleChildSpawnTemplate {
            particle_type_id: 24,
            particle_id: "minecraft:gust".to_string(),
            sprite_ids: vec!["minecraft:gust_0".to_string()],
        }];
        let gust_emitter_large =
            ParticleInstance::from_spawn_command(gust_emitter_large_command, &mut gust_random);
        assert_eq!(
            gust_emitter_large.provider,
            "GustSeedParticle.Provider(3.0,7,0)"
        );
        assert_eq!(
            gust_emitter_large.render_group,
            ParticleRenderGroup::NoRender
        );
        assert_eq!(gust_emitter_large.lifetime_ticks, 8);
        assert_eq!(gust_emitter_large.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(
            gust_emitter_large.child_emission,
            Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths: 30,
                vanilla_lifetime: 7,
                tick_delay: 0,
            })
        );

        let gust_emitter_small = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:gust_emitter_small", 1.0),
            &mut small_gust_random,
        );
        assert_eq!(
            gust_emitter_small.provider,
            "GustSeedParticle.Provider(1.0,3,2)"
        );
        assert_eq!(
            gust_emitter_small.render_group,
            ParticleRenderGroup::NoRender
        );
        assert_eq!(gust_emitter_small.lifetime_ticks, 4);
        assert_eq!(
            gust_emitter_small.child_emission,
            Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths: 10,
                vanilla_lifetime: 3,
                tick_delay: 2,
            })
        );
    }

    #[test]
    fn particle_instances_record_vanilla_render_groups_and_layers() {
        let mut random = ParticleRandom::new(0);
        let opaque = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:flame", 1.0),
            &mut random,
        );
        let cloud = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:cloud", 2.0),
            &mut random,
        );
        let squid_ink = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:squid_ink", 3.0),
            &mut random,
        );
        let sculk = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:sculk_charge", 4.0),
            &mut random,
        );
        let glow =
            ParticleInstance::from_spawn_command(spawn_command("minecraft:glow", 5.0), &mut random);
        let current_down = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:current_down", 6.0),
            &mut random,
        );
        let enchant = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:enchant", 7.0),
            &mut random,
        );
        let nautilus = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:nautilus", 8.0),
            &mut random,
        );
        let totem = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:totem_of_undying", 9.0),
            &mut random,
        );
        let vault = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:vault_connection", 10.0),
            &mut random,
        );
        let ominous_spawn = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:ominous_spawning", 11.0),
            &mut random,
        );
        let mut vibration_command = spawn_command("minecraft:vibration", 12.0);
        vibration_command.option_target = Some([12.0, 1.0, 0.0]);
        vibration_command.option_duration_ticks = Some(20);
        let vibration = ParticleInstance::from_spawn_command(vibration_command, &mut random);
        let unresolved_vibration = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:vibration", 13.0),
            &mut random,
        );
        let elder_guardian = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:elder_guardian", 14.0),
            &mut random,
        );
        let terrain = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block", 15.0),
            &mut random,
        );
        let block_marker = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block_marker", 16.0),
            &mut random,
        );
        let dust_pillar = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dust_pillar", 17.0),
            &mut random,
        );
        let block_crumble = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block_crumble", 18.0),
            &mut random,
        );
        let item = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item", 19.0),
            &mut random,
        );
        let item_slime = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_slime", 20.0),
            &mut random,
        );
        let item_cobweb = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_cobweb", 21.0),
            &mut random,
        );
        let item_snowball = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_snowball", 22.0),
            &mut random,
        );
        let falling_dust = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_dust", 23.0),
            &mut random,
        );

        assert_eq!(opaque.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(cloud.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(squid_ink.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(sculk.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(glow.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(current_down.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(enchant.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(nautilus.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(totem.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(vault.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(ominous_spawn.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(vibration.render_group, ParticleRenderGroup::SingleQuads);
        assert_eq!(
            unresolved_vibration.render_group,
            ParticleRenderGroup::NoRender
        );
        assert_eq!(
            elder_guardian.render_group,
            ParticleRenderGroup::ElderGuardians
        );
        assert_eq!(elder_guardian.provider, "ElderGuardianParticle.Provider");
        assert_eq!(elder_guardian.lifetime_ticks, 30);
        assert_eq!(opaque.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(cloud.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(squid_ink.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(sculk.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(totem.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(glow.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(current_down.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(enchant.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(nautilus.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(vault.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(ominous_spawn.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(vibration.render_layer, ParticleRenderLayer::Translucent);
        assert_eq!(
            elder_guardian.render_layer,
            ParticleRenderLayer::Translucent
        );
        assert_eq!(terrain.render_layer, ParticleRenderLayer::OpaqueTerrain);
        assert_eq!(
            block_marker.render_layer,
            ParticleRenderLayer::OpaqueTerrain
        );
        assert_eq!(dust_pillar.render_layer, ParticleRenderLayer::OpaqueTerrain);
        assert_eq!(
            block_crumble.render_layer,
            ParticleRenderLayer::OpaqueTerrain
        );
        assert_eq!(item.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_eq!(item_slime.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_eq!(item_cobweb.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_eq!(item_snowball.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_eq!(falling_dust.render_layer, ParticleRenderLayer::Opaque);
        for particle in [
            &opaque,
            &cloud,
            &squid_ink,
            &sculk,
            &glow,
            &current_down,
            &enchant,
            &nautilus,
            &totem,
            &vault,
            &ominous_spawn,
            &vibration,
            &unresolved_vibration,
            &elder_guardian,
            &falling_dust,
        ] {
            assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Particles);
        }
        for particle in [&terrain, &block_marker, &dust_pillar, &block_crumble] {
            assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Terrain);
        }
        for particle in [&item, &item_slime, &item_cobweb, &item_snowball] {
            assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Items);
        }
    }

    #[test]
    fn particle_instances_preserve_terrain_and_item_option_metadata() {
        let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
        let mut block_command = spawn_command("minecraft:block", 0.0);
        block_command.option_block = Some(ParticleBlockOptionState {
            block_state_id: 321,
        });
        let mut item_command = spawn_command("minecraft:item", 1.0);
        item_command.option_item = Some(ParticleItemOptionState {
            item_id: 42,
            count: 3,
            component_patch_len: 2,
        });

        let block = ParticleInstance::from_spawn_command(block_command, &mut random);
        let item = ParticleInstance::from_spawn_command(item_command, &mut random);

        assert_eq!(block.render_layer, ParticleRenderLayer::OpaqueTerrain);
        assert_eq!(
            block.option_block,
            Some(ParticleBlockOptionState {
                block_state_id: 321
            })
        );
        assert_eq!(block.option_item, None);
        assert_eq!(item.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_eq!(
            item.option_item,
            Some(ParticleItemOptionState {
                item_id: 42,
                count: 3,
                component_patch_len: 2,
            })
        );
        assert_eq!(item.option_block, None);
    }

    #[test]
    fn particle_instances_record_terrain_and_item_atlas_provider_shape_and_sub_rects() {
        let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
        let block = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block", 0.0),
            &mut random,
        );
        let block_marker = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block_marker", 1.0),
            &mut random,
        );
        let dust_pillar = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:dust_pillar", 2.0),
            &mut random,
        );
        let block_crumble = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:block_crumble", 3.0),
            &mut random,
        );
        let item =
            ParticleInstance::from_spawn_command(spawn_command("minecraft:item", 4.0), &mut random);
        let item_slime = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_slime", 5.0),
            &mut random,
        );
        let item_cobweb = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_cobweb", 6.0),
            &mut random,
        );
        let item_snowball = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:item_snowball", 7.0),
            &mut random,
        );
        let falling_dust = ParticleInstance::from_spawn_command(
            spawn_command("minecraft:falling_dust", 8.0),
            &mut random,
        );

        for terrain in [&block, &dust_pillar, &block_crumble] {
            assert_eq!(terrain.render_layer, ParticleRenderLayer::OpaqueTerrain);
            assert_range_f32(terrain.base_quad_size, 0.05, 0.1);
            assert_eq!(terrain.color, [0.6, 0.6, 0.6, 1.0]);
            assert_close_f32(terrain.gravity, 1.0);
            assert!(terrain.has_physics);
            assert_atlas_sub_rect(terrain);
        }
        assert_eq!(block.provider, "TerrainParticle.Provider");
        assert_eq!(dust_pillar.provider, "TerrainParticle.DustPillarProvider");
        assert_range_f32(dust_pillar.lifetime_ticks as f32, 20.0, 39.0);
        assert_eq!(block_crumble.provider, "TerrainParticle.CrumblingProvider");
        assert_range_f32(block_crumble.lifetime_ticks as f32, 1.0, 10.0);

        assert_eq!(block_marker.provider, "BlockMarker.Provider");
        assert_eq!(
            block_marker.render_layer,
            ParticleRenderLayer::OpaqueTerrain
        );
        assert_close_f32(block_marker.base_quad_size, 0.5);
        assert_eq!(block_marker.lifetime_ticks, 80);
        assert_close_f32(block_marker.gravity, 0.0);
        assert!(!block_marker.has_physics);
        assert_eq!(block_marker.atlas_uv_sub_rect, None);

        for item_particle in [&item, &item_slime, &item_cobweb, &item_snowball] {
            assert_eq!(item_particle.render_layer, ParticleRenderLayer::OpaqueItems);
            assert_range_f32(item_particle.base_quad_size, 0.05, 0.1);
            assert_eq!(item_particle.color, [1.0, 1.0, 1.0, 1.0]);
            assert_close_f32(item_particle.gravity, 1.0);
            assert!(item_particle.has_physics);
            assert_atlas_sub_rect(item_particle);
        }
        assert_eq!(item.provider, "BreakingItemParticle.Provider");
        assert_eq!(item_slime.provider, "BreakingItemParticle.SlimeProvider");
        assert_eq!(item_cobweb.provider, "BreakingItemParticle.CobwebProvider");
        assert_eq!(
            item_snowball.provider,
            "BreakingItemParticle.SnowballProvider"
        );

        assert_eq!(falling_dust.provider, "FallingDustParticle.Provider");
        assert_eq!(falling_dust.render_layer, ParticleRenderLayer::Opaque);
        assert_eq!(
            falling_dust.texture_atlas,
            ParticleTextureAtlasKind::Particles
        );
        assert_eq!(falling_dust.sprite_selection, ParticleSpriteSelection::Age);
        assert_eq!(falling_dust.current_sprite_index, Some(0));
        assert_range_f32(falling_dust.lifetime_ticks as f32, 28.0, 144.0);
        assert_range_f32(falling_dust.base_quad_size, 0.067_499_995, 0.135);
        assert_eq!(
            falling_dust.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );
        assert_eq!(falling_dust.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(falling_dust.velocity, [0.0, 0.0, 0.0]);
        assert_eq!(
            falling_dust.tick_motion,
            ParticleTickMotionDescriptor::FallingDust
        );
        assert_range_f32(falling_dust.roll_speed, -0.05, 0.05);
        assert_range_f32(falling_dust.roll, 0.0, std::f32::consts::PI * 2.0);
        assert_close_f32(falling_dust.previous_roll, falling_dust.roll);
        assert_eq!(falling_dust.atlas_uv_sub_rect, None);
    }

    #[test]
    fn falling_dust_instance_uses_option_color_for_block_dust_tint() {
        let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
        let mut command = spawn_command("minecraft:falling_dust", 0.0);
        command.option_color = Some([0.86, 0.83, 0.63, 1.0]);

        let falling_dust = ParticleInstance::from_spawn_command(command, &mut random);

        assert_eq!(falling_dust.provider, "FallingDustParticle.Provider");
        assert_eq!(falling_dust.color, [0.86, 0.83, 0.63, 1.0]);
        assert_eq!(
            falling_dust.quad_size_curve,
            ParticleQuadSizeCurve::GrowToBase
        );
    }

    #[test]
    fn particle_runtime_snowflake_applies_vanilla_post_tick_damping() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime("minecraft:snowflake", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.velocity = [1.0, 2.0, 3.0];
        particles.active_instances.push_back(instance);

        particles.advance(1);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.previous_position, [1.0, 2.0, 3.0]);
        assert_close_f64(instance.position[0], 2.0);
        assert_close_f64(instance.position[1], 3.991);
        assert_close_f64(instance.position[2], 6.0);
        assert_close_f64(instance.velocity[0], 0.95);
        assert_close_f64(instance.velocity[1], 1.7919);
        assert_close_f64(instance.velocity[2], 2.85);
    }

    #[test]
    fn particle_render_group_and_layer_order_match_vanilla_extract_passes() {
        assert_eq!(ParticleRenderGroup::SingleQuads.vanilla_order(), 0);
        assert_eq!(ParticleRenderGroup::ItemPickup.vanilla_order(), 1);
        assert_eq!(ParticleRenderGroup::ElderGuardians.vanilla_order(), 2);
        assert_eq!(ParticleRenderGroup::NoRender.vanilla_order(), 3);

        assert!(
            ParticleRenderLayer::OpaqueTerrain.vanilla_solid_translucent_order()
                < ParticleRenderLayer::TranslucentTerrain.vanilla_solid_translucent_order()
        );
        assert!(
            ParticleRenderLayer::OpaqueItems.vanilla_solid_translucent_order()
                < ParticleRenderLayer::TranslucentItems.vanilla_solid_translucent_order()
        );
        assert!(
            ParticleRenderLayer::Opaque.vanilla_solid_translucent_order()
                < ParticleRenderLayer::Translucent.vanilla_solid_translucent_order()
        );
    }

    #[test]
    fn particle_billboard_vertices_follow_vanilla_group_and_layer_order() {
        let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
        cloud.position = [10.0, 0.0, 0.0];
        cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut block = test_instance_with_lifetime("minecraft:block", 20);
        block.position = [15.0, 0.0, 0.0];
        block.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
        flame.position = [20.0, 0.0, 0.0];
        flame.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut item = test_instance_with_lifetime("minecraft:item", 20);
        item.position = [25.0, 0.0, 0.0];
        item.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut soul = test_instance_with_lifetime("minecraft:soul", 20);
        soul.position = [30.0, 0.0, 0.0];
        soul.current_sprite_id = Some("minecraft:generic_0".to_string());
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&cloud, &block, &flame, &item, &soul],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            None,
        );

        assert_eq!(vertices.len(), 30);
        assert_close_f32(vertices[0].position[0], 14.9);
        assert_close_f32(vertices[6].position[0], 24.9);
        assert_close_f32(vertices[12].position[0], 19.9);
        assert_close_f32(vertices[18].position[0], 9.9);
        assert_close_f32(vertices[24].position[0], 29.9);
    }

    #[test]
    fn particle_billboard_vertices_skip_non_single_quad_groups() {
        let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
        cloud.position = [10.0, 0.0, 0.0];
        cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut elder_guardian = test_instance_with_lifetime("minecraft:elder_guardian", 30);
        elder_guardian.position = [20.0, 0.0, 0.0];
        elder_guardian.current_sprite_id = Some("minecraft:generic_0".to_string());
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&cloud, &elder_guardian],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            None,
        );

        assert_eq!(vertices.len(), 6);
        assert_close_f32(vertices[0].position[0], 9.9);
    }

    #[test]
    fn particle_billboard_vertices_split_vanilla_opaque_and_translucent_pipelines() {
        let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
        cloud.position = [10.0, 0.0, 0.0];
        cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
        flame.position = [20.0, 0.0, 0.0];
        flame.current_sprite_id = Some("minecraft:generic_0".to_string());
        let mut soul = test_instance_with_lifetime("minecraft:soul", 20);
        soul.position = [30.0, 0.0, 0.0];
        soul.current_sprite_id = Some("minecraft:generic_0".to_string());
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);
        let axes = ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        };

        let opaque_vertices = particle_billboard_vertices(
            [&cloud, &flame, &soul],
            &sprite_uvs,
            axes,
            Some(ParticlePipelineKind::Opaque),
        );
        let translucent_vertices = particle_billboard_vertices(
            [&cloud, &flame, &soul],
            &sprite_uvs,
            axes,
            Some(ParticlePipelineKind::Translucent),
        );

        assert_eq!(opaque_vertices.len(), 6);
        assert_close_f32(opaque_vertices[0].position[0], 19.9);
        assert_eq!(translucent_vertices.len(), 12);
        assert_close_f32(translucent_vertices[0].position[0], 9.9);
        assert_close_f32(translucent_vertices[6].position[0], 29.9);
    }

    #[test]
    fn particle_pipeline_vertex_batches_split_texture_atlases_in_vanilla_layer_order() {
        let mut block = test_instance_with_lifetime("minecraft:block", 20);
        block.position = [20.0, 0.0, 0.0];
        block.current_sprite_id = Some("minecraft:block/oak_planks".to_string());
        let mut item = test_instance_with_lifetime("minecraft:item", 20);
        item.position = [30.0, 0.0, 0.0];
        item.current_sprite_id = Some("minecraft:item/apple".to_string());
        let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
        flame.position = [40.0, 0.0, 0.0];
        flame.current_sprite_id = Some("minecraft:generic_0".to_string());
        let particle_sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);
        let terrain_sprite_uvs = BTreeMap::from([(
            "minecraft:block/oak_planks".to_string(),
            ParticleUvRect {
                min: [0.1, 0.1],
                max: [0.2, 0.2],
            },
        )]);
        let item_sprite_uvs = BTreeMap::from([(
            "minecraft:item/apple".to_string(),
            ParticleUvRect {
                min: [0.3, 0.3],
                max: [0.4, 0.4],
            },
        )]);

        let batch = particle_pipeline_vertex_batch(
            [&flame, &item, &block],
            ParticleAtlasUvSets {
                particles: Some(&particle_sprite_uvs),
                terrain: Some(&terrain_sprite_uvs),
                items: Some(&item_sprite_uvs),
            },
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            ParticlePipelineKind::Opaque,
        );

        assert_eq!(batch.vertices.len(), 18);
        assert_eq!(
            batch.draws,
            vec![
                ParticleAtlasDrawRange {
                    texture_atlas: ParticleTextureAtlasKind::Terrain,
                    vertex_start: 0,
                    vertex_count: 6,
                },
                ParticleAtlasDrawRange {
                    texture_atlas: ParticleTextureAtlasKind::Items,
                    vertex_start: 6,
                    vertex_count: 6,
                },
                ParticleAtlasDrawRange {
                    texture_atlas: ParticleTextureAtlasKind::Particles,
                    vertex_start: 12,
                    vertex_count: 6,
                },
            ]
        );
        assert_close_f32(batch.vertices[0].position[0], 19.9);
        assert_close_f32(batch.vertices[6].position[0], 29.9);
        assert_close_f32(batch.vertices[12].position[0], 39.9);
        assert_eq!(batch.vertices[0].uv, [0.1, 0.2]);
        assert_eq!(batch.vertices[6].uv, [0.3, 0.4]);
        assert_eq!(batch.vertices[12].uv, [0.0, 1.0]);
    }

    #[test]
    fn particle_quad_size_curves_follow_vanilla_shapes() {
        let mut constant = test_instance_with_lifetime("minecraft:squid_ink", 20);
        constant.base_quad_size = 0.5;
        constant.quad_size_curve = ParticleQuadSizeCurve::Constant;
        assert_close_f32(constant.quad_size_at_partial_tick(0.0), 0.5);
        constant.age_ticks = 20;
        assert_close_f32(constant.quad_size_at_partial_tick(0.0), 0.5);

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

        let mut lava = test_instance_with_lifetime("minecraft:lava", 20);
        lava.base_quad_size = 0.2;
        lava.quad_size_curve = ParticleQuadSizeCurve::Lava;
        assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.2);
        lava.age_ticks = 10;
        assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.15);
        lava.age_ticks = 20;
        assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.0);

        let mut portal = test_instance_with_lifetime("minecraft:portal", 40);
        portal.base_quad_size = 0.06;
        portal.quad_size_curve = ParticleQuadSizeCurve::Portal;
        assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.0);
        portal.age_ticks = 20;
        assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.045);
        portal.age_ticks = 40;
        assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.06);

        let mut reverse_portal = test_instance_with_lifetime("minecraft:reverse_portal", 60);
        reverse_portal.base_quad_size = 0.09;
        reverse_portal.quad_size_curve = ParticleQuadSizeCurve::ReversePortal;
        assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.09);
        reverse_portal.age_ticks = 30;
        assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.06);
        reverse_portal.age_ticks = 60;
        assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.03);

        let mut shriek = test_instance_with_lifetime("minecraft:shriek", 30);
        shriek.base_quad_size = 0.85;
        shriek.quad_size_curve = ParticleQuadSizeCurve::Shriek;
        assert_close_f32(shriek.quad_size_at_partial_tick(0.0), 0.0);
        assert_close_f32(shriek.quad_size_at_partial_tick(0.5), 0.010_625);
        shriek.age_ticks = 30;
        assert_close_f32(shriek.quad_size_at_partial_tick(0.0), 0.637_5);

        let mut flash = test_instance_with_lifetime("minecraft:flash", 4);
        flash.quad_size_curve = ParticleQuadSizeCurve::FlashOverlay;
        flash.age_ticks = 1;
        assert_close_f32(
            flash.quad_size_at_partial_tick(0.5),
            7.1 * (0.5 * 0.25 * std::f32::consts::PI).sin(),
        );
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
    fn particle_runtime_enforces_vanilla_spore_blossom_particle_limit() {
        let mut particles = ParticleRuntimeState::with_capacities(1105, 1105);
        let commands = (0..=VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT)
            .map(|index| spawn_command("minecraft:spore_blossom_air", index as f64))
            .collect();
        particles.submit_batch(ParticleSpawnBatch {
            commands,
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(0);

        assert_eq!(summary.intaken_instances, 1000);
        assert_eq!(summary.limited_particle_drops, 1);
        assert_eq!(summary.total_limited_particle_drops, 1);
        assert_eq!(summary.dropped_active_instances, 0);
        assert_eq!(summary.active_instances, 1000);
        assert_eq!(
            particles.active_instances()[0].particle_limit,
            Some(ParticleLimitDescriptor::SporeBlossom)
        );
        assert_eq!(
            particles.active_instances()[0].position[0],
            0.0,
            "ParticleEngine.add rejects the over-limit particle instead of evicting accepted ones"
        );
        assert_eq!(particles.active_instances()[999].position[0], 999.0);
    }

    #[test]
    fn particle_runtime_releases_spore_blossom_limit_counts_on_expiry() {
        let mut particles = ParticleRuntimeState::with_capacities(1101, 1101);
        let commands = (0..VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT)
            .map(|index| spawn_command("minecraft:spore_blossom_air", index as f64))
            .collect();
        particles.submit_batch(ParticleSpawnBatch {
            commands,
            ..ParticleSpawnBatch::default()
        });
        let first_summary = particles.advance(0);
        assert_eq!(first_summary.intaken_instances, 1000);
        for instance in &mut particles.active_instances {
            instance.lifetime_ticks = 0;
        }
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![spawn_command("minecraft:spore_blossom_air", 1000.0)],
            ..ParticleSpawnBatch::default()
        });

        let summary = particles.advance(1);

        assert_eq!(summary.expired_instances, 1000);
        assert_eq!(summary.intaken_instances, 1);
        assert_eq!(summary.limited_particle_drops, 0);
        assert_eq!(summary.active_instances, 1);
        assert_eq!(particles.active_instances()[0].position[0], 1000.0);
    }

    #[test]
    fn particle_runtime_refreshes_active_lights_from_world_positions() {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut first = test_instance_with_lifetime("minecraft:cloud", 20);
        first.position = [1.25, 2.0, 3.75];
        let mut second = test_instance_with_lifetime("minecraft:smoke", 20);
        second.position = [-2.0, 9.5, 0.25];
        particles.active_instances.push_back(first);
        particles.active_instances.push_back(second);

        particles.refresh_lights(|position| {
            if position[0] < 0.0 {
                [1.25, f32::NAN]
            } else {
                [4.0 / 15.0, 11.0 / 15.0]
            }
        });

        assert_eq!(
            particles.active_instances()[0].light,
            [4.0 / 15.0, 11.0 / 15.0]
        );
        assert_eq!(particles.active_instances()[1].light, [1.0, 1.0]);
    }

    #[test]
    fn particle_runtime_applies_vanilla_particle_light_emission_overrides() {
        let sampled_light = [2.0 / 15.0, 7.0 / 15.0];
        let mut particles = ParticleRuntimeState::with_capacities(17, 17);
        let cloud = test_instance_with_lifetime("minecraft:cloud", 20);
        let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
        flame.age_ticks = 4;
        let mut glow = test_instance_with_lifetime("minecraft:glow", 4);
        glow.age_ticks = 1;
        let lava = test_instance_with_lifetime("minecraft:lava", 20);
        let sculk_soul = test_instance_with_lifetime("minecraft:sculk_soul", 20);
        let sculk_charge_pop = test_instance_with_lifetime("minecraft:sculk_charge_pop", 20);
        let attack_sweep = test_instance_with_lifetime("minecraft:sweep_attack", 4);
        let end_rod = test_instance_with_lifetime("minecraft:end_rod", 60);
        let totem = test_instance_with_lifetime("minecraft:totem_of_undying", 60);
        let mut enchant = test_instance_with_lifetime("minecraft:enchant", 40);
        enchant.age_ticks = 20;
        let mut portal = test_instance_with_lifetime("minecraft:portal", 40);
        portal.age_ticks = 20;
        let mut reverse_portal = test_instance_with_lifetime("minecraft:reverse_portal", 60);
        reverse_portal.age_ticks = 30;
        let shriek = test_instance_with_lifetime("minecraft:shriek", 30);
        let vault_connection = test_instance_with_lifetime("minecraft:vault_connection", 40);
        let vibration = test_instance_with_lifetime("minecraft:vibration", 40);
        let ominous_spawn = test_instance_with_lifetime("minecraft:ominous_spawning", 25);
        let mut firefly = test_instance_with_lifetime("minecraft:firefly", 100);
        firefly.age_ticks = 15;

        particles.active_instances.push_back(cloud);
        particles.active_instances.push_back(flame);
        particles.active_instances.push_back(glow);
        particles.active_instances.push_back(lava);
        particles.active_instances.push_back(sculk_soul);
        particles.active_instances.push_back(sculk_charge_pop);
        particles.active_instances.push_back(attack_sweep);
        particles.active_instances.push_back(end_rod);
        particles.active_instances.push_back(totem);
        particles.active_instances.push_back(enchant);
        particles.active_instances.push_back(portal);
        particles.active_instances.push_back(reverse_portal);
        particles.active_instances.push_back(shriek);
        particles.active_instances.push_back(vault_connection);
        particles.active_instances.push_back(vibration);
        particles.active_instances.push_back(ominous_spawn);
        particles.active_instances.push_back(firefly);

        particles.refresh_lights(|_| sampled_light);

        assert_eq!(particles.active_instances()[0].light, sampled_light);
        assert_close_f32(
            particles.active_instances()[1].light[0],
            sampled_light[0] + 4.5 / 20.0,
        );
        assert_close_f32(particles.active_instances()[1].light[1], sampled_light[1]);
        assert_close_f32(
            particles.active_instances()[2].light[0],
            sampled_light[0] + 1.5 / 4.0,
        );
        assert_close_f32(particles.active_instances()[2].light[1], sampled_light[1]);
        assert_eq!(
            particles.active_instances()[3].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(
            particles.active_instances()[4].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(
            particles.active_instances()[5].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(particles.active_instances()[6].light, [1.0, 1.0]);
        assert_eq!(particles.active_instances()[7].light, [1.0, 1.0]);
        assert_eq!(particles.active_instances()[8].light, [1.0, 1.0]);
        assert_close_f32(
            particles.active_instances()[9].light[0],
            sampled_light[0] + 0.5_f32.powi(4),
        );
        assert_close_f32(particles.active_instances()[9].light[1], sampled_light[1]);
        assert_close_f32(
            particles.active_instances()[10].light[0],
            sampled_light[0] + 0.5_f32.powi(4),
        );
        assert_close_f32(particles.active_instances()[10].light[1], sampled_light[1]);
        assert_close_f32(
            particles.active_instances()[11].light[0],
            sampled_light[0] + 0.5_f32.powi(4),
        );
        assert_close_f32(particles.active_instances()[11].light[1], sampled_light[1]);
        assert_eq!(
            particles.active_instances()[12].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(
            particles.active_instances()[13].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(
            particles.active_instances()[14].light,
            [1.0, sampled_light[1]]
        );
        assert_eq!(
            particles.active_instances()[15].light,
            [1.0, sampled_light[1]]
        );
        assert_close_f32(
            particles.active_instances()[16].light[0],
            firefly_fade_amount(15.5 / 100.0, 0.1, 0.3),
        );
        assert_close_f32(particles.active_instances()[16].light[1], 0.0);
    }

    #[test]
    fn particle_billboard_vertices_emit_camera_facing_textured_quad() {
        let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.4;
        instance.color = [0.25, 0.5, 0.75, 0.8];
        instance.light = [0.4, 0.8];
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
            None,
        );

        assert_eq!(vertices.len(), 6);
        assert_close3_f32(vertices[0].position, [0.8, 1.8, 3.0]);
        assert_eq!(vertices[0].uv, [0.25, 0.375]);
        assert_eq!(vertices[0].color, [0.25, 0.5, 0.75, 0.8]);
        assert_eq!(vertices[0].light, [0.4, 0.8]);
        assert_close3_f32(vertices[2].position, [1.2, 2.2, 3.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.125]);
        assert_eq!(vertices[2].color, [0.25, 0.5, 0.75, 0.8]);
        assert_eq!(vertices[2].light, [0.4, 0.8]);
        assert_close3_f32(vertices[5].position, [0.8, 2.2, 3.0]);
        assert_eq!(vertices[5].uv, [0.25, 0.125]);
        assert_eq!(vertices[5].color, [0.25, 0.5, 0.75, 0.8]);
        assert_eq!(vertices[5].light, [0.4, 0.8]);
    }

    #[test]
    fn particle_billboard_vertices_apply_vanilla_atlas_sub_rect_uvs() {
        let mut instance = test_instance_with_lifetime("minecraft:block", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:block/oak_planks".to_string());
        instance.base_quad_size = 0.4;
        instance.atlas_uv_sub_rect = Some(ParticleAtlasUvSubRect {
            u_offset: 1.0,
            v_offset: 2.0,
        });
        let sprite_uvs = BTreeMap::from([(
            "minecraft:block/oak_planks".to_string(),
            ParticleUvRect {
                min: [0.2, 0.4],
                max: [1.0, 0.8],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Opaque),
        );

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].uv, [0.6, 0.700_000_05]);
        assert_eq!(vertices[1].uv, [0.4, 0.700_000_05]);
        assert_eq!(vertices[2].uv, [0.4, 0.6]);
        assert_eq!(vertices[5].uv, [0.6, 0.6]);
    }

    #[test]
    fn particle_billboard_vertices_apply_vanilla_lookat_y_facing_mode() {
        let mut instance = test_instance_with_lifetime("minecraft:trial_spawner_detection", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 2.0;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::Z,
                up: Vec3::new(0.0, 0.5, 0.866_025_4),
            },
            Some(ParticlePipelineKind::Opaque),
        );

        assert_eq!(
            instance.facing_camera_mode,
            ParticleFacingCameraMode::LookAtY
        );
        assert_eq!(vertices.len(), 6);
        assert_close3_f32(vertices[0].position, [1.0, 1.0, 2.0]);
        assert_close3_f32(vertices[1].position, [1.0, 1.0, 4.0]);
        assert_close3_f32(vertices[2].position, [1.0, 3.0, 4.0]);
        assert_close3_f32(vertices[5].position, [1.0, 3.0, 2.0]);
    }

    #[test]
    fn particle_billboard_vertices_apply_vanilla_roll_transform() {
        let mut instance = test_instance_with_lifetime("minecraft:sculk_charge", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 2.0;
        instance.previous_roll = std::f32::consts::FRAC_PI_2;
        instance.roll = std::f32::consts::FRAC_PI_2;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        assert_eq!(vertices.len(), 6);
        assert_close3_f32(vertices[0].position, [2.0, 1.0, 3.0]);
        assert_close3_f32(vertices[1].position, [2.0, 3.0, 3.0]);
        assert_close3_f32(vertices[2].position, [0.0, 3.0, 3.0]);
        assert_close3_f32(vertices[5].position, [0.0, 1.0, 3.0]);
    }

    #[test]
    fn particle_billboard_vertices_apply_vault_connection_lifetime_alpha() {
        let mut instance = test_instance_with_lifetime("minecraft:vault_connection", 40);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.4;
        instance.color = [0.45, 0.45, 0.5, 0.0];
        instance.age_ticks = 20;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].color[0], 0.45);
        assert_eq!(vertices[0].color[1], 0.45);
        assert_eq!(vertices[0].color[2], 0.5);
        assert_close_f32(vertices[0].color[3], 0.21);
    }

    #[test]
    fn particle_billboard_vertices_apply_flash_overlay_alpha_and_size() {
        let mut instance = test_instance_with_lifetime("minecraft:flash", 4);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.quad_size_curve = ParticleQuadSizeCurve::FlashOverlay;
        instance.color = [0.1, 0.2, 0.3, 0.4];
        instance.age_ticks = 1;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        let size = 7.1 * (0.5 * 0.25 * std::f32::consts::PI).sin();
        assert_eq!(vertices.len(), 6);
        assert_close3_f32(
            vertices[0].position,
            [1.0 - size / 2.0, 2.0 - size / 2.0, 3.0],
        );
        assert_close_f32(vertices[0].color[0], 0.1);
        assert_close_f32(vertices[0].color[1], 0.2);
        assert_close_f32(vertices[0].color[2], 0.3);
        assert_close_f32(vertices[0].color[3], flash_overlay_alpha(1, 0.5));
    }

    #[test]
    fn particle_billboard_vertices_use_simple_animated_runtime_alpha() {
        let mut instance = test_instance_with_lifetime("minecraft:squid_ink", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.4;
        instance.color = [0.0, 0.0, 0.0, 0.95];
        instance.age_ticks = 11;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        assert_eq!(vertices.len(), 6);
        assert_close_f32(vertices[0].color[3], 0.95);
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
            None,
        );

        assert!(vertices.is_empty());
    }

    #[test]
    fn particle_billboard_vertices_skip_delayed_shriek_instances() {
        let mut instance = test_instance_with_lifetime("minecraft:shriek", 30);
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.delay_ticks = 1;
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        assert!(vertices.is_empty());
    }

    #[test]
    fn particle_billboard_vertices_emit_vanilla_shriek_rotated_quads() {
        let mut instance = test_instance_with_lifetime("minecraft:shriek", 30);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.85;
        instance.quad_size_curve = ParticleQuadSizeCurve::Shriek;
        instance.alpha_curve = ParticleAlphaCurve::ShriekFade;
        instance.light = [1.0, 0.4];
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.25, 0.5],
                max: [0.75, 0.875],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        assert_eq!(vertices.len(), 12);
        assert_close3_f32(
            vertices[0].position,
            [0.994_687_5, 1.997_343_8, 3.004_600_8],
        );
        assert_close3_f32(
            vertices[2].position,
            [1.005_312_5, 2.002_656_2, 2.995_399_2],
        );
        assert_close3_f32(
            vertices[6].position,
            [1.005_312_5, 1.997_343_8, 3.004_600_8],
        );
        assert_close3_f32(
            vertices[8].position,
            [0.994_687_5, 2.002_656_2, 2.995_399_2],
        );
        assert_eq!(vertices[0].uv, [0.25, 0.875]);
        assert_eq!(vertices[2].uv, [0.75, 0.5]);
        assert_eq!(vertices[6].light, [1.0, 0.4]);
        assert_eq!(vertices[6].color[0], 1.0);
        assert_eq!(vertices[6].color[1], 1.0);
        assert_eq!(vertices[6].color[2], 1.0);
        assert_close_f32(vertices[6].color[3], 1.0 - 0.5 / 30.0);
    }

    #[test]
    fn particle_billboard_vertices_emit_vanilla_vibration_rotated_quads() {
        let mut instance = test_instance_with_lifetime("minecraft:vibration", 20);
        instance.position = [1.0, 2.0, 3.0];
        instance.current_sprite_id = Some("minecraft:generic_0".to_string());
        instance.base_quad_size = 0.3;
        instance.previous_pitch = -std::f32::consts::FRAC_PI_2;
        instance.pitch = -std::f32::consts::FRAC_PI_2;
        instance.light = [1.0, 0.4];
        let sprite_uvs = BTreeMap::from([(
            "minecraft:generic_0".to_string(),
            ParticleUvRect {
                min: [0.25, 0.5],
                max: [0.75, 0.875],
            },
        )]);

        let vertices = particle_billboard_vertices(
            [&instance],
            &sprite_uvs,
            ParticleBillboardAxes {
                right: Vec3::X,
                up: Vec3::Y,
            },
            Some(ParticlePipelineKind::Translucent),
        );

        let half_size = 0.15;
        let random_sway = vibration_particle_sway(0, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
        let first_rotation = Quat::from_rotation_y(random_sway);
        let second_rotation = Quat::from_rotation_y(-std::f32::consts::PI + random_sway);
        let center = Vec3::new(1.0, 2.0, 3.0);
        let first_bottom_left = center + first_rotation * Vec3::new(-half_size, -half_size, 0.0);
        let second_bottom_left = center + second_rotation * Vec3::new(-half_size, -half_size, 0.0);

        assert_eq!(vertices.len(), 12);
        assert_close3_f32(vertices[0].position, first_bottom_left.to_array());
        assert_close3_f32(vertices[6].position, second_bottom_left.to_array());
        assert_eq!(vertices[0].uv, [0.25, 0.875]);
        assert_eq!(vertices[6].light, [1.0, 0.4]);
        assert_eq!(vertices[6].color, [1.0, 1.0, 1.0, 1.0]);
    }

    fn test_instance_with_lifetime(particle_id: &str, lifetime_ticks: u32) -> ParticleInstance {
        let descriptor = ParticleDescriptor::for_particle(particle_id);
        ParticleInstance {
            particle_type_id: 0,
            particle_id: particle_id.to_string(),
            sprite_ids: Vec::new(),
            current_sprite_id: None,
            current_sprite_index: None,
            start_position: [0.0, 0.0, 0.0],
            previous_position: [0.0, 0.0, 0.0],
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            age_ticks: 0,
            lifetime_ticks,
            previous_roll: 0.0,
            roll: 0.0,
            roll_speed: 0.0,
            previous_yaw: 0.0,
            yaw: 0.0,
            previous_pitch: 0.0,
            pitch: 0.0,
            base_quad_size: DEFAULT_PARTICLE_QUAD_SIZE,
            color: [1.0, 1.0, 1.0, 1.0],
            color_fade_target: descriptor.color_fade_target(),
            color_transition_target: None,
            light: DEFAULT_PARTICLE_LIGHT,
            light_emission: descriptor.light_emission(),
            alpha_curve: descriptor.alpha_curve(),
            quad_size_curve: ParticleQuadSizeCurve::Constant,
            provider: descriptor.provider.to_string(),
            render_group: particle_render_group_for_particle(particle_id),
            render_layer: particle_render_layer_for_particle(particle_id),
            texture_atlas: particle_render_layer_for_particle(particle_id).texture_atlas_kind(),
            facing_camera_mode: descriptor.facing_camera_mode(),
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            has_physics: descriptor.has_physics,
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            tick_motion: descriptor.tick_motion(),
            tick_angle: 0.0,
            particle_limit: particle_limit_for_particle(particle_id),
            child_emission: descriptor.child_emission(),
            child_spawn_templates: Vec::new(),
            falling_leaves_motion: None,
            sprite_selection: descriptor.sprite_selection,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
            delay_ticks: 0,
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            atlas_uv_sub_rect: None,
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
            initial_delay_ticks: 0,
            child_spawn_templates: Vec::new(),
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
        }
    }

    fn assert_close_f32(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_close_f64(actual: f64, expected: f64) {
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

    fn assert_atlas_sub_rect(instance: &ParticleInstance) {
        let sub_rect = instance
            .atlas_uv_sub_rect
            .expect("terrain/item atlas particle should record a 4x4 sub-rect offset");
        assert_range_f32(sub_rect.u_offset, 0.0, 3.0);
        assert_range_f32(sub_rect.v_offset, 0.0, 3.0);
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
