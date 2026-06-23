use super::{
    PartPose, PART_POSE_ZERO, TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_BLOCKFISH_PATTERN_TEXTURE_REF, TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_CLAYFISH_PATTERN_TEXTURE_REF, TROPICAL_FISH_DASHER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_FLOPPER_PATTERN_TEXTURE_REF, TROPICAL_FISH_GLITTER_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_KOB_PATTERN_TEXTURE_REF, TROPICAL_FISH_ORANGE,
    TROPICAL_FISH_SNOOPER_PATTERN_TEXTURE_REF, TROPICAL_FISH_SPOTTY_PATTERN_TEXTURE_REF,
    TROPICAL_FISH_STRIPEY_PATTERN_TEXTURE_REF, TROPICAL_FISH_SUNSTREAK_PATTERN_TEXTURE_REF,
};
use crate::entity_models::catalog::{TropicalFishModelShape, TropicalFishPattern};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};
use crate::entity_models::EntityModelTextureRef;

// Vanilla 26.1 `TropicalFishSmallModel.createBodyLayer` (kob-style body, atlas 32×32,
// `CubeDeformation.NONE`, so each `uv_size` equals the geometry size). The tail and top fin are
// zero-thickness planes flat in X; the two side fins are zero-thickness planes flat in Z, splayed
// ±π/4 about Y. Each unified cube carries both render paths' data: the colored debug tint
// (`TROPICAL_FISH_ORANGE`) and the textured `uv_size` / `texOffs` (the tail and top fin keep their
// negative `texOffs` V origins exactly as vanilla bakes them).
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_BODY: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.5, -3.0],
    [2.0, 3.0, 6.0],
    TROPICAL_FISH_ORANGE,
    [2.0, 3.0, 6.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TAIL: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.5, 0.0],
    [0.0, 3.0, 6.0],
    TROPICAL_FISH_ORANGE,
    [0.0, 3.0, 6.0],
    [22.0, -6.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_RIGHT_FIN: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, -1.0, 0.0],
        [2.0, 2.0, 0.0],
        TROPICAL_FISH_ORANGE,
        [2.0, 2.0, 0.0],
        [2.0, 16.0],
        false,
    )];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_LEFT_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, 0.0],
    [2.0, 2.0, 0.0],
    TROPICAL_FISH_ORANGE,
    [2.0, 2.0, 0.0],
    [2.0, 12.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TOP_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -3.0, 0.0],
    [0.0, 3.0, 6.0],
    TROPICAL_FISH_ORANGE,
    [0.0, 3.0, 6.0],
    [10.0, -5.0],
    false,
)];

