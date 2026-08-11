use anyhow::Result;

use super::{capture_gpu_errors, DEPTH_SAMPLE_FORMAT};

const DEPTH_COPY_COUNT: usize = 6;

const DEPTH_COPY_SHADER: &str = r#"
@group(0) @binding(0)
var source_depth: texture_depth_2d;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) f32 {
    return textureLoad(source_depth, vec2<i32>(input.position.xy), 0);
}
"#;

pub(super) struct PostEffectDepthCopies {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    surfaces: [DepthCopySurface; DEPTH_COPY_COUNT],
    size: [u32; 2],
}

impl PostEffectDepthCopies {
    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        capture_gpu_errors(device, || {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bbb-post-effect-depth-copy-bind-group-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    }],
                });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bbb-post-effect-depth-copy-pipeline-layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("bbb-post-effect-depth-copy-shader"),
                source: wgpu::ShaderSource::Wgsl(DEPTH_COPY_SHADER.into()),
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("bbb-post-effect-depth-copy-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: DEPTH_SAMPLE_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            let size = [width.max(1), height.max(1)];
            let surfaces = create_surfaces(device, size)?;
            Ok(Self {
                bind_group_layout,
                pipeline,
                surfaces,
                size,
            })
        })
    }

    pub(super) fn prepare_resize(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Option<DepthCopyReplacement>> {
        let size = [width.max(1), height.max(1)];
        if size == self.size {
            return Ok(None);
        }
        capture_gpu_errors(device, || {
            create_surfaces(device, size)
                .map(|surfaces| Some(DepthCopyReplacement { surfaces, size }))
        })
    }

    pub(super) fn apply_resize(&mut self, replacement: Option<DepthCopyReplacement>) {
        if let Some(replacement) = replacement {
            self.surfaces = replacement.surfaces;
            self.size = replacement.size;
        }
    }

    pub(super) fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        sources: [&wgpu::TextureView; DEPTH_COPY_COUNT],
    ) -> u64 {
        for (surface, source) in self.surfaces.iter().zip(sources) {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bbb-post-effect-depth-copy-bind-group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                }],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-post-effect-depth-copy-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        DEPTH_COPY_COUNT as u64
    }

    pub(super) fn views(&self) -> [&wgpu::TextureView; DEPTH_COPY_COUNT] {
        std::array::from_fn(|index| &self.surfaces[index].view)
    }
}

struct DepthCopySurface {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

pub(super) struct DepthCopyReplacement {
    surfaces: [DepthCopySurface; DEPTH_COPY_COUNT],
    size: [u32; 2],
}

fn create_surfaces(
    device: &wgpu::Device,
    size: [u32; 2],
) -> Result<[DepthCopySurface; DEPTH_COPY_COUNT]> {
    Ok(std::array::from_fn(|_| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-post-effect-depth-copy-target"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_SAMPLE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        DepthCopySurface {
            _texture: texture,
            view,
        }
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::PostEffectDepthCopies;

    #[test]
    fn depth_copy_preserves_sampled_d24_value_as_r32_float() {
        let Some((device, queue)) = request_test_device() else {
            return;
        };
        let copies = PostEffectDepthCopies::new(&device, 4, 4).unwrap();
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-post-effect-depth-copy-test-source"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-post-effect-depth-copy-test-readback"),
            size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bbb-post-effect-depth-copy-test-encoder"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-post-effect-depth-copy-test-clear"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &source_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.375),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        assert_eq!(
            copies.encode(&device, &mut encoder, std::array::from_fn(|_| &source_view),),
            6
        );
        encoder.copy_texture_to_buffer(
            copies.surfaces[0]._texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        receiver.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range().expect("mapped GPU readback");
        let sampled = f32::from_le_bytes(mapped[..4].try_into().unwrap());
        assert!(
            (sampled - 0.375).abs() <= 1.0 / 16_777_215.0,
            "D24Plus 0.375 must be exposed as an exact scalar sample, got {sampled}"
        );
    }

    fn request_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bbb-post-effect-depth-copy-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            ..Default::default()
        }))
        .ok()
    }
}
