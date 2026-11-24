//! Resource-owned model-space cuboids and frame-local part poses. Expansion writes the
//! expanded entity vertex ABI for mixed meshes and selected-pack render shaders.

use std::collections::HashMap;

use glam::{Mat3, Mat4, Vec3};

use crate::frame_buffers::StagedCommands;

use super::{
    geometry::*,
    model::{ModelCubes, ModelCubesKey},
    EntityModelUvRect,
};

mod vertex_pose;
pub(crate) use vertex_pose::{EntityPoseBinding, EntityPosePipelines};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ModelPartDraw {
    cubes: ModelCubes,
    texture_size: [f32; 2],
    pub(super) vertex_start: usize,
    pub(super) vertex_count: usize,
    pub(super) origin: [f32; 3],
    pub(super) transform: Mat4,
    pub(super) uv: EntityModelUvRect,
    pub(super) tint: [f32; 4],
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) pixel_origin: [f32; 2],
    pub(super) size_flags: [f32; 4],
}

impl ModelPartDraw {
    pub(super) fn asset_key(&self) -> (ModelCubesKey, [u32; 2]) {
        (self.cubes.key(), self.texture_size.map(f32::to_bits))
    }
}

pub(super) fn retain_model_part(
    mesh: &mut EntityModelTexturedMesh,
    cubes: ModelCubes,
    texture_size: [f32; 2],
    transform: Mat4,
    uv: EntityModelUvRect,
    tint: [f32; 4],
) {
    let count: usize = cubes
        .as_slice()
        .iter()
        .map(|cube| cube.visible_faces.count_ones() as usize * 4)
        .sum();
    if count == 0 {
        return;
    }
    let part = ModelPartDraw {
        cubes,
        texture_size,
        vertex_start: mesh.vertex_count(),
        vertex_count: count,
        origin: mesh.part_origin.unwrap_or([0.0; 3]),
        transform,
        uv,
        tint,
        light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
        overlay: ENTITY_VERTEX_NO_OVERLAY,
        pixel_origin: [0.0; 2],
        size_flags: [0.0; 4],
    };
    mesh.indices.reserve((count / 4) * 6);
    for start in (part.vertex_start..part.vertex_start + count).step_by(4) {
        let base = start as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh.deferred_vertices += count;
    mesh.cutout_faces += count / 4;
    mesh.parts.push(part);
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LocalVertex {
    position: [f32; 3],
    uv: [f32; 2],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PartJob {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
    tint: [f32; 4],
    uv: [f32; 4],
    light_overlay: [f32; 4],
    sampling: [f32; 4],
    pixel_origin: [f32; 4],
    range: [u32; 4],
}

impl PartJob {
    fn new(part: &ModelPartDraw, range: [u32; 4]) -> Self {
        Self {
            model: part.transform.to_cols_array_2d(),
            normal: part.transform.inverse().transpose().to_cols_array_2d(),
            tint: part.tint,
            uv: [
                part.uv.min[0],
                part.uv.min[1],
                part.uv.max[0] - part.uv.min[0],
                part.uv.max[1] - part.uv.min[1],
            ],
            light_overlay: [
                part.light[0],
                part.light[1],
                part.overlay[0],
                part.overlay[1],
            ],
            sampling: part.size_flags,
            pixel_origin: [
                part.pixel_origin[0],
                part.pixel_origin[1],
                part.origin[0],
                part.origin[1],
            ],
            range: [range[0], range[1], range[2], part.origin[2].to_bits()],
        }
    }

    fn posed(part: &ModelPartDraw) -> Self {
        let c0 = part.transform.x_axis.truncate();
        let c1 = part.transform.y_axis.truncate();
        let c2 = part.transform.z_axis.truncate();
        let n0 = c1.cross(c2);
        let det = c0.dot(n0);
        // Cuboid face normals are signed coordinate axes, so three unit cofactor columns
        // reproduce CPU face normalization for every face, including mirrored cubes.
        let normal = if det == 0.0 {
            [[0.0; 4]; 4]
        } else {
            let sign = det.signum();
            let col = |v: Vec3| (v * sign).normalize_or_zero().extend(0.0).to_array();
            [col(n0), col(c2.cross(c0)), col(c0.cross(c1)), [0.0; 4]]
        };
        Self {
            model: part.transform.to_cols_array_2d(),
            normal,
            tint: part.tint,
            uv: [
                part.uv.min[0],
                part.uv.min[1],
                part.uv.max[0] - part.uv.min[0],
                part.uv.max[1] - part.uv.min[1],
            ],
            light_overlay: [
                part.light[0],
                part.light[1],
                part.overlay[0],
                part.overlay[1],
            ],
            sampling: part.size_flags,
            pixel_origin: [
                part.pixel_origin[0],
                part.pixel_origin[1],
                part.origin[0],
                part.origin[1],
            ],
            range: [0, 0, 0, part.origin[2].to_bits()],
        }
    }

    fn write_vertices(
        &self,
        vertices: &[LocalVertex],
        output: &mut crate::frame_buffers::StagedWriteSlice<'_>,
        base_offset: usize,
    ) {
        let origin = Vec3::new(
            self.pixel_origin[2],
            self.pixel_origin[3],
            f32::from_bits(self.range[3]),
        );
        let model = Mat4::from_cols_array_2d(&self.model);
        let normals = Mat3::from_mat4(Mat4::from_cols_array_2d(&self.normal));
        let [u, v, du, dv] = self.uv;
        let [block, sky, overlay_u, overlay_v] = self.light_overlay;
        let face_bytes = size_of::<[EntityModelTexturedVertex; 4]>();
        for (index, face) in vertices.as_chunks::<4>().0.iter().enumerate() {
            let normal = (normals * Vec3::from_array(face[0].normal))
                .normalize_or_zero()
                .to_array();
            let posed = face.map(|vertex| EntityModelTexturedVertex {
                position: (origin + model.transform_point3(Vec3::from_array(vertex.position)))
                    .to_array(),
                uv: [u + vertex.uv[0] * du, v + vertex.uv[1] * dv],
                tint: self.tint,
                light: [block, sky],
                overlay: [overlay_u, overlay_v],
                normal,
                standalone_local_uv: vertex.uv,
                standalone_pixel_origin: [self.pixel_origin[0], self.pixel_origin[1]],
                standalone_size_flags: self.sampling,
            });
            output.write_at(
                base_offset + index * face_bytes,
                bytemuck::cast_slice(&posed),
            );
        }
    }
}

pub(super) fn uses_direct_vertex_upload(vertices: usize) -> bool {
    const DIRECT_UPLOAD_BYTES: usize = 256 * 1024;
    vertices <= DIRECT_UPLOAD_BYTES / size_of::<EntityModelTexturedVertex>()
}

struct PartAsset {
    start: u32,
    count: u32,
    min: Vec3,
    max: Vec3,
}

pub(crate) struct EntityGeometryResidency {
    assets: HashMap<(ModelCubesKey, [u32; 2]), PartAsset>,
    vertices: Vec<LocalVertex>,
    geometry: Option<wgpu::Buffer>,
    capacity: u64,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    binding_revision: u64,
    geometry_revision: u64,
    jobs: Option<wgpu::Buffer>,
    job_capacity: u64,
}

impl EntityGeometryResidency {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bbb-entity-part-expansion"),
            source: wgpu::ShaderSource::Wgsl(expansion_shader().into()),
        });
        Self::with_pipeline(
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("bbb-entity-part-expansion"),
                layout: None,
                module: &module,
                entry_point: Some("expand"),
                compilation_options: Default::default(),
                cache: None,
            }),
        )
    }

    pub(crate) fn empty_for_resource_reload(&self, _device: &wgpu::Device) -> Self {
        Self::with_pipeline(self.pipeline.clone())
    }

    pub(crate) fn collect_retire(self, buffers: &mut Vec<wgpu::Buffer>) {
        if let Some(geometry) = self.geometry {
            buffers.push(geometry);
        }
        if let Some(jobs) = self.jobs {
            buffers.push(jobs);
        }
    }

    fn with_pipeline(pipeline: wgpu::ComputePipeline) -> Self {
        Self {
            assets: HashMap::new(),
            vertices: Vec::new(),
            geometry: None,
            capacity: 0,
            layout: pipeline.get_bind_group_layout(0),
            pipeline,
            binding_revision: 0,
            geometry_revision: 0,
            jobs: None,
            job_capacity: 0,
        }
    }

    fn asset(&mut self, part: &ModelPartDraw) -> &PartAsset {
        self.assets.entry(part.asset_key()).or_insert_with(|| {
            let mut mesh = EntityModelTexturedMesh::new();
            for cube in part.cubes.as_slice() {
                emit_textured_model_cube(
                    &mut mesh,
                    Mat4::IDENTITY,
                    Mat4::IDENTITY,
                    cube,
                    part.texture_size,
                    EntityModelUvRect {
                        min: [0.0; 2],
                        max: [1.0; 2],
                    },
                    [1.0; 4],
                );
            }
            let start = self.vertices.len() as u32;
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for face in mesh.vertices.as_chunks::<4>().0 {
                let center = [
                    0.25 * (face[0].uv[0] + face[1].uv[0] + face[2].uv[0] + face[3].uv[0]),
                    0.25 * (face[0].uv[1] + face[1].uv[1] + face[2].uv[1] + face[3].uv[1]),
                ];
                for vertex in face {
                    min = min.min(Vec3::from_array(vertex.position));
                    max = max.max(Vec3::from_array(vertex.position));
                    let uv = std::array::from_fn(|axis| {
                        let size = part.texture_size[axis];
                        let delta = center[axis] - vertex.uv[axis];
                        if size > 0.0 {
                            vertex.uv[axis]
                                + delta.signum() * (1.0 / (256.0 * size)).min(delta.abs() * 0.25)
                        } else {
                            vertex.uv[axis]
                        }
                    });
                    self.vertices.push(LocalVertex {
                        position: vertex.position,
                        uv,
                        normal: vertex.normal,
                    });
                }
            }
            PartAsset {
                start,
                count: mesh.vertices.len() as u32,
                min,
                max,
            }
        })
    }

    pub(super) fn part_corners(&self, part: &ModelPartDraw) -> impl Iterator<Item = Vec3> {
        let asset = &self.assets[&part.asset_key()];
        let (min, max) = (asset.min, asset.max);
        let origin = Vec3::from_array(part.origin);
        let transform = part.transform;
        [min.x, max.x].into_iter().flat_map(move |x| {
            [min.y, max.y].into_iter().flat_map(move |y| {
                [min.z, max.z].map(|z| origin + transform.transform_point3(Vec3::new(x, y, z)))
            })
        })
    }

    pub(crate) fn restore_unsubmitted(&self, device: &wgpu::Device, uploads: &mut StagedCommands) {
        if let Some(geometry) = self.geometry.as_ref() {
            uploads.write(device, geometry, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    #[cfg(test)]
    pub(crate) fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    fn upload_geometry(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        old_bytes: usize,
    ) {
        let bytes = bytemuck::cast_slice(&self.vertices);
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            if let Some(old) =
                self.geometry
                    .replace(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("bbb-entity-local-geometry"),
                        size: self.capacity,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }))
            {
                uploads.belt().retire_buffer(old);
            }
            self.binding_revision += 1;
            self.geometry_revision += 1;
            uploads.write(device, self.geometry.as_ref().unwrap(), 0, bytes);
        } else if bytes.len() != old_bytes {
            uploads.write(
                device,
                self.geometry.as_ref().unwrap(),
                old_bytes as u64,
                &bytes[old_bytes..],
            );
        }
    }

    pub(super) fn expand(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        output: &wgpu::Buffer,
        bindings: &mut Option<(u64, wgpu::BindGroup)>,
        parts: &[ModelPartDraw],
        previous: &[ModelPartDraw],
    ) {
        let old_bytes = self.vertices.len() * size_of::<LocalVertex>();
        let mut changed_vertices = 0;
        let mut jobs = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            if previous.get(index) == Some(part) {
                continue;
            }
            let asset = self.asset(part);
            changed_vertices += part.vertex_count;
            jobs.push(PartJob::new(
                part,
                [asset.start, part.vertex_start as u32, asset.count, 0],
            ));
        }
        if jobs.is_empty() {
            return;
        }
        self.upload_geometry(device, uploads, old_bytes);
        let stride = size_of::<EntityModelTexturedVertex>();
        if uses_direct_vertex_upload(changed_vertices) {
            let (encoder, staging) = uploads.record(device);
            for run in jobs.chunk_by(|a, b| a.range[1] + a.range[2] == b.range[1]) {
                let first = run[0].range[1];
                let last = run.last().unwrap().range;
                let size = (last[1] + last[2] - first) as usize * stride;
                let Some(mut destination) =
                    staging.allocate_write(encoder, output, u64::from(first) * stride as u64, size)
                else {
                    continue;
                };
                for job in run {
                    let [src, dst, count, _] = job.range;
                    let offset = (dst - first) as usize * stride;
                    job.write_vertices(
                        &self.vertices[src as usize..(src + count) as usize],
                        &mut destination,
                        offset,
                    );
                }
            }
            return;
        }
        let job_stride = std::mem::size_of::<PartJob>() as u64;
        let max_storage_jobs = (device.limits().max_storage_buffer_binding_size / job_stride)
            .min(usize::MAX as u64) as usize;
        let max_dispatch_y = device.limits().max_compute_workgroups_per_dimension as usize;
        let max_batch_jobs = max_storage_jobs.min(max_dispatch_y).max(1);
        let batch_len = jobs.len().min(max_batch_jobs);
        let needed_bytes = (batch_len as u64) * job_stride;
        if needed_bytes > self.job_capacity {
            let max_capacity_jobs = max_batch_jobs as u64;
            self.job_capacity = (batch_len as u64)
                .next_power_of_two()
                .min(max_capacity_jobs)
                * job_stride;
            if let Some(old) = self
                .jobs
                .replace(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("bbb-entity-part-jobs"),
                    size: self.job_capacity,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }))
            {
                uploads.belt().retire_buffer(old);
            }
            self.binding_revision += 1;
        }
        let (encoder, staging) = uploads.record(device);
        let pipeline = &self.pipeline;
        if bindings
            .as_ref()
            .is_none_or(|(revision, _)| *revision != self.binding_revision)
        {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bbb-entity-part-expansion"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.geometry.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.jobs.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output.as_entire_binding(),
                    },
                ],
            });
            *bindings = Some((self.binding_revision, group));
        }
        let jobs_buffer = self.jobs.as_ref().unwrap();
        let bind_group = &bindings.as_ref().unwrap().1;
        // Chunk uploads and dispatches within both storage binding and workgroup-Y limits.
        for batch in jobs.chunks(max_batch_jobs) {
            let workgroups_x = batch
                .iter()
                .map(|job| job.range[2])
                .max()
                .unwrap_or(0)
                .div_ceil(64);
            if workgroups_x == 0 {
                continue;
            }
            staging.write_bytes(encoder, jobs_buffer, 0, bytemuck::cast_slice(batch));
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bbb-entity-part-expansion"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(workgroups_x, batch.len() as u32, 1);
        }
    }
}

