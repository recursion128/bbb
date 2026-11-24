//! Item-frame filled-map surfaces and vanilla `MapRenderer` decoration sprite submissions.

use std::{collections::BTreeMap, sync::Arc};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::{
    frame_buffers::FrameDataBuffer,
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    HudFontGlyphBook, Renderer, WorldTextRun,
};

use super::ITEM_MODEL_PIPELINE_CULL_MODE;

mod atlases;
#[cfg(test)]
use super::{ItemModelMesh, ItemModelVertex};
use atlases::*;

use crate::text_mesh::{
    argb_color, world_text_chars, world_text_visual_order, world_text_width, TextPen,
};
#[cfg(test)]
use crate::{HudAsciiGlyph, HudFontGlyphMap};
use bbb_render_types::vanilla_map_decoration_type;
pub use bbb_render_types::{
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture, ItemFrameMapDecorationTexture,
    VanillaMapDecorationType as ItemFrameMapDecorationType,
};

const ITEM_FRAME_MAP_SIZE: u32 = 128;
const ITEM_FRAME_MAP_RGBA_LEN: usize =
    ITEM_FRAME_MAP_SIZE as usize * ITEM_FRAME_MAP_SIZE as usize * 4;
const MAP_ATLAS_PAGE_SIDE_SLOTS: u32 = 8;
const MAP_ATLAS_PAGE_CAPACITY: usize =
    (MAP_ATLAS_PAGE_SIDE_SLOTS * MAP_ATLAS_PAGE_SIDE_SLOTS) as usize;
const MAX_RESIDENT_MAP_TEXTURES: usize = 4_096;
const MAX_RESIDENT_MAP_PAGES: usize = MAX_RESIDENT_MAP_TEXTURES / MAP_ATLAS_PAGE_CAPACITY;
// Cap transient overflow pages (8 * 64 = 512 maps = 32 MiB VRAM) and reuse them across frames.
const MAX_OVERFLOW_MAP_PAGES: usize = 8;
const ITEM_FRAME_MAP_DECORATION_ATLAS_PATH: &str = "minecraft:textures/atlas/map_decorations.png";
// All Component fonts share the uploaded font-book atlas.
const ITEM_FRAME_MAP_TEXT_FONT_PATH: &str = "minecraft:font/default";
const FIRST_PERSON_MAP_BACKGROUND_BORDER: f32 = 7.0;
const FIRST_PERSON_MAP_BACKGROUND_SIZE: f32 = 142.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexturePatch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    pub rgba: Arc<[u8]>,
}

/// Exact immutable source for vanilla's dynamic `minecraft:map/<id>` texture.
/// A repeated owner-generation/revision pair shares the full RGBA payload; a
/// consecutive canonical patch additionally carries only the changed rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexture {
    pub map_id: i32,
    pub owner_generation: u64,
    pub content_revision: u64,
    pub rgba: Arc<[u8]>,
    pub patch: Option<ItemFrameMapTexturePatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstPersonMapBackgroundTextureRef {
    pub kind: FirstPersonMapBackgroundKind,
}

