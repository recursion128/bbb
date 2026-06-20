use glam::{EulerRot, Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{camera::TerrainBounds, gpu::DEPTH_FORMAT, Renderer};

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MODEL_UNIT_SCALE: f32 = 1.0 / 16.0;
const MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS: f32 = 24.016;
const VILLAGER_LIKE_SCALE: f32 = 0.9375;
const HUSK_SCALE: f32 = 1.0625;
const WITHER_SKELETON_SCALE: f32 = 1.2;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityModelKind {
    Chicken {
        baby: bool,
    },
    Humanoid {
        family: HumanoidModelFamily,
        baby: bool,
    },
    ArmorStand {
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    },
    Slime {
        size: i32,
    },
    MagmaCube {
        size: i32,
    },
    Zombie {
        baby: bool,
    },
    ZombieVariant {
        family: ZombieVariantModelFamily,
        baby: bool,
    },
    Piglin {
        family: PiglinModelFamily,
        baby: bool,
    },
    Skeleton,
    SkeletonVariant {
        family: SkeletonModelFamily,
    },
    Cow {
        baby: bool,
    },
    Sheep {
        baby: bool,
    },
    Villager {
        baby: bool,
    },
    WanderingTrader,
    Quadruped {
        family: QuadrupedModelFamily,
        baby: bool,
    },
    Creeper,
    Spider,
    Enderman,
    IronGolem,
    SnowGolem,
    Witch,
    Illager {
        family: IllagerModelFamily,
    },
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
pub enum ZombieVariantModelFamily {
    Husk,
    Drowned,
    ZombieVillager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiglinModelFamily {
    Piglin,
    PiglinBrute,
    ZombifiedPiglin,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorStandModelPose {
    pub head: [f32; 3],
    pub body: [f32; 3],
    pub left_arm: [f32; 3],
    pub right_arm: [f32; 3],
    pub left_leg: [f32; 3],
    pub right_leg: [f32; 3],
}

pub const DEFAULT_ARMOR_STAND_MODEL_POSE: ArmorStandModelPose = ArmorStandModelPose {
    head: [0.0, 0.0, 0.0],
    body: [0.0, 0.0, 0.0],
    left_arm: [-10.0, 0.0, -10.0],
    right_arm: [-15.0, 0.0, 10.0],
    left_leg: [-1.0, 0.0, -1.0],
    right_leg: [1.0, 0.0, 1.0],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonModelFamily {
    Stray,
    Parched,
    WitherSkeleton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllagerModelFamily {
    Evoker,
    Illusioner,
    Pillager,
    Vindicator,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityModelTextureRef {
    pub path: &'static str,
    pub size: [u32; 2],
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
            Self::ArmorStand { small: false, .. } => "armor_stand",
            Self::ArmorStand { small: true, .. } => "armor_stand_small",
            Self::Slime { .. } => "slime",
            Self::MagmaCube { .. } => "magma_cube",
            Self::Zombie { baby: false } => "zombie",
            Self::Zombie { baby: true } => "zombie_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => "husk",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => "husk_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => "drowned",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => "drowned_baby",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => "zombie_villager",
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => "zombie_villager_baby",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => "piglin",
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => "piglin_baby",
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => "piglin_brute",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => "zombified_piglin",
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => "zombified_piglin_baby",
            Self::Skeleton => "skeleton",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => "stray",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => "parched",
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => "wither_skeleton",
            Self::Cow { baby: false } => "cow",
            Self::Cow { baby: true } => "cow_baby",
            Self::Sheep { baby: false } => "sheep",
            Self::Sheep { baby: true } => "sheep_baby",
            Self::Villager { baby: false } => "villager",
            Self::Villager { baby: true } => "villager_baby",
            Self::WanderingTrader => "wandering_trader",
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
            Self::Spider => "spider",
            Self::Enderman => "enderman",
            Self::IronGolem => "iron_golem",
            Self::SnowGolem => "snow_golem",
            Self::Witch => "witch",
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => "evoker",
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => "illusioner",
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => "pillager",
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => "vindicator",
            Self::Minecart => "minecart",
            Self::Boat { chest: false } => "boat",
            Self::Boat { chest: true } => "chest_boat",
            Self::Placeholder { name, .. } => name,
        }
    }

    pub fn vanilla_texture_ref(self) -> Option<EntityModelTextureRef> {
        match self {
            Self::ArmorStand { .. } => Some(ARMOR_STAND_TEXTURE_REF),
            Self::Slime { .. } => Some(SLIME_TEXTURE_REF),
            Self::MagmaCube { .. } => Some(MAGMA_CUBE_TEXTURE_REF),
            Self::Zombie { baby: false } => Some(ZOMBIE_TEXTURE_REF),
            Self::Zombie { baby: true } => Some(ZOMBIE_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            } => Some(HUSK_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            } => Some(HUSK_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            } => Some(DROWNED_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            } => Some(DROWNED_BABY_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            } => Some(ZOMBIE_VILLAGER_TEXTURE_REF),
            Self::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            } => Some(ZOMBIE_VILLAGER_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            } => Some(PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            } => Some(PIGLIN_BABY_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                ..
            } => Some(PIGLIN_BRUTE_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            } => Some(ZOMBIFIED_PIGLIN_TEXTURE_REF),
            Self::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            } => Some(ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF),
            Self::Skeleton => Some(SKELETON_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Stray,
            } => Some(STRAY_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::Parched,
            } => Some(PARCHED_TEXTURE_REF),
            Self::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton,
            } => Some(WITHER_SKELETON_TEXTURE_REF),
            Self::Sheep { baby: false } => Some(SHEEP_TEXTURE_REF),
            Self::Sheep { baby: true } => Some(SHEEP_BABY_TEXTURE_REF),
            Self::Villager { baby: false } => Some(VILLAGER_TEXTURE_REF),
            Self::Villager { baby: true } => Some(VILLAGER_BABY_TEXTURE_REF),
            Self::WanderingTrader => Some(WANDERING_TRADER_TEXTURE_REF),
            Self::Creeper => Some(CREEPER_TEXTURE_REF),
            Self::Spider => Some(SPIDER_TEXTURE_REF),
            Self::Enderman => Some(ENDERMAN_TEXTURE_REF),
            Self::IronGolem => Some(IRON_GOLEM_TEXTURE_REF),
            Self::SnowGolem => Some(SNOW_GOLEM_TEXTURE_REF),
            Self::Witch => Some(WITCH_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Evoker,
            } => Some(EVOKER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Illusioner,
            } => Some(ILLUSIONER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Pillager,
            } => Some(PILLAGER_TEXTURE_REF),
            Self::Illager {
                family: IllagerModelFamily::Vindicator,
            } => Some(VINDICATOR_TEXTURE_REF),
            _ => None,
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

    pub fn armor_stand(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            },
            position,
            y_rot,
        )
    }

    pub fn slime(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(entity_id, EntityModelKind::Slime { size }, position, y_rot)
    }

    pub fn magma_cube(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::MagmaCube { size },
            position,
            y_rot,
        )
    }

    pub fn zombie(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Zombie { baby }, position, y_rot)
    }

    pub fn zombie_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: ZombieVariantModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ZombieVariant { family, baby },
            position,
            y_rot,
        )
    }

    pub fn piglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: PiglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Piglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn skeleton(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Skeleton, position, y_rot)
    }

    pub fn skeleton_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: SkeletonModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::SkeletonVariant { family },
            position,
            y_rot,
        )
    }

    pub fn cow(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Cow { baby }, position, y_rot)
    }

    pub fn sheep(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Sheep { baby }, position, y_rot)
    }

    pub fn villager(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Villager { baby },
            position,
            y_rot,
        )
    }

    pub fn wandering_trader(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WanderingTrader, position, y_rot)
    }

    pub fn spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Spider, position, y_rot)
    }

    pub fn enderman(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Enderman, position, y_rot)
    }

    pub fn iron_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::IronGolem, position, y_rot)
    }

    pub fn snow_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::SnowGolem, position, y_rot)
    }

    pub fn witch(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Witch, position, y_rot)
    }

    pub fn illager(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: IllagerModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Illager { family },
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModelPartDesc {
    pose: PartPose,
    cubes: &'static [ModelCubeDesc],
    children: &'static [ModelPartDesc],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModelCubeDesc {
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
const HUSK_TAN: [f32; 4] = [0.60, 0.50, 0.31, 1.0];
const DROWNED_BLUE: [f32; 4] = [0.23, 0.48, 0.55, 1.0];
const ZOMBIE_VILLAGER_ROBE: [f32; 4] = [0.38, 0.55, 0.34, 1.0];
const PIGLIN_SKIN: [f32; 4] = [0.74, 0.44, 0.36, 1.0];
const PIGLIN_BRUTE_SKIN: [f32; 4] = [0.58, 0.35, 0.29, 1.0];
const ZOMBIFIED_PIGLIN_SKIN: [f32; 4] = [0.46, 0.62, 0.42, 1.0];
const SKELETON_BONE: [f32; 4] = [0.82, 0.82, 0.72, 1.0];
const WITHER_SKELETON_DARK: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
const PARCHED_BONE: [f32; 4] = [0.70, 0.62, 0.48, 1.0];
const VILLAGER_ROBE: [f32; 4] = [0.48, 0.34, 0.23, 1.0];
const ILLAGER_GRAY: [f32; 4] = [0.42, 0.45, 0.48, 1.0];
const ARMOR_STAND_WOOD: [f32; 4] = [0.55, 0.36, 0.19, 1.0];
const PIG_PINK: [f32; 4] = [0.92, 0.55, 0.62, 1.0];
const COW_BROWN: [f32; 4] = [0.38, 0.25, 0.18, 1.0];
const SHEEP_WOOL: [f32; 4] = [0.86, 0.86, 0.80, 1.0];
const HORSE_BROWN: [f32; 4] = [0.44, 0.27, 0.14, 1.0];
const WOLF_GRAY: [f32; 4] = [0.64, 0.66, 0.66, 1.0];
const CREEPER_GREEN: [f32; 4] = [0.24, 0.68, 0.23, 1.0];
const SPIDER_DARK: [f32; 4] = [0.16, 0.12, 0.12, 1.0];
const ENDERMAN_DARK: [f32; 4] = [0.08, 0.06, 0.10, 1.0];
const IRON_GOLEM_STONE: [f32; 4] = [0.74, 0.74, 0.68, 1.0];
const SNOW_GOLEM_WHITE: [f32; 4] = [0.90, 0.92, 0.88, 1.0];
const WITCH_ROBE: [f32; 4] = [0.28, 0.17, 0.36, 1.0];
const WITCH_HAT_COLOR: [f32; 4] = [0.16, 0.11, 0.20, 1.0];
const ILLAGER_ROBE: [f32; 4] = [0.38, 0.40, 0.44, 1.0];
const ILLAGER_HAT_COLOR: [f32; 4] = [0.30, 0.31, 0.34, 1.0];
const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];
const BOAT_WOOD: [f32; 4] = [0.55, 0.36, 0.18, 1.0];
const SLIME_GREEN: [f32; 4] = [0.42, 0.82, 0.30, 1.0];
const SLIME_FEATURE_DARK: [f32; 4] = [0.16, 0.28, 0.10, 1.0];
const MAGMA_CUBE_ORANGE: [f32; 4] = [0.92, 0.38, 0.12, 1.0];
const MAGMA_CUBE_CORE: [f32; 4] = [0.98, 0.72, 0.22, 1.0];
const PLACEHOLDER_COLOR: [f32; 4] = [0.80, 0.20, 0.72, 1.0];

const ZOMBIE_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/zombie.png",
    size: [64, 64],
};

const ZOMBIE_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/zombie_baby.png",
    size: [64, 64],
};

const HUSK_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/husk.png",
    size: [64, 64],
};

const HUSK_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/husk_baby.png",
    size: [64, 64],
};

const DROWNED_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/drowned.png",
    size: [64, 64],
};

const DROWNED_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie/drowned_baby.png",
    size: [64, 64],
};

const ZOMBIE_VILLAGER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie_villager/zombie_villager.png",
    size: [64, 64],
};

const ZOMBIE_VILLAGER_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/zombie_villager/zombie_villager_baby.png",
    size: [64, 64],
};

const PIGLIN_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/piglin/piglin.png",
    size: [64, 64],
};

const PIGLIN_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/piglin/piglin_baby.png",
    size: [64, 64],
};

const PIGLIN_BRUTE_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/piglin/piglin_brute.png",
    size: [64, 64],
};

const ZOMBIFIED_PIGLIN_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/piglin/zombified_piglin.png",
    size: [64, 64],
};

const ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/piglin/zombified_piglin_baby.png",
    size: [64, 64],
};

const SKELETON_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/skeleton/skeleton.png",
    size: [64, 32],
};

const STRAY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/skeleton/stray.png",
    size: [64, 32],
};

const PARCHED_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/skeleton/parched.png",
    size: [64, 64],
};

const WITHER_SKELETON_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/skeleton/wither_skeleton.png",
    size: [64, 32],
};

const SHEEP_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/sheep/sheep.png",
    size: [64, 32],
};

const SHEEP_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/sheep/sheep_baby.png",
    size: [64, 32],
};

const VILLAGER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/villager/villager.png",
    size: [64, 64],
};

const VILLAGER_BABY_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/villager/villager_baby.png",
    size: [64, 64],
};

const WANDERING_TRADER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/wandering_trader/wandering_trader.png",
    size: [64, 64],
};

const CREEPER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/creeper/creeper.png",
    size: [64, 32],
};

const SPIDER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/spider/spider.png",
    size: [64, 32],
};

const ENDERMAN_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/enderman/enderman.png",
    size: [64, 32],
};

const IRON_GOLEM_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/iron_golem/iron_golem.png",
    size: [128, 128],
};

const SNOW_GOLEM_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/snow_golem/snow_golem.png",
    size: [64, 64],
};

const WITCH_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/witch/witch.png",
    size: [64, 128],
};

const EVOKER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/evoker.png",
    size: [64, 64],
};

const ILLUSIONER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/illusioner.png",
    size: [64, 64],
};

const PILLAGER_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/pillager.png",
    size: [64, 64],
};

const VINDICATOR_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/illager/vindicator.png",
    size: [64, 64],
};

const ARMOR_STAND_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/armorstand/armorstand.png",
    size: [64, 64],
};

const SLIME_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/slime/slime.png",
    size: [64, 32],
};

const MAGMA_CUBE_TEXTURE_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/entity/slime/magmacube.png",
    size: [64, 64],
};

const ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -7.0, -1.0],
    size: [2.0, 7.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 0.0, -1.5],
    size: [12.0, 3.0, 3.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 11.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 3.0, -1.0],
    size: [2.0, 7.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [1.0, 3.0, -1.0],
    size: [2.0, 7.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 10.0, -1.0],
    size: [8.0, 2.0, 2.0],
    color: ARMOR_STAND_WOOD,
}];

const ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.0, 11.0, -6.0],
    size: [12.0, 1.0, 12.0],
    color: ARMOR_STAND_WOOD,
}];

// Vanilla 26.1 ArmorStandModel.createBodyLayer().
const ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

const SMALL_ARMOR_STAND_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.75, -5.25, -0.75],
    size: [1.5, 5.25, 1.5],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -0.75],
    size: [6.0, 1.5, 1.5],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -0.5],
    size: [1.0, 6.0, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.0, -0.5],
    size: [1.0, 6.0, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, -0.5],
    size: [1.0, 5.5, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_RIGHT_BODY_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 1.5, -0.5],
    size: [1.0, 3.5, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_LEFT_BODY_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.5, 1.5, -0.5],
    size: [1.0, 3.5, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_SHOULDER_STICK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 5.0, -0.5],
    size: [4.0, 1.0, 1.0],
    color: ARMOR_STAND_WOOD,
}];

