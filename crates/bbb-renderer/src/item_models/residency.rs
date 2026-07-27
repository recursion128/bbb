//! Resource-generation item-model CPU/GPU residency and narrow frame instances.
//!
//! The exact key is deliberately decided before a caller supplies model quads.
//! A resident hit therefore never scans or compares the vertex/index payload.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Instant,
};

use anyhow::{bail, Result};
use glam::{Mat3, Mat4};
use wgpu::util::DeviceExt;

use crate::{
    bake_generated_item_quads,
    counters::RendererCounters,
    entity_models::{bake_special_item_geometry, ResolvedSpecialItemTexture},
    frame_buffers::FrameDataBuffer,
    gpu::DEPTH_FORMAT,
    item_models::ItemModelVertex,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    EntityDynamicPlayerSkinStatus, ItemSpriteRect, Renderer, SpecialItemModel,
    SpecialItemModelBounds, SpecialItemTexture, SpriteAlphaMask,
};

use super::{
    bake_item_model_mesh, sheeted_decal_uv, ItemModelFoil, ItemModelQuad, ITEM_MODEL_GLINT_BLEND,
    ITEM_MODEL_GLINT_CULL_MODE, ITEM_MODEL_GLINT_DEPTH_COMPARE,
    ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED, ITEM_MODEL_PIPELINE_CULL_MODE,
    SPECIAL_FOIL_TEXTURE_SCALE,
};

const DEFAULT_MAX_RESIDENT_OWNERS: usize = 4_096;
const DEFAULT_MAX_RESIDENT_BYTES: u64 = 256 * 1024 * 1024;

/// Renderer-owned resource generation. A reload advances this value; keys from
/// an older generation are rejected rather than aliases for new resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemModelResourceGeneration(u64);

impl ItemModelResourceGeneration {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Renderer-owned atlas-layout generation for one item geometry material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemModelAtlasGeneration(u64);

impl ItemModelAtlasGeneration {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Texture atlas sampled by a resident item-model owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemModelAtlasKind {
    Blocks,
    Items,
    Entity,
    DynamicPlayerSkin,
    DynamicEntity,
}

/// Static geometry subset. Solid and translucent quads have separate owner
/// ranges, while z-offset and glint are dynamic draw phases over those owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemModelGeometryClass {
    Solid,
    Translucent,
}

/// Exact, payload-independent model selection identity.
///
/// `model_id` identifies the selected resource model. `variant` is the exact
/// canonical selector/property identity that changes static geometry (for
/// example a block-state property string or the selected generated layers),
/// and `part` splits independently tinted/static geometry groups. No entity or
/// level id belongs here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemModelAssetIdentity {
    model_id: Arc<str>,
    variant: Arc<str>,
    part: u32,
    portal_metadata: bool,
}

impl ItemModelAssetIdentity {
    pub fn new(model_id: impl Into<Arc<str>>, variant: impl Into<Arc<str>>, part: u32) -> Self {
        Self {
            model_id: model_id.into(),
            variant: variant.into(),
            part,
            portal_metadata: false,
        }
    }

    fn with_portal_metadata(mut self) -> Self {
        self.portal_metadata = true;
        self
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn variant(&self) -> &str {
        &self.variant
    }

    pub fn part(&self) -> u32 {
        self.part
    }
}

/// Exact resident owner key. Equality over all fields proves a cache hit; no
/// digest or geometry equality scan participates in the decision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemModelAssetKey {
    resource_generation: ItemModelResourceGeneration,
    atlas_generation: ItemModelAtlasGeneration,
    atlas: ItemModelAtlasKind,
    geometry_class: ItemModelGeometryClass,
    identity: ItemModelAssetIdentity,
}

impl ItemModelAssetKey {
    pub fn resource_generation(&self) -> ItemModelResourceGeneration {
        self.resource_generation
    }

    pub fn atlas_generation(&self) -> ItemModelAtlasGeneration {
        self.atlas_generation
    }

    pub fn atlas(&self) -> ItemModelAtlasKind {
        self.atlas
    }

    pub fn geometry_class(&self) -> ItemModelGeometryClass {
        self.geometry_class
    }

    pub fn identity(&self) -> &ItemModelAssetIdentity {
        &self.identity
    }
}

/// Vanilla item feature lane. The same static owner may be submitted through
/// several phases without duplicating its vertex/index payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemModelDrawPhase {
    Solid,
    SolidZOffsetForward,
    CutoutNoCull,
    CutoutZOffset,
    /// Vanilla `RenderPipelines.BANNER_PATTERN`: translucent blending, no
    /// depth writes, submitted after the solid banner/shield base.
    Pattern,
    Translucent,
    TranslucentNoCull,
    Glint,
    GlintTranslucent,
    EndPortal,
    EndGateway,
}

/// Render target/lifetime lane. It is not part of static asset identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemModelDrawTarget {
    World,
    Particle,
    FirstPerson,
    Gui,
    GuiBeforeBlur,
    GuiAfterBlur,
}

impl ItemModelDrawTarget {
    pub(crate) fn is_gui(self) -> bool {
        matches!(self, Self::Gui | Self::GuiBeforeBlur | Self::GuiAfterBlur)
    }
}

/// Physical-pixel GUI clip carried by one resident item submission.
///
/// This is frame-varying placement state, never part of resident asset
/// identity. Construction rejects empty rectangles; the render pass clamps the
/// right/bottom edge to its current attachment before issuing a draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemModelScissorRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl ItemModelScissorRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn x(self) -> u32 {
        self.x
    }

    pub fn y(self) -> u32 {
        self.y
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }

    pub(crate) fn clipped_to(self, width: u32, height: u32) -> Option<Self> {
        let right = self.x.saturating_add(self.width).min(width);
        let bottom = self.y.saturating_add(self.height).min(height);
        (self.x < right && self.y < bottom).then_some(Self {
            x: self.x,
            y: self.y,
            width: right - self.x,
            height: bottom - self.y,
        })
    }
}

/// Narrow per-frame item submission. Transform, tint, light, overlay, foil and
/// order may change every frame without changing the resident asset key.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemModelInstanceRecord {
    pub asset: ItemModelAssetKey,
    pub transform: Mat4,
    pub tint: [f32; 4],
    pub light: [f32; 2],
    pub overlay: [f32; 2],
    pub foil: ItemModelFoil,
    pub target: ItemModelDrawTarget,
    pub phase: ItemModelDrawPhase,
    pub submit_order: u32,
    pub scissor: Option<ItemModelScissorRect>,
}

impl ItemModelInstanceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        asset: ItemModelAssetKey,
        transform: Mat4,
        tint: [f32; 4],
        light: [f32; 2],
        overlay: [f32; 2],
        foil: ItemModelFoil,
        target: ItemModelDrawTarget,
        phase: ItemModelDrawPhase,
        submit_order: u32,
    ) -> Self {
        Self {
            asset,
            transform,
            tint,
            light,
            overlay,
            foil,
            target,
            phase,
            submit_order,
            scissor: None,
        }
    }

    pub fn with_scissor(mut self, scissor: Option<ItemModelScissorRect>) -> Self {
        self.scissor = scissor;
        self
    }
}

/// Small model-space source carried across the native extraction boundary.
/// The renderer consults the exact resident key before evaluating this source.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelAssetSource {
    Quads(Arc<[ItemModelQuad]>),
    Generated {
        mask: SpriteAlphaMask,
        rect: ItemSpriteRect,
        translucent: bool,
    },
}

impl ItemModelAssetSource {
    fn build(self) -> Vec<ItemModelQuad> {
        match self {
            Self::Quads(quads) => quads.as_ref().to_vec(),
            Self::Generated {
                mask,
                rect,
                translucent,
            } => bake_generated_item_quads(&mask, rect, [1.0; 4], translucent),
        }
    }
}

/// One exact static owner requested by a render extraction. This contains no
/// entity/level identity and no frame-varying transform or tint.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemModelAssetRequest {
    pub identity: ItemModelAssetIdentity,
    pub atlas: ItemModelAtlasKind,
    pub geometry_class: ItemModelGeometryClass,
    pub source: ItemModelAssetSource,
}

/// Payload-independent identity plus one narrow frame-varying submission.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemModelInstanceRequest {
    pub identity: ItemModelAssetIdentity,
    pub atlas: ItemModelAtlasKind,
    pub geometry_class: ItemModelGeometryClass,
    pub transform: Mat4,
    pub tint: [f32; 4],
    pub light: [f32; 2],
    pub overlay: [f32; 2],
    pub foil: ItemModelFoil,
    pub target: ItemModelDrawTarget,
    pub phase: ItemModelDrawPhase,
    pub submit_order: u32,
    pub scissor: Option<ItemModelScissorRect>,
}

/// Atomic resident-item portion of a renderer frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelResidencyFrame {
    pub assets: Vec<ItemModelAssetRequest>,
    pub instances: Vec<ItemModelInstanceRequest>,
    pub special_items: Vec<super::SpecialItemInstanceRequest>,
}