impl FirstPersonMapBackgroundTextureRef {
    pub fn vanilla_path(self) -> &'static str {
        self.kind.vanilla_path()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFrameMapRenderType {
    Text,
}

impl ItemFrameMapRenderType {
    pub fn vanilla_name(self) -> &'static str {
        match self {
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapTextureRef {
    pub map_id: i32,
}

impl ItemFrameMapTextureRef {
    pub fn vanilla_path(self) -> String {
        format!("minecraft:map/{}", self.map_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapSubmission {
    pub map_id: i32,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMapBackgroundSubmission {
    pub kind: FirstPersonMapBackgroundKind,
    pub render_type: ItemFrameMapRenderType,
    pub texture: FirstPersonMapBackgroundTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTextureRef {
    pub sprite_id: &'static str,
}

impl ItemFrameMapDecorationTextureRef {
    pub fn vanilla_atlas_path(self) -> &'static str {
        ITEM_FRAME_MAP_DECORATION_ATLAS_PATH
    }

    pub fn vanilla_sprite_id(self) -> &'static str {
        self.sprite_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapDecorationSubmission {
    pub type_id: i32,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapDecorationTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
    pub decoration_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapTextTextureRef;

impl ItemFrameMapTextTextureRef {
    pub fn vanilla_path(self) -> &'static str {
        ITEM_FRAME_MAP_TEXT_FONT_PATH
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapTextSubmission {
    pub type_id: i32,
    pub text: String,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapTextTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
    pub decoration_index: u32,
    pub width: f32,
    pub scale: f32,
}

/// The base map surface submit for an item frame. Production retains only the
/// narrow transform/light/map identity; tests reconstruct the legacy quad to
/// prove the static model-space instance is pose-equivalent.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapSurface {
    pub submission: ItemFrameMapSubmission,
    #[cfg(test)]
    mesh: ItemModelMesh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMapBackgroundSurface {
    pub submission: FirstPersonMapBackgroundSubmission,
    #[cfg(test)]
    mesh: ItemModelMesh,
}

impl ItemFrameMapSurface {
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn vertex_count(&self) -> usize {
        4
    }

    pub fn index_count(&self) -> usize {
        6
    }
}

impl FirstPersonMapBackgroundSurface {
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn vertex_count(&self) -> usize {
        4
    }

    pub fn index_count(&self) -> usize {
        6
    }
}

/// A vanilla `MapRenderer` decoration sprite submit. Production carries only
/// the narrow submission; tests retain the former world-space quad oracle.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapDecorationSurface {
    pub submission: ItemFrameMapDecorationSubmission,
    #[cfg(test)]
    mesh: ItemModelMesh,
}

/// Resident background, glyphs and effects for a map decoration name.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapTextSurface {
    pub submission: ItemFrameMapTextSubmission,
    dynamic_atlas_dims: Option<(u32, u32)>,
    background: ItemFrameMapTextGlyphInstance,
    glyphs: Vec<ItemFrameMapTextGlyphInstance>,
    #[cfg(test)]
    mesh: ItemModelMesh,
}

/// One resident map surface assigned to a sanitized GUI map-preview slot.
///
/// The preview index is deliberately independent of the map id: cartography
/// duplication renders the same map twice at different transforms and GUI
/// strata while sharing one dynamic texture owner.
#[derive(Debug, Clone, PartialEq)]
pub struct HudMapPreviewSurface<T> {
    pub preview_index: u8,
    pub surface: T,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ItemFrameMapTextGlyphInstance {
    transform: Mat4,
    uv: ItemFrameMapUvRect,
    tint: [f32; 4],
    solid: bool,
}

impl ItemFrameMapTextSurface {
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn vertex_count(&self) -> usize {
        self.glyphs.len().saturating_add(1).saturating_mul(4)
    }

    pub fn index_count(&self) -> usize {
        self.glyphs.len().saturating_add(1).saturating_mul(6)
    }
}

impl ItemFrameMapDecorationSurface {
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn vertex_count(&self) -> usize {
        4
    }

    pub fn index_count(&self) -> usize {
        6
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemFrameMapUvRect {
    min: [f32; 2],
    max: [f32; 2],
}

impl ItemFrameMapUvRect {
    #[cfg(test)]
    fn map(self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * uv[0],
            self.min[1] + (self.max[1] - self.min[1]) * uv[1],
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemFrameMapAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<i32, ItemFrameMapUvRect>,
}

pub(crate) struct ItemFrameMapAtlasGpu {
    pages: Vec<ItemFrameMapAtlasPageGpu>,
    overflow_pages: Vec<ItemFrameMapAtlasPageGpu>,
    overflow_locations: BTreeMap<i32, OverflowMapTexture>,
    pub(crate) layout: ItemFrameMapAtlasLayout,
    owners: BTreeMap<i32, ResidentMapTexture>,
    frame_index: u64,
    max_resident_textures: usize,
    max_overflow_pages: usize,
}

pub(crate) struct ItemFrameMapAtlasPageGpu {
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
    free_slots: Vec<u32>,
}

#[derive(Debug, Clone)]
struct ResidentMapTexture {
    map_id: i32,
    page: usize,
    slot: u32,
    owner_generation: u64,
    content_revision: u64,
    rgba: Arc<[u8]>,
    last_used_frame: u64,
}

#[derive(Debug, Clone, Copy)]
struct OverflowMapTexture {
    page: usize,
}

impl ItemFrameMapAtlasGpu {
    pub(crate) fn page_bind_group(
        &self,
        page: usize,
        target: MapSurfaceTarget,
    ) -> Option<&wgpu::BindGroup> {
        self.pages
            .get(page)
            .or_else(|| {
                self.overflow_pages
                    .get(page.saturating_sub(self.pages.len()))
            })
            .map(|page| match target {
                MapSurfaceTarget::World => &page.bind_group,
                MapSurfaceTarget::FirstPerson => &page.hud3d_bind_group,
                MapSurfaceTarget::Gui(_) => &page.gui_bind_group,
            })
    }

    fn map_page(&self, map_id: i32) -> Option<usize> {
        self.owners
            .get(&map_id)
            .map(|owner| owner.page)
            .or_else(|| self.overflow_locations.get(&map_id).map(|owner| owner.page))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FirstPersonMapBackgroundAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<FirstPersonMapBackgroundKind, ItemFrameMapUvRect>,
}

pub(crate) struct FirstPersonMapBackgroundAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: FirstPersonMapBackgroundAtlasLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemFrameMapDecorationAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<String, ItemFrameMapUvRect>,
}

pub(crate) struct ItemFrameMapDecorationAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
    pub(crate) layout: ItemFrameMapDecorationAtlasLayout,
}

pub(crate) struct ItemFrameMapTextFontAtlasGpu {
    _view: wgpu::TextureView,
    _effect_texture: wgpu::Texture,
    _effect_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) effect_bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
}

impl ItemFrameMapTextFontAtlasGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._effect_texture
    }
}

impl FirstPersonMapBackgroundAtlasGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._texture
    }
}

impl ItemFrameMapAtlasGpu {
    pub(crate) fn collect_retire(self, textures: &mut Vec<wgpu::Texture>) {
        textures.extend(self.pages.into_iter().map(|page| page.texture));
        textures.extend(self.overflow_pages.into_iter().map(|page| page.texture));
    }
}

impl ItemFrameMapDecorationAtlasGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._texture
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct MapSurfaceVertex {
    position: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct MapSurfaceGpuInstance {
    model: [[f32; 4]; 4],
    uv_rect: [f32; 4],
    light: [f32; 4],
    tint: [f32; 4],
}

impl MapSurfaceGpuInstance {
    fn new(transform: Mat4, uv: ItemFrameMapUvRect, light: [f32; 2]) -> Self {
        Self {
            model: transform.to_cols_array_2d(),
            uv_rect: [uv.min[0], uv.min[1], uv.max[0], uv.max[1]],
            light: [light[0], light[1], 0.0, 0.0],
            tint: [1.0; 4],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapSurfaceTarget {
    World,
    FirstPerson,
    Gui(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapSurfaceDraw {
    pub(crate) target: MapSurfaceTarget,
    pub(crate) page: usize,
    pub(crate) first_instance: u32,
    pub(crate) instance_count: u32,
}

pub(crate) struct MapSurfaceResidency {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) gui_pipeline: wgpu::RenderPipeline,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    dynamic_instances: Vec<MapSurfaceGpuInstance>,
    dynamic_buffer: FrameDataBuffer,
    pub(crate) dynamic_draws: Vec<MapSurfaceDraw>,
    dynamic_upload_bytes: u64,
    background_instances: Vec<MapSurfaceGpuInstance>,
    background_buffer: FrameDataBuffer,
    pub(crate) background_count: u32,
    background_upload_bytes: u64,
    decoration_instances: Vec<MapSurfaceGpuInstance>,
    decoration_buffer: FrameDataBuffer,
    pub(crate) decoration_draws: Vec<MapSurfaceDraw>,
    decoration_upload_bytes: u64,
    text_instances: Vec<MapSurfaceGpuInstance>,
    text_buffer: FrameDataBuffer,
    pub(crate) text_draws: Vec<MapSurfaceDraw>,
    text_upload_bytes: u64,
    text_atlas_dims: Option<(u32, u32)>,
    pending_text_atlas_dims: Option<(u32, u32)>,
}

impl MapSurfaceResidency {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        atlas_bind_group_layout: &wgpu::BindGroupLayout,
        lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let vertices = [
            MapSurfaceVertex {
                position: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
            MapSurfaceVertex {
                position: [1.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
            MapSurfaceVertex {
                position: [1.0, 0.0, 0.0],
                uv: [1.0, 0.0],
            },
            MapSurfaceVertex {
                position: [0.0, 0.0, 0.0],
                uv: [0.0, 0.0],
            },
        ];
        let indices = [0u32, 1, 2, 0, 2, 3];
        let vertex_buffer = crate::frame_buffers::create_initialized_buffer(
            device,
            Some("bbb-map-surface-resident-vertices"),
            bytemuck::cast_slice(&vertices),
            wgpu::BufferUsages::VERTEX,
        );
        let index_buffer = crate::frame_buffers::create_initialized_buffer(
            device,
            Some("bbb-map-surface-resident-indices"),
            bytemuck::cast_slice(&indices),
            wgpu::BufferUsages::INDEX,
        );
        Self {
            pipeline: create_map_surface_pipeline(
                device,
                format,
                atlas_bind_group_layout,
                lightmap_bind_group_layout,
            ),
            gui_pipeline: create_gui_map_surface_pipeline(device, format, atlas_bind_group_layout),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            dynamic_instances: Vec::new(),
            dynamic_buffer: FrameDataBuffer::vertex("bbb-map-surface-frame-instances"),
            dynamic_draws: Vec::new(),
            dynamic_upload_bytes: 0,
            background_instances: Vec::new(),
            background_buffer: FrameDataBuffer::vertex("bbb-map-background-frame-instances"),
            background_count: 0,
            background_upload_bytes: 0,
            decoration_instances: Vec::new(),
            decoration_buffer: FrameDataBuffer::vertex("bbb-map-decoration-frame-instances"),
            decoration_draws: Vec::new(),
            decoration_upload_bytes: 0,
            text_instances: Vec::new(),
            text_buffer: FrameDataBuffer::vertex("bbb-map-text-frame-instances"),
            text_draws: Vec::new(),
            text_upload_bytes: 0,
            text_atlas_dims: None,
            pending_text_atlas_dims: None,
        }
    }

    pub(crate) fn set_text_atlas_dims(&mut self, dims: Option<(u32, u32)>) {
        self.text_atlas_dims = dims;
        self.pending_text_atlas_dims = None;
    }

    pub(crate) fn queue_text_atlas_rescale(&mut self, new_dims: (u32, u32)) {
        self.pending_text_atlas_dims = Some(new_dims);
    }

    pub(crate) fn flush_pending_text_atlas_rescale(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if let Some(new_dims) = self.pending_text_atlas_dims.take() {
            self.rescale_text_atlas_uvs(device, queue, new_dims);
        }
    }

    pub(crate) fn collect_retire(mut self, buffers: &mut Vec<wgpu::Buffer>) {
        buffers.push(self.vertex_buffer);
        buffers.push(self.index_buffer);
        for frame_buf in [
            &mut self.dynamic_buffer,
            &mut self.background_buffer,
            &mut self.decoration_buffer,
            &mut self.text_buffer,
        ] {
            if let Some(buffer) = frame_buf.take_buffer() {
                buffers.push(buffer);
            }
        }
    }

    pub(crate) fn rescale_text_atlas_uvs(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_dims: (u32, u32),
    ) {
        let previous = self.text_atlas_dims.replace(new_dims);
        let Some((old_w, old_h)) = previous else {
            return;
        };
        if (old_w, old_h) == new_dims || new_dims.0 == 0 || new_dims.1 == 0 {
            return;
        }
        let scale_x = old_w as f32 / new_dims.0 as f32;
        let scale_y = old_h as f32 / new_dims.1 as f32;
        for instance in &mut self.text_instances {
            if instance.light[2] == 0.0 {
                instance.uv_rect[0] *= scale_x;
                instance.uv_rect[1] *= scale_y;
                instance.uv_rect[2] *= scale_x;
                instance.uv_rect[3] *= scale_y;
            }
        }
        if !self.text_instances.is_empty() {
            let upload =
                upload_map_instances(device, queue, &mut self.text_buffer, &self.text_instances);
            self.text_upload_bytes = upload.bytes;
        }
    }

    pub(crate) fn dynamic_buffer(&self) -> Option<&wgpu::Buffer> {
        (!self.dynamic_instances.is_empty())
            .then(|| self.dynamic_buffer.buffer())
            .flatten()
    }

    pub(crate) fn background_buffer(&self) -> Option<&wgpu::Buffer> {
        (self.background_count != 0)
            .then(|| self.background_buffer.buffer())
            .flatten()
    }

    pub(crate) fn decoration_buffer(&self) -> Option<&wgpu::Buffer> {
        (!self.decoration_instances.is_empty())
            .then(|| self.decoration_buffer.buffer())
            .flatten()
    }

    pub(crate) fn text_buffer(&self) -> Option<&wgpu::Buffer> {
        (!self.text_instances.is_empty())
            .then(|| self.text_buffer.buffer())
            .flatten()
    }

    fn set_dynamic(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: Option<&ItemFrameMapAtlasGpu>,
        item_frame_surfaces: Vec<ItemFrameMapSurface>,
        first_person_surfaces: Vec<ItemFrameMapSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapSurface>>,
    ) -> MapInstanceUpload {
        self.dynamic_instances.clear();
        self.dynamic_draws.clear();
        let Some(atlas) = atlas else {
            self.dynamic_upload_bytes = 0;
            return MapInstanceUpload::default();
        };
        self.push_dynamic_target(atlas, item_frame_surfaces, MapSurfaceTarget::World);
        self.push_dynamic_target(atlas, first_person_surfaces, MapSurfaceTarget::FirstPerson);
        for surface in gui_surfaces {
            self.push_dynamic_target(
                atlas,
                vec![surface.surface],
                MapSurfaceTarget::Gui(surface.preview_index),
            );
        }
        let upload = upload_map_instances(
            device,
            queue,
            &mut self.dynamic_buffer,
            &self.dynamic_instances,
        );
        self.dynamic_upload_bytes = upload.bytes;
        upload
    }

    fn push_dynamic_target(
        &mut self,
        atlas: &ItemFrameMapAtlasGpu,
        surfaces: Vec<ItemFrameMapSurface>,
        target: MapSurfaceTarget,
    ) {
        for surface in surfaces {
            let Some(rect) = atlas.layout.rects.get(&surface.submission.map_id).copied() else {
                continue;
            };
            let Some(page) = atlas.map_page(surface.submission.map_id) else {
                continue;
            };
            let first_instance = u32::try_from(self.dynamic_instances.len())
                .expect("map surface instance count fits u32");
            self.dynamic_instances.push(MapSurfaceGpuInstance::new(
                surface.submission.transform
                    * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.01))
                    * Mat4::from_scale(Vec3::new(
                        ITEM_FRAME_MAP_SIZE as f32,
                        ITEM_FRAME_MAP_SIZE as f32,
                        1.0,
                    )),
                rect,
                surface.submission.light,
            ));
            if let Some(draw) = self.dynamic_draws.last_mut() {
                if draw.target == target
                    && draw.page == page
                    && draw.first_instance.saturating_add(draw.instance_count) == first_instance
                {
                    draw.instance_count = draw.instance_count.saturating_add(1);
                    continue;
                }
            }
            self.dynamic_draws.push(MapSurfaceDraw {
                target,
                page,
                first_instance,
                instance_count: 1,
            });
        }
    }

    fn set_background(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: Option<&FirstPersonMapBackgroundAtlasGpu>,
        surfaces: Vec<FirstPersonMapBackgroundSurface>,
    ) -> MapInstanceUpload {
        self.background_instances.clear();
        let Some(atlas) = atlas else {
            self.background_count = 0;
            self.background_upload_bytes = 0;
            return MapInstanceUpload::default();
        };
        self.background_instances
            .extend(surfaces.into_iter().filter_map(|surface| {
                let rect = atlas.layout.rects.get(&surface.submission.kind).copied()?;
                Some(MapSurfaceGpuInstance::new(
                    surface.submission.transform
                        * Mat4::from_translation(Vec3::new(
                            -FIRST_PERSON_MAP_BACKGROUND_BORDER,
                            -FIRST_PERSON_MAP_BACKGROUND_BORDER,
                            0.0,
                        ))
                        * Mat4::from_scale(Vec3::new(
                            FIRST_PERSON_MAP_BACKGROUND_SIZE,
                            FIRST_PERSON_MAP_BACKGROUND_SIZE,
                            1.0,
                        )),
                    rect,
                    surface.submission.light,
                ))
            }));
        self.background_count = u32::try_from(self.background_instances.len())
            .expect("map background instance count fits u32");
        let upload = upload_map_instances(
            device,
            queue,
            &mut self.background_buffer,
            &self.background_instances,
        );
        self.background_upload_bytes = upload.bytes;
        upload
    }

    fn set_decorations(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: Option<&ItemFrameMapDecorationAtlasGpu>,
        item_frame_surfaces: Vec<ItemFrameMapDecorationSurface>,
        first_person_surfaces: Vec<ItemFrameMapDecorationSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapDecorationSurface>>,
    ) -> MapInstanceUpload {
        self.decoration_instances.clear();
        self.decoration_draws.clear();
        let Some(atlas) = atlas else {
            self.decoration_upload_bytes = 0;
            return MapInstanceUpload::default();
        };
        self.push_decoration_target(atlas, item_frame_surfaces, MapSurfaceTarget::World);
        self.push_decoration_target(atlas, first_person_surfaces, MapSurfaceTarget::FirstPerson);
        for surface in gui_surfaces {
            self.push_decoration_target(
                atlas,
                vec![surface.surface],
                MapSurfaceTarget::Gui(surface.preview_index),
            );
        }
        let upload = upload_map_instances(
            device,
            queue,
            &mut self.decoration_buffer,
            &self.decoration_instances,
        );
        self.decoration_upload_bytes = upload.bytes;
        upload
    }

    fn push_decoration_target(
        &mut self,
        atlas: &ItemFrameMapDecorationAtlasGpu,
        surfaces: Vec<ItemFrameMapDecorationSurface>,
        target: MapSurfaceTarget,
    ) {
        let first_instance = self.decoration_instances.len();
        self.decoration_instances
            .extend(surfaces.into_iter().filter_map(|surface| {
                let rect = atlas
                    .layout
                    .rects
                    .get(surface.submission.texture.sprite_id)
                    .copied()?;
                let rect = ItemFrameMapUvRect {
                    min: [rect.min[0], rect.max[1]],
                    max: [rect.max[0], rect.min[1]],
                };
                let local_z = surface.submission.decoration_index as f32 * -0.001;
                Some(MapSurfaceGpuInstance::new(
                    surface.submission.transform
                        * Mat4::from_translation(Vec3::new(-1.0, -1.0, local_z))
                        * Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0)),
                    rect,
                    surface.submission.light,
                ))
            }));
        let instance_count = self
            .decoration_instances
            .len()
            .saturating_sub(first_instance);
        if instance_count != 0 {
            let first_instance =
                u32::try_from(first_instance).expect("map decoration instance count fits u32");
            let instance_count =
                u32::try_from(instance_count).expect("map decoration instance count fits u32");
            if let Some(draw) = self.decoration_draws.last_mut() {
                if draw.target == target
                    && draw.first_instance.saturating_add(draw.instance_count) == first_instance
                {
                    draw.instance_count = draw.instance_count.saturating_add(instance_count);
                    return;
                }
            }
            self.decoration_draws.push(MapSurfaceDraw {
                target,
                page: 0,
                first_instance,
                instance_count,
            });
        }
    }

    fn set_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enabled: bool,
        item_frame_surfaces: Vec<ItemFrameMapTextSurface>,
        first_person_surfaces: Vec<ItemFrameMapTextSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapTextSurface>>,
    ) -> MapInstanceUpload {
        if let Some(pending_dims) = self.pending_text_atlas_dims.take() {
            self.text_atlas_dims = Some(pending_dims);
        }
        self.text_instances.clear();
        self.text_draws.clear();
        if !enabled {
            self.text_upload_bytes = 0;
            return MapInstanceUpload::default();
        }
        self.push_text_target(item_frame_surfaces, MapSurfaceTarget::World);
        self.push_text_target(first_person_surfaces, MapSurfaceTarget::FirstPerson);
        for surface in gui_surfaces {
            self.push_text_target(
                vec![surface.surface],
                MapSurfaceTarget::Gui(surface.preview_index),
            );
        }
        let upload =
            upload_map_instances(device, queue, &mut self.text_buffer, &self.text_instances);
        self.text_upload_bytes = upload.bytes;
        upload
    }

    fn push_text_target(
        &mut self,
        surfaces: Vec<ItemFrameMapTextSurface>,
        target: MapSurfaceTarget,
    ) {
        let first_instance = self.text_instances.len();
        let current_dims = self.text_atlas_dims;
        for surface in surfaces {
            let uv_scale = match (surface.dynamic_atlas_dims, current_dims) {
                (Some((old_w, old_h)), Some((cur_w, cur_h)))
                    if (old_w, old_h) != (cur_w, cur_h) && cur_w > 0 && cur_h > 0 =>
                {
                    Some((old_w as f32 / cur_w as f32, old_h as f32 / cur_h as f32))
                }
                _ => None,
            };
            self.text_instances.extend(
                std::iter::once(surface.background)
                    .chain(surface.glyphs)
                    .map(|mut glyph| {
                        if !glyph.solid {
                            if let Some((sx, sy)) = uv_scale {
                                glyph.uv.min[0] *= sx;
                                glyph.uv.min[1] *= sy;
                                glyph.uv.max[0] *= sx;
                                glyph.uv.max[1] *= sy;
                            }
                        }
                        let mut instance = MapSurfaceGpuInstance::new(
                            glyph.transform,
                            glyph.uv,
                            surface.submission.light,
                        );
                        instance.tint = glyph.tint;
                        instance.light[2] = f32::from(glyph.solid);
                        instance
                    }),
            );
        }
        let instance_count = self.text_instances.len().saturating_sub(first_instance);
        if instance_count != 0 {
            let first_instance =
                u32::try_from(first_instance).expect("map text instance count fits u32");
            let instance_count =
                u32::try_from(instance_count).expect("map text instance count fits u32");
            if let Some(draw) = self.text_draws.last_mut() {
                if draw.target == target
                    && draw.first_instance.saturating_add(draw.instance_count) == first_instance
                {
                    draw.instance_count = draw.instance_count.saturating_add(instance_count);
                    return;
                }
            }
            self.text_draws.push(MapSurfaceDraw {
                target,
                page: 0,
                first_instance,
                instance_count,
            });
        }
    }

    fn instance_count(&self) -> usize {
        self.dynamic_instances
            .len()
            .saturating_add(self.background_instances.len())
            .saturating_add(self.decoration_instances.len())
            .saturating_add(self.text_instances.len())
    }

    fn upload_bytes(&self) -> u64 {
        self.dynamic_upload_bytes
            .saturating_add(self.background_upload_bytes)
            .saturating_add(self.decoration_upload_bytes)
            .saturating_add(self.text_upload_bytes)
    }

    fn clear(&mut self) {
        self.dynamic_instances.clear();
        self.dynamic_draws.clear();
        self.dynamic_upload_bytes = 0;
        self.background_instances.clear();
        self.background_count = 0;
        self.background_upload_bytes = 0;
        self.decoration_instances.clear();
        self.decoration_draws.clear();
        self.decoration_upload_bytes = 0;
        self.text_instances.clear();
        self.text_draws.clear();
        self.text_upload_bytes = 0;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MapInstanceUpload {
    bytes: u64,
    capacity_grew: bool,
}

fn upload_map_instances(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut FrameDataBuffer,
    instances: &[MapSurfaceGpuInstance],
) -> MapInstanceUpload {
    if instances.is_empty() {
        return MapInstanceUpload::default();
    }
    let previous_capacity = buffer.capacity();
    let bytes = bytemuck::cast_slice(instances);
    let _uploaded = buffer.upload(device, queue, bytes);
    MapInstanceUpload {
        bytes: bytes.len() as u64,
        capacity_grew: buffer.capacity() > previous_capacity,
    }
}

const MAP_SURFACE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
const MAP_SURFACE_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x4,
    6 => Float32x4,
    7 => Float32x4,
    8 => Float32x4
];

fn create_map_surface_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let bind_group_layouts = [atlas_bind_group_layout, lightmap_bind_group_layout];
    let vertex_buffers = map_surface_vertex_buffer_layouts();
    RenderPipelineBuilder::new(device, "bbb-map-surface-resident-pipeline")
        .shader("bbb-map-surface-resident-shader", MAP_SURFACE_SHADER)
        .layout(
            "bbb-map-surface-resident-pipeline-layout",
            &bind_group_layouts,
        )
        .vertex_buffers(&vertex_buffers)
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .cull_mode(ITEM_MODEL_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            true,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

fn create_gui_map_surface_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    atlas_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let bind_group_layouts = [atlas_bind_group_layout];
    let vertex_buffers = map_surface_vertex_buffer_layouts();
    RenderPipelineBuilder::new(device, "bbb-map-surface-resident-gui-pipeline")
        .shader(
            "bbb-map-surface-resident-gui-shader",
            MAP_SURFACE_GUI_SHADER,
        )
        .layout(
            "bbb-map-surface-resident-gui-pipeline-layout",
            &bind_group_layouts,
        )
        .vertex_buffers(&vertex_buffers)
        .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
        .cull_mode(None)
        .build()
}

fn map_surface_vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 2] {
    [
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MapSurfaceVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MAP_SURFACE_VERTEX_ATTRIBUTES,
        },
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MapSurfaceGpuInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &MAP_SURFACE_INSTANCE_ATTRIBUTES,
        },
    ]
}

const MAP_SURFACE_SHADER: &str = concat!(
    crate::wgsl::camera!(),
    crate::wgsl::sample_lightmap!(),
    r#"
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var map_atlas: texture_2d<f32>;
@group(0) @binding(2) var map_sampler: sampler;
@group(1) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(1) var lightmap_sampler: sampler;
struct VertexIn {
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) model_0: vec4<f32>, @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>, @location(5) model_3: vec4<f32>,
    @location(6) uv_rect: vec4<f32>, @location(7) light: vec4<f32>,
    @location(8) tint: vec4<f32>,
};
struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) light: vec2<f32>, @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
    @location(4) tint: vec4<f32>, @location(5) @interpolate(flat) solid: f32,
};
fn linear_fog_value(distance: f32, start: f32, end: f32) -> f32 {
    if (distance <= start) { return 0.0; }
    if (distance >= end) { return 1.0; }
    return (distance - start) / (end - start);
}
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    out.uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.uv);
    out.light = input.light.xy;
    out.tint = input.tint;
    out.solid = input.light.z;
    let fog_position = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_position);
    out.cylindrical_distance = max(length(fog_position.xz), abs(fog_position.y));
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(map_atlas, map_sampler, input.uv);
    let sampled = select(texel, vec4<f32>(1.0), input.solid > 0.5) * input.tint;
    if (sampled.a < 0.1) { discard; }
    let fog_value = max(
        linear_fog_value(input.spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(input.cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    let lit = vec4<f32>(sampled.rgb * sample_lightmap(input.light), sampled.a);
    return vec4<f32>(mix(lit.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), lit.a);
}
"#,
);

const MAP_SURFACE_GUI_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var map_atlas: texture_2d<f32>;
@group(0) @binding(2) var map_sampler: sampler;
struct VertexIn {
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) model_0: vec4<f32>, @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>, @location(5) model_3: vec4<f32>,
    @location(6) uv_rect: vec4<f32>, @location(7) light: vec4<f32>,
    @location(8) tint: vec4<f32>,
};
struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>, @location(2) @interpolate(flat) solid: f32,
};
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    out.position = camera.view_proj * model * vec4<f32>(input.position, 1.0);
    out.uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.uv);
    out.tint = input.tint;
    out.solid = input.light.z;
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(map_atlas, map_sampler, input.uv);
    let sampled = select(texel, vec4<f32>(1.0), input.solid > 0.5) * input.tint;
    if (sampled.a < 0.1) { discard; }
    return sampled;
}
"#;

