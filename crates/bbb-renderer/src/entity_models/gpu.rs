use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::{
    camera::TerrainBounds,
    gpu::DEPTH_FORMAT,
    player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage},
    Renderer,
};

use super::{
    catalog::{
        EntityDynamicPlayerSkinAtlasEntry, EntityDynamicPlayerSkinAtlasLayout,
        EntityDynamicPlayerTextureAtlasEntry, EntityDynamicPlayerTextureAtlasLayout,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureImage,
        EntityModelUvRect,
    },
    entity_model_colored_runtime_mesh, entity_model_textured_meshes_with_dynamic_textures,
    geometry::{
        EntityModelScrollMesh, EntityModelScrollVertex, EntityModelTexturedMesh,
        EntityModelTexturedVertex, EntityModelVertex,
    },
    instances::EntityModelInstance,
};

pub(crate) struct EntityModelMeshGpu {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) bounds: Option<TerrainBounds>,
}

pub(crate) struct EntityModelTexturedMeshGpu {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) bounds: Option<TerrainBounds>,
}

pub(crate) struct EntityModelScrollMeshGpu {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) bounds: Option<TerrainBounds>,
}

pub(crate) struct EntityModelTextureAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityModelTextureAtlasLayout,
}

pub(crate) struct EntityDynamicPlayerSkinAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityDynamicPlayerSkinAtlasLayout,
}

pub(crate) struct EntityDynamicPlayerTextureAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityDynamicPlayerTextureAtlasLayout,
}

pub(super) const ENTITY_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2, 3 => Float32x2];
pub(super) const ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2];
pub(super) const ENTITY_MODEL_SCROLL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x4, 5 => Float32x2, 6 => Float32x2];

pub(super) const ENTITY_MODEL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) light: vec2<f32>,
    @location(2) overlay: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    out.light = input.light;
    out.overlay = input.overlay;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    var rgb = input.color.rgb;
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 179.0 / 255.0);
    } else {
        let overlay_alpha = 1.0 - input.overlay.x / 15.0 * 0.75;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    let light_level = max(input.light.x, input.light.y * 0.95);
    let shade = 0.16 + light_level * 0.84;
    return vec4<f32>(rgb * shade, input.color.a);
}
"#;

pub(super) const ENTITY_MODEL_TEXTURED_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) overlay: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    out.light = input.light;
    out.overlay = input.overlay;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint;
    if texel.a <= 0.01 {
        discard;
    }
    var rgb = texel.rgb;
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 179.0 / 255.0);
    } else {
        let overlay_alpha = 1.0 - input.overlay.x / 15.0 * 0.75;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    let light_level = max(input.light.x, input.light.y * 0.95);
    let shade = 0.16 + light_level * 0.84;
    return vec4<f32>(rgb * shade, texel.a);
}
"#;

pub(super) const ENTITY_MODEL_EYES_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint;
}
"#;

// The scrolling-overlay shader for vanilla `breezeWind`: texture-matrix scroll, lightmap-lit, no overlay,
// and no cardinal lighting. Because the texture lives in the shared atlas, the per-fragment `fract` of
// the (offset-baked) local UV reproduces the `GL_REPEAT` seam, then maps back into the atlas sub-rect.
// The vanilla `ALPHA_CUTOUT 0.1` discard is applied.
pub(super) const ENTITY_MODEL_SCROLL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) light: vec2<f32>,
};

fn lightmap_brightness(level: f32) -> f32 {
    return level / (4.0 - 3.0 * level);
}

fn packed_lightmap_shade(light: vec2<f32>) -> f32 {
    let block_brightness = lightmap_brightness(light.x) * 1.4;
    let sky_brightness = lightmap_brightness(light.y);
    return clamp(block_brightness + sky_brightness, 0.0, 1.0);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    out.light = input.light;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let texel = textureSample(entity_texture_atlas, entity_sampler, atlas_uv) * input.tint;
    if texel.a <= 0.1 {
        discard;
    }
    let shade = packed_lightmap_shade(input.light);
    return vec4<f32>(texel.rgb * shade, texel.a);
}
"#;

