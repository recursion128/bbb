//! Resource-generation item-model CPU/GPU residency and narrow frame instances.
//!
//! The exact key is deliberately decided before a caller supplies model quads.
//! A resident hit therefore never scans or compares the vertex/index payload.

use bbb_client_clock::Instant;
use std::{ops::Range, sync::Arc};

use anyhow::{bail, Result};
use glam::{Mat3, Mat4, Vec3};
use hashbrown::HashMap;

use crate::item_render::{
    bake_generated_item_quads,
    counters::RendererCounters,
    frame_buffers::{FrameDataBuffer, UploadBelt},
    gpu::DEPTH_FORMAT,
    pipeline_builder::{create_surface_pipeline, depth_stencil_state, RenderPipelineBuilder},
    ItemSpriteRect, SpriteAlphaMask,
};

use super::{
    item_model_position, item_quad_indices, item_quad_normal, sheeted_decal_uv, ItemModelFoil,
    ItemModelQuad, ITEM_MODEL_GLINT_BLEND, ITEM_MODEL_GLINT_CULL_MODE,
    ITEM_MODEL_GLINT_DEPTH_COMPARE, ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED,
    ITEM_MODEL_PIPELINE_CULL_MODE, SPECIAL_FOIL_TEXTURE_SCALE,
};

mod arena;
mod pipelines;
use arena::*;
pub use pipelines::{
    create_resident_item_model_cutout_no_cull_pipeline,
    create_resident_item_model_cutout_z_offset_pipeline,
    create_resident_item_model_end_gateway_pipeline,
    create_resident_item_model_end_portal_pipeline, create_resident_item_model_glint_pipeline,
    create_resident_item_model_outline_pipeline, create_resident_item_model_pattern_pipeline,
    create_resident_item_model_pipeline, create_resident_item_model_solid_no_cutout_pipeline,
    create_resident_item_model_translucent_no_cull_pipeline,
    create_resident_item_model_translucent_pipeline,
    create_resident_item_model_z_offset_forward_pipeline, instance_vertex_layout,
    resident_no_alpha_cutout_shader, resident_per_face_shader, resident_vertex_layout,
    ITEM_MODEL_RESIDENT_OUTLINE_BLEND, ITEM_MODEL_RESIDENT_OUTLINE_CULL_MODE,
    ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_COMPARE, ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_WRITE_ENABLED,
    ITEM_MODEL_RESIDENT_OUTLINE_SHADER, ITEM_MODEL_RESIDENT_SHADER,
};
use pipelines::{record_apply_time, record_lookup_time, resident_geometry, resident_logical_bytes};
const DEFAULT_MAX_RESIDENT_OWNERS: usize = 4_096;
const DEFAULT_MAX_RESIDENT_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_CHANGED_INSTANCE_UPLOAD_RANGES: usize = 16;

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

    pub fn with_portal_metadata(mut self) -> Self {
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
    /// Vanilla `RenderTypes.solidMovingBlock()`: depth-writing solid block
    /// geometry without the item/cutout alpha test.
    SolidNoCutout,
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
    /// Vanilla `RenderTypes.outline(...)`: texture alpha is a mask for the
    /// submitted outline color in the dedicated entity-outline target.
    Outline,
}

/// Render target/lifetime lane. It is not part of static asset identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemModelDrawTarget {
    World,
    /// Vanilla `BlockFeatureRenderer` submission lane. These models share the
    /// world attachments but draw after every `ItemFeatureRenderer` phase.
    BlockFeature,
    /// Vanilla `OutputTarget.OUTLINE_TARGET` for blocks-atlas entity
    /// attachments submitted through `BlockModelRenderState.submitOnlyOutline`
    /// or copied from an ordinary affects-outline `submit`.
    EntityOutline,
    Particle,
    FirstPerson,
    /// One sanitized GUI entity preview's private color/depth PIP target.
    /// The payload is the preview index in the active render plan.
    EntityPreview(u32),
    Gui,
    GuiBeforeBlur,
    GuiAfterBlur,
}

const STATIC_TARGET_BUCKETS: usize = 8;

impl ItemModelDrawTarget {
    pub fn is_gui(self) -> bool {
        matches!(self, Self::Gui | Self::GuiBeforeBlur | Self::GuiAfterBlur)
    }

    fn static_bucket_index(self) -> Option<usize> {
        match self {
            Self::World => Some(0),
            Self::BlockFeature => Some(1),
            Self::EntityOutline => Some(2),
            Self::Particle => Some(3),
            Self::FirstPerson => Some(4),
            Self::Gui => Some(5),
            Self::GuiBeforeBlur => Some(6),
            Self::GuiAfterBlur => Some(7),
            Self::EntityPreview(_) => None,
        }
    }
}