pub fn item_frame_map_decoration_type(type_id: i32) -> Option<ItemFrameMapDecorationType> {
    vanilla_map_decoration_type(type_id)
}

/// Bakes vanilla `MapRenderer.render`'s base map surface submit:
/// `RenderTypes.text(minecraft:map/<id>)`, white tint, order 0 / sequence 0, and the four
/// `(0,128,-0.01)..(0,0,-0.01)` vertices with UVs 0..1.
pub fn bake_item_frame_map_surface(
    map_id: i32,
    transform: Mat4,
    light: [f32; 2],
) -> ItemFrameMapSurface {
    #[cfg(test)]
    let mesh = legacy_map_surface_mesh(
        transform,
        [
            Vec3::new(0.0, 128.0, -0.01),
            Vec3::new(128.0, 128.0, -0.01),
            Vec3::new(128.0, 0.0, -0.01),
            Vec3::new(0.0, 0.0, -0.01),
        ],
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        light,
    );
    ItemFrameMapSurface {
        submission: ItemFrameMapSubmission {
            map_id,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapTextureRef { map_id },
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 0,
            submit_sequence: 0,
        },
        #[cfg(test)]
        mesh,
    }
}

/// Bakes vanilla `ItemInHandRenderer.renderMap`'s first-person map-background submit:
/// `RenderTypes.text(textures/map/map_background*.png)`, white tint, and the four
/// `(-7,135,0)..(-7,-7,0)` vertices with UVs 0..1 under the same map render transform used by the
/// dynamic 128x128 `MapRenderer` surface.
pub fn bake_first_person_map_background_surface(
    kind: FirstPersonMapBackgroundKind,
    transform: Mat4,
    light: [f32; 2],
) -> FirstPersonMapBackgroundSurface {
    #[cfg(test)]
    let mesh = {
        let min = -FIRST_PERSON_MAP_BACKGROUND_BORDER;
        let max = FIRST_PERSON_MAP_BACKGROUND_SIZE - FIRST_PERSON_MAP_BACKGROUND_BORDER;
        legacy_map_surface_mesh(
            transform,
            [
                Vec3::new(min, max, 0.0),
                Vec3::new(max, max, 0.0),
                Vec3::new(max, min, 0.0),
                Vec3::new(min, min, 0.0),
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            light,
        )
    };
    FirstPersonMapBackgroundSurface {
        submission: FirstPersonMapBackgroundSubmission {
            kind,
            render_type: ItemFrameMapRenderType::Text,
            texture: FirstPersonMapBackgroundTextureRef { kind },
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 0,
            submit_sequence: 0,
        },
        #[cfg(test)]
        mesh,
    }
}

/// Bakes one item-frame-visible vanilla `MapRenderer` decoration sprite submit. The `map_transform`
/// parameter is the same pose stack transform used for the base map surface; this function appends the
/// decoration-local translate/rotate/scale before writing the `(-1..1)` sprite quad. Player/off-map
/// markers return `None` here because `ItemFrameRenderer` calls `MapRenderer.render(..., true, ...)`.
#[allow(clippy::too_many_arguments)]
pub fn bake_item_frame_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
) -> Option<ItemFrameMapDecorationSurface> {
    bake_map_decoration_surface(
        type_id,
        x,
        y,
        rot,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn bake_first_person_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
) -> Option<ItemFrameMapDecorationSurface> {
    bake_map_decoration_surface(
        type_id,
        x,
        y,
        rot,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        false,
    )
}

// GuiGraphicsExtractor.map filters decorations by renderOnFrame.
#[allow(clippy::too_many_arguments)]
pub fn bake_hud_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
) -> Option<ItemFrameMapDecorationSurface> {
    bake_map_decoration_surface(
        type_id,
        x,
        y,
        rot,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn bake_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    show_only_frame: bool,
) -> Option<ItemFrameMapDecorationSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if show_only_frame && !decoration_type.render_on_frame {
        return None;
    }
    let transform = map_transform
        * Mat4::from_translation(Vec3::new(
            f32::from(x) / 2.0 + 64.0,
            f32::from(y) / 2.0 + 64.0,
            -0.02,
        ))
        * Mat4::from_rotation_z((f32::from(rot & 15) * 360.0 / 16.0).to_radians())
        * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
        * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0));
    #[cfg(test)]
    let mesh = {
        let z = decoration_index as f32 * -0.001;
        legacy_map_surface_mesh(
            transform,
            [
                Vec3::new(-1.0, 1.0, z),
                Vec3::new(1.0, 1.0, z),
                Vec3::new(1.0, -1.0, z),
                Vec3::new(-1.0, -1.0, z),
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            light,
        )
    };
    Some(ItemFrameMapDecorationSurface {
        submission: ItemFrameMapDecorationSubmission {
            type_id,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapDecorationTextureRef {
                sprite_id: decoration_type.sprite_id,
            },
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 0,
            submit_sequence,
            decoration_index,
        },
        #[cfg(test)]
        mesh,
    })
}

