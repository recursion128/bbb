use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockModelFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl BlockModelFace {
    pub const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }

    pub(super) fn from_name(name: &str) -> Option<Self> {
        match name {
            "down" => Some(Self::Down),
            "up" => Some(Self::Up),
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockFaceTextures {
    pub textures: [String; 6],
    pub tint_indices: [Option<i32>; 6],
    pub force_translucent: [bool; 6],
}

impl BlockFaceTextures {
    pub fn get(&self, face: BlockModelFace) -> &str {
        &self.textures[face.index()]
    }

    pub fn tint_index(&self, face: BlockModelFace) -> Option<i32> {
        self.tint_indices[face.index()]
    }

    pub fn force_translucent(&self, face: BlockModelFace) -> bool {
        self.force_translucent[face.index()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockModelShape {
    Cube,
    Cross { shade: bool, light_emission: u8 },
    Crosses(Vec<BlockModelCross>),
    Box(BlockModelBox),
    Boxes(Vec<BlockModelBox>),
    Custom,
}

impl Default for BlockModelShape {
    fn default() -> Self {
        Self::Custom
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRenderModel {
    pub face_textures: BlockFaceTextures,
    pub shape: BlockModelShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockModelBox {
    pub from: [u8; 3],
    pub to: [u8; 3],
    pub face_present: [bool; 6],
    pub face_uvs: [[u8; 4]; 6],
    pub face_uv_rotations: [u8; 6],
    pub face_shade: [bool; 6],
    pub face_light_emission: [u8; 6],
    pub face_cull: [Option<BlockModelFace>; 6],
    pub face_tint_indices: [Option<i32>; 6],
    pub face_textures: [Option<String>; 6],
    pub face_force_translucent: [bool; 6],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockModelCross {
    pub face_textures: [Option<String>; 6],
    pub face_tint_indices: [Option<i32>; 6],
    pub face_force_translucent: [bool; 6],
    pub shade: bool,
    pub light_emission: u8,
}
