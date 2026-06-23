use super::PartPose;
use crate::entity_models::model::{ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_STRAY_OUTER_LAYER: &str = "minecraft:stray#outer";
pub(in crate::entity_models) const MODEL_LAYER_BOGGED_OUTER_LAYER: &str = "minecraft:bogged#outer";

/// The clothing overlay renders textured-only (the stray frost / bogged mushroom layer has no colored
/// debug variant), so every cube's `color` is an unused placeholder; only the geometry/UV matter.
const CLOTHING_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// A textured-only inflated overlay cube (vanilla `addBox` with a `CubeDeformation`): the `uv_size`
/// stays the base box while `min`/`size` carry the inflation. `color` is the unused placeholder.
const fn clothing_cube(
    min: [f32; 3],
    size: [f32; 3],
    uv_size: [f32; 3],
    tex: [f32; 2],
    mirror: bool,
) -> ModelCube {
    ModelCube::new(min, size, CLOTHING_COLOR, uv_size, tex, mirror)
}

// Vanilla 26.1 ModelLayers.STRAY_OUTER_LAYER:
// HumanoidModel.createMesh(new CubeDeformation(0.25F), 0.0F), 64x32.
pub(in crate::entity_models) const STRAY_OUTER_HEAD: ModelCube = clothing_cube(
    [-4.25, -8.25, -4.25],
    [8.5, 8.5, 8.5],
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
);

pub(in crate::entity_models) const STRAY_OUTER_HAT: ModelCube = clothing_cube(
    [-4.75, -8.75, -4.75],
    [9.5, 9.5, 9.5],
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
);

pub(in crate::entity_models) const STRAY_OUTER_BODY: ModelCube = clothing_cube(
    [-4.25, -0.25, -2.25],
    [8.5, 12.5, 4.5],
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
);

pub(in crate::entity_models) const STRAY_OUTER_RIGHT_ARM: ModelCube = clothing_cube(
    [-3.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
);

pub(in crate::entity_models) const STRAY_OUTER_LEFT_ARM: ModelCube = clothing_cube(
    [-1.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    true,
);

pub(in crate::entity_models) const STRAY_OUTER_RIGHT_LEG: ModelCube = clothing_cube(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
);

pub(in crate::entity_models) const STRAY_OUTER_LEFT_LEG: ModelCube = clothing_cube(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    true,
);

// Vanilla 26.1 ModelLayers.BOGGED_OUTER_LAYER:
// HumanoidModel.createMesh(new CubeDeformation(0.2F), 0.0F), 64x32.
pub(in crate::entity_models) const BOGGED_OUTER_HEAD: ModelCube = clothing_cube(
    [-4.2, -8.2, -4.2],
    [8.4, 8.4, 8.4],
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
);

pub(in crate::entity_models) const BOGGED_OUTER_HAT: ModelCube = clothing_cube(
    [-4.7, -8.7, -4.7],
    [9.4, 9.4, 9.4],
    [8.0, 8.0, 8.0],
    [32.0, 0.0],
    false,
);

pub(in crate::entity_models) const BOGGED_OUTER_BODY: ModelCube = clothing_cube(
    [-4.2, -0.2, -2.2],
    [8.4, 12.4, 4.4],
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
);

pub(in crate::entity_models) const BOGGED_OUTER_RIGHT_ARM: ModelCube = clothing_cube(
    [-3.2, -2.2, -2.2],
    [4.4, 12.4, 4.4],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
);

pub(in crate::entity_models) const BOGGED_OUTER_LEFT_ARM: ModelCube = clothing_cube(
    [-1.2, -2.2, -2.2],
    [4.4, 12.4, 4.4],
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    true,
);

pub(in crate::entity_models) const BOGGED_OUTER_RIGHT_LEG: ModelCube = clothing_cube(
    [-2.2, -0.2, -2.2],
    [4.4, 12.4, 4.4],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
);

pub(in crate::entity_models) const BOGGED_OUTER_LEFT_LEG: ModelCube = clothing_cube(
    [-2.2, -0.2, -2.2],
    [4.4, 12.4, 4.4],
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    true,
);

/// Shared clothing limb part poses (vanilla `HumanoidModel.createMesh`): the arms sit at `±5`, the
/// legs at `±1.9` (the clothing overlay's slightly narrower leg offset).
pub(in crate::entity_models) const CLOTHING_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CLOTHING_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CLOTHING_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CLOTHING_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a textured-only clothing leaf part at `pose` carrying `cube`.
fn clothing_part(pose: PartPose, cube: ModelCube) -> ModelPart {
    ModelPart::leaf(pose, vec![cube])
}

/// Builds the `HumanoidModel`-shaped clothing root (`head` -> `hat`, `body`, the arms/legs) from the
/// six per-part cubes. Shared assembly for the stray and bogged overlays (same tree shape and poses;
/// only the inflated cube geometry/UV differs).
fn clothing_root(
    head: ModelCube,
    hat: ModelCube,
    body: ModelCube,
    right_arm: ModelCube,
    left_arm: ModelCube,
    right_leg: ModelCube,
    left_leg: ModelCube,
) -> ModelPart {
    let head = ModelPart::new(
        super::PART_POSE_ZERO,
        vec![head],
        vec![("hat", clothing_part(super::PART_POSE_ZERO, hat))],
    );
    ModelPart::new(
        super::PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", head),
            ("body", clothing_part(super::PART_POSE_ZERO, body)),
            (
                "right_arm",
                clothing_part(CLOTHING_RIGHT_ARM_POSE, right_arm),
            ),
            ("left_arm", clothing_part(CLOTHING_LEFT_ARM_POSE, left_arm)),
            (
                "right_leg",
                clothing_part(CLOTHING_RIGHT_LEG_POSE, right_leg),
            ),
            ("left_leg", clothing_part(CLOTHING_LEFT_LEG_POSE, left_leg)),
        ],
    )
}

/// Builds the stray frost clothing root (the `0.25` inflated `HumanoidModel` overlay).
pub(in crate::entity_models) fn stray_clothing_root() -> ModelPart {
    clothing_root(
        STRAY_OUTER_HEAD,
        STRAY_OUTER_HAT,
        STRAY_OUTER_BODY,
        STRAY_OUTER_RIGHT_ARM,
        STRAY_OUTER_LEFT_ARM,
        STRAY_OUTER_RIGHT_LEG,
        STRAY_OUTER_LEFT_LEG,
    )
}

/// Builds the bogged mushroom clothing root (the `0.2` inflated `HumanoidModel` overlay).
pub(in crate::entity_models) fn bogged_clothing_root() -> ModelPart {
    clothing_root(
        BOGGED_OUTER_HEAD,
        BOGGED_OUTER_HAT,
        BOGGED_OUTER_BODY,
        BOGGED_OUTER_RIGHT_ARM,
        BOGGED_OUTER_LEFT_ARM,
        BOGGED_OUTER_RIGHT_LEG,
        BOGGED_OUTER_LEFT_LEG,
    )
}