impl ItemModelResidencyFrame {
    /// Appends one independently extracted source while preserving the source's
    /// ordinary/special interleave encoded by `submit_order`.
    pub fn append(&mut self, mut source: Self) {
        let submit_offset = u32::try_from(self.instances.len() + self.special_items.len())
            .expect("resident item-model submit order overflow");
        for instance in &mut source.instances {
            instance.submit_order = instance
                .submit_order
                .checked_add(submit_offset)
                .expect("resident item-model submit order overflow");
        }
        for instance in &mut source.special_items {
            instance.submit_order = instance
                .submit_order
                .checked_add(submit_offset)
                .expect("resident special item-model submit order overflow");
        }
        self.assets.extend(source.assets);
        self.instances.extend(source.instances);
        self.special_items.extend(source.special_items);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemModelResidencyOutcome {
    Reused,
    Created,
    Empty,
    Stale,
    CapacityExceeded,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemModelResidentVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) normal_diffuse: [f32; 4],
    pub(crate) decal_uv: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemModelGpuInstance {
    pub(crate) model_0: [f32; 4],
    pub(crate) model_1: [f32; 4],
    pub(crate) model_2: [f32; 4],
    pub(crate) model_3: [f32; 4],
    pub(crate) normal_0: [f32; 4],
    pub(crate) normal_1: [f32; 4],
    pub(crate) normal_2: [f32; 4],
    pub(crate) tint: [f32; 4],
    pub(crate) light_overlay: [f32; 4],
    pub(crate) foil: [f32; 4],
}

impl ItemModelGpuInstance {
    fn from_record(record: &ItemModelInstanceRecord) -> Self {
        let model = record.transform.to_cols_array_2d();
        let model_normal = Mat3::from_mat4(record.transform);
        let determinant = model_normal.determinant();
        let normal = if determinant.is_finite() && determinant.abs() > f32::EPSILON {
            model_normal.inverse().transpose()
        } else {
            Mat3::ZERO
        }
        .to_cols_array_2d();
        let (foil_kind, foil_scale) = match record.foil {
            ItemModelFoil::None => (0.0, 1.0),
            ItemModelFoil::Standard => (1.0, 1.0),
            ItemModelFoil::Special { decal_pose_scale } => (2.0, decal_pose_scale),
        };
        Self {
            model_0: model[0],
            model_1: model[1],
            model_2: model[2],
            model_3: model[3],
            normal_0: [normal[0][0], normal[0][1], normal[0][2], 0.0],
            normal_1: [normal[1][0], normal[1][1], normal[1][2], 0.0],
            normal_2: [normal[2][0], normal[2][1], normal[2][2], 0.0],
            tint: record.tint,
            light_overlay: [
                record.light[0],
                record.light[1],
                record.overlay[0],
                record.overlay[1],
            ],
            foil: [foil_kind, foil_scale, 0.0, 0.0],
        }
    }
}

pub(crate) struct ResidentItemModelGpu {
    pub(crate) vertices: Arc<[ItemModelResidentVertex]>,
    pub(crate) indices: Arc<[u32]>,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) logical_bytes: u64,
    last_used_frame: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResidentItemModelDraw {
    pub(crate) asset: ItemModelAssetKey,
    pub(crate) target: ItemModelDrawTarget,
    pub(crate) phase: ItemModelDrawPhase,
    pub(crate) first_instance: u32,
    pub(crate) instance_count: u32,
    pub(crate) scissor: Option<ItemModelScissorRect>,
}

pub(crate) struct ItemModelResidency {
    resource_generation: ItemModelResourceGeneration,
    block_atlas_generation: ItemModelAtlasGeneration,
    item_atlas_generation: ItemModelAtlasGeneration,
    entity_atlas_generation: ItemModelAtlasGeneration,
    dynamic_player_skin_atlas_generation: ItemModelAtlasGeneration,
    dynamic_entity_atlas_generation: ItemModelAtlasGeneration,
    owners: BTreeMap<ItemModelAssetKey, ResidentItemModelGpu>,
    pinned: BTreeSet<ItemModelAssetKey>,
    instances: Vec<ItemModelInstanceRecord>,
    gpu_instances: Vec<ItemModelGpuInstance>,
    draws: Vec<ResidentItemModelDraw>,
    instance_buffer: FrameDataBuffer,
    frame_index: u64,
    resident_bytes: u64,
    max_owners: usize,
    max_bytes: u64,
    pending_vertex_upload_bytes: u64,
    pending_index_upload_bytes: u64,
    pending_instance_upload_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemModelUploadObservation {
    pub(crate) vertex_bytes: u64,
    pub(crate) index_bytes: u64,
    pub(crate) instance_bytes: u64,
    pub(crate) instances: usize,
}

impl ItemModelResidency {
    pub(crate) fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_RESIDENT_OWNERS, DEFAULT_MAX_RESIDENT_BYTES)
    }

    fn with_limits(max_owners: usize, max_bytes: u64) -> Self {
        Self {
            resource_generation: ItemModelResourceGeneration(1),
            block_atlas_generation: ItemModelAtlasGeneration(1),
            item_atlas_generation: ItemModelAtlasGeneration(1),
            entity_atlas_generation: ItemModelAtlasGeneration(1),
            dynamic_player_skin_atlas_generation: ItemModelAtlasGeneration(1),
            dynamic_entity_atlas_generation: ItemModelAtlasGeneration(1),
            owners: BTreeMap::new(),
            pinned: BTreeSet::new(),
            instances: Vec::new(),
            gpu_instances: Vec::new(),
            draws: Vec::new(),
            instance_buffer: FrameDataBuffer::vertex("bbb-item-model-frame-instances"),
            frame_index: 0,
            resident_bytes: 0,
            max_owners,
            max_bytes,
            pending_vertex_upload_bytes: 0,
            pending_index_upload_bytes: 0,
            pending_instance_upload_bytes: 0,
        }
    }

    fn empty_for_resource_reload(&self) -> Self {
        Self {
            resource_generation: self.resource_generation,
            block_atlas_generation: self.block_atlas_generation,
            item_atlas_generation: self.item_atlas_generation,
            entity_atlas_generation: self.entity_atlas_generation,
            dynamic_player_skin_atlas_generation: self.dynamic_player_skin_atlas_generation,
            dynamic_entity_atlas_generation: self.dynamic_entity_atlas_generation,
            frame_index: self.frame_index,
            ..Self::with_limits(self.max_owners, self.max_bytes)
        }
    }

    fn key(
        &self,
        identity: ItemModelAssetIdentity,
        atlas: ItemModelAtlasKind,
        geometry_class: ItemModelGeometryClass,
    ) -> ItemModelAssetKey {
        ItemModelAssetKey {
            resource_generation: self.resource_generation,
            atlas_generation: self.atlas_generation(atlas),
            atlas,
            geometry_class,
            identity,
        }
    }

    fn atlas_generation(&self, atlas: ItemModelAtlasKind) -> ItemModelAtlasGeneration {
        match atlas {
            ItemModelAtlasKind::Blocks => self.block_atlas_generation,
            ItemModelAtlasKind::Items => self.item_atlas_generation,
            ItemModelAtlasKind::Entity => self.entity_atlas_generation,
            ItemModelAtlasKind::DynamicPlayerSkin => self.dynamic_player_skin_atlas_generation,
            ItemModelAtlasKind::DynamicEntity => self.dynamic_entity_atlas_generation,
        }
    }

    fn key_is_current(&self, key: &ItemModelAssetKey) -> bool {
        key.resource_generation == self.resource_generation
            && key.atlas_generation == self.atlas_generation(key.atlas)
    }

    fn begin_frame(&mut self, counters: &mut RendererCounters) {
        self.frame_index = self
            .frame_index
            .checked_add(1)
            .expect("item frame overflow");
        self.pinned.clear();
        self.instances.clear();
        self.gpu_instances.clear();
        self.draws.clear();
        counters.item_model_lookup_nanos = 0;
        counters.item_gpu_apply_nanos = 0;
        counters.item_model_vertex_upload_bytes = 0;
        counters.item_model_index_upload_bytes = 0;
        counters.item_instance_upload_bytes = 0;
        counters.item_model_instances = 0;
    }

    fn ensure(
        &mut self,
        device: &wgpu::Device,
        counters: &mut RendererCounters,
        key: ItemModelAssetKey,
        build: impl FnOnce() -> Vec<ItemModelQuad>,
    ) -> ItemModelResidencyOutcome {
        if !self.key_is_current(&key) {
            counters.item_model_residency_stale =
                counters.item_model_residency_stale.saturating_add(1);
            return ItemModelResidencyOutcome::Stale;
        }

        let lookup_started = Instant::now();
        if let Some(owner) = self.owners.get_mut(&key) {
            owner.last_used_frame = self.frame_index;
            self.pinned.insert(key);
            counters.item_model_residency_hits =
                counters.item_model_residency_hits.saturating_add(1);
            counters.item_gpu_owners_reused = counters.item_gpu_owners_reused.saturating_add(1);
            record_lookup_time(counters, lookup_started);
            return ItemModelResidencyOutcome::Reused;
        }
        counters.item_model_residency_misses =
            counters.item_model_residency_misses.saturating_add(1);
        record_lookup_time(counters, lookup_started);

        if self.owners.len() >= self.max_owners
            && self.owners.keys().all(|owner| self.pinned.contains(owner))
        {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            return ItemModelResidencyOutcome::CapacityExceeded;
        }

        let apply_started = Instant::now();
        let quads = build();
        let mesh = resident_geometry(&quads, key.geometry_class, key.identity.portal_metadata);
        if mesh.1.is_empty() {
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::Empty;
        }
        let logical_bytes = resident_logical_bytes(&mesh.0, &mesh.1);
        if logical_bytes > self.max_bytes || !self.evict_until_fits(logical_bytes, counters) {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        }

        let (vertices, indices) = mesh;
        let Ok(index_count) = u32::try_from(indices.len()) else {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        };
        let vertex_upload_bytes =
            (vertices.len() * std::mem::size_of::<ItemModelResidentVertex>()) as u64;
        let index_upload_bytes = (indices.len() * std::mem::size_of::<u32>()) as u64;
        let vertices: Arc<[ItemModelResidentVertex]> = vertices.into();
        let indices: Arc<[u32]> = indices.into();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-item-model-resident-vertices"),
            contents: bytemuck::cast_slice(vertices.as_ref()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-item-model-resident-indices"),
            contents: bytemuck::cast_slice(indices.as_ref()),
            usage: wgpu::BufferUsages::INDEX,
        });
        let owner = ResidentItemModelGpu {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            index_count,
            logical_bytes,
            last_used_frame: self.frame_index,
        };
        self.resident_bytes = self.resident_bytes.saturating_add(logical_bytes);
        self.owners.insert(key.clone(), owner);
        self.pinned.insert(key);
        counters.item_model_residency_builds =
            counters.item_model_residency_builds.saturating_add(1);
        counters.item_gpu_owners_created = counters.item_gpu_owners_created.saturating_add(1);
        counters.item_model_vertex_upload_bytes = counters
            .item_model_vertex_upload_bytes
            .saturating_add(vertex_upload_bytes);
        counters.item_model_index_upload_bytes = counters
            .item_model_index_upload_bytes
            .saturating_add(index_upload_bytes);
        self.pending_vertex_upload_bytes = self
            .pending_vertex_upload_bytes
            .saturating_add(vertex_upload_bytes);
        self.pending_index_upload_bytes = self
            .pending_index_upload_bytes
            .saturating_add(index_upload_bytes);
        self.refresh_resident_counters(counters);
        record_apply_time(counters, apply_started);
        ItemModelResidencyOutcome::Created
    }

