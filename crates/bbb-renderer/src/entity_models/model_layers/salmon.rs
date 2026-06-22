use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    SALMON_RED,
};
use crate::entity_models::catalog::SalmonModelSize;

// Vanilla 26.1 `SalmonModel.createBodyLayer` (atlas 32×32). The body is split into a
// front and back segment (the back sways), each carrying a flat top fin; the back also
// carries the flat tail fin. The side fins are zero-thickness planes.
pub(in crate::entity_models) const SALMON_BODY_FRONT: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -2.5, 0.0],
    size: [3.0, 5.0, 8.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_BODY_BACK: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.5, -2.5, 0.0],
    size: [3.0, 5.0, 8.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_HEAD: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -2.0, -3.0],
    size: [2.0, 4.0, 3.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_BACK_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -2.5, 0.0],
    size: [0.0, 5.0, 6.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_TOP_FRONT_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [0.0, 2.0, 3.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_TOP_BACK_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [0.0, 2.0, 4.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_RIGHT_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 0.0, 0.0],
    size: [2.0, 0.0, 2.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_LEFT_FIN: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [2.0, 0.0, 2.0],
    color: SALMON_RED,
}];

pub(in crate::entity_models) const SALMON_BODY_FRONT_CHILDREN: [ModelPartDesc; 1] =
    [ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, 5.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_TOP_FRONT_FIN,
        children: &[],
    }];

pub(in crate::entity_models) const SALMON_BODY_BACK_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 0.0, 8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_BACK_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, -4.5, -1.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_TOP_BACK_FIN,
        children: &[],
    },
];

/// Vanilla `SalmonModel.createBodyLayer` root part order: body front (top fin child),
/// body back (tail + top fin children, swayed by `setupAnim`), head, right fin
/// (`zRot = -π/4`), left fin (`zRot = π/4`). The body back is index
/// [`SALMON_BODY_BACK_PART_INDEX`].
pub(in crate::entity_models) const SALMON_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, -7.2],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_BODY_FRONT,
        children: &SALMON_BODY_FRONT_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, 0.8],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_BODY_BACK,
        children: &SALMON_BODY_BACK_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.0, -7.2],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &SALMON_HEAD,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 21.5, -7.2],
            rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_4],
        },
        cubes: &SALMON_RIGHT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 21.5, -7.2],
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_4],
        },
        cubes: &SALMON_LEFT_FIN,
        children: &[],
    },
];

pub(in crate::entity_models) const SALMON_BODY_BACK_PART_INDEX: usize = 1;

/// Vanilla `SalmonModel`/`SalmonRenderer` swim multipliers: a salmon in water uses
/// `(amplitude 1.0, angle 1.0)`; a beached salmon thrashes harder and faster
/// `(1.3, 1.7)`. Shared by the body-back sway and the renderer body wiggle.
pub(in crate::entity_models) fn salmon_swim_multipliers(in_water: bool) -> (f32, f32) {
    if in_water {
        (1.0, 1.0)
    } else {
        (1.3, 1.7)
    }
}

/// Vanilla `SalmonModel.setupAnim`: `bodyBack.yRot = -amplitude * 0.25 * sin(angle *
/// 0.6 * ageInTicks)`. The rest pose has `yRot = 0`, so this is set absolutely.
pub(in crate::entity_models) fn salmon_body_back_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    let (amplitude, angle) = salmon_swim_multipliers(in_water);
    -amplitude * 0.25 * (angle * 0.6 * age_in_ticks).sin()
}

// Vanilla 26.1 `ModelLayers.SALMON` / `SALMON_SMALL` / `SALMON_LARGE` (`SalmonRenderer`).
// The geometry is shared; the size layers only differ by a `MeshTransformer.scaling`
// factor, which the renderer folds into the root transform.
pub(in crate::entity_models) const MODEL_LAYER_SALMON: &str = "minecraft:salmon#main";
pub(in crate::entity_models) const MODEL_LAYER_SALMON_SMALL: &str = "minecraft:salmon_small#main";
pub(in crate::entity_models) const MODEL_LAYER_SALMON_LARGE: &str = "minecraft:salmon_large#main";