#[cfg(test)]
fn legacy_map_surface_mesh(
    transform: Mat4,
    corners: [Vec3; 4],
    uvs: [[f32; 2]; 4],
    light: [f32; 2],
) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    mesh.append_raw_textured_quad(
        corners.map(|corner| transform.transform_point3(corner).to_array()),
        uvs,
        [1.0; 4],
        light,
    );
    mesh
}

#[allow(clippy::too_many_arguments)]
pub fn bake_item_frame_map_text_surface(
    type_id: i32,
    runs: &[WorldTextRun],
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        runs,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        fonts,
        obfuscated_seed,
        true,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn bake_first_person_map_text_surface(
    type_id: i32,
    runs: &[WorldTextRun],
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        runs,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        fonts,
        obfuscated_seed,
        false,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn bake_hud_map_text_surface(
    type_id: i32,
    runs: &[WorldTextRun],
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        runs,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        fonts,
        obfuscated_seed,
        true,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn bake_map_text_surface(
    type_id: i32,
    runs: &[WorldTextRun],
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
    show_only_frame: bool,
    flat: bool,
) -> Option<ItemFrameMapTextSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if show_only_frame && !decoration_type.render_on_frame {
        return None;
    }
    let width = item_frame_map_text_width(runs, fonts)? as f32;
    // MapRenderer measures the logical Component before visual ordering.
    let visual = world_text_visual_order(runs, fonts);
    let scale = (25.0 / width).clamp(0.0, 6.0 / 9.0);
    let transform = map_transform
        * Mat4::from_translation(Vec3::new(
            f32::from(x) / 2.0 + 64.0 - width * scale / 2.0,
            f32::from(y) / 2.0 + 64.0 + 4.0,
            -0.025,
        ))
        * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));
    let place_glyphs = || {
        let mut instances = Vec::new();
        let mut effects = Vec::new();
        let mut pen = TextPen::new(0.0, 0.0, obfuscated_seed);
        let mut background_left = -1.0_f32;
        let mut background_right = 0.0_f32;
        for &glyph in &visual {
            let placed = pen.place_world(
                glyph,
                fonts,
                0xFF00_0000 | glyph.1.color.unwrap_or(0xFF_FF_FF),
                false,
            );
            for (quad, z, argb) in placed.glyph_quads() {
                instances.push(map_text_quad(
                    transform,
                    quad.corners,
                    if flat { 0.0 } else { z },
                    ItemFrameMapUvRect {
                        min: quad.uv.min,
                        max: quad.uv.max,
                    },
                    argb_color(argb),
                    false,
                ));
            }
            for (rect, z, argb) in placed.effect_rects() {
                effects.push(map_text_quad(
                    transform,
                    [
                        [rect.x0, rect.y0],
                        [rect.x0, rect.y1],
                        [rect.x1, rect.y1],
                        [rect.x1, rect.y0],
                    ],
                    z,
                    ItemFrameMapUvRect {
                        min: [0.0; 2],
                        max: [0.0; 2],
                    },
                    argb_color(argb),
                    true,
                ));
            }
            // Java Math.min/max propagate NaN; the Rust operations ignore one NaN operand.
            background_left = if background_left.is_nan() || placed.x.is_nan() {
                f32::NAN
            } else {
                background_left.min(placed.x - 1.0)
            };
            let right = placed.x + placed.advance();
            background_right = if background_right.is_nan() || right.is_nan() {
                f32::NAN
            } else {
                background_right.max(right)
            };
        }
        instances.extend(effects);
        (instances, background_left, background_right)
    };
    let dims_before = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
    let (mut instances, mut background_left, mut background_right) = place_glyphs();
    let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
    if dims_after != dims_before {
        (instances, background_left, background_right) = place_glyphs();
    }
    #[cfg(test)]
    let mesh = map_text_instance_mesh(&instances, light);
    Some(ItemFrameMapTextSurface {
        submission: ItemFrameMapTextSubmission {
            type_id,
            text: runs.iter().map(|run| run.text.as_str()).collect(),
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapTextTextureRef,
            tint: [1.0; 4],
            transform,
            light,
            order: 1,
            submit_sequence,
            decoration_index,
            width,
            scale,
        },
        dynamic_atlas_dims: dims_after,
        background: map_text_quad(
            transform,
            [
                [background_left, -1.0],
                [background_left, 9.0],
                [background_right, 9.0],
                [background_right, -1.0],
            ],
            -0.01,
            ItemFrameMapUvRect {
                min: [0.0; 2],
                max: [0.0; 2],
            },
            [0.0, 0.0, 0.0, 128.0 / 255.0],
            true,
        ),
        glyphs: instances,
        #[cfg(test)]
        mesh,
    })
}

