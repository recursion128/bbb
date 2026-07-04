use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::Result;
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    entity_models::{
        ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
    },
    Renderer,
};

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
    create_particle_atlas_gpu, create_particle_pipeline, update_particle_atlas_gpu,
    ParticleAtlasGpu, ParticlePipelineKind, ParticleVertex,
};

const DEFAULT_MAX_PENDING_PARTICLE_SPAWNS: usize = 16_384;
const DEFAULT_MAX_ACTIVE_PARTICLE_INSTANCES: usize = 16_384;
const DEFAULT_PARTICLE_QUAD_SIZE: f32 = 0.2;
const DEFAULT_PARTICLE_COLLISION_WIDTH: f32 = 0.2;
const DEFAULT_PARTICLE_COLLISION_HEIGHT: f32 = 0.2;
const DEFAULT_PARTICLE_RENDER_PARTIAL_TICK: f32 = 0.5;
const DEFAULT_PARTICLE_LIGHT: [f32; 2] = [1.0, 1.0];
const SHRIEK_MAGICAL_X_ROT: f32 = 1.0472;
const LAVA_CHILD_SMOKE_PARTICLE_ID: &str = "minecraft:smoke";
const HUGE_EXPLOSION_CHILD_PARTICLE_ID: &str = "minecraft:explosion";
const GUST_CHILD_PARTICLE_ID: &str = "minecraft:gust";
const ITEM_PICKUP_PARTICLE_ID: &str = "minecraft:item_pickup";
const ITEM_PICKUP_PARTICLE_LIFETIME_TICKS: u32 = 3;
const ELDER_GUARDIAN_PARTICLE_ID: &str = "minecraft:elder_guardian";
const ELDER_GUARDIAN_PARTICLE_MODEL_SCALE: f32 = 0.425_531_92;
const ELDER_GUARDIAN_PARTICLE_BAKED_LAYER_SCALE: f32 = 2.35;
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
    pub option_entity_target_source: Option<ParticleEntityTargetSource>,
    #[serde(default)]
    pub option_duration_ticks: Option<u32>,
    #[serde(default)]
    pub option_roll: Option<f32>,
    #[serde(default)]
    pub option_block: Option<ParticleBlockOptionState>,
    #[serde(default)]
    pub option_item: Option<ParticleItemOptionState>,
    #[serde(default)]
    pub option_item_pickup_source_entity_id: Option<i32>,
    #[serde(default)]
    pub option_item_pickup_age_ticks: Option<f32>,
    #[serde(default)]
    pub option_item_pickup_light: Option<[f32; 2]>,
    #[serde(default)]
    pub option_item_pickup_experience_orb_icon: Option<i32>,
    #[serde(default)]
    pub option_firework_trail: bool,
    #[serde(default)]
    pub option_firework_twinkle: bool,
    #[serde(default)]
    pub option_firework_half_lifetime_age: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleLocalPlayerScopeContext {
    pub eye_position: [f64; 3],
    pub first_person: bool,
    pub scoping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleLocalPlayerMotionContext {
    pub position: [f64; 3],
    pub delta_movement: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParticleEntityTargetSource {
    pub entity_id: i32,
    pub y_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleEntityTargetContext {
    pub entity_id: i32,
    pub position: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleSoundEvent {
    pub sound_event_id: String,
    pub source: String,
    pub position: [f64; 3],
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
    pub distance_delay: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleScheduledSoundEvent {
    pub event: ParticleSoundEvent,
    pub delay_ticks: u32,
    #[serde(default)]
    pub far_sound_event_id: Option<String>,
    #[serde(default)]
    pub far_distance_squared: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleCollisionQuery {
    pub position: [f64; 3],
    pub movement: [f64; 3],
    pub half_width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleBlockFluidSurfaceQuery {
    pub position: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleFluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleBlockFluidSurfaceSample {
    pub block_collision_height: f64,
    pub fluid_height: f64,
    pub fluid_kind: Option<ParticleFluidKind>,
    pub block_is_air: bool,
}

impl Default for ParticleBlockFluidSurfaceSample {
    fn default() -> Self {
        Self {
            block_collision_height: 0.0,
            fluid_height: 0.0,
            fluid_kind: None,
            block_is_air: true,
        }
    }
}

impl ParticleBlockFluidSurfaceSample {
    fn max_surface_height(self) -> f64 {
        self.block_collision_height.max(self.fluid_height)
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemPickupParticleRenderState {
    pub source_entity_id: i32,
    pub item: ParticleItemOptionState,
    pub position: [f32; 3],
    pub age_ticks: f32,
    pub light: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperienceOrbPickupParticleRenderState {
    pub source_entity_id: i32,
    pub icon: i32,
    pub position: [f32; 3],
    pub age_ticks: f32,
    pub light: [f32; 2],
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
    pub has_translucent: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParticleSpawnBatch {
    #[serde(default)]
    pub commands: Vec<ParticleSpawnCommand>,
    #[serde(default)]
    pub sound_events: Vec<ParticleSoundEvent>,
    #[serde(default)]
    pub scheduled_sound_events: Vec<ParticleScheduledSoundEvent>,
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
    pending_sound_events: VecDeque<ParticleSoundEvent>,
    scheduled_sound_events: VecDeque<ParticleScheduledSoundEvent>,
    max_pending_spawns: usize,
    max_active_instances: usize,
    dropped_spawns: u64,
    instances_created: u64,
    instances_expired: u64,
    dropped_active_instances: u64,
    limited_particle_drops: u64,
    limited_particle_counts: BTreeMap<ParticleLimitDescriptor, usize>,
    random: ParticleRandom,
    sound_random: ParticleRandom,
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
    #[serde(default = "default_particle_original_alpha")]
    pub(crate) original_alpha: f32,
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
    #[serde(default)]
    pub(crate) moves_without_collision: bool,
    pub(crate) speed_up_when_y_motion_is_blocked: bool,
    #[serde(default = "default_particle_collision_width")]
    pub(crate) collision_width: f32,
    #[serde(default = "default_particle_collision_height")]
    pub(crate) collision_height: f32,
    #[serde(default)]
    pub(crate) on_ground: bool,
    #[serde(default)]
    pub(crate) hit_ground: bool,
    #[serde(default)]
    pub(crate) stopped_by_collision: bool,
    #[serde(default)]
    pub(crate) removed: bool,
    #[serde(default)]
    pub(crate) tick_motion: ParticleTickMotionDescriptor,
    #[serde(default)]
    pub(crate) drip_fluid: Option<ParticleFluidKind>,
    #[serde(default)]
    pub(crate) required_fluid: Option<ParticleFluidKind>,
    #[serde(default)]
    pub(crate) air_downward_acceleration: f64,
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
    pub(crate) option_entity_target_source: Option<ParticleEntityTargetSource>,
    #[serde(default)]
    pub(crate) option_duration_ticks: Option<u32>,
    #[serde(default)]
    pub(crate) option_roll: Option<f32>,
    #[serde(default)]
    pub(crate) option_block: Option<ParticleBlockOptionState>,
    #[serde(default)]
    pub(crate) option_item: Option<ParticleItemOptionState>,
    #[serde(default)]
    pub(crate) option_item_pickup_source_entity_id: Option<i32>,
    #[serde(default)]
    pub(crate) option_item_pickup_age_ticks: Option<f32>,
    #[serde(default)]
    pub(crate) option_item_pickup_light: Option<[f32; 2]>,
    #[serde(default)]
    pub(crate) option_item_pickup_experience_orb_icon: Option<i32>,
    #[serde(default)]
    pub(crate) firework_trail: bool,
    #[serde(default)]
    pub(crate) firework_twinkle: bool,
    #[serde(default)]
    pub(crate) item_pickup_previous_target: Option<[f64; 3]>,
    #[serde(default)]
    pub(crate) item_pickup_target: Option<[f64; 3]>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParticleRemovalReason {
    LifetimeExpired,
    RemovedDuringTick,
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
            && self.sound_events.is_empty()
            && self.scheduled_sound_events.is_empty()
            && self.missing_definition_count == 0
            && self.missing_sprite_count == 0
            && self.unknown_particle_type_count == 0
    }
}

impl ParticleScheduledSoundEvent {
    fn into_sound_event(mut self, camera_position: Option<[f64; 3]>) -> ParticleSoundEvent {
        if let (Some(far_sound_event_id), Some(far_distance_squared), Some(camera_position)) = (
            self.far_sound_event_id,
            self.far_distance_squared,
            camera_position,
        ) {
            let dx = self.event.position[0] - camera_position[0];
            let dy = self.event.position[1] - camera_position[1];
            let dz = self.event.position[2] - camera_position[2];
            if dx * dx + dy * dy + dz * dz >= far_distance_squared {
                self.event.sound_event_id = far_sound_event_id;
            }
        }
        self.event
    }
}

fn default_particle_quad_size() -> f32 {
    DEFAULT_PARTICLE_QUAD_SIZE
}

fn default_particle_collision_width() -> f32 {
    DEFAULT_PARTICLE_COLLISION_WIDTH
}

fn default_particle_collision_height() -> f32 {
    DEFAULT_PARTICLE_COLLISION_HEIGHT
}

fn default_particle_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_particle_original_alpha() -> f32 {
    1.0
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
            pending_sound_events: VecDeque::new(),
            scheduled_sound_events: VecDeque::new(),
            max_pending_spawns,
            max_active_instances,
            dropped_spawns: 0,
            instances_created: 0,
            instances_expired: 0,
            dropped_active_instances: 0,
            limited_particle_drops: 0,
            limited_particle_counts: BTreeMap::new(),
            random,
            sound_random: ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED.wrapping_add(1)),
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

        self.pending_sound_events.extend(batch.sound_events);
        self.scheduled_sound_events
            .extend(batch.scheduled_sound_events);

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
        self.advance_with_collision(ticks, |query| query.movement)
    }

    pub(crate) fn advance_with_collision<F>(
        &mut self,
        ticks: u32,
        collide: F,
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        self.advance_with_world(ticks, collide, |_| {
            ParticleBlockFluidSurfaceSample::default()
        })
    }

    pub(crate) fn advance_with_world<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            None,
            None,
            &[],
        )
    }

    pub(crate) fn advance_with_world_and_scope_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_with_world_and_player_context(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            None,
        )
    }

    pub(crate) fn advance_with_world_and_player_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            &[],
        )
    }

    pub(crate) fn advance_with_world_and_particle_contexts<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_with_world_and_particle_contexts_and_sound_camera(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            entity_target_contexts,
            None,
        )
    }

    pub(crate) fn advance_with_world_and_particle_contexts_and_sound_camera<F, S>(
        &mut self,
        ticks: u32,
        mut collide: F,
        mut block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
        sound_camera_position: Option<[f64; 3]>,
    ) -> ParticleAdvanceSummary
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let mut intaken_instances = 0;
        let mut expired_instances = 0;
        let mut dropped_active_instances = 0;
        let mut limited_particle_drops = 0;

        if ticks == 0 {
            self.release_due_scheduled_sound_events(sound_camera_position);
            self.drain_pending_spawns(
                &mut intaken_instances,
                &mut dropped_active_instances,
                &mut limited_particle_drops,
                scope_context,
            );
        } else {
            for _ in 0..ticks {
                expired_instances += self.tick_active_instances(
                    &mut collide,
                    &mut block_fluid_surface,
                    scope_context,
                    local_player_motion_context,
                    entity_target_contexts,
                );
                self.advance_scheduled_sound_events(sound_camera_position);
                self.drain_pending_spawns(
                    &mut intaken_instances,
                    &mut dropped_active_instances,
                    &mut limited_particle_drops,
                    scope_context,
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

    fn release_due_scheduled_sound_events(&mut self, camera_position: Option<[f64; 3]>) {
        self.update_scheduled_sound_events(camera_position, false);
    }

    fn advance_scheduled_sound_events(&mut self, camera_position: Option<[f64; 3]>) {
        self.update_scheduled_sound_events(camera_position, true);
    }

    fn update_scheduled_sound_events(
        &mut self,
        camera_position: Option<[f64; 3]>,
        decrement_ticks: bool,
    ) {
        let mut retained = VecDeque::with_capacity(self.scheduled_sound_events.len());
        while let Some(mut scheduled) = self.scheduled_sound_events.pop_front() {
            if decrement_ticks {
                scheduled.delay_ticks = scheduled.delay_ticks.saturating_sub(1);
            }
            if scheduled.delay_ticks == 0 {
                self.pending_sound_events
                    .push_back(scheduled.into_sound_event(camera_position));
            } else {
                retained.push_back(scheduled);
            }
        }
        self.scheduled_sound_events = retained;
    }

    fn tick_active_instances<F, S>(
        &mut self,
        collide: &mut F,
        block_fluid_surface: &mut S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) -> usize
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let mut expired_instances = 0;
        let mut active_instances = VecDeque::with_capacity(self.active_instances.len());
        while let Some(mut instance) = self.active_instances.pop_front() {
            if instance.delay_ticks > 0 {
                instance.delay_ticks = instance.delay_ticks.saturating_sub(1);
                active_instances.push_back(instance);
                continue;
            }
            if instance.age_ticks >= instance.lifetime_ticks {
                self.enqueue_removed_child_spawns_from(
                    &instance,
                    ParticleRemovalReason::LifetimeExpired,
                );
                self.enqueue_removed_sound_events_from(
                    &instance,
                    ParticleRemovalReason::LifetimeExpired,
                );
                self.decrement_particle_limit(instance.particle_limit);
                expired_instances += 1;
                continue;
            }
            instance.tick_motion(
                &mut self.random,
                collide,
                block_fluid_surface,
                entity_target_contexts,
            );
            if instance.removed {
                self.enqueue_removed_child_spawns_from(
                    &instance,
                    ParticleRemovalReason::RemovedDuringTick,
                );
                self.enqueue_removed_sound_events_from(
                    &instance,
                    ParticleRemovalReason::RemovedDuringTick,
                );
                self.decrement_particle_limit(instance.particle_limit);
                expired_instances += 1;
                continue;
            }
            instance.update_player_cloud_motion(local_player_motion_context);
            instance.age_ticks = instance.age_ticks.saturating_add(1);
            instance.update_sprite_from_age();
            instance.update_alpha_from_age();
            instance.update_spell_scope_alpha(scope_context);
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

    fn enqueue_removed_child_spawns_from(
        &mut self,
        instance: &ParticleInstance,
        reason: ParticleRemovalReason,
    ) {
        for command in instance.removal_child_spawn_commands(reason) {
            self.queue_pending_spawn(command);
        }
    }

    fn enqueue_removed_sound_events_from(
        &mut self,
        instance: &ParticleInstance,
        reason: ParticleRemovalReason,
    ) {
        if let Some(event) = instance.removal_sound_event(reason, &mut self.sound_random) {
            self.pending_sound_events.push_back(event);
        }
    }

    fn drain_pending_spawns(
        &mut self,
        intaken_instances: &mut usize,
        dropped_active_instances: &mut usize,
        limited_particle_drops: &mut usize,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) {
        while let Some(command) = self.pending_spawns.pop_front() {
            if self.max_active_instances == 0 {
                *dropped_active_instances += 1;
                continue;
            }
            let instance = if scope_context.is_some() {
                ParticleInstance::from_spawn_command_with_scope_context(
                    command,
                    &mut self.random,
                    scope_context,
                )
            } else {
                ParticleInstance::from_spawn_command(command, &mut self.random)
            };
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

    pub(crate) fn drain_sound_events(&mut self) -> Vec<ParticleSoundEvent> {
        self.pending_sound_events.drain(..).collect()
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
        Self::from_spawn_command_with_scope_context(command, random, None)
    }

    fn from_spawn_command_with_scope_context(
        command: ParticleSpawnCommand,
        random: &mut ParticleRandom,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) -> Self {
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
            if let Some(sprite_id) = fixed_item_particle_sprite_id(&command.particle_id) {
                (None, Some(sprite_id.to_string()))
            } else {
                select_initial_sprite(&command.sprite_ids, descriptor.sprite_selection, random)
            };
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
        let original_alpha = color[3];
        let color_fade_target = if descriptor.provider == "FireworkParticles.SparkProvider" {
            command
                .option_color_to
                .map(|color| [color[0], color[1], color[2]])
                .or_else(|| descriptor.color_fade_target())
        } else {
            descriptor.color_fade_target()
        };
        let [collision_width, collision_height] = descriptor.collision_size().unwrap_or([
            DEFAULT_PARTICLE_COLLISION_WIDTH,
            DEFAULT_PARTICLE_COLLISION_HEIGHT,
        ]);
        let item_pickup_target = if command.particle_id == ITEM_PICKUP_PARTICLE_ID {
            command.option_target.or(Some(position))
        } else {
            None
        };
        let is_firework_spark = descriptor.provider == "FireworkParticles.SparkProvider";
        let firework_trail = is_firework_spark && command.option_firework_trail;
        let firework_twinkle = is_firework_spark && command.option_firework_twinkle;
        let firework_half_lifetime_age =
            is_firework_spark && command.option_firework_half_lifetime_age;
        let mut instance = Self {
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
            original_alpha,
            color_fade_target,
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
            moves_without_collision: descriptor.moves_without_collision(),
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            collision_width,
            collision_height,
            on_ground: false,
            hit_ground: false,
            stopped_by_collision: false,
            removed: false,
            tick_motion: descriptor.tick_motion(),
            drip_fluid: descriptor.drip_fluid(),
            required_fluid: descriptor.required_fluid(),
            air_downward_acceleration: descriptor.air_downward_acceleration(),
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
            option_entity_target_source: command.option_entity_target_source,
            option_duration_ticks: command.option_duration_ticks,
            option_roll: command.option_roll,
            option_block: command.option_block,
            option_item: command.option_item,
            option_item_pickup_source_entity_id: command.option_item_pickup_source_entity_id,
            option_item_pickup_age_ticks: command.option_item_pickup_age_ticks,
            option_item_pickup_light: command.option_item_pickup_light,
            option_item_pickup_experience_orb_icon: command.option_item_pickup_experience_orb_icon,
            firework_trail,
            firework_twinkle,
            item_pickup_previous_target: item_pickup_target,
            item_pickup_target,
            atlas_uv_sub_rect,
        };
        if firework_half_lifetime_age {
            instance.age_ticks = instance.lifetime_ticks / 2;
        }
        instance.apply_constructor_tick_on_spawn();
        instance.apply_spell_scope_alpha_on_spawn(scope_context);
        instance
    }

    fn apply_constructor_tick_on_spawn(&mut self) {
        if !self.provider.starts_with("CritParticle.") {
            return;
        }
        self.previous_position = self.position;
        self.velocity[1] -= 0.04 * f64::from(self.gravity);
        self.position[0] += self.velocity[0];
        self.position[1] += self.velocity[1];
        self.position[2] += self.velocity[2];
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
        self.age_ticks = self.age_ticks.saturating_add(1);
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

    #[cfg(test)]
    fn tick_motion_without_collision(&mut self, random: &mut ParticleRandom) {
        self.tick_motion(
            random,
            &mut |query| query.movement,
            &mut |_| ParticleBlockFluidSurfaceSample::default(),
            &[],
        );
    }

    fn tick_motion<F, S>(
        &mut self,
        random: &mut ParticleRandom,
        collide: &mut F,
        block_fluid_surface: &mut S,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.previous_position = self.position;
        match self.tick_motion {
            ParticleTickMotionDescriptor::DefaultParticleTick => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.speed_up_when_y_motion_is_blocked && self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                self.remove_if_outside_required_fluid(block_fluid_surface);
                self.apply_air_downward_acceleration(block_fluid_surface);
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
                self.move_particle(self.velocity, collide);
                self.remove_if_outside_required_fluid(block_fluid_surface);
                if self.on_ground {
                    self.removed = true;
                }
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
                let Some(target) = self.vibration_target(entity_target_contexts) else {
                    self.removed = true;
                    return;
                };
                self.option_target = Some(target);
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
                if self.color[3] <= 0.0 {
                    self.removed = true;
                    return;
                }
                self.velocity[0] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[2] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
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
                self.remove_if_inside_matching_fluid(block_fluid_surface);
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
                self.remove_if_inside_matching_fluid(block_fluid_surface);
            }
            ParticleTickMotionDescriptor::DripFalling => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                if self.on_ground {
                    self.removed = true;
                } else {
                    let friction = f64::from(self.friction);
                    self.velocity[0] *= friction;
                    self.velocity[1] *= friction;
                    self.velocity[2] *= friction;
                    self.remove_if_inside_matching_fluid(block_fluid_surface);
                }
            }
            ParticleTickMotionDescriptor::DripFallAndLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                if self.on_ground {
                    self.removed = true;
                } else {
                    let friction = f64::from(self.friction);
                    self.velocity[0] *= friction;
                    self.velocity[1] *= friction;
                    self.velocity[2] *= friction;
                    self.remove_if_inside_matching_fluid(block_fluid_surface);
                }
            }
            ParticleTickMotionDescriptor::DripLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                self.remove_if_inside_matching_fluid(block_fluid_surface);
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
                self.move_particle(self.velocity, collide);
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    if random.next_f32() < 0.5 {
                        self.removed = true;
                    }
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
                    position: self.position,
                });
                let surface_height = surface.max_surface_height();
                if surface_height.is_finite() && surface_height > 0.0 {
                    let block_y = self.position[1].floor();
                    if self.position[1] < block_y + surface_height {
                        self.removed = true;
                    }
                }
            }
            ParticleTickMotionDescriptor::Wake => {
                let life =
                    60_u32.saturating_sub(self.lifetime_ticks.saturating_sub(self.age_ticks));
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
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
            ParticleTickMotionDescriptor::DragonBreath => {
                if self.on_ground {
                    self.velocity[1] = 0.0;
                    self.hit_ground = true;
                }
                if self.hit_ground {
                    self.velocity[1] += 0.002;
                }
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[2] *= friction;
                if self.hit_ground {
                    self.velocity[1] *= friction;
                }
            }
            ParticleTickMotionDescriptor::Firefly => {
                let next_age = self.age_ticks.saturating_add(1);
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.speed_up_when_y_motion_is_blocked && self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                self.remove_if_inside_non_air_block(block_fluid_surface);
                if self.removed {
                    return;
                }

                if random.next_f32() > 0.95 || next_age == 1 {
                    self.velocity = [
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                    ];
                }
            }
            ParticleTickMotionDescriptor::FallingLeaves => {
                self.tick_falling_leaves(collide);
            }
            ParticleTickMotionDescriptor::FallingDust => {
                if self.on_ground {
                    self.previous_roll = 0.0;
                    self.roll = 0.0;
                } else {
                    self.previous_roll = self.roll;
                    self.roll += std::f32::consts::PI * self.roll_speed * 2.0;
                }
                self.move_particle(self.velocity, collide);
                self.velocity[1] = (self.velocity[1] - 0.003).max(-0.14);
            }
            ParticleTickMotionDescriptor::ItemPickup => {
                self.tick_item_pickup(entity_target_contexts);
            }
        }
    }

    fn remove_if_inside_matching_fluid<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let Some(expected_fluid) = self.drip_fluid else {
            return;
        };
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.fluid_kind != Some(expected_fluid)
            || !surface.fluid_height.is_finite()
            || surface.fluid_height <= 0.0
        {
            return;
        }
        let block_y = self.position[1].floor();
        if self.position[1] < block_y + surface.fluid_height {
            self.removed = true;
        }
    }

    fn remove_if_outside_required_fluid<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let Some(required_fluid) = self.required_fluid else {
            return;
        };
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.fluid_kind != Some(required_fluid) {
            self.removed = true;
        }
    }

    fn apply_air_downward_acceleration<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        if self.removed || self.air_downward_acceleration == 0.0 {
            return;
        }
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.block_is_air {
            self.velocity[1] -= self.air_downward_acceleration;
        }
    }

    fn remove_if_inside_non_air_block<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if !surface.block_is_air {
            self.removed = true;
        }
    }

    fn move_particle<F>(&mut self, movement: [f64; 3], collide: &mut F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        if self.stopped_by_collision {
            return;
        }

        let mut adjusted = movement;
        if self.has_physics
            && !self.moves_without_collision
            && movement.iter().any(|value| *value != 0.0)
            && motion_length_squared(movement) < 10_000.0
        {
            adjusted = collide(ParticleCollisionQuery {
                position: self.position,
                movement,
                half_width: f64::from(self.collision_width) / 2.0,
                height: f64::from(self.collision_height),
            });
        }

        if adjusted.iter().any(|value| *value != 0.0) {
            self.position[0] += adjusted[0];
            self.position[1] += adjusted[1];
            self.position[2] += adjusted[2];
        }

        if movement[1].abs() >= 1.0e-5 && adjusted[1].abs() < 1.0e-5 {
            self.stopped_by_collision = true;
        }
        self.on_ground = movement[1] != adjusted[1] && movement[1] < 0.0;
        if movement[0] != adjusted[0] {
            self.velocity[0] = 0.0;
        }
        if movement[2] != adjusted[2] {
            self.velocity[2] = 0.0;
        }
    }

    fn tick_falling_leaves<F>(&mut self, collide: &mut F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
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
        self.move_particle(self.velocity, collide);
        if self.on_ground
            || (alive_ticks > 1 && (self.velocity[0] == 0.0 || self.velocity[2] == 0.0))
        {
            self.removed = true;
            return;
        }
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
    }

    fn tick_item_pickup(&mut self, entity_target_contexts: &[ParticleEntityTargetContext]) {
        let current_target = self
            .item_pickup_target
            .or(self.option_target)
            .unwrap_or(self.position);
        self.item_pickup_previous_target = Some(current_target);
        self.item_pickup_target = Some(
            self.item_pickup_target_from_context(entity_target_contexts)
                .unwrap_or(current_target),
        );
        if self.age_ticks.saturating_add(1) >= ITEM_PICKUP_PARTICLE_LIFETIME_TICKS {
            self.removed = true;
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

    fn apply_spell_scope_alpha_on_spawn(
        &mut self,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) {
        if self.provider == "SpellParticle.MobEffectProvider" || !self.is_spell_particle() {
            return;
        }
        if scope_context.is_some_and(|context| self.is_close_to_scoping_player(context)) {
            self.color[3] = 0.0;
        }
    }

    fn update_spell_scope_alpha(&mut self, scope_context: Option<ParticleLocalPlayerScopeContext>) {
        if !self.is_spell_particle() {
            return;
        }
        if scope_context.is_some_and(|context| self.is_close_to_scoping_player(context)) {
            self.color[3] = 0.0;
        } else {
            self.color[3] = lerp_f32(0.05, self.color[3], self.original_alpha);
        }
    }

    fn is_spell_particle(&self) -> bool {
        self.provider.starts_with("SpellParticle.")
    }

    fn is_close_to_scoping_player(&self, context: ParticleLocalPlayerScopeContext) -> bool {
        if !context.first_person || !context.scoping {
            return false;
        }
        let dx = context.eye_position[0] - self.position[0];
        let dy = context.eye_position[1] - self.position[1];
        let dz = context.eye_position[2] - self.position[2];
        dx * dx + dy * dy + dz * dz <= 9.0
    }

    fn vibration_target(&self, contexts: &[ParticleEntityTargetContext]) -> Option<[f64; 3]> {
        let Some(source) = self.option_entity_target_source else {
            return self.option_target;
        };
        contexts
            .iter()
            .find(|context| context.entity_id == source.entity_id)
            .map(|context| {
                [
                    context.position[0],
                    context.position[1] + f64::from(source.y_offset),
                    context.position[2],
                ]
            })
    }

    fn item_pickup_target_from_context(
        &self,
        contexts: &[ParticleEntityTargetContext],
    ) -> Option<[f64; 3]> {
        let source = self.option_entity_target_source?;
        contexts
            .iter()
            .find(|context| context.entity_id == source.entity_id)
            .map(|context| {
                [
                    context.position[0],
                    context.position[1] + f64::from(source.y_offset),
                    context.position[2],
                ]
            })
    }

    fn item_pickup_position_at_partial_tick(&self, partial_tick: f32) -> Option<[f64; 3]> {
        if self.render_group != ParticleRenderGroup::ItemPickup {
            return None;
        }
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let time = ((self.age_ticks as f32 + partial_tick)
            / ITEM_PICKUP_PARTICLE_LIFETIME_TICKS as f32)
            .powi(2);
        let previous_target = self
            .item_pickup_previous_target
            .or(self.item_pickup_target)
            .or(self.option_target)?;
        let target = self.item_pickup_target.or(self.option_target)?;
        let target = [
            lerp_f64(f64::from(partial_tick), previous_target[0], target[0]),
            lerp_f64(f64::from(partial_tick), previous_target[1], target[1]),
            lerp_f64(f64::from(partial_tick), previous_target[2], target[2]),
        ];
        Some([
            lerp_f64(f64::from(time), self.start_position[0], target[0]),
            lerp_f64(f64::from(time), self.start_position[1], target[1]),
            lerp_f64(f64::from(time), self.start_position[2], target[2]),
        ])
    }

    fn update_player_cloud_motion(&mut self, context: Option<ParticleLocalPlayerMotionContext>) {
        if !matches!(
            self.provider.as_str(),
            "PlayerCloudParticle.Provider" | "PlayerCloudParticle.SneezeProvider"
        ) {
            return;
        }
        let Some(context) = context else {
            return;
        };
        let dx = context.position[0] - self.position[0];
        let dy = context.position[1] - self.position[1];
        let dz = context.position[2] - self.position[2];
        if dx * dx + dy * dy + dz * dz > 4.0 || self.position[1] <= context.position[1] {
            return;
        }
        self.position[1] += (context.position[1] - self.position[1]) * 0.2;
        self.velocity[1] += (context.delta_movement[1] - self.velocity[1]) * 0.2;
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
        let mut commands: Vec<_> = self
            .firework_trail_child_spawn_command()
            .into_iter()
            .collect();
        let mut descriptor_commands = match self.child_emission {
            Some(ParticleChildEmissionDescriptor::LavaSmoke) => self
                .lava_child_smoke_spawn_command(random)
                .into_iter()
                .collect(),
            Some(
                ParticleChildEmissionDescriptor::DripHangToFall
                | ParticleChildEmissionDescriptor::DripFallAndLand,
            ) => Vec::new(),
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
        };
        commands.append(&mut descriptor_commands);
        commands
    }

    fn firework_trail_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        if self.provider != "FireworkParticles.SparkProvider"
            || !self.firework_trail
            || self.age_ticks >= self.lifetime_ticks / 2
            || (self.age_ticks + self.lifetime_ticks) % 2 != 0
        {
            return None;
        }
        Some(ParticleSpawnCommand {
            particle_type_id: self.particle_type_id,
            particle_id: self.particle_id.clone(),
            sprite_ids: self.sprite_ids.clone(),
            position: self.position,
            velocity: [0.0, 0.0, 0.0],
            override_limiter: self.override_limiter,
            always_show: self.always_show,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: Vec::new(),
            option_color: Some([self.color[0], self.color[1], self.color[2], 0.99]),
            option_color_to: self
                .color_fade_target
                .map(|target| [target[0], target[1], target[2], 1.0]),
            option_scale: None,
            option_power: None,
            option_target: None,
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_firework_trail: false,
            option_firework_twinkle: self.firework_twinkle,
            option_firework_half_lifetime_age: true,
        })
    }

    fn removal_child_spawn_commands(
        &self,
        reason: ParticleRemovalReason,
    ) -> Vec<ParticleSpawnCommand> {
        match (self.child_emission, reason) {
            (
                Some(ParticleChildEmissionDescriptor::DripHangToFall),
                ParticleRemovalReason::LifetimeExpired,
            ) => self
                .drip_hang_falling_child_spawn_command()
                .into_iter()
                .collect(),
            (
                Some(ParticleChildEmissionDescriptor::DripFallAndLand),
                ParticleRemovalReason::RemovedDuringTick,
            ) if self.on_ground => self
                .drip_landing_child_spawn_command()
                .into_iter()
                .collect(),
            _ => Vec::new(),
        }
    }

    fn removal_sound_event(
        &self,
        reason: ParticleRemovalReason,
        random: &mut ParticleRandom,
    ) -> Option<ParticleSoundEvent> {
        if reason != ParticleRemovalReason::RemovedDuringTick || !self.on_ground {
            return None;
        }
        let sound_event_id = match self.particle_id.as_str() {
            "minecraft:falling_honey" => "minecraft:block.beehive.drip",
            "minecraft:falling_dripstone_lava" => "minecraft:block.pointed_dripstone.drip_lava",
            "minecraft:falling_dripstone_water" => "minecraft:block.pointed_dripstone.drip_water",
            _ => return None,
        };
        Some(ParticleSoundEvent {
            sound_event_id: sound_event_id.to_string(),
            source: "block".to_string(),
            position: self.position,
            volume: 0.3 + random.next_f32() * 0.7,
            pitch: 1.0,
            seed: random.next_i64(),
            distance_delay: false,
        })
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
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
        })
    }

    fn drip_hang_falling_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        let child_particle_id = match self.particle_id.as_str() {
            "minecraft:dripping_honey" => "minecraft:falling_honey",
            "minecraft:dripping_obsidian_tear" => "minecraft:falling_obsidian_tear",
            "minecraft:dripping_lava" => "minecraft:falling_lava",
            "minecraft:dripping_water" => "minecraft:falling_water",
            "minecraft:dripping_dripstone_lava" => "minecraft:falling_dripstone_lava",
            "minecraft:dripping_dripstone_water" => "minecraft:falling_dripstone_water",
            _ => return None,
        };
        self.drip_child_spawn_command(child_particle_id, self.position, self.velocity)
    }

    fn drip_landing_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        let child_particle_id = match self.particle_id.as_str() {
            "minecraft:falling_honey" => "minecraft:landing_honey",
            "minecraft:falling_obsidian_tear" => "minecraft:landing_obsidian_tear",
            "minecraft:falling_lava" | "minecraft:falling_dripstone_lava" => {
                "minecraft:landing_lava"
            }
            "minecraft:falling_water" | "minecraft:falling_dripstone_water" => "minecraft:splash",
            _ => return None,
        };
        self.drip_child_spawn_command(child_particle_id, self.position, [0.0, 0.0, 0.0])
    }

    fn drip_child_spawn_command(
        &self,
        child_particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
    ) -> Option<ParticleSpawnCommand> {
        let template = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == child_particle_id)?;
        Some(ParticleSpawnCommand {
            particle_type_id: template.particle_type_id,
            particle_id: template.particle_id.clone(),
            sprite_ids: template.sprite_ids.clone(),
            position,
            velocity,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: self
                .child_spawn_templates
                .iter()
                .filter(|template| template.particle_id != child_particle_id)
                .cloned()
                .collect(),
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
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
                    option_entity_target_source: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                    option_item_pickup_source_entity_id: None,
                    option_item_pickup_age_ticks: None,
                    option_item_pickup_light: None,
                    option_item_pickup_experience_orb_icon: None,
                    option_firework_trail: false,
                    option_firework_twinkle: false,
                    option_firework_half_lifetime_age: false,
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
                    option_entity_target_source: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                    option_item_pickup_source_entity_id: None,
                    option_item_pickup_age_ticks: None,
                    option_item_pickup_light: None,
                    option_item_pickup_experience_orb_icon: None,
                    option_firework_trail: false,
                    option_firework_twinkle: false,
                    option_firework_half_lifetime_age: false,
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

    pub fn update_particle_atlas(&mut self, rgba: &[u8]) -> Result<()> {
        let Some(atlas) = self.particle_atlas.as_ref() else {
            return Ok(());
        };
        update_particle_atlas_gpu(&self.queue, atlas, rgba)
    }

    pub fn set_terrain_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        let (uvs, translucent_sprites) = particle_sprite_uv_map(sprite_uvs);
        self.terrain_particle_sprite_uvs = uvs;
        self.terrain_particle_translucent_sprites = translucent_sprites;
    }

    pub fn set_item_particle_sprite_uvs(&mut self, sprite_uvs: Vec<ParticleSpriteUv>) {
        let (uvs, translucent_sprites) = particle_sprite_uv_map(sprite_uvs);
        self.item_particle_sprite_uvs = uvs;
        self.item_particle_translucent_sprites = translucent_sprites;
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
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_collision<F>(&mut self, ticks: u32, collide: F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        let summary = self.particles.advance_with_collision(ticks, collide);
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self
            .particles
            .advance_with_world(ticks, collide, block_fluid_surface);
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_scope_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self.particles.advance_with_world_and_scope_context(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
        );
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_player_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.advance_particles_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            &[],
        );
    }

    pub fn advance_particles_with_world_and_particle_contexts<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self.particles.advance_with_world_and_particle_contexts(
            ticks,
            collide,
            block_fluid_surface,
            scope_context,
            local_player_motion_context,
            entity_target_contexts,
        );
        self.record_particle_advance_summary(summary);
    }

    pub fn advance_particles_with_world_and_particle_contexts_and_sound_camera<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        local_player_motion_context: Option<ParticleLocalPlayerMotionContext>,
        entity_target_contexts: &[ParticleEntityTargetContext],
        sound_camera_position: Option<[f64; 3]>,
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let summary = self
            .particles
            .advance_with_world_and_particle_contexts_and_sound_camera(
                ticks,
                collide,
                block_fluid_surface,
                scope_context,
                local_player_motion_context,
                entity_target_contexts,
                sound_camera_position,
            );
        self.record_particle_advance_summary(summary);
    }

    pub fn drain_particle_sound_events(&mut self) -> Vec<ParticleSoundEvent> {
        self.particles.drain_sound_events()
    }

    fn record_particle_advance_summary(&mut self, summary: ParticleAdvanceSummary) {
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
            terrain_translucent_sprites: Some(&self.terrain_particle_translucent_sprites),
            item_translucent_sprites: Some(&self.item_particle_translucent_sprites),
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

    pub(crate) fn collect_elder_guardian_particle_render_instances(
        &self,
    ) -> Vec<ElderGuardianParticleRenderInstance> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        elder_guardian_particle_render_instances(self.particles.active_instances.iter(), pose)
    }

    pub(crate) fn collect_experience_orb_pickup_particle_render_instances(
        &self,
    ) -> Vec<ExperienceOrbPickupParticleRenderInstance> {
        let Some(pose) = self.camera_pose else {
            return Vec::new();
        };
        experience_orb_pickup_particle_render_instances(
            self.particles.active_instances.iter(),
            pose,
        )
    }

    pub fn item_pickup_particle_render_states(&self) -> Vec<ItemPickupParticleRenderState> {
        item_pickup_particle_render_states(self.particles.active_instances.iter())
    }
}

