use super::{
    PartPose, ARMOR_CHAINMAIL_HUMANOID_TEXTURE_REF, ARMOR_CHAINMAIL_LEGGINGS_TEXTURE_REF,
    ARMOR_COPPER_HUMANOID_TEXTURE_REF, ARMOR_COPPER_LEGGINGS_TEXTURE_REF,
    ARMOR_DIAMOND_HUMANOID_TEXTURE_REF, ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF,
    ARMOR_GOLD_HUMANOID_TEXTURE_REF, ARMOR_GOLD_LEGGINGS_TEXTURE_REF,
    ARMOR_IRON_HUMANOID_TEXTURE_REF, ARMOR_IRON_LEGGINGS_TEXTURE_REF,
    ARMOR_LEATHER_HUMANOID_TEXTURE_REF, ARMOR_LEATHER_LEGGINGS_TEXTURE_REF,
    ARMOR_NETHERITE_HUMANOID_TEXTURE_REF, ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF,
    ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF, PART_POSE_ZERO,
};
use crate::entity_models::catalog::{EntityArmorMaterial, EntityModelTextureRef};
use crate::entity_models::model::{ModelCube, ModelPart};

// Vanilla 26.1 humanoid armor layer (`HumanoidModel.createBaseArmorMesh` / `createArmorMeshSet`,
// atlas 64×32). The armor is the standard humanoid mesh (`createMesh`) grown by a `CubeDeformation`
// so it floats just outside the body, then split into four per-slot models by `retainExactParts`:
//   HEAD  (helmet):     head (+ hat child), OUTER deformation `1.0` (hat `g.extend(0.5)` = `1.5`)
//   CHEST (chestplate): body + both arms,   OUTER deformation `1.0`
//   LEGS  (leggings):   body + both legs,   INNER deformation `0.5` (legs `g.extend(-0.1)` = `0.4`)
//   FEET  (boots):      both legs,          OUTER deformation `1.0` (legs `g.extend(-0.1)` = `0.9`)
// The legs are grown by `g - 0.1` so the leggings (inner) and the body/boots (outer) layers do not
// z-fight where they overlap. Each per-slot tree is draped on the host humanoid model's posed limbs
// via [`ModelPart::copy_child_poses_from`] (vanilla `copyPropertiesTo`), so the armor inherits the
// host's `setup_anim` without re-running it. The mesh carries the textured `uv_size` / `texOffs`; the
// colored debug tint is a never-rendered placeholder (armor is a textured-only overlay).

const ARMOR_PLACEHOLDER_COLOR: [f32; 4] = [0.55, 0.55, 0.58, 1.0];

const OUTER_ARMOR_DEFORMATION: f32 = 1.0;
const INNER_ARMOR_DEFORMATION: f32 = 0.5;

/// Vanilla `CubeListBuilder.addBox(..., g)` with `CubeDeformation(g)`: grows the box (`min -= g`,
/// `size += 2·g`) while keeping the base `texOffs` and the base box dims for the UV mapping.
const fn armor_cube(
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
    g: f32,
) -> ModelCube {
    ModelCube::new(
        [min[0] - g, min[1] - g, min[2] - g],
        [size[0] + 2.0 * g, size[1] + 2.0 * g, size[2] + 2.0 * g],
        ARMOR_PLACEHOLDER_COLOR,
        size,
        tex,
        mirror,
    )
}

// The base humanoid boxes (`HumanoidModel.createMesh`, g = 0). Grown per slot below.
// `head` texOffs(0,0) 8×8×8; `hat` texOffs(32,0) 8×8×8; `body` texOffs(16,16) 8×12×4;
// arms texOffs(40,16) 4×12×4; legs texOffs(0,16) 4×12×4.
const HEAD_MIN: [f32; 3] = [-4.0, -8.0, -4.0];
const HEAD_SIZE: [f32; 3] = [8.0, 8.0, 8.0];
const BODY_MIN: [f32; 3] = [-4.0, 0.0, -2.0];
const BODY_SIZE: [f32; 3] = [8.0, 12.0, 4.0];
const RIGHT_ARM_MIN: [f32; 3] = [-3.0, -2.0, -2.0];
const LEFT_ARM_MIN: [f32; 3] = [-1.0, -2.0, -2.0];
const ARM_SIZE: [f32; 3] = [4.0, 12.0, 4.0];
const LEG_MIN: [f32; 3] = [-2.0, 0.0, -2.0];
const LEG_SIZE: [f32; 3] = [4.0, 12.0, 4.0];

