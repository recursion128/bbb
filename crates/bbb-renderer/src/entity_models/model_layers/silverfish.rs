use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};

// The silverfish fallback paints its body a stony gray.
pub(in crate::entity_models) const SILVERFISH_GRAY: [f32; 4] = [0.50, 0.50, 0.53, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_SILVERFISH: &str = "minecraft:silverfish#main";

/// The number of body segments in the silverfish body layer (parts `0..=6`); the three
/// overlay `layer` parts follow at `7..=9`.
pub(in crate::entity_models) const SILVERFISH_SEGMENT_COUNT: usize = 7;

/// The number of overlay `layer` parts in the silverfish body layer.
pub(in crate::entity_models) const SILVERFISH_LAYER_COUNT: usize = 3;

// Vanilla 26.1 SilverfishModel.createBodyLayer: seven nested body segments
// (BODY_SIZES[i] = (sx, sy, sz), each addBox(-sx/2, 0, -sz/2, sx, sy, sz) at offset
// (0, 24 - sy, placement)) plus three wider overlay layers riding segments 2/4/1.
const SILVERFISH_SEGMENT_0_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.0],
    size: [3.0, 2.0, 2.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_1_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -1.0],
    size: [4.0, 3.0, 2.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_2_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -1.5],
    size: [6.0, 4.0, 3.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_3_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -1.5],
    size: [3.0, 3.0, 3.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_4_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.5],
    size: [2.0, 2.0, 3.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_5_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 1.0, 2.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_SEGMENT_6_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, -1.0],
    size: [1.0, 1.0, 2.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_LAYER_0_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, 0.0, -1.5],
    size: [10.0, 8.0, 3.0],
    color: SILVERFISH_GRAY,
}];
const SILVERFISH_LAYER_1_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -1.5],
    size: [6.0, 4.0, 3.0],
    color: SILVERFISH_GRAY,
}];
// Vanilla quirk: layer2 takes its z-min from BODY_SIZES[4][2] (3 => -1.5) but its z-size
// from BODY_SIZES[1][2] (2), so it is offset asymmetrically in z.
const SILVERFISH_LAYER_2_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -1.5],
    size: [6.0, 5.0, 2.0],
    color: SILVERFISH_GRAY,
}];

const fn silverfish_part(offset: [f32; 3], cubes: &'static [ModelCubeDesc]) -> ModelPartDesc {
    ModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children: &[],
    }
}

pub(in crate::entity_models) const SILVERFISH_PARTS: [ModelPartDesc; 10] = [
    silverfish_part([0.0, 22.0, -3.5], &SILVERFISH_SEGMENT_0_CUBE),
    silverfish_part([0.0, 21.0, -1.5], &SILVERFISH_SEGMENT_1_CUBE),
    silverfish_part([0.0, 20.0, 1.0], &SILVERFISH_SEGMENT_2_CUBE),
    silverfish_part([0.0, 21.0, 4.0], &SILVERFISH_SEGMENT_3_CUBE),
    silverfish_part([0.0, 22.0, 7.0], &SILVERFISH_SEGMENT_4_CUBE),
    silverfish_part([0.0, 23.0, 9.5], &SILVERFISH_SEGMENT_5_CUBE),
    silverfish_part([0.0, 23.0, 11.5], &SILVERFISH_SEGMENT_6_CUBE),
    silverfish_part([0.0, 16.0, 1.0], &SILVERFISH_LAYER_0_CUBE),
    silverfish_part([0.0, 20.0, 7.0], &SILVERFISH_LAYER_1_CUBE),
    silverfish_part([0.0, 19.0, -1.5], &SILVERFISH_LAYER_2_CUBE),
];

const SILVERFISH_TEXTURED_SEGMENT_0_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-1.5, 0.0, -1.0],
    [3.0, 2.0, 2.0],
    [0.0, 0.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_1_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-2.0, 0.0, -1.0],
    [4.0, 3.0, 2.0],
    [0.0, 4.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_2_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-3.0, 0.0, -1.5],
    [6.0, 4.0, 3.0],
    [0.0, 9.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_3_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-1.5, 0.0, -1.5],
    [3.0, 3.0, 3.0],
    [0.0, 16.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_4_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-1.0, 0.0, -1.5],
    [2.0, 2.0, 3.0],
    [0.0, 22.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_5_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-1.0, 0.0, -1.0],
    [2.0, 1.0, 2.0],
    [11.0, 0.0],
)];
const SILVERFISH_TEXTURED_SEGMENT_6_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-0.5, 0.0, -1.0],
    [1.0, 1.0, 2.0],
    [13.0, 4.0],
)];
const SILVERFISH_TEXTURED_LAYER_0_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-5.0, 0.0, -1.5],
    [10.0, 8.0, 3.0],
    [20.0, 0.0],
)];
const SILVERFISH_TEXTURED_LAYER_1_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-3.0, 0.0, -1.5],
    [6.0, 4.0, 3.0],
    [20.0, 11.0],
)];
const SILVERFISH_TEXTURED_LAYER_2_CUBE: [TexturedModelCubeDesc; 1] = [silverfish_textured_cube(
    [-3.0, 0.0, -1.5],
    [6.0, 5.0, 2.0],
    [20.0, 18.0],
)];