fn map_text_quad(
    transform: Mat4,
    corners: [[f32; 2]; 4],
    depth: f32,
    uv: ItemFrameMapUvRect,
    tint: [f32; 4],
    solid: bool,
) -> ItemFrameMapTextGlyphInstance {
    let [tl, bl, _br, tr] = corners.map(|[x, y]| Vec3::new(x, y, depth));
    // Keep the resident quad winding aligned with the map face under Back cull.
    ItemFrameMapTextGlyphInstance {
        transform: transform
            * Mat4::from_cols(
                (tr - tl).extend(0.0),
                (bl - tl).extend(0.0),
                Vec3::Z.extend(0.0),
                tl.extend(1.0),
            ),
        uv,
        tint,
        solid,
    }
}

pub fn item_frame_map_text_width(runs: &[WorldTextRun], fonts: &HudFontGlyphBook) -> Option<i32> {
    let available = fonts.glyphs(None).is_available()
        || runs
            .iter()
            .any(|run| fonts.glyphs(run.font.as_deref()).is_available());
    available.then(|| world_text_width(world_text_chars(runs), fonts))
}

#[cfg(test)]
fn map_text_instance_mesh(
    instances: &[ItemFrameMapTextGlyphInstance],
    light: [f32; 2],
) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    for instance in instances {
        mesh.append_raw_textured_quad(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]
            .map(|point| {
                instance
                    .transform
                    .transform_point3(Vec3::from(point))
                    .to_array()
            }),
            [
                instance.uv.min,
                [instance.uv.max[0], instance.uv.min[1]],
                instance.uv.max,
                [instance.uv.min[0], instance.uv.max[1]],
            ],
            instance.tint,
            light,
        );
    }
    mesh
}

