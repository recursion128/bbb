use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::Result;
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    entity_models::{
        ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
        ProjectilePickupParticleRenderInstance,
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
    /// Opaque, serialized `DataComponentPatchSummary` bytes for the picked-up
    /// item stack. The renderer never inspects this payload (it cannot name the
    /// protocol summary types); it round-trips the blob so the native
    /// item-pickup bake can rebuild the component-rich stack after the renderer
    /// owns the target interpolation. `None` for empty/default patches.
    #[serde(default)]
    pub option_item_pickup_component_patch: Option<Vec<u8>>,
    #[serde(default)]
    pub option_item_pickup_projectile_model: Option<ParticleItemPickupProjectileModel>,
    #[serde(default)]
    pub option_firework_trail: bool,
    #[serde(default)]
    pub option_firework_twinkle: bool,
    #[serde(default)]
    pub option_firework_half_lifetime_age: bool,
}

/// The carried projectile model for a `minecraft:item_pickup` particle whose
/// picked-up source entity is an arrow / spectral arrow / thrown trident.
/// Vanilla `ItemPickupParticleGroup.State.submit` renders the extracted
/// `EntityRenderState` through the entity render dispatcher;
/// `ArrowRenderer.submit` / `ThrownTridentRenderer.submit` orient the
/// projectile model with the entity's `yRot`/`xRot`. Projected by native from
/// the world pickup state (the renderer cannot name world/protocol types);
/// item stacks and experience orbs use the dedicated option fields instead.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParticleItemPickupProjectileModel {
    pub kind: ParticleItemPickupProjectileKind,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleItemPickupProjectileKind {
    /// Plain `arrow.png` (vanilla `TippableArrowRenderer`, `getColor() <= 0`).
    Arrow,
    /// `arrow_tipped.png` (vanilla `TippableArrowRenderer.isTipped`).
    TippedArrow,
    /// `arrow_spectral.png` (vanilla `SpectralArrowRenderer`).
    SpectralArrow,
    /// `trident.png`, plus the `entityGlint` foil pass when
    /// `ThrownTridentRenderState.isFoil` (vanilla `ThrownTridentRenderer`).
    Trident { foil: bool },
}

/// One `minecraft:item_pickup` particle whose carried model is a projectile
/// (arrow/trident), extracted at the vanilla quadratic-interpolated position
/// (`ItemPickupParticleGroup.ParticleInstance.fromParticle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectilePickupParticleRenderState {
    pub(crate) model: ParticleItemPickupProjectileModel,
    pub(crate) position: [f32; 3],
    pub(crate) light: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleLocalPlayerScopeContext {
    pub eye_position: [f64; 3],
    pub first_person: bool,
    pub scoping: bool,
}

/// One candidate for the per-particle nearest-player selection performed by
/// the PlayerCloud / Sneeze tick (vanilla
/// `level.getNearestPlayer(x, y, z, 2.0, false)`,
/// `PlayerCloudParticle.java:51`). The native pump projects the local player
/// plus every remote player entity, already filtered through vanilla
/// `EntitySelector.NO_SPECTATORS` (`EntityGetter.java:95-98`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticlePlayerMotionContext {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ItemPickupParticleRenderState {
    pub source_entity_id: i32,
    pub item: ParticleItemOptionState,
    /// Opaque, serialized `DataComponentPatchSummary` bytes carried through the
    /// pickup channel. The renderer treats it as an uninterpreted blob; the
    /// native bake deserializes it to rebuild the component-rich stack. `None`
    /// for empty/default patches.
    pub component_patch: Option<Vec<u8>>,
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
    pub(crate) option_item_pickup_component_patch: Option<Vec<u8>>,
    #[serde(default)]
    pub(crate) option_item_pickup_projectile_model: Option<ParticleItemPickupProjectileModel>,
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
            &[],
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
            &[],
        )
    }

    pub(crate) fn advance_with_world_and_player_context<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        player_motion_contexts: &[ParticlePlayerMotionContext],
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
            player_motion_contexts,
            &[],
        )
    }

    pub(crate) fn advance_with_world_and_particle_contexts<F, S>(
        &mut self,
        ticks: u32,
        collide: F,
        block_fluid_surface: S,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
        player_motion_contexts: &[ParticlePlayerMotionContext],
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
            player_motion_contexts,
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
        player_motion_contexts: &[ParticlePlayerMotionContext],
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
                    player_motion_contexts,
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
        player_motion_contexts: &[ParticlePlayerMotionContext],
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
            instance.update_player_cloud_motion(player_motion_contexts);
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

mod instance;
mod render;

use render::*;

#[cfg(test)]
mod tests;
