//! Vanilla-compatible GPU composition for animated terrain-atlas sprites.
//!
//! Vanilla 26.1's `TextureAtlas` uploads each unique source-frame mip chain
//! once. `cycleAnimationFrames` then renders dirty or interpolated sprites into
//! each atlas mip. Keep that lifecycle here: client ticks only select source
//! frames; no runtime CPU interpolation, mip generation, or atlas patch upload
//! is permitted on this path.

use std::collections::HashMap;

use anyhow::{bail, Result};
use bbb_render_types::{
    AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasRect, SpriteAnimation, SpriteMipLevel,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{gpu::TerrainAtlasGpu, pipeline_builder::RenderPipelineBuilder};

const SHADER_SHARED: &str = r#"
@group(0) @binding(0) var sprite_frames: texture_2d_array<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;
struct AtlasAnimationInfo {
    destination_x: u32,
    destination_y: u32,
    content_width: u32,
    content_height: u32,
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    current_layer: u32,
    next_layer: u32,
    progress_thousandths: u32,
    _padding_0: u32,
    _padding_1: u32,
};
@group(0) @binding(2) var<uniform> animation_info: AtlasAnimationInfo;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) animation_progress: f32,
    @location(1) uv: vec2<f32>,
    @location(2) mip_level: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let position_index = vertex_index & 7u;
    let encoded = vertex_index >> 3u;
    let mip_level = encoded;
    var position = vec2<f32>(0.0, 0.0);
    switch position_index {
        case 1u, 4u: { position = vec2<f32>(1.0, 0.0); }
        case 2u, 3u: { position = vec2<f32>(0.0, 1.0); }
        case 5u: { position = vec2<f32>(1.0, 1.0); }
        default: {}
    }
    let source_size = vec2<f32>(textureDimensions(sprite_frames, 0));
    let uv_padding = vec2<f32>(f32(animation_info.padding)) / source_size;
    let direction = position * 2.0 - vec2<f32>(1.0);
    let destination = vec2<f32>(
        f32(animation_info.destination_x >> mip_level),
        f32(animation_info.destination_y >> mip_level),
    );
    let destination_size = vec2<f32>(
        f32((animation_info.content_width + animation_info.padding * 2u) >> mip_level),
        f32((animation_info.content_height + animation_info.padding * 2u) >> mip_level),
    );
    let atlas_size = vec2<f32>(
        f32(animation_info.atlas_width >> mip_level),
        f32(animation_info.atlas_height >> mip_level),
    );
    let atlas_position = destination + position * destination_size;

    var output: VertexOutput;
    output.position = vec4<f32>(
        atlas_position.x / atlas_size.x * 2.0 - 1.0,
        1.0 - atlas_position.y / atlas_size.y * 2.0,
        0.0,
        1.0,
    );
    output.animation_progress = f32(animation_info.progress_thousandths) / 1000.0;
    output.uv = position + uv_padding * direction;
    output.mip_level = f32(mip_level);
    return output;
}
"#;

const BLIT_FRAGMENT: &str = r#"
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(
        sprite_frames,
        sprite_sampler,
        input.uv,
        i32(animation_info.current_layer),
        input.mip_level,
    );
}
"#;

const INTERPOLATE_FRAGMENT: &str = r#"
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let current = textureSampleLevel(
        sprite_frames,
        sprite_sampler,
        input.uv,
        i32(animation_info.current_layer),
        input.mip_level,
    );
    let next = textureSampleLevel(
        sprite_frames,
        sprite_sampler,
        input.uv,
        i32(animation_info.next_layer),
        input.mip_level,
    );
    return mix(current, next, input.animation_progress);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AtlasAnimationInfo {
    destination_x: u32,
    destination_y: u32,
    content_width: u32,
    content_height: u32,
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    current_layer: u32,
    next_layer: u32,
    progress_thousandths: u32,
    _padding_0: u32,
    _padding_1: u32,
}

struct SpriteAnimationGpu {
    animation: SpriteAnimation,
    frame_layers: HashMap<u32, u32>,
    base_info: AtlasAnimationInfo,
    bind_group: wgpu::BindGroup,
    uniform_offset: u32,
}

