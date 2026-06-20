use glam::{EulerRot, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{camera::TerrainBounds, gpu::DEPTH_FORMAT, Renderer};

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MODEL_UNIT_SCALE: f32 = 1.0 / 16.0;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityModelKind {
    Chicken { baby: bool },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    pub y_rot: f32,
}

impl EntityModelInstance {
    pub fn chicken(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self {
            entity_id,
            kind: EntityModelKind::Chicken { baby },
            position,
            y_rot,
        }
    }
}

pub(super) struct EntityModelMeshGpu {
    pub(super) instances: Vec<EntityModelInstance>,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
    pub(super) bounds: Option<TerrainBounds>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelVertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 4],
}

struct EntityModelMesh {
    vertices: Vec<EntityModelVertex>,
    indices: Vec<u32>,
    opaque_faces: usize,
}

impl EntityModelMesh {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            opaque_faces: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct ModelPartDesc {
    pose: PartPose,
    cubes: &'static [ModelCubeDesc],
    children: &'static [ModelPartDesc],
}

#[derive(Clone, Copy)]
struct ModelCubeDesc {
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
}

#[derive(Clone, Copy)]
struct PartPose {
    offset: [f32; 3],
    rotation: [f32; 3],
}

const PART_POSE_ZERO: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

const ENTITY_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

const ENTITY_MODEL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const CHICKEN_WHITE: [f32; 4] = [0.94, 0.94, 0.86, 1.0];
const CHICKEN_WING: [f32; 4] = [0.82, 0.82, 0.76, 1.0];
const CHICKEN_BEAK: [f32; 4] = [0.95, 0.62, 0.18, 1.0];
const CHICKEN_RED: [f32; 4] = [0.86, 0.08, 0.08, 1.0];
const CHICKEN_LEG: [f32; 4] = [0.82, 0.48, 0.12, 1.0];

const ADULT_CHICKEN_BEAK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -4.0, -4.0],
    size: [4.0, 2.0, 2.0],
    color: CHICKEN_BEAK,
}];

const ADULT_CHICKEN_RED_THING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -3.0],
    size: [2.0, 2.0, 2.0],
    color: CHICKEN_RED,
}];

const ADULT_CHICKEN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_BEAK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_CHICKEN_RED_THING,
        children: &[],
    },
];

const ADULT_CHICKEN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -6.0, -2.0],
    size: [4.0, 6.0, 3.0],
    color: CHICKEN_WHITE,
}];

const ADULT_CHICKEN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -3.0],
    size: [6.0, 8.0, 6.0],
    color: CHICKEN_WHITE,
}];

const ADULT_CHICKEN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [3.0, 5.0, 3.0],
    color: CHICKEN_LEG,
}];

const ADULT_CHICKEN_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -3.0],
    size: [1.0, 4.0, 6.0],
    color: CHICKEN_WING,
}];

const ADULT_CHICKEN_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -3.0],
    size: [1.0, 4.0, 6.0],
    color: CHICKEN_WING,
}];

const ADULT_CHICKEN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_HEAD,
        children: &ADULT_CHICKEN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 19.0, 1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_RIGHT_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_CHICKEN_LEFT_WING,
        children: &[],
    },
];

const BABY_CHICKEN_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -2.25, -0.75],
        size: [4.0, 4.0, 4.0],
        color: CHICKEN_WHITE,
    },
    ModelCubeDesc {
        min: [-1.0, -0.25, -1.75],
        size: [2.0, 1.0, 1.0],
        color: CHICKEN_BEAK,
    },
];

const BABY_CHICKEN_LEFT_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        color: CHICKEN_LEG,
    },
    ModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        color: CHICKEN_LEG,
    },
];

const BABY_CHICKEN_RIGHT_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-0.5, 0.0, 0.0],
        size: [1.0, 2.0, 0.0],
        color: CHICKEN_LEG,
    },
    ModelCubeDesc {
        min: [-0.5, 2.0, -1.0],
        size: [1.0, 0.0, 1.0],
        color: CHICKEN_LEG,
    },
];

const BABY_CHICKEN_RIGHT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [1.0, 0.0, 2.0],
    color: CHICKEN_WING,
}];

const BABY_CHICKEN_LEFT_WING: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [1.0, 0.0, 2.0],
    color: CHICKEN_WING,
}];

const BABY_CHICKEN_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.25, -1.25],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 22.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_RIGHT_WING,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_CHICKEN_LEFT_WING,
        children: &[],
    },
];

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
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

impl Renderer {
    pub fn set_entity_model_instances(&mut self, instances: Vec<EntityModelInstance>) {
        let instances = sanitize_entity_model_instances(instances);
        if self
            .entity_model_mesh
            .as_ref()
            .map(|mesh| mesh.instances.as_slice())
            == Some(instances.as_slice())
        {
            return;
        }

        self.entity_model_mesh = create_entity_model_mesh_gpu(&self.device, instances);
        self.entity_model_bounds = self.entity_model_mesh.as_ref().and_then(|mesh| mesh.bounds);
        self.update_camera();
    }
}

fn create_entity_model_mesh_gpu(
    device: &wgpu::Device,
    instances: Vec<EntityModelInstance>,
) -> Option<EntityModelMeshGpu> {
    let mesh = entity_model_mesh(&instances);
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
        instances,
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
        bounds,
    })
}