#[cfg(test)]
pub(crate) fn merge_item_frame_map_surfaces(
    surfaces: &[ItemFrameMapSurface],
    atlas: &ItemFrameMapAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas.rects.get(&surface.submission.map_id).copied() else {
            continue;
        };
        let base = u32::try_from(vertices.len()).expect("item-frame map vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied().map(|mut vertex| {
            vertex.uv = rect.map(vertex.uv);
            vertex
        }));
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

#[cfg(test)]
pub(crate) fn merge_first_person_map_background_surfaces(
    surfaces: &[FirstPersonMapBackgroundSurface],
    atlas: &FirstPersonMapBackgroundAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas.rects.get(&surface.submission.kind).copied() else {
            continue;
        };
        let base =
            u32::try_from(vertices.len()).expect("first-person map background vertex count fits");
        vertices.extend(surface.mesh.vertices.iter().copied().map(|mut vertex| {
            vertex.uv = rect.map(vertex.uv);
            vertex
        }));
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

#[cfg(test)]
pub(crate) fn merge_item_frame_map_decoration_surfaces(
    surfaces: &[ItemFrameMapDecorationSurface],
    atlas: &ItemFrameMapDecorationAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas
            .rects
            .get(surface.submission.texture.sprite_id)
            .copied()
        else {
            continue;
        };
        let base = u32::try_from(vertices.len())
            .expect("item-frame map decoration vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied().map(|mut vertex| {
            vertex.uv = rect.map(vertex.uv);
            vertex
        }));
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

impl Renderer {
    #[cfg(test)]
    fn set_map_texture_residency_limit_for_test(&mut self, max_resident_textures: usize) {
        let atlas = self
            .pack
            .item_frame_map_atlas
            .get_or_insert_with(create_item_frame_map_atlas_gpu);
        atlas.max_resident_textures = max_resident_textures;
    }

    pub fn set_first_person_map_background_surfaces(
        &mut self,
        textures: Vec<FirstPersonMapBackgroundTexture>,
        surfaces: Vec<FirstPersonMapBackgroundSurface>,
    ) {
        #[cfg(test)]
        {
            self.first_person_map_background_surfaces = surfaces.clone();
        }
        if self.pack.first_person_map_background_atlas.is_none() {
            let max_dim = self.device.limits().max_texture_dimension_2d;
            match build_first_person_map_background_atlas_with_limits(
                &textures,
                max_dim,
                MAX_MAP_AUX_ATLAS_BYTES,
            ) {
                Ok(Some((layout, rgba))) => {
                    match create_first_person_map_background_atlas_gpu(
                        &self.device,
                        &self.queue,
                        &self.terrain_bind_group_layout,
                        &self.camera_buffers.hud3d(),
                        layout,
                        &rgba,
                    ) {
                        Ok(atlas) => self.pack.first_person_map_background_atlas = Some(atlas),
                        Err(error) => tracing::warn!(
                            %error,
                            "skipping oversized first-person map background atlas"
                        ),
                    }
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(
                    %error,
                    "rejecting oversized first-person map background atlas"
                ),
            }
        }
        let upload = self.pack.map_surface_residency.set_background(
            &self.device,
            &self.queue,
            self.pack.first_person_map_background_atlas.as_ref(),
            surfaces,
        );
        self.record_map_instance_upload(upload);
    }

    /// Sets this frame's filled-map item-frame submissions and updates only
    /// changed owners in the persistent dynamic-map atlas.
    pub fn set_item_frame_map_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapTexture>,
        surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.set_map_surfaces(textures, Vec::new(), surfaces, Vec::new());
    }

    /// Sets all decoded-map base surfaces. World-space item-frame maps and
    /// depth-cleared first-person maps share stable map-id slots; repeated
    /// revisions do not upload pixels and consecutive patches write only their
    /// changed rectangle.
    pub fn set_map_surfaces(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapTexture>,
        first_person_textures: Vec<ItemFrameMapTexture>,
        item_frame_surfaces: Vec<ItemFrameMapSurface>,
        first_person_surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.set_map_surfaces_with_gui(
            item_frame_textures,
            first_person_textures,
            Vec::new(),
            item_frame_surfaces,
            first_person_surfaces,
            Vec::new(),
        );
    }

