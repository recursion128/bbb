use super::{ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// The endermite fallback paints its four chitin segments a dark End purple.
pub(in crate::entity_models) const ENDERMITE_PURPLE: [f32; 4] = [0.18, 0.10, 0.24, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_ENDERMITE: &str = "minecraft:endermite#main";

/// The number of body segments in the endermite body layer.
pub(in crate::entity_models) const ENDERMITE_SEGMENT_COUNT: usize = 4;

// Vanilla 26.1 EndermiteModel.createBodyLayer: four nested chitin segments. Each segment
// `i` is a box of BODY_SIZES[i] = (sx, sy, sz) drawn from texOffs(BODY_TEXS[i]) at
// addBox(-sx/2, 0, -sz/2, sx, sy, sz), posed at offset (0, 24 - sy, placement) where
// `placement` walks back to front by half the summed depths of adjacent segments.
const ENDERMITE_SEGMENT_0_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, -1.0],
    size: [4.0, 3.0, 2.0],
    color: ENDERMITE_PURPLE,
}];
const ENDERMITE_SEGMENT_1_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, 0.0, -2.5],
    size: [6.0, 4.0, 5.0],
    color: ENDERMITE_PURPLE,
}];
const ENDERMITE_SEGMENT_2_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, 0.0, -0.5],
    size: [3.0, 3.0, 1.0],
    color: ENDERMITE_PURPLE,
}];
const ENDERMITE_SEGMENT_3_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, -0.5],
    size: [1.0, 2.0, 1.0],
    color: ENDERMITE_PURPLE,
}];

pub(in crate::entity_models) const ENDERMITE_PARTS: [ModelPartDesc; ENDERMITE_SEGMENT_COUNT] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 21.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_SEGMENT_0_CUBE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_SEGMENT_1_CUBE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_SEGMENT_2_CUBE,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_SEGMENT_3_CUBE,
        children: &[],
    },
];

const ENDERMITE_TEXTURED_SEGMENT_0_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-2.0, 0.0, -1.0],
    size: [4.0, 3.0, 2.0],
    uv_size: [4.0, 3.0, 2.0],
    tex: [0.0, 0.0],
    mirror: false,
}];
const ENDERMITE_TEXTURED_SEGMENT_1_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-3.0, 0.0, -2.5],
    size: [6.0, 4.0, 5.0],
    uv_size: [6.0, 4.0, 5.0],
    tex: [0.0, 5.0],
    mirror: false,
}];
const ENDERMITE_TEXTURED_SEGMENT_2_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-1.5, 0.0, -0.5],
    size: [3.0, 3.0, 1.0],
    uv_size: [3.0, 3.0, 1.0],
    tex: [0.0, 14.0],
    mirror: false,
}];
const ENDERMITE_TEXTURED_SEGMENT_3_CUBE: [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [-0.5, 0.0, -0.5],
    size: [1.0, 2.0, 1.0],
    uv_size: [1.0, 2.0, 1.0],
    tex: [0.0, 18.0],
    mirror: false,
}];

pub(in crate::entity_models) const ENDERMITE_TEXTURED_PARTS: [TexturedModelPartDesc;
    ENDERMITE_SEGMENT_COUNT] = [
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 21.0, -3.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_TEXTURED_SEGMENT_0_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_TEXTURED_SEGMENT_1_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_TEXTURED_SEGMENT_2_CUBE,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ENDERMITE_TEXTURED_SEGMENT_3_CUBE,
        children: &[],
    },
];

/// Vanilla `EndermiteModel.setupAnim` segment wiggle for segment `index`, driven purely by
/// `ageInTicks` (`super.setupAnim` first resets every part to its rest pose). With `phase =
/// ageInTicks * 0.9 + index * 0.15 * π` and `dist = |index - 2|`, vanilla *sets*
/// `segment.yRot = cos(phase) * π * 0.01 * (1 + dist)` and `segment.x = sin(phase) * π *
/// 0.1 * dist`. The middle segment (`index == 2`, `dist == 0`) never shifts in `x` and only
/// wags `yRot` by the smallest amount; the head and tail (`dist == 2`) swing the most. Only
/// `offset.x` and `rotation.yRot` change; the rest `offset.y`/`offset.z` and the zero
/// `xRot`/`zRot` are preserved. Because `ageInTicks` advances every frame and the rest phase
/// already carries nonzero `cos`/`sin` terms, the endermite never sits at its layer pose.
pub(in crate::entity_models) fn endermite_segment_pose(
    base: PartPose,
    index: usize,
    age_in_ticks: f32,
) -> PartPose {
    use std::f32::consts::PI;
    let phase = age_in_ticks * 0.9 + index as f32 * 0.15 * PI;
    let dist = (index as i32 - 2).abs() as f32;
    let y_rot = phase.cos() * PI * 0.01 * (1.0 + dist);
    let x = phase.sin() * PI * 0.1 * dist;
    PartPose {
        offset: [x, base.offset[1], base.offset[2]],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Mutable endermite model, mirroring vanilla `EndermiteModel`. The unified tree is zipped from the
/// baked colored ([`ENDERMITE_PARTS`]) and textured ([`ENDERMITE_TEXTURED_PARTS`]) trees; `setup_anim`
/// wiggles all four chitin segments from `ageInTicks` ([`endermite_segment_pose`]). There is no head
/// look or walk swing, and no `MeshTransformer` scaling (unit model root).
pub(in crate::entity_models) struct EndermiteModel {
    root: ModelPart,
}

impl EndermiteModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&ENDERMITE_PARTS, &ENDERMITE_TEXTURED_PARTS),
        }
    }
}

impl EntityModel for EndermiteModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let age_in_ticks = instance.render_state.age_in_ticks;
        for index in 0..ENDERMITE_SEGMENT_COUNT {
            let segment = self.root.child_at_mut(index);
            segment.pose = endermite_segment_pose(segment.pose, index, age_in_ticks);
        }
    }
}
