use super::{
    inflate_textured_cube, ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc,
    TexturedModelPartDesc, TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF, TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF, TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF, TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF, TROPICAL_FISH_ORANGE,
    TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF, TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF, TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
};
use crate::entity_models::catalog::{TropicalFishModelShape, TropicalFishPattern};
use crate::entity_models::EntityModelTextureRef;

// Vanilla 26.1 `TropicalFishSmallModel.createBodyLayer` (kob-style body, atlas 32×32,
// `CubeDeformation.NONE`). The tail and top fin are zero-thickness planes flat in X; the
// two side fins are zero-thickness planes flat in Z, splayed ±π/4 about Y.
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.5, -3.0],
    size: [2.0, 3.0, 6.0],
    color: TROPICAL_FISH_ORANGE,
}];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -1.5, 0.0],
    size: [0.0, 3.0, 6.0],
    color: TROPICAL_FISH_ORANGE,
}];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_RIGHT_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, -1.0, 0.0],
        size: [2.0, 2.0, 0.0],
        color: TROPICAL_FISH_ORANGE,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_LEFT_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, -1.0, 0.0],
        size: [2.0, 2.0, 0.0],
        color: TROPICAL_FISH_ORANGE,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TOP_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, -3.0, 0.0],
        size: [0.0, 3.0, 6.0],
        color: TROPICAL_FISH_ORANGE,
    }];

/// Vanilla `TropicalFishSmallModel.createBodyLayer` root part order: body, tail (swayed by
/// `setupAnim`), right fin (`yRot = π/4`), left fin (`yRot = -π/4`), top fin. The tail is
/// index [`TROPICAL_FISH_TAIL_PART_INDEX`].
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PARTS: [ModelPartDesc; 5] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_SMALL_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_SMALL_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 22.5, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &TROPICAL_FISH_SMALL_RIGHT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 22.5, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &TROPICAL_FISH_SMALL_LEFT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 20.5, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_SMALL_TOP_FIN,
        children: &[],
    },
];

// Vanilla 26.1 `TropicalFishLargeModel.createBodyLayer` (flopper-style body, atlas 32×32,
// `CubeDeformation.NONE`). Adds a bottom fin to the small layout; the tail is a 5-deep
// plane and the body is twice as tall.
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -3.0, -3.0],
    size: [2.0, 6.0, 6.0],
    color: TROPICAL_FISH_ORANGE,
}];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, -3.0, 0.0],
    size: [0.0, 6.0, 5.0],
    color: TROPICAL_FISH_ORANGE,
}];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_RIGHT_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [-2.0, 0.0, 0.0],
        size: [2.0, 2.0, 0.0],
        color: TROPICAL_FISH_ORANGE,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_LEFT_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [2.0, 2.0, 0.0],
        color: TROPICAL_FISH_ORANGE,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TOP_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, -4.0, 0.0],
        size: [0.0, 4.0, 6.0],
        color: TROPICAL_FISH_ORANGE,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BOTTOM_FIN: [ModelCubeDesc; 1] =
    [ModelCubeDesc {
        min: [0.0, 0.0, 0.0],
        size: [0.0, 4.0, 6.0],
        color: TROPICAL_FISH_ORANGE,
    }];

/// Vanilla `TropicalFishLargeModel.createBodyLayer` root part order: body, tail (swayed by
/// `setupAnim`), right fin (`yRot = π/4`), left fin (`yRot = -π/4`), top fin, bottom fin.
/// The tail is index [`TROPICAL_FISH_TAIL_PART_INDEX`].
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PARTS: [ModelPartDesc; 6] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_TAIL,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 20.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_RIGHT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.0, 20.0, 0.0],
            rotation: [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_LEFT_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 16.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_TOP_FIN,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 22.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &TROPICAL_FISH_LARGE_BOTTOM_FIN,
        children: &[],
    },
];

/// Both tropical fish layers list the tail as root part index `1`; `setupAnim` sets its
/// `yRot`.
pub(in crate::entity_models) const TROPICAL_FISH_TAIL_PART_INDEX: usize = 1;