const SMALL_ARMOR_STAND_BASE_PLATE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 5.5, -3.0],
    size: [6.0, 0.5, 6.0],
    color: ARMOR_STAND_WOOD,
}];

// Vanilla 26.1 ModelLayers.ARMOR_STAND_SMALL applies HumanoidModel.BABY_TRANSFORMER:
// head root parts are translated by y=16 then scaled 0.75; all other root parts
// are translated by y=24 then scaled 0.5.
const SMALL_ARMOR_STAND_PARTS: [ModelPartDesc; 10] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.95, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_RIGHT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_LEFT_BODY_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_SHOULDER_STICK,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SMALL_ARMOR_STAND_BASE_PLATE,
        children: &[],
    },
];

const SLIME_INNER_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 17.0, -3.0],
    size: [6.0, 6.0, 6.0],
    color: SLIME_GREEN,
}];

const SLIME_RIGHT_EYE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, 18.0, -3.5],
    size: [2.0, 2.0, 2.0],
    color: SLIME_FEATURE_DARK,
}];

const SLIME_LEFT_EYE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [1.25, 18.0, -3.5],
    size: [2.0, 2.0, 2.0],
    color: SLIME_FEATURE_DARK,
}];

const SLIME_MOUTH: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 21.0, -3.5],
    size: [1.0, 1.0, 1.0],
    color: SLIME_FEATURE_DARK,
}];

const SLIME_OUTER_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 16.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: SLIME_GREEN,
}];

// Vanilla 26.1 ModelLayers.SLIME plus ModelLayers.SLIME_OUTER.
const SLIME_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_INNER_CUBE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_RIGHT_EYE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_LEFT_EYE,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_MOUTH,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SLIME_OUTER_CUBE,
        children: &[],
    },
];

const MAGMA_CUBE_SEGMENT_0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 16.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 17.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_2: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 18.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_3: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 19.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_4: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 20.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_5: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 21.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_6: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 22.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_SEGMENT_7: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 23.0, -4.0],
    size: [8.0, 1.0, 8.0],
    color: MAGMA_CUBE_ORANGE,
}];

const MAGMA_CUBE_INSIDE_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 18.0, -2.0],
    size: [4.0, 4.0, 4.0],
    color: MAGMA_CUBE_CORE,
}];

// Vanilla 26.1 MagmaCubeModel.createBodyLayer().
const MAGMA_CUBE_PARTS: [ModelPartDesc; 9] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_0,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_1,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_2,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_3,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_4,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_5,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_6,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_SEGMENT_7,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &MAGMA_CUBE_INSIDE_CUBE,
        children: &[],
    },
];

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

const ADULT_ZOMBIE_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ZOMBIE_GREEN,
}];

const ADULT_ZOMBIE_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: ZOMBIE_GREEN,
}];

const ADULT_ZOMBIE_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_ZOMBIE_HAT,
    children: &[],
}];

const ADULT_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

const ADULT_ZOMBIE_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

const ADULT_ZOMBIE_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

const ADULT_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 ModelLayers.ZOMBIE: HumanoidModel.createMesh(CubeDeformation.NONE, 0.0F).
const ADULT_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_HEAD,
        children: &ADULT_ZOMBIE_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_LEG,
        children: &[],
    },
];

const BABY_ZOMBIE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, -1.0],
    size: [4.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

const BABY_ZOMBIE_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.0, -6.25, -3.0],
        size: [6.0, 6.0, 6.0],
        color: ZOMBIE_GREEN,
    },
    // BabyZombieModel bakes CubeDeformation(0.25F) into ModelPart.Cube bounds.
    ModelCubeDesc {
        min: [-3.25, -6.4, -3.25],
        size: [6.5, 6.5, 6.5],
        color: ZOMBIE_GREEN,
    },
];

const BABY_ZOMBIE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_GREEN,
}];

const BABY_ZOMBIE_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 4.0, 2.0],
    color: ZOMBIE_GREEN,
}];

// Vanilla 26.1 BabyZombieModel.createBodyLayer(CubeDeformation.NONE).
const BABY_ZOMBIE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.25, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_LEG,
        children: &[],
    },
];

const ADULT_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -10.0, -4.0],
        size: [8.0, 10.0, 8.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-1.0, -3.0, -6.0],
        size: [2.0, 4.0, 2.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

const ADULT_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -10.5, -4.5],
    size: [9.0, 11.0, 9.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const ADULT_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -6.0],
    size: [16.0, 16.0, 1.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const ADULT_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.05, -0.05, -3.05],
        size: [8.1, 20.1, 6.1],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

const ADULT_ZOMBIE_VILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const ADULT_ZOMBIE_VILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const ADULT_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 0.0, 0.0],
        rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    },
    cubes: &ADULT_ZOMBIE_VILLAGER_HAT_RIM,
    children: &[],
}];

const ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_ZOMBIE_VILLAGER_HAT,
    children: &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN,
}];

// Vanilla 26.1 ZombieVillagerModel.createBodyLayer().
const ADULT_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_HEAD,
        children: &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];

const BABY_ZOMBIE_VILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-2.0, -2.75, -1.5],
        size: [4.0, 5.0, 3.0],
        color: ZOMBIE_VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-2.1, -2.85, -1.6],
        size: [4.2, 6.2, 3.2],
        color: ZOMBIE_VILLAGER_ROBE,
    },
];

const BABY_ZOMBIE_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -3.5],
    size: [8.0, 8.0, 7.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -0.5, -6.0],
    size: [14.0, 1.0, 12.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 5.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: ZOMBIE_VILLAGER_ROBE,
}];

const BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -1.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyZombieVillagerModel.createBodyLayer().
const BABY_ZOMBIE_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_HEAD,
        children: &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 15.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_ZOMBIE_VILLAGER_LEG,
        children: &[],
    },
];

const ADULT_PIGLIN_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-5.0, -8.0, -4.0],
        size: [10.0, 8.0, 8.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-2.0, -4.0, -5.0],
        size: [4.0, 4.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [2.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-3.0, -2.0, -5.0],
        size: [1.0, 2.0, 1.0],
        color: PIGLIN_SKIN,
    },
];

const ADULT_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -2.0],
    size: [1.0, 5.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_LEFT_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.5, -6.0, 0.0],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_6],
        },
        cubes: &ADULT_PIGLIN_RIGHT_EAR,
        children: &[],
    },
];

const ADULT_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_RIGHT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_LEFT_SLEEVE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.25, -2.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_RIGHT_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_RIGHT_SLEEVE,
    children: &[],
}];

const ADULT_PIGLIN_LEFT_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_LEFT_SLEEVE,
    children: &[],
}];

const ADULT_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_PANTS: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.25, -0.25, -2.25],
    size: [4.5, 12.5, 4.5],
    color: PIGLIN_SKIN,
}];

const ADULT_PIGLIN_LEG_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_PIGLIN_PANTS,
    children: &[],
}];

// Vanilla 26.1 AdultPiglinModel.createBodyLayer().
const ADULT_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_HEAD,
        children: &ADULT_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_RIGHT_ARM,
        children: &ADULT_PIGLIN_RIGHT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEFT_ARM,
        children: &ADULT_PIGLIN_LEFT_ARM_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.9, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIGLIN_LEG,
        children: &ADULT_PIGLIN_LEG_CHILDREN,
    },
];

const BABY_PIGLIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -1.0],
    size: [6.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

const BABY_PIGLIN_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.5, -3.0, -4.5],
        size: [3.0, 3.0, 1.0],
        color: PIGLIN_SKIN,
    },
    ModelCubeDesc {
        min: [-4.5, -6.0, -3.5],
        size: [9.0, 6.0, 7.0],
        color: PIGLIN_SKIN,
    },
];

const BABY_PIGLIN_LEFT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

const BABY_PIGLIN_RIGHT_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -3.0, -2.0],
    size: [1.0, 6.0, 4.0],
    color: PIGLIN_SKIN,
}];

const BABY_PIGLIN_HAT_CHILD: ModelPartDesc = ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &[],
    children: &[],
};

const BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.0, 1.75, 0.0],
        rotation: [0.0, 0.0, -0.6109],
    },
    cubes: &BABY_PIGLIN_LEFT_EAR,
    children: &[],
}];

const BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [-1.0, 1.75, 0.0],
        rotation: [0.0, 0.0, 0.6109],
    },
    cubes: &BABY_PIGLIN_RIGHT_EAR,
    children: &[],
}];

const BABY_PIGLIN_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    BABY_PIGLIN_HAT_CHILD,
    ModelPartDesc {
        pose: PartPose {
            offset: [4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.2, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN,
    },
];

const BABY_PIGLIN_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

const BABY_PIGLIN_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 5.0, 3.0],
    color: PIGLIN_SKIN,
}];

const BABY_PIGLIN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 4.0, 3.0],
    color: PIGLIN_SKIN,
}];

// Vanilla 26.1 BabyPiglinModel.createBodyLayer().
const BABY_PIGLIN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.0, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_HEAD,
        children: &BABY_PIGLIN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIGLIN_LEG,
        children: &[],
    },
];

const SKELETON_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: SKELETON_BONE,
}];

const SKELETON_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -8.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SKELETON_BONE,
}];

const SKELETON_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &SKELETON_HAT,
    children: &[],
}];

const SKELETON_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: SKELETON_BONE,
}];

const SKELETON_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: SKELETON_BONE,
}];

const SKELETON_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 12.0, 2.0],
    color: SKELETON_BONE,
}];

// Vanilla 26.1 SkeletonModel.createBodyLayer().
const SKELETON_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_HEAD,
        children: &SKELETON_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &SKELETON_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SKELETON_LEG,
        children: &[],
    },
];

const PARCHED_BODY: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -2.0],
        size: [8.0, 12.0, 4.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.0, 10.0, -2.0],
        size: [8.0, 1.0, 4.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.025, -0.025, -2.025],
        size: [8.05, 12.05, 4.05],
        color: PARCHED_BONE,
    },
];

const PARCHED_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -8.0, -4.0],
        size: [8.0, 8.0, 8.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-4.2, -8.2, -4.2],
        size: [8.4, 8.4, 8.4],
        color: PARCHED_BONE,
    },
];

const PARCHED_EMPTY_HAT: [ModelCubeDesc; 0] = [];

const PARCHED_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &PARCHED_EMPTY_HAT,
    children: &[],
}];

const PARCHED_RIGHT_ARM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.55, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

const PARCHED_LEFT_ARM: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.45, -2.025, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

const PARCHED_LEG: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 12.0, 2.0],
        color: PARCHED_BONE,
    },
    ModelCubeDesc {
        min: [-1.5, 0.0, -1.5],
        size: [3.0, 12.0, 3.0],
        color: PARCHED_BONE,
    },
];

// Vanilla 26.1 SkeletonModel.createSingleModelDualBodyLayer().
const PARCHED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PARCHED_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &PARCHED_HEAD,
        children: &PARCHED_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.5, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.5, 2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &PARCHED_LEG,
        children: &[],
    },
];

const ADULT_PIG_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -8.0],
        size: [8.0, 8.0, 8.0],
        color: PIG_PINK,
    },
    ModelCubeDesc {
        min: [-2.0, 0.0, -9.0],
        size: [4.0, 3.0, 1.0],
        color: PIG_PINK,
    },
];

const ADULT_PIG_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -10.0, -7.0],
    size: [10.0, 16.0, 8.0],
    color: PIG_PINK,
}];

const ADULT_PIG_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 6.0, 4.0],
    color: PIG_PINK,
}];

// Vanilla 26.1 PigModel.createBodyLayer(CubeDeformation.NONE).
const ADULT_PIG_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 12.0, -6.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 11.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 18.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_PIG_LEG,
        children: &[],
    },
];

const BABY_PIG_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -4.5],
    size: [7.0, 6.0, 9.0],
    color: PIG_PINK,
}];

const BABY_PIG_HEAD: [ModelCubeDesc; 2] = [
    // BabyPigModel bakes CubeDeformation into ModelPart.Cube render bounds.
    ModelCubeDesc {
        min: [-3.525, -5.025, -5.025],
        size: [7.05, 6.05, 6.05],
        color: PIG_PINK,
    },
    ModelCubeDesc {
        min: [-1.515, -1.99, -6.015],
        size: [3.03, 2.03, 1.03],
        color: PIG_PINK,
    },
];

const BABY_PIG_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 2.0, 2.0],
    color: PIG_PINK,
}];

// Vanilla 26.1 BabyPigModel.createBodyLayer(CubeDeformation.NONE).
const BABY_PIG_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_PIG_LEG,
        children: &[],
    },
];

const ADULT_COW_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-4.0, -4.0, -6.0],
        size: [8.0, 8.0, 6.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-3.0, 1.0, -7.0],
        size: [6.0, 3.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-5.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [4.0, -5.0, -5.0],
        size: [1.0, 3.0, 1.0],
        color: COW_BROWN,
    },
];

const ADULT_COW_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-6.0, -10.0, -7.0],
        size: [12.0, 18.0, 10.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-2.0, 2.0, -8.0],
        size: [4.0, 6.0, 1.0],
        color: COW_BROWN,
    },
];

const ADULT_COW_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: COW_BROWN,
}];

// Vanilla 26.1 CowModel.createBodyLayer().
const ADULT_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_COW_LEG,
        children: &[],
    },
];

const BABY_COW_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-3.0, -4.569, -4.8333],
        size: [6.0, 6.0, 5.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [3.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-4.0, -5.569, -3.8333],
        size: [1.0, 2.0, 1.0],
        color: COW_BROWN,
    },
    ModelCubeDesc {
        min: [-2.0, -1.569, -5.8333],
        size: [4.0, 3.0, 1.0],
        color: COW_BROWN,
    },
];

const BABY_COW_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -7.0, -1.0],
    size: [8.0, 6.0, 12.0],
    color: COW_BROWN,
}];

const BABY_COW_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 6.0, 3.0],
    color: COW_BROWN,
}];

// Vanilla 26.1 BabyCowModel.createBodyLayer().
const BABY_COW_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.569, -5.1667],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 19.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 18.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.5, 18.0, 3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_COW_LEG,
        children: &[],
    },
];

const ADULT_SHEEP_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -4.0, -6.0],
    size: [6.0, 6.0, 8.0],
    color: SHEEP_WOOL,
}];

const ADULT_SHEEP_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -7.0],
    size: [8.0, 16.0, 6.0],
    color: SHEEP_WOOL,
}];

const ADULT_SHEEP_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: SHEEP_WOOL,
}];

// Vanilla 26.1 SheepModel.createBodyLayer().
const ADULT_SHEEP_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 5.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [3.0, 12.0, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_SHEEP_LEG,
        children: &[],
    },
];

const BABY_SHEEP_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.5, -4.5, -3.5],
    size: [5.0, 5.0, 5.0],
    color: SHEEP_WOOL,
}];

const BABY_SHEEP_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -4.5],
    size: [6.0, 4.0, 9.0],
    color: SHEEP_WOOL,
}];

const BABY_SHEEP_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 5.0, 2.0],
    color: SHEEP_WOOL,
}];

// Vanilla 26.1 BabySheepModel.createBodyLayer().
const BABY_SHEEP_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.0, 0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.5, -2.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 19.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 19.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_SHEEP_LEG,
        children: &[],
    },
];

const ADULT_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.51, -10.51, -4.51],
    size: [9.02, 11.02, 9.02],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-8.0, -8.0, -6.0],
    size: [16.0, 16.0, 1.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: VILLAGER_ROBE,
    },
];

const ADULT_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: VILLAGER_ROBE,
}];