fn expansion_shader() -> String {
    use std::mem::{offset_of, size_of};
    let fields = [
        ("VERTEX_WORDS", size_of::<EntityModelTexturedVertex>()),
        ("POSITION", offset_of!(EntityModelTexturedVertex, position)),
        ("UV", offset_of!(EntityModelTexturedVertex, uv)),
        ("TINT", offset_of!(EntityModelTexturedVertex, tint)),
        (
            "LIGHT_OVERLAY",
            offset_of!(EntityModelTexturedVertex, light),
        ),
        ("NORMAL", offset_of!(EntityModelTexturedVertex, normal)),
        (
            "LOCAL_UV",
            offset_of!(EntityModelTexturedVertex, standalone_local_uv),
        ),
        (
            "PIXEL_ORIGIN",
            offset_of!(EntityModelTexturedVertex, standalone_pixel_origin),
        ),
        (
            "SAMPLING",
            offset_of!(EntityModelTexturedVertex, standalone_size_flags),
        ),
    ];
    let mut source = String::new();
    for (name, offset) in fields {
        source.push_str(&format!(
            "const {name}: u32 = {}u;\n",
            offset / size_of::<f32>()
        ));
    }
    source.push_str(EXPAND);
    source
}

const EXPAND: &str = r#"
struct Part {
    model: mat4x4<f32>, normal: mat4x4<f32>, tint: vec4<f32>, uv: vec4<f32>,
    light_overlay: vec4<f32>, sampling: vec4<f32>, pixel_origin: vec4<f32>, range: vec4<u32>,
}
@group(0) @binding(0) var<storage, read> geometry: array<f32>;
@group(0) @binding(1) var<storage, read> parts: array<Part>;
@group(0) @binding(2) var<storage, read_write> vertices: array<f32>;
@compute @workgroup_size(64)
fn expand(@builtin(global_invocation_id) id: vec3<u32>) {
    let part = parts[id.y];
    if id.x >= part.range.z { return; }
    let src = (part.range.x + id.x) * 8u;
    let dst = (part.range.y + id.x) * VERTEX_WORDS;
    let origin = vec3<f32>(part.pixel_origin.z, part.pixel_origin.w, bitcast<f32>(part.range.w));
    let position = origin + (part.model * vec4<f32>(geometry[src], geometry[src+1u], geometry[src+2u], 1.0)).xyz;
    let uv = vec2<f32>(geometry[src+3u], geometry[src+4u]);
    let n = (part.normal * vec4<f32>(geometry[src+5u], geometry[src+6u], geometry[src+7u], 0.0)).xyz;
    let length_sq = dot(n, n);
    var normal = vec3<f32>(0.0);
    if length_sq > 0.0 && length_sq < 3.402823e38 { normal = n * inverseSqrt(length_sq); }
    vertices[dst+POSITION] = position.x; vertices[dst+POSITION+1u] = position.y; vertices[dst+POSITION+2u] = position.z;
    let atlas_uv = part.uv.xy + uv * part.uv.zw;
    vertices[dst+UV] = atlas_uv.x; vertices[dst+UV+1u] = atlas_uv.y;
    for (var i = 0u; i < 4u; i++) { vertices[dst+TINT+i] = part.tint[i]; vertices[dst+LIGHT_OVERLAY+i] = part.light_overlay[i]; }
    vertices[dst+NORMAL] = normal.x; vertices[dst+NORMAL+1u] = normal.y; vertices[dst+NORMAL+2u] = normal.z;
    vertices[dst+LOCAL_UV] = uv.x; vertices[dst+LOCAL_UV+1u] = uv.y;
    vertices[dst+PIXEL_ORIGIN] = part.pixel_origin.x; vertices[dst+PIXEL_ORIGIN+1u] = part.pixel_origin.y;
    for (var i = 0u; i < 4u; i++) { vertices[dst+SAMPLING+i] = part.sampling[i]; }
}
"#;

