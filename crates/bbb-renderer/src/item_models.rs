//! 3D block-model / item-model rendering: baking parsed cuboid models (and extruded flat items) into a
//! mesh of textured quads, drawn standalone with a model transform.
//!
//! Mirrors the entity-model split: the renderer owns the mesh assembly + GPU pipeline, while the native
//! layer (which holds the parsed `bbb-pack` models + the block/item texture atlas) produces the
//! atlas-resolved [`ItemModelQuad`]s. A quad's `corners` are in vanilla model space (the `0..=16` box
//! coordinates, the same units `from`/`to` use), normalized to the `0..1` unit cube at bake time so the
//! caller's `transform` places the model in world / GUI / hand space exactly like vanilla's display
//! transforms. `uvs` are atlas-absolute into the shared block/item atlas. `tint` is the per-face color
//! (biome/dye tint, or white), `shade` is the directional face-shade multiplier (vanilla
//! `Direction.getShade` with ambient occlusion off), and `light` is the packed block/sky light projected
//! to shader-space. The baked vertex color is `tint × shade`; the shader applies light.

use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::{gpu::DEPTH_FORMAT, Renderer};

/// Vanilla model space is `0..=16`; the unit cube is that divided by 16.
const MODEL_SPACE_SCALE: f32 = 1.0 / 16.0;
const ITEM_FRAME_MAP_SIZE: u32 = 128;
const ITEM_FRAME_MAP_RGBA_LEN: usize =
    ITEM_FRAME_MAP_SIZE as usize * ITEM_FRAME_MAP_SIZE as usize * 4;

/// Shader-space full-bright light: block 15 and sky 15. Existing generic item-model consumers use this
/// unless they explicitly carry vanilla `lightCoords` from an entity renderer.
pub const ITEM_MODEL_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];

/// One textured quad of a baked block/item model: four corners wound counter-clockwise (front face),
/// in vanilla `0..=16` model space, with atlas-absolute UVs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelQuad {
    pub corners: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    /// Per-face tint (biome/dye/potion color, or white when untinted). Multiplied into the vertex color.
    pub tint: [f32; 4],
    /// Directional face-shade multiplier (vanilla `Direction.getShade`, AO off). `1.0` = unshaded.
    pub shade: f32,
}

/// A hotbar slot's 3D block item: the block model's quads (atlas-absolute UVs over the blocks atlas, in
/// `0..=16` model space) plus its resolved `gui` display transform. The renderer seats it in the slot's
/// pixel rect and draws it under the GUI ortho camera (vanilla 3D inventory icon).
#[derive(Debug, Clone, PartialEq)]
pub struct HudBlockItemModel {
    pub quads: Vec<ItemModelQuad>,
    pub gui_display: Mat4,
}

/// A baked block/item model vertex: the model-space position normalized to the unit cube and pushed
/// through the caller's `transform`, the atlas-absolute UV, the `tint × shade` color, and shader-space
/// block/sky light.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemModelVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) light: [f32; 2],
}

/// A baked block/item model mesh: an indexed triangle list ready for the item-model pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelMesh {
    pub(crate) vertices: Vec<ItemModelVertex>,
    pub(crate) indices: Vec<u32>,
}