/// Vanilla `TropicalFishSmallModel`/`TropicalFishLargeModel.setupAnim`: `tail.yRot =
/// -amplitude * 0.45 * sin(0.6 * ageInTicks)`, with `amplitude = isInWater ? 1.0 : 1.5`.
/// This is identical to `CodModel.setupAnim`'s tail sway, so both share
/// [`super::cod_tail_fin_yrot`]; this alias documents the shared formula at the tropical
/// fish call site.
pub(in crate::entity_models) fn tropical_fish_tail_yrot(age_in_ticks: f32, in_water: bool) -> f32 {
    super::cod_tail_fin_yrot(age_in_ticks, in_water)
}

// Vanilla 26.1 `ModelLayers.TROPICAL_FISH_SMALL` / `TROPICAL_FISH_LARGE`
// (`TropicalFishRenderer`). The pattern overlay layers
// (`TROPICAL_FISH_{SMALL,LARGE}_PATTERN`) inflate by `FISH_PATTERN_DEFORMATION` and are
// deferred.
pub(in crate::entity_models) const MODEL_LAYER_TROPICAL_FISH_SMALL: &str =
    "minecraft:tropical_fish_small#main";
pub(in crate::entity_models) const MODEL_LAYER_TROPICAL_FISH_LARGE: &str =
    "minecraft:tropical_fish_large#main";

/// Vanilla `TropicalFishRenderer` selects the small (kob) or large (flopper) body layer by
/// `TropicalFish.Pattern.base()`.
pub(in crate::entity_models) fn tropical_fish_model_layer(
    shape: TropicalFishModelShape,
) -> &'static str {
    match shape {
        TropicalFishModelShape::Small => MODEL_LAYER_TROPICAL_FISH_SMALL,
        TropicalFishModelShape::Large => MODEL_LAYER_TROPICAL_FISH_LARGE,
    }
}

// Textured counterparts of the small (kob) cubes (atlas 32×32, `CubeDeformation.NONE`, so
// each `uv_size` equals the geometry size). The tail and top fin keep their negative
// `texOffs` V origins exactly as vanilla bakes them.
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.5, -3.0],
        size: [2.0, 3.0, 6.0],
        uv_size: [2.0, 3.0, 6.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -1.5, 0.0],
        size: [0.0, 3.0, 6.0],
        uv_size: [0.0, 3.0, 6.0],
        tex: [22.0, -6.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_RIGHT_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.0, -1.0, 0.0],
    size: [2.0, 2.0, 0.0],
    uv_size: [2.0, 2.0, 0.0],
    tex: [2.0, 16.0],
    mirror: false,
}];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_LEFT_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [0.0, -1.0, 0.0],
    size: [2.0, 2.0, 0.0],
    uv_size: [2.0, 2.0, 0.0],
    tex: [2.0, 12.0],
    mirror: false,
}];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_TOP_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [0.0, -3.0, 0.0],
    size: [0.0, 3.0, 6.0],
    uv_size: [0.0, 3.0, 6.0],
    tex: [10.0, -5.0],
    mirror: false,
}];

/// Textured small (kob) parts mirroring [`TROPICAL_FISH_SMALL_PARTS`].
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TEXTURED_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[0].pose,
        cubes: &TROPICAL_FISH_SMALL_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[1].pose,
        cubes: &TROPICAL_FISH_SMALL_TEXTURED_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[2].pose,
        cubes: &TROPICAL_FISH_SMALL_TEXTURED_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[3].pose,
        cubes: &TROPICAL_FISH_SMALL_TEXTURED_LEFT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[4].pose,
        cubes: &TROPICAL_FISH_SMALL_TEXTURED_TOP_FIN,
        children: &[],
    },
];