// The additive scrolling-overlay shader for vanilla `energySwirl`: texture-matrix scroll, alpha cutout,
// additive blend, and emissive `NO_OVERLAY` / `NO_CARDINAL_LIGHTING` rendering.
pub(super) const ENTITY_MODEL_SCROLL_EMISSIVE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let texel = textureSample(entity_texture_atlas, entity_sampler, atlas_uv) * input.tint;
    if texel.a <= 0.1 {
        discard;
    }
    return texel;
}
"#;

pub(crate) fn create_entity_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-entity-model-shader"),
        source: wgpu::ShaderSource::Wgsl(ENTITY_MODEL_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-entity-model-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-entity-model-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[entity_model_vertex_layout()],
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

pub(crate) fn create_entity_model_textured_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_model_textured_pipeline_with_depth(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-textured",
        ENTITY_MODEL_TEXTURED_SHADER,
        Some(wgpu::BlendState::REPLACE),
        true,
    )
}

pub(crate) fn create_entity_model_eyes_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_model_textured_pipeline_with_depth(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-eyes",
        ENTITY_MODEL_EYES_SHADER,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        false,
    )
}

pub(crate) fn create_entity_model_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_model_textured_pipeline_with_depth(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-translucent",
        ENTITY_MODEL_TEXTURED_SHADER,
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
    )
}

/// Vanilla `BlendFunction.ADDITIVE`: `src·srcAlpha + dst·1` for colour, `src·1 + dst·1` for alpha.
/// Used by the `energySwirl` render type (the charged-creeper / wither energy-swirl glow).
const ENTITY_MODEL_ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The scrolling-overlay pipeline for vanilla `breezeWind` (the wind charge): translucent
/// (`BlendFunction.TRANSLUCENT`), lightmap-lit, depth-writing, cull off.
pub(crate) fn create_entity_model_scroll_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_model_scroll_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-scroll",
        ENTITY_MODEL_SCROLL_SHADER,
        wgpu::BlendState::ALPHA_BLENDING,
    )
}

/// The scrolling-overlay pipeline for vanilla `energySwirl` (the charged-creeper / wither glow):
/// additive ([`ENTITY_MODEL_ADDITIVE_BLEND`]), emissive, depth-writing, cull off.
pub(crate) fn create_entity_model_scroll_additive_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_entity_model_scroll_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-scroll-additive",
        ENTITY_MODEL_SCROLL_EMISSIVE_SHADER,
        ENTITY_MODEL_ADDITIVE_BLEND,
    )
}

