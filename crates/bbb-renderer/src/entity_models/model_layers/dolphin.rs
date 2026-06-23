use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    DOLPHIN_GRAY,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

use std::f32::consts::PI;

// Vanilla 26.1 `DolphinModel.createBodyLayer` (atlas 64×64). The `body` is the root child and
// parents the back/left/right fins, the tail (with its tail fin), and the head (with its nose).
// The baby uses the `MeshTransformer.scaling(0.5)` of this same geometry.
pub(in crate::entity_models) const DOLPHIN_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -7.0, 0.0],
    size: [8.0, 7.0, 13.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_BACK_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, 0.0, 8.0],
    size: [1.0, 4.0, 5.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_SIDE_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-0.5, -4.0, 0.0],
    size: [1.0, 4.0, 7.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, -2.5, 0.0],
    size: [4.0, 5.0, 11.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_TAIL_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-5.0, -0.5, 0.0],
    size: [10.0, 1.0, 6.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-4.0, -3.0, -3.0],
    size: [8.0, 7.0, 6.0],
    color: DOLPHIN_GRAY,
}];

pub(in crate::entity_models) const DOLPHIN_NOSE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 2.0, -7.0],
    size: [2.0, 2.0, 4.0],
    color: DOLPHIN_GRAY,
}];

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

// The same geometry with the vanilla `DolphinModel.createBodyLayer` texOffs UV coordinates (atlas
// 64×64); no `CubeDeformation`, so each `uv_size` matches its box `size`. The left fin is mirrored.
pub(in crate::entity_models) const DOLPHIN_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -7.0, 0.0],
        size: [8.0, 7.0, 13.0],
        uv_size: [8.0, 7.0, 13.0],
        tex: [22.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_BACK_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, 0.0, 8.0],
        size: [1.0, 4.0, 5.0],
        uv_size: [1.0, 4.0, 5.0],
        tex: [51.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_LEFT_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, -4.0, 0.0],
        size: [1.0, 4.0, 7.0],
        uv_size: [1.0, 4.0, 7.0],
        tex: [48.0, 20.0],
        mirror: true,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_RIGHT_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-0.5, -4.0, 0.0],
        size: [1.0, 4.0, 7.0],
        uv_size: [1.0, 4.0, 7.0],
        tex: [48.0, 20.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, -2.5, 0.0],
        size: [4.0, 5.0, 11.0],
        uv_size: [4.0, 5.0, 11.0],
        tex: [0.0, 19.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_TAIL_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-5.0, -0.5, 0.0],
        size: [10.0, 1.0, 6.0],
        uv_size: [10.0, 1.0, 6.0],
        tex: [19.0, 20.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-4.0, -3.0, -3.0],
        size: [8.0, 7.0, 6.0],
        uv_size: [8.0, 7.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const DOLPHIN_TEXTURED_NOSE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 2.0, -7.0],
        size: [2.0, 2.0, 4.0],
        uv_size: [2.0, 2.0, 4.0],
        tex: [0.0, 13.0],
        mirror: false,
    }];

// Colored dolphin tree: `body` (root child) → back/left/right fins, the tail chain (tail → tail fin),
// and the head chain (head → nose), in the emit order (preserved for byte-identical meshes). Mirrors
// vanilla `DolphinModel.createBodyLayer`. Zipped with the textured tree by `DolphinModel::new`; the
// body steer and the tail wave are applied in `setup_anim`.
const DOLPHIN_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: DOLPHIN_TAIL_FIN_POSE,
    cubes: &DOLPHIN_TAIL_FIN,
    children: &[],
}];
const DOLPHIN_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: DOLPHIN_NOSE_POSE,
    cubes: &DOLPHIN_NOSE,
    children: &[],
}];
const DOLPHIN_BODY_CHILDREN: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: DOLPHIN_BACK_FIN_POSE,
        cubes: &DOLPHIN_BACK_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: DOLPHIN_LEFT_FIN_POSE,
        cubes: &DOLPHIN_SIDE_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: DOLPHIN_RIGHT_FIN_POSE,
        cubes: &DOLPHIN_SIDE_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: DOLPHIN_TAIL_POSE,
        cubes: &DOLPHIN_TAIL,
        children: &DOLPHIN_TAIL_CHILDREN,
    },
    ModelPartDesc {
        pose: DOLPHIN_HEAD_POSE,
        cubes: &DOLPHIN_HEAD,
        children: &DOLPHIN_HEAD_CHILDREN,
    },
];
pub(in crate::entity_models) const DOLPHIN_PARTS: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: DOLPHIN_BODY_POSE,
    cubes: &DOLPHIN_BODY,
    children: &DOLPHIN_BODY_CHILDREN,
}];