// OUTER model parts (g = 1.0; hat `g + 0.5 = 1.5`; legs `g - 0.1 = 0.9`).
const ARMOR_HEAD_OUTER: [ModelCube; 1] = [armor_cube(
    HEAD_MIN,
    HEAD_SIZE,
    [0.0, 0.0],
    false,
    OUTER_ARMOR_DEFORMATION,
)];
const ARMOR_HAT_OUTER: [ModelCube; 1] = [armor_cube(
    HEAD_MIN,
    HEAD_SIZE,
    [32.0, 0.0],
    false,
    OUTER_ARMOR_DEFORMATION + 0.5,
)];
const ARMOR_BODY_OUTER: [ModelCube; 1] = [armor_cube(
    BODY_MIN,
    BODY_SIZE,
    [16.0, 16.0],
    false,
    OUTER_ARMOR_DEFORMATION,
)];
const ARMOR_RIGHT_ARM_OUTER: [ModelCube; 1] = [armor_cube(
    RIGHT_ARM_MIN,
    ARM_SIZE,
    [40.0, 16.0],
    false,
    OUTER_ARMOR_DEFORMATION,
)];
const ARMOR_LEFT_ARM_OUTER: [ModelCube; 1] = [armor_cube(
    LEFT_ARM_MIN,
    ARM_SIZE,
    [40.0, 16.0],
    true,
    OUTER_ARMOR_DEFORMATION,
)];
const ARMOR_RIGHT_LEG_OUTER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    false,
    OUTER_ARMOR_DEFORMATION - 0.1,
)];
const ARMOR_LEFT_LEG_OUTER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    true,
    OUTER_ARMOR_DEFORMATION - 0.1,
)];

// INNER model parts (g = 0.5; legs `g - 0.1 = 0.4`).
const ARMOR_BODY_INNER: [ModelCube; 1] = [armor_cube(
    BODY_MIN,
    BODY_SIZE,
    [16.0, 16.0],
    false,
    INNER_ARMOR_DEFORMATION,
)];
const ARMOR_RIGHT_LEG_INNER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    false,
    INNER_ARMOR_DEFORMATION - 0.1,
)];
const ARMOR_LEFT_LEG_INNER: [ModelCube; 1] = [armor_cube(
    LEG_MIN,
    LEG_SIZE,
    [0.0, 16.0],
    true,
    INNER_ARMOR_DEFORMATION - 0.1,
)];

// Vanilla `HumanoidModel.createMesh` bind poses (yOffset = 0).
const HEAD_POSE: PartPose = PART_POSE_ZERO;
const BODY_POSE: PartPose = PART_POSE_ZERO;
const RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The four humanoid armor slots, in the vanilla `HumanoidArmorLayer.submit` render order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum HumanoidArmorSlot {
    Chest,
    Legs,
    Feet,
    Head,
}

