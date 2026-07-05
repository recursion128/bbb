//! World border (forcefield) rendering.
//!
//! Transcribed from vanilla 26.1 `WorldBorderRenderer.java`: the extraction
//! side (bbb-native) fills [`WorldBorderRenderState`] like
//! `WorldBorderRenderer.extract`, and [`build_world_border_mesh`] rebuilds the
//! four wall quads each frame like `rebuildWorldBorderBuffer` plus the
//! per-face `closestBorder` draw selection in `WorldBorderRenderer.render`.
//! The draw itself happens inside the weather target pass (render.rs), after
//! rain/snow, matching `LevelRenderer.addWeatherPass` (LevelRenderer.java:751-758).

use anyhow::{bail, Result};

use crate::gpu::DEPTH_FORMAT;
use crate::pipeline_builder::RenderPipelineBuilder;

/// Vanilla `WorldBorderRenderer.FORCEFIELD_LOCATION`
/// (`WorldBorderRenderer.java:34`).
pub const WORLD_BORDER_FORCEFIELD_TEXTURE_PATH: &str = "textures/misc/forcefield.png";

/// Per-frame world border render state, mirroring vanilla
/// `WorldBorderRenderState` (minX/maxX/minZ/maxZ/tint/alpha) plus the render
/// inputs vanilla threads through `WorldBorderRenderer.render(state, cameraPos,
/// renderDistance, depthFar)` and `Util.getMillis()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBorderRenderState {
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
    /// Vanilla `BorderStatus` color as `0xRRGGBB` (`BorderStatus.java:4-6`).
    pub tint: u32,
    /// Vanilla `WorldBorderRenderState.alpha`; `<= 0` skips rendering
    /// (`WorldBorderRenderer.java:127`).
    pub alpha: f64,
    /// Camera eye position (vanilla `cameraPos` in `WorldBorderRenderer.render`).
    pub camera_position: [f64; 3],
    /// Render distance in blocks (`LevelRenderer.addWeatherPass:744`:
    /// `optionsRenderState.renderDistance * 16`).
    pub render_distance: f64,
    /// Vanilla `CameraRenderState.depthFar` (`Camera.java:91-92`:
    /// `max(renderDistanceBlocks * 4, cloudRangeChunks * 16)`), used as the
    /// wall's half height (`WorldBorderRenderer.java:130`).
    pub depth_far: f32,
    /// Vanilla UV scroll `offset = (Util.getMillis() % 3000L) / 3000.0F`
    /// (`WorldBorderRenderer.java:134`).
    pub texture_offset: f32,
}

impl Default for WorldBorderRenderState {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            max_x: 0.0,
            min_z: 0.0,
            max_z: 0.0,
            tint: 0,
            // Vanilla `WorldBorderRenderState.reset()` zeroes alpha, which is
            // the "nothing to draw" state (`WorldBorderRenderer.java:127`).
            alpha: 0.0,
            camera_position: [0.0, 0.0, 0.0],
            render_distance: 0.0,
            depth_far: 0.0,
            texture_offset: 0.0,
        }
    }
}

/// Vanilla vertex format is `POSITION_TEX` with the tint/alpha supplied as the
/// `ColorModulator` dynamic uniform (`WorldBorderRenderer.java:157-162`); bbb
/// has no per-draw dynamic uniforms, so the constant modulator is baked into a
/// per-vertex color, and the fragment shader keeps vanilla's
/// `texture * ColorModulator` order (`rendertype_world_border.fsh:11-14`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct WorldBorderVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorldBorderMesh {
    /// All four wall quads in vanilla buffer order (SOUTH, WEST, NORTH, EAST),
    /// like the persistent `worldBorderBuffer` (`WorldBorderRenderer.java:68-84`).
    pub(crate) vertices: Vec<WorldBorderVertex>,
    /// Indices for the visible faces only, closest face first, mirroring the
    /// sorted `closestBorder` draw list (`WorldBorderRenderer.java:176-181`).
    pub(crate) indices: Vec<u32>,
}

const WORLD_BORDER_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];