// Textured counterparts of the large (flopper) cubes (atlas 32×32, `CubeDeformation.NONE`).
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -3.0, -3.0],
        size: [2.0, 6.0, 6.0],
        uv_size: [2.0, 6.0, 6.0],
        tex: [0.0, 20.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, -3.0, 0.0],
        size: [0.0, 6.0, 5.0],
        uv_size: [0.0, 6.0, 5.0],
        tex: [21.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_RIGHT_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [-2.0, 0.0, 0.0],
    size: [2.0, 2.0, 0.0],
    uv_size: [2.0, 2.0, 0.0],
    tex: [2.0, 16.0],
    mirror: false,
}];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_LEFT_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [2.0, 2.0, 0.0],
    uv_size: [2.0, 2.0, 0.0],
    tex: [2.0, 12.0],
    mirror: false,
}];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_TOP_FIN: [TexturedModelCubeDesc;
    1] = [TexturedModelCubeDesc {
    min: [0.0, -4.0, 0.0],
    size: [0.0, 4.0, 6.0],
    uv_size: [0.0, 4.0, 6.0],
    tex: [20.0, 11.0],
    mirror: false,
}];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_BOTTOM_FIN:
    [TexturedModelCubeDesc; 1] = [TexturedModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [0.0, 4.0, 6.0],
    uv_size: [0.0, 4.0, 6.0],
    tex: [20.0, 21.0],
    mirror: false,
}];

/// Textured large (flopper) parts mirroring [`TROPICAL_FISH_LARGE_PARTS`].
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TEXTURED_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[0].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[1].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[2].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[3].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_LEFT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[4].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_TOP_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[5].pose,
        cubes: &TROPICAL_FISH_LARGE_TEXTURED_BOTTOM_FIN,
        children: &[],
    },
];

/// The textured parts for a tropical fish body shape.
pub(in crate::entity_models) fn tropical_fish_textured_parts(
    shape: TropicalFishModelShape,
) -> &'static [TexturedModelPartDesc] {
    match shape {
        TropicalFishModelShape::Small => &TROPICAL_FISH_SMALL_TEXTURED_PARTS,
        TropicalFishModelShape::Large => &TROPICAL_FISH_LARGE_TEXTURED_PARTS,
    }
}

// Vanilla `LayerDefinitions.FISH_PATTERN_DEFORMATION = new CubeDeformation(0.008F)`: the
// `TropicalFishPatternLayer` bakes the same body geometry one notch larger so the overlay sits
// just outside the base body without z-fighting.
pub(in crate::entity_models) const FISH_PATTERN_DEFORMATION: f32 = 0.008;

// Vanilla `ModelLayers.TROPICAL_FISH_{SMALL,LARGE}_PATTERN` (`register("tropical_fish_small",
// "pattern")`): the pattern overlay reuses the body mesh baked with `FISH_PATTERN_DEFORMATION`.
pub(in crate::entity_models) const MODEL_LAYER_TROPICAL_FISH_SMALL_PATTERN: &str =
    "minecraft:tropical_fish_small#pattern";
pub(in crate::entity_models) const MODEL_LAYER_TROPICAL_FISH_LARGE_PATTERN: &str =
    "minecraft:tropical_fish_large#pattern";

/// The pattern overlay model-layer key for a body shape.
pub(in crate::entity_models) fn tropical_fish_pattern_model_layer(
    shape: TropicalFishModelShape,
) -> &'static str {
    match shape {
        TropicalFishModelShape::Small => MODEL_LAYER_TROPICAL_FISH_SMALL_PATTERN,
        TropicalFishModelShape::Large => MODEL_LAYER_TROPICAL_FISH_LARGE_PATTERN,
    }
}

// Pattern overlay cubes: the base body cubes inflated by `FISH_PATTERN_DEFORMATION`, keeping
// the base box as `uv_size` (`inflate_textured_cube` reproduces vanilla `CubeDeformation`).
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_BODY: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_SMALL_TEXTURED_BODY[0],
        FISH_PATTERN_DEFORMATION,
    )];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_TAIL: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_SMALL_TEXTURED_TAIL[0],
        FISH_PATTERN_DEFORMATION,
    )];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_RIGHT_FIN: [TexturedModelCubeDesc;
    1] = [inflate_textured_cube(
    TROPICAL_FISH_SMALL_TEXTURED_RIGHT_FIN[0],
    FISH_PATTERN_DEFORMATION,
)];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_LEFT_FIN: [TexturedModelCubeDesc;
    1] = [inflate_textured_cube(
    TROPICAL_FISH_SMALL_TEXTURED_LEFT_FIN[0],
    FISH_PATTERN_DEFORMATION,
)];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_TOP_FIN: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_SMALL_TEXTURED_TOP_FIN[0],
        FISH_PATTERN_DEFORMATION,
    )];

