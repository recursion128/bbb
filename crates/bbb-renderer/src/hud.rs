use std::mem;

use anyhow::{anyhow, bail, Result};
use winit::dpi::PhysicalSize;

use crate::Renderer;

const HUD_HOTBAR_WIDTH: u32 = 182;
const HUD_HOTBAR_HEIGHT: u32 = 22;
const HUD_EXPERIENCE_BAR_WIDTH: u32 = 182;
const HUD_EXPERIENCE_BAR_HEIGHT: u32 = 5;
const HUD_EXPERIENCE_MARGIN_BOTTOM: f32 = 24.0;
const HUD_HEARTS_PER_ROW: u32 = 10;
const HUD_HEART_SPACING: f32 = 8.0;
const HUD_FOOD_ICONS_PER_ROW: u32 = 10;
const HUD_FOOD_SPACING: f32 = 8.0;

const HUD_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

const HUD_SHADER: &str = r#"
@group(0) @binding(0)
var hud_texture: texture_2d<f32>;
@group(0) @binding(1)
var hud_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(hud_texture, hud_sampler, input.uv);
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

pub(super) struct HudSpriteGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(super) bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

pub(super) struct HudDrawCommand<'a> {
    pub(super) sprite: &'a HudSpriteGpu,
    pub(super) start: u32,
    pub(super) end: u32,
}

#[derive(Debug, Clone, Copy)]
struct HudRect {
    x: f32,
    y: f32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudIconFill {
    Empty,
    Half,
    Full,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct HudVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Renderer {
    pub fn upload_hud_crosshair(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        let crosshair = self.upload_hud_sprite(width, height, rgba)?;
        self.counters.hud_crosshair_width = width;
        self.counters.hud_crosshair_height = height;
        self.hud_crosshair = Some(crosshair);
        Ok(())
    }

    pub fn upload_hud_hotbar(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_hotbar = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hotbar_selection(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hotbar_selection = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_container(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_container = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_empty(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn set_hud_health(&mut self, health: Option<f32>) {
        self.hud_health = health.filter(|health| health.is_finite());
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        self.hud_food = food;
    }

    pub fn set_hud_experience_progress(&mut self, progress: Option<f32>) {
        self.hud_experience_progress_value = progress
            .filter(|progress| progress.is_finite())
            .map(|progress| progress.clamp(0.0, 1.0));
    }

    pub fn set_hud_selected_slot(&mut self, slot: u8) {
        self.hud_selected_slot = slot.min(8);
    }

    fn upload_hud_sprite(&self, width: u32, height: u32, rgba: &[u8]) -> Result<HudSpriteGpu> {
        create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )
    }

    pub(super) fn collect_hud_draws(&self) -> (Vec<HudVertex>, Vec<HudDrawCommand<'_>>) {
        let mut vertices = Vec::new();
        let mut commands = Vec::new();
        let surface_size = self.surface_size();

        if let Some(crosshair) = &self.hud_crosshair {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                crosshair,
                surface_size,
                centered_hud_rect(surface_size, crosshair.width, crosshair.height),
            );
        }

        if let Some(hotbar) = &self.hud_hotbar {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                hotbar,
                surface_size,
                hotbar_hud_rect(surface_size, hotbar.width, hotbar.height),
            );
        }
        if let Some(selection) = &self.hud_hotbar_selection {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                selection,
                surface_size,
                hotbar_selection_hud_rect(
                    surface_size,
                    self.hud_selected_slot,
                    selection.width,
                    selection.height,
                ),
            );
        }

        if let (Some(progress), Some(background)) = (
            self.hud_experience_progress_value,
            &self.hud_experience_background,
        ) {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                background,
                surface_size,
                experience_bar_hud_rect(surface_size, background.width, background.height),
            );

            let progress_width = hud_experience_progress_width(progress);
            if progress_width > 0 {
                if let Some(progress_sprite) = &self.hud_experience_progress {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        progress_sprite,
                        surface_size,
                        experience_bar_hud_rect(
                            surface_size,
                            progress_width,
                            progress_sprite.height,
                        ),
                        progress_width as f32 / progress_sprite.width.max(1) as f32,
                    );
                }
            }
        }

        if let (Some(health), Some(container)) = (self.hud_health, &self.hud_heart_container) {
            for index in 0..HUD_HEARTS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    container,
                    surface_size,
                    heart_hud_rect(surface_size, index, container.width, container.height),
                );
            }