const ADULT_VILLAGER_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 0.0, 0.0],
        rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    },
    cubes: &ADULT_VILLAGER_HAT_RIM,
    children: &[],
}];

const ADULT_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HAT,
        children: &ADULT_VILLAGER_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_NOSE,
        children: &[],
    },
];

const ADULT_VILLAGER_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_VILLAGER_JACKET,
    children: &[],
}];

// Vanilla 26.1 VillagerModel.createBodyModel(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
const ADULT_VILLAGER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_HEAD,
        children: &ADULT_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_VILLAGER_BODY,
        children: &ADULT_VILLAGER_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_VILLAGER_LEG,
        children: &[],
    },
];

const BABY_VILLAGER_RIGHT_HAND: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-1.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [5.0, -2.4925, -1.8401],
        size: [2.0, 4.0, 2.0],
        color: VILLAGER_ROBE,
    },
];

const BABY_VILLAGER_MIDDLE_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -0.9924, -0.9825],
    size: [4.0, 2.0, 2.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -0.5, -1.0],
    size: [2.0, 3.0, 2.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -3.5],
    size: [8.0, 8.0, 7.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.3, -4.3, -3.8],
    size: [8.6, 8.6, 7.6],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_HAT_RIM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-7.0, -0.5, -6.0],
    size: [14.0, 1.0, 12.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.75, -1.5],
    size: [4.0, 5.0, 3.0],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_BB_MAIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.7, -8.2, -1.7],
    size: [4.4, 6.4, 3.4],
    color: VILLAGER_ROBE,
}];

const BABY_VILLAGER_ARMS_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-3.0, 1.4025, -0.9599],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_RIGHT_HAND,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.9024, -1.8175],
            rotation: [-1.0472, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_MIDDLE_ARM,
        children: &[],
    },
];

const BABY_VILLAGER_HEAD_CHILDREN: [ModelPartDesc; 3] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HAT_RIM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_NOSE,
        children: &[],
    },
];

// Vanilla 26.1 BabyVillagerModel.createBodyModel().
const BABY_VILLAGER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 17.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_VILLAGER_ARMS_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 21.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_HEAD,
        children: &BABY_VILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_VILLAGER_BB_MAIN,
        children: &[],
    },
];

const CREEPER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: CREEPER_GREEN,
}];

const CREEPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: CREEPER_GREEN,
}];

const CREEPER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 6.0, 4.0],
    color: CREEPER_GREEN,
}];

// Vanilla 26.1 CreeperModel.createBodyLayer(CubeDeformation.NONE).
const CREEPER_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 6.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 18.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &CREEPER_LEG,
        children: &[],
    },
];

const SPIDER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -4.0, -8.0],
    size: [8.0, 8.0, 8.0],
    color: SPIDER_DARK,
}];

const SPIDER_BODY_0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -3.0],
    size: [6.0, 6.0, 6.0],
    color: SPIDER_DARK,
}];

const SPIDER_BODY_1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -4.0, -6.0],
    size: [10.0, 8.0, 12.0],
    color: SPIDER_DARK,
}];

const SPIDER_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-15.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

const SPIDER_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -1.0],
    size: [16.0, 2.0, 2.0],
    color: SPIDER_DARK,
}];

// Vanilla 26.1 SpiderModel.createSpiderBodyLayer().
const SPIDER_PARTS: [ModelPartDesc; 11] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_0,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 15.0, 9.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SPIDER_BODY_1,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 2.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 2.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 1.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 1.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 15.0, -1.0],
            rotation: [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [4.0, 15.0, -1.0],
            rotation: [
                0.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
        },
        cubes: &SPIDER_LEFT_LEG,
        children: &[],
    },
];

const ENDERMAN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -8.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: ENDERMAN_DARK,
}];

const ENDERMAN_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: ENDERMAN_DARK,
}];

const ENDERMAN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -2.0],
    size: [8.0, 12.0, 4.0],
    color: ENDERMAN_DARK,
}];

const ENDERMAN_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

const ENDERMAN_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 30.0, 2.0],
    color: ENDERMAN_DARK,
}];

const ENDERMAN_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ENDERMAN_HAT,
    children: &[],
}];

// Vanilla 26.1 EndermanModel.createBodyLayer().
const ENDERMAN_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_HEAD,
        children: &ENDERMAN_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, -12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMAN_LEG,
        children: &[],
    },
];

const IRON_GOLEM_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, -12.0, -5.5],
        size: [8.0, 10.0, 8.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-1.0, -5.0, -7.5],
        size: [2.0, 4.0, 2.0],
        color: IRON_GOLEM_STONE,
    },
];

const IRON_GOLEM_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-9.0, -2.0, -6.0],
        size: [18.0, 12.0, 11.0],
        color: IRON_GOLEM_STONE,
    },
    ModelCubeDesc {
        min: [-5.0, 9.5, -3.5],
        size: [10.0, 6.0, 7.0],
        color: IRON_GOLEM_STONE,
    },
];

const IRON_GOLEM_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-13.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

const IRON_GOLEM_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [9.0, -2.5, -3.0],
    size: [4.0, 30.0, 6.0],
    color: IRON_GOLEM_STONE,
}];

const IRON_GOLEM_RIGHT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

const IRON_GOLEM_LEFT_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -3.0, -3.0],
    size: [6.0, 16.0, 5.0],
    color: IRON_GOLEM_STONE,
}];

// Vanilla 26.1 IronGolemModel.createBodyLayer().
const IRON_GOLEM_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, -2.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-4.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_RIGHT_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 11.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &IRON_GOLEM_LEFT_LEG,
        children: &[],
    },
];

const SNOW_GOLEM_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.5, -7.5, -3.5],
    size: [7.0, 7.0, 7.0],
    color: SNOW_GOLEM_WHITE,
}];

const SNOW_GOLEM_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.5, -0.5],
    size: [11.0, 1.0, 1.0],
    color: SNOW_GOLEM_WHITE,
}];

const SNOW_GOLEM_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -9.5, -4.5],
    size: [9.0, 9.0, 9.0],
    color: SNOW_GOLEM_WHITE,
}];

const SNOW_GOLEM_LOWER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.5, -11.5, -5.5],
    size: [11.0, 11.0, 11.0],
    color: SNOW_GOLEM_WHITE,
}];

// Vanilla 26.1 SnowGolemModel.createBodyLayer().
const SNOW_GOLEM_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 4.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [5.0, 6.0, 1.0],
            rotation: [0.0, 0.0, 1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, 6.0, -1.0],
            rotation: [0.0, std::f32::consts::PI, -1.0],
        },
        cubes: &SNOW_GOLEM_ARM,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 13.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_UPPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SNOW_GOLEM_LOWER_BODY,
        children: &[],
    },
];

const WITCH_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: WITCH_ROBE,
}];

const WITCH_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [10.0, 2.0, 10.0],
    color: WITCH_HAT_COLOR,
}];

const WITCH_HAT_2: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [7.0, 4.0, 7.0],
    color: WITCH_HAT_COLOR,
}];

const WITCH_HAT_3: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [4.0, 4.0, 4.0],
    color: WITCH_HAT_COLOR,
}];

const WITCH_HAT_4: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.25, -0.25, -0.25],
    size: [1.5, 2.5, 1.5],
    color: WITCH_HAT_COLOR,
}];

const WITCH_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: WITCH_ROBE,
}];

const WITCH_MOLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.25, 3.25, -6.5],
    size: [0.5, 0.5, 0.5],
    color: WITCH_ROBE,
}];

const WITCH_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, 0.0, -3.0],
    size: [8.0, 12.0, 6.0],
    color: WITCH_ROBE,
}];

const WITCH_JACKET: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.5, -0.5, -3.5],
    size: [9.0, 21.0, 7.0],
    color: WITCH_ROBE,
}];

const WITCH_ARMS: [ModelCubeDesc; 3] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [4.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: WITCH_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: WITCH_ROBE,
    },
];

const WITCH_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: WITCH_ROBE,
}];

const WITCH_HAT_3_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -2.0, 2.0],
        rotation: [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
    },
    cubes: &WITCH_HAT_4,
    children: &[],
}];

const WITCH_HAT_2_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.10471976, 0.0, 0.05235988],
    },
    cubes: &WITCH_HAT_3,
    children: &WITCH_HAT_3_CHILDREN,
}];

const WITCH_HAT_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [1.75, -4.0, 2.0],
        rotation: [-0.05235988, 0.0, 0.02617994],
    },
    cubes: &WITCH_HAT_2,
    children: &WITCH_HAT_2_CHILDREN,
}];

const WITCH_NOSE_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &WITCH_MOLE,
    children: &[],
}];

const WITCH_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-5.0, -10.03125, -5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_HAT,
        children: &WITCH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_NOSE,
        children: &WITCH_NOSE_CHILDREN,
    },
];

const WITCH_BODY_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &WITCH_JACKET,
    children: &[],
}];

// Vanilla 26.1 WitchModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
const WITCH_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_HEAD,
        children: &WITCH_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &WITCH_BODY,
        children: &WITCH_BODY_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 3.0, -1.0],
            rotation: [-0.75, 0.0, 0.0],
        },
        cubes: &WITCH_ARMS,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &WITCH_LEG,
        children: &[],
    },
];

const ILLAGER_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -10.0, -4.0],
    size: [8.0, 10.0, 8.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_HAT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.45, -10.45, -4.45],
    size: [8.9, 12.9, 8.9],
    color: ILLAGER_HAT_COLOR,
}];

const ILLAGER_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -6.0],
    size: [2.0, 4.0, 2.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_BODY: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-4.0, 0.0, -3.0],
        size: [8.0, 12.0, 6.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.5, -0.5, -3.5],
        size: [9.0, 21.0, 7.0],
        color: ILLAGER_ROBE,
    },
];

const ILLAGER_CROSSED_ARMS: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-8.0, -2.0, -2.0],
        size: [4.0, 8.0, 4.0],
        color: ILLAGER_ROBE,
    },
    ModelCubeDesc {
        min: [-4.0, 2.0, -2.0],
        size: [8.0, 4.0, 4.0],
        color: ILLAGER_ROBE,
    },
];

const ILLAGER_LEFT_SHOULDER: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [4.0, -2.0, -2.0],
    size: [4.0, 8.0, 4.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_RIGHT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_LEFT_ARM: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -2.0],
    size: [4.0, 12.0, 4.0],
    color: ILLAGER_ROBE,
}];

const ILLAGER_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_NOSE,
    children: &[],
}];

const ILLAGER_HEAD_WITH_HAT_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HAT,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -2.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_NOSE,
        children: &[],
    },
];

const ILLAGER_CROSSED_ARM_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ILLAGER_LEFT_SHOULDER,
    children: &[],
}];

const ILLAGER_CROSSED_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [0.0, 3.0, -1.0],
        rotation: [-0.75, 0.0, 0.0],
    },
    cubes: &ILLAGER_CROSSED_ARMS,
    children: &ILLAGER_CROSSED_ARM_CHILDREN,
};

const ILLAGER_RIGHT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_RIGHT_ARM,
    children: &[],
};

const ILLAGER_LEFT_ARM_PART: ModelPartDesc = ModelPartDesc {
    pose: PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    },
    cubes: &ILLAGER_LEFT_ARM,
    children: &[],
};

// Vanilla 26.1 IllagerModel.createBodyLayer(), with LayerDefinitions'
// MeshTransformer.scaling(0.9375F) applied by the emitter root transform.
const ILLAGER_SHARED_CROSSED_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
];

const ILLAGER_SHARED_UNCROSSED_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ILLAGER_RIGHT_ARM_PART,
    ILLAGER_LEFT_ARM_PART,
];

const ILLAGER_ILLUSIONER_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_HEAD,
        children: &ILLAGER_HEAD_WITH_HAT_CHILDREN,
    },
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ILLAGER_BODY,
        children: &[],
    },
    ILLAGER_CROSSED_ARM_PART,
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ILLAGER_LEG,
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
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            } => emit_armor_stand_model(
                &mut mesh,
                *instance,
                small,
                show_arms,
                show_base_plate,
                pose,
            ),
            EntityModelKind::Slime { size } => emit_slime_model(&mut mesh, *instance, size),
            EntityModelKind::MagmaCube { size } => {
                emit_magma_cube_model(&mut mesh, *instance, size)
            }
            EntityModelKind::Zombie { baby } => emit_zombie_model(&mut mesh, *instance, baby),
            EntityModelKind::ZombieVariant { family, baby } => {
                emit_zombie_variant_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Piglin { family, baby } => {
                emit_piglin_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Skeleton => emit_skeleton_model(&mut mesh, *instance),
            EntityModelKind::SkeletonVariant { family } => {
                emit_skeleton_variant_model(&mut mesh, *instance, family)
            }
            EntityModelKind::Cow { baby } => emit_cow_model(&mut mesh, *instance, baby),
            EntityModelKind::Sheep { baby } => emit_sheep_model(&mut mesh, *instance, baby),
            EntityModelKind::Villager { baby } => emit_villager_model(&mut mesh, *instance, baby),
            EntityModelKind::WanderingTrader => emit_wandering_trader_model(&mut mesh, *instance),
            EntityModelKind::Quadruped { family, baby } => {
                emit_quadruped_model(&mut mesh, *instance, family, baby)
            }
            EntityModelKind::Creeper => emit_creeper_model(&mut mesh, *instance),
            EntityModelKind::Spider => emit_spider_model(&mut mesh, *instance),
            EntityModelKind::Enderman => emit_enderman_model(&mut mesh, *instance),
            EntityModelKind::IronGolem => emit_iron_golem_model(&mut mesh, *instance),
            EntityModelKind::SnowGolem => emit_snow_golem_model(&mut mesh, *instance),
            EntityModelKind::Witch => emit_witch_model(&mut mesh, *instance),
            EntityModelKind::Illager { family } => emit_illager_model(&mut mesh, *instance, family),
            EntityModelKind::Minecart => emit_minecart_model(&mut mesh, *instance),
            EntityModelKind::Boat { chest } => emit_boat_model(&mut mesh, *instance, chest),
            EntityModelKind::Placeholder { bounds, .. } => {
                emit_placeholder_bounds_model(&mut mesh, *instance, bounds)
            }
        }
    }
    mesh
}

fn emit_armor_stand_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
) {
    let parts = if small {
        &SMALL_ARMOR_STAND_PARTS
    } else {
        &ARMOR_STAND_PARTS
    };
    let transform = entity_model_root_transform(instance);
    emit_armor_stand_part(mesh, transform, &parts[0], degrees_to_radians3(pose.head));
    emit_armor_stand_part(mesh, transform, &parts[1], degrees_to_radians3(pose.body));
    if show_arms {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[2],
            degrees_to_radians3(pose.right_arm),
        );
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[3],
            degrees_to_radians3(pose.left_arm),
        );
    }
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[4],
        degrees_to_radians3(pose.right_leg),
    );
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[5],
        degrees_to_radians3(pose.left_leg),
    );
    emit_armor_stand_part(mesh, transform, &parts[6], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[7], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[8], degrees_to_radians3(pose.body));
    if show_base_plate {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[9],
            [0.0, -instance.y_rot.to_radians(), 0.0],
        );
    }
}

fn emit_armor_stand_part(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    part: &ModelPartDesc,
    rotation: [f32; 3],
) {
    emit_model_cubes_at_pose(
        mesh,
        transform,
        PartPose {
            offset: part.pose.offset,
            rotation,
        },
        part.cubes,
    );
}

fn emit_slime_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    let size = size as f32;
    let transform = living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(0.999))
            * Mat4::from_translation(Vec3::new(0.0, 0.001, 0.0))
            * Mat4::from_scale(Vec3::splat(size)),
    );
    emit_model_parts(mesh, &SLIME_PARTS, transform);
}

