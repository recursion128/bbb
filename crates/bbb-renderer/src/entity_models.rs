use glam::{EulerRot, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{camera::TerrainBounds, gpu::DEPTH_FORMAT, Renderer};

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MODEL_UNIT_SCALE: f32 = 1.0 / 16.0;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityModelKind {
    Chicken {
        baby: bool,
    },
    Humanoid {
        family: HumanoidModelFamily,
        baby: bool,
    },
    Quadruped {
        family: QuadrupedModelFamily,
        baby: bool,
    },
    Creeper,
    Minecart,
    Boat {
        chest: bool,
    },
    Placeholder {
        name: &'static str,
        bounds: EntityModelBounds,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanoidModelFamily {
    Player,
    Zombie,
    Skeleton,
    Villager,
    Illager,
    ArmorStand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadrupedModelFamily {
    Pig,
    Cow,
    Sheep,
    Horse,
    Wolf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelBounds {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}

impl EntityModelKind {
    pub fn model_key(self) -> &'static str {
        match self {
            Self::Chicken { baby: false } => "chicken",
            Self::Chicken { baby: true } => "chicken_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: false,
            } => "humanoid_player",
            Self::Humanoid {
                family: HumanoidModelFamily::Player,
                baby: true,
            } => "humanoid_player_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: false,
            } => "humanoid_zombie",
            Self::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: true,
            } => "humanoid_zombie_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: false,
            } => "humanoid_skeleton",
            Self::Humanoid {
                family: HumanoidModelFamily::Skeleton,
                baby: true,
            } => "humanoid_skeleton_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: false,
            } => "humanoid_villager",
            Self::Humanoid {
                family: HumanoidModelFamily::Villager,
                baby: true,
            } => "humanoid_villager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: false,
            } => "humanoid_illager",
            Self::Humanoid {
                family: HumanoidModelFamily::Illager,
                baby: true,
            } => "humanoid_illager_baby",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: false,
            } => "humanoid_armor_stand",
            Self::Humanoid {
                family: HumanoidModelFamily::ArmorStand,
                baby: true,
            } => "humanoid_armor_stand_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: false,
            } => "quadruped_pig",
            Self::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: true,
            } => "quadruped_pig_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: false,
            } => "quadruped_cow",
            Self::Quadruped {
                family: QuadrupedModelFamily::Cow,
                baby: true,
            } => "quadruped_cow_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: false,
            } => "quadruped_sheep",
            Self::Quadruped {
                family: QuadrupedModelFamily::Sheep,
                baby: true,
            } => "quadruped_sheep_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: false,
            } => "quadruped_horse",
            Self::Quadruped {
                family: QuadrupedModelFamily::Horse,
                baby: true,
            } => "quadruped_horse_baby",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: false,
            } => "quadruped_wolf",
            Self::Quadruped {
                family: QuadrupedModelFamily::Wolf,
                baby: true,
            } => "quadruped_wolf_baby",
            Self::Creeper => "creeper",
            Self::Minecart => "minecart",
            Self::Boat { chest: false } => "boat",
            Self::Boat { chest: true } => "chest_boat",
            Self::Placeholder { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    pub y_rot: f32,
}

impl EntityModelInstance {
    pub fn new(entity_id: i32, kind: EntityModelKind, position: [f32; 3], y_rot: f32) -> Self {
        Self {
            entity_id,
            kind,
            position,
            y_rot,
        }
    }