/// GUI-space clip carried by one resident item submission. The values are in
/// logical GUI pixels (`Window.setGuiScale`); the draw boundary converts them
/// to physical pixels with the `physical / gui` ratio before issuing the
/// native scissor.
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

    pub fn clipped_to(self, width: u32, height: u32) -> Option<Self> {
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

/// Immutable resource-model geometry shared by resolved stacks and resident submissions.
#[derive(Debug, PartialEq)]
pub struct ItemModelGeometry {
    quads: Box<[ItemModelQuad]>,
    extents: Option<(Vec3, Vec3)>,
    geometry_classes: [bool; 2],
}

impl ItemModelGeometry {
    pub(crate) fn new(quads: Vec<ItemModelQuad>) -> Self {
        let extents = quad_extents(&quads, Mat4::IDENTITY);
        let mut geometry_classes = [false; 2];
        for quad in &quads {
            geometry_classes[usize::from(quad.translucent)] = true;
        }
        Self {
            quads: quads.into_boxed_slice(),
            extents,
            geometry_classes,
        }
    }

    fn quads(&self) -> &[ItemModelQuad] {
        &self.quads
    }

    pub fn has_geometry_class(&self, class: ItemModelGeometryClass) -> bool {
        self.geometry_classes[usize::from(class == ItemModelGeometryClass::Translucent)]
    }

    pub(crate) fn transformed_extents(&self, transform: Mat4) -> Option<(Vec3, Vec3)> {
        quad_extents(&self.quads, transform)
    }
}

fn quad_extents(quads: &[ItemModelQuad], transform: Mat4) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for quad in quads {
        for corner in quad.corners {
            let point =
                transform.transform_point3(Vec3::from_array(corner) / MODEL_UNITS_PER_BLOCK);
            min = min.min(point);
            max = max.max(point);
        }
    }
    min.is_finite().then_some((min, max))
}

/// Small model-space source carried across the native extraction boundary.
/// The renderer consults the exact resident key before evaluating this source.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelAssetSource {
    Model(Arc<ItemModelGeometry>),
    Quads(Arc<[ItemModelQuad]>),
    Generated {
        mask: SpriteAlphaMask,
        rect: ItemSpriteRect,
        translucent: bool,
    },
}

impl ItemModelAssetSource {
    pub fn build(self) -> Vec<ItemModelQuad> {
        match self {
            Self::Model(model) => model.quads().to_vec(),
            Self::Quads(quads) => quads.as_ref().to_vec(),
            Self::Generated {
                mask,
                rect,
                translucent,
            } => bake_generated_item_quads(&mask, rect, [1.0; 4], translucent),
        }
    }

    /// The source geometry's model-space corner extents in block units, i.e.
    /// the same points vanilla `ItemStackRenderState.visitExtents` feeds into
    /// `getModelBoundingBox`. `None` when the source bakes no geometry.
    pub fn model_extents(&self) -> Option<(Vec3, Vec3)> {
        match self {
            Self::Model(model) => model.extents,
            Self::Quads(quads) => quad_extents(quads, Mat4::IDENTITY),
            Self::Generated { .. } => {
                use crate::item_render::generated_item::{MAX_Z, MIN_Z};
                Some((
                    Vec3::new(0.0, 0.0, MIN_Z) / MODEL_UNITS_PER_BLOCK,
                    Vec3::new(16.0, 16.0, MAX_Z) / MODEL_UNITS_PER_BLOCK,
                ))
            }
        }
    }
}

/// Item model geometry is authored in sixteenths of a block.
const MODEL_UNITS_PER_BLOCK: f32 = 16.0;

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
pub struct ItemModelResidentVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub normal_diffuse: [f32; 4],
    pub decal_uv: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ItemModelGpuInstance {
    pub model_0: [f32; 4],
    pub model_1: [f32; 4],
    pub model_2: [f32; 4],
    pub model_3: [f32; 4],
    pub normal_0: [f32; 4],
    pub normal_1: [f32; 4],
    pub normal_2: [f32; 4],
    pub tint: [f32; 4],
    pub light_overlay: [f32; 4],
    pub foil: [f32; 4],
}