    fn evict_until_fits(&mut self, incoming_bytes: u64, counters: &mut RendererCounters) -> bool {
        while self.owners.len() >= self.max_owners
            || self.resident_bytes.saturating_add(incoming_bytes) > self.max_bytes
        {
            let candidate = self
                .owners
                .iter()
                .filter(|(key, _)| !self.pinned.contains(*key))
                .min_by(|(left_key, left), (right_key, right)| {
                    (left.last_used_frame, *left_key).cmp(&(right.last_used_frame, *right_key))
                })
                .map(|(key, _)| key.clone());
            let Some(candidate) = candidate else {
                return false;
            };
            if let Some(removed) = self.owners.remove(&candidate) {
                self.resident_bytes = self.resident_bytes.saturating_sub(removed.logical_bytes);
                counters.item_gpu_owners_replaced =
                    counters.item_gpu_owners_replaced.saturating_add(1);
            }
        }
        true
    }

    fn set_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        counters: &mut RendererCounters,
        instances: Vec<ItemModelInstanceRecord>,
    ) -> Result<()> {
        if instances.len() > u32::MAX as usize {
            bail!("item-model frame exceeds the u32 instance address space");
        }
        for instance in &instances {
            if !self.key_is_current(&instance.asset) {
                counters.item_model_residency_stale =
                    counters.item_model_residency_stale.saturating_add(1);
                bail!("item-model instance references a stale resource or atlas generation");
            }
            if !self.owners.contains_key(&instance.asset) {
                bail!("item-model instance references a missing resident owner");
            }
        }

        for instance in &instances {
            if let Some(owner) = self.owners.get_mut(&instance.asset) {
                owner.last_used_frame = self.frame_index;
            }
            self.pinned.insert(instance.asset.clone());
        }

        let apply_started = Instant::now();
        self.gpu_instances.clear();
        self.gpu_instances
            .extend(instances.iter().map(ItemModelGpuInstance::from_record));
        self.draws.clear();
        for (index, instance) in instances.iter().enumerate() {
            let index = u32::try_from(index).expect("instance count checked above");
            if let Some(previous) = self.draws.last_mut() {
                if previous.asset == instance.asset
                    && previous.target == instance.target
                    && previous.phase == instance.phase
                    && previous.scissor == instance.scissor
                    && previous
                        .first_instance
                        .saturating_add(previous.instance_count)
                        == index
                {
                    previous.instance_count = previous.instance_count.saturating_add(1);
                    continue;
                }
            }
            self.draws.push(ResidentItemModelDraw {
                asset: instance.asset.clone(),
                target: instance.target,
                phase: instance.phase,
                first_instance: index,
                instance_count: 1,
                scissor: instance.scissor,
            });
        }
        let previous_capacity = self.instance_buffer.capacity();
        let bytes = bytemuck::cast_slice(&self.gpu_instances);
        let _uploaded = self.instance_buffer.upload(device, queue, bytes);
        if self.instance_buffer.capacity() > previous_capacity {
            counters.item_extraction_scratch_capacity_growths = counters
                .item_extraction_scratch_capacity_growths
                .saturating_add(1);
        }
        counters.item_instance_upload_bytes = bytes.len() as u64;
        self.pending_instance_upload_bytes = self
            .pending_instance_upload_bytes
            .saturating_add(bytes.len() as u64);
        counters.item_model_instances = instances.len();
        self.instances = instances;
        record_apply_time(counters, apply_started);
        Ok(())
    }

    fn invalidate_atlas(&mut self, atlas: ItemModelAtlasKind, counters: &mut RendererCounters) {
        match atlas {
            ItemModelAtlasKind::Blocks => {
                self.block_atlas_generation = ItemModelAtlasGeneration(
                    self.block_atlas_generation
                        .0
                        .checked_add(1)
                        .expect("item block-atlas generation overflow"),
                );
            }
            ItemModelAtlasKind::Items => {
                self.item_atlas_generation = ItemModelAtlasGeneration(
                    self.item_atlas_generation
                        .0
                        .checked_add(1)
                        .expect("item atlas generation overflow"),
                );
            }
            ItemModelAtlasKind::Entity => {
                self.entity_atlas_generation = ItemModelAtlasGeneration(
                    self.entity_atlas_generation
                        .0
                        .checked_add(1)
                        .expect("item entity-atlas generation overflow"),
                );
            }
            ItemModelAtlasKind::DynamicPlayerSkin => {
                self.dynamic_player_skin_atlas_generation = ItemModelAtlasGeneration(
                    self.dynamic_player_skin_atlas_generation
                        .0
                        .checked_add(1)
                        .expect("item dynamic-player-skin atlas generation overflow"),
                );
            }
            ItemModelAtlasKind::DynamicEntity => {
                self.dynamic_entity_atlas_generation = ItemModelAtlasGeneration(
                    self.dynamic_entity_atlas_generation
                        .0
                        .checked_add(1)
                        .expect("item dynamic-entity atlas generation overflow"),
                );
            }
        }
        let removed = self
            .owners
            .keys()
            .filter(|key| key.atlas == atlas)
            .cloned()
            .collect::<Vec<_>>();
        for key in removed {
            if let Some(owner) = self.owners.remove(&key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(owner.logical_bytes);
                counters.item_gpu_owners_replaced =
                    counters.item_gpu_owners_replaced.saturating_add(1);
            }
            self.pinned.remove(&key);
        }
        self.instances
            .retain(|instance| instance.asset.atlas != atlas);
        self.refresh_resident_counters(counters);
    }

    fn invalidate_resources(&mut self, counters: &mut RendererCounters) {
        self.resource_generation = ItemModelResourceGeneration(
            self.resource_generation
                .0
                .checked_add(1)
                .expect("item resource generation overflow"),
        );
        let removed = self.owners.len() as u64;
        if removed != 0 {
            counters.item_full_residency_replacements =
                counters.item_full_residency_replacements.saturating_add(1);
            counters.item_gpu_owners_replaced =
                counters.item_gpu_owners_replaced.saturating_add(removed);
        }
        self.owners.clear();
        self.pinned.clear();
        self.instances.clear();
        self.gpu_instances.clear();
        self.draws.clear();
        self.resident_bytes = 0;
        self.refresh_resident_counters(counters);
    }

    fn refresh_resident_counters(&self, counters: &mut RendererCounters) {
        debug_assert!(self.owners.values().all(|owner| {
            owner.index_count as usize == owner.indices.len()
                && resident_logical_bytes(&owner.vertices, &owner.indices) == owner.logical_bytes
        }));
        counters.item_model_resident_bytes = self.resident_bytes;
        counters.item_model_peak_resident_bytes = counters
            .item_model_peak_resident_bytes
            .max(self.resident_bytes);
    }

    pub(crate) fn owner(&self, key: &ItemModelAssetKey) -> Option<&ResidentItemModelGpu> {
        self.owners.get(key)
    }

    pub(crate) fn draws(&self) -> &[ResidentItemModelDraw] {
        &self.draws
    }

    pub(crate) fn instance_buffer(&self) -> Option<&wgpu::Buffer> {
        self.instance_buffer.buffer()
    }

    pub(crate) fn take_upload_observation(&mut self) -> ItemModelUploadObservation {
        let observation = ItemModelUploadObservation {
            vertex_bytes: self.pending_vertex_upload_bytes,
            index_bytes: self.pending_index_upload_bytes,
            instance_bytes: self.pending_instance_upload_bytes,
            instances: self.instances.len(),
        };
        self.pending_vertex_upload_bytes = 0;
        self.pending_index_upload_bytes = 0;
        self.pending_instance_upload_bytes = 0;
        observation
    }

    #[cfg(test)]
    fn set_limits(&mut self, max_owners: usize, max_bytes: u64) {
        self.max_owners = max_owners;
        self.max_bytes = max_bytes;
    }
}

const ITEM_MODEL_RESIDENT_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x2
];

const ITEM_MODEL_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
    5 => Float32x4,
    6 => Float32x4,
    7 => Float32x4,
    8 => Float32x4,
    9 => Float32x4,
    10 => Float32x4,
    11 => Float32x4,
    12 => Float32x4,
    13 => Float32x4,
    14 => Float32x4
];

fn resident_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelResidentVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_MODEL_RESIDENT_VERTEX_ATTRIBUTES,
    }
}