pub(super) struct TerrainAtlasAnimationGpu {
    atlas_width: u32,
    atlas_height: u32,
    mip_level_count: u32,
    blit_pipeline: wgpu::RenderPipeline,
    _frame_textures: Vec<wgpu::Texture>,
    _frame_views: Vec<wgpu::TextureView>,
    _sampler: wgpu::Sampler,
    info_buffer: wgpu::Buffer,
    info_bytes: Vec<u8>,
    info_stride: usize,
    interpolate_bundles: Vec<wgpu::RenderBundle>,
    interpolated_draw_calls: u64,
    sprites: Vec<SpriteAnimationGpu>,
    pending_tick: Option<(u64, u64)>,
}

impl TerrainAtlasAnimationGpu {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: AtlasGpuAnimationPlan,
    ) -> Result<Self> {
        if plan.sprites.is_empty() {
            bail!("terrain atlas animation plan has no animated sprites");
        }
        let mip_level_count = plan
            .mip_level
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("terrain animation mip count overflow"))?;
        let bind_group_layout = create_bind_group_layout(device);
        let blit_source = format!("{SHADER_SHARED}{BLIT_FRAGMENT}");
        let interpolate_source = format!("{SHADER_SHARED}{INTERPOLATE_FRAGMENT}");
        let layouts = [&bind_group_layout];
        let blit_pipeline = RenderPipelineBuilder::new(device, "bbb-terrain-animation-blit")
            .shader("bbb-terrain-animation-blit-shader", &blit_source)
            .layout("bbb-terrain-animation-blit-layout", &layouts)
            .color_target(wgpu::TextureFormat::Rgba8Unorm, None)
            .build();
        let interpolate_pipeline =
            RenderPipelineBuilder::new(device, "bbb-terrain-animation-interpolate")
                .shader(
                    "bbb-terrain-animation-interpolate-shader",
                    &interpolate_source,
                )
                .layout("bbb-terrain-animation-interpolate-layout", &layouts)
                .color_target(wgpu::TextureFormat::Rgba8Unorm, None)
                .build();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bbb-terrain-animation-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let atlas_width = plan.atlas_width;
        let atlas_height = plan.atlas_height;
        let padding = plan.padding;
        let info_size = std::mem::size_of::<AtlasAnimationInfo>();
        let info_alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let info_stride = info_size
            .checked_add(info_alignment.saturating_sub(1))
            .map(|size| size / info_alignment * info_alignment)
            .ok_or_else(|| anyhow::anyhow!("terrain animation uniform stride overflow"))?;
        let mut info_bytes = vec![
            0;
            info_stride.checked_mul(plan.sprites.len()).ok_or_else(
                || anyhow::anyhow!("terrain animation uniform bytes overflow")
            )?
        ];
        let mut frame_textures = Vec::new();
        let mut frame_views = Vec::new();
        let mut pending_sprites = Vec::with_capacity(plan.sprites.len());
        for (sprite_index, sprite) in plan.sprites.into_iter().enumerate() {
            validate_sprite_destination(atlas_width, atlas_height, padding, &sprite)?;
            let layer_count = u32::try_from(sprite.frames.len()).map_err(|_| {
                anyhow::anyhow!("animated sprite {} has too many frames", sprite.id)
            })?;
            if layer_count == 0 || layer_count > device.limits().max_texture_array_layers {
                bail!(
                    "animated sprite {} frame layers {} exceed device limit {}",
                    sprite.id,
                    layer_count,
                    device.limits().max_texture_array_layers
                );
            }
            let base = sprite
                .frames
                .first()
                .and_then(|frame| frame.mips.first())
                .ok_or_else(|| anyhow::anyhow!("animated sprite {} has no base mip", sprite.id))?;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bbb-terrain-animation-source-frames"),
                size: wgpu::Extent3d {
                    width: base.width,
                    height: base.height,
                    depth_or_array_layers: layer_count,
                },
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let mut frame_layers = HashMap::new();
            for (layer, frame) in sprite.frames.into_iter().enumerate() {
                if frame.mips.len() as u32 != mip_level_count {
                    bail!(
                        "animated sprite {} frame {} has {} mips, expected {}",
                        sprite.id,
                        frame.index,
                        frame.mips.len(),
                        mip_level_count
                    );
                }
                let base = frame.mips.first().ok_or_else(|| {
                    anyhow::anyhow!("animated sprite {} has no base mip", sprite.id)
                })?;
                if base.width != sprite.content_width || base.height != sprite.content_height {
                    bail!(
                        "animated sprite {} frame {} base mip is {}x{}, expected {}x{}",
                        sprite.id,
                        frame.index,
                        base.width,
                        base.height,
                        sprite.content_width,
                        sprite.content_height
                    );
                }
                for (level, mip) in frame.mips.iter().enumerate() {
                    validate_source_mip(
                        &sprite.id,
                        frame.index,
                        level as u32,
                        sprite.content_width,
                        sprite.content_height,
                        mip,
                    )?;
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: level as u32,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: layer as u32,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &mip.rgba,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(mip.width * 4),
                            rows_per_image: Some(mip.height),
                        },
                        wgpu::Extent3d {
                            width: mip.width,
                            height: mip.height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                if frame_layers.insert(frame.index, layer as u32).is_some() {
                    bail!(
                        "animated sprite {} repeats source frame {}",
                        sprite.id,
                        frame.index
                    );
                }
            }
            validate_animation_schedule_frames(&sprite.id, &sprite.animation, &frame_layers)?;
            let info = animation_info_at_tick(
                sprite.destination,
                sprite.content_width,
                sprite.content_height,
                atlas_width,
                atlas_height,
                padding,
                &sprite.animation,
                &frame_layers,
                0,
            )?;
            write_animation_info(&mut info_bytes, info_stride, sprite_index, info);
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("bbb-terrain-animation-source-frames-view"),
                format: Some(wgpu::TextureFormat::Rgba8Unorm),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                usage: None,
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(mip_level_count),
                base_array_layer: 0,
                array_layer_count: Some(layer_count),
            });
            let view_index = frame_views.len();
            frame_textures.push(texture);
            frame_views.push(view);
            pending_sprites.push((sprite.animation, frame_layers, view_index, info));
        }
        let info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-terrain-animation-info"),
            contents: &info_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let info_binding_size = std::num::NonZeroU64::new(info_size as u64)
            .ok_or_else(|| anyhow::anyhow!("terrain animation uniform is empty"))?;
        let mut sprites = Vec::with_capacity(pending_sprites.len());
        for (sprite_index, (animation, frame_layers, view_index, base_info)) in
            pending_sprites.into_iter().enumerate()
        {
            let uniform_offset = u32::try_from(sprite_index * info_stride)
                .map_err(|_| anyhow::anyhow!("terrain animation uniform offset overflow"))?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bbb-terrain-animation-sprite"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&frame_views[view_index]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &info_buffer,
                            offset: 0,
                            size: Some(info_binding_size),
                        }),
                    },
                ],
            });
            sprites.push(SpriteAnimationGpu {
                animation,
                frame_layers,
                base_info,
                bind_group,
                uniform_offset,
            });
        }
        let (interpolate_bundles, interpolated_draw_calls) =
            create_interpolate_bundles(device, mip_level_count, &interpolate_pipeline, &sprites);
        tracing::info!(
            animated_sprites = sprites.len(),
            interpolated_sprites = interpolated_draw_calls,
            "prepared Vanilla-style GPU terrain atlas animation"
        );

        Ok(Self {
            atlas_width,
            atlas_height,
            mip_level_count,
            blit_pipeline,
            _frame_textures: frame_textures,
            _frame_views: frame_views,
            _sampler: sampler,
            info_buffer,
            info_bytes,
            info_stride,
            interpolate_bundles,
            interpolated_draw_calls,
            sprites,
            pending_tick: None,
        })
    }

    pub(super) fn matches_atlas(&self, atlas: &TerrainAtlasGpu) -> bool {
        (self.atlas_width, self.atlas_height, self.mip_level_count)
            == (atlas.width, atlas.height, atlas.mip_level_count)
    }

    pub(super) fn advance(&mut self, queue: &wgpu::Queue, previous_tick: u64, tick: u64) {
        for (sprite_index, sprite) in self.sprites.iter().enumerate() {
            let Some(current) = sprite.animation.frame_at_tick(tick) else {
                continue;
            };
            let Some(&current_layer) = sprite.frame_layers.get(&current.frame_index) else {
                continue;
            };
            let Some(&next_layer) = sprite.frame_layers.get(&current.next_frame_index) else {
                continue;
            };
            let mut info = sprite.base_info;
            info.current_layer = current_layer;
            info.next_layer = next_layer;
            info.progress_thousandths = vanilla_progress_thousandths(current.frame_progress);
            write_animation_info(&mut self.info_bytes, self.info_stride, sprite_index, info);
        }
        queue.write_buffer(&self.info_buffer, 0, &self.info_bytes);
        self.pending_tick = Some((previous_tick, tick));
    }

    pub(super) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        atlas: &TerrainAtlasGpu,
    ) -> (u64, Option<(u64, u64)>) {
        let Some((previous_tick, tick)) = self.pending_tick.take() else {
            return (0, None);
        };
        if !self.matches_atlas(atlas) {
            return (0, None);
        }
        let mut draw_calls = 0;
        for level in 0..self.mip_level_count {
            let view = atlas.texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("bbb-terrain-animation-atlas-mip"),
                format: Some(wgpu::TextureFormat::Rgba8Unorm),
                dimension: Some(wgpu::TextureViewDimension::D2),
                usage: None,
                aspect: wgpu::TextureAspect::All,
                base_mip_level: level,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-terrain-animation-atlas-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            if let Some(bundle) = self.interpolate_bundles.get(level as usize) {
                pass.execute_bundles(std::iter::once(bundle));
                draw_calls += self.interpolated_draw_calls;
            }
            for sprite in &self.sprites {
                if sprite.animation.interpolate
                    || !sprite.needs_non_interpolated_draw(previous_tick, tick)
                {
                    continue;
                }
                pass.set_pipeline(&self.blit_pipeline);
                pass.set_bind_group(0, &sprite.bind_group, &[sprite.uniform_offset]);
                let first_vertex = level * 8;
                pass.draw(first_vertex..first_vertex + 6, 0..1);
                draw_calls += 1;
            }
        }
        (draw_calls, Some((previous_tick, tick)))
    }

    /// Restores an animation update whose command encoder was discarded before
    /// queue submission (for example after a lost/outdated surface acquire).
    pub(super) fn restore_unsubmitted_tick(&mut self, tick: Option<(u64, u64)>) {
        let Some(tick) = tick else {
            return;
        };
        debug_assert!(self.pending_tick.is_none());
        self.pending_tick = Some(tick);
    }
}