#[cfg(test)]
mod tests {
    use super::super::model_layers::{CHEST_NORMAL_TEXTURE_REF, COW_TEMPERATE_TEXTURE_REF};
    use crate::{ChestModelHalf, ChestModelTexture, EntityModelInstance, EntityModelTextureImage};

    #[test]
    fn moving_cuboid_preserves_static_chest_and_resource_reload() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(160, 120) else {
            return;
        };
        let images = |cow_color: [u8; 4]| {
            [
                EntityModelTextureImage::new(
                    COW_TEMPERATE_TEXTURE_REF,
                    cow_color.repeat(
                        (COW_TEMPERATE_TEXTURE_REF.size[0] * COW_TEMPERATE_TEXTURE_REF.size[1])
                            as usize,
                    ),
                ),
                EntityModelTextureImage::new(
                    CHEST_NORMAL_TEXTURE_REF,
                    [16, 255, 16, 255].repeat(
                        (CHEST_NORMAL_TEXTURE_REF.size[0] * CHEST_NORMAL_TEXTURE_REF.size[1])
                            as usize,
                    ),
                ),
            ]
        };
        renderer
            .upload_entity_model_textures(&images([255, 16, 16, 255]))
            .unwrap();
        let camera = crate::RendererCameraFrame {
            pose: Some(crate::CameraPose {
                position: [0.0, 65.0, -4.0],
                eye_height: 0.0,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
            ..Default::default()
        };
        let mut cow = EntityModelInstance::cow(1, [-1.0, 64.0, 2.0], 0.0, false);
        let chest = EntityModelInstance::chest(
            -1,
            [1.0, 64.0, 2.0],
            0.0,
            ChestModelTexture::Normal,
            ChestModelHalf::Single,
        );
        let center = |rgba: &[u8], channel: usize| {
            let mut x_sum = 0usize;
            let mut count = 0usize;
            for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
                if pixel[channel] > 80
                    && (0..3)
                        .filter(|other| *other != channel)
                        .all(|other| u16::from(pixel[channel]) > u16::from(pixel[other]) * 2)
                {
                    count += 1;
                    x_sum += index % 160;
                }
            }
            assert!(count > 20, "colored entity remains visible");
            x_sum as f32 / count as f32
        };
        renderer.replace_scene_frame(camera, vec![cow, chest]);
        let first = renderer.render_offscreen_frame().unwrap();
        let cow_x = center(&first.rgba, 0);
        let chest_x = center(&first.rgba, 1);
        cow.position[0] += 0.75;
        cow.render_state.body_rot = 25.0;
        renderer.replace_scene_frame(camera, vec![cow, chest]);
        let moved = renderer.render_offscreen_frame().unwrap();
        assert!((center(&moved.rgba, 0) - cow_x).abs() > 3.0);
        assert!((center(&moved.rgba, 1) - chest_x).abs() < 1.0);

        let mut herd = vec![cow, chest];
        herd.extend((2..18).map(|id| EntityModelInstance::cow(id, [120.0, 64.0, 2.0], 0.0, false)));
        renderer.replace_scene_frame(camera, herd);
        let crowded = renderer.render_offscreen_frame().unwrap();
        assert!((center(&crowded.rgba, 0) - center(&moved.rgba, 0)).abs() < 1.0);
        assert!((center(&crowded.rgba, 1) - chest_x).abs() < 1.0);
        let mut moved_chest = chest;
        moved_chest.position[0] += 0.75;
        renderer.replace_scene_frame(camera, vec![cow, moved_chest]);
        let after_growth = renderer.render_offscreen_frame().unwrap();
        assert!((center(&after_growth.rgba, 1) - chest_x).abs() > 3.0);

        let mut distant_cow = cow;
        distant_cow.position = [200.0, 64.0, -160.0];
        distant_cow.render_state.scale = 8.0;
        renderer.set_entity_model_instances(vec![distant_cow]);
        renderer.set_camera_pose(None);
        let framed = renderer.render_offscreen_frame().unwrap();
        assert!((center(&framed.rgba, 0) - 80.0).abs() < 8.0);

        renderer.replace_scene_frame(camera, vec![cow, chest]);
        renderer
            .upload_entity_model_textures(&images([16, 16, 255, 255]))
            .unwrap();
        let reloaded = renderer.render_offscreen_frame().unwrap();
        center(&reloaded.rgba, 2);
        center(&reloaded.rgba, 1);
    }
}

#[cfg(test)]
#[path = "retained/pose_pixels.rs"]
mod pose_pixels;