fn instance_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelGpuInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ITEM_MODEL_INSTANCE_ATTRIBUTES,
    }
}

const ITEM_MODEL_RESIDENT_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var item_atlas: texture_2d<f32>;
@group(0) @binding(2) var item_sampler: sampler;
@group(1) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(1) var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>,
    @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>,
    @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>,
    @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>,
    @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>,
    @location(14) foil: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
};

const ALPHA_CUTOUT: f32 = 0.1;

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) { return 0.0; }
    if (vertex_distance >= fog_end) { return 1.0; }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0), vec2<f32>(0.5 / 16.0), vec2<f32>(15.5 / 16.0));
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}

fn diffuse_light(normal: vec3<f32>) -> f32 {
    let light0 = normalize(camera.minecraft_light0.xyz);
    let light1 = normalize(camera.minecraft_light1.xyz);
    let light_value = max(vec2<f32>(0.0), vec2<f32>(dot(light0, normal), dot(light1, normal)));
    return min(1.0, (light_value.x + light_value.y) * 0.6 + 0.4);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let normal_matrix = mat3x3<f32>(input.normal_0.xyz, input.normal_1.xyz, input.normal_2.xyz);
    let world = model * vec4<f32>(input.position, 1.0);
    var normal = normal_matrix * input.normal_diffuse.xyz;
    if (dot(normal, normal) <= 0.0000001) { normal = vec3<f32>(0.0, 0.0, 1.0); }
    out.position = camera.view_proj * world;
    out.uv = input.uv;
    out.color = input.color * input.tint;
    out.light = input.light_overlay.xy;
    out.normal_diffuse = vec4<f32>(normalize(normal), input.normal_diffuse.w);
    let fog_pos = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(item_atlas, item_sampler, input.uv);
    if (sampled.a < ALPHA_CUTOUT) { discard; }
    let texel = sampled * input.color;
    let light_color = sample_lightmap(input.light);
    let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w);
    return apply_fog(vec4<f32>(texel.rgb * diffuse * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#;

fn resident_z_offset_forward_shader() -> String {
    ITEM_MODEL_RESIDENT_SHADER
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n    view_proj_view_offset_z_forward: mat4x4<f32>,\n};",
        )
        .replace("out.position = camera.view_proj * world;", "out.position = camera.view_proj_view_offset_z_forward * world;")
}

fn resident_per_face_shader() -> String {
    ITEM_MODEL_RESIDENT_SHADER
        .replace(
            "fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {",
            "fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {",
        )
        .replace(
            "    let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w);",
            "    let face_normal = select(-input.normal_diffuse.xyz, input.normal_diffuse.xyz, front_facing);\n    let diffuse = mix(1.0, diffuse_light(face_normal), input.normal_diffuse.w);",
        )
}

fn resident_cutout_z_offset_shader() -> String {
    resident_per_face_shader()
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n};",
        )
        .replace(
            "out.position = camera.view_proj * world;",
            "out.position = camera.view_proj_view_offset_z * world;",
        )
}

const ITEM_MODEL_RESIDENT_GLINT_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
    glint_offsets: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var item_glint_texture: texture_2d<f32>;
@group(0) @binding(2) var item_glint_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>, @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>, @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>, @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>, @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>, @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>, @location(14) foil: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) spherical_distance: f32, @location(2) cylindrical_distance: f32,
};

const GLINT_UV_SCALE: f32 = 8.0;
const GLINT_ALPHA: f32 = 0.75;
const GLINT_ANGLE: f32 = 0.1745329252;

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) { return 0.0; }
    if (vertex_distance >= fog_end) { return 1.0; }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn total_fog_value(spherical_distance: f32, cylindrical_distance: f32) -> f32 {
    return max(linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y), linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w));
}

fn glint_uv(local_uv: vec2<f32>) -> vec2<f32> {
    let scaled = local_uv * GLINT_UV_SCALE;
    let cos_angle = cos(GLINT_ANGLE);
    let sin_angle = sin(GLINT_ANGLE);
    let rotated = vec2<f32>(scaled.x * cos_angle - scaled.y * sin_angle, scaled.x * sin_angle + scaled.y * cos_angle);
    return rotated + camera.glint_offsets.xy;
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    let decal_scale = max(abs(input.foil.y), 0.0001);
    let source_uv = select(input.uv, input.decal_uv / decal_scale, input.foil.x > 1.5);
    out.uv = glint_uv(source_uv);
    let fog_pos = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(item_glint_texture, item_glint_sampler, fract(input.uv));
    if (color.a < 0.1) { discard; }
    let fade = (1.0 - total_fog_value(input.spherical_distance, input.cylindrical_distance)) * GLINT_ALPHA;
    return vec4<f32>(color.rgb * fade, color.a);
}
"#;

pub(crate) fn create_resident_item_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_resident_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        ITEM_MODEL_RESIDENT_SHADER,
        "bbb-item-model-resident-pipeline",
        wgpu::BlendState::REPLACE,
    )
}

pub(crate) fn create_resident_item_model_z_offset_forward_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = resident_z_offset_forward_shader();
    create_resident_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        &shader,
        "bbb-item-model-resident-z-offset-forward-pipeline",
        wgpu::BlendState::REPLACE,
    )
}

pub(crate) fn create_resident_item_model_cutout_no_cull_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = resident_per_face_shader();
    create_resident_item_model_pipeline_with_state(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        &shader,
        "bbb-item-model-resident-cutout-no-cull-pipeline",
        wgpu::BlendState::REPLACE,
        None,
        true,
    )
}

pub(crate) fn create_resident_item_model_cutout_z_offset_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = resident_cutout_z_offset_shader();
    create_resident_item_model_pipeline_with_state(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        &shader,
        "bbb-item-model-resident-cutout-z-offset-pipeline",
        wgpu::BlendState::REPLACE,
        None,
        true,
    )
}

pub(crate) fn create_resident_item_model_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_resident_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        ITEM_MODEL_RESIDENT_SHADER,
        "bbb-item-model-resident-translucent-pipeline",
        wgpu::BlendState::ALPHA_BLENDING,
    )
}

pub(crate) fn create_resident_item_model_translucent_no_cull_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = resident_per_face_shader();
    create_resident_item_model_pipeline_with_state(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        &shader,
        "bbb-item-model-resident-translucent-no-cull-pipeline",
        wgpu::BlendState::ALPHA_BLENDING,
        None,
        true,
    )
}

pub(crate) fn create_resident_item_model_pattern_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader =
        ITEM_MODEL_RESIDENT_SHADER.replace("    if (sampled.a < ALPHA_CUTOUT) { discard; }\n", "");
    RenderPipelineBuilder::new(device, "bbb-item-model-resident-pattern-pipeline")
        .shader("bbb-item-model-resident-pattern-shader", &shader)
        .layout(
            "bbb-item-model-resident-pattern-pipeline-layout",
            &[bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .cull_mode(ITEM_MODEL_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            false,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

pub(crate) fn create_resident_item_model_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-item-model-resident-glint-pipeline")
        .shader(
            "bbb-item-model-resident-glint-shader",
            ITEM_MODEL_RESIDENT_GLINT_SHADER,
        )
        .layout(
            "bbb-item-model-resident-glint-pipeline-layout",
            &[bind_group_layout],
        )
        .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
        .color_target(format, Some(ITEM_MODEL_GLINT_BLEND))
        .cull_mode(ITEM_MODEL_GLINT_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED,
            ITEM_MODEL_GLINT_DEPTH_COMPARE,
        ))
        .build()
}

pub(crate) fn create_resident_item_model_end_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_resident_item_model_end_cube_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-item-model-resident-end-portal",
        15,
    )
}

pub(crate) fn create_resident_item_model_end_gateway_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_resident_item_model_end_cube_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-item-model-resident-end-gateway",
        16,
    )
}