/// Bind poses of the small (kob) layer's parts: body, tail (swayed by `setupAnim`), right fin
/// (`yRot = π/4`), left fin (`yRot = -π/4`), top fin — the vanilla
/// `TropicalFishSmallModel.createBodyLayer` part order.
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, 3.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_RIGHT_FIN_POSE: PartPose = PartPose {
    offset: [-1.0, 22.5, 0.0],
    rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_LEFT_FIN_POSE: PartPose = PartPose {
    offset: [1.0, 22.5, 0.0],
    rotation: [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_TOP_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 20.5, -3.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `TropicalFishLargeModel.createBodyLayer` (flopper-style body, atlas 32×32,
// `CubeDeformation.NONE`). Adds a bottom fin to the small layout; the tail is a 5-deep plane and the
// body is twice as tall.
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BODY: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -3.0, -3.0],
    [2.0, 6.0, 6.0],
    TROPICAL_FISH_ORANGE,
    [2.0, 6.0, 6.0],
    [0.0, 20.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TAIL: [ModelCube; 1] = [ModelCube::new(
    [0.0, -3.0, 0.0],
    [0.0, 6.0, 5.0],
    TROPICAL_FISH_ORANGE,
    [0.0, 6.0, 5.0],
    [21.0, 16.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_RIGHT_FIN: [ModelCube; 1] =
    [ModelCube::new(
        [-2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        TROPICAL_FISH_ORANGE,
        [2.0, 2.0, 0.0],
        [2.0, 16.0],
        false,
    )];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_LEFT_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [2.0, 2.0, 0.0],
    TROPICAL_FISH_ORANGE,
    [2.0, 2.0, 0.0],
    [2.0, 12.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TOP_FIN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -4.0, 0.0],
    [0.0, 4.0, 6.0],
    TROPICAL_FISH_ORANGE,
    [0.0, 4.0, 6.0],
    [20.0, 11.0],
    false,
)];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BOTTOM_FIN: [ModelCube; 1] =
    [ModelCube::new(
        [0.0, 0.0, 0.0],
        [0.0, 4.0, 6.0],
        TROPICAL_FISH_ORANGE,
        [0.0, 4.0, 6.0],
        [20.0, 21.0],
        false,
    )];

/// Bind poses of the large (flopper) layer's parts: body, tail, right fin (`yRot = π/4`), left fin
/// (`yRot = -π/4`), top fin, bottom fin.
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TAIL_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, 3.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_RIGHT_FIN_POSE: PartPose = PartPose {
    offset: [-1.0, 20.0, 0.0],
    rotation: [0.0, std::f32::consts::FRAC_PI_4, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_LEFT_FIN_POSE: PartPose = PartPose {
    offset: [1.0, 20.0, 0.0],
    rotation: [0.0, -std::f32::consts::FRAC_PI_4, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_TOP_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_BOTTOM_FIN_POSE: PartPose = PartPose {
    offset: [0.0, 22.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};

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

// Vanilla `LayerDefinitions.FISH_PATTERN_DEFORMATION = new CubeDeformation(0.008F)`: the
// `TropicalFishPatternLayer` bakes the same body geometry one notch larger so the overlay sits
// just outside the base body without z-fighting.
pub(in crate::entity_models) const FISH_PATTERN_DEFORMATION: f32 = 0.008;

/// Inflates a base-body cube by `FISH_PATTERN_DEFORMATION` for the pattern overlay, reproducing
/// vanilla `CubeDeformation` (`min -= g`, `size += 2·g`) while keeping the base box for UVs and the
/// same `texOffs`/mirror. The overlay has no colored debug variant, so the inflated cube's `color`
/// is the base body's tint (unused — only [`ModelPart::render_textured`] is called on the overlay).
const fn inflate_pattern_cube(cube: ModelCube) -> ModelCube {
    let g = FISH_PATTERN_DEFORMATION;
    ModelCube::new(
        [cube.min[0] - g, cube.min[1] - g, cube.min[2] - g],
        [
            cube.size[0] + 2.0 * g,
            cube.size[1] + 2.0 * g,
            cube.size[2] + 2.0 * g,
        ],
        cube.color,
        cube.uv_size,
        cube.tex,
        cube.mirror,
    )
}

pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_BODY: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_SMALL_BODY[0])];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_TAIL: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_SMALL_TAIL[0])];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_RIGHT_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_SMALL_RIGHT_FIN[0])];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_LEFT_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_SMALL_LEFT_FIN[0])];
pub(in crate::entity_models) const TROPICAL_FISH_SMALL_PATTERN_TOP_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_SMALL_TOP_FIN[0])];

pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_BODY: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_BODY[0])];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_TAIL: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_TAIL[0])];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_RIGHT_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_RIGHT_FIN[0])];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_LEFT_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_LEFT_FIN[0])];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_TOP_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_TOP_FIN[0])];
pub(in crate::entity_models) const TROPICAL_FISH_LARGE_PATTERN_BOTTOM_FIN: [ModelCube; 1] =
    [inflate_pattern_cube(TROPICAL_FISH_LARGE_BOTTOM_FIN[0])];

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

