use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockModelGuiLight {
    Front,
    #[default]
    Side,
}

impl BlockModelGuiLight {
    pub fn light_like_block(self) -> bool {
        self == Self::Side
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockModelDisplayContext {
    #[serde(rename = "thirdperson_lefthand")]
    ThirdPersonLeftHand,
    #[serde(rename = "thirdperson_righthand")]
    ThirdPersonRightHand,
    #[serde(rename = "firstperson_lefthand")]
    FirstPersonLeftHand,
    #[serde(rename = "firstperson_righthand")]
    FirstPersonRightHand,
    #[serde(rename = "head")]
    Head,
    #[serde(rename = "gui")]
    Gui,
    #[serde(rename = "ground")]
    Ground,
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "on_shelf")]
    OnShelf,
}

impl BlockModelDisplayContext {
    pub const ALL: [Self; 9] = [
        Self::ThirdPersonLeftHand,
        Self::ThirdPersonRightHand,
        Self::FirstPersonLeftHand,
        Self::FirstPersonRightHand,
        Self::Head,
        Self::Gui,
        Self::Ground,
        Self::Fixed,
        Self::OnShelf,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            Self::ThirdPersonLeftHand => 0,
            Self::ThirdPersonRightHand => 1,
            Self::FirstPersonLeftHand => 2,
            Self::FirstPersonRightHand => 3,
            Self::Head => 4,
            Self::Gui => 5,
            Self::Ground => 6,
            Self::Fixed => 7,
            Self::OnShelf => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockModelDisplayTransform {
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for BlockModelDisplayTransform {
    fn default() -> Self {
        Self {
            rotation: [0.0; 3],
            translation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockModelDisplayTransforms {
    pub third_person_left_hand: BlockModelDisplayTransform,
    pub third_person_right_hand: BlockModelDisplayTransform,
    pub first_person_left_hand: BlockModelDisplayTransform,
    pub first_person_right_hand: BlockModelDisplayTransform,
    pub head: BlockModelDisplayTransform,
    pub gui: BlockModelDisplayTransform,
    pub ground: BlockModelDisplayTransform,
    pub fixed: BlockModelDisplayTransform,
    pub on_shelf: BlockModelDisplayTransform,
}

impl BlockModelDisplayTransforms {
    pub fn get(&self, context: BlockModelDisplayContext) -> BlockModelDisplayTransform {
        match context {
            BlockModelDisplayContext::ThirdPersonLeftHand => self.third_person_left_hand,
            BlockModelDisplayContext::ThirdPersonRightHand => self.third_person_right_hand,
            BlockModelDisplayContext::FirstPersonLeftHand => self.first_person_left_hand,
            BlockModelDisplayContext::FirstPersonRightHand => self.first_person_right_hand,
            BlockModelDisplayContext::Head => self.head,
            BlockModelDisplayContext::Gui => self.gui,
            BlockModelDisplayContext::Ground => self.ground,
            BlockModelDisplayContext::Fixed => self.fixed,
            BlockModelDisplayContext::OnShelf => self.on_shelf,
        }
    }
}

impl Default for BlockModelDisplayTransforms {
    fn default() -> Self {
        Self {
            third_person_left_hand: BlockModelDisplayTransform::default(),
            third_person_right_hand: BlockModelDisplayTransform::default(),
            first_person_left_hand: BlockModelDisplayTransform::default(),
            first_person_right_hand: BlockModelDisplayTransform::default(),
            head: BlockModelDisplayTransform::default(),
            gui: BlockModelDisplayTransform::default(),
            ground: BlockModelDisplayTransform::default(),
            fixed: BlockModelDisplayTransform::default(),
            on_shelf: BlockModelDisplayTransform::default(),
        }
    }
}

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
    pub(crate) fn uniform(texture: impl Into<String>) -> Self {
        let texture = texture.into();
        Self {
            textures: std::array::from_fn(|_| texture.clone()),
            tint_indices: [None; 6],
            force_translucent: [false; 6],
        }
    }

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockModelShape {
    Cube,
    Cross { shade: bool, light_emission: u8 },
    Crosses(Vec<BlockModelCross>),
    Box(BlockModelBox),
    Boxes(Vec<BlockModelBox>),
    Quads(Vec<BlockModelQuad>),
    Custom,
}

impl Default for BlockModelShape {
    fn default() -> Self {
        Self::Custom
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRenderModel {
    pub face_textures: BlockFaceTextures,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub particle_texture: Option<String>,
    pub shape: BlockModelShape,
    pub use_ambient_occlusion: bool,
    #[serde(default)]
    pub gui_light: BlockModelGuiLight,
    #[serde(default)]
    pub display_transforms: BlockModelDisplayTransforms,
}

impl BlockRenderModel {
    pub(crate) fn empty() -> Self {
        Self {
            face_textures: BlockFaceTextures::uniform("minecraft:block/stone"),
            particle_texture: Some("minecraft:block/stone".to_string()),
            shape: BlockModelShape::Boxes(Vec::new()),
            use_ambient_occlusion: true,
            gui_light: BlockModelGuiLight::default(),
            display_transforms: BlockModelDisplayTransforms::default(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockModelQuad {
    pub face: BlockModelFace,
    pub corners: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub uvs: [[f32; 2]; 4],
    pub cull: Option<BlockModelFace>,
    pub tint_index: Option<i32>,
    pub texture: Option<String>,
    pub force_translucent: bool,
    pub shade: bool,
    pub light_emission: u8,
}