/// Builds a scrolling-overlay pipeline (its own scroll vertex layout + [`ENTITY_MODEL_SCROLL_SHADER`],
/// depth-writing, cull off) with the given blend and shader.
fn create_entity_model_scroll_pipeline_with_blend(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label_prefix: &str,
    shader_source: &str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let shader_label = format!("{label_prefix}-shader");
    let pipeline_layout_label = format!("{label_prefix}-pipeline-layout");
    let pipeline_label = format!("{label_prefix}-pipeline");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(shader_label.as_str()),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(pipeline_layout_label.as_str()),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(pipeline_label.as_str()),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[entity_model_scroll_vertex_layout()],
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
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_entity_model_textured_pipeline_with_depth(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label_prefix: &str,
    shader_source: &str,
    blend: Option<wgpu::BlendState>,
    depth_write_enabled: bool,
) -> wgpu::RenderPipeline {
    let shader_label = format!("{label_prefix}-shader");
    let pipeline_layout_label = format!("{label_prefix}-pipeline-layout");
    let pipeline_label = format!("{label_prefix}-pipeline");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(shader_label.as_str()),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(pipeline_layout_label.as_str()),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(pipeline_label.as_str()),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[entity_model_textured_vertex_layout()],
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
            depth_write_enabled,
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
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

impl Renderer {
    pub fn upload_entity_model_textures(
        &mut self,
        images: &[EntityModelTextureImage],
    ) -> Result<()> {
        self.entity_model_texture_atlas = Some(create_entity_model_texture_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            images,
        )?);
        self.rebuild_entity_model_meshes();
        Ok(())
    }

    pub fn upload_dynamic_player_skin(&mut self, image: DynamicPlayerSkinImage) -> Result<()> {
        validate_dynamic_player_skin_image(&image)?;
        match self
            .entity_dynamic_player_skin_images
            .iter_mut()
            .find(|skin| skin.handle == image.handle)
        {
            Some(existing) => *existing = image,
            None => self.entity_dynamic_player_skin_images.push(image),
        }
        self.entity_dynamic_player_skin_images
            .sort_by_key(|skin| skin.handle);
        self.entity_dynamic_player_skin_atlas = Some(create_dynamic_player_skin_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.entity_dynamic_player_skin_images,
        )?);
        self.rebuild_entity_model_meshes();
        Ok(())
    }

    pub fn upload_dynamic_player_texture(
        &mut self,
        image: DynamicPlayerTextureImage,
    ) -> Result<()> {
        validate_dynamic_player_texture_image(&image)?;
        match self
            .entity_dynamic_player_texture_images
            .iter_mut()
            .find(|texture| texture.handle == image.handle)
        {
            Some(existing) => *existing = image,
            None => self.entity_dynamic_player_texture_images.push(image),
        }
        self.entity_dynamic_player_texture_images
            .sort_by_key(|texture| texture.handle);
        self.entity_dynamic_player_texture_atlas = Some(create_dynamic_player_texture_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.entity_dynamic_player_texture_images,
        )?);
        self.rebuild_entity_model_meshes();
        Ok(())
    }

    pub fn set_entity_model_instances(&mut self, instances: Vec<EntityModelInstance>) {
        let instances = sanitize_entity_model_instances(instances);
        if self.entity_model_instances.as_slice() == instances.as_slice() {
            return;
        }

        self.entity_model_instances = instances;
        self.rebuild_entity_model_meshes();
    }

    fn rebuild_entity_model_meshes(&mut self) {
        self.entity_model_mesh =
            create_entity_model_mesh_gpu(&self.device, self.entity_model_instances.clone());
        if let Some(atlas) = &self.entity_model_texture_atlas {
            let dynamic_player_skin_atlas = self
                .entity_dynamic_player_skin_atlas
                .as_ref()
                .map(|atlas| &atlas.layout);
            let dynamic_player_texture_atlas = self
                .entity_dynamic_player_texture_atlas
                .as_ref()
                .map(|atlas| &atlas.layout);
            let meshes = entity_model_textured_meshes_with_dynamic_textures(
                &self.entity_model_instances,
                &atlas.layout,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
            );
            self.entity_model_textured_mesh = create_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                meshes.cutout,
                "bbb-entity-model-textured",
            );
            self.entity_model_eyes_mesh = create_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                meshes.eyes,
                "bbb-entity-model-eyes",
            );
            self.entity_model_outline_mesh = create_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                meshes.outline,
                "bbb-entity-model-outline",
            );
            self.entity_model_translucent_mesh = create_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                meshes.translucent,
                "bbb-entity-model-translucent",
            );
            self.entity_dynamic_player_skin_cutout_mesh =
                create_entity_model_textured_mesh_gpu_from_mesh(
                    &self.device,
                    meshes.dynamic_player_skin_cutout,
                    "bbb-entity-dynamic-player-skin-cutout",
                );
            self.entity_dynamic_player_skin_translucent_mesh =
                create_entity_model_textured_mesh_gpu_from_mesh(
                    &self.device,
                    meshes.dynamic_player_skin_translucent,
                    "bbb-entity-dynamic-player-skin-translucent",
                );
            self.entity_dynamic_player_texture_cutout_mesh =
                create_entity_model_textured_mesh_gpu_from_mesh(
                    &self.device,
                    meshes.dynamic_player_texture_cutout,
                    "bbb-entity-dynamic-player-texture-cutout",
                );
            self.entity_dynamic_player_texture_translucent_mesh =
                create_entity_model_textured_mesh_gpu_from_mesh(
                    &self.device,
                    meshes.dynamic_player_texture_translucent,
                    "bbb-entity-dynamic-player-texture-translucent",
                );
            self.entity_model_scroll_mesh = create_entity_model_scroll_mesh_gpu_from_mesh(
                &self.device,
                meshes.scroll,
                "bbb-entity-model-scroll",
            );
            self.entity_model_scroll_additive_mesh = create_entity_model_scroll_mesh_gpu_from_mesh(
                &self.device,
                meshes.scroll_additive,
                "bbb-entity-model-scroll-additive",
            );
        } else {
            self.entity_model_textured_mesh = None;
            self.entity_model_translucent_mesh = None;
            self.entity_model_eyes_mesh = None;
            self.entity_model_outline_mesh = None;
            self.entity_dynamic_player_skin_cutout_mesh = None;
            self.entity_dynamic_player_skin_translucent_mesh = None;
            self.entity_dynamic_player_texture_cutout_mesh = None;
            self.entity_dynamic_player_texture_translucent_mesh = None;
            self.entity_model_scroll_mesh = None;
            self.entity_model_scroll_additive_mesh = None;
        }
        self.entity_model_bounds = merged_entity_model_bounds(&[
            self.entity_model_mesh.as_ref().and_then(|mesh| mesh.bounds),
            self.entity_model_textured_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_model_translucent_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_model_eyes_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_model_outline_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_dynamic_player_skin_cutout_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_dynamic_player_skin_translucent_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_dynamic_player_texture_cutout_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_dynamic_player_texture_translucent_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_model_scroll_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
            self.entity_model_scroll_additive_mesh
                .as_ref()
                .and_then(|mesh| mesh.bounds),
        ]);
        self.update_camera();
    }
}