fn particle_sprite_uv_map(
    sprite_uvs: Vec<ParticleSpriteUv>,
) -> (BTreeMap<String, ParticleUvRect>, BTreeSet<String>) {
    let mut uvs = BTreeMap::new();
    let mut translucent_sprites = BTreeSet::new();
    for sprite in sprite_uvs {
        if sprite.has_translucent {
            translucent_sprites.insert(sprite.id.clone());
        }
        uvs.insert(sprite.id, sprite.uv);
    }
    (uvs, translucent_sprites)
}

#[derive(Debug, Clone, Copy)]
struct ParticleAtlasUvSets<'a> {
    particles: Option<&'a BTreeMap<String, ParticleUvRect>>,
    terrain: Option<&'a BTreeMap<String, ParticleUvRect>>,
    items: Option<&'a BTreeMap<String, ParticleUvRect>>,
    terrain_translucent_sprites: Option<&'a BTreeSet<String>>,
    item_translucent_sprites: Option<&'a BTreeSet<String>>,
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

    fn has_translucent_sprite(
        self,
        texture_atlas: ParticleTextureAtlasKind,
        sprite_id: &str,
    ) -> bool {
        match texture_atlas {
            ParticleTextureAtlasKind::Particles => false,
            ParticleTextureAtlasKind::Terrain => self
                .terrain_translucent_sprites
                .is_some_and(|sprites| sprites.contains(sprite_id)),
            ParticleTextureAtlasKind::Items => self
                .item_translucent_sprites
                .is_some_and(|sprites| sprites.contains(sprite_id)),
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
        .filter_map(|instance| {
            let sprite_id = instance.current_sprite_id.as_deref()?;
            let render_layer = particle_render_layer_for_sprite(instance, atlas_uvs, sprite_id);
            (render_layer.pipeline_kind() == pipeline_kind).then_some((instance, render_layer))
        })
        .collect();
    instances.sort_by_key(|(_, render_layer)| {
        (
            ParticleRenderGroup::SingleQuads.vanilla_order(),
            render_layer.vanilla_solid_translucent_order(),
        )
    });
    for (instance, _) in instances {
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

fn particle_render_layer_for_sprite(
    instance: &ParticleInstance,
    atlas_uvs: ParticleAtlasUvSets<'_>,
    sprite_id: &str,
) -> ParticleRenderLayer {
    let has_translucent = atlas_uvs.has_translucent_sprite(instance.texture_atlas, sprite_id);
    match instance.render_layer {
        ParticleRenderLayer::OpaqueTerrain | ParticleRenderLayer::TranslucentTerrain => {
            if has_translucent {
                ParticleRenderLayer::TranslucentTerrain
            } else {
                ParticleRenderLayer::OpaqueTerrain
            }
        }
        ParticleRenderLayer::OpaqueItems | ParticleRenderLayer::TranslucentItems => {
            if has_translucent {
                ParticleRenderLayer::TranslucentItems
            } else {
                ParticleRenderLayer::OpaqueItems
            }
        }
        render_layer => render_layer,
    }
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

fn elder_guardian_particle_render_instances<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    pose: crate::CameraPose,
) -> Vec<ElderGuardianParticleRenderInstance> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ElderGuardians)
        .filter(|instance| instance.delay_ticks == 0)
        .map(|instance| {
            let age_scale = elder_guardian_particle_age_scale(
                instance.age_ticks,
                instance.lifetime_ticks,
                DEFAULT_PARTICLE_RENDER_PARTIAL_TICK,
            );
            ElderGuardianParticleRenderInstance {
                transform: elder_guardian_particle_model_transform(pose, age_scale),
                tint: [1.0, 1.0, 1.0, elder_guardian_particle_alpha(age_scale)],
            }
        })
        .collect()
}

fn item_pickup_particle_render_states<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
) -> Vec<ItemPickupParticleRenderState> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ItemPickup)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(|instance| {
            let item = instance.option_item?;
            let source_entity_id = instance.option_item_pickup_source_entity_id?;
            let position = instance
                .item_pickup_position_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)?;
            Some(ItemPickupParticleRenderState {
                source_entity_id,
                item,
                position: [position[0] as f32, position[1] as f32, position[2] as f32],
                age_ticks: instance
                    .option_item_pickup_age_ticks
                    .unwrap_or(instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK),
                light: instance
                    .option_item_pickup_light
                    .unwrap_or(DEFAULT_PARTICLE_LIGHT),
            })
        })
        .collect()
}