    /// Sets decoded map surfaces for world, first-person, and GUI consumers.
    /// All three targets share the same persistent map-id atlas.
    pub fn set_map_surfaces_with_gui(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapTexture>,
        first_person_textures: Vec<ItemFrameMapTexture>,
        gui_textures: Vec<ItemFrameMapTexture>,
        item_frame_surfaces: Vec<ItemFrameMapSurface>,
        first_person_surfaces: Vec<ItemFrameMapSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapSurface>>,
    ) {
        #[cfg(test)]
        {
            self.item_frame_map_surfaces = item_frame_surfaces.clone();
            self.first_person_map_surfaces = first_person_surfaces.clone();
        }
        self.counters.item_map_upload_bytes = 0;
        let mut textures = BTreeMap::new();
        for texture in item_frame_textures
            .into_iter()
            .chain(first_person_textures)
            .chain(gui_textures)
        {
            if texture.rgba.len() == ITEM_FRAME_MAP_RGBA_LEN {
                textures.insert(texture.map_id, texture);
            }
        }
        self.prepare_map_atlas_capacity(&textures);
        let mut overflow = Vec::new();
        for texture in textures.into_values() {
            if let Some(texture) = self.update_resident_map_texture(texture) {
                overflow.push(texture);
            }
        }
        self.install_overflow_map_textures(overflow);
        self.refresh_map_texture_counters();
        let upload = self.pack.map_surface_residency.set_dynamic(
            &self.device,
            &self.queue,
            self.pack.item_frame_map_atlas.as_ref(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }

    fn prepare_map_atlas_capacity(&mut self, textures: &BTreeMap<i32, ItemFrameMapTexture>) {
        if textures.is_empty() {
            if let Some(atlas) = &mut self.pack.item_frame_map_atlas {
                if !atlas.overflow_pages.is_empty() {
                    crate::frame_buffers::retire_gpu_textures(
                        &self.queue,
                        atlas.overflow_pages.drain(..).map(|page| page.texture),
                    );
                }
                atlas.overflow_locations.clear();
                atlas
                    .layout
                    .rects
                    .retain(|map_id, _| atlas.owners.contains_key(map_id));
            }
            return;
        }
        let atlas = self
            .pack
            .item_frame_map_atlas
            .get_or_insert_with(create_item_frame_map_atlas_gpu);
        atlas.frame_index = atlas
            .frame_index
            .checked_add(1)
            .expect("map atlas frame overflow");
        // Reuse existing overflow pages across frames instead of reallocating and leaking VRAM.
        for page in &mut atlas.overflow_pages {
            page.free_slots.clear();
            page.free_slots
                .extend((0..MAP_ATLAS_PAGE_CAPACITY as u32).rev());
        }
        atlas.overflow_locations.clear();
        atlas
            .layout
            .rects
            .retain(|map_id, _| atlas.owners.contains_key(map_id));
        let requested = textures
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut missing = requested
            .iter()
            .filter(|map_id| !atlas.owners.contains_key(map_id))
            .count();
        while atlas.owners.len().saturating_add(missing) > atlas.max_resident_textures {
            let candidate = atlas
                .owners
                .iter()
                .filter(|(map_id, _)| !requested.contains(map_id))
                .min_by_key(|(map_id, owner)| (owner.last_used_frame, **map_id))
                .map(|(map_id, _)| *map_id);
            let Some(candidate) = candidate else {
                break;
            };
            remove_resident_map_texture(atlas, candidate);
            missing = requested
                .iter()
                .filter(|map_id| !atlas.owners.contains_key(map_id))
                .count();
        }
    }

    fn update_resident_map_texture(
        &mut self,
        texture: ItemFrameMapTexture,
    ) -> Option<ItemFrameMapTexture> {
        let Some(atlas) = &mut self.pack.item_frame_map_atlas else {
            return Some(texture);
        };
        if let Some(owner) = atlas.owners.get(&texture.map_id).cloned() {
            if owner.owner_generation == texture.owner_generation
                && owner.content_revision == texture.content_revision
            {
                atlas
                    .owners
                    .get_mut(&texture.map_id)
                    .expect("resident map owner checked above")
                    .last_used_frame = atlas.frame_index;
                self.counters.map_textures_reused =
                    self.counters.map_textures_reused.saturating_add(1);
                return None;
            }
            if owner.owner_generation > texture.owner_generation
                || (owner.owner_generation == texture.owner_generation
                    && owner.content_revision > texture.content_revision)
            {
                self.counters.map_textures_stale =
                    self.counters.map_textures_stale.saturating_add(1);
                return None;
            }
            let page = &atlas.pages[owner.page];
            let patched = owner.owner_generation == texture.owner_generation
                && owner.content_revision.checked_add(1) == Some(texture.content_revision)
                && texture.patch.as_ref().is_some_and(|patch| {
                    write_map_texture_patch(
                        &self.queue,
                        &page.texture,
                        MAP_ATLAS_PAGE_SIDE_SLOTS,
                        owner.slot,
                        patch,
                    )
                });
            let uploaded_bytes = if patched {
                texture
                    .patch
                    .as_ref()
                    .map_or(0, |patch| patch.rgba.len() as u64)
            } else {
                let replacement = ResidentMapTexture {
                    map_id: texture.map_id,
                    page: owner.page,
                    slot: owner.slot,
                    owner_generation: texture.owner_generation,
                    content_revision: texture.content_revision,
                    rgba: Arc::clone(&texture.rgba),
                    last_used_frame: atlas.frame_index,
                };
                write_full_map_texture(&self.queue, &page.texture, &replacement);
                texture.rgba.len() as u64
            };
            let owner = atlas
                .owners
                .get_mut(&texture.map_id)
                .expect("resident map owner checked above");
            owner.last_used_frame = atlas.frame_index;
            owner.owner_generation = texture.owner_generation;
            owner.content_revision = texture.content_revision;
            owner.rgba = texture.rgba;
            self.counters.item_map_upload_bytes = self
                .counters
                .item_map_upload_bytes
                .saturating_add(uploaded_bytes);
            self.counters.map_textures_updated =
                self.counters.map_textures_updated.saturating_add(1);
            return None;
        }

        if atlas.owners.len() >= atlas.max_resident_textures {
            return Some(texture);
        }
        let Some((page_index, slot)) = allocate_resident_map_slot(
            atlas,
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
        ) else {
            return Some(texture);
        };
        let owner = ResidentMapTexture {
            map_id: texture.map_id,
            page: page_index,
            slot,
            owner_generation: texture.owner_generation,
            content_revision: texture.content_revision,
            rgba: texture.rgba,
            last_used_frame: atlas.frame_index,
        };
        write_full_map_texture(&self.queue, &atlas.pages[page_index].texture, &owner);
        self.counters.item_map_upload_bytes = self
            .counters
            .item_map_upload_bytes
            .saturating_add(owner.rgba.len() as u64);
        self.counters.map_textures_created = self.counters.map_textures_created.saturating_add(1);
        insert_resident_map_texture(atlas, owner);
        None
    }

    fn install_overflow_map_textures(&mut self, textures: Vec<ItemFrameMapTexture>) {
        let Some(atlas) = &mut self.pack.item_frame_map_atlas else {
            return;
        };
        let mut local_page = 0usize;
        for texture in textures {
            while local_page < atlas.overflow_pages.len()
                && atlas.overflow_pages[local_page].free_slots.is_empty()
            {
                local_page += 1;
            }
            if local_page == atlas.overflow_pages.len() {
                if atlas.overflow_pages.len() >= atlas.max_overflow_pages {
                    break;
                }
                atlas
                    .overflow_pages
                    .push(create_item_frame_map_atlas_page_gpu(
                        &self.device,
                        &self.terrain_bind_group_layout,
                        &self.camera_buffers.world(),
                        &self.camera_buffers.hud3d(),
                        &self.camera_buffers.gui(),
                    ));
            }
            let page = &mut atlas.overflow_pages[local_page];
            let slot = page
                .free_slots
                .pop()
                .expect("overflow map page has a free slot");
            write_full_map_pixels(&self.queue, &page.texture, slot, &texture.rgba);
            let page_index = atlas.pages.len() + local_page;
            atlas.layout.rects.insert(
                texture.map_id,
                map_texture_slot_rect(slot, MAP_ATLAS_PAGE_SIDE_SLOTS),
            );
            atlas
                .overflow_locations
                .insert(texture.map_id, OverflowMapTexture { page: page_index });
            self.counters.item_map_upload_bytes = self
                .counters
                .item_map_upload_bytes
                .saturating_add(texture.rgba.len() as u64);
            self.counters.map_texture_fallbacks =
                self.counters.map_texture_fallbacks.saturating_add(1);
        }
        let used_pages = atlas
            .overflow_pages
            .iter()
            .rposition(|page| page.free_slots.len() < MAP_ATLAS_PAGE_CAPACITY)
            .map_or(0, |last| last + 1);
        if atlas.overflow_pages.len() > used_pages {
            crate::frame_buffers::retire_gpu_textures(
                &self.queue,
                atlas
                    .overflow_pages
                    .drain(used_pages..)
                    .map(|page| page.texture),
            );
        }
    }

    pub fn remove_map_texture(&mut self, map_id: i32) {
        if let Some(atlas) = &mut self.pack.item_frame_map_atlas {
            remove_resident_map_texture(atlas, map_id);
            atlas.overflow_locations.remove(&map_id);
            atlas.layout.rects.remove(&map_id);
        }
        self.refresh_map_texture_counters();
    }

    pub fn invalidate_map_texture_residency(&mut self) {
        let mut retired = Vec::new();
        if let Some(map_atlas) = self.pack.item_frame_map_atlas.take() {
            self.counters.map_full_residency_replacements = self
                .counters
                .map_full_residency_replacements
                .saturating_add(1);
            map_atlas.collect_retire(&mut retired);
        }
        if let Some(bg) = self.pack.first_person_map_background_atlas.take() {
            retired.push(bg.into_texture());
        }
        if let Some(decorations) = self.pack.item_frame_map_decoration_atlas.take() {
            retired.push(decorations.into_texture());
        }
        if !retired.is_empty() {
            crate::frame_buffers::retire_gpu_textures(&self.queue, retired);
        }
        self.pack.map_surface_residency.clear();
        self.refresh_map_texture_counters();
        self.refresh_map_instance_counters();
    }

    fn refresh_map_texture_counters(&mut self) {
        let bytes = self.pack.item_frame_map_atlas.as_ref().map_or(0, |atlas| {
            (atlas
                .owners
                .len()
                .saturating_add(atlas.overflow_locations.len()) as u64)
                .saturating_mul(ITEM_FRAME_MAP_RGBA_LEN as u64)
        });
        self.counters.map_texture_resident_bytes = bytes;
        self.counters.map_texture_peak_resident_bytes =
            self.counters.map_texture_peak_resident_bytes.max(bytes);
    }

    fn record_map_instance_upload(&mut self, upload: MapInstanceUpload) {
        if upload.capacity_grew {
            self.counters.item_extraction_scratch_capacity_growths = self
                .counters
                .item_extraction_scratch_capacity_growths
                .saturating_add(1);
        }
        self.refresh_map_instance_counters();
    }

    fn refresh_map_instance_counters(&mut self) {
        self.counters.map_instance_upload_bytes = self.pack.map_surface_residency.upload_bytes();
        self.counters.map_surface_instances = self.pack.map_surface_residency.instance_count();
    }

    /// Sets this frame's `MapRenderer` decoration sprite submissions for filled-map item frames. The
    /// atlas is the vanilla `textures/atlas/map_decorations.png` sheet, rebuilt here from the decoded
    /// pack sprites supplied by native.
    pub fn set_item_frame_map_decoration_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapDecorationTexture>,
        surfaces: Vec<ItemFrameMapDecorationSurface>,
    ) {
        self.set_map_decoration_surfaces(textures, Vec::new(), surfaces, Vec::new());
    }

    /// Sets all `MapRenderer` decoration sprite submissions. Item-frame maps use
    /// `showOnlyFrame=true`; first-person maps use `showOnlyFrame=false`, but both sample the same
    /// vanilla map-decoration atlas.
    pub fn set_map_decoration_surfaces(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapDecorationTexture>,
        first_person_textures: Vec<ItemFrameMapDecorationTexture>,
        item_frame_surfaces: Vec<ItemFrameMapDecorationSurface>,
        first_person_surfaces: Vec<ItemFrameMapDecorationSurface>,
    ) {
        self.set_map_decoration_surfaces_with_gui(
            item_frame_textures,
            first_person_textures,
            Vec::new(),
            item_frame_surfaces,
            first_person_surfaces,
            Vec::new(),
        );
    }

    /// Sets map-decoration surfaces for world, first-person, and GUI targets.
    pub fn set_map_decoration_surfaces_with_gui(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapDecorationTexture>,
        first_person_textures: Vec<ItemFrameMapDecorationTexture>,
        gui_textures: Vec<ItemFrameMapDecorationTexture>,
        item_frame_surfaces: Vec<ItemFrameMapDecorationSurface>,
        first_person_surfaces: Vec<ItemFrameMapDecorationSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapDecorationSurface>>,
    ) {
        #[cfg(test)]
        {
            self.item_frame_map_decoration_surfaces = item_frame_surfaces.clone();
            self.first_person_map_decoration_surfaces = first_person_surfaces.clone();
        }
        if self.pack.item_frame_map_decoration_atlas.is_none() {
            let max_dim = self.device.limits().max_texture_dimension_2d;
            match build_item_frame_map_decoration_atlas_with_limits(
                item_frame_textures
                    .iter()
                    .chain(first_person_textures.iter())
                    .chain(gui_textures.iter()),
                max_dim,
                MAX_MAP_AUX_ATLAS_BYTES,
            ) {
                Ok(Some((layout, rgba))) => {
                    match create_item_frame_map_decoration_atlas_gpu(
                        &self.device,
                        &self.queue,
                        &self.terrain_bind_group_layout,
                        &self.camera_buffers.world(),
                        &self.camera_buffers.hud3d(),
                        &self.camera_buffers.gui(),
                        layout,
                        &rgba,
                    ) {
                        Ok(atlas) => self.pack.item_frame_map_decoration_atlas = Some(atlas),
                        Err(error) => {
                            tracing::warn!(%error, "skipping oversized map decoration atlas");
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, "rejecting oversized map decoration atlas");
                }
            }
        }
        let upload = self.pack.map_surface_residency.set_decorations(
            &self.device,
            &self.queue,
            self.pack.item_frame_map_decoration_atlas.as_ref(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }

    /// Binds the uploaded HUD font texture for world text: map labels, signs, text displays and
    /// name tags sample the same atlas as the HUD.
    pub(crate) fn bind_world_text_font_atlas(&mut self, font_texture: &wgpu::Texture) {
        let replacement = create_item_frame_map_text_font_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            font_texture,
        );
        if let Some(previous) = self
            .pack
            .item_frame_map_text_font_atlas
            .replace(replacement)
        {
            self.frame_uploads.retire_texture(previous.into_texture());
        }
    }

    pub fn set_item_frame_map_text_surfaces(&mut self, surfaces: Vec<ItemFrameMapTextSurface>) {
        self.set_map_text_surfaces(surfaces, Vec::new());
    }

    pub fn set_map_text_surfaces(
        &mut self,
        item_frame_surfaces: Vec<ItemFrameMapTextSurface>,
        first_person_surfaces: Vec<ItemFrameMapTextSurface>,
    ) {
        self.set_map_text_surfaces_with_gui(item_frame_surfaces, first_person_surfaces, Vec::new());
    }

    pub fn set_map_text_surfaces_with_gui(
        &mut self,
        item_frame_surfaces: Vec<ItemFrameMapTextSurface>,
        first_person_surfaces: Vec<ItemFrameMapTextSurface>,
        gui_surfaces: Vec<HudMapPreviewSurface<ItemFrameMapTextSurface>>,
    ) {
        let upload = self.pack.map_surface_residency.set_text(
            &self.device,
            &self.queue,
            self.pack.item_frame_map_text_font_atlas.is_some(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT;

    mod background;
    mod rich_text;
    mod surfaces;

    fn map_texture(map_id: i32, rgba: Vec<u8>) -> ItemFrameMapTexture {
        map_texture_at_revision(map_id, 1, 0, rgba, None)
    }

    fn map_texture_at_revision(
        map_id: i32,
        owner_generation: u64,
        content_revision: u64,
        rgba: Vec<u8>,
        patch: Option<ItemFrameMapTexturePatch>,
    ) -> ItemFrameMapTexture {
        ItemFrameMapTexture {
            map_id,
            owner_generation,
            content_revision,
            rgba: rgba.into(),
            patch,
        }
    }

    #[test]
    fn generated_map_decoration_catalog_drives_all_surface_metadata() {
        let map_transform = Mat4::IDENTITY;
        let light = ITEM_MODEL_FULL_BRIGHT_LIGHT;
        for entry in bbb_render_types::vanilla_map_decoration_types() {
            assert_eq!(item_frame_map_decoration_type(entry.type_id), Some(entry));

            let first_person = bake_first_person_map_decoration_surface(
                entry.type_id,
                0,
                0,
                0,
                0,
                map_transform,
                light,
                0,
            )
            .expect("first-person maps show every registered decoration");
            assert_eq!(
                first_person.submission.texture.vanilla_sprite_id(),
                entry.sprite_id
            );

            let item_frame = bake_item_frame_map_decoration_surface(
                entry.type_id,
                0,
                0,
                0,
                0,
                map_transform,
                light,
                0,
            );
            assert_eq!(item_frame.is_some(), entry.render_on_frame);
            if let Some(surface) = item_frame {
                assert_eq!(
                    surface.submission.texture.vanilla_sprite_id(),
                    entry.sprite_id
                );
            }
        }
        assert!(item_frame_map_decoration_type(-1).is_none());
        let unknown_type_id = bbb_render_types::vanilla_map_decoration_types().len() as i32;
        assert!(item_frame_map_decoration_type(unknown_type_id).is_none());
    }

    fn test_map_text_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for byte in b' '..=b'~' {
            glyphs.insert_first_wins(
                char::from(byte),
                HudAsciiGlyph {
                    uv: crate::HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    width: 6,
                    height: 8,
                    advance: match byte {
                        b' ' => 4.0,
                        b'?' => 5.0,
                        _ => 6.0,
                    },
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }
}