fn create_entity_model_texture_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    images: &[EntityModelTextureImage],
) -> Result<EntityModelTextureAtlasGpu> {
    let (layout, rgba) = build_entity_model_texture_atlas(images)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-entity-model-texture-atlas"),
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
        &rgba,
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
        label: Some("bbb-entity-model-texture-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-entity-model-texture-bind-group"),
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

    Ok(EntityModelTextureAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    })
}

fn create_dynamic_player_skin_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    images: &[DynamicPlayerSkinImage],
) -> Result<EntityDynamicPlayerSkinAtlasGpu> {
    let (layout, rgba) = build_dynamic_player_skin_atlas(images)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-dynamic-player-skin-atlas"),
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
        &rgba,
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
        label: Some("bbb-dynamic-player-skin-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-dynamic-player-skin-bind-group"),
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

    Ok(EntityDynamicPlayerSkinAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    })
}

fn create_dynamic_player_texture_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    images: &[DynamicPlayerTextureImage],
) -> Result<EntityDynamicPlayerTextureAtlasGpu> {
    let (layout, rgba) = build_dynamic_player_texture_atlas(images)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-dynamic-player-texture-atlas"),
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
        &rgba,
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
        label: Some("bbb-dynamic-player-texture-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-dynamic-player-texture-bind-group"),
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

    Ok(EntityDynamicPlayerTextureAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_dynamic_player_skin_atlas(
    images: &[DynamicPlayerSkinImage],
) -> Result<(EntityDynamicPlayerSkinAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("dynamic player skin atlas requires at least one image");
    }

    let width = DynamicPlayerSkinImage::SIZE[0];
    let skin_height = DynamicPlayerSkinImage::SIZE[1];
    let height = skin_height
        .checked_mul(u32::try_from(images.len()).map_err(|_| {
            anyhow!(
                "dynamic player skin atlas image count {} overflows u32",
                images.len()
            )
        })?)
        .ok_or_else(|| anyhow!("dynamic player skin atlas height overflow"))?;
    let atlas_len = rgba_len(width, height, "dynamic player skin atlas")?;
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());

    let mut y = 0u32;
    for image in images {
        validate_dynamic_player_skin_image(image)?;
        let row_len = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| anyhow!("dynamic player skin row size overflow"))?;
        for row in 0..skin_height {
            let src_start = rgba_offset(width, row, 0, "dynamic player skin source")?;
            let src_end = src_start + row_len;
            let dst_start = rgba_offset(width, y + row, 0, "dynamic player skin atlas")?;
            let dst_end = dst_start + row_len;
            rgba[dst_start..dst_end].copy_from_slice(&image.rgba[src_start..src_end]);
        }
        entries.push(EntityDynamicPlayerSkinAtlasEntry {
            handle: image.handle,
            uv: EntityModelUvRect {
                min: [0.0, y as f32 / height as f32],
                max: [1.0, (y + skin_height) as f32 / height as f32],
            },
        });
        y += skin_height;
    }

    Ok((
        EntityDynamicPlayerSkinAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_dynamic_player_texture_atlas(
    images: &[DynamicPlayerTextureImage],
) -> Result<(EntityDynamicPlayerTextureAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("dynamic player texture atlas requires at least one image");
    }

    let mut seen = BTreeMap::new();
    let mut width = 0u32;
    let mut height = 0u32;
    for image in images {
        validate_dynamic_player_texture_image(image)?;
        if seen.insert(image.handle, ()).is_some() {
            bail!("duplicate dynamic player texture handle {}", image.handle);
        }
        width = width.max(image.size[0]);
        height = height
            .checked_add(image.size[1])
            .ok_or_else(|| anyhow!("dynamic player texture atlas height overflow"))?;
    }
    if width == 0 || height == 0 {
        bail!("dynamic player texture atlas dimensions must be non-zero");
    }
    let atlas_len = rgba_len(width, height, "dynamic player texture atlas")?;
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());

    let mut y = 0u32;
    for image in images {
        let image_width = image.size[0];
        let image_height = image.size[1];
        let row_len = usize::try_from(image_width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| anyhow!("dynamic player texture row size overflow"))?;
        for row in 0..image_height {
            let src_start = rgba_offset(image_width, row, 0, "dynamic player texture source")?;
            let src_end = src_start + row_len;
            let dst_start = rgba_offset(width, y + row, 0, "dynamic player texture atlas")?;
            let dst_end = dst_start + row_len;
            rgba[dst_start..dst_end].copy_from_slice(&image.rgba[src_start..src_end]);
        }
        entries.push(EntityDynamicPlayerTextureAtlasEntry {
            handle: image.handle,
            size: image.size,
            uv: EntityModelUvRect {
                min: [0.0, y as f32 / height as f32],
                max: [
                    image_width as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
        });
        y += image_height;
    }

    Ok((
        EntityDynamicPlayerTextureAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

pub(super) fn build_entity_model_texture_atlas(
    images: &[EntityModelTextureImage],
) -> Result<(EntityModelTextureAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("entity model texture atlas requires at least one image");
    }
    let mut seen = BTreeMap::new();
    let mut width = 0u32;
    let mut height = 0u32;
    for image in images {
        validate_entity_model_texture_image(image)?;
        if seen.insert(image.texture.path, ()).is_some() {
            bail!("duplicate entity model texture {}", image.texture.path);
        }
        width = width.max(image.texture.size[0]);
        height = height
            .checked_add(image.texture.size[1])
            .ok_or_else(|| anyhow!("entity model texture atlas height overflow"))?;
    }
    if width == 0 || height == 0 {
        bail!("entity model texture atlas dimensions must be non-zero");
    }
    let atlas_len = rgba_len(width, height, "entity model texture atlas")?;
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());
    let mut y = 0u32;
    for image in images {
        let image_width = image.texture.size[0];
        let image_height = image.texture.size[1];
        let row_len = usize::try_from(image_width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| anyhow!("entity model texture row size overflow"))?;
        for row in 0..image_height {
            let src_start = rgba_offset(image_width, row, 0, "entity model texture source")?;
            let src_end = src_start + row_len;
            let dst_start = rgba_offset(width, y + row, 0, "entity model texture atlas")?;
            let dst_end = dst_start + row_len;
            rgba[dst_start..dst_end].copy_from_slice(&image.rgba[src_start..src_end]);
        }
        entries.push(EntityModelTextureAtlasEntry {
            texture: image.texture,
            uv: EntityModelUvRect {
                min: [0.0, y as f32 / height as f32],
                max: [
                    image_width as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
        });
        y += image_height;
    }

    Ok((
        EntityModelTextureAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

fn validate_entity_model_texture_image(image: &EntityModelTextureImage) -> Result<()> {
    let [width, height] = image.texture.size;
    if width == 0 || height == 0 {
        bail!(
            "entity model texture {} has zero-sized dimensions",
            image.texture.path
        );
    }
    let expected_len = rgba_len(width, height, image.texture.path)?;
    if image.rgba.len() != expected_len {
        bail!(
            "entity model texture {} has {} RGBA bytes, expected {} for {}x{}",
            image.texture.path,
            image.rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn validate_dynamic_player_skin_image(image: &DynamicPlayerSkinImage) -> Result<()> {
    let [width, height] = DynamicPlayerSkinImage::SIZE;
    let expected_len = rgba_len(width, height, "dynamic player skin")?;
    if image.rgba.len() != expected_len {
        bail!(
            "dynamic player skin {} has {} RGBA bytes, expected {} for {}x{}",
            image.handle,
            image.rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn validate_dynamic_player_texture_image(image: &DynamicPlayerTextureImage) -> Result<()> {
    let [width, height] = image.size;
    if width == 0 || height == 0 {
        bail!(
            "dynamic player texture {} has zero-sized dimensions",
            image.handle
        );
    }
    let expected_len = rgba_len(width, height, "dynamic player texture")?;
    if image.rgba.len() != expected_len {
        bail!(
            "dynamic player texture {} has {} RGBA bytes, expected {} for {}x{}",
            image.handle,
            image.rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

fn rgba_len(width: u32, height: u32, label: &str) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA size overflow"))
}

pub(super) fn rgba_offset(width: u32, y: u32, x: u32, label: &str) -> Result<usize> {
    let width = usize::try_from(width).map_err(|_| anyhow!("{label} width overflow"))?;
    let x = usize::try_from(x).map_err(|_| anyhow!("{label} x overflow"))?;
    let y = usize::try_from(y).map_err(|_| anyhow!("{label} y overflow"))?;
    y.checked_mul(width)
        .and_then(|offset| offset.checked_add(x))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA offset overflow"))
}

fn create_entity_model_mesh_gpu(
    device: &wgpu::Device,
    instances: Vec<EntityModelInstance>,
) -> Option<EntityModelMeshGpu> {
    let mesh = entity_model_colored_runtime_mesh(&instances);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let bounds = TerrainBounds::from_points(
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position)),
    );
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-entity-model-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(EntityModelMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}

fn create_entity_model_textured_mesh_gpu_from_mesh(
    device: &wgpu::Device,
    mesh: EntityModelTexturedMesh,
    label_prefix: &str,
) -> Option<EntityModelTexturedMeshGpu> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let bounds = TerrainBounds::from_points(
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position)),
    );
    let vertex_label = format!("{label_prefix}-vertices");
    let index_label = format!("{label_prefix}-indices");
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(vertex_label.as_str()),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(index_label.as_str()),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(EntityModelTexturedMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}

fn merged_entity_model_bounds(bounds: &[Option<TerrainBounds>]) -> Option<TerrainBounds> {
    let mut merged: Option<TerrainBounds> = None;
    for bounds in bounds.iter().flatten() {
        match &mut merged {
            Some(merged) => merged.include_bounds(*bounds),
            None => merged = Some(*bounds),
        }
    }
    merged
}

pub(super) fn sanitize_entity_model_instances(
    instances: Vec<EntityModelInstance>,
) -> Vec<EntityModelInstance> {
    instances
        .into_iter()
        .filter(|instance| {
            instance.render_state.body_rot.is_finite()
                && instance
                    .position
                    .iter()
                    .all(|component| component.is_finite())
        })
        .collect()
}

pub(super) fn entity_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_textured_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelTexturedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_scroll_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelScrollVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_SCROLL_VERTEX_ATTRIBUTES,
    }
}

fn create_entity_model_scroll_mesh_gpu_from_mesh(
    device: &wgpu::Device,
    mesh: EntityModelScrollMesh,
    label_prefix: &str,
) -> Option<EntityModelScrollMeshGpu> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let bounds = TerrainBounds::from_points(
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position)),
    );
    let vertex_label = format!("{label_prefix}-vertices");
    let index_label = format!("{label_prefix}-indices");
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(vertex_label.as_str()),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(index_label.as_str()),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(EntityModelScrollMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}