            for index in 0..HUD_HEARTS_PER_ROW {
                let sprite = match hud_heart_fill(health, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_heart_half.as_ref(),
                    HudIconFill::Full => self.hud_heart_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        heart_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        if let (Some(food), Some(empty)) = (self.hud_food, &self.hud_food_empty) {
            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    empty,
                    surface_size,
                    food_hud_rect(surface_size, index, empty.width, empty.height),
                );
            }

            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                let sprite = match hud_food_fill(food, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_food_half.as_ref(),
                    HudIconFill::Full => self.hud_food_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        food_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        (vertices, commands)
    }
}

pub(super) fn create_hud_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-hud-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_hud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-hud-shader"),
        source: wgpu::ShaderSource::Wgsl(HUD_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-hud-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-hud-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[hud_vertex_layout()],
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
        depth_stencil: None,
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

fn create_hud_sprite_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<HudSpriteGpu> {
    if width == 0 || height == 0 {
        bail!("hud sprite dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("hud sprite size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "hud sprite has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-hud-sprite-texture"),
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
        label: Some("bbb-hud-sprite-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-hud-sprite-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(HudSpriteGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        width,
        height,
    })
}

fn push_hud_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    push_hud_draw_with_uv(vertices, commands, sprite, surface_size, rect, 1.0);
}

fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv_max_x: f32,
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        rect,
        uv_max_x.clamp(0.0, 1.0),
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand { sprite, start, end });
}

fn centered_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    HudRect {
        x: (surface_size.width.max(1) as f32 - width as f32) * 0.5,
        y: (surface_size.height.max(1) as f32 - height as f32) * 0.5,
        width,
        height,
    }
}

fn hotbar_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_HOTBAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_HOTBAR_HEIGHT as f32,
        width,
        height,
    }
}

fn experience_bar_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_EXPERIENCE_BAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_EXPERIENCE_MARGIN_BOTTOM - HUD_EXPERIENCE_BAR_HEIGHT as f32,
        width,
        height,
    }
}

fn hud_experience_progress_width(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 183.0).floor() as u32).min(HUD_EXPERIENCE_BAR_WIDTH)
}

fn hotbar_selection_hud_rect(
    surface_size: PhysicalSize<u32>,
    selected_slot: u8,
    width: u32,
    height: u32,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: hotbar.x - 1.0 + f32::from(selected_slot.min(8)) * 20.0,
        y: hotbar.y - 1.0,
        width,
        height,
    }
}

fn heart_hud_rect(surface_size: PhysicalSize<u32>, index: u32, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 - 91.0 + index as f32 * HUD_HEART_SPACING,
        y: surface_height - 39.0,
        width,
        height,
    }
}

fn food_hud_rect(surface_size: PhysicalSize<u32>, index: u32, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_FOOD_SPACING - width as f32,
        y: surface_height - 39.0,
        width,
        height,
    }
}

fn hud_heart_fill(health: f32, index: u32) -> HudIconFill {
    let current_halves = health.ceil().clamp(0.0, (HUD_HEARTS_PER_ROW * 2) as f32) as u32;
    let start_half = index * 2;
    if start_half >= current_halves {
        HudIconFill::Empty
    } else if start_half + 1 == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Full
    }
}

fn hud_food_fill(food: i32, index: u32) -> HudIconFill {
    let current_halves = food.clamp(0, (HUD_FOOD_ICONS_PER_ROW * 2) as i32) as u32;
    let center_half = index * 2 + 1;
    if center_half < current_halves {
        HudIconFill::Full
    } else if center_half == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Empty
    }
}