/// Vanilla `RenderPipelines.WORLD_BORDER` uses `BlendFunction.OVERLAY`
/// (`RenderPipelines.java:573-585`), which is
/// `(SRC_ALPHA, ONE, ONE, ZERO)` (`BlendFunction.java:9`).
pub(crate) const WORLD_BORDER_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::Zero,
        operation: wgpu::BlendOperation::Add,
    },
};
/// Vanilla `RenderPipelines.WORLD_BORDER` `.withCull(false)`
/// (`RenderPipelines.java:579`).
pub(crate) const WORLD_BORDER_CULL_MODE: Option<wgpu::Face> = None;
/// Vanilla `DepthStencilState(LESS_THAN_OR_EQUAL, true, -3.0F, -3.0F)`
/// (`RenderPipelines.java:582`): depth test LEQUAL, depth write on, depth bias
/// scale factor -3 and constant -3.
pub(crate) const WORLD_BORDER_DEPTH_WRITE_ENABLED: bool = true;
pub(crate) const WORLD_BORDER_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;
pub(crate) const WORLD_BORDER_DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -3,
    slope_scale: -3.0,
    clamp: 0.0,
};

/// WGSL port of vanilla `core/rendertype_world_border` (vsh/fsh): position is
/// transformed by the camera matrix, the UV scroll (vanilla `TextureMat`
/// translation) and `ColorModulator` are baked per-vertex, and the fragment
/// stage keeps `if (color.a == 0.0) discard;` followed by the modulator
/// multiply (`rendertype_world_border.fsh:10-15`). No fog (the vanilla shader
/// applies none).
pub(crate) const WORLD_BORDER_SHADER: &str = r#"
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
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var forcefield_texture: texture_2d<f32>;

@group(0) @binding(2)
var forcefield_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(forcefield_texture, forcefield_sampler, input.uv);
    if (texel.a == 0.0) {
        discard;
    }
    return texel * input.color;
}
"#;

fn world_border_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<WorldBorderVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &WORLD_BORDER_VERTEX_ATTRIBUTES,
    }
}

pub(crate) fn create_world_border_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, "bbb-world-border-pipeline")
        .shader("bbb-world-border-shader", WORLD_BORDER_SHADER)
        .layout("bbb-world-border-pipeline-layout", &[bind_group_layout])
        .vertex_buffers(&[world_border_vertex_layout()])
        .color_target(format, Some(WORLD_BORDER_BLEND))
        .cull_mode(WORLD_BORDER_CULL_MODE)
        .depth_stencil(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: WORLD_BORDER_DEPTH_WRITE_ENABLED,
            depth_compare: WORLD_BORDER_DEPTH_COMPARE,
            stencil: wgpu::StencilState::default(),
            bias: WORLD_BORDER_DEPTH_BIAS,
        })
        .build()
}

pub(crate) struct WorldBorderTextureGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
}

