//! Particle meshing: single quads into the particle pipelines, plus the
//! item-pickup / elder-guardian particles drawn as entity and item models.

use anyhow::Result;
#[cfg(test)]
use bbb_render_types::ParticleFacingCameraMode;
pub use bbb_render_types::ParticleTextureAtlasKind;
use glam::{Mat4, Vec3};

pub use crate::entity_models::{
    ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
    ProjectilePickupParticleRenderInstance,
};
use crate::Renderer;

mod gpu;

#[cfg(test)]
pub(super) use gpu::ParticleQuad;
pub(super) use gpu::{
    create_particle_atlas_gpu, create_particle_core_pipelines, create_selected_particle_pipeline,
    update_particle_atlas_gpu, ParticleAtlasGpu, ParticleCorePipelines,
};
pub use gpu::{ParticleGpuInstance, ParticleQuadBuffer};

#[allow(clippy::approx_constant)]
pub const SHRIEK_MAGICAL_X_ROT: f32 = 1.0472;
const ELDER_GUARDIAN_PARTICLE_MODEL_SCALE: f32 = 0.425_531_92;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// One atlas sprite, at its atlas sprite index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleSpriteUv {
    pub uv: ParticleUvRect,
    pub has_translucent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleItemOptionState {
    pub item_id: i32,
    pub count: i32,
    pub component_patch_len: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemPickupParticleRenderState {
    pub source_entity_id: i32,
    pub item: ParticleItemOptionState,
    /// Component patch of the picked-up stack; `None` for default patches.
    pub component_patch: Option<std::sync::Arc<bbb_protocol::packets::DataComponentPatchSummary>>,
    pub position: [f32; 3],
    pub age_ticks: f32,
    pub bob_offset: f32,
    pub light: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// One frame's particle draw data, extracted from the client-tick engine.
#[derive(Default)]
pub struct ParticleFrame {
    pub quads: ParticleBatches,
    pub elder_guardians: Vec<ElderGuardianParticleRenderInstance>,
    pub experience_orbs: Vec<ExperienceOrbPickupParticleRenderInstance>,
    pub projectiles: Vec<ProjectilePickupParticleRenderInstance>,
    /// The engine has live particles of any render group.
    pub live: bool,
}

#[derive(Default)]
pub struct ParticleLightSamples {
    world_revision: u64,
    samples: Vec<Option<[f32; 2]>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParticleBatches {
    pub opaque: ParticleBatch,
    pub translucent: ParticleBatch,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParticleBatch {
    pub quads: ParticleQuadBuffer,
    pub draws: Vec<ParticleAtlasDrawRange>,
}

impl ParticleBatch {
    pub fn clear(&mut self) {
        self.quads.clear();
        self.draws.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleAtlasDrawRange {
    pub texture_atlas: ParticleTextureAtlasKind,
    pub instances: std::ops::Range<u32>,
}

mod render;

pub use render::{
    elder_guardian_particle_model_transform, projectile_pickup_particle_model_transform,
    ParticleAtlasUvs, ParticleExtractionTarget, ParticleQuadSources, ParticleSourceKey,
};

pub use crate::camera::camera_billboard_orientation;