fn create_resident_item_model_end_cube_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label: &str,
    layers: u32,
) -> wgpu::RenderPipeline {
    let shader = resident_end_cube_shader(layers);
    RenderPipelineBuilder::new(device, label)
        .shader(label, &shader)
        .layout(&format!("{label}-layout"), &[bind_group_layout])
        .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
        .color_target(format, None)
        .cull_mode(ITEM_MODEL_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            true,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

fn resident_end_cube_shader(layers: u32) -> String {
    let samples = (0..layers)
        .map(|index| {
            let layer = index + 1;
            format!(
                "    color += atlas_repeat_sample(input.portal_rect_min, input.portal_rect_size, end_portal_layer_uv(input.tex_proj, {layer}.0)) * COLORS[{index}];"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"
struct Camera {{
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
    glint_offsets: vec4<f32>,
    view_proj_view_offset_z: mat4x4<f32>,
    view_proj_view_offset_z_forward: mat4x4<f32>,
    projection: mat4x4<f32>,
    sky_model_view: mat4x4<f32>,
    shader_game_time: vec4<f32>,
}};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var entity_texture_atlas: texture_2d<f32>;
@group(0) @binding(2) var entity_sampler: sampler;

struct VertexIn {{
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>,
    @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>,
    @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>,
    @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>,
    @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>,
    @location(14) foil: vec4<f32>,
}};

struct VertexOut {{
    @builtin(position) position: vec4<f32>,
    @location(0) tex_proj: vec4<f32>,
    @location(1) sky_rect_min: vec2<f32>,
    @location(2) sky_rect_size: vec2<f32>,
    @location(3) portal_rect_min: vec2<f32>,
    @location(4) portal_rect_size: vec2<f32>,
    @location(5) spherical_distance: f32,
    @location(6) cylindrical_distance: f32,
}};

fn linear_fog_value(distance: f32, start: f32, end: f32) -> f32 {{
    if (distance <= start) {{ return 0.0; }}
    if (distance >= end) {{ return 1.0; }}
    return (distance - start) / (end - start);
}}

fn apply_fog(color: vec4<f32>, spherical: f32, cylindrical: f32) -> vec4<f32> {{
    let fog = max(
        linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog * camera.fog_color.a), color.a);
}}

fn projection_from_position(position: vec4<f32>) -> vec4<f32> {{
    var projection = position * 0.5;
    projection.x += projection.w;
    projection.y += projection.w;
    projection.z = position.z;
    projection.w = position.w;
    return projection;
}}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {{
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    out.tex_proj = projection_from_position(out.position);
    out.sky_rect_min = input.color.xy;
    out.sky_rect_size = input.color.zw;
    out.portal_rect_min = input.normal_diffuse.xy;
    out.portal_rect_size = input.normal_diffuse.zw;
    let fog_pos = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}}

fn atlas_repeat_sample(rect_min: vec2<f32>, rect_size: vec2<f32>, uv: vec2<f32>) -> vec3<f32> {{
    return textureSample(entity_texture_atlas, entity_sampler, rect_min + fract(uv) * rect_size).rgb;
}}

fn end_portal_layer_uv(tex_proj: vec4<f32>, layer: f32) -> vec2<f32> {{
    let angle = radians((layer * layer * 4321.0 + layer * 9.0) * 2.0);
    let scale = (4.5 - layer / 4.0) * 2.0;
    let c = cos(angle);
    let s = sin(angle);
    let rotated = vec2<f32>(
        (tex_proj.x * c - tex_proj.y * s) * scale,
        (tex_proj.x * s + tex_proj.y * c) * scale,
    );
    let translated = rotated + vec2<f32>(
        17.0 / layer,
        (2.0 + layer / 1.5) * (camera.shader_game_time.x * 1.5),
    ) * tex_proj.w;
    return (translated * 0.5 + vec2<f32>(0.25 * tex_proj.w)) / tex_proj.w;
}}

const COLORS: array<vec3<f32>, 16> = array<vec3<f32>, 16>(
    vec3<f32>(0.022087, 0.098399, 0.110818),
    vec3<f32>(0.011892, 0.095924, 0.089485),
    vec3<f32>(0.027636, 0.101689, 0.100326),
    vec3<f32>(0.046564, 0.109883, 0.114838),
    vec3<f32>(0.064901, 0.117696, 0.097189),
    vec3<f32>(0.063761, 0.086895, 0.123646),
    vec3<f32>(0.084817, 0.111994, 0.166380),
    vec3<f32>(0.097489, 0.154120, 0.091064),
    vec3<f32>(0.106152, 0.131144, 0.195191),
    vec3<f32>(0.097721, 0.110188, 0.187229),
    vec3<f32>(0.133516, 0.138278, 0.148582),
    vec3<f32>(0.070006, 0.243332, 0.235792),
    vec3<f32>(0.196766, 0.142899, 0.214696),
    vec3<f32>(0.047281, 0.315338, 0.321970),
    vec3<f32>(0.204675, 0.390010, 0.302066),
    vec3<f32>(0.080955, 0.314821, 0.661491),
);

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {{
    var color = atlas_repeat_sample(
        input.sky_rect_min,
        input.sky_rect_size,
        input.tex_proj.xy / input.tex_proj.w,
    ) * COLORS[0];
{samples}
    return apply_fog(vec4<f32>(color, 1.0), input.spherical_distance, input.cylindrical_distance);
}}
"#
    )
}

fn create_resident_item_model_pipeline_with_blend(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shader: &str,
    label: &str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    create_resident_item_model_pipeline_with_state(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        shader,
        label,
        blend,
        ITEM_MODEL_PIPELINE_CULL_MODE,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_resident_item_model_pipeline_with_state(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shader: &str,
    label: &str,
    blend: wgpu::BlendState,
    cull_mode: Option<wgpu::Face>,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, label)
        .shader(label, shader)
        .layout(
            &format!("{label}-layout"),
            &[bind_group_layout, lightmap_bind_group_layout],
        )
        .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
        .color_target(format, Some(blend))
        .cull_mode(cull_mode)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            depth_write_enabled,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

fn resident_geometry(
    quads: &[ItemModelQuad],
    geometry_class: ItemModelGeometryClass,
    portal_metadata: bool,
) -> (Vec<ItemModelResidentVertex>, Vec<u32>) {
    let selected = quads
        .iter()
        .copied()
        .filter(|quad| {
            quad.translucent == matches!(geometry_class, ItemModelGeometryClass::Translucent)
        })
        .collect::<Vec<_>>();
    let mesh = bake_item_model_mesh(&selected, Mat4::IDENTITY);
    let mut vertices = mesh
        .vertices
        .into_iter()
        .map(|vertex: ItemModelVertex| {
            let decal_uv = sheeted_decal_uv(
                vertex.position,
                [
                    vertex.normal_diffuse[0],
                    vertex.normal_diffuse[1],
                    vertex.normal_diffuse[2],
                ],
                SPECIAL_FOIL_TEXTURE_SCALE,
            );
            ItemModelResidentVertex {
                position: vertex.position,
                uv: vertex.uv,
                color: vertex.color,
                normal_diffuse: vertex.normal_diffuse,
                decal_uv,
            }
        })
        .collect::<Vec<_>>();
    if portal_metadata {
        for (quad, face_vertices) in selected.iter().zip(vertices.chunks_exact_mut(4)) {
            for vertex in face_vertices {
                vertex.normal_diffuse =
                    [quad.normal[0], quad.normal[1], quad.normal[2], quad.shade];
            }
        }
    }
    (vertices, mesh.indices)
}

fn resident_logical_bytes(vertices: &[ItemModelResidentVertex], indices: &[u32]) -> u64 {
    (vertices.len() * std::mem::size_of::<ItemModelResidentVertex>()
        + indices.len() * std::mem::size_of::<u32>()) as u64
}

fn record_lookup_time(counters: &mut RendererCounters, started: Instant) {
    let nanos = duration_nanos(started.elapsed());
    counters.item_model_lookup_nanos = counters.item_model_lookup_nanos.saturating_add(nanos);
    counters.item_model_lookup_peak_nanos = counters
        .item_model_lookup_peak_nanos
        .max(counters.item_model_lookup_nanos);
}

fn record_apply_time(counters: &mut RendererCounters, started: Instant) {
    let nanos = duration_nanos(started.elapsed());
    counters.item_gpu_apply_nanos = counters.item_gpu_apply_nanos.saturating_add(nanos);
    counters.item_gpu_apply_peak_nanos = counters
        .item_gpu_apply_peak_nanos
        .max(counters.item_gpu_apply_nanos);
}

fn duration_nanos(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

impl Renderer {
    pub fn item_model_resource_generation(&self) -> ItemModelResourceGeneration {
        self.item_model_residency.resource_generation
    }

    pub fn item_model_asset_key(
        &self,
        identity: ItemModelAssetIdentity,
        atlas: ItemModelAtlasKind,
        geometry_class: ItemModelGeometryClass,
    ) -> ItemModelAssetKey {
        self.item_model_residency
            .key(identity, atlas, geometry_class)
    }

    pub fn begin_item_model_frame(&mut self) {
        self.item_model_residency.begin_frame(&mut self.counters);
    }

    /// Ensures one exact model-space owner. `build` is not evaluated on a hit
    /// or stale key, so the cache decision always precedes the large payload.
    pub fn ensure_item_model_resident(
        &mut self,
        key: ItemModelAssetKey,
        build: impl FnOnce() -> Vec<ItemModelQuad>,
    ) -> ItemModelResidencyOutcome {
        self.item_model_residency
            .ensure(&self.device, &mut self.counters, key, build)
    }

    pub fn set_resident_item_model_instances(
        &mut self,
        instances: Vec<ItemModelInstanceRecord>,
    ) -> Result<()> {
        self.item_model_residency.set_instances(
            &self.device,
            &self.queue,
            &mut self.counters,
            instances,
        )
    }

    /// Applies one atomic model-space asset/instance extraction. Exact keys
    /// are minted from the renderer's current resource generations, and each
    /// ordinary asset source is evaluated only after a miss. Special
    /// submissions first expand into their exact static parts and keys.
    pub fn set_item_model_residency_frame(
        &mut self,
        mut frame: ItemModelResidencyFrame,
    ) -> Result<()> {
        self.append_hud_block_item_residency(&mut frame);
        self.append_special_item_residency(&mut frame)?;
        if frame
            .instances
            .iter()
            .any(|request| request.scissor.is_some() && !request.target.is_gui())
        {
            bail!("resident item scissor is only valid for GUI draw targets");
        }
        self.begin_item_model_frame();
        let mut requested_keys = BTreeSet::new();
        for request in frame.assets {
            let key =
                self.item_model_asset_key(request.identity, request.atlas, request.geometry_class);
            if !requested_keys.insert(key.clone()) {
                continue;
            }
            match self.ensure_item_model_resident(key, || request.source.build()) {
                ItemModelResidencyOutcome::Reused | ItemModelResidencyOutcome::Created => {}
                ItemModelResidencyOutcome::Empty => {
                    bail!("item-model frame requested an empty resident geometry owner")
                }
                ItemModelResidencyOutcome::Stale => {
                    bail!("renderer minted a stale item-model asset key")
                }
                ItemModelResidencyOutcome::CapacityExceeded => {
                    bail!("item-model resident owner capacity exceeded")
                }
            }
        }
        let mut instances = frame
            .instances
            .into_iter()
            .map(|request| {
                ItemModelInstanceRecord::new(
                    self.item_model_asset_key(
                        request.identity,
                        request.atlas,
                        request.geometry_class,
                    ),
                    request.transform,
                    request.tint,
                    request.light,
                    request.overlay,
                    request.foil,
                    request.target,
                    request.phase,
                    request.submit_order,
                )
                .with_scissor(request.scissor)
            })
            .collect::<Vec<_>>();
        // Composite leaves publish a single monotonically increasing order.
        // Stable sorting keeps the renderer's expansion order for the several
        // geometry parts belonging to one special leaf.
        instances.sort_by_key(|instance| instance.submit_order);
        self.set_resident_item_model_instances(instances)
    }

    fn append_special_item_residency(&self, frame: &mut ItemModelResidencyFrame) -> Result<()> {
        for request in std::mem::take(&mut frame.special_items) {
            let model_variant = Arc::<str>::from(format!("{:?}", request.model));
            let model_id = Arc::<str>::from(format!(
                "minecraft:special/{}",
                request.model.registered_type_name()
            ));
            let parts = bake_special_item_geometry(&request.model, |texture| {
                self.resolve_special_item_texture(texture)
            })?;
            for (part_index, part) in parts.into_iter().enumerate() {
                let part_index = u32::try_from(part_index)
                    .map_err(|_| anyhow::anyhow!("special item has too many geometry parts"))?;
                let portal_metadata = matches!(
                    part.phase,
                    ItemModelDrawPhase::EndPortal | ItemModelDrawPhase::EndGateway
                );
                let mut identity = ItemModelAssetIdentity::new(
                    model_id.clone(),
                    Arc::<str>::from(format!(
                        "{}|{}|{}",
                        model_variant, part.semantic_part, part.texture_identity
                    )),
                    part_index,
                );
                if portal_metadata {
                    identity = identity.with_portal_metadata();
                }
                let geometry_class = match part.phase {
                    ItemModelDrawPhase::Pattern
                    | ItemModelDrawPhase::Translucent
                    | ItemModelDrawPhase::TranslucentNoCull => ItemModelGeometryClass::Translucent,
                    ItemModelDrawPhase::Solid
                    | ItemModelDrawPhase::SolidZOffsetForward
                    | ItemModelDrawPhase::CutoutNoCull
                    | ItemModelDrawPhase::CutoutZOffset
                    | ItemModelDrawPhase::Glint
                    | ItemModelDrawPhase::GlintTranslucent
                    | ItemModelDrawPhase::EndPortal
                    | ItemModelDrawPhase::EndGateway => ItemModelGeometryClass::Solid,
                };
                frame.assets.push(ItemModelAssetRequest {
                    identity: identity.clone(),
                    atlas: part.atlas,
                    geometry_class,
                    source: ItemModelAssetSource::Quads(part.quads.into()),
                });
                let foil = if part.accepts_foil {
                    request.foil
                } else {
                    ItemModelFoil::None
                };
                frame.instances.push(ItemModelInstanceRequest {
                    identity: identity.clone(),
                    atlas: part.atlas,
                    geometry_class,
                    transform: request.transform,
                    tint: part.tint,
                    light: request.light,
                    overlay: request.overlay,
                    foil,
                    target: request.target,
                    phase: part.phase,
                    submit_order: request.submit_order,
                    scissor: request.scissor,
                });
                if part.accepts_foil && request.foil.has_foil() {
                    frame.instances.push(ItemModelInstanceRequest {
                        identity,
                        atlas: part.atlas,
                        geometry_class,
                        transform: request.transform,
                        tint: part.tint,
                        light: request.light,
                        overlay: request.overlay,
                        foil: request.foil,
                        target: request.target,
                        phase: ItemModelDrawPhase::Glint,
                        submit_order: request.submit_order,
                        scissor: request.scissor,
                    });
                }
            }
        }
        Ok(())
    }

    fn resolve_special_item_texture(
        &self,
        texture: &SpecialItemTexture,
    ) -> Result<ResolvedSpecialItemTexture> {
        match texture {
            SpecialItemTexture::Static(texture) => {
                if texture.size.contains(&0) {
                    bail!(
                        "entity model texture `{}` has zero-sized logical dimensions",
                        texture.path
                    );
                }
                let atlas = self
                    .entity_model_texture_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entity model texture atlas is not uploaded"))?;
                let entry = atlas
                    .layout
                    .entries
                    .iter()
                    .find(|entry| entry.texture == *texture)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "entity model texture `{}` is absent from the static atlas",
                            texture.path
                        )
                    })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::Entity,
                    logical_size: texture.size,
                    uv: entry.uv,
                    identity: Arc::from(format!("static:{}", texture.path)),
                })
            }
            SpecialItemTexture::Pack(texture) => {
                let atlas = self
                    .entity_model_texture_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entity model texture atlas is not uploaded"))?;
                let uv = atlas
                    .owned_entries
                    .iter()
                    .find(|entry| entry.texture == *texture)
                    .map(|entry| entry.uv)
                    // The same physical pack image may be referenced by
                    // different special model kinds with different logical
                    // UV dimensions. Reuse the uploaded image by path while
                    // retaining this request's logical size.
                    .or_else(|| {
                        atlas
                            .owned_entries
                            .iter()
                            .find(|entry| entry.texture.path == texture.path)
                            .map(|entry| entry.uv)
                    })
                    // A direct pack texture may already be present through a
                    // static entity layer. Reuse that physical atlas entry
                    // while retaining the special layer's logical UV size.
                    .or_else(|| {
                        atlas
                            .layout
                            .entries
                            .iter()
                            .find(|entry| entry.texture.path == texture.path.as_ref())
                            .map(|entry| entry.uv)
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "entity model texture `{}` is absent from the pack-owned atlas",
                            texture.path
                        )
                    })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::Entity,
                    logical_size: texture.size,
                    uv,
                    identity: Arc::from(format!("pack:{}:{:?}", texture.path, texture.size)),
                })
            }
            SpecialItemTexture::Dynamic {
                texture,
                logical_size,
            } => {
                if logical_size.contains(&0) {
                    bail!("dynamic special item texture has zero-sized logical dimensions");
                }
                let atlas = self
                    .entity_dynamic_player_texture_atlas
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("dynamic entity texture atlas is not uploaded")
                    })?;
                let entry = atlas
                    .layout
                    .entries
                    .iter()
                    .find(|entry| entry.handle == texture.handle)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "dynamic entity texture handle {} is absent from the atlas",
                            texture.handle
                        )
                    })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::DynamicEntity,
                    logical_size: *logical_size,
                    uv: entry.uv,
                    identity: Arc::from(format!("dynamic:{}:{logical_size:?}", texture.handle)),
                })
            }
            SpecialItemTexture::DynamicPlayerSkin(skin) => {
                if skin.status != EntityDynamicPlayerSkinStatus::Ready {
                    bail!(
                        "dynamic player skin {} is not ready for special item rendering",
                        skin.handle
                    );
                }
                let atlas = self
                    .entity_dynamic_player_skin_atlas
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("dynamic player skin atlas is not uploaded"))?;
                let entry = atlas
                    .layout
                    .entries
                    .iter()
                    .find(|entry| entry.handle == skin.handle)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "dynamic player skin handle {} is absent from the atlas",
                            skin.handle
                        )
                    })?;
                Ok(ResolvedSpecialItemTexture {
                    atlas: ItemModelAtlasKind::DynamicPlayerSkin,
                    logical_size: [64, 64],
                    uv: entry.uv,
                    identity: Arc::from(format!(
                        "dynamic_player_skin:{}:{:?}",
                        skin.handle, skin.model
                    )),
                })
            }
        }
    }

    /// Computes the exact model-space bounds used by Vanilla's item extent
    /// visits and oversized-GUI bounds. The same resolved atlas rectangles and
    /// posed builders feed both this query and resident geometry.
    pub fn special_item_model_bounds(
        &self,
        model: &SpecialItemModel,
    ) -> Result<SpecialItemModelBounds> {
        let mut bounds_model = model.clone();
        if let SpecialItemModel::Banner { attachment, .. } = &mut bounds_model {
            // Vanilla `BannerRenderer.getExtents` always visits the standing
            // frame and flag, independently of the configured attachment.
            *attachment = super::SpecialItemBannerAttachment::Ground;
        }
        let parts = bake_special_item_geometry(&bounds_model, |texture| {
            self.resolve_special_item_texture(texture)
        })?;
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for corner in parts
            .iter()
            // `DecoratedPotRenderer.getExtents` visits neck/top/bottom only;
            // its material side quads are submit geometry, not extents.
            .filter(|part| {
                !matches!(model, SpecialItemModel::DecoratedPot { .. })
                    || part.semantic_part.as_ref() == "decorated_pot_base"
            })
            .flat_map(|part| &part.quads)
            .flat_map(|quad| quad.corners)
        {
            for axis in 0..3 {
                let coordinate = corner[axis] / 16.0;
                min[axis] = min[axis].min(coordinate);
                max[axis] = max[axis].max(coordinate);
            }
        }
        if min.iter().any(|value| !value.is_finite()) || max.iter().any(|value| !value.is_finite())
        {
            bail!("special item model produced no finite extents");
        }
        Ok(SpecialItemModelBounds { min, max })
    }

    /// Advances the resource generation and drops all derived model owners.
    pub fn invalidate_item_model_resources(&mut self) {
        if self.resource_reload_in_progress() {
            let next = self.item_model_residency.empty_for_resource_reload();
            let previous = std::mem::replace(&mut self.item_model_residency, next);
            self.register_resource_owner_restore(Box::new(move |renderer| {
                renderer.item_model_residency = previous;
            }));
        }
        self.item_model_residency
            .invalidate_resources(&mut self.counters);
    }

    pub(crate) fn invalidate_item_model_atlas(&mut self, atlas: ItemModelAtlasKind) {
        if self.resource_reload_in_progress() {
            return;
        }
        self.item_model_residency
            .invalidate_atlas(atlas, &mut self.counters);
    }

    #[cfg(test)]
    pub(crate) fn set_item_model_residency_limits_for_test(
        &mut self,
        max_owners: usize,
        max_bytes: u64,
    ) {
        self.item_model_residency.set_limits(max_owners, max_bytes);
    }

    #[cfg(test)]
    pub(crate) fn resident_item_model_instance_orders_for_test(&self) -> Vec<u32> {
        self.item_model_residency
            .instances
            .iter()
            .map(|instance| instance.submit_order)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECIAL_TRIDENT_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_trident.png",
        size: [64, 64],
    };
    const SPECIAL_SKY_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_sky.png",
        size: [16, 16],
    };
    const SPECIAL_PORTAL_TEXTURE: crate::EntityModelTextureRef = crate::EntityModelTextureRef {
        path: "textures/entity/test_special_portal.png",
        size: [16, 16],
    };

    fn quad(translucent: bool) -> ItemModelQuad {
        ItemModelQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            tint: [1.0; 4],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent,
        }
    }

    fn identity(name: &str) -> ItemModelAssetIdentity {
        ItemModelAssetIdentity::new(Arc::<str>::from(name), Arc::<str>::from("plain"), 0)
    }

    fn block_frame(transform: Mat4) -> ItemModelResidencyFrame {
        let identity = identity("minecraft:stone");
        ItemModelResidencyFrame {
            assets: vec![ItemModelAssetRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                source: ItemModelAssetSource::Quads(Arc::from([quad(false)])),
            }],
            instances: vec![ItemModelInstanceRequest {
                identity,
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                transform,
                tint: [1.0; 4],
                light: [1.0; 2],
                overlay: [0.0, 10.0],
                foil: ItemModelFoil::None,
                target: ItemModelDrawTarget::FirstPerson,
                phase: ItemModelDrawPhase::Solid,
                submit_order: 0,
                scissor: None,
            }],
            special_items: Vec::new(),
        }
    }

    #[test]
    fn residency_frame_append_preserves_ordinary_special_submit_order() {
        let mut target = block_frame(Mat4::IDENTITY);
        let mut source = block_frame(Mat4::from_translation(glam::Vec3::X));
        source
            .special_items
            .push(super::super::SpecialItemInstanceRequest {
                model: SpecialItemModel::Trident {
                    texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                },
                transform: Mat4::IDENTITY,
                light: [1.0; 2],
                overlay: [0.0, 10.0],
                foil: ItemModelFoil::None,
                target: ItemModelDrawTarget::World,
                submit_order: 1,
                scissor: None,
            });

        target.append(source);

        assert_eq!(
            target
                .instances
                .iter()
                .map(|instance| instance.submit_order)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(target.special_items[0].submit_order, 2);
    }

    #[test]
    fn special_item_texture_sources_resolve_all_entity_atlas_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let owned_texture = crate::EntityModelOwnedTextureRef::new(
            "textures/entity/test_special_owned.png",
            [64, 32],
        );
        renderer
            .upload_entity_model_textures_with_owned(
                &[crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_TRIDENT_TEXTURE,
                    [1, 1],
                    vec![255; 4],
                )],
                &[crate::EntityModelOwnedTextureImage::with_physical_size(
                    owned_texture.clone(),
                    [2, 1],
                    vec![127; 8],
                )],
            )
            .unwrap();
        renderer
            .upload_dynamic_entity_texture(crate::DynamicEntityTextureImage {
                handle: 42,
                size: [2, 3],
                rgba: vec![63; 2 * 3 * 4],
            })
            .unwrap();
        renderer
            .upload_dynamic_player_skin(crate::DynamicPlayerSkinImage {
                handle: 43,
                rgba: vec![31; 64 * 64 * 4],
            })
            .unwrap();

        let static_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE))
            .unwrap();
        assert_eq!(static_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(static_texture.logical_size, [64, 64]);

        let pack_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Pack(owned_texture.clone()))
            .unwrap();
        assert_eq!(pack_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(pack_texture.logical_size, [64, 32]);
        let shared_pack_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Pack(
                crate::EntityModelOwnedTextureRef::new(owned_texture.path, [256, 256]),
            ))
            .unwrap();
        assert_eq!(shared_pack_texture.atlas, ItemModelAtlasKind::Entity);
        assert_eq!(shared_pack_texture.logical_size, [256, 256]);

        let dynamic_texture = renderer
            .resolve_special_item_texture(&SpecialItemTexture::Dynamic {
                texture: crate::EntityDynamicTexture { handle: 42 },
                logical_size: [128, 64],
            })
            .unwrap();
        assert_eq!(dynamic_texture.atlas, ItemModelAtlasKind::DynamicEntity);
        assert_eq!(dynamic_texture.logical_size, [128, 64]);

        let dynamic_skin = renderer
            .resolve_special_item_texture(&SpecialItemTexture::DynamicPlayerSkin(
                crate::EntityDynamicPlayerSkin {
                    handle: 43,
                    fallback: crate::EntityDefaultPlayerSkin::WideSteve,
                    model: crate::EntityPlayerSkinModel::Wide,
                    status: crate::EntityDynamicPlayerSkinStatus::Ready,
                },
            ))
            .unwrap();
        assert_eq!(dynamic_skin.atlas, ItemModelAtlasKind::DynamicPlayerSkin);
        assert_eq!(dynamic_skin.logical_size, [64, 64]);
    }

    #[test]
    fn exact_hit_does_not_evaluate_geometry_builder_or_upload_static_payload() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let mut builds = 0;
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || {
                builds += 1;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Created
        );
        let uploaded = renderer.counters().item_model_vertex_upload_bytes
            + renderer.counters().item_model_index_upload_bytes;
        assert!(uploaded > 0);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key, || {
                builds += 1;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Reused
        );
        assert_eq!(builds, 1);
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
    }

    #[test]
    fn dynamic_instance_changes_upload_only_narrow_instance_payload() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:diamond"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key.clone(),
                Mat4::IDENTITY,
                [1.0; 4],
                [1.0; 2],
                [0.0, 10.0],
                ItemModelFoil::None,
                ItemModelDrawTarget::World,
                ItemModelDrawPhase::Solid,
                9,
            )])
            .unwrap();

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        renderer
            .set_resident_item_model_instances(vec![ItemModelInstanceRecord::new(
                key,
                Mat4::from_translation(glam::Vec3::X),
                [0.5, 1.0, 1.0, 1.0],
                [0.5, 1.0],
                [0.0, 10.0],
                ItemModelFoil::Standard,
                ItemModelDrawTarget::World,
                ItemModelDrawPhase::Glint,
                9,
            )])
            .unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert_eq!(
            renderer.counters().item_instance_upload_bytes,
            std::mem::size_of::<ItemModelGpuInstance>() as u64
        );
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![9]
        );
    }

    #[test]
    fn special_foil_resident_decal_uv_matches_legacy_world_bake() {
        let source = [quad(false)];
        let transform = Mat4::from_translation(glam::Vec3::new(2.0, -1.0, 4.0))
            * Mat4::from_rotation_y(0.73)
            * Mat4::from_scale(glam::Vec3::new(0.5, 0.75, 1.25));
        let (vertices, _) = resident_geometry(&source, ItemModelGeometryClass::Solid, false);

        for decal_pose_scale in [1.0, 0.75, 0.5] {
            let legacy = super::super::bake_item_model_meshes_with_light_and_overlay_and_foil_mode(
                &source,
                transform,
                [1.0; 2],
                [0.0, 10.0],
                ItemModelFoil::Special { decal_pose_scale },
            );
            assert_eq!(legacy.glint.vertices.len(), vertices.len());
            for (legacy, resident) in legacy.glint.vertices.iter().zip(&vertices) {
                let expected = [
                    resident.decal_uv[0] / decal_pose_scale,
                    resident.decal_uv[1] / decal_pose_scale,
                ];
                assert!((legacy.uv[0] - expected[0]).abs() < 1.0e-5);
                assert!((legacy.uv[1] - expected[1]).abs() < 1.0e-5);
            }
        }
    }

    #[test]
    fn high_level_frame_reuses_static_owner_and_full_render_consumes_upload_once() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .update_terrain_texture_atlas(&[255, 255, 255, 255])
            .unwrap();
        renderer.update_camera();
        renderer
            .set_item_model_residency_frame(block_frame(Mat4::IDENTITY))
            .unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert!(renderer.counters().item_model_vertex_upload_bytes > 0);
        assert!(renderer.counters().item_model_index_upload_bytes > 0);
        assert!(renderer.counters().item_instance_upload_bytes > 0);

        renderer
            .set_item_model_residency_frame(block_frame(Mat4::from_translation(glam::Vec3::new(
                0.25, 0.0, 0.0,
            ))))
            .unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert!(renderer.counters().item_instance_upload_bytes > 0);

        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
        assert_eq!(renderer.counters().item_instance_upload_bytes, 0);
    }

    #[test]
    fn consecutive_instances_share_one_draw_without_reordering() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        let mut second = frame.instances[0].clone();
        second.transform = Mat4::from_translation(glam::Vec3::X);
        second.submit_order = 1;
        frame.instances.push(second);
        renderer.set_item_model_residency_frame(frame).unwrap();

        assert_eq!(renderer.item_model_residency.draws().len(), 1);
        assert_eq!(renderer.item_model_residency.draws()[0].instance_count, 2);
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![0, 1]
        );
    }

    #[test]
    fn resident_draw_batching_splits_equal_assets_at_scissor_boundaries() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        frame.instances[0].target = ItemModelDrawTarget::Gui;
        frame.instances[0].scissor = ItemModelScissorRect::new(0, 0, 8, 16);
        let mut second = frame.instances[0].clone();
        second.transform = Mat4::from_translation(glam::Vec3::X);
        second.submit_order = 1;
        second.scissor = None;
        frame.instances.push(second);
        renderer.set_item_model_residency_frame(frame).unwrap();

        assert_eq!(renderer.item_model_residency.draws().len(), 2);
        assert_eq!(
            renderer.item_model_residency.draws()[0].scissor,
            ItemModelScissorRect::new(0, 0, 8, 16)
        );
        assert_eq!(renderer.item_model_residency.draws()[1].scissor, None);
    }

    #[test]
    fn resident_scissor_rejects_empty_and_clips_to_attachment_bounds() {
        assert_eq!(ItemModelScissorRect::new(0, 0, 0, 1), None);
        assert_eq!(ItemModelScissorRect::new(0, 0, 1, 0), None);
        assert_eq!(
            ItemModelScissorRect::new(6, 7, 8, 9).and_then(|rect| rect.clipped_to(10, 12)),
            ItemModelScissorRect::new(6, 7, 4, 5)
        );
        assert_eq!(
            ItemModelScissorRect::new(10, 0, 1, 1).and_then(|rect| rect.clipped_to(10, 12)),
            None
        );
    }

    #[test]
    fn resident_scissor_rejects_non_gui_targets() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut frame = block_frame(Mat4::IDENTITY);
        frame.instances[0].scissor = ItemModelScissorRect::new(0, 0, 8, 8);
        let error = renderer
            .set_item_model_residency_frame(frame)
            .expect_err("world/first-person scissor must fail closed");
        assert!(error
            .to_string()
            .contains("scissor is only valid for GUI draw targets"));
    }

    #[test]
    fn reload_rejects_old_key_without_evaluating_builder() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let key = renderer.item_model_asset_key(
            identity("minecraft:clock"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        let previous_generation = key.resource_generation();
        renderer.invalidate_item_model_resources();
        let mut rebuilt = false;
        assert_eq!(
            renderer.ensure_item_model_resident(key, || {
                rebuilt = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Stale
        );
        assert!(!rebuilt);
        assert!(renderer.item_model_resource_generation() > previous_generation);
        assert_eq!(renderer.counters().item_full_residency_replacements, 1);
    }

    #[test]
    fn atlas_replacement_invalidates_only_matching_material_owners() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.begin_item_model_frame();
        let block_key = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let item_key = renderer.item_model_asset_key(
            identity("minecraft:apple"),
            ItemModelAtlasKind::Items,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(block_key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        assert_eq!(
            renderer.ensure_item_model_resident(item_key.clone(), || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );

        renderer.invalidate_item_model_atlas(ItemModelAtlasKind::Items);
        assert_eq!(
            renderer.ensure_item_model_resident(block_key, Vec::new),
            ItemModelResidencyOutcome::Reused
        );
        let mut stale_item_built = false;
        assert_eq!(
            renderer.ensure_item_model_resident(item_key, || {
                stale_item_built = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::Stale
        );
        assert!(!stale_item_built);
        assert_eq!(renderer.counters().item_full_residency_replacements, 0);
    }

    #[test]
    fn current_frame_pins_make_capacity_fallback_explicit() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_item_model_residency_limits_for_test(1, u64::MAX);
        renderer.begin_item_model_frame();
        let first = renderer.item_model_asset_key(
            identity("minecraft:stone"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        let second = renderer.item_model_asset_key(
            identity("minecraft:dirt"),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
        );
        assert_eq!(
            renderer.ensure_item_model_resident(first, || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        let mut second_built_while_pinned = false;
        assert_eq!(
            renderer.ensure_item_model_resident(second.clone(), || {
                second_built_while_pinned = true;
                vec![quad(false)]
            }),
            ItemModelResidencyOutcome::CapacityExceeded
        );
        assert!(!second_built_while_pinned);

        renderer.begin_item_model_frame();
        assert_eq!(
            renderer.ensure_item_model_resident(second, || vec![quad(false)]),
            ItemModelResidencyOutcome::Created
        );
        assert_eq!(renderer.counters().item_model_resident_bytes > 0, true);
        assert_eq!(renderer.counters().item_model_fallbacks, 1);
    }

    #[test]
    fn special_item_model_residency_reuses_entity_geometry_and_draws_end_cube_pipeline() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .upload_entity_model_textures(&[
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_TRIDENT_TEXTURE,
                    [1, 1],
                    vec![255; 4],
                ),
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_SKY_TEXTURE,
                    [1, 1],
                    vec![31, 63, 95, 255],
                ),
                crate::EntityModelTextureImage::with_physical_size(
                    SPECIAL_PORTAL_TEXTURE,
                    [1, 1],
                    vec![127, 63, 191, 255],
                ),
            ])
            .unwrap();
        renderer
            .upload_dynamic_player_skin(crate::DynamicPlayerSkinImage {
                handle: 41,
                rgba: vec![0xff; 64 * 64 * 4],
            })
            .unwrap();
        renderer.update_camera();

        let frame = || ItemModelResidencyFrame {
            special_items: vec![
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Trident {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::Standard,
                    target: ItemModelDrawTarget::World,
                    submit_order: 7,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::EndCube {
                        sky_texture: SpecialItemTexture::Static(SPECIAL_SKY_TEXTURE),
                        portal_texture: SpecialItemTexture::Static(SPECIAL_PORTAL_TEXTURE),
                        effect: super::super::SpecialItemEndCubeEffect::Gateway,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::Standard,
                    target: ItemModelDrawTarget::World,
                    submit_order: 8,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Trident {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    submit_order: 9,
                    scissor: ItemModelScissorRect::new(1, 2, 3, 4),
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Banner {
                        attachment: super::super::SpecialItemBannerAttachment::Ground,
                        base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        pattern_base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        base_color: crate::EntityDyeColor::Blue,
                        patterns: Arc::from([super::super::SpecialItemPatternLayer::new(
                            SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                            crate::EntityDyeColor::Red,
                        )]),
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    submit_order: 10,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::CopperGolemStatue {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        pose: super::super::SpecialItemCopperGolemPose::Standing,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    submit_order: 11,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::Head {
                        texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
                        kind: super::super::SpecialItemHeadKind::Skeleton,
                        animation: 0.0,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    submit_order: 12,
                    scissor: None,
                },
                super::super::SpecialItemInstanceRequest {
                    model: SpecialItemModel::PlayerHead {
                        skin: Some(crate::EntityDynamicPlayerSkin {
                            handle: 41,
                            fallback: crate::EntityDefaultPlayerSkin::WideSteve,
                            model: crate::EntityPlayerSkinModel::Wide,
                            status: crate::EntityDynamicPlayerSkinStatus::Ready,
                        }),
                        fallback_texture: SPECIAL_TRIDENT_TEXTURE,
                    },
                    transform: Mat4::IDENTITY,
                    light: [1.0; 2],
                    overlay: [0.0, 10.0],
                    foil: ItemModelFoil::None,
                    target: ItemModelDrawTarget::Gui,
                    submit_order: 13,
                    scissor: None,
                },
            ],
            ..ItemModelResidencyFrame::default()
        };

        renderer.set_item_model_residency_frame(frame()).unwrap();
        assert_eq!(
            renderer.resident_item_model_instance_orders_for_test(),
            vec![7, 7, 8, 9, 10, 10, 10, 10, 11, 12, 13]
        );
        assert!(renderer
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.phase == ItemModelDrawPhase::EndGateway));
        assert!(renderer
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.phase == ItemModelDrawPhase::Pattern));
        assert!(renderer
            .item_model_residency
            .draws()
            .iter()
            .any(|draw| draw.scissor == ItemModelScissorRect::new(1, 2, 3, 4)));
        for phase in [
            ItemModelDrawPhase::CutoutNoCull,
            ItemModelDrawPhase::CutoutZOffset,
            ItemModelDrawPhase::TranslucentNoCull,
        ] {
            assert!(renderer
                .item_model_residency
                .draws()
                .iter()
                .any(|draw| draw.phase == phase));
        }
        assert!(renderer.item_model_residency.draws().iter().any(|draw| {
            draw.phase == ItemModelDrawPhase::TranslucentNoCull
                && draw.asset.atlas() == ItemModelAtlasKind::DynamicPlayerSkin
        }));
        let bounds = renderer
            .special_item_model_bounds(&SpecialItemModel::Trident {
                texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            })
            .unwrap();
        assert!(bounds.size().iter().all(|size| *size > 0.0));
        let banner = |attachment| SpecialItemModel::Banner {
            attachment,
            base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            pattern_base_texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            base_color: crate::EntityDyeColor::White,
            patterns: Arc::from([]),
        };
        assert_eq!(
            renderer
                .special_item_model_bounds(&banner(
                    super::super::SpecialItemBannerAttachment::Ground
                ))
                .unwrap(),
            renderer
                .special_item_model_bounds(&banner(super::super::SpecialItemBannerAttachment::Wall))
                .unwrap()
        );
        renderer.render_offscreen_frame().unwrap();

        renderer.set_item_model_residency_frame(frame()).unwrap();
        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_model_vertex_upload_bytes, 0);
        assert_eq!(renderer.counters().item_model_index_upload_bytes, 0);
    }
}