fn hud_quad_vertices(
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv_max_x: f32,
) -> [HudVertex; 6] {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width as f32;
    let y1 = rect.y + rect.height as f32;
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let left = x0 / width * 2.0 - 1.0;
    let right = x1 / width * 2.0 - 1.0;
    let top = 1.0 - y0 / height * 2.0;
    let bottom = 1.0 - y1 / height * 2.0;
    [
        HudVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, top],
            uv: [uv_max_x, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: [uv_max_x, 1.0],
        },
        HudVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: [uv_max_x, 1.0],
        },
        HudVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
    ]
}

fn hud_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<HudVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &HUD_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_quad_vertices_center_sprite_in_ndc() {
        let surface_size = PhysicalSize::new(1280, 720);
        let vertices = hud_quad_vertices(surface_size, centered_hud_rect(surface_size, 16, 8), 1.0);
        assert_f32_near(vertices[0].position[0], -0.0125);
        assert_f32_near(vertices[0].position[1], 0.011111111);
        assert_f32_near(vertices[2].position[0], 0.0125);
        assert_f32_near(vertices[2].position[1], -0.011111111);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn hud_layout_matches_vanilla_hotbar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let hotbar = hotbar_hud_rect(surface_size, 182, 22);
        assert_eq!(hotbar.x, 549.0);
        assert_eq!(hotbar.y, 698.0);
        assert_eq!(hotbar.width, 182);
        assert_eq!(hotbar.height, 22);

        let selection = hotbar_selection_hud_rect(surface_size, 0, 24, 23);
        assert_eq!(selection.x, 548.0);
        assert_eq!(selection.y, 697.0);
        assert_eq!(selection.width, 24);
        assert_eq!(selection.height, 23);

        let last_selection = hotbar_selection_hud_rect(surface_size, 8, 24, 23);
        assert_eq!(last_selection.x, 708.0);
        assert_eq!(last_selection.y, 697.0);
    }

    #[test]
    fn hud_layout_matches_vanilla_experience_bar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let bar = experience_bar_hud_rect(surface_size, 182, 5);
        assert_eq!(bar.x, 549.0);
        assert_eq!(bar.y, 691.0);
        assert_eq!(bar.width, 182);
        assert_eq!(bar.height, 5);

        assert_eq!(hud_experience_progress_width(0.0), 0);
        assert_eq!(hud_experience_progress_width(0.5), 91);
        assert_eq!(hud_experience_progress_width(1.0), 182);
    }

    #[test]
    fn hud_layout_places_player_hearts_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = heart_hud_rect(surface_size, 0, 9, 9);
        let last = heart_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 549.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 621.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_layout_places_food_icons_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = food_hud_rect(surface_size, 0, 9, 9);
        let last = food_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 722.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_heart_fill_uses_ceiled_health_halves() {
        assert_eq!(hud_heart_fill(0.0, 0), HudIconFill::Empty);
        assert_eq!(hud_heart_fill(5.0, 0), HudIconFill::Full);
        assert_eq!(hud_heart_fill(5.0, 2), HudIconFill::Half);
        assert_eq!(hud_heart_fill(5.5, 2), HudIconFill::Full);
        assert_eq!(hud_heart_fill(20.0, 9), HudIconFill::Full);
        assert_eq!(hud_heart_fill(25.0, 9), HudIconFill::Full);
    }

    #[test]
    fn hud_food_fill_uses_server_food_halves() {
        assert_eq!(hud_food_fill(0, 0), HudIconFill::Empty);
        assert_eq!(hud_food_fill(5, 0), HudIconFill::Full);
        assert_eq!(hud_food_fill(5, 2), HudIconFill::Half);
        assert_eq!(hud_food_fill(6, 2), HudIconFill::Full);
        assert_eq!(hud_food_fill(20, 9), HudIconFill::Full);
        assert_eq!(hud_food_fill(25, 9), HudIconFill::Full);
    }

    fn assert_f32_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000001,
            "actual {actual} expected {expected}"
        );
    }
}