    pub fn chicken(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Chicken { baby },
            position,
            y_rot,
        )
    }

    pub fn humanoid(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HumanoidModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Humanoid { family, baby },
            position,
            y_rot,
        )
    }

    pub fn quadruped(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: QuadrupedModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Quadruped { family, baby },
            position,
            y_rot,
        )
    }

    pub fn placeholder(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        name: &'static str,
        width: f32,
        height: f32,
        depth: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Placeholder {
                name,
                bounds: EntityModelBounds {
                    width,
                    height,
                    depth,
                },
            },
            position,
            y_rot,
        )
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
const PLAYER_BLUE: [f32; 4] = [0.22, 0.42, 0.78, 1.0];
const ZOMBIE_GREEN: [f32; 4] = [0.33, 0.62, 0.34, 1.0];
const SKELETON_BONE: [f32; 4] = [0.82, 0.82, 0.72, 1.0];
const VILLAGER_ROBE: [f32; 4] = [0.48, 0.34, 0.23, 1.0];
const ILLAGER_GRAY: [f32; 4] = [0.42, 0.45, 0.48, 1.0];
const ARMOR_STAND_WOOD: [f32; 4] = [0.55, 0.36, 0.19, 1.0];
const PIG_PINK: [f32; 4] = [0.92, 0.55, 0.62, 1.0];
const COW_BROWN: [f32; 4] = [0.38, 0.25, 0.18, 1.0];
const SHEEP_WOOL: [f32; 4] = [0.86, 0.86, 0.80, 1.0];
const HORSE_BROWN: [f32; 4] = [0.44, 0.27, 0.14, 1.0];
const WOLF_GRAY: [f32; 4] = [0.64, 0.66, 0.66, 1.0];
const CREEPER_GREEN: [f32; 4] = [0.24, 0.68, 0.23, 1.0];
const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];
const BOAT_WOOD: [f32; 4] = [0.55, 0.36, 0.18, 1.0];
const PLACEHOLDER_COLOR: [f32; 4] = [0.80, 0.20, 0.72, 1.0];

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
            EntityModelKind::Humanoid { family, baby } => {
                emit_humanoid_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Quadruped { family, baby } => {
                emit_quadruped_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Creeper => emit_creeper_model(&mut mesh, *instance),
            EntityModelKind::Minecart => emit_minecart_model(&mut mesh, *instance),
            EntityModelKind::Boat { chest } => emit_boat_model(&mut mesh, *instance, chest),
            EntityModelKind::Placeholder { bounds, .. } => {
                emit_placeholder_bounds_model(&mut mesh, *instance, bounds)
            }
        }
    }
    mesh
}

fn emit_humanoid_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: HumanoidModelFamily,
    baby: bool,
) {
    let color = humanoid_model_color(family);
    let transform = scaled_model_root_transform(instance, if baby { 0.5 } else { 1.0 });
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform * part_pose_transform(PART_POSE_ZERO),
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color,
        },
    );

    let limb_width = if family == HumanoidModelFamily::Skeleton {
        2.0
    } else {
        4.0
    };
    let arm_half = limb_width / 2.0;
    for (x, min_x) in [(-5.0, -arm_half), (5.0, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 2.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, -2.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }
    for (x, min_x) in [(-1.9, -arm_half), (1.9, -arm_half)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 12.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [min_x, 0.0, -arm_half],
                size: [limb_width, 12.0, limb_width],
                color,
            },
        );
    }

    if matches!(
        family,
        HumanoidModelFamily::Villager | HumanoidModelFamily::Illager
    ) {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(PART_POSE_ZERO),
            ModelCubeDesc {
                min: [-2.0, -2.0, -6.0],
                size: [4.0, 4.0, 2.0],
                color,
            },
        );
    }
}

fn emit_quadruped_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: QuadrupedModelFamily,
    baby: bool,
) {
    let color = quadruped_model_color(family);
    let scale = if baby { 0.5 } else { 1.0 };
    let transform = scaled_model_root_transform(instance, scale);
    let (head, body, leg_size, head_offset, body_offset, leg_x) = match family {
        QuadrupedModelFamily::Pig => (
            ([-4.0, -4.0, -8.0], [8.0, 8.0, 8.0]),
            ([-5.0, -10.0, -7.0], [10.0, 16.0, 8.0]),
            6.0,
            [0.0, 12.0, -6.0],
            [0.0, 11.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Cow => (
            ([-4.0, -4.0, -6.0], [8.0, 8.0, 6.0]),
            ([-6.0, -10.0, -7.0], [12.0, 18.0, 10.0]),
            12.0,
            [0.0, 4.0, -8.0],
            [0.0, 5.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Sheep => (
            ([-3.0, -4.0, -6.0], [6.0, 6.0, 8.0]),
            ([-4.0, -10.0, -7.0], [8.0, 16.0, 6.0]),
            12.0,
            [0.0, 6.0, -8.0],
            [0.0, 5.0, 2.0],
            3.0,
        ),
        QuadrupedModelFamily::Horse => (
            ([-3.0, -4.0, -8.0], [6.0, 5.0, 7.0]),
            ([-5.0, -8.0, -9.0], [10.0, 10.0, 22.0]),
            12.0,
            [0.0, 7.0, -10.0],
            [0.0, 11.0, 2.0],
            4.0,
        ),
        QuadrupedModelFamily::Wolf => (
            ([-3.0, -3.0, -4.0], [6.0, 6.0, 6.0]),
            ([-4.0, -2.0, -3.0], [8.0, 6.0, 9.0]),
            8.0,
            [0.0, 13.5, -7.0],
            [0.0, 14.0, 2.0],
            2.5,
        ),
    };

    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: head_offset,
                rotation: [0.0, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: head.0,
            size: head.1,
            color,
        },
    );
    emit_model_cube(
        mesh,
        transform
            * part_pose_transform(PartPose {
                offset: body_offset,
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            }),
        ModelCubeDesc {
            min: body.0,
            size: body.1,
            color,
        },
    );
    for (x, z) in [(-leg_x, 7.0), (leg_x, 7.0), (-leg_x, -5.0), (leg_x, -5.0)] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [x, 24.0 - leg_size, z],
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, leg_size, 4.0],
                color,
            },
        );
    }
}