/// Uploads `textures/misc/forcefield.png` bytes fed from the native side.
/// Repeat addressing is required because the wall UVs tile 0.5 per block
/// horizontally and `depthFar / (2 * depthFar)` vertically
/// (`WorldBorderRenderer.java:62-84,135-136`); nearest filtering matches the
/// vanilla texture's default (non-blurred) sampler.
pub(crate) fn create_world_border_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<WorldBorderTextureGpu> {
    if width == 0 || height == 0 {
        bail!("world border texture must have non-zero dimensions");
    }
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        bail!(
            "world border texture rgba length {} does not match {}x{}",
            rgba.len(),
            width,
            height
        );
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-world-border-forcefield-texture"),
        size: wgpu::Extent3d {
            width,
            height,
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
        label: Some("bbb-world-border-forcefield-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-world-border-forcefield-bind-group"),
        layout,
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

    Ok(WorldBorderTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

/// Vanilla sequential QUADS index pattern per quad: `0, 1, 2, 2, 3, 0`
/// (`RenderSystem.AutoStorageIndexBuffer` for `VertexFormat.Mode.QUADS`).
const QUAD_INDEX_PATTERN: [u32; 6] = [0, 1, 2, 2, 3, 0];

/// Rebuilds the world border wall mesh for the current frame.
///
/// Vertex construction transcribes `rebuildWorldBorderBuffer`
/// (`WorldBorderRenderer.java:46-100`); vanilla caches the buffer until the
/// border bounds change, while bbb rebuilds the same formula every frame
/// through a persistent `FrameDataBuffer`, which also keeps the camera-derived
/// wall extent fresh. Vanilla renders camera-relative with
/// `ModelOffset = (lastMinX - cameraX, -cameraY, lastMinZ - cameraZ)`
/// (`WorldBorderRenderer.java:157-163`); bbb's camera uniform consumes world
/// coordinates, so the `(minX, 0, minZ)` anchor is added back here (the local
/// vertex Y already is the world Y in vanilla's formulation).
pub(crate) fn build_world_border_mesh(state: &WorldBorderRenderState) -> Option<WorldBorderMesh> {
    // Vanilla skips rendering entirely for alpha <= 0 (WorldBorderRenderer.java:127).
    if state.alpha <= 0.0 {
        return None;
    }
    let camera_x = state.camera_position[0];
    let camera_y = state.camera_position[1];
    let camera_z = state.camera_position[2];
    let render_distance = state.render_distance;
    // halfHeightY = (float)depthFar (WorldBorderRenderer.java:130).
    let half_height = state.depth_far;
    // red/green/blue = ARGB components of the tint / 255 (WorldBorderRenderer.java:131-133).
    let red = ((state.tint >> 16) & 0xFF) as f32 / 255.0;
    let green = ((state.tint >> 8) & 0xFF) as f32 / 255.0;
    let blue = (state.tint & 0xFF) as f32 / 255.0;
    let color = [red, green, blue, state.alpha as f32];
    // v0 = -Mth.frac(cameraPos.y * 0.5); v1 = v0 + halfHeightY
    // (WorldBorderRenderer.java:135-136).
    let v0 = (-((camera_y * 0.5) - (camera_y * 0.5).floor())) as f32;
    let v1 = v0 + half_height;
    // UV scroll: vanilla applies TextureMat = translation(offset, offset, 0)
    // in the vertex shader (WorldBorderRenderer.java:163,
    // rendertype_world_border.vsh:15); baked into the UVs here.
    let scroll = state.texture_offset;

    // rebuildWorldBorderBuffer (WorldBorderRenderer.java:56-67).
    let border_min_x = state.min_x;
    let border_max_x = state.max_x;
    let border_min_z = state.min_z;
    let border_max_z = state.max_z;
    let min_z = f64::max((camera_z - render_distance).floor(), border_min_z);
    let max_z = f64::min((camera_z + render_distance).ceil(), border_max_z);
    let u0z = ((min_z.floor() as i64) & 1) as f32 * 0.5;
    let u1z = ((max_z - min_z) as f32) / 2.0;
    let min_x = f64::max((camera_x - render_distance).floor(), border_min_x);
    let max_x = f64::min((camera_x + render_distance).ceil(), border_max_x);
    let u0x = ((min_x.floor() as i64) & 1) as f32 * 0.5;
    let u1x = ((max_x - min_x) as f32) / 2.0;

    // Quad-local vertices in vanilla buffer order (WorldBorderRenderer.java:69-84):
    // quad 0 = SOUTH wall (z = borderMaxZ), quad 1 = WEST wall (x = minX),
    // quad 2 = NORTH wall (z = minZ), quad 3 = EAST wall (x = borderMaxX).
    // Vanilla vertex y = ±halfHeightY with ModelOffset.y = -cameraY, so the
    // world-space wall spans [-depthFar, depthFar] independent of the camera.
    let h = f64::from(half_height);
    let local_quads: [[([f64; 3], [f32; 2]); 4]; 4] = [
        [
            ([0.0, -h, border_max_z - min_z], [u0x, v1]),
            ([max_x - min_x, -h, border_max_z - min_z], [u1x + u0x, v1]),
            ([max_x - min_x, h, border_max_z - min_z], [u1x + u0x, v0]),
            ([0.0, h, border_max_z - min_z], [u0x, v0]),
        ],
        [
            ([0.0, -h, 0.0], [u0z, v1]),
            ([0.0, -h, max_z - min_z], [u1z + u0z, v1]),
            ([0.0, h, max_z - min_z], [u1z + u0z, v0]),
            ([0.0, h, 0.0], [u0z, v0]),
        ],
        [
            ([max_x - min_x, -h, 0.0], [u0x, v1]),
            ([0.0, -h, 0.0], [u1x + u0x, v1]),
            ([0.0, h, 0.0], [u1x + u0x, v0]),
            ([max_x - min_x, h, 0.0], [u0x, v0]),
        ],
        [
            ([border_max_x - min_x, -h, max_z - min_z], [u0z, v1]),
            ([border_max_x - min_x, -h, 0.0], [u1z + u0z, v1]),
            ([border_max_x - min_x, h, 0.0], [u1z + u0z, v0]),
            ([border_max_x - min_x, h, max_z - min_z], [u0z, v0]),
        ],
    ];
    let vertices: Vec<WorldBorderVertex> = local_quads
        .iter()
        .flat_map(|quad| quad.iter())
        .map(|(local, uv)| WorldBorderVertex {
            position: [
                (local[0] + min_x) as f32,
                local[1] as f32,
                (local[2] + min_z) as f32,
            ],
            uv: [uv[0] + scroll, uv[1] + scroll],
            color,
        })
        .collect();

    // WorldBorderRenderState.closestBorder builds (direction, distance) pairs
    // in NORTH, SOUTH, WEST, EAST insertion order and stable-sorts them by
    // distance (WorldBorderRenderState.java:17-24); render draws every side
    // with distance < renderDistance using index range 6 * get2DDataValue
    // (WorldBorderRenderer.java:176-181), where get2DDataValue is SOUTH=0,
    // WEST=1, NORTH=2, EAST=3 (Direction.java:35-38) — the buffer quad order.
    let mut faces = [
        (2_usize, camera_z - state.min_z),
        (0_usize, state.max_z - camera_z),
        (1_usize, camera_x - state.min_x),
        (3_usize, state.max_x - camera_x),
    ];
    faces.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut indices = Vec::new();
    for (quad, distance) in faces {
        if distance < render_distance {
            indices.extend(
                QUAD_INDEX_PATTERN
                    .iter()
                    .map(|offset| offset + 4 * quad as u32),
            );
        }
    }
    if indices.is_empty() {
        return None;
    }
    Some(WorldBorderMesh { vertices, indices })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible_state() -> WorldBorderRenderState {
        WorldBorderRenderState {
            min_x: -32.0,
            max_x: 32.0,
            min_z: -32.0,
            max_z: 32.0,
            tint: 0x20A0FF,
            alpha: 0.5,
            camera_position: [-28.0, 70.0, 0.0],
            render_distance: 16.0,
            depth_far: 64.0,
            texture_offset: 0.25,
        }
    }

    #[test]
    fn world_border_mesh_skips_non_positive_alpha() {
        // WorldBorderRenderer.render: `if (!(state.alpha <= 0.0))`
        // (WorldBorderRenderer.java:127).
        let mut state = visible_state();
        state.alpha = 0.0;
        assert_eq!(build_world_border_mesh(&state), None);
        state.alpha = -1.0;
        assert_eq!(build_world_border_mesh(&state), None);
    }

    #[test]
    fn world_border_mesh_matches_vanilla_rebuild_formula_for_west_wall() {
        let state = visible_state();
        let mesh = build_world_border_mesh(&state).expect("west wall visible");

        // All four quads are built in vanilla buffer order
        // (WorldBorderRenderer.java:69-84).
        assert_eq!(mesh.vertices.len(), 16);
        // Only the WEST wall is within renderDistance (distance 4 < 16); its
        // quad index via Direction.get2DDataValue is 1 (Direction.java:37).
        assert_eq!(mesh.indices, vec![4, 5, 6, 6, 7, 4]);

        // rebuildWorldBorderBuffer inputs for this state:
        // minZ = max(floor(0 - 16), -32) = -16, maxZ = min(ceil(0 + 16), 32) = 16,
        // u0z = (floor(-16) & 1) * 0.5 = 0, u1z = (16 - -16) / 2 = 16,
        // minX = max(floor(-28 - 16), -32) = -32, v0 = -frac(70 * 0.5) = 0,
        // v1 = v0 + depthFar = 64 (WorldBorderRenderer.java:60-67,135-136).
        // WEST wall local x = 0 anchored at minX; world z spans minZ..maxZ; the
        // 0.25 texture scroll offset is baked into both UV axes.
        let west: Vec<_> = mesh.vertices[4..8].to_vec();
        let expected = [
            ([-32.0_f32, -64.0, -16.0], [0.25_f32, 64.25]),
            ([-32.0, -64.0, 16.0], [16.25, 64.25]),
            ([-32.0, 64.0, 16.0], [16.25, 0.25]),
            ([-32.0, 64.0, -16.0], [0.25, 0.25]),
        ];
        for (vertex, (position, uv)) in west.iter().zip(expected) {
            assert_eq!(vertex.position, position);
            assert_eq!(vertex.uv, uv);
            // ColorModulator = (tint components / 255, alpha)
            // (WorldBorderRenderer.java:131-133,157-162), baked per vertex.
            assert_eq!(
                vertex.color,
                [0x20 as f32 / 255.0, 0xA0 as f32 / 255.0, 1.0, 0.5]
            );
        }
    }

    #[test]
    fn world_border_mesh_orders_visible_faces_closest_first() {
        // closestBorder sorts faces by camera distance
        // (WorldBorderRenderState.java:17-24): camera at (-28, _, -30) is 2
        // blocks from NORTH (quad 2) and 4 from WEST (quad 1).
        let mut state = visible_state();
        state.camera_position = [-28.0, 70.0, -30.0];
        let mesh = build_world_border_mesh(&state).expect("two walls visible");
        assert_eq!(mesh.indices, vec![8, 9, 10, 10, 11, 8, 4, 5, 6, 6, 7, 4]);
    }

    #[test]
    fn world_border_mesh_culls_all_faces_beyond_render_distance() {
        // render() only draws sides with distance < renderDistance
        // (WorldBorderRenderer.java:177); at the border center every wall is
        // 32 blocks away with renderDistance 16.
        let mut state = visible_state();
        state.camera_position = [0.0, 70.0, 0.0];
        assert_eq!(build_world_border_mesh(&state), None);
    }

    #[test]
    fn world_border_pipeline_state_matches_vanilla_world_border_pipeline() {
        // RenderPipelines.WORLD_BORDER (RenderPipelines.java:573-585):
        // BlendFunction.OVERLAY = (SRC_ALPHA, ONE, ONE, ZERO) (BlendFunction.java:9),
        // cull off, DepthStencilState(LESS_THAN_OR_EQUAL, true, -3.0F, -3.0F).
        assert_eq!(
            WORLD_BORDER_BLEND.color,
            wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            }
        );
        assert_eq!(
            WORLD_BORDER_BLEND.alpha,
            wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            }
        );
        assert_eq!(WORLD_BORDER_CULL_MODE, None);
        assert!(WORLD_BORDER_DEPTH_WRITE_ENABLED);
        assert_eq!(WORLD_BORDER_DEPTH_COMPARE, wgpu::CompareFunction::LessEqual);
        assert_eq!(WORLD_BORDER_DEPTH_BIAS.constant, -3);
        assert_eq!(WORLD_BORDER_DEPTH_BIAS.slope_scale, -3.0);
    }

    #[test]
    fn world_border_shader_keeps_vanilla_discard_and_modulator_order() {
        // rendertype_world_border.fsh:10-15: sample, discard on alpha == 0,
        // then multiply by ColorModulator (baked as the vertex color).
        let discard = WORLD_BORDER_SHADER
            .find("if (texel.a == 0.0)")
            .expect("alpha == 0 discard");
        let modulate = WORLD_BORDER_SHADER
            .find("return texel * input.color;")
            .expect("modulator multiply");
        assert!(discard < modulate);
    }
}
