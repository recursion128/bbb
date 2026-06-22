use super::{
    ModelCubeDesc, ModelPartDesc, TexturedModelCubeDesc, TexturedModelPartDesc, PART_POSE_ZERO,
};

// The blaze fallback paints the head and rods a single fiery orange.
pub(in crate::entity_models) const BLAZE_ORANGE: [f32; 4] = [0.94, 0.55, 0.10, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_BLAZE: &str = "minecraft:blaze#main";

pub(in crate::entity_models) const BLAZE_HEAD_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -4.0, -4.0],
    size: [8.0, 8.0, 8.0],
    color: BLAZE_ORANGE,
}];

// Vanilla reuses one `rod` CubeListBuilder for all twelve rods: addBox(0, 0, 0, 2, 8, 2).
pub(in crate::entity_models) const BLAZE_ROD_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [2.0, 8.0, 2.0],
    color: BLAZE_ORANGE,
}];

const BLAZE_ROD_PART: ModelPartDesc = ModelPartDesc {
    // The rod offsets are overwritten every frame by `blaze_rod_offset`, so the layer rest
    // offset is irrelevant; vanilla never displays the un-posed rods.
    pose: PART_POSE_ZERO,
    cubes: &BLAZE_ROD_CUBE,
    children: &[],
};

// Vanilla 26.1 ModelLayers.BLAZE: BlazeModel.createBodyLayer() — a head plus twelve rods,
// all positioned by `BlazeModel.setupAnim` from `ageInTicks`.
pub(in crate::entity_models) const BLAZE_PARTS: [ModelPartDesc; 13] = [
    ModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BLAZE_HEAD_CUBE,
        children: &[],
    },
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
    BLAZE_ROD_PART,
];

pub(in crate::entity_models) const BLAZE_TEXTURED_HEAD_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -4.0, -4.0],
        size: [8.0, 8.0, 8.0],
        uv_size: [8.0, 8.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

// Vanilla reuses `texOffs(0, 16)` for every rod, so all twelve sample the same region.
pub(in crate::entity_models) const BLAZE_TEXTURED_ROD_CUBE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 16.0],
        mirror: false,
    }];

const BLAZE_TEXTURED_ROD_PART: TexturedModelPartDesc = TexturedModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &BLAZE_TEXTURED_ROD_CUBE,
    children: &[],
};

pub(in crate::entity_models) const BLAZE_TEXTURED_PARTS: [TexturedModelPartDesc; 13] = [
    TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &BLAZE_TEXTURED_HEAD_CUBE,
        children: &[],
    },
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
    BLAZE_TEXTURED_ROD_PART,
];

/// The number of rods in the blaze body layer (parts `1..=12`; part `0` is the head).
pub(in crate::entity_models) const BLAZE_ROD_COUNT: usize = 12;

/// Vanilla `BlazeModel.setupAnim` rod placement: the twelve rods orbit in three rings of
/// four, their `x`/`y`/`z` offsets SET every frame from `ageInTicks`. Ring 0 (rods 0..4) at
/// radius 9, ring 1 (4..8) at radius 7, ring 2 (8..12) at radius 5; each ring spins at its
/// own rate and the rods bob in `y`. Returns the part offset for rod `index`.
pub(in crate::entity_models) fn blaze_rod_offset(index: usize, age_in_ticks: f32) -> [f32; 3] {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
    let i = index as f32;
    let (radius, y, base_angle) = if index < 4 {
        (
            9.0,
            -2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            age_in_ticks * PI * -0.1,
        )
    } else if index < 8 {
        (
            7.0,
            2.0 + ((2.0 * i + age_in_ticks) * 0.25).cos(),
            FRAC_PI_4 + age_in_ticks * PI * 0.03,
        )
    } else {
        (
            5.0,
            11.0 + ((1.5 * i + age_in_ticks) * 0.5).cos(),
            0.47123894 + age_in_ticks * PI * -0.05,
        )
    };
    let angle = base_angle + (index % 4) as f32 * FRAC_PI_2;
    [angle.cos() * radius, y, angle.sin() * radius]
}
