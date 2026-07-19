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
    counters::RendererCounters, frame_buffers::FrameDataBuffer, item_models::ItemModelVertex,
    Renderer,
};

use super::{bake_item_model_mesh, ItemModelFoil, ItemModelQuad};

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
}

impl ItemModelAssetIdentity {
    pub fn new(model_id: impl Into<Arc<str>>, variant: impl Into<Arc<str>>, part: u32) -> Self {
        Self {
            model_id: model_id.into(),
            variant: variant.into(),
            part,
        }
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
    Translucent,
    Glint,
    GlintTranslucent,
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
    pub phase: ItemModelDrawPhase,
    pub submit_order: u32,
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
            phase,
            submit_order,
        }
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
    pub(crate) _vertex_buffer: wgpu::Buffer,
    pub(crate) _index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) logical_bytes: u64,
    last_used_frame: u64,
}

pub(crate) struct ItemModelResidency {
    resource_generation: ItemModelResourceGeneration,
    block_atlas_generation: ItemModelAtlasGeneration,
    item_atlas_generation: ItemModelAtlasGeneration,
    owners: BTreeMap<ItemModelAssetKey, ResidentItemModelGpu>,
    pinned: BTreeSet<ItemModelAssetKey>,
    instances: Vec<ItemModelInstanceRecord>,
    gpu_instances: Vec<ItemModelGpuInstance>,
    instance_buffer: FrameDataBuffer,
    frame_index: u64,
    resident_bytes: u64,
    max_owners: usize,
    max_bytes: u64,
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
            owners: BTreeMap::new(),
            pinned: BTreeSet::new(),
            instances: Vec::new(),
            gpu_instances: Vec::new(),
            instance_buffer: FrameDataBuffer::vertex("bbb-item-model-frame-instances"),
            frame_index: 0,
            resident_bytes: 0,
            max_owners,
            max_bytes,
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
        let mesh = resident_geometry(&quads, key.geometry_class);
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
            _vertex_buffer: vertex_buffer,
            _index_buffer: index_buffer,
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
        let previous_capacity = self.instance_buffer.capacity();
        let bytes = bytemuck::cast_slice(&self.gpu_instances);
        let _uploaded = self.instance_buffer.upload(device, queue, bytes);
        if self.instance_buffer.capacity() > previous_capacity {
            counters.item_extraction_scratch_capacity_growths = counters
                .item_extraction_scratch_capacity_growths
                .saturating_add(1);
        }
        counters.item_instance_upload_bytes = bytes.len() as u64;
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

    #[cfg(test)]
    fn set_limits(&mut self, max_owners: usize, max_bytes: u64) {
        self.max_owners = max_owners;
        self.max_bytes = max_bytes;
    }
}

fn resident_geometry(
    quads: &[ItemModelQuad],
    geometry_class: ItemModelGeometryClass,
) -> (Vec<ItemModelResidentVertex>, Vec<u32>) {
    let selected = quads
        .iter()
        .copied()
        .filter(|quad| {
            quad.translucent == matches!(geometry_class, ItemModelGeometryClass::Translucent)
        })
        .collect::<Vec<_>>();
    let mesh = bake_item_model_mesh(&selected, Mat4::IDENTITY);
    let vertices = mesh
        .vertices
        .into_iter()
        .map(|vertex: ItemModelVertex| ItemModelResidentVertex {
            position: vertex.position,
            uv: vertex.uv,
            color: vertex.color,
            normal_diffuse: vertex.normal_diffuse,
        })
        .collect();
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

    /// Advances the resource generation and drops all derived model owners.
    pub fn invalidate_item_model_resources(&mut self) {
        self.item_model_residency
            .invalidate_resources(&mut self.counters);
    }

    pub(crate) fn invalidate_item_model_atlas(&mut self, atlas: ItemModelAtlasKind) {
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
}