// Textured counterpart of `DOLPHIN_PARTS` (same hierarchy and bind poses, UV cubes — the left fin's
// UV is mirrored, so the two side fins use distinct textured cubes).
const DOLPHIN_TEXTURED_TAIL_CHILDREN: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: DOLPHIN_TAIL_FIN_POSE,
    cubes: &DOLPHIN_TEXTURED_TAIL_FIN,
    children: &[],
}];
const DOLPHIN_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] = [TexturedModelPartDesc {
    pose: DOLPHIN_NOSE_POSE,
    cubes: &DOLPHIN_TEXTURED_NOSE,
    children: &[],
}];
const DOLPHIN_TEXTURED_BODY_CHILDREN: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: DOLPHIN_BACK_FIN_POSE,
        cubes: &DOLPHIN_TEXTURED_BACK_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: DOLPHIN_LEFT_FIN_POSE,
        cubes: &DOLPHIN_TEXTURED_LEFT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: DOLPHIN_RIGHT_FIN_POSE,
        cubes: &DOLPHIN_TEXTURED_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: DOLPHIN_TAIL_POSE,
        cubes: &DOLPHIN_TEXTURED_TAIL,
        children: &DOLPHIN_TEXTURED_TAIL_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: DOLPHIN_HEAD_POSE,
        cubes: &DOLPHIN_TEXTURED_HEAD,
        children: &DOLPHIN_TEXTURED_HEAD_CHILDREN,
    },
];
pub(in crate::entity_models) const DOLPHIN_TEXTURED_PARTS: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: DOLPHIN_BODY_POSE,
        cubes: &DOLPHIN_TEXTURED_BODY,
        children: &DOLPHIN_TEXTURED_BODY_CHILDREN,
    }];

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

    let body = root.child_at_mut(0);
    let body_x_rot = head_pitch + if moving { -0.05 - 0.05 * wave } else { 0.0 };
    body.pose.rotation = [body_x_rot, head_yaw, 0.0];

    let tail = body.child_at_mut(3);
    tail.pose.rotation = [
        if moving {
            -0.1 * wave
        } else {
            DOLPHIN_TAIL_BIND_X_ROT
        },
        0.0,
        0.0,
    ];
    tail.child_at_mut(0).pose.rotation = [if moving { -0.2 * wave } else { 0.0 }, 0.0, 0.0];
}

/// Mutable dolphin model, mirroring vanilla `DolphinModel`. The unified tree is zipped from the `body`
/// → (fins, tail chain, head chain) hierarchy ([`DOLPHIN_PARTS`] / [`DOLPHIN_TEXTURED_PARTS`]);
/// `setup_anim` runs [`apply_dolphin_anim`]. The same posed tree drives the colored fallback and the
/// cutout textured layer; the body bob, baby `0.5` scale, and adult/baby texture live outside the model.
pub(in crate::entity_models) struct DolphinModel {
    root: ModelPart,
}

impl DolphinModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_descs(&DOLPHIN_PARTS, &DOLPHIN_TEXTURED_PARTS),
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