/// Builds a tropical fish body tree (base body or inflated pattern overlay) with the vanilla
/// `TropicalFish{Small,Large}Model.createBodyLayer` child names. The `body_cubes`/.../`tail_cubes`
/// closures pick the base or inflated cube set per part, so the base body and the pattern overlay
/// share one tree shape (and thus one `setup_anim` tail sway). The large layer adds a `bottom_fin`.
fn tropical_fish_tree(
    shape: TropicalFishModelShape,
    cubes_for: impl Fn(&'static str) -> Vec<ModelCube>,
) -> ModelPart {
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(6);
    match shape {
        TropicalFishModelShape::Small => {
            children.push((
                "body",
                ModelPart::leaf(TROPICAL_FISH_SMALL_BODY_POSE, cubes_for("body")),
            ));
            children.push((
                "tail",
                ModelPart::leaf(TROPICAL_FISH_SMALL_TAIL_POSE, cubes_for("tail")),
            ));
            children.push((
                "right_fin",
                ModelPart::leaf(TROPICAL_FISH_SMALL_RIGHT_FIN_POSE, cubes_for("right_fin")),
            ));
            children.push((
                "left_fin",
                ModelPart::leaf(TROPICAL_FISH_SMALL_LEFT_FIN_POSE, cubes_for("left_fin")),
            ));
            children.push((
                "top_fin",
                ModelPart::leaf(TROPICAL_FISH_SMALL_TOP_FIN_POSE, cubes_for("top_fin")),
            ));
        }
        TropicalFishModelShape::Large => {
            children.push((
                "body",
                ModelPart::leaf(TROPICAL_FISH_LARGE_BODY_POSE, cubes_for("body")),
            ));
            children.push((
                "tail",
                ModelPart::leaf(TROPICAL_FISH_LARGE_TAIL_POSE, cubes_for("tail")),
            ));
            children.push((
                "right_fin",
                ModelPart::leaf(TROPICAL_FISH_LARGE_RIGHT_FIN_POSE, cubes_for("right_fin")),
            ));
            children.push((
                "left_fin",
                ModelPart::leaf(TROPICAL_FISH_LARGE_LEFT_FIN_POSE, cubes_for("left_fin")),
            ));
            children.push((
                "top_fin",
                ModelPart::leaf(TROPICAL_FISH_LARGE_TOP_FIN_POSE, cubes_for("top_fin")),
            ));
            children.push((
                "bottom_fin",
                ModelPart::leaf(TROPICAL_FISH_LARGE_BOTTOM_FIN_POSE, cubes_for("bottom_fin")),
            ));
        }
    }
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Applies the vanilla `TropicalFish{Small,Large}Model.setupAnim` tail sway to a unified tree: the
/// `tail` child's `yRot` is set to [`tropical_fish_tail_yrot`] (absolute, like vanilla); every other
/// part stays at the bind pose. Shared by the base body and the pattern overlay so the overlay sways
/// with the body. The body shape, swim wiggle, and out-of-water flop live in the root transform.
fn apply_tropical_fish_tail_sway(root: &mut ModelPart, instance: &EntityModelInstance) {
    let in_water = instance.render_state.in_water;
    let tail_yrot = tropical_fish_tail_yrot(instance.render_state.age_in_ticks, in_water);
    root.child_mut("tail").pose.rotation[1] = tail_yrot;
}

/// Mutable tropical fish base-body model, mirroring vanilla `TropicalFishSmallModel` /
/// `TropicalFishLargeModel`. The unified tree is built with named children selected by `shape`;
/// `setup_anim` runs the shared [`apply_tropical_fish_tail_sway`]. The colored fallback recolors the
/// whole body with the base color's diffuse tint; the textured base layer renders it tinted by the
/// same color (`getModelTint`).
pub(in crate::entity_models) struct TropicalFishModel {
    root: ModelPart,
}

impl TropicalFishModel {
    pub(in crate::entity_models) fn new(shape: TropicalFishModelShape) -> Self {
        let cubes_for = |name: &'static str| match (shape, name) {
            (TropicalFishModelShape::Small, "body") => TROPICAL_FISH_SMALL_BODY.to_vec(),
            (TropicalFishModelShape::Small, "tail") => TROPICAL_FISH_SMALL_TAIL.to_vec(),
            (TropicalFishModelShape::Small, "right_fin") => TROPICAL_FISH_SMALL_RIGHT_FIN.to_vec(),
            (TropicalFishModelShape::Small, "left_fin") => TROPICAL_FISH_SMALL_LEFT_FIN.to_vec(),
            (TropicalFishModelShape::Small, "top_fin") => TROPICAL_FISH_SMALL_TOP_FIN.to_vec(),
            (TropicalFishModelShape::Large, "body") => TROPICAL_FISH_LARGE_BODY.to_vec(),
            (TropicalFishModelShape::Large, "tail") => TROPICAL_FISH_LARGE_TAIL.to_vec(),
            (TropicalFishModelShape::Large, "right_fin") => TROPICAL_FISH_LARGE_RIGHT_FIN.to_vec(),
            (TropicalFishModelShape::Large, "left_fin") => TROPICAL_FISH_LARGE_LEFT_FIN.to_vec(),
            (TropicalFishModelShape::Large, "top_fin") => TROPICAL_FISH_LARGE_TOP_FIN.to_vec(),
            (TropicalFishModelShape::Large, "bottom_fin") => {
                TROPICAL_FISH_LARGE_BOTTOM_FIN.to_vec()
            }
            _ => panic!("tropical fish base body has no part named `{name}`"),
        };
        Self {
            root: tropical_fish_tree(shape, cubes_for),
        }
    }
}

impl EntityModel for TropicalFishModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_tropical_fish_tail_sway(&mut self.root, instance);
    }
}

