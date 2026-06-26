use super::{
    apply_head_look, apply_quadruped_leg_swing, PartPose, PART_POSE_ZERO, PIG_COLD_FUR, PIG_PINK,
};
use crate::entity_models::catalog::PigModelVariant;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PIG: &str = "minecraft:pig#main";
pub(in crate::entity_models) const MODEL_LAYER_PIG_BABY: &str = "minecraft:pig_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_PIG: &str = "minecraft:cold_pig#main";
#[cfg(test)]
pub(in crate::entity_models) const MODEL_LAYER_PIG_SADDLE: &str = "minecraft:pig#saddle";

const PIG_SADDLE_DEFORMATION: f32 = 0.5;

const fn pig_saddle_cube(cube: ModelCube) -> ModelCube {
    ModelCube::new(
        [
            cube.min[0] - PIG_SADDLE_DEFORMATION,
            cube.min[1] - PIG_SADDLE_DEFORMATION,
            cube.min[2] - PIG_SADDLE_DEFORMATION,
        ],
        [
            cube.size[0] + 2.0 * PIG_SADDLE_DEFORMATION,
            cube.size[1] + 2.0 * PIG_SADDLE_DEFORMATION,
            cube.size[2] + 2.0 * PIG_SADDLE_DEFORMATION,
        ],
        PIG_PINK,
        cube.uv_size,
        cube.tex,
        cube.mirror,
    )
}

// Vanilla 26.1 PigModel.createBodyLayer(CubeDeformation.NONE). Each cube carries both render paths'
// data: the colored debug tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_PIG_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -4.0, -8.0],
        [8.0, 8.0, 8.0],
        PIG_PINK,
        [8.0, 8.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 0.0, -9.0],
        [4.0, 3.0, 1.0],
        PIG_PINK,
        [4.0, 3.0, 1.0],
        [16.0, 16.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_PIG_BODY: [ModelCube; 1] = [ModelCube::new(
    [-5.0, -10.0, -7.0],
    [10.0, 16.0, 8.0],
    PIG_PINK,
    [10.0, 16.0, 8.0],
    [28.0, 8.0],
    false,
)];

// The cold pig body's second cube carries a `CubeDeformation`: the colored min/size is the inflated
// 11×17×9 box while the textured `uv_size` keeps the base 10×16×8 (the squid body precedent).
pub(in crate::entity_models) const COLD_PIG_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -10.0, -7.0],
        [10.0, 16.0, 8.0],
        PIG_PINK,
        [10.0, 16.0, 8.0],
        [28.0, 8.0],
        false,
    ),
    ModelCube::new(
        [-5.5, -10.5, -7.5],
        [11.0, 17.0, 9.0],
        PIG_COLD_FUR,
        [10.0, 16.0, 8.0],
        [28.0, 32.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_PIG_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 6.0, 4.0],
    PIG_PINK,
    [4.0, 6.0, 4.0],
    [0.0, 16.0],
    false,
)];

// Vanilla 26.1 ModelLayers.PIG_SADDLE = PigModel.createBodyLayer(CubeDeformation(0.5F)).
pub(in crate::entity_models) const PIG_SADDLE_HEAD: [ModelCube; 2] = [
    pig_saddle_cube(ADULT_PIG_HEAD[0]),
    pig_saddle_cube(ADULT_PIG_HEAD[1]),
];
pub(in crate::entity_models) const PIG_SADDLE_BODY: [ModelCube; 1] =
    [pig_saddle_cube(ADULT_PIG_BODY[0])];
pub(in crate::entity_models) const PIG_SADDLE_LEG: [ModelCube; 1] =
    [pig_saddle_cube(ADULT_PIG_LEG[0])];

pub(in crate::entity_models) const BABY_PIG_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -3.0, -4.5],
    [7.0, 6.0, 9.0],
    PIG_PINK,
    [7.0, 6.0, 9.0],
    [0.0, 0.0],
    false,
)];

// BabyPigModel bakes CubeDeformation into the ModelPart.Cube render bounds (colored geometry), while
// the textured `uv_size` keeps the base box.
pub(in crate::entity_models) const BABY_PIG_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-3.525, -5.025, -5.025],
        [7.05, 6.05, 6.05],
        PIG_PINK,
        [7.0, 6.0, 6.0],
        [0.0, 15.0],
        false,
    ),
    ModelCube::new(
        [-1.515, -1.99, -6.015],
        [3.03, 2.03, 1.03],
        PIG_PINK,
        [3.0, 2.0, 1.0],
        [6.0, 27.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_PIG_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    PIG_PINK,
    [2.0, 2.0, 2.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIG_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    PIG_PINK,
    [2.0, 2.0, 2.0],
    [23.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIG_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    PIG_PINK,
    [2.0, 2.0, 2.0],
    [0.0, 4.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIG_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 2.0, 2.0],
    PIG_PINK,
    [2.0, 2.0, 2.0],
    [23.0, 4.0],
    false,
)];

/// The adult/cold pig head part pose (vanilla `PartPose.offset(0, 12, -6)`).
const ADULT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 12.0, -6.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The adult/cold pig body part pose (`PartPose.offsetAndRotation(0, 11, 2, π/2, 0, 0)`).
const ADULT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 11.0, 2.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

/// Builds a leaf part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes.to_vec(),
    )
}