fn emit_creeper_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    let transform = entity_model_root_transform(instance);
    for cube in [
        ([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 6.0, 0.0]),
        ([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], [0.0, 6.0, 0.0]),
        ([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [-2.0, 18.0, 4.0]),
        ([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [2.0, 18.0, 4.0]),
        ([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [-2.0, 18.0, -4.0]),
        ([-2.0, 0.0, -2.0], [4.0, 6.0, 4.0], [2.0, 18.0, -4.0]),
    ] {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: cube.2,
                    rotation: [0.0, 0.0, 0.0],
                }),
            ModelCubeDesc {
                min: cube.0,
                size: cube.1,
                color: CREEPER_GREEN,
            },
        );
    }
}

fn emit_minecart_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    let transform = entity_model_root_transform(instance);
    for (min, size, pose) in [
        (
            [-10.0, -8.0, -1.0],
            [20.0, 16.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 0.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [-9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [9.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, -7.0],
                rotation: [0.0, std::f32::consts::PI, 0.0],
            },
        ),
        (
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 7.0],
                rotation: [0.0, 0.0, 0.0],
            },
        ),
    ] {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(pose),
            ModelCubeDesc {
                min,
                size,
                color: MINECART_GRAY,
            },
        );
    }
}

fn emit_boat_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, chest: bool) {
    let transform = entity_model_root_transform(instance);
    for (min, size, pose) in [
        (
            [-14.0, -9.0, -3.0],
            [28.0, 16.0, 3.0],
            PartPose {
                offset: [0.0, 3.0, 1.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
        ),
        (
            [-13.0, -7.0, -1.0],
            [18.0, 6.0, 2.0],
            PartPose {
                offset: [-15.0, 4.0, 4.0],
                rotation: [0.0, std::f32::consts::PI * 1.5, 0.0],
            },
        ),
        (
            [-8.0, -7.0, -1.0],
            [16.0, 6.0, 2.0],
            PartPose {
                offset: [15.0, 4.0, 0.0],
                rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
            },
        ),
        (
            [-14.0, -7.0, -1.0],
            [28.0, 6.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, -9.0],
                rotation: [0.0, std::f32::consts::PI, 0.0],
            },
        ),
        (
            [-14.0, -7.0, -1.0],
            [28.0, 6.0, 2.0],
            PartPose {
                offset: [0.0, 4.0, 9.0],
                rotation: [0.0, 0.0, 0.0],
            },
        ),
    ] {
        emit_model_cube(
            mesh,
            transform * part_pose_transform(pose),
            ModelCubeDesc {
                min,
                size,
                color: BOAT_WOOD,
            },
        );
    }

    if chest {
        emit_model_cube(
            mesh,
            transform
                * part_pose_transform(PartPose {
                    offset: [-2.0, -5.0, -6.0],
                    rotation: [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
                }),
            ModelCubeDesc {
                min: [0.0, 0.0, 0.0],
                size: [12.0, 8.0, 12.0],
                color: BOAT_WOOD,
            },
        );
    }
}

fn emit_placeholder_bounds_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    bounds: EntityModelBounds,
) {
    let width = bounds.width.max(0.0625);
    let height = bounds.height.max(0.0625);
    let depth = bounds.depth.max(0.0625);
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians());
    emit_model_cube_world_units(
        mesh,
        transform,
        [-width * 0.5, 0.0, -depth * 0.5],
        [width, height, depth],
        PLACEHOLDER_COLOR,
    );
}

fn scaled_model_root_transform(instance: EntityModelInstance, scale: f32) -> Mat4 {
    entity_model_root_transform(instance) * Mat4::from_scale(Vec3::splat(scale))
}

fn humanoid_model_color(family: HumanoidModelFamily) -> [f32; 4] {
    match family {
        HumanoidModelFamily::Player => PLAYER_BLUE,
        HumanoidModelFamily::Zombie => ZOMBIE_GREEN,
        HumanoidModelFamily::Skeleton => SKELETON_BONE,
        HumanoidModelFamily::Villager => VILLAGER_ROBE,
        HumanoidModelFamily::Illager => ILLAGER_GRAY,
        HumanoidModelFamily::ArmorStand => ARMOR_STAND_WOOD,
    }
}

