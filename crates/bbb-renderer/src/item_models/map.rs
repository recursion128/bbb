//! Item-frame filled-map surfaces and vanilla `MapRenderer` decoration sprite submissions.

use std::{collections::BTreeMap, sync::Arc};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    frame_buffers::FrameDataBuffer,
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    HudAsciiGlyph, HudFontGlyphMap, Renderer,
};

use super::ITEM_MODEL_PIPELINE_CULL_MODE;
#[cfg(test)]
use super::{ItemModelMesh, ItemModelVertex};

use bbb_render_types::{vanilla_map_decoration_type, visual_order};
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
const ITEM_FRAME_MAP_DECORATION_ATLAS_PATH: &str = "minecraft:textures/atlas/map_decorations.png";
/// The uploaded map-text font texture is the baked multi-page
/// `minecraft:default` glyph atlas (vanilla `Style.DEFAULT_FONT`), not a
/// single provider page.
const ITEM_FRAME_MAP_TEXT_FONT_PATH: &str = "minecraft:font/default";
const ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH: char = '?';
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

/// A vanilla `MapRenderer` decoration name text submit for an item-frame map. `mesh` contains one
/// textured quad per glyph in world space and samples the vanilla ASCII font atlas uploaded by native.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapTextSurface {
    pub submission: ItemFrameMapTextSubmission,
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
}

impl ItemFrameMapTextSurface {
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.glyphs.len().saturating_mul(4)
    }

    pub fn index_count(&self) -> usize {
        self.glyphs.len().saturating_mul(6)
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
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _effect_texture: wgpu::Texture,
    _effect_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) effect_bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
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
}

impl MapSurfaceGpuInstance {
    fn new(transform: Mat4, uv: ItemFrameMapUvRect, light: [f32; 2]) -> Self {
        Self {
            model: transform.to_cols_array_2d(),
            uv_rect: [uv.min[0], uv.min[1], uv.max[0], uv.max[1]],
            light: [light[0], light[1], 0.0, 0.0],
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-map-surface-resident-vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-map-surface-resident-indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
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
        for surface in surfaces {
            self.text_instances
                .extend(surface.glyphs.into_iter().map(|glyph| {
                    MapSurfaceGpuInstance::new(glyph.transform, glyph.uv, surface.submission.light)
                }));
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
const MAP_SURFACE_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x4,
    6 => Float32x4,
    7 => Float32x4
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
        .color_target(format, Some(wgpu::BlendState::REPLACE))
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

const MAP_SURFACE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>, lightmap_factors: vec4<f32>, lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>, sky_light_color: vec4<f32>, ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>, camera_position: vec4<f32>, fog_color: vec4<f32>,
    fog_distances: vec4<f32>, fog_visibility_ends: vec4<f32>,
    minecraft_light0: vec4<f32>, minecraft_light1: vec4<f32>,
};
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
};
struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) light: vec2<f32>, @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
};
fn linear_fog_value(distance: f32, start: f32, end: f32) -> f32 {
    if (distance <= start) { return 0.0; }
    if (distance >= end) { return 1.0; }
    return (distance - start) / (end - start);
}
fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0), vec2<f32>(0.5 / 16.0), vec2<f32>(15.5 / 16.0));
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    out.uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.uv);
    out.light = input.light.xy;
    let fog_position = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_position);
    out.cylindrical_distance = max(length(fog_position.xz), abs(fog_position.y));
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(map_atlas, map_sampler, input.uv);
    if (sampled.a < 0.1) { discard; }
    let fog_value = max(
        linear_fog_value(input.spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(input.cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    let lit = vec4<f32>(sampled.rgb * sample_lightmap(input.light), sampled.a);
    return vec4<f32>(mix(lit.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), lit.a);
}
"#;

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
};
struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
};
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    out.position = camera.view_proj * model * vec4<f32>(input.position, 1.0);
    out.uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.uv);
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(map_atlas, map_sampler, input.uv);
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

/// Bakes a GUI map decoration. Cartography previews use the same
/// `showOnlyFrame=false` visibility rule as an ordinary held map.
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
        false,
    )
}

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

/// Bakes one item-frame-visible vanilla `MapRenderer` decoration name submit. `MapRenderer.render`
/// submits these labels through `submitNodeCollector.order(1).submitText(...)` after the order-0
/// base map and decoration sprite submissions. The width and clamp mirror vanilla `Font.width` plus
/// `Mth.clamp(25.0F / width, 0.0F, 6.0F / 9.0F)` over the `minecraft:default` bitmap glyph pages.
pub fn bake_item_frame_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        text,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        glyphs,
        true,
    )
}

pub fn bake_first_person_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        text,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        glyphs,
        false,
    )
}

/// Bakes GUI map label text with the ordinary-map decoration visibility rule.
pub fn bake_hud_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        text,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        glyphs,
        false,
    )
}