fn emit_magma_cube_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, size: i32) {
    let transform = living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(size as f32)),
    );
    emit_model_parts(mesh, &MAGMA_CUBE_PARTS, transform);
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

fn emit_zombie_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_ZOMBIE_PARTS
        } else {
            &ADULT_ZOMBIE_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_zombie_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: ZombieVariantModelFamily,
    baby: bool,
) {
    match (family, baby) {
        (ZombieVariantModelFamily::Husk, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_PARTS,
            mesh_transformer_scaled_model_root_transform(instance, HUSK_SCALE),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Husk, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            HUSK_TAN,
        ),
        (ZombieVariantModelFamily::Drowned, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::Drowned, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_PARTS,
            entity_model_root_transform(instance),
            DROWNED_BLUE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, false) => emit_model_parts_with_color(
            mesh,
            &ADULT_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
        (ZombieVariantModelFamily::ZombieVillager, true) => emit_model_parts_with_color(
            mesh,
            &BABY_ZOMBIE_VILLAGER_PARTS,
            entity_model_root_transform(instance),
            ZOMBIE_VILLAGER_ROBE,
        ),
    }
}

fn emit_piglin_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: PiglinModelFamily,
    baby: bool,
) {
    let parts = if baby && family != PiglinModelFamily::PiglinBrute {
        &BABY_PIGLIN_PARTS
    } else {
        &ADULT_PIGLIN_PARTS
    };
    emit_model_parts_with_color(
        mesh,
        parts,
        entity_model_root_transform(instance),
        piglin_model_color(family),
    );
}

fn emit_skeleton_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &SKELETON_PARTS, entity_model_root_transform(instance));
}

fn emit_skeleton_variant_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: SkeletonModelFamily,
) {
    match family {
        SkeletonModelFamily::Stray => {
            emit_model_parts(mesh, &SKELETON_PARTS, entity_model_root_transform(instance));
        }
        SkeletonModelFamily::Parched => {
            emit_model_parts(mesh, &PARCHED_PARTS, entity_model_root_transform(instance));
        }
        SkeletonModelFamily::WitherSkeleton => emit_model_parts_with_color(
            mesh,
            &SKELETON_PARTS,
            mesh_transformer_scaled_model_root_transform(instance, WITHER_SKELETON_SCALE),
            WITHER_SKELETON_DARK,
        ),
    }
}

fn emit_cow_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_COW_PARTS
        } else {
            &ADULT_COW_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_sheep_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_SHEEP_PARTS
        } else {
            &ADULT_SHEEP_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_villager_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    if baby {
        emit_model_parts(
            mesh,
            &BABY_VILLAGER_PARTS,
            entity_model_root_transform(instance),
        );
    } else {
        emit_model_parts(
            mesh,
            &ADULT_VILLAGER_PARTS,
            villager_adult_model_root_transform(instance),
        );
    }
}

fn emit_wandering_trader_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &ADULT_VILLAGER_PARTS,
        villager_adult_model_root_transform(instance),
    );
}

fn emit_witch_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &WITCH_PARTS,
        villager_adult_model_root_transform(instance),
    );
}

fn emit_illager_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: IllagerModelFamily,
) {
    emit_model_parts(
        mesh,
        illager_model_parts(family),
        villager_adult_model_root_transform(instance),
    );
}

fn illager_model_parts(family: IllagerModelFamily) -> &'static [ModelPartDesc] {
    match family {
        IllagerModelFamily::Evoker | IllagerModelFamily::Vindicator => {
            &ILLAGER_SHARED_CROSSED_PARTS
        }
        IllagerModelFamily::Illusioner => &ILLAGER_ILLUSIONER_PARTS,
        IllagerModelFamily::Pillager => &ILLAGER_SHARED_UNCROSSED_PARTS,
    }
}

fn emit_quadruped_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: QuadrupedModelFamily,
    baby: bool,
) {
    if family == QuadrupedModelFamily::Pig {
        emit_pig_model(mesh, instance, baby);
        return;
    }

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

fn emit_pig_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance, baby: bool) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_PIG_PARTS
        } else {
            &ADULT_PIG_PARTS
        },
        entity_model_root_transform(instance),
    );
}

fn emit_creeper_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &CREEPER_PARTS, entity_model_root_transform(instance));
}

fn emit_spider_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &SPIDER_PARTS, entity_model_root_transform(instance));
}

fn emit_enderman_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(mesh, &ENDERMAN_PARTS, entity_model_root_transform(instance));
}

fn emit_iron_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &IRON_GOLEM_PARTS,
        entity_model_root_transform(instance),
    );
}

fn emit_snow_golem_model(mesh: &mut EntityModelMesh, instance: EntityModelInstance) {
    emit_model_parts(
        mesh,
        &SNOW_GOLEM_PARTS,
        entity_model_root_transform(instance),
    );
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

fn mesh_transformer_scaled_model_root_transform(instance: EntityModelInstance, scale: f32) -> Mat4 {
    entity_model_root_transform(instance)
        * part_pose_transform(PartPose {
            offset: [
                0.0,
                MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - scale),
                0.0,
            ],
            rotation: [0.0, 0.0, 0.0],
        })
        * Mat4::from_scale(Vec3::splat(scale))
}

fn villager_adult_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, VILLAGER_LIKE_SCALE)
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

fn piglin_model_color(family: PiglinModelFamily) -> [f32; 4] {
    match family {
        PiglinModelFamily::Piglin => PIGLIN_SKIN,
        PiglinModelFamily::PiglinBrute => PIGLIN_BRUTE_SKIN,
        PiglinModelFamily::ZombifiedPiglin => ZOMBIFIED_PIGLIN_SKIN,
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

fn emit_model_parts_with_color(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
    color: [f32; 4],
) {
    for part in parts {
        emit_model_part_with_color(mesh, part, parent_transform, color);
    }
}

fn emit_model_cubes_at_pose(
    mesh: &mut EntityModelMesh,
    parent_transform: Mat4,
    pose: PartPose,
    cubes: &[ModelCubeDesc],
) {
    let transform = parent_transform * part_pose_transform(pose);
    for cube in cubes {
        emit_model_cube(mesh, transform, *cube);
    }
}

fn emit_model_part(mesh: &mut EntityModelMesh, part: &ModelPartDesc, parent_transform: Mat4) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube(mesh, transform, *cube);
    }
    emit_model_parts(mesh, part.children, transform);
}

fn emit_model_part_with_color(
    mesh: &mut EntityModelMesh,
    part: &ModelPartDesc,
    parent_transform: Mat4,
    color: [f32; 4],
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube_with_color(mesh, transform, *cube, color);
    }
    emit_model_parts_with_color(mesh, part.children, transform, color);
}

fn emit_model_cube_with_color(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    cube: ModelCubeDesc,
    color: [f32; 4],
) {
    emit_model_cube(
        mesh,
        transform,
        ModelCubeDesc {
            min: cube.min,
            size: cube.size,
            color,
        },
    );
}

fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.y_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
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

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
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
    fn zombie_adult_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ADULT_ZOMBIE_HAT[0],
            ModelCubeDesc {
                min: [-4.5, -8.5, -4.5],
                size: [9.0, 9.0, 9.0],
                color: ZOMBIE_GREEN,
            }
        );
        assert_eq!(ADULT_ZOMBIE_PARTS.len(), 6);
        assert_eq!(ADULT_ZOMBIE_PARTS[0].pose, PART_POSE_ZERO);
        assert_eq!(ADULT_ZOMBIE_PARTS[0].cubes, ADULT_ZOMBIE_HEAD.as_slice());
        assert_eq!(
            ADULT_ZOMBIE_PARTS[0].children,
            ADULT_ZOMBIE_HEAD_CHILDREN.as_slice()
        );
        assert_part(
            &ADULT_ZOMBIE_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_HAT.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_BODY.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_PARTS[2],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_PARTS[3],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_LEFT_ARM.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_PARTS[4],
            [-1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_LEG.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_PARTS[5],
            [1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_LEG.as_slice(),
        );
    }

    #[test]
    fn zombie_adult_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::zombie(
            54,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.5, 64.001, -0.28125]);
        assert_close3(max, [0.5, 66.03225, 0.28125]);
    }

    #[test]
    fn zombie_baby_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            BABY_ZOMBIE_HEAD,
            [
                ModelCubeDesc {
                    min: [-3.0, -6.25, -3.0],
                    size: [6.0, 6.0, 6.0],
                    color: ZOMBIE_GREEN,
                },
                ModelCubeDesc {
                    min: [-3.25, -6.4, -3.25],
                    size: [6.5, 6.5, 6.5],
                    color: ZOMBIE_GREEN,
                },
            ]
        );
        assert_eq!(BABY_ZOMBIE_PARTS.len(), 6);
        assert_part(
            &BABY_ZOMBIE_PARTS[0],
            [0.0, 17.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_BODY.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_PARTS[1],
            [0.0, 15.25, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_HEAD.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_PARTS[2],
            [-3.0, 15.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_ARM.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_PARTS[3],
            [3.0, 15.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_ARM.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_PARTS[4],
            [-1.0, 20.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_LEG.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_PARTS[5],
            [1.0, 20.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_LEG.as_slice(),
        );
    }

    #[test]
    fn zombie_villager_model_parts_match_vanilla_26_1_body_layers() {
        assert_eq!(
            ADULT_ZOMBIE_VILLAGER_HEAD,
            [
                ModelCubeDesc {
                    min: [-4.0, -10.0, -4.0],
                    size: [8.0, 10.0, 8.0],
                    color: ZOMBIE_VILLAGER_ROBE,
                },
                ModelCubeDesc {
                    min: [-1.0, -3.0, -6.0],
                    size: [2.0, 4.0, 2.0],
                    color: ZOMBIE_VILLAGER_ROBE,
                },
            ]
        );
        assert_eq!(
            ADULT_ZOMBIE_VILLAGER_BODY[1],
            ModelCubeDesc {
                min: [-4.05, -0.05, -3.05],
                size: [8.1, 20.1, 6.1],
                color: ZOMBIE_VILLAGER_ROBE,
            }
        );
        assert_eq!(ADULT_ZOMBIE_VILLAGER_PARTS.len(), 6);
        assert_part_tree(
            &ADULT_ZOMBIE_VILLAGER_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_HEAD.as_slice(),
            ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
        );
        assert_part_tree(
            &ADULT_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_HAT.as_slice(),
            ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_HAT_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_BODY.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_PARTS[2],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_PARTS[3],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_LEFT_ARM.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_PARTS[4],
            [-2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
        );
        assert_part(
            &ADULT_ZOMBIE_VILLAGER_PARTS[5],
            [2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_ZOMBIE_VILLAGER_LEG.as_slice(),
        );

        assert_eq!(
            BABY_ZOMBIE_VILLAGER_BODY[1],
            ModelCubeDesc {
                min: [-2.1, -2.85, -1.6],
                size: [4.2, 6.2, 3.2],
                color: ZOMBIE_VILLAGER_ROBE,
            }
        );
        assert_eq!(BABY_ZOMBIE_VILLAGER_PARTS.len(), 6);
        assert_part(
            &BABY_ZOMBIE_VILLAGER_PARTS[0],
            [0.0, 18.75, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_BODY.as_slice(),
        );
        assert_part_tree(
            &BABY_ZOMBIE_VILLAGER_PARTS[1],
            [0.0, 16.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_HEAD.as_slice(),
            BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[0],
            [0.0, -4.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_HAT.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[1],
            [0.0, -4.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_HAT_RIM.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_HEAD_CHILDREN[2],
            [0.0, -1.0, -4.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_NOSE.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_PARTS[2],
            [-3.0, 15.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_PARTS[3],
            [3.0, 15.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_ARM.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_PARTS[4],
            [-1.0, 21.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
        );
        assert_part(
            &BABY_ZOMBIE_VILLAGER_PARTS[5],
            [1.0, 21.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_ZOMBIE_VILLAGER_LEG.as_slice(),
        );
    }

    #[test]
    fn piglin_model_parts_match_vanilla_26_1_body_layers() {
        assert_eq!(
            ADULT_PIGLIN_HEAD,
            [
                ModelCubeDesc {
                    min: [-5.0, -8.0, -4.0],
                    size: [10.0, 8.0, 8.0],
                    color: PIGLIN_SKIN,
                },
                ModelCubeDesc {
                    min: [-2.0, -4.0, -5.0],
                    size: [4.0, 4.0, 1.0],
                    color: PIGLIN_SKIN,
                },
                ModelCubeDesc {
                    min: [2.0, -2.0, -5.0],
                    size: [1.0, 2.0, 1.0],
                    color: PIGLIN_SKIN,
                },
                ModelCubeDesc {
                    min: [-3.0, -2.0, -5.0],
                    size: [1.0, 2.0, 1.0],
                    color: PIGLIN_SKIN,
                },
            ]
        );
        assert_eq!(ADULT_PIGLIN_PARTS.len(), 6);
        assert_part_tree(
            &ADULT_PIGLIN_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_HEAD.as_slice(),
            ADULT_PIGLIN_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_HEAD_CHILDREN[0],
            [4.5, -6.0, 0.0],
            [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
            ADULT_PIGLIN_LEFT_EAR.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_HEAD_CHILDREN[1],
            [-4.5, -6.0, 0.0],
            [0.0, 0.0, std::f32::consts::FRAC_PI_6],
            ADULT_PIGLIN_RIGHT_EAR.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_BODY.as_slice(),
        );
        assert_part_tree(
            &ADULT_PIGLIN_PARTS[2],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_RIGHT_ARM.as_slice(),
            ADULT_PIGLIN_RIGHT_ARM_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_RIGHT_ARM_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_RIGHT_SLEEVE.as_slice(),
        );
        assert_part_tree(
            &ADULT_PIGLIN_PARTS[3],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_LEFT_ARM.as_slice(),
            ADULT_PIGLIN_LEFT_ARM_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_LEFT_ARM_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_LEFT_SLEEVE.as_slice(),
        );
        assert_part_tree(
            &ADULT_PIGLIN_PARTS[4],
            [-1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_LEG.as_slice(),
            ADULT_PIGLIN_LEG_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_PIGLIN_LEG_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_PANTS.as_slice(),
        );
        assert_part_tree(
            &ADULT_PIGLIN_PARTS[5],
            [1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_PIGLIN_LEG.as_slice(),
            ADULT_PIGLIN_LEG_CHILDREN.as_slice(),
        );

        assert_eq!(BABY_PIGLIN_PARTS.len(), 6);
        assert_part(
            &BABY_PIGLIN_PARTS[0],
            [0.0, 18.0, -0.5],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_BODY.as_slice(),
        );
        assert_part_tree(
            &BABY_PIGLIN_PARTS[1],
            [0.0, 15.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_HEAD.as_slice(),
            BABY_PIGLIN_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            &[],
        );
        assert_part_tree(
            &BABY_PIGLIN_HEAD_CHILDREN[1],
            [4.2, -4.0, 0.0],
            [0.0, 0.0, 0.0],
            &[],
            BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_LEFT_EAR_ROTATED_CHILDREN[0],
            [1.0, 1.75, 0.0],
            [0.0, 0.0, -0.6109],
            BABY_PIGLIN_LEFT_EAR.as_slice(),
        );
        assert_part_tree(
            &BABY_PIGLIN_HEAD_CHILDREN[2],
            [-4.2, -4.0, 0.0],
            [0.0, 0.0, 0.0],
            &[],
            BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_RIGHT_EAR_ROTATED_CHILDREN[0],
            [-1.0, 1.75, 0.0],
            [0.0, 0.0, 0.6109],
            BABY_PIGLIN_RIGHT_EAR.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_PARTS[2],
            [4.0, 15.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_LEFT_ARM.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_PARTS[3],
            [-4.0, 15.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_PARTS[4],
            [-1.5, 20.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_LEG.as_slice(),
        );
        assert_part(
            &BABY_PIGLIN_PARTS[5],
            [1.5, 20.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_PIGLIN_LEG.as_slice(),
        );
    }

    #[test]
    fn zombie_baby_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh =
            entity_model_mesh(&[EntityModelInstance::zombie(55, [0.0, 64.0, 0.0], 0.0, true)]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.25, 64.001, -0.203125]);
        assert_close3(max, [0.25, 64.947876, 0.203125]);
    }

    #[test]
    fn zombie_variant_meshes_use_vanilla_body_layer_geometry() {
        let zombie = entity_model_mesh(&[EntityModelInstance::zombie(
            150,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )]);
        let baby_zombie = entity_model_mesh(&[EntityModelInstance::zombie(
            150,
            [0.0, 64.0, 0.0],
            0.0,
            true,
        )]);

        let husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            67,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Husk,
            false,
        )]);
        assert_eq!(husk.opaque_faces, 42);
        assert_eq!(husk.vertices.len(), 168);
        assert_eq!(husk.indices.len(), 252);
        assert!(husk
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));
        let (husk_min, husk_max) = mesh_extents(&husk);
        assert_close3(husk_min, [-0.53125, 64.00106, -0.29882815]);
        assert_close3(husk_max, [0.53125, 66.15926, 0.29882815]);

        let baby_husk = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            67,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Husk,
            true,
        )]);
        assert_same_geometry(&baby_husk, &baby_zombie);
        assert!(baby_husk
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(HUSK_TAN, 0.78)));

        let drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            38,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        )]);
        assert_same_geometry(&drowned, &zombie);
        assert!(drowned
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

        let baby_drowned = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            38,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            true,
        )]);
        assert_same_geometry(&baby_drowned, &baby_zombie);
        assert!(baby_drowned
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(DROWNED_BLUE, 0.78)));

        let zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            153,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            false,
        )]);
        assert_eq!(zombie_villager.opaque_faces, 60);
        assert_eq!(zombie_villager.vertices.len(), 240);
        assert_eq!(zombie_villager.indices.len(), 360);
        assert!(zombie_villager
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
        let (zombie_villager_min, zombie_villager_max) = mesh_extents(&zombie_villager);
        assert_close3(zombie_villager_min, [-0.50000006, 64.001, -0.50000006]);
        assert_close3(zombie_villager_max, [0.50000006, 66.15725, 0.50000006]);

        let baby_zombie_villager = entity_model_mesh(&[EntityModelInstance::zombie_variant(
            153,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            true,
        )]);
        assert_eq!(baby_zombie_villager.opaque_faces, 60);
        assert_eq!(baby_zombie_villager.vertices.len(), 240);
        assert_eq!(baby_zombie_villager.indices.len(), 360);
        assert!(baby_zombie_villager
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(ZOMBIE_VILLAGER_ROBE, 0.78)));
        let (baby_zombie_villager_min, baby_zombie_villager_max) =
            mesh_extents(&baby_zombie_villager);
        assert_close3(baby_zombie_villager_min, [-0.43750003, 64.001, -0.37500003]);
        assert_close3(baby_zombie_villager_max, [0.43750003, 65.01975, 0.37500003]);
    }

    #[test]
    fn piglin_meshes_use_vanilla_body_layer_geometry() {
        let piglin = entity_model_mesh(&[EntityModelInstance::piglin(
            101,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::Piglin,
            false,
        )]);
        assert_eq!(piglin.opaque_faces, 90);
        assert_eq!(piglin.vertices.len(), 360);
        assert_eq!(piglin.indices.len(), 540);
        assert!(piglin
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
        let (piglin_min, piglin_max) = mesh_extents(&piglin);
        assert_close3(piglin_min, [-0.515625, 63.985374, -0.25000003]);
        assert_close3(piglin_max, [0.515625, 66.001, 0.31250003]);

        let baby_piglin = entity_model_mesh(&[EntityModelInstance::piglin(
            101,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::Piglin,
            true,
        )]);
        assert_eq!(baby_piglin.opaque_faces, 54);
        assert_eq!(baby_piglin.vertices.len(), 216);
        assert_eq!(baby_piglin.indices.len(), 324);
        assert!(baby_piglin
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
        let (baby_piglin_min, baby_piglin_max) = mesh_extents(&baby_piglin);
        assert_close3(baby_piglin_min, [-0.45814878, 64.001, -0.21875003]);
        assert_close3(baby_piglin_max, [0.45814878, 64.9385, 0.28125]);

        let brute = entity_model_mesh(&[EntityModelInstance::piglin(
            102,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::PiglinBrute,
            false,
        )]);
        assert_same_geometry(&brute, &piglin);
        assert!(brute
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PIGLIN_BRUTE_SKIN, 0.78)));

        let zombified = entity_model_mesh(&[EntityModelInstance::piglin(
            154,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::ZombifiedPiglin,
            false,
        )]);
        assert_same_geometry(&zombified, &piglin);
        assert!(zombified
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));

        let baby_zombified = entity_model_mesh(&[EntityModelInstance::piglin(
            154,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::ZombifiedPiglin,
            true,
        )]);
        assert_same_geometry(&baby_zombified, &baby_piglin);
        assert!(baby_zombified
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));
    }

    #[test]
    fn skeleton_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            SKELETON_HAT[0],
            ModelCubeDesc {
                min: [-4.5, -8.5, -4.5],
                size: [9.0, 9.0, 9.0],
                color: SKELETON_BONE,
            }
        );
        assert_eq!(SKELETON_PARTS.len(), 6);
        assert_eq!(SKELETON_PARTS[0].pose, PART_POSE_ZERO);
        assert_eq!(SKELETON_PARTS[0].cubes, SKELETON_HEAD.as_slice());
        assert_eq!(
            SKELETON_PARTS[0].children,
            SKELETON_HEAD_CHILDREN.as_slice()
        );
        assert_part(
            &SKELETON_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SKELETON_BODY.as_slice(),
        );
        assert_part(
            &SKELETON_PARTS[2],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            SKELETON_ARM.as_slice(),
        );
        assert_part(
            &SKELETON_PARTS[3],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            SKELETON_ARM.as_slice(),
        );
        assert_part(
            &SKELETON_PARTS[4],
            [-2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SKELETON_LEG.as_slice(),
        );
        assert_part(
            &SKELETON_PARTS[5],
            [2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SKELETON_LEG.as_slice(),
        );
    }

    #[test]
    fn skeleton_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::skeleton(115, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.375, 64.001, -0.28125]);
        assert_close3(max, [0.375, 66.03225, 0.28125]);
    }

    #[test]
    fn zombie_and_skeleton_texture_refs_match_vanilla_renderers() {
        assert_eq!(
            EntityModelKind::Zombie { baby: false }.model_key(),
            "zombie"
        );
        assert_eq!(
            EntityModelKind::Zombie { baby: false }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/zombie.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Zombie { baby: true }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/zombie_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/husk.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/husk_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/drowned.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie/drowned_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie_villager/zombie_villager.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/zombie_villager/zombie_villager_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/piglin/piglin.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/piglin/piglin_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/piglin/piglin_brute.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/piglin/zombified_piglin.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true,
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/piglin/zombified_piglin_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
        assert_eq!(
            EntityModelKind::Skeleton.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/skeleton/skeleton.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Stray
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/skeleton/stray.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Parched
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/skeleton/parched.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton
            }
            .vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/skeleton/wither_skeleton.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::Humanoid {
                family: HumanoidModelFamily::Zombie,
                baby: false,
            }
            .vanilla_texture_ref(),
            None
        );
    }

    #[test]
    fn skeleton_variant_parts_match_vanilla_26_1_body_layers() {
        assert_eq!(
            PARCHED_BODY,
            [
                ModelCubeDesc {
                    min: [-4.0, 0.0, -2.0],
                    size: [8.0, 12.0, 4.0],
                    color: PARCHED_BONE,
                },
                ModelCubeDesc {
                    min: [-4.0, 10.0, -2.0],
                    size: [8.0, 1.0, 4.0],
                    color: PARCHED_BONE,
                },
                ModelCubeDesc {
                    min: [-4.025, -0.025, -2.025],
                    size: [8.05, 12.05, 4.05],
                    color: PARCHED_BONE,
                },
            ]
        );
        assert_eq!(
            PARCHED_HEAD[1],
            ModelCubeDesc {
                min: [-4.2, -8.2, -4.2],
                size: [8.4, 8.4, 8.4],
                color: PARCHED_BONE,
            }
        );

        assert_eq!(PARCHED_PARTS.len(), 6);
        assert_part_tree(
            &PARCHED_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_HEAD.as_slice(),
            PARCHED_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &PARCHED_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_EMPTY_HAT.as_slice(),
        );
        assert_part(
            &PARCHED_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_BODY.as_slice(),
        );
        assert_part(
            &PARCHED_PARTS[2],
            [-5.5, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &PARCHED_PARTS[3],
            [5.5, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_LEFT_ARM.as_slice(),
        );
        assert_part(
            &PARCHED_PARTS[4],
            [-2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_LEG.as_slice(),
        );
        assert_part(
            &PARCHED_PARTS[5],
            [2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            PARCHED_LEG.as_slice(),
        );
    }

    #[test]
    fn skeleton_variant_meshes_use_vanilla_body_layer_geometry() {
        let skeleton =
            entity_model_mesh(&[EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0)]);
        let stray = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
            128,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Stray,
        )]);
        assert_eq!(stray.vertices, skeleton.vertices);
        assert_eq!(stray.indices, skeleton.indices);

        let wither = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
            146,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::WitherSkeleton,
        )]);
        assert_eq!(wither.opaque_faces, 42);
        assert_eq!(wither.vertices.len(), 168);
        assert_eq!(wither.indices.len(), 252);
        assert!(wither
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(WITHER_SKELETON_DARK, 0.78)));
        let (wither_min, wither_max) = mesh_extents(&wither);
        assert_close3(wither_min, [-0.45000002, 64.0012, -0.33750004]);
        assert_close3(wither_max, [0.45000002, 66.4387, 0.33750004]);

        let parched = entity_model_mesh(&[EntityModelInstance::skeleton_variant(
            97,
            [0.0, 64.0, 0.0],
            0.0,
            SkeletonModelFamily::Parched,
        )]);
        assert_eq!(parched.opaque_faces, 78);
        assert_eq!(parched.vertices.len(), 312);
        assert_eq!(parched.indices.len(), 468);
        let (parched_min, parched_max) = mesh_extents(&parched);
        assert_close3(parched_min, [-0.440625, 64.001, -0.26250002]);
        assert_close3(parched_max, [0.440625, 66.0135, 0.26250002]);
    }

    #[test]
    fn pig_adult_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ADULT_PIG_HEAD,
            [
                ModelCubeDesc {
                    min: [-4.0, -4.0, -8.0],
                    size: [8.0, 8.0, 8.0],
                    color: PIG_PINK,
                },
                ModelCubeDesc {
                    min: [-2.0, 0.0, -9.0],
                    size: [4.0, 3.0, 1.0],
                    color: PIG_PINK,
                },
            ]
        );
        assert_eq!(
            ADULT_PIG_BODY[0],
            ModelCubeDesc {
                min: [-5.0, -10.0, -7.0],
                size: [10.0, 16.0, 8.0],
                color: PIG_PINK,
            }
        );
        assert_eq!(
            ADULT_PIG_LEG[0],
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, 6.0, 4.0],
                color: PIG_PINK,
            }
        );

        assert_eq!(ADULT_PIG_PARTS.len(), 6);
        assert_part(
            &ADULT_PIG_PARTS[0],
            [0.0, 12.0, -6.0],
            [0.0, 0.0, 0.0],
            ADULT_PIG_HEAD.as_slice(),
        );
        assert_part(
            &ADULT_PIG_PARTS[1],
            [0.0, 11.0, 2.0],
            [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            ADULT_PIG_BODY.as_slice(),
        );

        for (part, expected_offset) in ADULT_PIG_PARTS[2..].iter().zip([
            [-3.0, 18.0, 7.0],
            [3.0, 18.0, 7.0],
            [-3.0, 18.0, -5.0],
            [3.0, 18.0, -5.0],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                ADULT_PIG_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn pig_adult_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::quadruped(
            90,
            [0.0, 64.0, 0.0],
            0.0,
            QuadrupedModelFamily::Pig,
            false,
        )]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.3125, 64.001, -0.5625]);
        assert_close3(max, [0.3125, 65.001, 0.9375]);
    }

    #[test]
    fn pig_baby_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            BABY_PIG_BODY[0],
            ModelCubeDesc {
                min: [-3.5, -3.0, -4.5],
                size: [7.0, 6.0, 9.0],
                color: PIG_PINK,
            }
        );
        assert_eq!(
            BABY_PIG_HEAD,
            [
                ModelCubeDesc {
                    min: [-3.525, -5.025, -5.025],
                    size: [7.05, 6.05, 6.05],
                    color: PIG_PINK,
                },
                ModelCubeDesc {
                    min: [-1.515, -1.99, -6.015],
                    size: [3.03, 2.03, 1.03],
                    color: PIG_PINK,
                },
            ]
        );
        assert_eq!(
            BABY_PIG_LEG[0],
            ModelCubeDesc {
                min: [-1.0, 0.0, -1.0],
                size: [2.0, 2.0, 2.0],
                color: PIG_PINK,
            }
        );

        assert_eq!(BABY_PIG_PARTS.len(), 6);
        assert_part(
            &BABY_PIG_PARTS[0],
            [0.0, 19.0, 0.5],
            [0.0, 0.0, 0.0],
            BABY_PIG_BODY.as_slice(),
        );
        assert_part(
            &BABY_PIG_PARTS[1],
            [0.0, 19.0, -2.0],
            [0.0, 0.0, 0.0],
            BABY_PIG_HEAD.as_slice(),
        );

        for (part, expected_offset) in BABY_PIG_PARTS[2..].iter().zip([
            [2.5, 22.0, -3.0],
            [-2.5, 22.0, -3.0],
            [2.5, 22.0, 4.0],
            [-2.5, 22.0, 4.0],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                BABY_PIG_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn pig_baby_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::quadruped(
            91,
            [0.0, 64.0, 0.0],
            0.0,
            QuadrupedModelFamily::Pig,
            true,
        )]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.2203125, 64.001, -0.3125]);
        assert_close3(max, [0.2203125, 64.62756, 0.5009375]);
    }

    #[test]
    fn cow_adult_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ADULT_COW_HEAD,
            [
                ModelCubeDesc {
                    min: [-4.0, -4.0, -6.0],
                    size: [8.0, 8.0, 6.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [-3.0, 1.0, -7.0],
                    size: [6.0, 3.0, 1.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [-5.0, -5.0, -5.0],
                    size: [1.0, 3.0, 1.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [4.0, -5.0, -5.0],
                    size: [1.0, 3.0, 1.0],
                    color: COW_BROWN,
                },
            ]
        );
        assert_eq!(ADULT_COW_PARTS.len(), 6);
        assert_part(
            &ADULT_COW_PARTS[0],
            [0.0, 4.0, -8.0],
            [0.0, 0.0, 0.0],
            ADULT_COW_HEAD.as_slice(),
        );
        assert_part(
            &ADULT_COW_PARTS[1],
            [0.0, 5.0, 2.0],
            [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            ADULT_COW_BODY.as_slice(),
        );
        for (part, expected_offset) in ADULT_COW_PARTS[2..].iter().zip([
            [-4.0, 12.0, 7.0],
            [4.0, 12.0, 7.0],
            [-4.0, 12.0, -5.0],
            [4.0, 12.0, -5.0],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                ADULT_COW_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn cow_adult_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::cow(92, [0.0, 64.0, 0.0], 0.0, false)]);

        assert_eq!(mesh.opaque_faces, 60);
        assert_eq!(mesh.vertices.len(), 240);
        assert_eq!(mesh.indices.len(), 360);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.375, 64.001, -0.625]);
        assert_close3(max, [0.375, 65.5635, 0.9375]);
    }

    #[test]
    fn cow_baby_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            BABY_COW_HEAD,
            [
                ModelCubeDesc {
                    min: [-3.0, -4.569, -4.8333],
                    size: [6.0, 6.0, 5.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [3.0, -5.569, -3.8333],
                    size: [1.0, 2.0, 1.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [-4.0, -5.569, -3.8333],
                    size: [1.0, 2.0, 1.0],
                    color: COW_BROWN,
                },
                ModelCubeDesc {
                    min: [-2.0, -1.569, -5.8333],
                    size: [4.0, 3.0, 1.0],
                    color: COW_BROWN,
                },
            ]
        );
        assert_eq!(BABY_COW_PARTS.len(), 6);
        assert_part(
            &BABY_COW_PARTS[0],
            [0.0, 13.569, -5.1667],
            [0.0, 0.0, 0.0],
            BABY_COW_HEAD.as_slice(),
        );
        assert_part(
            &BABY_COW_PARTS[1],
            [3.0, 19.0, -5.0],
            [0.0, 0.0, 0.0],
            BABY_COW_BODY.as_slice(),
        );
        for (part, expected_offset) in BABY_COW_PARTS[2..].iter().zip([
            [-2.5, 18.0, -3.5],
            [2.5, 18.0, -3.5],
            [-2.5, 18.0, 3.5],
            [2.5, 18.0, 3.5],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                BABY_COW_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn cow_baby_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::cow(93, [0.0, 64.0, 0.0], 0.0, true)]);

        assert_eq!(mesh.opaque_faces, 54);
        assert_eq!(mesh.vertices.len(), 216);
        assert_eq!(mesh.indices.len(), 324);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.25, 64.001, -0.375]);
        assert_close3(max, [0.25, 65.001, 0.6875]);
    }

    #[test]
    fn sheep_adult_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ADULT_SHEEP_HEAD[0],
            ModelCubeDesc {
                min: [-3.0, -4.0, -6.0],
                size: [6.0, 6.0, 8.0],
                color: SHEEP_WOOL,
            }
        );
        assert_eq!(ADULT_SHEEP_PARTS.len(), 6);
        assert_part(
            &ADULT_SHEEP_PARTS[0],
            [0.0, 6.0, -8.0],
            [0.0, 0.0, 0.0],
            ADULT_SHEEP_HEAD.as_slice(),
        );
        assert_part(
            &ADULT_SHEEP_PARTS[1],
            [0.0, 5.0, 2.0],
            [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            ADULT_SHEEP_BODY.as_slice(),
        );
        for (part, expected_offset) in ADULT_SHEEP_PARTS[2..].iter().zip([
            [-3.0, 12.0, 7.0],
            [3.0, 12.0, 7.0],
            [-3.0, 12.0, -5.0],
            [3.0, 12.0, -5.0],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                ADULT_SHEEP_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn sheep_adult_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh =
            entity_model_mesh(&[EntityModelInstance::sheep(94, [0.0, 64.0, 0.0], 0.0, false)]);

        assert_eq!(mesh.opaque_faces, 36);
        assert_eq!(mesh.vertices.len(), 144);
        assert_eq!(mesh.indices.len(), 216);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.3125, 64.001, -0.5625]);
        assert_close3(max, [0.3125, 65.376, 0.875]);
    }

    #[test]
    fn sheep_baby_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            BABY_SHEEP_BODY[0],
            ModelCubeDesc {
                min: [-3.0, -2.0, -4.5],
                size: [6.0, 4.0, 9.0],
                color: SHEEP_WOOL,
            }
        );
        assert_eq!(BABY_SHEEP_PARTS.len(), 6);
        assert_part(
            &BABY_SHEEP_PARTS[0],
            [0.0, 17.0, 0.5],
            [0.0, 0.0, 0.0],
            BABY_SHEEP_BODY.as_slice(),
        );
        assert_part(
            &BABY_SHEEP_PARTS[1],
            [0.0, 15.5, -2.5],
            [0.0, 0.0, 0.0],
            BABY_SHEEP_HEAD.as_slice(),
        );
        for (part, expected_offset) in BABY_SHEEP_PARTS[2..].iter().zip([
            [-2.0, 19.0, 3.0],
            [2.0, 19.0, 3.0],
            [-2.0, 19.0, -2.0],
            [2.0, 19.0, -2.0],
        ]) {
            assert_part(
                part,
                expected_offset,
                [0.0, 0.0, 0.0],
                BABY_SHEEP_LEG.as_slice(),
            );
        }
    }

    #[test]
    fn sheep_baby_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh =
            entity_model_mesh(&[EntityModelInstance::sheep(95, [0.0, 64.0, 0.0], 0.0, true)]);

        assert_eq!(mesh.opaque_faces, 36);
        assert_eq!(mesh.vertices.len(), 144);
        assert_eq!(mesh.indices.len(), 216);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.1875, 64.001, -0.3125]);
        assert_close3(max, [0.1875, 64.8135, 0.375]);
    }

    #[test]
    fn cow_and_sheep_texture_refs_match_vanilla_renderers() {
        assert_eq!(EntityModelKind::Cow { baby: false }.model_key(), "cow");
        assert_eq!(EntityModelKind::Cow { baby: true }.model_key(), "cow_baby");
        assert_eq!(
            EntityModelKind::Cow { baby: false }.vanilla_texture_ref(),
            None
        );
        assert_eq!(EntityModelKind::Sheep { baby: false }.model_key(), "sheep");
        assert_eq!(
            EntityModelKind::Sheep { baby: false }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/sheep/sheep.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::Sheep { baby: true }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/sheep/sheep_baby.png",
                size: [64, 32],
            })
        );
    }

    #[test]
    fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ADULT_VILLAGER_HAT[0],
            ModelCubeDesc {
                min: [-4.51, -10.51, -4.51],
                size: [9.02, 11.02, 9.02],
                color: VILLAGER_ROBE,
            }
        );
        assert_eq!(
            ADULT_VILLAGER_JACKET[0],
            ModelCubeDesc {
                min: [-4.5, -0.5, -3.5],
                size: [9.0, 21.0, 7.0],
                color: VILLAGER_ROBE,
            }
        );
        assert_eq!(ADULT_VILLAGER_PARTS.len(), 5);
        assert_part_tree(
            &ADULT_VILLAGER_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_HEAD.as_slice(),
            ADULT_VILLAGER_HEAD_CHILDREN.as_slice(),
        );
        assert_part_tree(
            &ADULT_VILLAGER_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_HAT.as_slice(),
            ADULT_VILLAGER_HAT_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_HAT_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            ADULT_VILLAGER_HAT_RIM.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_HEAD_CHILDREN[1],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_NOSE.as_slice(),
        );
        assert_part_tree(
            &ADULT_VILLAGER_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_BODY.as_slice(),
            ADULT_VILLAGER_BODY_CHILDREN.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_BODY_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_JACKET.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_PARTS[2],
            [0.0, 3.0, -1.0],
            [-0.75, 0.0, 0.0],
            ADULT_VILLAGER_ARMS.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_PARTS[3],
            [-2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_LEG.as_slice(),
        );
        assert_part(
            &ADULT_VILLAGER_PARTS[4],
            [2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ADULT_VILLAGER_LEG.as_slice(),
        );
    }

    #[test]
    fn villager_adult_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::villager(
            139,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )]);

        assert_eq!(mesh.opaque_faces, 66);
        assert_eq!(mesh.vertices.len(), 264);
        assert_eq!(mesh.indices.len(), 396);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.46875003, 64.00094, -0.46875006]);
        assert_close3(max, [0.46875003, 66.02301, 0.46875003]);

        let wandering_trader_mesh = entity_model_mesh(&[EntityModelInstance::wandering_trader(
            141,
            [0.0, 64.0, 0.0],
            0.0,
        )]);
        assert_eq!(wandering_trader_mesh.opaque_faces, mesh.opaque_faces);
        assert_eq!(wandering_trader_mesh.vertices, mesh.vertices);
        assert_eq!(wandering_trader_mesh.indices, mesh.indices);
    }

    #[test]
    fn villager_baby_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            BABY_VILLAGER_RIGHT_HAND,
            [
                ModelCubeDesc {
                    min: [-1.0, -2.4925, -1.8401],
                    size: [2.0, 4.0, 2.0],
                    color: VILLAGER_ROBE,
                },
                ModelCubeDesc {
                    min: [5.0, -2.4925, -1.8401],
                    size: [2.0, 4.0, 2.0],
                    color: VILLAGER_ROBE,
                },
            ]
        );
        assert_eq!(
            BABY_VILLAGER_BB_MAIN[0],
            ModelCubeDesc {
                min: [-2.7, -8.2, -1.7],
                size: [4.4, 6.4, 3.4],
                color: VILLAGER_ROBE,
            }
        );
        assert_eq!(BABY_VILLAGER_PARTS.len(), 6);
        assert_part_tree(
            &BABY_VILLAGER_PARTS[0],
            [0.0, 17.5, 0.0],
            [0.0, 0.0, 0.0],
            &[],
            BABY_VILLAGER_ARMS_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_ARMS_CHILDREN[0],
            [-3.0, 1.4025, -0.9599],
            [-1.0472, 0.0, 0.0],
            BABY_VILLAGER_RIGHT_HAND.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_ARMS_CHILDREN[1],
            [0.0, 0.9024, -1.8175],
            [-1.0472, 0.0, 0.0],
            BABY_VILLAGER_MIDDLE_ARM.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_PARTS[1],
            [-1.0, 21.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_LEG.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_PARTS[2],
            [1.0, 21.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_LEG.as_slice(),
        );
        assert_part_tree(
            &BABY_VILLAGER_PARTS[3],
            [0.0, 16.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_HEAD.as_slice(),
            BABY_VILLAGER_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_HEAD_CHILDREN[0],
            [0.0, -4.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_HAT.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_HEAD_CHILDREN[1],
            [0.0, -4.5, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_HAT_RIM.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_HEAD_CHILDREN[2],
            [0.0, -2.0, -4.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_NOSE.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_PARTS[4],
            [0.0, 18.75, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_BODY.as_slice(),
        );
        assert_part(
            &BABY_VILLAGER_PARTS[5],
            [0.5, 24.0, 0.0],
            [0.0, 0.0, 0.0],
            BABY_VILLAGER_BB_MAIN.as_slice(),
        );
    }

    #[test]
    fn villager_baby_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::villager(
            140,
            [0.0, 64.0, 0.0],
            0.0,
            true,
        )]);

        assert_eq!(mesh.opaque_faces, 66);
        assert_eq!(mesh.vertices.len(), 264);
        assert_eq!(mesh.indices.len(), 396);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.43750003, 64.001, -0.37500003]);
        assert_close3(max, [0.43750003, 65.01975, 0.37500003]);
    }

    #[test]
    fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
        assert_eq!(
            EntityModelKind::Villager { baby: false }.model_key(),
            "villager"
        );
        assert_eq!(
            EntityModelKind::Villager { baby: false }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/villager/villager.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::Villager { baby: true }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/villager/villager_baby.png",
                size: [64, 64],
            })
        );
        assert_eq!(
            EntityModelKind::WanderingTrader.model_key(),
            "wandering_trader"
        );
        assert_eq!(
            EntityModelKind::WanderingTrader.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/wandering_trader/wandering_trader.png",
                size: [64, 64],
            })
        );
    }

    #[test]
    fn creeper_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            CREEPER_HEAD[0],
            ModelCubeDesc {
                min: [-4.0, -8.0, -4.0],
                size: [8.0, 8.0, 8.0],
                color: CREEPER_GREEN
            }
        );
        assert_eq!(
            CREEPER_BODY[0],
            ModelCubeDesc {
                min: [-4.0, 0.0, -2.0],
                size: [8.0, 12.0, 4.0],
                color: CREEPER_GREEN
            }
        );
        assert_eq!(
            CREEPER_LEG[0],
            ModelCubeDesc {
                min: [-2.0, 0.0, -2.0],
                size: [4.0, 6.0, 4.0],
                color: CREEPER_GREEN
            }
        );

        assert_eq!(CREEPER_PARTS.len(), 6);
        assert_eq!(CREEPER_PARTS[0].pose.offset, [0.0, 6.0, 0.0]);
        assert_eq!(CREEPER_PARTS[0].cubes, CREEPER_HEAD.as_slice());
        assert_eq!(CREEPER_PARTS[1].pose.offset, [0.0, 6.0, 0.0]);
        assert_eq!(CREEPER_PARTS[1].cubes, CREEPER_BODY.as_slice());

        let leg_offsets = [
            [-2.0, 18.0, 4.0],
            [2.0, 18.0, 4.0],
            [-2.0, 18.0, -4.0],
            [2.0, 18.0, -4.0],
        ];
        for (part, expected_offset) in CREEPER_PARTS[2..].iter().zip(leg_offsets) {
            assert_eq!(part.pose.offset, expected_offset);
            assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
            assert_eq!(part.cubes, CREEPER_LEG.as_slice());
            assert!(part.children.is_empty());
        }
    }

    #[test]
    fn creeper_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::new(
            50,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        )]);

        assert_eq!(mesh.opaque_faces, 36);
        assert_eq!(mesh.vertices.len(), 144);
        assert_eq!(mesh.indices.len(), 216);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.25, 64.001, -0.375]);
        assert_close3(max, [0.25, 65.626, 0.375]);
    }

    #[test]
    fn creeper_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::Creeper.model_key(), "creeper");
        assert_eq!(
            EntityModelKind::Creeper.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/creeper/creeper.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::Chicken { baby: false }.vanilla_texture_ref(),
            None
        );
    }

    #[test]
    fn slime_and_magma_cube_parts_match_vanilla_26_1_body_layers() {
        assert_eq!(SLIME_PARTS.len(), 5);
        assert_part(
            &SLIME_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SLIME_INNER_CUBE.as_slice(),
        );
        assert_part(
            &SLIME_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SLIME_RIGHT_EYE.as_slice(),
        );
        assert_part(
            &SLIME_PARTS[2],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SLIME_LEFT_EYE.as_slice(),
        );
        assert_part(
            &SLIME_PARTS[3],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SLIME_MOUTH.as_slice(),
        );
        assert_part(
            &SLIME_PARTS[4],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            SLIME_OUTER_CUBE.as_slice(),
        );

        let magma_segments = [
            MAGMA_CUBE_SEGMENT_0.as_slice(),
            MAGMA_CUBE_SEGMENT_1.as_slice(),
            MAGMA_CUBE_SEGMENT_2.as_slice(),
            MAGMA_CUBE_SEGMENT_3.as_slice(),
            MAGMA_CUBE_SEGMENT_4.as_slice(),
            MAGMA_CUBE_SEGMENT_5.as_slice(),
            MAGMA_CUBE_SEGMENT_6.as_slice(),
            MAGMA_CUBE_SEGMENT_7.as_slice(),
        ];
        for (index, (part, cubes)) in MAGMA_CUBE_PARTS[..8].iter().zip(magma_segments).enumerate() {
            assert_part(part, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], cubes);
            assert_eq!(part.cubes[0].min, [-4.0, 16.0 + index as f32, -4.0]);
            assert_eq!(part.cubes[0].size, [8.0, 1.0, 8.0]);
        }
        assert_part(
            &MAGMA_CUBE_PARTS[8],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            MAGMA_CUBE_INSIDE_CUBE.as_slice(),
        );
    }

    #[test]
    fn slime_and_magma_cube_meshes_use_vanilla_size_scaling() {
        let slime = entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 1)]);
        assert_eq!(slime.opaque_faces, 30);
        assert_eq!(slime.vertices.len(), 120);
        assert_eq!(slime.indices.len(), 180);
        let (slime_min, slime_max) = mesh_extents(&slime);
        assert_close3(slime_min, [-0.24975, 64.0, -0.24975]);
        assert_close3(slime_max, [0.24975, 64.4995, 0.24975]);

        let large_slime =
            entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 4)]);
        assert_eq!(large_slime.opaque_faces, slime.opaque_faces);
        let (large_slime_min, large_slime_max) = mesh_extents(&large_slime);
        assert_close3(large_slime_min, [-0.999, 64.00299, -0.999]);
        assert_close3(large_slime_max, [0.999, 66.00099, 0.999]);

        let magma_cube = entity_model_mesh(&[EntityModelInstance::magma_cube(
            80,
            [0.0, 64.0, 0.0],
            0.0,
            3,
        )]);
        assert_eq!(magma_cube.opaque_faces, 54);
        assert_eq!(magma_cube.vertices.len(), 216);
        assert_eq!(magma_cube.indices.len(), 324);
        let (magma_min, magma_max) = mesh_extents(&magma_cube);
        assert_close3(magma_min, [-0.75, 64.003, -0.75]);
        assert_close3(magma_max, [0.75, 65.503, 0.75]);
    }

    #[test]
    fn slime_and_magma_cube_texture_refs_match_vanilla_renderers() {
        assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
        assert_eq!(
            EntityModelKind::Slime { size: 4 }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/slime/slime.png",
                size: [64, 32],
            })
        );
        assert_eq!(
            EntityModelKind::MagmaCube { size: 3 }.model_key(),
            "magma_cube"
        );
        assert_eq!(
            EntityModelKind::MagmaCube { size: 3 }.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/slime/magmacube.png",
                size: [64, 64],
            })
        );
    }

    #[test]
    fn spider_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            SPIDER_HEAD[0],
            ModelCubeDesc {
                min: [-4.0, -4.0, -8.0],
                size: [8.0, 8.0, 8.0],
                color: SPIDER_DARK,
            }
        );
        assert_eq!(
            SPIDER_BODY_0[0],
            ModelCubeDesc {
                min: [-3.0, -3.0, -3.0],
                size: [6.0, 6.0, 6.0],
                color: SPIDER_DARK,
            }
        );
        assert_eq!(
            SPIDER_BODY_1[0],
            ModelCubeDesc {
                min: [-5.0, -4.0, -6.0],
                size: [10.0, 8.0, 12.0],
                color: SPIDER_DARK,
            }
        );
        assert_eq!(
            SPIDER_RIGHT_LEG[0],
            ModelCubeDesc {
                min: [-15.0, -1.0, -1.0],
                size: [16.0, 2.0, 2.0],
                color: SPIDER_DARK,
            }
        );
        assert_eq!(
            SPIDER_LEFT_LEG[0],
            ModelCubeDesc {
                min: [-1.0, -1.0, -1.0],
                size: [16.0, 2.0, 2.0],
                color: SPIDER_DARK,
            }
        );

        assert_eq!(SPIDER_PARTS.len(), 11);
        assert_part(
            &SPIDER_PARTS[0],
            [0.0, 15.0, -3.0],
            [0.0, 0.0, 0.0],
            SPIDER_HEAD.as_slice(),
        );
        assert_part(
            &SPIDER_PARTS[1],
            [0.0, 15.0, 0.0],
            [0.0, 0.0, 0.0],
            SPIDER_BODY_0.as_slice(),
        );
        assert_part(
            &SPIDER_PARTS[2],
            [0.0, 15.0, 9.0],
            [0.0, 0.0, 0.0],
            SPIDER_BODY_1.as_slice(),
        );

        let leg_specs = [
            (
                [-4.0, 15.0, 2.0],
                [
                    0.0,
                    std::f32::consts::FRAC_PI_4,
                    -std::f32::consts::FRAC_PI_4,
                ],
                SPIDER_RIGHT_LEG.as_slice(),
            ),
            (
                [4.0, 15.0, 2.0],
                [
                    0.0,
                    -std::f32::consts::FRAC_PI_4,
                    std::f32::consts::FRAC_PI_4,
                ],
                SPIDER_LEFT_LEG.as_slice(),
            ),
            (
                [-4.0, 15.0, 1.0],
                [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
                SPIDER_RIGHT_LEG.as_slice(),
            ),
            (
                [4.0, 15.0, 1.0],
                [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
                SPIDER_LEFT_LEG.as_slice(),
            ),
            (
                [-4.0, 15.0, 0.0],
                [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
                SPIDER_RIGHT_LEG.as_slice(),
            ),
            (
                [4.0, 15.0, 0.0],
                [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
                SPIDER_LEFT_LEG.as_slice(),
            ),
            (
                [-4.0, 15.0, -1.0],
                [
                    0.0,
                    -std::f32::consts::FRAC_PI_4,
                    -std::f32::consts::FRAC_PI_4,
                ],
                SPIDER_RIGHT_LEG.as_slice(),
            ),
            (
                [4.0, 15.0, -1.0],
                [
                    0.0,
                    std::f32::consts::FRAC_PI_4,
                    std::f32::consts::FRAC_PI_4,
                ],
                SPIDER_LEFT_LEG.as_slice(),
            ),
        ];
        for (part, (offset, rotation, cubes)) in SPIDER_PARTS[3..].iter().zip(leg_specs) {
            assert_part(part, offset, rotation, cubes);
        }
    }

    #[test]
    fn spider_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::spider(124, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 66);
        assert_eq!(mesh.vertices.len(), 264);
        assert_eq!(mesh.indices.len(), 396);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-1.0282283, 64.0193, -0.9375]);
        assert_close3(max, [1.0282283, 64.8135, 0.7696068]);
    }

    #[test]
    fn spider_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::Spider.model_key(), "spider");
        assert_eq!(
            EntityModelKind::Spider.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/spider/spider.png",
                size: [64, 32],
            })
        );
    }

    #[test]
    fn enderman_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ENDERMAN_HEAD[0],
            ModelCubeDesc {
                min: [-4.0, -8.0, -4.0],
                size: [8.0, 8.0, 8.0],
                color: ENDERMAN_DARK,
            }
        );
        assert_eq!(
            ENDERMAN_HAT[0],
            ModelCubeDesc {
                min: [-3.5, -7.5, -3.5],
                size: [7.0, 7.0, 7.0],
                color: ENDERMAN_DARK,
            }
        );
        assert_eq!(
            ENDERMAN_BODY[0],
            ModelCubeDesc {
                min: [-4.0, 0.0, -2.0],
                size: [8.0, 12.0, 4.0],
                color: ENDERMAN_DARK,
            }
        );
        assert_eq!(
            ENDERMAN_ARM[0],
            ModelCubeDesc {
                min: [-1.0, -2.0, -1.0],
                size: [2.0, 30.0, 2.0],
                color: ENDERMAN_DARK,
            }
        );
        assert_eq!(
            ENDERMAN_LEG[0],
            ModelCubeDesc {
                min: [-1.0, 0.0, -1.0],
                size: [2.0, 30.0, 2.0],
                color: ENDERMAN_DARK,
            }
        );

        assert_eq!(ENDERMAN_PARTS.len(), 6);
        assert_part_tree(
            &ENDERMAN_PARTS[0],
            [0.0, -13.0, 0.0],
            [0.0, 0.0, 0.0],
            ENDERMAN_HEAD.as_slice(),
            ENDERMAN_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &ENDERMAN_HEAD_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ENDERMAN_HAT.as_slice(),
        );
        assert_part(
            &ENDERMAN_PARTS[1],
            [0.0, -14.0, 0.0],
            [0.0, 0.0, 0.0],
            ENDERMAN_BODY.as_slice(),
        );

        let limb_specs = [
            ([-5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
            ([5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
            ([-2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
            ([2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
        ];
        for (part, (offset, cubes)) in ENDERMAN_PARTS[2..].iter().zip(limb_specs) {
            assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
        }
    }

    #[test]
    fn enderman_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::enderman(141, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 42);
        assert_eq!(mesh.vertices.len(), 168);
        assert_eq!(mesh.indices.len(), 252);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.375, 63.9385, -0.25]);
        assert_close3(max, [0.375, 66.8135, 0.25]);
    }

    #[test]
    fn enderman_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
        assert_eq!(
            EntityModelKind::Enderman.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/enderman/enderman.png",
                size: [64, 32],
            })
        );
    }

    #[test]
    fn iron_golem_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            IRON_GOLEM_HEAD,
            [
                ModelCubeDesc {
                    min: [-4.0, -12.0, -5.5],
                    size: [8.0, 10.0, 8.0],
                    color: IRON_GOLEM_STONE,
                },
                ModelCubeDesc {
                    min: [-1.0, -5.0, -7.5],
                    size: [2.0, 4.0, 2.0],
                    color: IRON_GOLEM_STONE,
                },
            ]
        );
        assert_eq!(
            IRON_GOLEM_BODY,
            [
                ModelCubeDesc {
                    min: [-9.0, -2.0, -6.0],
                    size: [18.0, 12.0, 11.0],
                    color: IRON_GOLEM_STONE,
                },
                ModelCubeDesc {
                    min: [-5.0, 9.5, -3.5],
                    size: [10.0, 6.0, 7.0],
                    color: IRON_GOLEM_STONE,
                },
            ]
        );
        assert_eq!(
            IRON_GOLEM_RIGHT_ARM[0],
            ModelCubeDesc {
                min: [-13.0, -2.5, -3.0],
                size: [4.0, 30.0, 6.0],
                color: IRON_GOLEM_STONE,
            }
        );
        assert_eq!(
            IRON_GOLEM_LEFT_ARM[0],
            ModelCubeDesc {
                min: [9.0, -2.5, -3.0],
                size: [4.0, 30.0, 6.0],
                color: IRON_GOLEM_STONE,
            }
        );
        assert_eq!(
            IRON_GOLEM_RIGHT_LEG[0],
            ModelCubeDesc {
                min: [-3.5, -3.0, -3.0],
                size: [6.0, 16.0, 5.0],
                color: IRON_GOLEM_STONE,
            }
        );
        assert_eq!(IRON_GOLEM_LEFT_LEG, IRON_GOLEM_RIGHT_LEG);

        assert_eq!(IRON_GOLEM_PARTS.len(), 6);
        let part_specs = [
            ([0.0, -7.0, -2.0], IRON_GOLEM_HEAD.as_slice()),
            ([0.0, -7.0, 0.0], IRON_GOLEM_BODY.as_slice()),
            ([0.0, -7.0, 0.0], IRON_GOLEM_RIGHT_ARM.as_slice()),
            ([0.0, -7.0, 0.0], IRON_GOLEM_LEFT_ARM.as_slice()),
            ([-4.0, 11.0, 0.0], IRON_GOLEM_RIGHT_LEG.as_slice()),
            ([5.0, 11.0, 0.0], IRON_GOLEM_LEFT_LEG.as_slice()),
        ];
        for (part, (offset, cubes)) in IRON_GOLEM_PARTS.iter().zip(part_specs) {
            assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
        }
    }

    #[test]
    fn iron_golem_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 48);
        assert_eq!(mesh.vertices.len(), 192);
        assert_eq!(mesh.indices.len(), 288);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.8125, 64.001, -0.3125]);
        assert_close3(max, [0.8125, 66.6885, 0.59375]);
    }

    #[test]
    fn iron_golem_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
        assert_eq!(
            EntityModelKind::IronGolem.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/iron_golem/iron_golem.png",
                size: [128, 128],
            })
        );
    }

    #[test]
    fn snow_golem_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            SNOW_GOLEM_HEAD[0],
            ModelCubeDesc {
                min: [-3.5, -7.5, -3.5],
                size: [7.0, 7.0, 7.0],
                color: SNOW_GOLEM_WHITE,
            }
        );
        assert_eq!(
            SNOW_GOLEM_ARM[0],
            ModelCubeDesc {
                min: [-0.5, 0.5, -0.5],
                size: [11.0, 1.0, 1.0],
                color: SNOW_GOLEM_WHITE,
            }
        );
        assert_eq!(
            SNOW_GOLEM_UPPER_BODY[0],
            ModelCubeDesc {
                min: [-4.5, -9.5, -4.5],
                size: [9.0, 9.0, 9.0],
                color: SNOW_GOLEM_WHITE,
            }
        );
        assert_eq!(
            SNOW_GOLEM_LOWER_BODY[0],
            ModelCubeDesc {
                min: [-5.5, -11.5, -5.5],
                size: [11.0, 11.0, 11.0],
                color: SNOW_GOLEM_WHITE,
            }
        );

        assert_eq!(SNOW_GOLEM_PARTS.len(), 5);
        let part_specs = [
            ([0.0, 4.0, 0.0], [0.0, 0.0, 0.0], SNOW_GOLEM_HEAD.as_slice()),
            ([5.0, 6.0, 1.0], [0.0, 0.0, 1.0], SNOW_GOLEM_ARM.as_slice()),
            (
                [-5.0, 6.0, -1.0],
                [0.0, std::f32::consts::PI, -1.0],
                SNOW_GOLEM_ARM.as_slice(),
            ),
            (
                [0.0, 13.0, 0.0],
                [0.0, 0.0, 0.0],
                SNOW_GOLEM_UPPER_BODY.as_slice(),
            ),
            (
                [0.0, 24.0, 0.0],
                [0.0, 0.0, 0.0],
                SNOW_GOLEM_LOWER_BODY.as_slice(),
            ),
        ];
        for (part, (offset, rotation, cubes)) in SNOW_GOLEM_PARTS.iter().zip(part_specs) {
            assert_part(part, offset, rotation, cubes);
        }
    }

    #[test]
    fn snow_golem_model_mesh_uses_vanilla_body_layer_geometry() {
        let mesh =
            entity_model_mesh(&[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 30);
        assert_eq!(mesh.vertices.len(), 120);
        assert_eq!(mesh.indices.len(), 180);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.6407774, 64.03225, -0.34375]);
        assert_close3(max, [0.6407774, 65.71975, 0.34375]);
    }

    #[test]
    fn snow_golem_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
        assert_eq!(
            EntityModelKind::SnowGolem.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/snow_golem/snow_golem.png",
                size: [64, 64],
            })
        );
    }

    #[test]
    fn witch_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            WITCH_HEAD[0],
            ModelCubeDesc {
                min: [-4.0, -10.0, -4.0],
                size: [8.0, 10.0, 8.0],
                color: WITCH_ROBE,
            }
        );
        assert_eq!(
            WITCH_HAT_4[0],
            ModelCubeDesc {
                min: [-0.25, -0.25, -0.25],
                size: [1.5, 2.5, 1.5],
                color: WITCH_HAT_COLOR,
            }
        );
        assert_eq!(
            WITCH_MOLE[0],
            ModelCubeDesc {
                min: [0.25, 3.25, -6.5],
                size: [0.5, 0.5, 0.5],
                color: WITCH_ROBE,
            }
        );

        assert_eq!(WITCH_PARTS.len(), 5);
        assert_part_tree(
            &WITCH_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_HEAD.as_slice(),
            WITCH_HEAD_CHILDREN.as_slice(),
        );
        assert_part_tree(
            &WITCH_HEAD_CHILDREN[0],
            [-5.0, -10.03125, -5.0],
            [0.0, 0.0, 0.0],
            WITCH_HAT.as_slice(),
            WITCH_HAT_CHILDREN.as_slice(),
        );
        assert_part_tree(
            &WITCH_HAT_CHILDREN[0],
            [1.75, -4.0, 2.0],
            [-0.05235988, 0.0, 0.02617994],
            WITCH_HAT_2.as_slice(),
            WITCH_HAT_2_CHILDREN.as_slice(),
        );
        assert_part_tree(
            &WITCH_HAT_2_CHILDREN[0],
            [1.75, -4.0, 2.0],
            [-0.10471976, 0.0, 0.05235988],
            WITCH_HAT_3.as_slice(),
            WITCH_HAT_3_CHILDREN.as_slice(),
        );
        assert_part(
            &WITCH_HAT_3_CHILDREN[0],
            [1.75, -2.0, 2.0],
            [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
            WITCH_HAT_4.as_slice(),
        );
        assert_part_tree(
            &WITCH_HEAD_CHILDREN[1],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_NOSE.as_slice(),
            WITCH_NOSE_CHILDREN.as_slice(),
        );
        assert_part(
            &WITCH_NOSE_CHILDREN[0],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_MOLE.as_slice(),
        );
        assert_part_tree(
            &WITCH_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_BODY.as_slice(),
            WITCH_BODY_CHILDREN.as_slice(),
        );
        assert_part(
            &WITCH_BODY_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_JACKET.as_slice(),
        );
        assert_part(
            &WITCH_PARTS[2],
            [0.0, 3.0, -1.0],
            [-0.75, 0.0, 0.0],
            WITCH_ARMS.as_slice(),
        );
        assert_part(
            &WITCH_PARTS[3],
            [-2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_LEG.as_slice(),
        );
        assert_part(
            &WITCH_PARTS[4],
            [2.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            WITCH_LEG.as_slice(),
        );
    }

    #[test]
    fn witch_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
        let mesh = entity_model_mesh(&[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)]);

        assert_eq!(mesh.opaque_faces, 84);
        assert_eq!(mesh.vertices.len(), 336);
        assert_eq!(mesh.indices.len(), 504);

        let (min, max) = mesh_extents(&mesh);
        assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
        assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
    }

    #[test]
    fn witch_texture_ref_matches_vanilla_renderer() {
        assert_eq!(EntityModelKind::Witch.model_key(), "witch");
        assert_eq!(
            EntityModelKind::Witch.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/witch/witch.png",
                size: [64, 128],
            })
        );
    }

    #[test]
    fn illager_model_parts_match_vanilla_26_1_body_layer() {
        assert_eq!(
            ILLAGER_HEAD[0],
            ModelCubeDesc {
                min: [-4.0, -10.0, -4.0],
                size: [8.0, 10.0, 8.0],
                color: ILLAGER_ROBE,
            }
        );
        assert_eq!(
            ILLAGER_HAT[0],
            ModelCubeDesc {
                min: [-4.45, -10.45, -4.45],
                size: [8.9, 12.9, 8.9],
                color: ILLAGER_HAT_COLOR,
            }
        );
        assert_eq!(
            ILLAGER_BODY[1],
            ModelCubeDesc {
                min: [-4.5, -0.5, -3.5],
                size: [9.0, 21.0, 7.0],
                color: ILLAGER_ROBE,
            }
        );

        assert_eq!(ILLAGER_SHARED_CROSSED_PARTS.len(), 5);
        assert_part_tree(
            &ILLAGER_SHARED_CROSSED_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_HEAD.as_slice(),
            ILLAGER_HEAD_CHILDREN.as_slice(),
        );
        assert_part(
            &ILLAGER_HEAD_CHILDREN[0],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_NOSE.as_slice(),
        );
        assert_part(
            &ILLAGER_SHARED_CROSSED_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_BODY.as_slice(),
        );
        assert_part_tree(
            &ILLAGER_SHARED_CROSSED_PARTS[2],
            [0.0, 3.0, -1.0],
            [-0.75, 0.0, 0.0],
            ILLAGER_CROSSED_ARMS.as_slice(),
            ILLAGER_CROSSED_ARM_CHILDREN.as_slice(),
        );
        assert_part(
            &ILLAGER_CROSSED_ARM_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_LEFT_SHOULDER.as_slice(),
        );

        assert_eq!(ILLAGER_SHARED_UNCROSSED_PARTS.len(), 6);
        assert_part(
            &ILLAGER_SHARED_UNCROSSED_PARTS[4],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &ILLAGER_SHARED_UNCROSSED_PARTS[5],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_LEFT_ARM.as_slice(),
        );

        assert_part_tree(
            &ILLAGER_ILLUSIONER_PARTS[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_HEAD.as_slice(),
            ILLAGER_HEAD_WITH_HAT_CHILDREN.as_slice(),
        );
        assert_part(
            &ILLAGER_HEAD_WITH_HAT_CHILDREN[0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_HAT.as_slice(),
        );
        assert_part(
            &ILLAGER_HEAD_WITH_HAT_CHILDREN[1],
            [0.0, -2.0, 0.0],
            [0.0, 0.0, 0.0],
            ILLAGER_NOSE.as_slice(),
        );
    }

    #[test]
    fn illager_model_meshes_use_vanilla_scaled_body_layer_geometry() {
        let evoker = entity_model_mesh(&[EntityModelInstance::illager(
            46,
            [0.0, 64.0, 0.0],
            0.0,
            IllagerModelFamily::Evoker,
        )]);
        assert_eq!(evoker.opaque_faces, 54);
        assert_eq!(evoker.vertices.len(), 216);
        assert_eq!(evoker.indices.len(), 324);
        let (evoker_min, evoker_max) = mesh_extents(&evoker);
        assert_close3(evoker_min, [-0.46875, 64.00094, -0.23437501]);
        assert_close3(evoker_max, [0.46875003, 65.993126, 0.3839772]);

        let illusioner = entity_model_mesh(&[EntityModelInstance::illager(
            68,
            [0.0, 64.0, 0.0],
            0.0,
            IllagerModelFamily::Illusioner,
        )]);
        assert_eq!(illusioner.opaque_faces, 60);
        assert_eq!(illusioner.vertices.len(), 240);
        assert_eq!(illusioner.indices.len(), 360);
        let (illusioner_min, illusioner_max) = mesh_extents(&illusioner);
        assert_close3(illusioner_min, [-0.46875, 64.00094, -0.26074222]);
        assert_close3(illusioner_max, [0.46875003, 66.01949, 0.3839772]);

        let pillager = entity_model_mesh(&[EntityModelInstance::illager(
            103,
            [0.0, 64.0, 0.0],
            0.0,
            IllagerModelFamily::Pillager,
        )]);
        assert_eq!(pillager.opaque_faces, 48);
        assert_eq!(pillager.vertices.len(), 192);
        assert_eq!(pillager.indices.len(), 288);
        let (pillager_min, pillager_max) = mesh_extents(&pillager);
        assert_close3(pillager_min, [-0.46875, 64.00094, -0.23437501]);
        assert_close3(pillager_max, [0.46875, 65.993126, 0.3515625]);

        let vindicator = entity_model_mesh(&[EntityModelInstance::illager(
            140,
            [0.0, 64.0, 0.0],
            0.0,
            IllagerModelFamily::Vindicator,
        )]);
        assert_eq!(vindicator.vertices, evoker.vertices);
        assert_eq!(vindicator.indices, evoker.indices);
    }

    #[test]
    fn illager_texture_refs_match_vanilla_renderers() {
        let cases = [
            (
                IllagerModelFamily::Evoker,
                "evoker",
                EntityModelTextureRef {
                    path: "textures/entity/illager/evoker.png",
                    size: [64, 64],
                },
            ),
            (
                IllagerModelFamily::Illusioner,
                "illusioner",
                EntityModelTextureRef {
                    path: "textures/entity/illager/illusioner.png",
                    size: [64, 64],
                },
            ),
            (
                IllagerModelFamily::Pillager,
                "pillager",
                EntityModelTextureRef {
                    path: "textures/entity/illager/pillager.png",
                    size: [64, 64],
                },
            ),
            (
                IllagerModelFamily::Vindicator,
                "vindicator",
                EntityModelTextureRef {
                    path: "textures/entity/illager/vindicator.png",
                    size: [64, 64],
                },
            ),
        ];

        for (family, model_key, texture) in cases {
            let kind = EntityModelKind::Illager { family };
            assert_eq!(kind.model_key(), model_key);
            assert_eq!(kind.vanilla_texture_ref(), Some(texture));
        }
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
    fn armor_stand_parts_match_vanilla_26_1_body_layers() {
        assert_eq!(ARMOR_STAND_PARTS.len(), 10);
        assert_part(
            &ARMOR_STAND_PARTS[0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_HEAD.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[1],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_BODY.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[2],
            [-5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[3],
            [5.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_LEFT_ARM.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[4],
            [-1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_LEG.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[5],
            [1.9, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_LEG.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[6],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[7],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[8],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_SHOULDER_STICK.as_slice(),
        );
        assert_part(
            &ARMOR_STAND_PARTS[9],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            ARMOR_STAND_BASE_PLATE.as_slice(),
        );

        assert_eq!(SMALL_ARMOR_STAND_PARTS.len(), 10);
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[0],
            [0.0, 12.75, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_HEAD.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[1],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_BODY.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[2],
            [-2.5, 13.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_RIGHT_ARM.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[3],
            [2.5, 13.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_LEFT_ARM.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[4],
            [-0.95, 18.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_LEG.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[5],
            [0.95, 18.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_LEG.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[6],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_RIGHT_BODY_STICK.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[7],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_LEFT_BODY_STICK.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[8],
            [0.0, 12.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_SHOULDER_STICK.as_slice(),
        );
        assert_part(
            &SMALL_ARMOR_STAND_PARTS[9],
            [0.0, 18.0, 0.0],
            [0.0, 0.0, 0.0],
            SMALL_ARMOR_STAND_BASE_PLATE.as_slice(),
        );
    }

    #[test]
    fn armor_stand_mesh_uses_vanilla_visibility_and_pose_state() {
        let default = entity_model_mesh(&[EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            true,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )]);
        assert_eq!(default.opaque_faces, 48);
        assert_eq!(default.vertices.len(), 192);
        assert_eq!(default.indices.len(), 288);

        let arms_without_base = entity_model_mesh(&[EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )]);
        assert_eq!(arms_without_base.opaque_faces, 54);
        assert_eq!(arms_without_base.vertices.len(), 216);
        assert_eq!(arms_without_base.indices.len(), 324);

        let small = entity_model_mesh(&[EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
            true,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        )]);
        assert_eq!(small.opaque_faces, 48);
        assert_eq!(small.vertices.len(), 192);
        assert_eq!(small.indices.len(), 288);

        let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
        pose.head = [0.0, 45.0, 0.0];
        pose.body = [0.0, 0.0, 12.0];
        let posed = entity_model_mesh(&[EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            true,
            pose,
        )]);
        assert_eq!(posed.opaque_faces, default.opaque_faces);
        assert_ne!(posed.vertices, default.vertices);
    }

    #[test]
    fn armor_stand_texture_refs_match_vanilla_renderer() {
        let adult = EntityModelKind::ArmorStand {
            small: false,
            show_arms: false,
            show_base_plate: true,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        };
        let small = EntityModelKind::ArmorStand {
            small: true,
            show_arms: false,
            show_base_plate: true,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        };

        assert_eq!(adult.model_key(), "armor_stand");
        assert_eq!(small.model_key(), "armor_stand_small");
        assert_eq!(
            adult.vanilla_texture_ref(),
            Some(EntityModelTextureRef {
                path: "textures/entity/armorstand/armorstand.png",
                size: [64, 64],
            })
        );
        assert_eq!(small.vanilla_texture_ref(), adult.vanilla_texture_ref());
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
            EntityModelKind::ArmorStand {
                small: true,
                show_arms: true,
                show_base_plate: false,
                pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
            }
            .model_key(),
            "armor_stand_small"
        );
        assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
        assert_eq!(
            EntityModelKind::MagmaCube { size: 3 }.model_key(),
            "magma_cube"
        );
        assert_eq!(
            EntityModelKind::Zombie { baby: true }.model_key(),
            "zombie_baby"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: false
            }
            .model_key(),
            "husk"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Husk,
                baby: true
            }
            .model_key(),
            "husk_baby"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: false
            }
            .model_key(),
            "drowned"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::Drowned,
                baby: true
            }
            .model_key(),
            "drowned_baby"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: false
            }
            .model_key(),
            "zombie_villager"
        );
        assert_eq!(
            EntityModelKind::ZombieVariant {
                family: ZombieVariantModelFamily::ZombieVillager,
                baby: true
            }
            .model_key(),
            "zombie_villager_baby"
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: false
            }
            .model_key(),
            "piglin"
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                baby: true
            }
            .model_key(),
            "piglin_baby"
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::PiglinBrute,
                baby: false
            }
            .model_key(),
            "piglin_brute"
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: false
            }
            .model_key(),
            "zombified_piglin"
        );
        assert_eq!(
            EntityModelKind::Piglin {
                family: PiglinModelFamily::ZombifiedPiglin,
                baby: true
            }
            .model_key(),
            "zombified_piglin_baby"
        );
        assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Stray
            }
            .model_key(),
            "stray"
        );
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::Parched
            }
            .model_key(),
            "parched"
        );
        assert_eq!(
            EntityModelKind::SkeletonVariant {
                family: SkeletonModelFamily::WitherSkeleton
            }
            .model_key(),
            "wither_skeleton"
        );
        assert_eq!(EntityModelKind::Cow { baby: true }.model_key(), "cow_baby");
        assert_eq!(
            EntityModelKind::Sheep { baby: true }.model_key(),
            "sheep_baby"
        );
        assert_eq!(
            EntityModelKind::Villager { baby: true }.model_key(),
            "villager_baby"
        );
        assert_eq!(
            EntityModelKind::WanderingTrader.model_key(),
            "wandering_trader"
        );
        assert_eq!(EntityModelKind::Spider.model_key(), "spider");
        assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
        assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
        assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
        assert_eq!(EntityModelKind::Witch.model_key(), "witch");
        assert_eq!(
            EntityModelKind::Illager {
                family: IllagerModelFamily::Evoker
            }
            .model_key(),
            "evoker"
        );
        assert_eq!(
            EntityModelKind::Illager {
                family: IllagerModelFamily::Illusioner
            }
            .model_key(),
            "illusioner"
        );
        assert_eq!(
            EntityModelKind::Illager {
                family: IllagerModelFamily::Pillager
            }
            .model_key(),
            "pillager"
        );
        assert_eq!(
            EntityModelKind::Illager {
                family: IllagerModelFamily::Vindicator
            }
            .model_key(),
            "vindicator"
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

    fn assert_same_geometry(actual: &EntityModelMesh, expected: &EntityModelMesh) {
        assert_eq!(actual.opaque_faces, expected.opaque_faces);
        assert_eq!(actual.indices, expected.indices);
        assert_eq!(actual.vertices.len(), expected.vertices.len());
        for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
            assert_eq!(actual.position, expected.position);
        }
    }

    fn assert_part(
        part: &ModelPartDesc,
        offset: [f32; 3],
        rotation: [f32; 3],
        cubes: &[ModelCubeDesc],
    ) {
        assert_eq!(part.pose.offset, offset);
        assert_eq!(part.pose.rotation, rotation);
        assert_eq!(part.cubes, cubes);
        assert!(part.children.is_empty());
    }

    fn assert_part_tree(
        part: &ModelPartDesc,
        offset: [f32; 3],
        rotation: [f32; 3],
        cubes: &[ModelCubeDesc],
        children: &[ModelPartDesc],
    ) {
        assert_eq!(part.pose.offset, offset);
        assert_eq!(part.pose.rotation, rotation);
        assert_eq!(part.cubes, cubes);
        assert_eq!(part.children, children);
    }
}