fn quadruped_model_color(family: QuadrupedModelFamily) -> [f32; 4] {
    match family {
        QuadrupedModelFamily::Pig => PIG_PINK,
        QuadrupedModelFamily::Cow => COW_BROWN,
        QuadrupedModelFamily::Sheep => SHEEP_WOOL,
        QuadrupedModelFamily::Horse => HORSE_BROWN,
        QuadrupedModelFamily::Wolf => WOLF_GRAY,
    }
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
    emit_model_cube_from_min_max(mesh, transform, min, max, cube.color);
}

fn emit_model_cube_world_units(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
) {
    let min = Vec3::from_array(min);
    let max = min + Vec3::from_array(size);
    emit_model_cube_from_min_max(mesh, transform, min, max, color);
}

fn emit_model_cube_from_min_max(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: Vec3,
    max: Vec3,
    color: [f32; 4],
) {
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
            shade_color(color, shade),
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
    fn humanoid_model_families_emit_deterministic_non_empty_meshes() {
        for family in [
            HumanoidModelFamily::Player,
            HumanoidModelFamily::Zombie,
            HumanoidModelFamily::Skeleton,
            HumanoidModelFamily::Villager,
            HumanoidModelFamily::Illager,
            HumanoidModelFamily::ArmorStand,
        ] {
            let instance = EntityModelInstance::humanoid(1, [0.0, 64.0, 0.0], 0.0, family, false);
            let mesh = entity_model_mesh(&[instance]);
            let repeat = entity_model_mesh(&[instance]);

            assert!(!mesh.vertices.is_empty());
            assert!(!mesh.indices.is_empty());
            assert_eq!(mesh.vertices, repeat.vertices);
            assert_eq!(mesh.indices, repeat.indices);
            let (min, max) = mesh_extents(&mesh);
            assert!(max[0] > min[0]);
            assert!(max[1] > min[1]);
            assert!(max[2] > min[2]);
        }
    }

    #[test]
    fn quadruped_model_families_emit_deterministic_non_empty_meshes() {
        for family in [
            QuadrupedModelFamily::Pig,
            QuadrupedModelFamily::Cow,
            QuadrupedModelFamily::Sheep,
            QuadrupedModelFamily::Horse,
            QuadrupedModelFamily::Wolf,
        ] {
            let instance = EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, family, false);
            let mesh = entity_model_mesh(&[instance]);
            let repeat = entity_model_mesh(&[instance]);

            assert!(!mesh.vertices.is_empty());
            assert!(!mesh.indices.is_empty());
            assert_eq!(mesh.vertices, repeat.vertices);
            assert_eq!(mesh.indices, repeat.indices);
            let (min, max) = mesh_extents(&mesh);
            assert!(max[0] > min[0]);
            assert!(max[1] > min[1]);
            assert!(max[2] > min[2]);
        }
    }

    #[test]
    fn vehicle_and_placeholder_models_emit_sane_bounds() {
        let cases = [
            EntityModelInstance::new(1, EntityModelKind::Minecart, [0.0, 64.0, 0.0], 0.0),
            EntityModelInstance::new(
                2,
                EntityModelKind::Boat { chest: true },
                [3.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::placeholder(
                3,
                [6.0, 64.0, 0.0],
                0.0,
                "todo_test_bounds",
                1.0,
                2.0,
                0.5,
            ),
        ];

        for instance in cases {
            let mesh = entity_model_mesh(&[instance]);
            assert!(!mesh.vertices.is_empty());
            assert!(!mesh.indices.is_empty());
            let (min, max) = mesh_extents(&mesh);
            assert!(max[0] > min[0]);
            assert!(max[1] > min[1]);
            assert!(max[2] > min[2]);
        }
    }

    #[test]
    fn entity_model_kind_exposes_stable_model_keys() {
        assert_eq!(
            EntityModelKind::Chicken { baby: false }.model_key(),
            "chicken"
        );
        assert_eq!(
            EntityModelKind::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: true
            }
            .model_key(),
            "humanoid_zombie_baby"
        );
        assert_eq!(
            EntityModelKind::Placeholder {
                name: "todo_test_bounds",
                bounds: EntityModelBounds {
                    width: 1.0,
                    height: 1.0,
                    depth: 1.0
                }
            }
            .model_key(),
            "todo_test_bounds"
        );
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