/// Mutable tropical fish pattern-overlay model, mirroring vanilla `TropicalFishPatternLayer`. The
/// overlay is the body mesh inflated by `FISH_PATTERN_DEFORMATION`, with no colored debug variant, so
/// the tree carries the inflated cubes; `setup_anim` runs the same [`apply_tropical_fish_tail_sway`]
/// so the pattern sways with the body. Rendered only on the textured path, tinted by the pattern
/// color's diffuse tint.
pub(in crate::entity_models) struct TropicalFishPatternModel {
    root: ModelPart,
}

impl TropicalFishPatternModel {
    pub(in crate::entity_models) fn new(shape: TropicalFishModelShape) -> Self {
        let cubes_for = |name: &'static str| match (shape, name) {
            (TropicalFishModelShape::Small, "body") => TROPICAL_FISH_SMALL_PATTERN_BODY.to_vec(),
            (TropicalFishModelShape::Small, "tail") => TROPICAL_FISH_SMALL_PATTERN_TAIL.to_vec(),
            (TropicalFishModelShape::Small, "right_fin") => {
                TROPICAL_FISH_SMALL_PATTERN_RIGHT_FIN.to_vec()
            }
            (TropicalFishModelShape::Small, "left_fin") => {
                TROPICAL_FISH_SMALL_PATTERN_LEFT_FIN.to_vec()
            }
            (TropicalFishModelShape::Small, "top_fin") => {
                TROPICAL_FISH_SMALL_PATTERN_TOP_FIN.to_vec()
            }
            (TropicalFishModelShape::Large, "body") => TROPICAL_FISH_LARGE_PATTERN_BODY.to_vec(),
            (TropicalFishModelShape::Large, "tail") => TROPICAL_FISH_LARGE_PATTERN_TAIL.to_vec(),
            (TropicalFishModelShape::Large, "right_fin") => {
                TROPICAL_FISH_LARGE_PATTERN_RIGHT_FIN.to_vec()
            }
            (TropicalFishModelShape::Large, "left_fin") => {
                TROPICAL_FISH_LARGE_PATTERN_LEFT_FIN.to_vec()
            }
            (TropicalFishModelShape::Large, "top_fin") => {
                TROPICAL_FISH_LARGE_PATTERN_TOP_FIN.to_vec()
            }
            (TropicalFishModelShape::Large, "bottom_fin") => {
                TROPICAL_FISH_LARGE_PATTERN_BOTTOM_FIN.to_vec()
            }
            _ => panic!("tropical fish pattern overlay has no part named `{name}`"),
        };
        Self {
            root: tropical_fish_tree(shape, cubes_for),
        }
    }
}

impl EntityModel for TropicalFishPatternModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_tropical_fish_tail_sway(&mut self.root, instance);
    }
}