impl SpriteAnimationGpu {
    fn needs_non_interpolated_draw(&self, previous_tick: u64, tick: u64) -> bool {
        let Some(previous) = self.animation.frame_at_tick(previous_tick) else {
            return false;
        };
        let Some(current) = self.animation.frame_at_tick(tick) else {
            return false;
        };
        previous.frame_index != current.frame_index
    }
}

fn create_interpolate_bundles(
    device: &wgpu::Device,
    mip_level_count: u32,
    pipeline: &wgpu::RenderPipeline,
    sprites: &[SpriteAnimationGpu],
) -> (Vec<wgpu::RenderBundle>, u64) {
    let interpolated_draw_calls = sprites
        .iter()
        .filter(|sprite| sprite.animation.interpolate)
        .count() as u64;
    if interpolated_draw_calls == 0 {
        return (Vec::new(), 0);
    }
    let mut bundles = Vec::with_capacity(mip_level_count as usize);
    for level in 0..mip_level_count {
        let mut bundle =
            device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: Some("bbb-terrain-interpolated-animation-bundle"),
                color_formats: &[Some(wgpu::TextureFormat::Rgba8Unorm)],
                depth_stencil: None,
                sample_count: 1,
                multiview: None,
            });
        bundle.set_pipeline(pipeline);
        for sprite in sprites.iter().filter(|sprite| sprite.animation.interpolate) {
            bundle.set_bind_group(0, &sprite.bind_group, &[sprite.uniform_offset]);
            let first_vertex = level * 8;
            bundle.draw(first_vertex..first_vertex + 6, 0..1);
        }
        bundles.push(bundle.finish(&wgpu::RenderBundleDescriptor {
            label: Some("bbb-terrain-interpolated-animation-bundle"),
        }));
    }
    (bundles, interpolated_draw_calls)
}