impl ItemModelMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Appends `quads` at full brightness. Use [`append_quads_with_light`](Self::append_quads_with_light)
    /// when the caller carries vanilla entity-renderer light coords.
    pub fn append_quads(&mut self, quads: &[ItemModelQuad], transform: Mat4) {
        self.append_quads_with_light(quads, transform, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    }

    /// Appends `quads` to the mesh, normalizing each corner from vanilla `0..=16` model space to the unit
    /// cube and applying `transform` (the model→target-space matrix: world placement, GUI projection, or
    /// the hand attach transform). Each quad becomes two triangles wound from its four corners; the
    /// vertex color is the quad's `tint` scaled by its directional `shade` (alpha preserved), and every
    /// vertex carries the caller-provided shader-space block/sky light.
    pub fn append_quads_with_light(
        &mut self,
        quads: &[ItemModelQuad],
        transform: Mat4,
        light: [f32; 2],
    ) {
        for quad in quads {
            let base =
                u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
            let [tr, tg, tb, ta] = quad.tint;
            let color = [tr * quad.shade, tg * quad.shade, tb * quad.shade, ta];
            for (corner, uv) in quad.corners.iter().zip(quad.uvs.iter()) {
                let local = Vec3::from_array(*corner) * MODEL_SPACE_SCALE;
                let position = transform.transform_point3(local).to_array();
                self.vertices.push(ItemModelVertex {
                    position,
                    uv: *uv,
                    color,
                    light,
                });
            }
            // Two triangles (0,1,2)+(0,2,3) over the CCW quad corners.
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn append_raw_textured_quad(
        &mut self,
        corners: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        color: [f32; 4],
        light: [f32; 2],
    ) {
        let base = u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
        for (position, uv) in corners.into_iter().zip(uvs.into_iter()) {
            self.vertices.push(ItemModelVertex {
                position,
                uv,
                color,
                light,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Decoded RGBA pixels for vanilla's dynamic `minecraft:map/<id>` texture. The renderer packs these
/// 128x128 textures into a per-frame map atlas and draws item-frame maps as textured quads, matching
/// `MapTextureManager.prepareMapTexture` plus `MapRenderer.render`'s base surface submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexture {
    pub map_id: i32,
    pub rgba: Vec<u8>,
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

/// The base map surface submit for an item frame. `mesh` is the single vanilla `MapRenderer` quad in
/// world space with local 0..1 UVs; the renderer remaps those UVs to the dynamic map atlas at draw time.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapSurface {
    pub submission: ItemFrameMapSubmission,
    mesh: ItemModelMesh,
}

impl ItemFrameMapSurface {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemFrameMapUvRect {
    min: [f32; 2],
    max: [f32; 2],
}

impl ItemFrameMapUvRect {
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
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: ItemFrameMapAtlasLayout,
}

/// Bakes vanilla `MapRenderer.render`'s base map surface submit:
/// `RenderTypes.text(minecraft:map/<id>)`, white tint, order 0 / sequence 0, and the four
/// `(0,128,-0.01)..(0,0,-0.01)` vertices with UVs 0..1.
pub fn bake_item_frame_map_surface(
    map_id: i32,
    transform: Mat4,
    light: [f32; 2],
) -> ItemFrameMapSurface {
    let mut mesh = ItemModelMesh::new();
    let corners = [
        transform
            .transform_point3(Vec3::new(0.0, 128.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(128.0, 128.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(128.0, 0.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(0.0, 0.0, -0.01))
            .to_array(),
    ];
    mesh.append_raw_textured_quad(
        corners,
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        [1.0, 1.0, 1.0, 1.0],
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
        mesh,
    }
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`. Convenience over
/// [`ItemModelMesh::append_quads`] for the common one-model case.
pub fn bake_item_model_mesh(quads: &[ItemModelQuad], transform: Mat4) -> ItemModelMesh {
    bake_item_model_mesh_with_light(quads, transform, ITEM_MODEL_FULL_BRIGHT_LIGHT)
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`, carrying explicit shader-space
/// block/sky light.
pub fn bake_item_model_mesh_with_light(
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    mesh.append_quads_with_light(quads, transform, light);
    mesh
}

/// Concatenates several baked meshes into one vertex + index buffer, rebasing each mesh's indices onto
/// the running vertex count. The renderer uploads this once per frame and draws it indexed.
pub(crate) fn merge_item_model_meshes(
    meshes: &[ItemModelMesh],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for mesh in meshes {
        let base = u32::try_from(vertices.len()).expect("item-model vertex count fits in u32");
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

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

impl Renderer {
    /// Sets the baked **block-item** model meshes to draw this frame — those whose UVs are absolute into
    /// the blocks atlas (the same atlas terrain samples). Each mesh is already in world space with
    /// `tint × shade` vertex colors (the caller applies the world / display transform at bake time via
    /// [`ItemModelMesh::append_quads`]); the renderer concatenates and draws them indexed against the
    /// resident blocks atlas.
    pub fn set_block_item_model_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.block_item_model_meshes = meshes;
    }

    /// Sets the baked **flat / generated** item-model meshes to draw this frame — those whose UVs are
    /// absolute into the item atlas (the same atlas the dropped-item billboards sample). Drawn only when
    /// that atlas has been uploaded; otherwise skipped.
    pub fn set_flat_item_model_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.flat_item_model_meshes = meshes;
    }

    /// Sets this frame's filled-map item-frame submissions and their dynamic 128x128 map textures. The
    /// textures are packed into a transient atlas for the existing item-model shader; surfaces whose map
    /// texture is absent or malformed are skipped.
    pub fn set_item_frame_map_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapTexture>,
        surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.item_frame_map_atlas = build_item_frame_map_atlas(&textures).map(|(layout, rgba)| {
            create_item_frame_map_atlas_gpu(
                &self.device,
                &self.queue,
                &self.terrain_bind_group_layout,
                &self.camera_buffer,
                layout,
                &rgba,
            )
        });
        self.item_frame_map_surfaces = if let Some(atlas) = &self.item_frame_map_atlas {
            surfaces
                .into_iter()
                .filter(|surface| {
                    !surface.is_empty()
                        && atlas.layout.rects.contains_key(&surface.submission.map_id)
                })
                .collect()
        } else {
            Vec::new()
        };
    }

    /// Sets this frame's 3D block items for the hotbar slots (`None` for an empty slot or a flat item,
    /// which keeps its 2D sprite). Each is the block's model quads plus its `gui` display transform; the
    /// renderer seats them in their slot pixel rects and draws them in the GUI item pass (vanilla 3D
    /// inventory icons). Index `i` is hotbar slot `i`.
    pub fn set_hud_hotbar_block_item_models(&mut self, models: Vec<Option<HudBlockItemModel>>) {
        self.hud_hotbar_block_item_models = models;
    }

    /// Concatenates this frame's block-item meshes into one vertex + index buffer for upload.
    pub(crate) fn collect_block_item_model_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.block_item_model_meshes)
    }

    /// Concatenates this frame's flat-item meshes into one vertex + index buffer for upload.
    pub(crate) fn collect_flat_item_model_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.flat_item_model_meshes)
    }

    pub(crate) fn collect_item_frame_map_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.item_frame_map_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_item_frame_map_surfaces(&self.item_frame_map_surfaces, &atlas.layout)
    }
}

fn build_item_frame_map_atlas(
    textures: &[ItemFrameMapTexture],
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

fn create_item_frame_map_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    layout: ItemFrameMapAtlasLayout,
    rgba: &[u8],
) -> ItemFrameMapAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-atlas-texture"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
    ItemFrameMapAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    }
}

const ITEM_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2];

fn item_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_MODEL_VERTEX_ATTRIBUTES,
    }
}