impl ItemModelGpuInstance {
    pub fn from_record(record: &ItemModelInstanceRecord) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentItemModelGpu {
    pub base_vertex: i32,
    pub first_index: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub logical_bytes: u64,
    pub last_used_frame: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentItemModelDraw {
    pub asset: ItemModelAssetKey,
    pub(crate) slot: u32,
    pub target: ItemModelDrawTarget,
    pub phase: ItemModelDrawPhase,
    pub first_instance: u32,
    pub instance_count: u32,
    pub scissor: Option<ItemModelScissorRect>,
}

pub struct ItemModelResidency {
    pub resource_generation: ItemModelResourceGeneration,
    pub block_atlas_generation: ItemModelAtlasGeneration,
    pub item_atlas_generation: ItemModelAtlasGeneration,
    pub entity_atlas_generation: ItemModelAtlasGeneration,
    pub dynamic_player_skin_atlas_generation: ItemModelAtlasGeneration,
    pub dynamic_entity_atlas_generation: ItemModelAtlasGeneration,
    index_by_key: HashMap<ItemModelAssetKey, u32>,
    slots: Vec<Option<OwnerSlot>>,
    free_slots: Vec<u32>,
    lru_head: u32,
    lru_tail: u32,
    active_owners: usize,
    pin_epoch: u64,
    pinned_count: usize,
    vertex_arena: ItemArenaBuffer,
    index_arena: ItemArenaBuffer,
    pub instances: Vec<ItemModelInstanceRecord>,
    pub gpu_instances: Vec<ItemModelGpuInstance>,
    pub changed_instance_byte_ranges: Vec<Range<usize>>,
    pending_instance_upload_ranges: Vec<Range<usize>>,
    retired_instance_buffers: Vec<wgpu::Buffer>,
    unsubmitted_instance_upload: bool,
    draws: Vec<ResidentItemModelDraw>,
    static_target_draw_indices: [Vec<usize>; STATIC_TARGET_BUCKETS],
    entity_preview_draw_indices: HashMap<u32, Vec<usize>>,
    instance_buffer: FrameDataBuffer,
    pub frame_index: u64,
    pub resident_bytes: u64,
    pub max_owners: usize,
    pub max_bytes: u64,
    pending_vertex_upload_bytes: u64,
    pending_index_upload_bytes: u64,
    pending_instance_upload_bytes: u64,
    pending_instance_upload_writes: u64,
    pending_instance_buffer_replacements: u64,
    pub(crate) special_geometry:
        std::cell::RefCell<crate::item_models::special_geometry::SpecialItemGeometryCache>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemModelUploadObservation {
    pub vertex_bytes: u64,
    pub index_bytes: u64,
    pub instance_bytes: u64,
    pub instance_writes: u64,
    pub instance_buffer_replacements: u64,
    pub instances: usize,
}

impl ItemModelResidency {
    pub(crate) fn collect_retire(mut self, buffers: &mut Vec<wgpu::Buffer>) {
        if let Some(buffer) = self.vertex_arena.buffer.take() {
            buffers.push(buffer);
        }
        if let Some((buffer, _)) = self.vertex_arena.unsubmitted_old_buffer.take() {
            buffers.push(buffer);
        }
        buffers.append(&mut self.vertex_arena.retired_buffers);
        if let Some(buffer) = self.index_arena.buffer.take() {
            buffers.push(buffer);
        }
        if let Some((buffer, _)) = self.index_arena.unsubmitted_old_buffer.take() {
            buffers.push(buffer);
        }
        buffers.append(&mut self.index_arena.retired_buffers);
        if let Some(buffer) = self.instance_buffer.take_buffer() {
            buffers.push(buffer);
        }
        buffers.append(&mut self.retired_instance_buffers);
    }

    pub fn new() -> Self {
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
            index_by_key: HashMap::new(),
            slots: Vec::new(),
            free_slots: Vec::new(),
            lru_head: LRU_NONE,
            lru_tail: LRU_NONE,
            active_owners: 0,
            pin_epoch: 1,
            pinned_count: 0,
            vertex_arena: ItemArenaBuffer::new(
                "bbb-client-item-model-vertex-arena",
                wgpu::BufferUsages::VERTEX,
                std::mem::size_of::<ItemModelResidentVertex>(),
                i32::MAX as u32,
            ),
            index_arena: ItemArenaBuffer::new(
                "bbb-client-item-model-index-arena",
                wgpu::BufferUsages::INDEX,
                std::mem::size_of::<u32>(),
                u32::MAX,
            ),
            instances: Vec::new(),
            gpu_instances: Vec::new(),
            changed_instance_byte_ranges: Vec::new(),
            pending_instance_upload_ranges: Vec::new(),
            retired_instance_buffers: Vec::new(),
            unsubmitted_instance_upload: false,
            draws: Vec::new(),
            static_target_draw_indices: std::array::from_fn(|_| Vec::new()),
            entity_preview_draw_indices: HashMap::new(),
            instance_buffer: FrameDataBuffer::vertex("bbb-client-item-model-frame-instances"),
            frame_index: 0,
            resident_bytes: 0,
            max_owners,
            max_bytes,
            pending_vertex_upload_bytes: 0,
            pending_index_upload_bytes: 0,
            pending_instance_upload_bytes: 0,
            pending_instance_upload_writes: 0,
            pending_instance_buffer_replacements: 0,
            special_geometry: Default::default(),
        }
    }

    /// Resource and atlas generations a baked special part's texture
    /// rectangles depend on.
    pub(crate) fn special_geometry_generation(
        &self,
    ) -> crate::item_models::special_geometry::SpecialGeometryGeneration {
        [
            self.resource_generation.0,
            self.block_atlas_generation.0,
            self.item_atlas_generation.0,
            self.entity_atlas_generation.0,
            self.dynamic_player_skin_atlas_generation.0,
            self.dynamic_entity_atlas_generation.0,
        ]
    }

    pub fn empty_for_resource_reload(&self) -> Self {
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

    pub fn key(
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

    pub fn atlas_generation(&self, atlas: ItemModelAtlasKind) -> ItemModelAtlasGeneration {
        match atlas {
            ItemModelAtlasKind::Blocks => self.block_atlas_generation,
            ItemModelAtlasKind::Items => self.item_atlas_generation,
            ItemModelAtlasKind::Entity => self.entity_atlas_generation,
            ItemModelAtlasKind::DynamicPlayerSkin => self.dynamic_player_skin_atlas_generation,
            ItemModelAtlasKind::DynamicEntity => self.dynamic_entity_atlas_generation,
        }
    }

    pub fn key_is_current(&self, key: &ItemModelAssetKey) -> bool {
        key.resource_generation == self.resource_generation
            && key.atlas_generation == self.atlas_generation(key.atlas)
    }

    pub fn begin_frame(&mut self, counters: &mut RendererCounters) {
        self.frame_index = self
            .frame_index
            .checked_add(1)
            .expect("item frame overflow");
        self.pin_epoch = self
            .pin_epoch
            .checked_add(1)
            .expect("item pin epoch overflow");
        self.pinned_count = 0;
        self.instances.clear();
        self.draws.clear();
        self.clear_draw_indices();
        counters.item_model_lookup_nanos = 0;
        counters.item_gpu_apply_nanos = 0;
        counters.item_model_vertex_upload_bytes = 0;
        counters.item_model_index_upload_bytes = 0;
        counters.item_instance_upload_bytes = 0;
        counters.item_instance_upload_writes = 0;
        counters.item_instance_buffer_replacements = 0;
        counters.item_model_instances = 0;
    }

    fn lru_unlink(&mut self, idx: u32) {
        let (prev, next) = {
            let slot = self.slots[idx as usize]
                .as_ref()
                .expect("occupied item owner slot");
            (slot.lru_prev, slot.lru_next)
        };
        if prev != LRU_NONE {
            self.slots[prev as usize].as_mut().unwrap().lru_next = next;
        } else {
            self.lru_head = next;
        }
        if next != LRU_NONE {
            self.slots[next as usize].as_mut().unwrap().lru_prev = prev;
        } else {
            self.lru_tail = prev;
        }
        let slot = self.slots[idx as usize].as_mut().unwrap();
        slot.lru_prev = LRU_NONE;
        slot.lru_next = LRU_NONE;
    }

    fn lru_push_back(&mut self, idx: u32) {
        let prev = self.lru_tail;
        if prev != LRU_NONE {
            self.slots[prev as usize].as_mut().unwrap().lru_next = idx;
        } else {
            self.lru_head = idx;
        }
        let slot = self.slots[idx as usize].as_mut().unwrap();
        slot.lru_prev = prev;
        slot.lru_next = LRU_NONE;
        self.lru_tail = idx;
    }

    fn touch_and_pin(&mut self, idx: u32) {
        let epoch = self.pin_epoch;
        let frame_index = self.frame_index;
        let slot = self.slots[idx as usize]
            .as_mut()
            .expect("occupied item owner slot");
        slot.gpu.last_used_frame = frame_index;
        if slot.pinned_epoch != epoch {
            slot.pinned_epoch = epoch;
            self.pinned_count += 1;
        }
        if self.lru_tail != idx {
            self.lru_unlink(idx);
            self.lru_push_back(idx);
        }
    }

    fn remove_slot(&mut self, idx: u32) -> OwnerSlot {
        self.lru_unlink(idx);
        let removed = self.slots[idx as usize]
            .take()
            .expect("occupied item owner slot");
        self.index_by_key.remove(&removed.key);
        if removed.pinned_epoch == self.pin_epoch {
            self.pinned_count = self.pinned_count.saturating_sub(1);
        }
        self.active_owners = self.active_owners.saturating_sub(1);
        self.free_slots.push(idx);
        self.vertex_arena
            .free(removed.gpu.base_vertex as u32, removed.gpu.vertex_count);
        self.index_arena
            .free(removed.gpu.first_index, removed.gpu.index_count);
        self.resident_bytes = self
            .resident_bytes
            .saturating_sub(removed.gpu.logical_bytes);
        removed
    }

    pub fn ensure(
        &mut self,
        _device: &wgpu::Device,
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
        if let Some(&slot_idx) = self.index_by_key.get(&key) {
            self.touch_and_pin(slot_idx);
            counters.item_model_residency_hits =
                counters.item_model_residency_hits.saturating_add(1);
            counters.item_gpu_owners_reused = counters.item_gpu_owners_reused.saturating_add(1);
            record_lookup_time(counters, lookup_started);
            return ItemModelResidencyOutcome::Reused;
        }
        counters.item_model_residency_misses =
            counters.item_model_residency_misses.saturating_add(1);
        record_lookup_time(counters, lookup_started);

        if self.max_owners == 0
            || (self.active_owners >= self.max_owners && self.pinned_count >= self.active_owners)
        {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            return ItemModelResidencyOutcome::CapacityExceeded;
        }

        let apply_started = Instant::now();
        let (vertices, indices) =
            resident_geometry(build(), key.geometry_class, key.identity.portal_metadata);
        if indices.is_empty() {
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::Empty;
        }
        let logical_bytes = resident_logical_bytes(&vertices, &indices);
        if logical_bytes > self.max_bytes || !self.evict_until_fits(logical_bytes, counters) {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        }

        let (Ok(vertex_count), Ok(index_count)) =
            (u32::try_from(vertices.len()), u32::try_from(indices.len()))
        else {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        };
        let Some(first_vertex) = self.vertex_arena.allocate(vertex_count) else {
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        };
        let Some(first_index) = self.index_arena.allocate(index_count) else {
            self.vertex_arena.free(first_vertex, vertex_count);
            counters.item_model_fallbacks = counters.item_model_fallbacks.saturating_add(1);
            record_apply_time(counters, apply_started);
            return ItemModelResidencyOutcome::CapacityExceeded;
        };
        let base_vertex = i32::try_from(first_vertex).expect("vertex arena capped at i32::MAX");
        let vertex_bytes = bytemuck::cast_slice(&vertices);
        let index_bytes = bytemuck::cast_slice(&indices);
        let vertex_upload_bytes = vertex_bytes.len() as u64;
        let index_upload_bytes = index_bytes.len() as u64;
        self.vertex_arena.queue_write(first_vertex, vertex_bytes);
        self.index_arena.queue_write(first_index, index_bytes);

        let gpu = ResidentItemModelGpu {
            base_vertex,
            first_index,
            vertex_count,
            index_count,
            logical_bytes,
            last_used_frame: self.frame_index,
        };
        let slot_idx = if let Some(reused) = self.free_slots.pop() {
            self.slots[reused as usize] = Some(OwnerSlot {
                key: key.clone(),
                gpu,
                pinned_epoch: self.pin_epoch,
                lru_prev: LRU_NONE,
                lru_next: LRU_NONE,
            });
            reused
        } else {
            let next = u32::try_from(self.slots.len()).expect("item owner slot count fits u32");
            self.slots.push(Some(OwnerSlot {
                key: key.clone(),
                gpu,
                pinned_epoch: self.pin_epoch,
                lru_prev: LRU_NONE,
                lru_next: LRU_NONE,
            }));
            next
        };
        self.lru_push_back(slot_idx);
        self.index_by_key.insert(key, slot_idx);
        self.active_owners += 1;
        self.pinned_count += 1;
        self.resident_bytes = self.resident_bytes.saturating_add(logical_bytes);

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

    pub fn evict_until_fits(
        &mut self,
        incoming_bytes: u64,
        counters: &mut RendererCounters,
    ) -> bool {
        if self.max_owners == 0 || incoming_bytes > self.max_bytes {
            return false;
        }
        while self.active_owners >= self.max_owners
            || self.resident_bytes.saturating_add(incoming_bytes) > self.max_bytes
        {
            if self.pinned_count >= self.active_owners {
                return false;
            }
            let mut candidate = self.lru_head;
            while candidate != LRU_NONE {
                let slot = self.slots[candidate as usize]
                    .as_ref()
                    .expect("occupied LRU item slot");
                if slot.pinned_epoch != self.pin_epoch {
                    break;
                }
                candidate = slot.lru_next;
            }
            if candidate == LRU_NONE {
                return false;
            }
            self.remove_slot(candidate);
            counters.item_gpu_owners_replaced = counters.item_gpu_owners_replaced.saturating_add(1);
        }
        true
    }

    pub fn set_instances(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        counters: &mut RendererCounters,
        instances: Vec<ItemModelInstanceRecord>,
    ) -> Result<()> {
        if instances.len() > u32::MAX as usize {
            bail!("item-model frame exceeds the u32 instance address space");
        }
        let mut instance_slots = Vec::with_capacity(instances.len());
        for instance in &instances {
            if !self.key_is_current(&instance.asset) {
                counters.item_model_residency_stale =
                    counters.item_model_residency_stale.saturating_add(1);
                bail!("item-model instance references a stale resource or atlas generation");
            }
            let Some(&slot_idx) = self.index_by_key.get(&instance.asset) else {
                bail!("item-model instance references a missing resident owner");
            };
            instance_slots.push(slot_idx);
        }

        for &slot_idx in &instance_slots {
            self.touch_and_pin(slot_idx);
        }

        let apply_started = Instant::now();
        let instance_size = std::mem::size_of::<ItemModelGpuInstance>();
        self.changed_instance_byte_ranges.clear();
        for (index, instance) in instances.iter().enumerate() {
            let next = ItemModelGpuInstance::from_record(instance);
            let changed = if let Some(current) = self.gpu_instances.get_mut(index) {
                if bytemuck::bytes_of(current) != bytemuck::bytes_of(&next) {
                    *current = next;
                    true
                } else {
                    false
                }
            } else {
                self.gpu_instances.push(next);
                true
            };
            if changed {
                let start = index
                    .checked_mul(instance_size)
                    .expect("instance byte offset overflow");
                let end = start
                    .checked_add(instance_size)
                    .expect("instance byte range overflow");
                if self
                    .changed_instance_byte_ranges
                    .last_mut()
                    .is_some_and(|range| {
                        if range.end == start {
                            range.end = end;
                            true
                        } else {
                            false
                        }
                    })
                {
                    continue;
                }
                self.changed_instance_byte_ranges.push(start..end);
            }
        }
        self.gpu_instances.truncate(instances.len());
        if self.changed_instance_byte_ranges.len() > MAX_CHANGED_INSTANCE_UPLOAD_RANGES {
            let start = self
                .changed_instance_byte_ranges
                .first()
                .expect("range count checked above")
                .start;
            let end = self
                .changed_instance_byte_ranges
                .last()
                .expect("range count checked above")
                .end;
            self.changed_instance_byte_ranges.clear();
            self.changed_instance_byte_ranges.push(start..end);
        }
        self.draws.clear();
        self.clear_draw_indices();
        for (index, (instance, &slot_idx)) in instances.iter().zip(&instance_slots).enumerate() {
            let index = u32::try_from(index).expect("instance count checked above");
            if let Some(previous) = self.draws.last_mut() {
                if previous.slot == slot_idx
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
            let draw_index = self.draws.len();
            self.draws.push(ResidentItemModelDraw {
                asset: instance.asset.clone(),
                slot: slot_idx,
                target: instance.target,
                phase: instance.phase,
                first_instance: index,
                instance_count: 1,
                scissor: instance.scissor,
            });
            self.push_draw_index(instance.target, draw_index);
        }
        let previous_capacity = self.instance_buffer.capacity();
        let bytes_len = self.gpu_instances.len() * instance_size;
        let (uploaded, retired) = self.instance_buffer.prepare_changed_ranges(
            device,
            bytes_len,
            &self.changed_instance_byte_ranges,
        );
        if let Some(old) = retired {
            self.retired_instance_buffers.push(old);
        }
        if uploaded.buffer_replaced {
            self.pending_instance_upload_ranges.clear();
            if bytes_len > 0 {
                self.pending_instance_upload_ranges.push(0..bytes_len);
            }
        } else {
            merge_byte_ranges(
                &mut self.pending_instance_upload_ranges,
                &self.changed_instance_byte_ranges,
                bytes_len,
            );
        }
        if self.instance_buffer.capacity() > previous_capacity {
            counters.item_extraction_scratch_capacity_growths = counters
                .item_extraction_scratch_capacity_growths
                .saturating_add(1);
        }
        counters.item_instance_upload_bytes = uploaded.bytes;
        counters.item_instance_upload_writes = uploaded.writes;
        counters.item_instance_buffer_replacements = u64::from(uploaded.buffer_replaced);
        self.pending_instance_upload_bytes = self
            .pending_instance_upload_bytes
            .saturating_add(uploaded.bytes);
        self.pending_instance_upload_writes = self
            .pending_instance_upload_writes
            .saturating_add(uploaded.writes);
        self.pending_instance_buffer_replacements = self
            .pending_instance_buffer_replacements
            .saturating_add(u64::from(uploaded.buffer_replaced));
        counters.item_model_instances = instances.len();
        self.instances = instances;
        record_apply_time(counters, apply_started);
        Ok(())
    }

    pub(crate) fn stage_pending_uploads(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut UploadBelt,
    ) {
        self.vertex_arena.stage(device, encoder, belt);
        self.index_arena.stage(device, encoder, belt);
        for old in self.retired_instance_buffers.drain(..) {
            belt.retire_buffer(old);
        }
        if !self.pending_instance_upload_ranges.is_empty() {
            if let Some(buffer) = self.instance_buffer.buffer() {
                let bytes: &[u8] = bytemuck::cast_slice(&self.gpu_instances);
                for range in self.pending_instance_upload_ranges.drain(..) {
                    let end = range.end.min(bytes.len());
                    if range.start < end {
                        belt.write_bytes(
                            encoder,
                            buffer,
                            range.start as u64,
                            &bytes[range.start..end],
                        );
                    }
                }
                self.unsubmitted_instance_upload = true;
            } else {
                self.pending_instance_upload_ranges.clear();
            }
        }
    }

    pub(crate) fn mark_submitted(&mut self) {
        self.vertex_arena.mark_submitted();
        self.index_arena.mark_submitted();
        self.unsubmitted_instance_upload = false;
    }

    pub(crate) fn drain_retired_into(&mut self, belt: &mut UploadBelt) {
        self.vertex_arena.drain_retired_into(belt);
        self.index_arena.drain_retired_into(belt);
        for old in self.retired_instance_buffers.drain(..) {
            belt.retire_buffer(old);
        }
    }

    pub(crate) fn restore_unsubmitted(&mut self) {
        self.vertex_arena.restore_unsubmitted();
        self.index_arena.restore_unsubmitted();
        if self.unsubmitted_instance_upload {
            self.unsubmitted_instance_upload = false;
            let bytes_len = self.gpu_instances.len() * std::mem::size_of::<ItemModelGpuInstance>();
            self.pending_instance_upload_ranges.clear();
            if bytes_len > 0 {
                self.pending_instance_upload_ranges.push(0..bytes_len);
            }
        }
    }

    pub fn invalidate_atlas(&mut self, atlas: ItemModelAtlasKind, counters: &mut RendererCounters) {
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
        let matching_slots = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| {
                slot.as_ref()
                    .filter(|slot| slot.key.atlas == atlas)
                    .map(|_| idx as u32)
            })
            .collect::<Vec<_>>();
        for idx in matching_slots {
            self.remove_slot(idx);
            counters.item_gpu_owners_replaced = counters.item_gpu_owners_replaced.saturating_add(1);
        }
        self.instances
            .retain(|instance| instance.asset.atlas != atlas);
        self.draws.retain(|draw| draw.asset.atlas != atlas);
        self.rebuild_draw_indices();
        self.refresh_resident_counters(counters);
    }

    pub fn invalidate_resources(&mut self, counters: &mut RendererCounters) {
        self.resource_generation = ItemModelResourceGeneration(
            self.resource_generation
                .0
                .checked_add(1)
                .expect("item resource generation overflow"),
        );
        let removed = self.active_owners as u64;
        if removed != 0 {
            counters.item_full_residency_replacements =
                counters.item_full_residency_replacements.saturating_add(1);
            counters.item_gpu_owners_replaced =
                counters.item_gpu_owners_replaced.saturating_add(removed);
        }
        self.index_by_key.clear();
        self.slots.clear();
        self.free_slots.clear();
        self.lru_head = LRU_NONE;
        self.lru_tail = LRU_NONE;
        self.active_owners = 0;
        self.pinned_count = 0;
        self.vertex_arena.clear();
        self.index_arena.clear();
        self.instances.clear();
        self.gpu_instances.clear();
        self.pending_instance_upload_ranges.clear();
        self.unsubmitted_instance_upload = false;
        self.draws.clear();
        self.clear_draw_indices();
        self.resident_bytes = 0;
        self.refresh_resident_counters(counters);
    }

    pub fn refresh_resident_counters(&self, counters: &mut RendererCounters) {
        debug_assert_eq!(self.active_owners, self.index_by_key.len());
        counters.item_model_resident_bytes = self.resident_bytes;
        counters.item_model_peak_resident_bytes = counters
            .item_model_peak_resident_bytes
            .max(self.resident_bytes);
    }

    pub fn owner(&self, key: &ItemModelAssetKey) -> Option<&ResidentItemModelGpu> {
        let &idx = self.index_by_key.get(key)?;
        self.slots.get(idx as usize)?.as_ref().map(|slot| &slot.gpu)
    }

    pub(crate) fn draw_owner(&self, draw: &ResidentItemModelDraw) -> Option<&ResidentItemModelGpu> {
        self.slots
            .get(draw.slot as usize)?
            .as_ref()
            .map(|slot| &slot.gpu)
    }

    pub(crate) fn shared_buffers(&self) -> Option<(&wgpu::Buffer, &wgpu::Buffer, &wgpu::Buffer)> {
        Some((
            self.vertex_arena.buffer()?,
            self.index_arena.buffer()?,
            self.instance_buffer.buffer()?,
        ))
    }

    fn clear_draw_indices(&mut self) {
        for bucket in &mut self.static_target_draw_indices {
            bucket.clear();
        }
        self.entity_preview_draw_indices.clear();
    }

    fn push_draw_index(&mut self, target: ItemModelDrawTarget, index: usize) {
        match target.static_bucket_index() {
            Some(bucket) => self.static_target_draw_indices[bucket].push(index),
            None => {
                let ItemModelDrawTarget::EntityPreview(preview_index) = target else {
                    unreachable!("only EntityPreview lacks a static bucket");
                };
                self.entity_preview_draw_indices
                    .entry(preview_index)
                    .or_default()
                    .push(index);
            }
        }
    }

    fn rebuild_draw_indices(&mut self) {
        self.clear_draw_indices();
        for index in 0..self.draws.len() {
            let target = self.draws[index].target;
            self.push_draw_index(target, index);
        }
    }

    fn draw_indices_for_target(&self, target: ItemModelDrawTarget) -> &[usize] {
        match target.static_bucket_index() {
            Some(bucket) => &self.static_target_draw_indices[bucket],
            None => {
                let ItemModelDrawTarget::EntityPreview(preview_index) = target else {
                    unreachable!("only EntityPreview lacks a static bucket");
                };
                self.entity_preview_draw_indices
                    .get(&preview_index)
                    .map_or(&[], Vec::as_slice)
            }
        }
    }

    pub(crate) fn has_draws_for_target(&self, target: ItemModelDrawTarget) -> bool {
        !self.draw_indices_for_target(target).is_empty()
    }

    pub(crate) fn draws_for_target(
        &self,
        target: ItemModelDrawTarget,
    ) -> impl Iterator<Item = &ResidentItemModelDraw> + '_ {
        self.draw_indices_for_target(target)
            .iter()
            .map(|&index| &self.draws[index])
    }

    pub fn draws(&self) -> &[ResidentItemModelDraw] {
        &self.draws
    }

    pub fn instance_buffer(&self) -> Option<&wgpu::Buffer> {
        self.instance_buffer.buffer()
    }

    pub fn take_upload_observation(&mut self) -> ItemModelUploadObservation {
        let observation = ItemModelUploadObservation {
            vertex_bytes: self.pending_vertex_upload_bytes,
            index_bytes: self.pending_index_upload_bytes,
            instance_bytes: self.pending_instance_upload_bytes,
            instance_writes: self.pending_instance_upload_writes,
            instance_buffer_replacements: self.pending_instance_buffer_replacements,
            instances: self.instances.len(),
        };
        self.pending_vertex_upload_bytes = 0;
        self.pending_index_upload_bytes = 0;
        self.pending_instance_upload_bytes = 0;
        self.pending_instance_upload_writes = 0;
        self.pending_instance_buffer_replacements = 0;
        observation
    }

    pub fn set_limits(&mut self, max_owners: usize, max_bytes: u64) {
        self.max_owners = max_owners;
        self.max_bytes = max_bytes;
    }
}

impl Default for ItemModelResidency {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
