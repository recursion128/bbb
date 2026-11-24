//! Far-terrain horizon LOD mesh storage and rendering pipeline.

use std::mem;

use crate::frame_buffers::FrameDataBuffer;
use crate::gpu::DEPTH_FORMAT;
use crate::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LodTerrainVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
    pub uv: [f32; 2],
    pub light: [u8; 2],
    pub _pad: [u8; 2],
}

const _: () = assert!(mem::size_of::<LodTerrainVertex>() == 28);

const LOD_TERRAIN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2, 3 => Uint8x4];

const LOD_TERRAIN_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::evaluate_dynamic_lights!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var terrain_atlas: texture_2d<f32>;

@group(0) @binding(2)
var terrain_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light_pad: vec4<u32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) vertex_color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) fog_pos: vec3<f32>,
};

fn sample_lod_lightmap(light: vec2<u32>) -> vec3<f32> {
    let uv = clamp(
        vec2<f32>(light) / 256.0 + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSampleLevel(lightmap_texture, lightmap_sampler, uv, 0.0).rgb;
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.position = camera.projection * (camera.sky_model_view * vec4<f32>(fog_pos, 1.0));
    var light_rgb = sample_lod_lightmap(input.light_pad.xy);
    if (u32(camera.dynamic_light_params.x) > 0u) {
        light_rgb = max(light_rgb, evaluate_dynamic_lights(fog_pos));
    }
    out.vertex_color = vec4<f32>(input.color.rgb * light_rgb, input.color.a);
    out.uv = input.uv;
    out.fog_pos = fog_pos;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(terrain_atlas, terrain_sampler, input.uv);
    let color = vec4<f32>(texel.rgb * input.vertex_color.rgb, 1.0);
    let lod_end = max(camera.fog_visibility_ends.z, camera.fog_distances.w);
    let lod_start = lod_end * 0.65;
    let spherical = length(input.fog_pos);
    let cylindrical = max(length(input.fog_pos.xz), abs(input.fog_pos.y));
    let environmental_fog = linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y);
    let horizon_fog = linear_fog_value(max(spherical, cylindrical), lod_start, lod_end);
    let fog_value = max(environmental_fog, horizon_fog);
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), 1.0);
}
"#
);

pub(crate) fn create_lod_terrain_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    terrain_layout: &wgpu::BindGroupLayout,
    lightmap_sample_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layouts = [terrain_layout, lightmap_sample_layout];
    RenderPipelineBuilder::new(device, "bbb-lod-terrain-pipeline")
        .shader("bbb-lod-terrain-shader", LOD_TERRAIN_SHADER)
        .layout("bbb-lod-terrain-layout", &layouts)
        .vertex_buffers(&[wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<LodTerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &LOD_TERRAIN_VERTEX_ATTRIBUTES,
        }])
        .color_target(format, Some(wgpu::BlendState::REPLACE))
        .cull_mode(Some(wgpu::Face::Back))
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            true,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

pub(crate) struct LodTerrainGpu {
    vertices: FrameDataBuffer,
    indices: FrameDataBuffer,
    index_count: u32,
}

impl Default for LodTerrainGpu {
    fn default() -> Self {
        Self {
            vertices: FrameDataBuffer::vertex("bbb-lod-terrain-vertices"),
            indices: FrameDataBuffer::index("bbb-lod-terrain-indices"),
            index_count: 0,
        }
    }
}

impl LodTerrainGpu {
    pub(crate) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[LodTerrainVertex],
        indices: &[u32],
    ) {
        if vertices.is_empty() || indices.is_empty() {
            self.index_count = 0;
            return;
        }
        let v_ok = self
            .vertices
            .upload(device, queue, bytemuck::cast_slice(vertices));
        let i_ok = self
            .indices
            .upload(device, queue, bytemuck::cast_slice(indices));
        self.index_count = if v_ok && i_ok {
            u32::try_from(indices.len()).unwrap_or(0)
        } else {
            0
        };
    }

    pub(crate) fn clear(&mut self) {
        self.index_count = 0;
    }

    pub(crate) fn has_draws(&self) -> bool {
        self.index_count > 0 && self.vertices.buffer().is_some() && self.indices.buffer().is_some()
    }

    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        terrain_bind_group: &wgpu::BindGroup,
        lightmap_bind_group: &wgpu::BindGroup,
    ) -> bool {
        if !self.has_draws() {
            return false;
        }
        let Some(vbuf) = self.vertices.buffer() else {
            return false;
        };
        let Some(ibuf) = self.indices.buffer() else {
            return false;
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, terrain_bind_group, &[]);
        pass.set_bind_group(1, lightmap_bind_group, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_terrain_shader_declares_expected_groups_and_fog() {
        assert!(LOD_TERRAIN_SHADER.contains("@group(0) @binding(1)"));
        assert!(LOD_TERRAIN_SHADER.contains("@group(1) @binding(0)"));
        assert!(LOD_TERRAIN_SHADER.contains("camera.fog_visibility_ends.z"));
        assert!(LOD_TERRAIN_SHADER.contains(
            "linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y)"
        ));
        assert!(LOD_TERRAIN_SHADER.contains("max(environmental_fog, horizon_fog)"));
        assert!(LOD_TERRAIN_SHADER.contains("evaluate_dynamic_lights"));
        assert!(
            LOD_TERRAIN_SHADER.contains("textureSample(terrain_atlas, terrain_sampler, input.uv)")
        );
        assert!(!LOD_TERRAIN_SHADER.contains("textureSampleLevel(terrain_atlas"));
    }

    #[test]
    fn lod_terrain_pipeline_compiles_on_offscreen_device() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(64, 64) else {
            return;
        };
        let vertices = [
            LodTerrainVertex {
                position: [200.0, 64.0, 200.0],
                color: [120, 180, 90, 255],
                uv: [0.5, 0.5],
                light: [0, 240],
                _pad: [0, 0],
            },
            LodTerrainVertex {
                position: [204.0, 64.0, 200.0],
                color: [120, 180, 90, 255],
                uv: [0.5, 0.5],
                light: [0, 240],
                _pad: [0, 0],
            },
            LodTerrainVertex {
                position: [204.0, 64.0, 204.0],
                color: [120, 180, 90, 255],
                uv: [0.5, 0.5],
                light: [0, 240],
                _pad: [0, 0],
            },
        ];
        renderer.set_lod_terrain_mesh(&vertices, &[0, 1, 2], 1024.0);
        assert!(renderer.lod_terrain.has_draws());
        renderer.clear_lod_terrain_mesh();
        assert!(!renderer.lod_terrain.has_draws());
    }
}
