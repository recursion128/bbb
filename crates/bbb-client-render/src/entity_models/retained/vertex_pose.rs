use super::*;
use crate::entity_models::gpu::{
    solid_entity_source, ENTITY_MODEL_POSED_SHADER, ENTITY_MODEL_SURFACE_CULL_MODE,
    ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED, ENTITY_MODEL_SURFACE_NO_CULL_MODE,
    ENTITY_MODEL_SURFACE_OPAQUE_BLEND, ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
};
use crate::pipeline_builder::{depth_stencil_state, RenderPipelineBuilder};

pub(crate) struct EntityPosePipelines {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(in crate::entity_models) storage_limit: u64,
    pub(crate) solid: wgpu::RenderPipeline,
    pub(crate) cutout: wgpu::RenderPipeline,
    pub(crate) cutout_cull: wgpu::RenderPipeline,
}

impl EntityPosePipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        atlas: &wgpu::BindGroupLayout,
        lightmap: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bbb-entity-vertex-pose"),
            entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }),
        });
        let make = |label, shader: &str, cull| {
            RenderPipelineBuilder::new(device, label)
                .shader(label, shader)
                .layout(label, &[atlas, lightmap, &layout])
                .vertex_buffers(&[wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Uint32x2],
                }])
                .color_target(format, ENTITY_MODEL_SURFACE_OPAQUE_BLEND)
                .depth_stencil(depth_stencil_state(
                    crate::gpu::DEPTH_FORMAT,
                    ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
                    ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
                ))
                .cull_mode(cull)
                .build()
        };
        let solid = make(
            "bbb-posed-entity-solid",
            &solid_entity_source(ENTITY_MODEL_POSED_SHADER),
            ENTITY_MODEL_SURFACE_CULL_MODE,
        );
        let cutout = make(
            "bbb-posed-entity-cutout",
            ENTITY_MODEL_POSED_SHADER,
            ENTITY_MODEL_SURFACE_NO_CULL_MODE,
        );
        let cutout_cull = make(
            "bbb-posed-entity-cutout-cull",
            ENTITY_MODEL_POSED_SHADER,
            ENTITY_MODEL_SURFACE_CULL_MODE,
        );
        Self {
            storage_limit: device.limits().max_storage_buffer_binding_size,
            layout,
            solid,
            cutout,
            cutout_cull,
        }
    }
}

pub(crate) struct EntityPoseBinding {
    pub(crate) group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    jobs: Vec<PartJob>,
    geometry_revision: u64,
    geometry_bytes: u64,
}

impl EntityPoseBinding {
    pub(in crate::entity_models) fn fits(parts: usize, limit: u64) -> bool {
        (parts * size_of::<PartJob>()) as u64 <= limit
    }

    pub(crate) fn into_buffer(self) -> wgpu::Buffer {
        self.buffer
    }

    pub(crate) fn stage_all(&self, device: &wgpu::Device, uploads: &mut StagedCommands) {
        uploads.write(device, &self.buffer, 0, bytemuck::cast_slice(&self.jobs));
    }

    #[cfg(test)]
    pub(crate) fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    pub(in crate::entity_models) fn update(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        parts: &[ModelPartDraw],
        previous: &[ModelPartDraw],
    ) {
        const COALESCE_GAP: usize = 32;
        if self.jobs.len() != parts.len() || previous.len() != parts.len() {
            self.jobs.clear();
            self.jobs.extend(parts.iter().map(PartJob::posed));
            self.stage_all(device, uploads);
            return;
        }
        let stride = size_of::<PartJob>();
        let mut index = 0;
        while index < parts.len() {
            if previous[index] == parts[index] {
                index += 1;
                continue;
            }
            let start = index;
            let mut end = index;
            while index < parts.len() {
                if previous[index] != parts[index] {
                    self.jobs[index] = PartJob::posed(&parts[index]);
                    index += 1;
                    end = index;
                } else if index - end < COALESCE_GAP {
                    index += 1;
                } else {
                    break;
                }
            }
            uploads.write(
                device,
                &self.buffer,
                (start * stride) as u64,
                bytemuck::cast_slice(&self.jobs[start..end]),
            );
        }
    }
}

impl EntityGeometryResidency {
    pub(in crate::entity_models) fn vertex_pose(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        layout: &wgpu::BindGroupLayout,
        parts: &[ModelPartDraw],
        limit: u64,
    ) -> Option<(Vec<[u32; 2]>, EntityPoseBinding)> {
        let old_bytes = self.vertices.len() * size_of::<LocalVertex>();
        let mut keys = Vec::with_capacity(
            parts.last().unwrap().vertex_start + parts.last().unwrap().vertex_count,
        );
        let mut end = 0;
        for (index, part) in parts.iter().enumerate() {
            let asset = self.asset(part);
            end = end.max(asset.start + asset.count);
            debug_assert_eq!(keys.len(), part.vertex_start);
            keys.extend((asset.start..asset.start + asset.count).map(|v| [v, index as u32]));
        }
        self.upload_geometry(device, uploads, old_bytes);
        let bytes = u64::from(end) * size_of::<LocalVertex>() as u64;
        if bytes > limit {
            return None;
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-vertex-part-poses"),
            size: (parts.len() * size_of::<PartJob>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Geometry is append-only, so existing vertex indices stay valid across growths,
        // but on WebGPU `retire_buffer` calls `old.destroy()` once the growth frame finishes;
        // `ensure_pose_binding_current` rebinds `group` whenever `geometry_revision` advances.
        let group = Self::create_pose_bind_group(
            device,
            layout,
            self.geometry.as_ref().unwrap(),
            bytes,
            &buffer,
        );
        Some((
            keys,
            EntityPoseBinding {
                group,
                buffer,
                jobs: Vec::new(),
                geometry_revision: self.geometry_revision,
                geometry_bytes: bytes,
            },
        ))
    }

    pub(in crate::entity_models) fn ensure_pose_binding_current(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        binding: &mut EntityPoseBinding,
    ) {
        if binding.geometry_revision == self.geometry_revision {
            return;
        }
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        binding.group = Self::create_pose_bind_group(
            device,
            layout,
            geometry,
            binding.geometry_bytes,
            &binding.buffer,
        );
        binding.geometry_revision = self.geometry_revision;
    }

    fn create_pose_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        geometry: &wgpu::Buffer,
        geometry_bytes: u64,
        pose_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-vertex-part-poses"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: geometry,
                        offset: 0,
                        size: wgpu::BufferSize::new(geometry_bytes),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pose_buffer.as_entire_binding(),
                },
            ],
        })
    }
}