/// Item-model shader: samples the shared block/item atlas (bound exactly like the terrain pass —
/// `view_proj` uniform `@0`, atlas texture `@1`, sampler `@2`), multiplies by the baked vertex color
/// (the per-face `tint × Direction.getShade`), then applies the same simple block/sky light factor used
/// by entity models. Alpha cutout: transparent texels are discarded, so the thin generated-item slab and
/// partial block faces read cleanly against the depth buffer.
const ITEM_MODEL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var item_atlas: texture_2d<f32>;

@group(0) @binding(2)
var item_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    out.light = input.light;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(item_atlas, item_sampler, input.uv) * input.color;
    if texel.a <= 0.01 {
        discard;
    }
    let light_level = max(input.light.x, input.light.y * 0.95);
    let shade = 0.16 + light_level * 0.84;
    return vec4<f32>(texel.rgb * shade, texel.a);
}
"#;

/// Builds the item-model render pipeline. Reuses the terrain camera+atlas bind-group layout (so it binds
/// the resident blocks atlas directly), renders solid (depth-tested and depth-writing, since item models
/// are real 3D geometry) and un-culled (the generated-item slab's faces are emitted without winding
/// canonicalization, so both sides must draw).
pub(crate) fn create_item_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-item-model-shader"),
        source: wgpu::ShaderSource::Wgsl(ITEM_MODEL_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-item-model-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-item-model-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[item_model_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_quad(shade: f32, tint: [f32; 4]) -> ItemModelQuad {
        // A full-face quad on the south side of a unit (0..=16) cube, atlas UVs 0..1.
        ItemModelQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            tint,
            shade,
        }
    }

    #[test]
    fn baking_a_quad_emits_two_triangles_normalized_to_the_unit_cube() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        // The `0..=16` corners are normalized to the `0..1` unit cube.
        assert_eq!(mesh.vertices[0].position, [0.0, 0.0, 1.0]);
        assert_eq!(mesh.vertices[2].position, [1.0, 1.0, 1.0]);
        assert_eq!(mesh.vertices[1].uv, [1.0, 1.0]);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.light == ITEM_MODEL_FULL_BRIGHT_LIGHT));
    }

    #[test]
    fn shade_scales_rgb_but_not_alpha() {
        let mesh = bake_item_model_mesh(&[unit_quad(0.6, [1.0, 0.5, 0.25, 1.0])], Mat4::IDENTITY);
        // Vanilla applies `Direction.getShade` to the RGB only; alpha stays put.
        assert_eq!(mesh.vertices[0].color, [0.6, 0.3, 0.15, 1.0]);
    }

    #[test]
    fn explicit_light_is_carried_by_every_vertex() {
        let light = [5.0 / 15.0, 9.0 / 15.0];
        let mesh = bake_item_model_mesh_with_light(
            &[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])],
            Mat4::IDENTITY,
            light,
        );
        assert!(mesh.vertices.iter().all(|vertex| vertex.light == light));
    }

    #[test]
    fn transform_places_the_model_in_target_space() {
        // A translation places the unit cube; the corner at unit (1,1,1) lands at the offset + 1.
        let transform = Mat4::from_translation(Vec3::new(10.0, 64.0, -5.0));
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], transform);
        assert_eq!(mesh.vertices[2].position, [11.0, 65.0, -4.0]);
        assert_eq!(mesh.vertices[0].position, [10.0, 64.0, -4.0]);
    }

    #[test]
    fn merging_meshes_rebases_indices_onto_the_running_vertex_count() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        let (vertices, indices) = merge_item_model_meshes(&[mesh.clone(), mesh]);
        assert_eq!(vertices.len(), 8);
        // The second mesh's indices are shifted past the first mesh's four vertices.
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn merging_no_meshes_is_empty() {
        let (vertices, indices) = merge_item_model_meshes(&[]);
        assert!(vertices.is_empty());
        assert!(indices.is_empty());
    }

    #[test]
    fn append_quads_rebases_indices_across_models() {
        let mut mesh = ItemModelMesh::new();
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 8);
        // The second quad's triangles are rebased onto its own vertices.
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
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
            ItemFrameMapTexture {
                map_id: 10,
                rgba: second_map_rgba,
            },
            ItemFrameMapTexture {
                map_id: 5,
                rgba: first_map_rgba,
            },
            ItemFrameMapTexture {
                map_id: 99,
                rgba: vec![1, 2, 3],
            },
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
    fn item_model_shader_applies_vertex_light_after_sampling() {
        assert!(ITEM_MODEL_SHADER.contains("@location(3) light: vec2<f32>"));
        assert!(ITEM_MODEL_SHADER
            .contains("let light_level = max(input.light.x, input.light.y * 0.95);"));
        assert!(ITEM_MODEL_SHADER.contains("return vec4<f32>(texel.rgb * shade, texel.a);"));
    }
}