fn experience_orb_pickup_particle_render_states<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
) -> Vec<ExperienceOrbPickupParticleRenderState> {
    instances
        .into_iter()
        .filter(|instance| instance.render_group == ParticleRenderGroup::ItemPickup)
        .filter(|instance| instance.delay_ticks == 0)
        .filter_map(|instance| {
            let icon = instance.option_item_pickup_experience_orb_icon?;
            let source_entity_id = instance.option_item_pickup_source_entity_id?;
            let position = instance
                .item_pickup_position_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)?;
            Some(ExperienceOrbPickupParticleRenderState {
                source_entity_id,
                icon,
                position: [position[0] as f32, position[1] as f32, position[2] as f32],
                age_ticks: instance
                    .option_item_pickup_age_ticks
                    .unwrap_or(instance.age_ticks as f32 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK),
                light: instance
                    .option_item_pickup_light
                    .unwrap_or(DEFAULT_PARTICLE_LIGHT),
            })
        })
        .collect()
}

fn experience_orb_pickup_particle_render_instances<'a>(
    instances: impl IntoIterator<Item = &'a ParticleInstance>,
    pose: crate::CameraPose,
) -> Vec<ExperienceOrbPickupParticleRenderInstance> {
    experience_orb_pickup_particle_render_states(instances)
        .into_iter()
        .map(|state| ExperienceOrbPickupParticleRenderInstance {
            transform: experience_orb_pickup_particle_model_transform(pose, state.position),
            icon: state.icon,
            age_ticks: state.age_ticks,
            light: state.light,
        })
        .collect()
}

