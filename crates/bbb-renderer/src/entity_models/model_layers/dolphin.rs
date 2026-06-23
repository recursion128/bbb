use super::{PartPose, DOLPHIN_GRAY, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use std::f32::consts::PI;

// Vanilla 26.1 `DolphinModel.createBodyLayer` (atlas 64×64). The `body` is the root child and
// parents the back/left/right fins, the tail (with its tail fin), and the head (with its nose).
// The baby uses the `MeshTransformer.scaling(0.5)` of this same geometry. Each cube carries both
// render paths' data: the colored debug tint (`DOLPHIN_GRAY`) and the textured `uv_size` /
// `texOffs` / `mirror` (`CubeDeformation.NONE`, so `uv_size == size`). The left side fin's UV is
// mirrored, so the two side fins use distinct cubes.
pub(in crate::entity_models) const DOLPHIN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -7.0, 0.0],
    [8.0, 7.0, 13.0],
    DOLPHIN_GRAY,
    [8.0, 7.0, 13.0],
    [22.0, 0.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_BACK_FIN: [ModelCube; 1] = [ModelCube::new(
    [-0.5, 0.0, 8.0],
    [1.0, 4.0, 5.0],
    DOLPHIN_GRAY,
    [1.0, 4.0, 5.0],
    [51.0, 0.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_LEFT_FIN: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -4.0, 0.0],
    [1.0, 4.0, 7.0],
    DOLPHIN_GRAY,
    [1.0, 4.0, 7.0],
    [48.0, 20.0],
    true,
)];

pub(in crate::entity_models) const DOLPHIN_RIGHT_FIN: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -4.0, 0.0],
    [1.0, 4.0, 7.0],
    DOLPHIN_GRAY,
    [1.0, 4.0, 7.0],
    [48.0, 20.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_TAIL: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.5, 0.0],
    [4.0, 5.0, 11.0],
    DOLPHIN_GRAY,
    [4.0, 5.0, 11.0],
    [0.0, 19.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_TAIL_FIN: [ModelCube; 1] = [ModelCube::new(
    [-5.0, -0.5, 0.0],
    [10.0, 1.0, 6.0],
    DOLPHIN_GRAY,
    [10.0, 1.0, 6.0],
    [19.0, 20.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -3.0, -3.0],
    [8.0, 7.0, 6.0],
    DOLPHIN_GRAY,
    [8.0, 7.0, 6.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const DOLPHIN_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 2.0, -7.0],
    [2.0, 2.0, 4.0],
    DOLPHIN_GRAY,
    [2.0, 2.0, 4.0],
    [0.0, 13.0],
    false,
)];

// Vanilla `body` bind pose (offset only); its `xRot`/`yRot` are set per frame by `setupAnim`.
pub(in crate::entity_models) const DOLPHIN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const DOLPHIN_BACK_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [PI / 3.0, 0.0, 0.0],
};
pub(in crate::entity_models) const DOLPHIN_LEFT_FIN_POSE: PartPose = PartPose {
    offset: [2.0, -2.0, 4.0],
    rotation: [PI / 3.0, 0.0, 2.0 * PI / 3.0],
};
pub(in crate::entity_models) const DOLPHIN_RIGHT_FIN_POSE: PartPose = PartPose {
    offset: [-2.0, -2.0, 4.0],
    rotation: [PI / 3.0, 0.0, -2.0 * PI / 3.0],
};
// The tail's bind pitch; `setupAnim` overrides it with the wave while moving.
pub(in crate::entity_models) const DOLPHIN_TAIL_BIND_X_ROT: f32 = -0.10471976;
pub(in crate::entity_models) const DOLPHIN_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, -2.5, 11.0],
    rotation: [DOLPHIN_TAIL_BIND_X_ROT, 0.0, 0.0],
};
pub(in crate::entity_models) const DOLPHIN_TAIL_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 9.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const DOLPHIN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, -4.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const DOLPHIN_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `DolphinModel.setupAnim` swim wave term: `cos(ageInTicks · 0.3)`.
pub(in crate::entity_models) fn dolphin_wave(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 0.3).cos()
}

/// Applies the vanilla `DolphinModel.setupAnim` to the unified tree: the `body` steers by the look
/// pitch/yaw and, while moving, adds the swim body tilt and the tail / tail-fin wave (`cos(ageInTicks
/// · 0.3)`). At rest the tail holds its bind pitch and the tail fin is level, so a still dolphin is
/// byte-identical to its bind tree. The body bob and the baby `0.5` scale live in the root transform.
fn apply_dolphin_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let moving = instance.render_state.is_moving;
    let age = instance.render_state.age_in_ticks;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();
    let wave = dolphin_wave(age);

    let body = root.child_mut("body");
    let body_x_rot = head_pitch + if moving { -0.05 - 0.05 * wave } else { 0.0 };
    body.pose.rotation = [body_x_rot, head_yaw, 0.0];

    let tail = body.child_mut("tail");
    tail.pose.rotation = [
        if moving {
            -0.1 * wave
        } else {
            DOLPHIN_TAIL_BIND_X_ROT
        },
        0.0,
        0.0,
    ];
    tail.child_mut("tail_fin").pose.rotation = [if moving { -0.2 * wave } else { 0.0 }, 0.0, 0.0];
}

/// Mutable dolphin model, mirroring vanilla `DolphinModel`. The unified tree is built once with named
/// children: a synthetic root → `body`, with `body` parenting the back fin, left fin, right fin, the
/// tail chain (`tail` → `tail_fin`), and the head chain (`head` → `nose`), in the emit order (preserved
/// for byte-identical meshes). Each cube carries both the colored tint and the textured UV, so one tree
/// drives both render paths; `setup_anim` runs [`apply_dolphin_anim`] (the body steer and the tail
/// wave). The same posed tree drives the colored fallback and the cutout textured layer; the body bob,
/// baby `0.5` scale, and adult/baby texture live outside the model.
pub(in crate::entity_models) struct DolphinModel {
    root: ModelPart,
}

impl DolphinModel {
    pub(in crate::entity_models) fn new() -> Self {
        let body = ModelPart::new(
            DOLPHIN_BODY_POSE,
            DOLPHIN_BODY.to_vec(),
            vec![
                (
                    "back_fin",
                    ModelPart::leaf(DOLPHIN_BACK_FIN_POSE, DOLPHIN_BACK_FIN.to_vec()),
                ),
                (
                    "left_fin",
                    ModelPart::leaf(DOLPHIN_LEFT_FIN_POSE, DOLPHIN_LEFT_FIN.to_vec()),
                ),
                (
                    "right_fin",
                    ModelPart::leaf(DOLPHIN_RIGHT_FIN_POSE, DOLPHIN_RIGHT_FIN.to_vec()),
                ),
                (
                    "tail",
                    ModelPart::new(
                        DOLPHIN_TAIL_POSE,
                        DOLPHIN_TAIL.to_vec(),
                        vec![(
                            "tail_fin",
                            ModelPart::leaf(DOLPHIN_TAIL_FIN_POSE, DOLPHIN_TAIL_FIN.to_vec()),
                        )],
                    ),
                ),
                (
                    "head",
                    ModelPart::new(
                        DOLPHIN_HEAD_POSE,
                        DOLPHIN_HEAD.to_vec(),
                        vec![(
                            "nose",
                            ModelPart::leaf(DOLPHIN_NOSE_POSE, DOLPHIN_NOSE.to_vec()),
                        )],
                    ),
                ),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("body", body)]),
        }
    }
}

impl EntityModel for DolphinModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_dolphin_anim(&mut self.root, instance);
    }
}