/// Builds the four adult-pig legs (hind-first, vanilla order) under the vanilla `QuadrupedModel`
/// child names. The adult and cold layouts share the same leg poses and cube.
fn adult_legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_hind_leg", leg([-3.0, 18.0, 7.0], &ADULT_PIG_LEG)),
        ("left_hind_leg", leg([3.0, 18.0, 7.0], &ADULT_PIG_LEG)),
        ("right_front_leg", leg([-3.0, 18.0, -5.0], &ADULT_PIG_LEG)),
        ("left_front_leg", leg([3.0, 18.0, -5.0], &ADULT_PIG_LEG)),
    ]
}

/// Builds a unified pig root for `variant`/`baby`, mirroring the vanilla layer choice (cold pigs
/// carry their fur layer, babies their squat layout) with the vanilla `QuadrupedModel` child names.
fn pig_tree(variant: PigModelVariant, baby: bool) -> ModelPart {
    if baby {
        let children = vec![
            ("body", leg([0.0, 19.0, 0.5], &BABY_PIG_BODY)),
            (
                "head",
                ModelPart::leaf(
                    PartPose {
                        offset: [0.0, 19.0, -2.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_PIG_HEAD.to_vec(),
                ),
            ),
            (
                "left_front_leg",
                leg([2.5, 22.0, -3.0], &BABY_PIG_LEFT_FRONT_LEG),
            ),
            (
                "right_front_leg",
                leg([-2.5, 22.0, -3.0], &BABY_PIG_RIGHT_FRONT_LEG),
            ),
            (
                "left_hind_leg",
                leg([2.5, 22.0, 4.0], &BABY_PIG_LEFT_HIND_LEG),
            ),
            (
                "right_hind_leg",
                leg([-2.5, 22.0, 4.0], &BABY_PIG_RIGHT_HIND_LEG),
            ),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let body_cubes: &[ModelCube] = match variant {
        PigModelVariant::Cold => &COLD_PIG_BODY,
        _ => &ADULT_PIG_BODY,
    };
    let mut children = vec![
        (
            "head",
            ModelPart::leaf(ADULT_HEAD_POSE, ADULT_PIG_HEAD.to_vec()),
        ),
        (
            "body",
            ModelPart::leaf(ADULT_BODY_POSE, body_cubes.to_vec()),
        ),
    ];
    children.extend(adult_legs());
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

fn pig_saddle_tree() -> ModelPart {
    let mut children = vec![
        (
            "head",
            ModelPart::leaf(ADULT_HEAD_POSE, PIG_SADDLE_HEAD.to_vec()),
        ),
        (
            "body",
            ModelPart::leaf(ADULT_BODY_POSE, PIG_SADDLE_BODY.to_vec()),
        ),
    ];
    children.extend(vec![
        ("right_hind_leg", leg([-3.0, 18.0, 7.0], &PIG_SADDLE_LEG)),
        ("left_hind_leg", leg([3.0, 18.0, 7.0], &PIG_SADDLE_LEG)),
        ("right_front_leg", leg([-3.0, 18.0, -5.0], &PIG_SADDLE_LEG)),
        ("left_front_leg", leg([3.0, 18.0, -5.0], &PIG_SADDLE_LEG)),
    ]);
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable pig model, mirroring vanilla `PigModel` (a `QuadrupedModel`). The unified tree is built
/// for the selected `variant`/`baby` layout with the vanilla child names. `setup_anim` looks the
/// head ([`apply_head_look`] on `head`) and swings the four legs ([`apply_quadruped_leg_swing`]).
pub(in crate::entity_models) struct PigModel {
    root: ModelPart,
}

impl PigModel {
    pub(in crate::entity_models) fn new(variant: PigModelVariant, baby: bool) -> Self {
        Self {
            root: pig_tree(variant, baby),
        }
    }

    pub(in crate::entity_models) fn new_saddle() -> Self {
        Self {
            root: pig_saddle_tree(),
        }
    }
}

impl EntityModel for PigModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