fn bake_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
    show_only_frame: bool,
) -> Option<ItemFrameMapTextSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if show_only_frame && !decoration_type.render_on_frame {
        return None;
    }
    let text = text.into();
    let width = item_frame_map_text_width(&text, glyphs)? as f32;
    // MapRenderer positions/scales from `Font.width(Component)` in logical
    // order, then submits `Component.getVisualOrderText()` for glyph output.
    let logical = text.chars().map(|ch| (ch, ())).collect::<Vec<_>>();
    let visual = visual_order(&logical, glyphs.default_right_to_left());
    let scale = (25.0 / width).clamp(0.0, 6.0 / 9.0);
    let transform = map_transform
        * Mat4::from_translation(Vec3::new(
            f32::from(x) / 2.0 + 64.0 - width * scale / 2.0,
            f32::from(y) / 2.0 + 64.0 + 4.0,
            -0.025,
        ))
        * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));

    let mut glyph_instances = Vec::with_capacity(visual.len());
    #[cfg(test)]
    let mut mesh = ItemModelMesh::new();
    let mut pen_x = 0.0;
    for (ch, ()) in visual {
        let glyph = item_frame_map_text_glyph(ch, glyphs);
        if glyph.width > 0 && glyph.height > 0 {
            let x0 = pen_x;
            #[cfg(test)]
            let x1 = pen_x + glyph.rendered_width();
            // Baseline alignment across font pages (vanilla
            // `GlyphBitmap.getTop()` = `7 - ascent`): ascent-7 pages keep
            // y0 = 0, the accented page rises 3px above the line top.
            let y0 = glyph.baseline_offset();
            let y1 = y0 + glyph.rendered_height();
            glyph_instances.push(ItemFrameMapTextGlyphInstance {
                transform: transform
                    * Mat4::from_translation(Vec3::new(x0, y1, 0.0))
                    * Mat4::from_scale(Vec3::new(
                        glyph.rendered_width(),
                        -glyph.rendered_height(),
                        1.0,
                    )),
                uv: ItemFrameMapUvRect {
                    min: [glyph.uv.min[0], glyph.uv.max[1]],
                    max: [glyph.uv.max[0], glyph.uv.min[1]],
                },
            });
            #[cfg(test)]
            {
                let corners = [
                    transform
                        .transform_point3(Vec3::new(x0, y0, 0.0))
                        .to_array(),
                    transform
                        .transform_point3(Vec3::new(x1, y0, 0.0))
                        .to_array(),
                    transform
                        .transform_point3(Vec3::new(x1, y1, 0.0))
                        .to_array(),
                    transform
                        .transform_point3(Vec3::new(x0, y1, 0.0))
                        .to_array(),
                ];
                mesh.append_raw_textured_quad(
                    corners,
                    [
                        [glyph.uv.min[0], glyph.uv.min[1]],
                        [glyph.uv.max[0], glyph.uv.min[1]],
                        [glyph.uv.max[0], glyph.uv.max[1]],
                        [glyph.uv.min[0], glyph.uv.max[1]],
                    ],
                    [1.0, 1.0, 1.0, 1.0],
                    light,
                );
            }
        }
        pen_x += glyph.styled_advance_f32(Default::default());
    }

    Some(ItemFrameMapTextSurface {
        submission: ItemFrameMapTextSubmission {
            type_id,
            text,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapTextTextureRef,
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 1,
            submit_sequence,
            decoration_index,
            width,
            scale,
        },
        glyphs: glyph_instances,
        #[cfg(test)]
        mesh,
    })
}

pub fn item_frame_map_text_width(text: &str, glyphs: &HudFontGlyphMap) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(item_frame_map_text_glyph(ch, glyphs).advance)?;
    }
    (width > 0).then_some(width)
}