fn experience_orb_pickup_particle_model_transform(
    pose: crate::CameraPose,
    position: [f32; 3],
) -> Mat4 {
    let axes = camera_billboard_axes(pose);
    let forward = axes.right.cross(axes.up).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let orientation = Mat4::from_cols(
        axes.right.extend(0.0),
        axes.up.extend(0.0),
        (-forward).extend(0.0),
        Vec3::ZERO.extend(1.0),
    );
    Mat4::from_translation(Vec3::from_array(position) + Vec3::Y * 0.1)
        * orientation
        * Mat4::from_scale(Vec3::splat(0.3))
}

fn elder_guardian_particle_model_transform(pose: crate::CameraPose, age_scale: f32) -> Mat4 {
    camera_to_world_transform(pose)
        * Mat4::from_rotation_x((60.0 - 150.0 * age_scale).to_radians())
        * Mat4::from_scale(Vec3::new(
            ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
            -ELDER_GUARDIAN_PARTICLE_MODEL_SCALE,
        ))
        * Mat4::from_translation(Vec3::new(0.0, -0.56, 3.5))
        * Mat4::from_scale(Vec3::splat(ELDER_GUARDIAN_PARTICLE_BAKED_LAYER_SCALE))
}

fn elder_guardian_particle_age_scale(
    age_ticks: u32,
    lifetime_ticks: u32,
    partial_tick: f32,
) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    (age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime
}