const fn silverfish_textured_cube(
    min: [f32; 3],
    size: [f32; 3],
    tex: [f32; 2],
) -> TexturedModelCubeDesc {
    TexturedModelCubeDesc {
        min,
        size,
        uv_size: size,
        tex,
        mirror: false,
    }
}

const fn silverfish_textured_part(
    offset: [f32; 3],
    cubes: &'static [TexturedModelCubeDesc],
) -> TexturedModelPartDesc {
    TexturedModelPartDesc {
        pose: PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
        children: &[],
    }
}

pub(in crate::entity_models) const SILVERFISH_TEXTURED_PARTS: [TexturedModelPartDesc; 10] = [
    silverfish_textured_part([0.0, 22.0, -3.5], &SILVERFISH_TEXTURED_SEGMENT_0_CUBE),
    silverfish_textured_part([0.0, 21.0, -1.5], &SILVERFISH_TEXTURED_SEGMENT_1_CUBE),
    silverfish_textured_part([0.0, 20.0, 1.0], &SILVERFISH_TEXTURED_SEGMENT_2_CUBE),
    silverfish_textured_part([0.0, 21.0, 4.0], &SILVERFISH_TEXTURED_SEGMENT_3_CUBE),
    silverfish_textured_part([0.0, 22.0, 7.0], &SILVERFISH_TEXTURED_SEGMENT_4_CUBE),
    silverfish_textured_part([0.0, 23.0, 9.5], &SILVERFISH_TEXTURED_SEGMENT_5_CUBE),
    silverfish_textured_part([0.0, 23.0, 11.5], &SILVERFISH_TEXTURED_SEGMENT_6_CUBE),
    silverfish_textured_part([0.0, 16.0, 1.0], &SILVERFISH_TEXTURED_LAYER_0_CUBE),
    silverfish_textured_part([0.0, 20.0, 7.0], &SILVERFISH_TEXTURED_LAYER_1_CUBE),
    silverfish_textured_part([0.0, 19.0, -1.5], &SILVERFISH_TEXTURED_LAYER_2_CUBE),
];

/// Each overlay `layer` part copies one body segment's animation: `(source_segment,
/// copy_x)`. Vanilla sets `layer0.yRot = segment2.yRot` (x untouched), `layer1.yRot =
/// segment4.yRot` and `layer1.x = segment4.x`, `layer2.yRot = segment1.yRot` and `layer2.x =
/// segment1.x`.
pub(in crate::entity_models) const SILVERFISH_LAYER_RULES: [(usize, bool); SILVERFISH_LAYER_COUNT] =
    [(2, false), (4, true), (1, true)];

/// Vanilla `SilverfishModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.05 * (1 + dist)` and `segment.x = sin(phase) * π * 0.2
/// * dist` — the same shape as the endermite, but with the larger `0.05`/`0.2` amplitudes of
/// the silverfish's faster scuttle. Only `offset.x` and `rotation.yRot` change; the rest
/// `offset.y`/`offset.z` and the zero `xRot`/`zRot` are preserved. `ageInTicks` advances
/// every frame and the rest phase already carries nonzero `cos`/`sin` terms, so the
/// silverfish never sits at its layer pose.
pub(in crate::entity_models) fn silverfish_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.05 * (1.0 + dist);
    let x = phase.sin() * PI * 0.2 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Applies a [`SILVERFISH_LAYER_RULES`] copy to one overlay `layer` part: its `yRot` always
/// follows the source segment's animated `yRot`, and its `offset.x` follows the source's `x`
/// only when `copy_x` (layer0 leaves its `x` at the layer rest). The rest `offset.y`/`z` and
/// `xRot`/`zRot` are preserved.
pub(in crate::entity_models) fn silverfish_layer_pose(
    base: PartPose,
    source: PartPose,
    copy_x: bool,
) -> PartPose {
    PartPose {
        offset: [
            if copy_x {
                source.offset[0]
            } else {
                base.offset[0]
            },
            base.offset[1],
            base.offset[2],
        ],
        rotation: [base.rotation[0], source.rotation[1], base.rotation[2]],
    }
}