/// Vanilla `SalmonRenderer` selects the small / medium / large `SalmonModel` layer by
/// `Salmon.Variant`; the medium layer is the unscaled `ModelLayers.SALMON`.
pub(in crate::entity_models) fn salmon_model_layer(size: SalmonModelSize) -> &'static str {
    match size {
        SalmonModelSize::Small => MODEL_LAYER_SALMON_SMALL,
        SalmonModelSize::Medium => MODEL_LAYER_SALMON,
        SalmonModelSize::Large => MODEL_LAYER_SALMON_LARGE,
    }
}

// Textured counterparts of the salmon cubes (atlas 32×32). `CubeDeformation.NONE`, so each
// `uv_size` equals the geometry size, and no cube mirrors. The UV origins mirror vanilla
// `SalmonModel.createBodyLayer`'s `texOffs` (the right fin keeps its negative
// `texOffs(-4, 0)` U origin).
pub(in crate::entity_models) const SALMON_TEXTURED_BODY_FRONT: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -2.5, 0.0],
        size: [3.0, 5.0, 8.0],
        uv_size: [3.0, 5.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_BODY_BACK: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.5, -2.5, 0.0],
        size: [3.0, 5.0, 8.0],
        uv_size: [3.0, 5.0, 8.0],
        tex: [0.0, 13.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_HEAD: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -2.0, -3.0],
        size: [2.0, 4.0, 3.0],
        uv_size: [2.0, 4.0, 3.0],
        tex: [22.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_BACK_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -2.5, 0.0],
        size: [0.0, 5.0, 6.0],
        uv_size: [0.0, 5.0, 6.0],
        tex: [20.0, 10.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_TOP_FRONT_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [0.0, 2.0, 3.0],
        uv_size: [0.0, 2.0, 3.0],
        tex: [2.0, 1.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_TOP_BACK_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [0.0, 2.0, 4.0],
        uv_size: [0.0, 2.0, 4.0],
        tex: [0.0, 2.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_RIGHT_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-2.0, 0.0, 0.0],
        size: [2.0, 0.0, 2.0],
        uv_size: [2.0, 0.0, 2.0],
        tex: [-4.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_LEFT_FIN: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [2.0, 0.0, 2.0],
        uv_size: [2.0, 0.0, 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_BODY_FRONT_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: SALMON_BODY_FRONT_CHILDREN[0].pose,
        cubes: &SALMON_TEXTURED_TOP_FRONT_FIN,
        children: &[],
    }];

pub(in crate::entity_models) const SALMON_TEXTURED_BODY_BACK_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: SALMON_BODY_BACK_CHILDREN[0].pose,
        cubes: &SALMON_TEXTURED_BACK_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SALMON_BODY_BACK_CHILDREN[1].pose,
        cubes: &SALMON_TEXTURED_TOP_BACK_FIN,
        children: &[],
    },
];

/// Textured salmon parts mirroring [`SALMON_PARTS`]: body front (top fin child), body
/// back (tail fin + rear top fin children, swayed by `setupAnim`), head, right fin, left
/// fin. The body back is index [`SALMON_BODY_BACK_PART_INDEX`].
pub(in crate::entity_models) const SALMON_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: SALMON_PARTS[0].pose,
        cubes: &SALMON_TEXTURED_BODY_FRONT,
        children: &SALMON_TEXTURED_BODY_FRONT_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: SALMON_PARTS[1].pose,
        cubes: &SALMON_TEXTURED_BODY_BACK,
        children: &SALMON_TEXTURED_BODY_BACK_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: SALMON_PARTS[2].pose,
        cubes: &SALMON_TEXTURED_HEAD,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SALMON_PARTS[3].pose,
        cubes: &SALMON_TEXTURED_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: SALMON_PARTS[4].pose,
        cubes: &SALMON_TEXTURED_LEFT_FIN,
        children: &[],
    },
];
