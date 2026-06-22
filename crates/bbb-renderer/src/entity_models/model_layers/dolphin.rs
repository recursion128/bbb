use super::{ModelCubeDesc, PartPose, TexturedModelCubeDesc, DOLPHIN_GRAY};

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