fn sanitize_entity_model_instances(
    instances: Vec<EntityModelInstance>,
) -> Vec<EntityModelInstance> {
    instances
        .into_iter()
        .filter(|instance| {
            instance.y_rot.is_finite()
                && instance
                    .position
                    .iter()
                    .all(|component| component.is_finite())
        })
        .collect()
}

fn entity_model_mesh(instances: &[EntityModelInstance]) -> EntityModelMesh {
    let mut mesh = EntityModelMesh::new();
    for instance in instances {
        match instance.kind {
            EntityModelKind::Chicken { baby } => emit_model_parts(
                &mut mesh,
                if baby {
                    &BABY_CHICKEN_PARTS
                } else {
                    &ADULT_CHICKEN_PARTS
                },
                entity_model_root_transform(*instance),
            ),
        }
    }
    mesh
}

fn emit_model_parts(mesh: &mut EntityModelMesh, parts: &[ModelPartDesc], parent_transform: Mat4) {
    for part in parts {
        emit_model_part(mesh, part, parent_transform);
    }
}

fn emit_model_part(mesh: &mut EntityModelMesh, part: &ModelPartDesc, parent_transform: Mat4) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube(mesh, transform, *cube);
    }
    emit_model_parts(mesh, part.children, transform);
}

fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn part_pose_transform(pose: PartPose) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(pose.offset) * MODEL_UNIT_SCALE)
        * Mat4::from_euler(
            EulerRot::ZYX,
            pose.rotation[2],
            pose.rotation[1],
            pose.rotation[0],
        )
}

fn emit_model_cube(mesh: &mut EntityModelMesh, transform: Mat4, cube: ModelCubeDesc) {
    let min = Vec3::from_array(cube.min) * MODEL_UNIT_SCALE;
    let max = min + Vec3::from_array(cube.size) * MODEL_UNIT_SCALE;
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let faces = [
        ([4, 0, 1, 5], 0.56),
        ([2, 3, 7, 6], 1.0),
        ([0, 3, 2, 1], 0.78),
        ([5, 6, 7, 4], 0.86),
        ([0, 4, 7, 3], 0.68),
        ([1, 2, 6, 5], 0.68),
    ];

    for (face, shade) in faces {
        emit_model_face(
            mesh,
            face.map(|index| transform.transform_point3(corners[index])),
            shade_color(cube.color, shade),
        );
    }
}

fn emit_model_face(mesh: &mut EntityModelMesh, corners: [Vec3; 4], color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(corners.map(|position| EntityModelVertex {
            position: position.to_array(),
            color,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.opaque_faces += 1;
}

fn shade_color(color: [f32; 4], shade: f32) -> [f32; 4] {
    [
        (color[0] * shade).clamp(0.0, 1.0),
        (color[1] * shade).clamp(0.0, 1.0),
        (color[2] * shade).clamp(0.0, 1.0),
        color[3].clamp(0.0, 1.0),
    ]
}

fn entity_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chicken_adult_model_mesh_uses_vanilla_cuboid_parts() {
        let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
            26,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )]);

        assert_eq!(mesh.opaque_faces, 48);
        assert_eq!(mesh.vertices.len(), 192);
        assert_eq!(mesh.indices.len(), 288);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.25, 64.001, -0.25]);
        assert_close3(max, [0.25, 64.9385, 0.5]);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(CHICKEN_RED, 0.78)));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(CHICKEN_BEAK, 0.78)));
    }

    #[test]
    fn chicken_baby_model_mesh_uses_flat_vanilla_baby_parts() {
        let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
            27,
            [0.0, 70.0, 0.0],
            0.0,
            true,
        )]);

        assert_eq!(mesh.opaque_faces, 48);
        assert_eq!(mesh.vertices.len(), 192);
        assert_eq!(mesh.indices.len(), 288);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.1875, 70.001, -0.125]);
        assert_close3(max, [0.1875, 70.376, 0.1875]);
    }

    #[test]
    fn entity_model_root_transform_rotates_instances_by_body_yaw() {
        let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
            26,
            [10.0, 64.0, -3.0],
            90.0,
            false,
        )]);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [9.5, 64.001, -3.25]);
        assert_close3(max, [10.25, 64.9385, -2.75]);
    }

    #[test]
    fn sanitize_entity_model_instances_drops_non_finite_instances() {
        assert_eq!(
            sanitize_entity_model_instances(vec![
                EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false),
                EntityModelInstance::chicken(2, [0.0, f32::NAN, 0.0], 0.0, false),
                EntityModelInstance::chicken(3, [0.0, 0.0, 0.0], f32::INFINITY, false),
            ]),
            vec![EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false)]
        );
    }

    #[test]
    fn entity_model_vertex_layout_matches_shader_inputs() {
        let layout = entity_model_vertex_layout();

        assert_eq!(
            layout.array_stride,
            std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress
        );
        assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES.len(), 2);
        assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[0].shader_location, 0);
        assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[1].shader_location, 1);
    }

    fn mesh_extents(mesh: &EntityModelMesh) -> ([f32; 3], [f32; 3]) {
        let mut vertices = mesh.vertices.iter();
        let first = vertices.next().expect("mesh has vertices").position;
        let mut min = Vec3::from_array(first);
        let mut max = Vec3::from_array(first);
        for vertex in vertices {
            let position = Vec3::from_array(vertex.position);
            min = min.min(position);
            max = max.max(position);
        }
        (min.to_array(), max.to_array())
    }

    fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-4,
                "expected {expected}, got {actual}"
            );
        }
    }
}