/// Pattern overlay parts for the small (kob) body — the base poses with inflated cubes.
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_PARTS: [TexturedModelPartDesc; 5] = [
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[0].pose,
        cubes: &TROPICAL_FISH_SMALL_PATTERN_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[1].pose,
        cubes: &TROPICAL_FISH_SMALL_PATTERN_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[2].pose,
        cubes: &TROPICAL_FISH_SMALL_PATTERN_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[3].pose,
        cubes: &TROPICAL_FISH_SMALL_PATTERN_LEFT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_SMALL_PARTS[4].pose,
        cubes: &TROPICAL_FISH_SMALL_PATTERN_TOP_FIN,
        children: &[],
    },
];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_BODY: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_LARGE_TEXTURED_BODY[0],
        FISH_PATTERN_DEFORMATION,
    )];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_TAIL: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_LARGE_TEXTURED_TAIL[0],
        FISH_PATTERN_DEFORMATION,
    )];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_RIGHT_FIN: [TexturedModelCubeDesc;
    1] = [inflate_textured_cube(
    TROPICAL_FISH_LARGE_TEXTURED_RIGHT_FIN[0],
    FISH_PATTERN_DEFORMATION,
)];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_LEFT_FIN: [TexturedModelCubeDesc;
    1] = [inflate_textured_cube(
    TROPICAL_FISH_LARGE_TEXTURED_LEFT_FIN[0],
    FISH_PATTERN_DEFORMATION,
)];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_TOP_FIN: [TexturedModelCubeDesc; 1] =
    [inflate_textured_cube(
        TROPICAL_FISH_LARGE_TEXTURED_TOP_FIN[0],
        FISH_PATTERN_DEFORMATION,
    )];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_BOTTOM_FIN: [TexturedModelCubeDesc;
    1] = [inflate_textured_cube(
    TROPICAL_FISH_LARGE_TEXTURED_BOTTOM_FIN[0],
    FISH_PATTERN_DEFORMATION,
)];

/// Pattern overlay parts for the large (flopper) body — the base poses with inflated cubes.
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_PARTS: [TexturedModelPartDesc; 6] = [
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[0].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[1].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_TAIL,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[2].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_RIGHT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[3].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_LEFT_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[4].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_TOP_FIN,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: TROPICAL_FISH_LARGE_PARTS[5].pose,
        cubes: &TROPICAL_FISH_LARGE_PATTERN_BOTTOM_FIN,
        children: &[],
    },
];

/// The pattern overlay parts for a body shape.
pub(in crate::entity_models) fn tropical_fish_pattern_textured_parts(
    shape: TropicalFishModelShape,
) -> &'static [TexturedModelPartDesc] {
    match shape {
        TropicalFishModelShape::Small => &TROPICAL_FISH_SMALL_PATTERN_PARTS,
        TropicalFishModelShape::Large => &TROPICAL_FISH_LARGE_PATTERN_PARTS,
    }
}

/// Vanilla `TropicalFishPatternLayer` texture for a pattern (`tropical_{a,b}_pattern_{1..6}`).
pub(in crate::entity_models) fn tropical_fish_pattern_texture_ref(
    pattern: TropicalFishPattern,
) -> EntityModelTextureRef {
    match pattern {
        TropicalFishPattern::Kob => TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Sunstreak => TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Snooper => TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Dasher => TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Brinely => TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Spotty => TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Flopper => TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Stripey => TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Glitter => TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Blockfish => TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Betty => TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
        TropicalFishPattern::Clayfish => TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF,
    }
}
