use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};
use std::f32::consts::{FRAC_PI_2, PI};

const PI_3_HALVES: f32 = PI * 1.5;

// The colored fallback paints the whole cart a neutral iron grey.
pub(in crate::entity_models) const MINECART_GRAY: [f32; 4] = [0.34, 0.35, 0.37, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_MINECART: &str = "minecraft:minecart#main";

// Vanilla 26.1 MinecartModel.createBodyLayer(): the floor `bottom` panel (`texOffs(0, 10)`,
// a 20x16x2 box laid flat) plus four identical 16x8x2 wall panels (`texOffs(0, 0)`) rotated
// to box in the cart. None are mirrored.
const MINECART_BOTTOM_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 0.0],
    rotation: [FRAC_PI_2, 0.0, 0.0],
};
const MINECART_FRONT_POSE: PartPose = PartPose {
    offset: [-9.0, 4.0, 0.0],
    rotation: [0.0, PI_3_HALVES, 0.0],
};
const MINECART_BACK_POSE: PartPose = PartPose {
    offset: [9.0, 4.0, 0.0],
    rotation: [0.0, FRAC_PI_2, 0.0],
};
const MINECART_LEFT_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, -7.0],
    rotation: [0.0, PI, 0.0],
};
const MINECART_RIGHT_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, 7.0],
    rotation: [0.0, 0.0, 0.0],
};

const MINECART_BOTTOM_MIN: [f32; 3] = [-10.0, -8.0, -1.0];
const MINECART_BOTTOM_SIZE: [f32; 3] = [20.0, 16.0, 2.0];
const MINECART_WALL_MIN: [f32; 3] = [-8.0, -9.0, -1.0];
const MINECART_WALL_SIZE: [f32; 3] = [16.0, 8.0, 2.0];

const fn minecart_colored_part(pose: PartPose, cubes: &'static [ModelCubeDesc]) -> ModelPartDesc {
    ModelPartDesc {
        pose,
        cubes,
        children: &[],
    }
}

const MINECART_BOTTOM_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: MINECART_BOTTOM_MIN,
    size: MINECART_BOTTOM_SIZE,
    color: MINECART_GRAY,
}];
const MINECART_WALL_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: MINECART_WALL_MIN,
    size: MINECART_WALL_SIZE,
    color: MINECART_GRAY,
}];

pub(in crate::entity_models) const MINECART_PARTS: [ModelPartDesc; 5] = [
    minecart_colored_part(MINECART_BOTTOM_POSE, &MINECART_BOTTOM_CUBE),
    minecart_colored_part(MINECART_FRONT_POSE, &MINECART_WALL_CUBE),
    minecart_colored_part(MINECART_BACK_POSE, &MINECART_WALL_CUBE),
    minecart_colored_part(MINECART_LEFT_POSE, &MINECART_WALL_CUBE),
    minecart_colored_part(MINECART_RIGHT_POSE, &MINECART_WALL_CUBE),
];

const fn minecart_textured_part(
    pose: PartPose,
    cubes: &'static [TexturedModelCubeDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose,
        cubes,
        children: &[],
    }
}

const MINECART_TEXTURED_BOTTOM_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: MINECART_BOTTOM_MIN,
    size: MINECART_BOTTOM_SIZE,
    uv_size: MINECART_BOTTOM_SIZE,
    tex: [0.0, 10.0],
    mirror: false,
}];
const MINECART_TEXTURED_WALL_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: MINECART_WALL_MIN,
    size: MINECART_WALL_SIZE,
    uv_size: MINECART_WALL_SIZE,
    tex: [0.0, 0.0],
    mirror: false,
}];

pub(in crate::entity_models) const MINECART_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    minecart_textured_part(MINECART_BOTTOM_POSE, &MINECART_TEXTURED_BOTTOM_CUBE),
    minecart_textured_part(MINECART_FRONT_POSE, &MINECART_TEXTURED_WALL_CUBE),
    minecart_textured_part(MINECART_BACK_POSE, &MINECART_TEXTURED_WALL_CUBE),
    minecart_textured_part(MINECART_LEFT_POSE, &MINECART_TEXTURED_WALL_CUBE),
    minecart_textured_part(MINECART_RIGHT_POSE, &MINECART_TEXTURED_WALL_CUBE),
];
