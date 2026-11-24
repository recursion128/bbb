use std::sync::Arc;

pub use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDyeColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}
impl EntityDyeColor {
    pub fn from_vanilla_id(id: i32) -> Self {
        match id {
            0 => Self::White,
            1 => Self::Orange,
            2 => Self::Magenta,
            3 => Self::LightBlue,
            4 => Self::Yellow,
            5 => Self::Lime,
            6 => Self::Pink,
            7 => Self::Gray,
            8 => Self::LightGray,
            9 => Self::Cyan,
            10 => Self::Purple,
            11 => Self::Blue,
            12 => Self::Brown,
            13 => Self::Green,
            14 => Self::Red,
            15 => Self::Black,
            _ => Self::White,
        }
    }

    pub fn vanilla_id(self) -> i32 {
        match self {
            Self::White => 0,
            Self::Orange => 1,
            Self::Magenta => 2,
            Self::LightBlue => 3,
            Self::Yellow => 4,
            Self::Lime => 5,
            Self::Pink => 6,
            Self::Gray => 7,
            Self::LightGray => 8,
            Self::Cyan => 9,
            Self::Purple => 10,
            Self::Blue => 11,
            Self::Brown => 12,
            Self::Green => 13,
            Self::Red => 14,
            Self::Black => 15,
        }
    }

    pub fn texture_diffuse_color(self) -> [f32; 4] {
        let [red, green, blue] = match self {
            Self::White => [249, 255, 254],
            Self::Orange => [249, 128, 29],
            Self::Magenta => [199, 78, 189],
            Self::LightBlue => [58, 179, 218],
            Self::Yellow => [254, 216, 61],
            Self::Lime => [128, 199, 31],
            Self::Pink => [243, 139, 170],
            Self::Gray => [71, 79, 82],
            Self::LightGray => [157, 157, 151],
            Self::Cyan => [22, 156, 156],
            Self::Purple => [137, 50, 184],
            Self::Blue => [60, 68, 170],
            Self::Brown => [131, 84, 50],
            Self::Green => [94, 124, 22],
            Self::Red => [176, 46, 38],
            Self::Black => [29, 29, 33],
        };
        [
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        ]
    }
}
/// Vanilla `ChestType`: which of the three `ChestModel` meshes a chest block renders
/// (`MultiblockChestResources.select` — `single`, the double chest's `left`, or its `right` half).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestModelHalf {
    Single,
    Left,
    Right,
}
/// The five sign meshes: vanilla `PlainSignBlock.Attachment` (GROUND / WALL —
/// `StandingSignRenderer.createSignLayer(standing)`) and
/// `HangingSignBlock.Attachment` (CEILING / CEILING_MIDDLE / WALL —
/// `HangingSignRenderer.createHangingSignLayer(attachment)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignModelAttachment {
    Standing,
    Wall,
    HangingCeiling,
    HangingCeilingMiddle,
    HangingWall,
}
impl SignModelAttachment {
    /// Whether this mesh is a hanging-sign form (`HangingSignRenderer`
    /// transforms, 60px text lines) rather than a plain sign
    /// (`StandingSignRenderer` transforms, 90px text lines).
    pub fn is_hanging(self) -> bool {
        matches!(
            self,
            Self::HangingCeiling | Self::HangingCeilingMiddle | Self::HangingWall
        )
    }
}
/// Vanilla `BedPart` (`BedBlock.PART`): which of the two `BedRenderer` meshes a bed block renders
/// (`createHeadLayer` / `createFootLayer` — each half is its own block and renders only its own
/// piece, `BedRenderer.submit` → `getPieceModel(part)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedModelPart {
    Head,
    Foot,
}
/// Owned pack texture reference admitted to the shared entity atlas for
/// resource-defined special item models. Static entity layers retain their
/// compact `&'static str` references; only pack-defined paths pay for `Arc`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityModelOwnedTextureRef {
    pub path: Arc<str>,
    /// Logical UV extent used by the model layer, independent of the physical
    /// pack image resolution.
    pub size: [u32; 2],
}
impl EntityModelOwnedTextureRef {
    pub fn new(path: impl Into<Arc<str>>, size: [u32; 2]) -> Self {
        Self {
            path: path.into(),
            size,
        }
    }
}

/// Vanilla `StandingSignRenderer.baseTransformation` /
/// `HangingSignRenderer.baseTransformation`, with `instance.position` at the
/// sign block's min corner and `body_rot` carrying `-angle` degrees
/// (`Axis.YP.rotationDegrees(-angle)`):
/// - plain forms: `translate(0.5, 0.5, 0.5) · Ry(-angle)`, the WALL
///   attachment appending `translate(0, -0.3125, -0.4375)`;
/// - hanging forms: `translation(0.5, 0.9375, 0.5) · Ry(-angle) ·
///   translate(0, -0.3125, 0)`.
pub fn sign_base_transformation(
    position: [f32; 3],
    attachment: SignModelAttachment,
    body_rot_degrees: f32,
) -> Mat4 {
    let rotation = Mat4::from_rotation_y(body_rot_degrees.to_radians());
    let base = Mat4::from_translation(Vec3::from_array(position));
    if attachment.is_hanging() {
        base * Mat4::from_translation(Vec3::new(0.5, 0.9375, 0.5))
            * rotation
            * Mat4::from_translation(Vec3::new(0.0, -0.3125, 0.0))
    } else {
        let mut transform = base * Mat4::from_translation(Vec3::new(0.5, 0.5, 0.5)) * rotation;
        if matches!(attachment, SignModelAttachment::Wall) {
            transform *= Mat4::from_translation(Vec3::new(0.0, -0.3125, -0.4375));
        }
        transform
    }
}