fn validate_animation_schedule_frames(
    sprite_id: &str,
    animation: &SpriteAnimation,
    frame_layers: &HashMap<u32, u32>,
) -> Result<()> {
    for frame in &animation.frames {
        if !frame_layers.contains_key(&frame.index) {
            bail!(
                "animated sprite {} schedule references missing frame {}",
                sprite_id,
                frame.index
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn animation_info_at_tick(
    destination: AtlasRect,
    content_width: u32,
    content_height: u32,
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    animation: &SpriteAnimation,
    frame_layers: &HashMap<u32, u32>,
    tick: u64,
) -> Result<AtlasAnimationInfo> {
    let current = animation
        .frame_at_tick(tick)
        .ok_or_else(|| anyhow::anyhow!("animated sprite has no frame at tick {tick}"))?;
    let current_layer = frame_layers
        .get(&current.frame_index)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("animated sprite missing current frame layer"))?;
    let next_layer = frame_layers
        .get(&current.next_frame_index)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("animated sprite missing next frame layer"))?;
    Ok(AtlasAnimationInfo {
        destination_x: destination.x,
        destination_y: destination.y,
        content_width,
        content_height,
        atlas_width,
        atlas_height,
        padding,
        current_layer,
        next_layer,
        progress_thousandths: vanilla_progress_thousandths(current.frame_progress),
        _padding_0: 0,
        _padding_1: 0,
    })
}

fn write_animation_info(
    bytes: &mut [u8],
    stride: usize,
    sprite_index: usize,
    info: AtlasAnimationInfo,
) {
    let start = sprite_index * stride;
    let info_bytes = bytemuck::bytes_of(&info);
    bytes[start..start + info_bytes.len()].copy_from_slice(info_bytes);
}

fn vanilla_progress_thousandths(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 1000.0) as u32).min(999)
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-terrain-animation-bind-group-layout"),
        entries: &[
            texture_entry(0),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    }
}