fn elder_guardian_particle_alpha(age_scale: f32) -> f32 {
    0.05 + 0.5 * (age_scale * std::f32::consts::PI).sin()
}

fn camera_to_world_transform(pose: crate::CameraPose) -> Mat4 {
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
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
    let up = if up.length_squared() > 0.0 {
        up
    } else {
        Vec3::Y
    };
    Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        (-forward).extend(0.0),
        eye.extend(1.0),
    )
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
    if firework_twinkle_hidden(instance) {
        color[3] = 0.0;
    }
    color
}

fn firework_twinkle_hidden(instance: &ParticleInstance) -> bool {
    instance.firework_twinkle
        && instance.age_ticks >= instance.lifetime_ticks / 3
        && ((instance.age_ticks + instance.lifetime_ticks) / 3) % 2 != 0
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

fn motion_length_squared(movement: [f64; 3]) -> f64 {
    movement[0] * movement[0] + movement[1] * movement[1] + movement[2] * movement[2]
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
        ITEM_PICKUP_PARTICLE_ID => ParticleRenderGroup::ItemPickup,
        ELDER_GUARDIAN_PARTICLE_ID => ParticleRenderGroup::ElderGuardians,
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
        | ELDER_GUARDIAN_PARTICLE_ID
        | "minecraft:infested"
        | "minecraft:raid_omen"
        | "minecraft:trial_omen"
        | "minecraft:witch" => ParticleRenderLayer::Translucent,
        _ => ParticleRenderLayer::Opaque,
    }
}

fn fixed_item_particle_sprite_id(particle_id: &str) -> Option<&'static str> {
    match particle_id {
        "minecraft:item_slime" => Some("minecraft:item/slime_ball"),
        "minecraft:item_cobweb" => Some("minecraft:block/cobweb"),
        "minecraft:item_snowball" => Some("minecraft:item/snowball"),
        _ => None,
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
mod tests;