/// Codepoint lookup over the baked first-provider-wins glyph map; codepoints
/// absent from every resolved provider degrade to `?`.
fn item_frame_map_text_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
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
        if self.first_person_map_background_atlas.is_none() {
            self.first_person_map_background_atlas =
                build_first_person_map_background_atlas(&textures).map(|(layout, rgba)| {
                    create_first_person_map_background_atlas_gpu(
                        &self.device,
                        &self.queue,
                        &self.terrain_bind_group_layout,
                        &self.hud3d_camera_buffer,
                        layout,
                        &rgba,
                    )
                });
        }
        let upload = self.map_surface_residency.set_background(
            &self.device,
            &self.queue,
            self.first_person_map_background_atlas.as_ref(),
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
        let upload = self.map_surface_residency.set_dynamic(
            &self.device,
            &self.queue,
            self.item_frame_map_atlas.as_ref(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }

    fn prepare_map_atlas_capacity(&mut self, textures: &BTreeMap<i32, ItemFrameMapTexture>) {
        if textures.is_empty() {
            if let Some(atlas) = &mut self.item_frame_map_atlas {
                atlas.overflow_pages.clear();
                atlas.overflow_locations.clear();
                atlas
                    .layout
                    .rects
                    .retain(|map_id, _| atlas.owners.contains_key(map_id));
            }
            return;
        }
        let atlas = self
            .item_frame_map_atlas
            .get_or_insert_with(|| create_item_frame_map_atlas_gpu());
        atlas.frame_index = atlas
            .frame_index
            .checked_add(1)
            .expect("map atlas frame overflow");
        atlas.overflow_pages.clear();
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
        let Some(atlas) = &mut self.item_frame_map_atlas else {
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
            &self.camera_buffer,
            &self.hud3d_camera_buffer,
            &self.gui_item_camera_buffer,
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
        let Some(atlas) = &mut self.item_frame_map_atlas else {
            return;
        };
        for texture in textures {
            let local_page = atlas.overflow_pages.len().saturating_sub(1);
            let needs_page = atlas
                .overflow_pages
                .get(local_page)
                .is_none_or(|page| page.free_slots.is_empty());
            if needs_page {
                atlas
                    .overflow_pages
                    .push(create_item_frame_map_atlas_page_gpu(
                        &self.device,
                        &self.terrain_bind_group_layout,
                        &self.camera_buffer,
                        &self.hud3d_camera_buffer,
                        &self.gui_item_camera_buffer,
                    ));
            }
            let local_page = atlas.overflow_pages.len() - 1;
            let page = &mut atlas.overflow_pages[local_page];
            let slot = page
                .free_slots
                .pop()
                .expect("new overflow map page has a free slot");
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
    }

    pub fn remove_map_texture(&mut self, map_id: i32) {
        if let Some(atlas) = &mut self.item_frame_map_atlas {
            remove_resident_map_texture(atlas, map_id);
            atlas.overflow_locations.remove(&map_id);
            atlas.layout.rects.remove(&map_id);
        }
        self.refresh_map_texture_counters();
    }

    pub fn invalidate_map_texture_residency(&mut self) {
        if self.item_frame_map_atlas.is_some() {
            self.counters.map_full_residency_replacements = self
                .counters
                .map_full_residency_replacements
                .saturating_add(1);
        }
        self.item_frame_map_atlas = None;
        self.first_person_map_background_atlas = None;
        self.item_frame_map_decoration_atlas = None;
        self.map_surface_residency.clear();
        self.refresh_map_texture_counters();
        self.refresh_map_instance_counters();
    }

    fn refresh_map_texture_counters(&mut self) {
        let bytes = self.item_frame_map_atlas.as_ref().map_or(0, |atlas| {
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
        self.counters.map_instance_upload_bytes = self.map_surface_residency.upload_bytes();
        self.counters.map_surface_instances = self.map_surface_residency.instance_count();
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
        if self.item_frame_map_decoration_atlas.is_none() {
            self.item_frame_map_decoration_atlas = build_item_frame_map_decoration_atlas(
                item_frame_textures
                    .iter()
                    .chain(first_person_textures.iter())
                    .chain(gui_textures.iter()),
            )
            .map(|(layout, rgba)| {
                create_item_frame_map_decoration_atlas_gpu(
                    &self.device,
                    &self.queue,
                    &self.terrain_bind_group_layout,
                    &self.camera_buffer,
                    &self.hud3d_camera_buffer,
                    &self.gui_item_camera_buffer,
                    layout,
                    &rgba,
                )
            });
        }
        let upload = self.map_surface_residency.set_decorations(
            &self.device,
            &self.queue,
            self.item_frame_map_decoration_atlas.as_ref(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }

    /// Uploads the persistent vanilla ASCII font atlas for map label text. Native uses the same glyph
    /// metrics to emit narrow glyph instances; no world-space glyph mesh is retained in production.
    pub fn upload_item_frame_map_text_font(&mut self, width: u32, height: u32, rgba: &[u8]) {
        self.item_frame_map_text_font_atlas = create_item_frame_map_text_font_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.hud3d_camera_buffer,
            &self.gui_item_camera_buffer,
            width,
            height,
            rgba,
        );
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
        let upload = self.map_surface_residency.set_text(
            &self.device,
            &self.queue,
            self.item_frame_map_text_font_atlas.is_some(),
            item_frame_surfaces,
            first_person_surfaces,
            gui_surfaces,
        );
        self.record_map_instance_upload(upload);
    }
}

fn build_first_person_map_background_atlas(
    textures: &[FirstPersonMapBackgroundTexture],
) -> Option<(FirstPersonMapBackgroundAtlasLayout, Vec<u8>)> {
    let mut by_kind: BTreeMap<FirstPersonMapBackgroundKind, &FirstPersonMapBackgroundTexture> =
        BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_kind.insert(texture.kind, texture);
        }
    }
    if by_kind.is_empty() {
        return None;
    }
    let width = by_kind.values().map(|texture| texture.width).max()?;
    let height = by_kind
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))?;
    let atlas_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok())?;
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (kind, texture) in by_kind {
        for y in 0..texture.height {
            let src = usize::try_from((y * texture.width) * 4).ok()?;
            let dst = usize::try_from(((y_offset + y) * width) * 4).ok()?;
            let row_len = usize::try_from(texture.width * 4).ok()?;
            atlas_rgba[dst..dst + row_len].copy_from_slice(&texture.rgba[src..src + row_len]);
        }
        rects.insert(
            kind,
            ItemFrameMapUvRect {
                min: [0.0, y_offset as f32 / height as f32],
                max: [
                    texture.width as f32 / width as f32,
                    (y_offset + texture.height) as f32 / height as f32,
                ],
            },
        );
        y_offset += texture.height;
    }
    Some((
        FirstPersonMapBackgroundAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn create_first_person_map_background_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    layout: FirstPersonMapBackgroundAtlasLayout,
    rgba: &[u8],
) -> FirstPersonMapBackgroundAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-first-person-map-background-atlas-texture"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(layout.width * 4),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-first-person-map-background-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-first-person-map-background-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    FirstPersonMapBackgroundAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    }
}

#[cfg(test)]
fn build_item_frame_map_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapTexture>,
) -> Option<(ItemFrameMapAtlasLayout, Vec<u8>)> {
    let mut by_id: BTreeMap<i32, &[u8]> = BTreeMap::new();
    for texture in textures {
        if texture.rgba.len() == ITEM_FRAME_MAP_RGBA_LEN {
            by_id.insert(texture.map_id, &texture.rgba);
        }
    }
    let map_count = u32::try_from(by_id.len()).ok()?;
    if map_count == 0 {
        return None;
    }
    let width = ITEM_FRAME_MAP_SIZE;
    let height = ITEM_FRAME_MAP_SIZE.checked_mul(map_count)?;
    let mut atlas_rgba = vec![0; width as usize * height as usize * 4];
    let mut rects = BTreeMap::new();
    let per_map_len = ITEM_FRAME_MAP_RGBA_LEN;
    for (slot, (map_id, rgba)) in by_id.into_iter().enumerate() {
        let dst = slot * per_map_len;
        atlas_rgba[dst..dst + per_map_len].copy_from_slice(rgba);
        let v0 = slot as f32 / map_count as f32;
        let v1 = (slot as f32 + 1.0) / map_count as f32;
        rects.insert(
            map_id,
            ItemFrameMapUvRect {
                min: [0.0, v0],
                max: [1.0, v1],
            },
        );
    }
    Some((
        ItemFrameMapAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn map_texture_slot_origin(slot: u32, side_slots: u32) -> wgpu::Origin3d {
    wgpu::Origin3d {
        x: (slot % side_slots) * ITEM_FRAME_MAP_SIZE,
        y: (slot / side_slots) * ITEM_FRAME_MAP_SIZE,
        z: 0,
    }
}

fn map_texture_slot_rect(slot: u32, side_slots: u32) -> ItemFrameMapUvRect {
    let atlas_size = (side_slots * ITEM_FRAME_MAP_SIZE) as f32;
    let origin = map_texture_slot_origin(slot, side_slots);
    ItemFrameMapUvRect {
        min: [
            (origin.x as f32 + 0.5) / atlas_size,
            (origin.y as f32 + 0.5) / atlas_size,
        ],
        max: [
            (origin.x as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
            (origin.y as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
        ],
    }
}

fn create_item_frame_map_atlas_gpu() -> ItemFrameMapAtlasGpu {
    let atlas_size = MAP_ATLAS_PAGE_SIDE_SLOTS * ITEM_FRAME_MAP_SIZE;
    ItemFrameMapAtlasGpu {
        pages: Vec::new(),
        overflow_pages: Vec::new(),
        overflow_locations: BTreeMap::new(),
        layout: ItemFrameMapAtlasLayout {
            width: atlas_size,
            height: atlas_size,
            rects: BTreeMap::new(),
        },
        owners: BTreeMap::new(),
        frame_index: 0,
        max_resident_textures: MAX_RESIDENT_MAP_TEXTURES,
    }
}

fn create_item_frame_map_atlas_page_gpu(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    hud3d_camera_buffer: &wgpu::Buffer,
    gui_camera_buffer: &wgpu::Buffer,
) -> ItemFrameMapAtlasPageGpu {
    let atlas_size = MAP_ATLAS_PAGE_SIDE_SLOTS * ITEM_FRAME_MAP_SIZE;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-atlas-texture"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let hud3d_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-atlas-hud3d-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: hud3d_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let gui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-atlas-gui-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: gui_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    ItemFrameMapAtlasPageGpu {
        texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        free_slots: (0..MAP_ATLAS_PAGE_SIDE_SLOTS * MAP_ATLAS_PAGE_SIDE_SLOTS)
            .rev()
            .collect(),
    }
}

fn allocate_resident_map_slot(
    atlas: &mut ItemFrameMapAtlasGpu,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    hud3d_camera_buffer: &wgpu::Buffer,
    gui_camera_buffer: &wgpu::Buffer,
) -> Option<(usize, u32)> {
    if let Some((page_index, slot)) = atlas
        .pages
        .iter_mut()
        .enumerate()
        .find_map(|(page, owner)| owner.free_slots.pop().map(|slot| (page, slot)))
    {
        return Some((page_index, slot));
    }
    if atlas.pages.len() >= MAX_RESIDENT_MAP_PAGES {
        return None;
    }
    atlas.pages.push(create_item_frame_map_atlas_page_gpu(
        device,
        bind_group_layout,
        camera_buffer,
        hud3d_camera_buffer,
        gui_camera_buffer,
    ));
    let page_index = atlas.pages.len() - 1;
    let slot = atlas.pages[page_index]
        .free_slots
        .pop()
        .expect("new resident map page has a free slot");
    Some((page_index, slot))
}

fn write_full_map_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    owner: &ResidentMapTexture,
) {
    write_full_map_pixels(queue, texture, owner.slot, &owner.rgba);
}

fn write_full_map_pixels(queue: &wgpu::Queue, texture: &wgpu::Texture, slot: u32, rgba: &[u8]) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: map_texture_slot_origin(slot, MAP_ATLAS_PAGE_SIDE_SLOTS),
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(ITEM_FRAME_MAP_SIZE * 4),
            rows_per_image: Some(ITEM_FRAME_MAP_SIZE),
        },
        wgpu::Extent3d {
            width: ITEM_FRAME_MAP_SIZE,
            height: ITEM_FRAME_MAP_SIZE,
            depth_or_array_layers: 1,
        },
    );
}

fn write_map_texture_patch(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    side_slots: u32,
    slot: u32,
    patch: &ItemFrameMapTexturePatch,
) -> bool {
    let width = u32::from(patch.width);
    let height = u32::from(patch.height);
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok());
    if width == 0 || height == 0 || expected != Some(patch.rgba.len()) {
        return false;
    }
    let mut origin = map_texture_slot_origin(slot, side_slots);
    origin.x += u32::from(patch.start_x);
    origin.y += u32::from(patch.start_y);
    if u32::from(patch.start_x) + width > ITEM_FRAME_MAP_SIZE
        || u32::from(patch.start_y) + height > ITEM_FRAME_MAP_SIZE
    {
        return false;
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        &patch.rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    true
}

fn insert_resident_map_texture(atlas: &mut ItemFrameMapAtlasGpu, owner: ResidentMapTexture) {
    let map_id = owner.map_id;
    atlas.layout.rects.insert(
        map_id,
        map_texture_slot_rect(owner.slot, MAP_ATLAS_PAGE_SIDE_SLOTS),
    );
    atlas.owners.insert(map_id, owner);
}

fn remove_resident_map_texture(atlas: &mut ItemFrameMapAtlasGpu, map_id: i32) {
    if let Some(owner) = atlas.owners.remove(&map_id) {
        atlas.pages[owner.page].free_slots.push(owner.slot);
        atlas.layout.rects.remove(&map_id);
    }
}

fn build_item_frame_map_decoration_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapDecorationTexture>,
) -> Option<(ItemFrameMapDecorationAtlasLayout, Vec<u8>)> {
    let mut by_id: BTreeMap<&str, &ItemFrameMapDecorationTexture> = BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_id.insert(texture.sprite_id.as_str(), texture);
        }
    }
    if by_id.is_empty() {
        return None;
    }
    let width = by_id.values().map(|texture| texture.width).max()?;
    let height = by_id
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))?;
    let atlas_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok())?;
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (sprite_id, texture) in by_id {
        for y in 0..texture.height {
            let src = usize::try_from((y * texture.width) * 4).ok()?;
            let dst = usize::try_from(((y_offset + y) * width) * 4).ok()?;
            let row_len = usize::try_from(texture.width * 4).ok()?;
            atlas_rgba[dst..dst + row_len].copy_from_slice(&texture.rgba[src..src + row_len]);
        }
        rects.insert(
            sprite_id.to_string(),
            ItemFrameMapUvRect {
                min: [0.0, y_offset as f32 / height as f32],
                max: [
                    texture.width as f32 / width as f32,
                    (y_offset + texture.height) as f32 / height as f32,
                ],
            },
        );
        y_offset += texture.height;
    }
    Some((
        ItemFrameMapDecorationAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn create_item_frame_map_decoration_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    hud3d_camera_buffer: &wgpu::Buffer,
    gui_camera_buffer: &wgpu::Buffer,
    layout: ItemFrameMapDecorationAtlasLayout,
    rgba: &[u8],
) -> ItemFrameMapDecorationAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-texture"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(layout.width * 4),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let hud3d_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-hud3d-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: hud3d_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let gui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-gui-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: gui_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    ItemFrameMapDecorationAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        layout,
    }
}