fn validate_sprite_destination(
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    sprite: &AtlasGpuAnimationSprite,
) -> Result<()> {
    let gutter = padding
        .checked_mul(2)
        .ok_or_else(|| anyhow::anyhow!("animated atlas padding overflow"))?;
    let width = sprite.content_width.checked_add(gutter).ok_or_else(|| {
        anyhow::anyhow!("animated sprite {} destination width overflow", sprite.id)
    })?;
    let height = sprite.content_height.checked_add(gutter).ok_or_else(|| {
        anyhow::anyhow!("animated sprite {} destination height overflow", sprite.id)
    })?;
    if sprite.destination.x.saturating_add(width) > atlas_width
        || sprite.destination.y.saturating_add(height) > atlas_height
    {
        bail!("animated sprite {} destination exceeds atlas", sprite.id);
    }
    if sprite.destination.width < width || sprite.destination.height < height {
        bail!(
            "animated sprite {} padded allocation is smaller than its draw destination",
            sprite.id
        );
    }
    if sprite.animation.frames.is_empty() {
        bail!("animated sprite {} has no schedule frames", sprite.id);
    }
    Ok(())
}

fn validate_source_mip(
    sprite_id: &str,
    frame_index: u32,
    level: u32,
    base_width: u32,
    base_height: u32,
    mip: &SpriteMipLevel,
) -> Result<()> {
    let expected_width = base_width.checked_shr(level).unwrap_or(0);
    let expected_height = base_height.checked_shr(level).unwrap_or(0);
    let expected = usize::try_from(mip.width)
        .ok()
        .and_then(|width| {
            usize::try_from(mip.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("animated sprite {sprite_id} mip size overflow"))?;
    if mip.width != expected_width || mip.height != expected_height || mip.rgba.len() != expected {
        bail!(
            "animated sprite {} frame {} mip {} has invalid {}x{} RGBA bytes {}",
            sprite_id,
            frame_index,
            level,
            mip.width,
            mip.height,
            mip.rgba.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bbb_render_types::{
        AtlasGpuAnimationFrame, AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasRect,
        SpriteAnimation, SpriteAnimationFrame, SpriteMipLevel,
    };

    use super::{
        AtlasAnimationInfo, TerrainAtlasAnimationGpu, BLIT_FRAGMENT, INTERPOLATE_FRAGMENT,
        SHADER_SHARED,
    };

    #[test]
    fn shader_reads_vanilla_progress_and_frame_layers_from_dynamic_uniform() {
        assert_eq!(std::mem::size_of::<AtlasAnimationInfo>(), 48);
        assert!(SHADER_SHARED.contains("let position_index = vertex_index & 7u"));
        assert!(SHADER_SHARED.contains("let mip_level = encoded;"));
        assert!(SHADER_SHARED.contains("animation_info.progress_thousandths"));
        assert!(BLIT_FRAGMENT.contains("animation_info.current_layer"));
        assert!(INTERPOLATE_FRAGMENT.contains("animation_info.next_layer"));
        assert!(INTERPOLATE_FRAGMENT.contains("mix(current, next, input.animation_progress)"));
    }

    #[test]
    fn gpu_interpolated_animation_bundle_updates_content_and_replicated_gutter() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            }))
        else {
            return;
        };
        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bbb-terrain-animation-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            }))
        else {
            return;
        };
        let initial = [200, 0, 0, 255].repeat(16);
        let atlas = crate::gpu::create_terrain_atlas_gpu(&device, &queue, 4, 4, &initial)
            .expect("test atlas");
        let plan = AtlasGpuAnimationPlan {
            atlas_width: 4,
            atlas_height: 4,
            padding: 1,
            mip_level: 0,
            sprites: vec![AtlasGpuAnimationSprite {
                id: "minecraft:block/test".to_string(),
                destination: AtlasRect {
                    x: 0,
                    y: 0,
                    width: 3,
                    height: 3,
                },
                content_width: 1,
                content_height: 1,
                animation: SpriteAnimation {
                    frame_count: 2,
                    default_frame_time: 1,
                    interpolate: true,
                    frames: vec![
                        SpriteAnimationFrame { index: 0, time: 1 },
                        SpriteAnimationFrame { index: 1, time: 1 },
                    ],
                },
                frames: vec![
                    AtlasGpuAnimationFrame {
                        index: 0,
                        mips: vec![SpriteMipLevel {
                            width: 1,
                            height: 1,
                            rgba: vec![200, 0, 0, 255],
                        }],
                    },
                    AtlasGpuAnimationFrame {
                        index: 1,
                        mips: vec![SpriteMipLevel {
                            width: 1,
                            height: 1,
                            rgba: vec![0, 20, 220, 255],
                        }],
                    },
                ],
            }],
        };
        let mut animation =
            TerrainAtlasAnimationGpu::new(&device, &queue, plan).expect("GPU animation");
        animation.advance(&queue, 0, 1);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bbb-terrain-animation-test"),
        });
        let rendered = animation.render(&mut encoder, &atlas);
        assert_eq!(rendered, (1, Some((0, 1))));
        animation.restore_unsubmitted_tick(rendered.1);
        assert_eq!(
            animation.render(&mut encoder, &atlas),
            (1, Some((0, 1))),
            "a discarded surface frame must retry the selected animation tick"
        );
        let bytes_per_row = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-terrain-animation-readback"),
            size: u64::from(bytes_per_row * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(4),
                },
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        receiver
            .recv()
            .expect("map callback")
            .expect("map readback");
        let mapped = slice.get_mapped_range().expect("mapped GPU readback");
        for y in 0..4usize {
            for x in 0..4usize {
                let offset = y * bytes_per_row as usize + x * 4;
                let expected = if x < 3 && y < 3 {
                    [0, 20, 220, 255]
                } else {
                    [200, 0, 0, 255]
                };
                assert_eq!(&mapped[offset..offset + 4], expected.as_slice());
            }
        }
        drop(mapped);
        readback.unmap();
    }
}