impl HumanoidArmorSlot {
    /// The humanoid part names this slot's armor model retains — also the parts whose host pose is
    /// copied onto the armor tree.
    pub(in crate::entity_models) fn part_names(self) -> &'static [&'static str] {
        match self {
            Self::Head => &["head"],
            Self::Chest => &["body", "right_arm", "left_arm"],
            Self::Legs => &["body", "right_leg", "left_leg"],
            Self::Feet => &["right_leg", "left_leg"],
        }
    }

    /// Whether this slot uses the inner (leggings) model and the `humanoid_leggings` texture; only
    /// `LEGS` does (vanilla `HumanoidArmorLayer.usesInnerModel`).
    pub(in crate::entity_models) fn uses_inner_model(self) -> bool {
        matches!(self, Self::Legs)
    }

    /// Builds this slot's armor overlay tree: a fresh root carrying exactly the slot's parts, each at
    /// its humanoid bind pose with the inflated armor cubes. The host's posed limbs are copied in by
    /// [`ModelPart::copy_child_poses_from`] before rendering.
    pub(in crate::entity_models) fn build_tree(self) -> ModelPart {
        let children: Vec<(&'static str, ModelPart)> = match self {
            Self::Head => vec![(
                "head",
                ModelPart::new(
                    HEAD_POSE,
                    ARMOR_HEAD_OUTER.to_vec(),
                    vec![(
                        "hat",
                        ModelPart::leaf(PART_POSE_ZERO, ARMOR_HAT_OUTER.to_vec()),
                    )],
                ),
            )],
            Self::Chest => vec![
                (
                    "body",
                    ModelPart::leaf(BODY_POSE, ARMOR_BODY_OUTER.to_vec()),
                ),
                (
                    "right_arm",
                    ModelPart::leaf(RIGHT_ARM_POSE, ARMOR_RIGHT_ARM_OUTER.to_vec()),
                ),
                (
                    "left_arm",
                    ModelPart::leaf(LEFT_ARM_POSE, ARMOR_LEFT_ARM_OUTER.to_vec()),
                ),
            ],
            Self::Legs => vec![
                (
                    "body",
                    ModelPart::leaf(BODY_POSE, ARMOR_BODY_INNER.to_vec()),
                ),
                (
                    "right_leg",
                    ModelPart::leaf(RIGHT_LEG_POSE, ARMOR_RIGHT_LEG_INNER.to_vec()),
                ),
                (
                    "left_leg",
                    ModelPart::leaf(LEFT_LEG_POSE, ARMOR_LEFT_LEG_INNER.to_vec()),
                ),
            ],
            Self::Feet => vec![
                (
                    "right_leg",
                    ModelPart::leaf(RIGHT_LEG_POSE, ARMOR_RIGHT_LEG_OUTER.to_vec()),
                ),
                (
                    "left_leg",
                    ModelPart::leaf(LEFT_LEG_POSE, ARMOR_LEFT_LEG_OUTER.to_vec()),
                ),
            ],
        };
        ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
    }
}

/// The vanilla `EquipmentLayerRenderer.getColorForLayer` per-layer tint: leather is the only dyeable
/// humanoid material, so undyed it tints by its default `DyedItemColor.LEATHER_COLOR` (`0xA06540`);
/// every other material is non-dyeable and renders white (vanilla color `-1`). The per-item
/// `DyedItemColor` override (custom-dyed leather) is deferred pending dye-color projection.
pub(in crate::entity_models) fn armor_layer_tint(material: EntityArmorMaterial) -> [f32; 4] {
    match material {
        EntityArmorMaterial::Leather => [
            0xA0 as f32 / 255.0,
            0x65 as f32 / 255.0,
            0x40 as f32 / 255.0,
            1.0,
        ],
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

/// The equipment-asset texture for a given armor material in a given slot: the `humanoid_leggings`
/// variant for the inner (legs) slot, the `humanoid` variant otherwise (vanilla
/// `EquipmentClientInfo.LayerType` → `getTextureLocation`). `TurtleScute` only ever fills the head
/// slot, so its (non-existent) leggings texture falls back to its humanoid texture.
pub(in crate::entity_models) fn armor_slot_texture(
    material: EntityArmorMaterial,
    slot: HumanoidArmorSlot,
) -> EntityModelTextureRef {
    use EntityArmorMaterial::*;
    if slot.uses_inner_model() {
        match material {
            Leather => ARMOR_LEATHER_LEGGINGS_TEXTURE_REF,
            Copper => ARMOR_COPPER_LEGGINGS_TEXTURE_REF,
            Chainmail => ARMOR_CHAINMAIL_LEGGINGS_TEXTURE_REF,
            Iron => ARMOR_IRON_LEGGINGS_TEXTURE_REF,
            Gold => ARMOR_GOLD_LEGGINGS_TEXTURE_REF,
            Diamond => ARMOR_DIAMOND_LEGGINGS_TEXTURE_REF,
            Netherite => ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF,
            TurtleScute => ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF,
        }
    } else {
        match material {
            Leather => ARMOR_LEATHER_HUMANOID_TEXTURE_REF,
            Copper => ARMOR_COPPER_HUMANOID_TEXTURE_REF,
            Chainmail => ARMOR_CHAINMAIL_HUMANOID_TEXTURE_REF,
            Iron => ARMOR_IRON_HUMANOID_TEXTURE_REF,
            Gold => ARMOR_GOLD_HUMANOID_TEXTURE_REF,
            Diamond => ARMOR_DIAMOND_HUMANOID_TEXTURE_REF,
            TurtleScute => ARMOR_TURTLE_SCUTE_HUMANOID_TEXTURE_REF,
            Netherite => ARMOR_NETHERITE_HUMANOID_TEXTURE_REF,
        }
    }
}