fn create_item_frame_map_text_font_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    hud3d_camera_buffer: &wgpu::Buffer,
    gui_camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Option<ItemFrameMapTextFontAtlasGpu> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok());
    if width == 0 || height == 0 || expected_len != Some(rgba.len()) {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    // Vanilla's font provider exposes a dedicated opaque white `effect()`
    // glyph for underline/strikethrough rectangles. Keep the equivalent 1x1
    // material beside the shared font atlas instead of assuming a resource
    // glyph contains an opaque white texel.
    let effect_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-text-effect-texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &effect_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 255, 255, 255],
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let effect_view = effect_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-text-effect-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&effect_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let hud3d_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-hud3d-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: hud3d_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let gui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-gui-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: gui_camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    Some(ItemFrameMapTextFontAtlasGpu {
        _texture: texture,
        _view: view,
        _effect_texture: effect_texture,
        _effect_view: effect_view,
        _sampler: sampler,
        bind_group,
        effect_bind_group,
        hud3d_bind_group,
        gui_bind_group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT;

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
    fn item_frame_map_surface_uses_dynamic_map_texture_submission() {
        let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [13.0 / 15.0, 1.0];
        let surface = bake_item_frame_map_surface(10, transform, light);

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.map_id, 10);
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:map/10"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (0, 0)
        );

        let first_map_rgba = vec![10; ITEM_FRAME_MAP_RGBA_LEN];
        let second_map_rgba = vec![20; ITEM_FRAME_MAP_RGBA_LEN];
        let (atlas, rgba) = build_item_frame_map_atlas(&[
            map_texture(10, second_map_rgba),
            map_texture(5, first_map_rgba),
            map_texture(99, vec![1, 2, 3]),
        ])
        .expect("valid map atlas");

        // The transient atlas is deterministic by map id and ignores malformed RGBA payloads.
        assert_eq!(atlas.width, ITEM_FRAME_MAP_SIZE);
        assert_eq!(atlas.height, ITEM_FRAME_MAP_SIZE * 2);
        assert_eq!(&rgba[..4], &[10, 10, 10, 10]);
        assert_eq!(
            &rgba[ITEM_FRAME_MAP_RGBA_LEN..ITEM_FRAME_MAP_RGBA_LEN + 4],
            &[20, 20, 20, 20]
        );
        assert!(!atlas.rects.contains_key(&99));

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].uv, [0.0, 1.0]);
        assert_eq!(vertices[2].uv, [1.0, 0.5]);
        assert!(vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
    }

    #[test]
    fn item_frame_map_surface_waits_when_dynamic_map_texture_is_absent() {
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        let (atlas, _) =
            build_item_frame_map_atlas(&[map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN])])
                .expect("valid map atlas");

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

        assert!(
            vertices.is_empty() && indices.is_empty(),
            "absent dynamic map texture must not fold stale map geometry"
        );
    }

    #[test]
    fn item_frame_map_surface_waits_when_dynamic_map_texture_is_malformed() {
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        let (atlas, _) = build_item_frame_map_atlas(&[
            map_texture(10, vec![1, 2, 3]),
            map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN]),
        ])
        .expect("valid atlas from the unrelated map");

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

        assert!(
            vertices.is_empty() && indices.is_empty(),
            "malformed dynamic map texture must not fold stale map geometry"
        );
    }

    #[test]
    fn persistent_map_owner_reuses_unchanged_pixels_and_uploads_only_one_patch() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, [1.0; 2]);
        let second_surface = bake_item_frame_map_surface(11, Mat4::IDENTITY, [1.0; 2]);
        let first_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
        let second_rgba = vec![9; ITEM_FRAME_MAP_RGBA_LEN];
        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(10, 7, 0, first_rgba.clone(), None),
                map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
            ],
            Vec::new(),
            vec![surface.clone(), second_surface.clone()],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 131_072);
        assert_eq!(renderer.counters().map_textures_created, 2);
        assert_eq!(renderer.counters().map_texture_resident_bytes, 131_072);
        assert_eq!(renderer.counters().map_surface_instances, 2);
        assert!(renderer.counters().map_instance_upload_bytes > 0);
        renderer
            .render(None)
            .expect("resident map instances render through the static quad pipeline");
        let slot = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&10)
            .unwrap()
            .slot;
        let second_owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&11)
            .unwrap();
        let second_slot = second_owner.slot;
        let second_owner_rgba = Arc::clone(&second_owner.rgba);

        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(10, 7, 0, first_rgba, None),
                map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
            ],
            Vec::new(),
            vec![surface.clone(), second_surface.clone()],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 0);
        assert_eq!(renderer.counters().map_textures_reused, 2);

        let pixel = [11, 22, 33, 255];
        let mut updated_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
        let offset = (2 + 3 * ITEM_FRAME_MAP_SIZE as usize) * 4;
        updated_rgba[offset..offset + 4].copy_from_slice(&pixel);
        let replacements = renderer.counters().item_full_residency_replacements;
        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(
                    10,
                    7,
                    1,
                    updated_rgba,
                    Some(ItemFrameMapTexturePatch {
                        start_x: 2,
                        start_y: 3,
                        width: 1,
                        height: 1,
                        rgba: Arc::from(pixel),
                    }),
                ),
                map_texture_at_revision(11, 8, 0, second_rgba, None),
            ],
            Vec::new(),
            vec![surface, second_surface],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 4);
        assert_eq!(renderer.counters().map_textures_updated, 1);
        assert_eq!(
            renderer.counters().item_full_residency_replacements,
            replacements
        );
        let owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&10)
            .unwrap();
        assert_eq!(owner.slot, slot);
        assert_eq!(owner.content_revision, 1);
        let second_owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&11)
            .unwrap();
        assert_eq!(second_owner.slot, second_slot);
        assert_eq!(second_owner.content_revision, 0);
        assert!(Arc::ptr_eq(&second_owner.rgba, &second_owner_rgba));
    }

    #[test]
    fn map_owner_capacity_falls_back_to_current_frame_page_without_hiding_surface() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer.set_map_texture_residency_limit_for_test(1);
        renderer.set_map_surfaces(
            vec![
                map_texture(10, vec![1; ITEM_FRAME_MAP_RGBA_LEN]),
                map_texture(11, vec![2; ITEM_FRAME_MAP_RGBA_LEN]),
            ],
            Vec::new(),
            vec![
                bake_item_frame_map_surface(10, Mat4::IDENTITY, [1.0; 2]),
                bake_item_frame_map_surface(11, Mat4::IDENTITY, [1.0; 2]),
            ],
            Vec::new(),
        );

        let atlas = renderer.item_frame_map_atlas.as_ref().unwrap();
        assert_eq!(atlas.owners.len(), 1);
        assert_eq!(atlas.overflow_locations.len(), 1);
        assert_eq!(renderer.map_surface_residency.dynamic_draws.len(), 2);
        assert_eq!(renderer.counters().map_surface_instances, 2);
        assert_eq!(renderer.counters().map_texture_fallbacks, 1);
        assert_eq!(renderer.counters().item_map_upload_bytes, 131_072);
        assert_eq!(renderer.counters().map_full_residency_replacements, 0);
    }

    #[test]
    fn adding_second_map_page_uploads_only_new_owner_without_replacing_first_page() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let first_page = (0..MAP_ATLAS_PAGE_CAPACITY as i32)
            .map(|map_id| map_texture(map_id, vec![map_id as u8; ITEM_FRAME_MAP_RGBA_LEN]))
            .collect::<Vec<_>>();
        renderer.set_map_surfaces(first_page.clone(), Vec::new(), Vec::new(), Vec::new());
        assert_eq!(
            renderer.item_frame_map_atlas.as_ref().unwrap().pages.len(),
            1
        );
        assert_eq!(renderer.counters().map_full_residency_replacements, 0);

        let mut second_page = first_page;
        second_page.push(map_texture(64, vec![64; ITEM_FRAME_MAP_RGBA_LEN]));
        renderer.set_map_surfaces(second_page, Vec::new(), Vec::new(), Vec::new());
        let atlas = renderer.item_frame_map_atlas.as_ref().unwrap();
        assert_eq!(atlas.pages.len(), 2);
        assert_eq!(atlas.owners.len(), MAP_ATLAS_PAGE_CAPACITY + 1);
        assert_eq!(renderer.counters().item_map_upload_bytes, 65_536);
        assert_eq!(renderer.counters().map_full_residency_replacements, 0);
    }

    #[test]
    fn map_owner_remove_and_resource_invalidation_drop_old_handles() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_map_surfaces(
            vec![map_texture_at_revision(
                3,
                9,
                2,
                vec![1; ITEM_FRAME_MAP_RGBA_LEN],
                None,
            )],
            Vec::new(),
            vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
            Vec::new(),
        );
        renderer.set_map_surfaces(
            vec![map_texture_at_revision(
                3,
                9,
                1,
                vec![2; ITEM_FRAME_MAP_RGBA_LEN],
                None,
            )],
            Vec::new(),
            vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 0);
        assert_eq!(renderer.counters().map_textures_stale, 1);
        assert_eq!(
            renderer
                .item_frame_map_atlas
                .as_ref()
                .unwrap()
                .owners
                .get(&3)
                .unwrap()
                .content_revision,
            2
        );
        renderer.remove_map_texture(3);
        let atlas = renderer.item_frame_map_atlas.as_ref().unwrap();
        assert!(!atlas.owners.contains_key(&3));
        assert!(!atlas.layout.rects.contains_key(&3));

        renderer.invalidate_map_texture_residency();
        assert!(renderer.item_frame_map_atlas.is_none());
        assert!(renderer.first_person_map_background_atlas.is_none());
        assert!(renderer.item_frame_map_decoration_atlas.is_none());
    }

    #[test]
    fn first_person_map_background_surface_uses_vanilla_render_map_geometry() {
        let light = ITEM_MODEL_FULL_BRIGHT_LIGHT;
        let surface = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            light,
        );

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(
            surface.submission.kind,
            FirstPersonMapBackgroundKind::Checkerboard
        );
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:textures/map/map_background_checkerboard.png"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            surface
                .mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            vec![
                [-7.0, 135.0, 0.0],
                [135.0, 135.0, 0.0],
                [135.0, -7.0, 0.0],
                [-7.0, -7.0, 0.0],
            ]
        );
    }

    #[test]
    fn first_person_map_background_atlas_filters_and_remaps_uvs() {
        let (atlas, rgba) = build_first_person_map_background_atlas(&[
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Checkerboard,
                width: 1,
                height: 1,
                rgba: vec![40, 41, 42, 255],
            },
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Plain,
                width: 2,
                height: 1,
                rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
            },
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Plain,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3],
            },
        ])
        .expect("valid map background atlas");

        assert_eq!(atlas.width, 2);
        assert_eq!(atlas.height, 2);
        assert_eq!(
            rgba,
            vec![10, 11, 12, 255, 13, 14, 15, 255, 40, 41, 42, 255, 0, 0, 0, 0,]
        );
        assert_eq!(atlas.rects.len(), 2);
        let surface = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
        );
        let (vertices, indices) = merge_first_person_map_background_surfaces(&[surface], &atlas);
        assert_eq!(vertices.len(), 4);
        assert_eq!(indices.len(), 6);
        assert_eq!(vertices[0].uv, [0.0, 1.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.5]);
    }

    #[test]
    fn resource_map_atlases_persist_when_later_frames_carry_only_instances() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let background = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            [1.0; 2],
        );
        renderer.set_first_person_map_background_surfaces(
            vec![FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Checkerboard,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            }],
            vec![background.clone()],
        );
        renderer.set_first_person_map_background_surfaces(Vec::new(), vec![background]);
        assert!(renderer.first_person_map_background_atlas.is_some());
        assert_eq!(renderer.first_person_map_background_surfaces.len(), 1);

        let decoration =
            bake_first_person_map_decoration_surface(1, 0, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0)
                .unwrap();
        let sprite_id = decoration.submission.texture.sprite_id.to_string();
        renderer.set_map_decoration_surfaces(
            Vec::new(),
            vec![ItemFrameMapDecorationTexture {
                sprite_id,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            }],
            Vec::new(),
            vec![decoration.clone()],
        );
        renderer.set_map_decoration_surfaces(Vec::new(), Vec::new(), Vec::new(), vec![decoration]);
        assert!(renderer.item_frame_map_decoration_atlas.is_some());
        assert_eq!(renderer.first_person_map_decoration_surfaces.len(), 1);
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

    #[test]
    fn item_frame_map_decoration_surface_uses_vanilla_text_submission() {
        let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [7.0 / 15.0, 12.0 / 15.0];
        let surface =
            bake_item_frame_map_decoration_surface(1, -20, 30, 7, 2, map_transform, light, 3)
                .expect("frame decorations render on item-frame maps");

        let expected_transform = map_transform
            * Mat4::from_translation(Vec3::new(-20.0 / 2.0 + 64.0, 30.0 / 2.0 + 64.0, -0.02))
            * Mat4::from_rotation_z((7.0_f32 * 360.0 / 16.0).to_radians())
            * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
            * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0));

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.type_id, 1);
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_atlas_path(),
            "minecraft:textures/atlas/map_decorations.png"
        );
        assert_eq!(
            surface.submission.texture.vanilla_sprite_id(),
            "minecraft:frame"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, expected_transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (0, 3)
        );
        assert_eq!(surface.submission.decoration_index, 2);
        assert_eq!(
            surface.mesh.vertices[0].position,
            expected_transform
                .transform_point3(Vec3::new(-1.0, 1.0, -0.002))
                .to_array()
        );
        assert!(surface
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
        assert!(
            bake_item_frame_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1)
                .is_none(),
            "player marker has showOnItemFrame=false in vanilla"
        );
        let first_person =
            bake_first_person_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1)
                .expect("first-person maps render all MapRenderer decorations");
        assert_eq!(first_person.submission.type_id, 0);
        assert_eq!(
            first_person.submission.texture.vanilla_sprite_id(),
            "minecraft:player"
        );
    }

    #[test]
    fn item_frame_map_text_surface_uses_vanilla_order_one_text_submission() {
        let glyphs = test_map_text_glyphs();
        let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [7.0 / 15.0, 12.0 / 15.0];
        let surface = bake_item_frame_map_text_surface(
            1,
            "Frame",
            -20,
            30,
            0,
            map_transform,
            light,
            0,
            &glyphs,
        )
        .expect("frame-visible decoration name text");

        let width = 30.0;
        let scale = 6.0 / 9.0;
        let expected_transform = map_transform
            * Mat4::from_translation(Vec3::new(
                -20.0 / 2.0 + 64.0 - width * scale / 2.0,
                30.0 / 2.0 + 64.0 + 4.0,
                -0.025,
            ))
            * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));

        assert_eq!(surface.vertex_count(), 20);
        assert_eq!(surface.index_count(), 30);
        assert_eq!(surface.submission.type_id, 1);
        assert_eq!(surface.submission.text, "Frame");
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:font/default"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, expected_transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (1, 0)
        );
        assert_eq!(surface.submission.decoration_index, 0);
        assert_eq!(surface.submission.width, width);
        assert_eq!(surface.submission.scale, scale);
        assert_eq!(
            surface.mesh.vertices[0].position,
            expected_transform
                .transform_point3(Vec3::new(0.0, 0.0, 0.0))
                .to_array()
        );
        assert!(surface
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
        let unit_positions = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let unit_uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        for (glyph, vertices) in surface
            .glyphs
            .iter()
            .zip(surface.mesh.vertices.chunks_exact(4))
        {
            for ((position, uv), legacy) in unit_positions.iter().zip(unit_uvs).zip(vertices) {
                let position = glyph.transform.transform_point3(*position).to_array();
                let uv = glyph.uv.map(uv);
                assert!(position
                    .iter()
                    .zip(legacy.position)
                    .all(|(actual, expected)| (*actual - expected).abs() < 1.0e-5));
                assert!(uv
                    .iter()
                    .zip(legacy.uv)
                    .all(|(actual, expected)| (*actual - expected).abs() < 1.0e-6));
            }
        }

        assert!(
            bake_item_frame_map_text_surface(
                0,
                "Hidden",
                0,
                0,
                0,
                map_transform,
                light,
                0,
                &glyphs
            )
            .is_none(),
            "player marker has showOnItemFrame=false in vanilla"
        );
        let first_person = bake_first_person_map_text_surface(
            0,
            "Player",
            0,
            0,
            0,
            map_transform,
            light,
            0,
            &glyphs,
        )
        .expect("first-person maps render text for non-frame decorations");
        assert_eq!(first_person.submission.type_id, 0);
        assert_eq!(first_person.submission.text, "Player");
    }

    #[test]
    fn map_text_uses_resident_quad_and_narrow_glyph_instances() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer.upload_item_frame_map_text_font(1, 1, &[255, 255, 255, 255]);
        let surface = bake_item_frame_map_text_surface(
            1,
            "Frame",
            0,
            0,
            0,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            0,
            &test_map_text_glyphs(),
        )
        .expect("map text");
        let glyph_count = surface.glyphs.len();
        renderer.set_map_text_surfaces(vec![surface], Vec::new());

        assert_eq!(
            renderer.map_surface_residency.text_instances.len(),
            glyph_count
        );
        assert_eq!(renderer.map_surface_residency.text_draws.len(), 1);
        assert_eq!(
            renderer.map_surface_residency.text_draws[0].target,
            MapSurfaceTarget::World
        );
        assert_eq!(renderer.counters().map_surface_instances, glyph_count);
        assert_eq!(
            renderer.counters().map_instance_upload_bytes,
            (glyph_count * std::mem::size_of::<MapSurfaceGpuInstance>()) as u64
        );
        renderer.update_camera();
        renderer.render_offscreen_frame().unwrap();
        assert_eq!(renderer.counters().item_transient_buffer_creations, 0);
    }

    #[test]
    fn item_frame_map_text_width_uses_glyph_map_replacement_fallback() {
        let mut glyphs = test_map_text_glyphs();
        glyphs.insert_first_wins(
            'é',
            HudAsciiGlyph {
                width: 6,
                height: 12,
                advance: 7,
                ascent: 10,
                ..HudAsciiGlyph::default()
            },
        );
        assert_eq!(item_frame_map_text_width("A A", &glyphs), Some(16));
        // Mapped accented codepoints resolve their own glyph now.
        assert_eq!(item_frame_map_text_width("é", &glyphs), Some(7));
        // This fixture deliberately has no unihex provider, so CJK is `?`.
        assert_eq!(item_frame_map_text_width("钻", &glyphs), Some(5));
        assert_eq!(item_frame_map_text_width("", &glyphs), None);
    }

    #[test]
    fn map_text_positions_from_logical_width_and_draws_visual_glyph_order() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, min_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4), ('?', 0.5)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: crate::HudUvRect {
                        min: [min_x, 0.0],
                        max: [min_x + 0.05, 0.5],
                    },
                    width: 5,
                    height: 8,
                    advance: 6,
                    ascent: 7,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs.set_default_right_to_left(true);

        let surface = bake_item_frame_map_text_surface(
            1,
            "123-",
            0,
            0,
            0,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            0,
            &glyphs,
        )
        .unwrap();
        assert_eq!(surface.submission.text, "123-");
        assert_eq!(surface.submission.width, 24.0);
        let glyph_uvs = surface
            .glyphs
            .iter()
            .map(|glyph| glyph.uv.min[0])
            .collect::<Vec<_>>();
        assert_eq!(glyph_uvs, [0.4, 0.1, 0.2, 0.3]);
    }

    #[test]
    fn item_frame_map_decoration_atlas_remaps_sprite_uvs() {
        let (atlas, rgba) = build_item_frame_map_decoration_atlas(&[
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:target_x".to_string(),
                width: 1,
                height: 1,
                rgba: vec![20, 21, 22, 255],
            },
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:frame".to_string(),
                width: 2,
                height: 1,
                rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
            },
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:bad".to_string(),
                width: 2,
                height: 2,
                rgba: vec![1, 2, 3],
            },
        ])
        .expect("valid map decoration atlas");

        assert_eq!(atlas.width, 2);
        assert_eq!(atlas.height, 2);
        assert_eq!(&rgba[0..8], &[10, 11, 12, 255, 13, 14, 15, 255]);
        assert_eq!(&rgba[8..16], &[20, 21, 22, 255, 0, 0, 0, 0]);
        assert!(!atlas.rects.contains_key("minecraft:bad"));

        let surface = bake_item_frame_map_decoration_surface(
            1,
            0,
            0,
            0,
            0,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            1,
        )
        .expect("frame decoration surface");
        let (vertices, indices) = merge_item_frame_map_decoration_surfaces(&[surface], &atlas);
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 0.5]);
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
                        b' ' => 4,
                        b'?' => 5,
                        _ => 6,
                    },
                    ascent: 7,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }
}
