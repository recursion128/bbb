use super::{apply_head_look, apply_quadruped_leg_swing, PartPose, PART_POSE_ZERO};
use crate::entity_models::catalog::CowModelVariant;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const COW_BROWN: [f32; 4] = [0.38, 0.25, 0.18, 1.0];
pub(in crate::entity_models) const COW_COLD_FUR: [f32; 4] = [0.70, 0.66, 0.58, 1.0];

// Vanilla 26.1 CowModel.createBodyLayer(). Each cube carries both render paths' data: the colored
// debug tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_COW_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-4.0, -4.0, -6.0],
        [8.0, 8.0, 6.0],
        COW_BROWN,
        [8.0, 8.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-3.0, 1.0, -7.0],
        [6.0, 3.0, 1.0],
        COW_BROWN,
        [6.0, 3.0, 1.0],
        [1.0, 33.0],
        false,
    ),
    ModelCube::new(
        [-5.0, -5.0, -5.0],
        [1.0, 3.0, 1.0],
        COW_BROWN,
        [1.0, 3.0, 1.0],
        [22.0, 0.0],
        false,
    ),
    ModelCube::new(
        [4.0, -5.0, -5.0],
        [1.0, 3.0, 1.0],
        COW_BROWN,
        [1.0, 3.0, 1.0],
        [22.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const WARM_COW_HEAD: [ModelCube; 6] = [
    ModelCube::new(
        [-4.0, -4.0, -6.0],
        [8.0, 8.0, 6.0],
        COW_BROWN,
        [8.0, 8.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-3.0, 1.0, -7.0],
        [6.0, 3.0, 1.0],
        COW_BROWN,
        [6.0, 3.0, 1.0],
        [1.0, 33.0],
        false,
    ),
    ModelCube::new(
        [-8.0, -3.0, -5.0],
        [4.0, 2.0, 2.0],
        COW_BROWN,
        [4.0, 2.0, 2.0],
        [27.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-8.0, -5.0, -5.0],
        [2.0, 2.0, 2.0],
        COW_BROWN,
        [2.0, 2.0, 2.0],
        [39.0, 0.0],
        false,
    ),
    ModelCube::new(
        [4.0, -3.0, -5.0],
        [4.0, 2.0, 2.0],
        COW_BROWN,
        [4.0, 2.0, 2.0],
        [27.0, 0.0],
        true,
    ),
    ModelCube::new(
        [6.0, -5.0, -5.0],
        [2.0, 2.0, 2.0],
        COW_BROWN,
        [2.0, 2.0, 2.0],
        [39.0, 0.0],
        true,
    ),
];

pub(in crate::entity_models) const COLD_COW_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -4.0, -6.0],
        [8.0, 8.0, 6.0],
        COW_BROWN,
        [8.0, 8.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-3.0, 1.0, -7.0],
        [6.0, 3.0, 1.0],
        COW_BROWN,
        [6.0, 3.0, 1.0],
        [9.0, 33.0],
        false,
    ),
];

pub(in crate::entity_models) const COLD_COW_RIGHT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -4.5, -0.5],
    [2.0, 6.0, 2.0],
    COW_COLD_FUR,
    [2.0, 6.0, 2.0],
    [0.0, 40.0],
    false,
)];

pub(in crate::entity_models) const COLD_COW_LEFT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -3.0, -0.5],
    [2.0, 6.0, 2.0],
    COW_COLD_FUR,
    [2.0, 6.0, 2.0],
    [0.0, 32.0],
    false,
)];

/// Cold-cow head horn child part poses (vanilla `ColdCowModel.createBodyLayer`).
pub(in crate::entity_models) const COLD_COW_RIGHT_HORN_POSE: PartPose = PartPose {
    offset: [-4.5, -2.5, -3.5],
    rotation: [1.5708, 0.0, 0.0],
};

pub(in crate::entity_models) const COLD_COW_LEFT_HORN_POSE: PartPose = PartPose {
    offset: [5.5, -2.5, -5.0],
    rotation: [1.5708, 0.0, 0.0],
};

pub(in crate::entity_models) const ADULT_COW_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-6.0, -10.0, -7.0],
        [12.0, 18.0, 10.0],
        COW_BROWN,
        [12.0, 18.0, 10.0],
        [18.0, 4.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 2.0, -8.0],
        [4.0, 6.0, 1.0],
        COW_BROWN,
        [4.0, 6.0, 1.0],
        [52.0, 0.0],
        false,
    ),
];

// The cold cow body's first cube carries a `CubeDeformation`: the colored min/size is the inflated
// 13×19×11 box while the textured `uv_size` keeps the base 12×18×10 (the squid body precedent).
pub(in crate::entity_models) const COLD_COW_BODY: [ModelCube; 3] = [
    ModelCube::new(
        [-6.5, -10.5, -7.5],
        [13.0, 19.0, 11.0],
        COW_COLD_FUR,
        [12.0, 18.0, 10.0],
        [20.0, 32.0],
        false,
    ),
    ModelCube::new(
        [-6.0, -10.0, -7.0],
        [12.0, 18.0, 10.0],
        COW_BROWN,
        [12.0, 18.0, 10.0],
        [18.0, 4.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 2.0, -8.0],
        [4.0, 6.0, 1.0],
        COW_BROWN,
        [4.0, 6.0, 1.0],
        [52.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_COW_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    COW_BROWN,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_COW_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    COW_BROWN,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    true,
)];

pub(in crate::entity_models) const BABY_COW_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-3.0, -4.569, -4.8333],
        [6.0, 6.0, 5.0],
        COW_BROWN,
        [6.0, 6.0, 5.0],
        [0.0, 18.0],
        false,
    ),
    ModelCube::new(
        [3.0, -5.569, -3.8333],
        [1.0, 2.0, 1.0],
        COW_BROWN,
        [1.0, 2.0, 1.0],
        [8.0, 29.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -5.569, -3.8333],
        [1.0, 2.0, 1.0],
        COW_BROWN,
        [1.0, 2.0, 1.0],
        [4.0, 29.0],
        true,
    ),
    ModelCube::new(
        [-2.0, -1.569, -5.8333],
        [4.0, 3.0, 1.0],
        COW_BROWN,
        [4.0, 3.0, 1.0],
        [12.0, 29.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_COW_BODY: [ModelCube; 1] = [ModelCube::new(
    [-7.0, -7.0, -1.0],
    [8.0, 6.0, 12.0],
    COW_BROWN,
    [8.0, 6.0, 12.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_COW_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    COW_BROWN,
    [3.0, 6.0, 3.0],
    [22.0, 18.0],
    false,
)];

pub(in crate::entity_models) const BABY_COW_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    COW_BROWN,
    [3.0, 6.0, 3.0],
    [34.0, 18.0],
    false,
)];

pub(in crate::entity_models) const BABY_COW_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    COW_BROWN,
    [3.0, 6.0, 3.0],
    [22.0, 27.0],
    false,
)];

pub(in crate::entity_models) const BABY_COW_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    COW_BROWN,
    [3.0, 6.0, 3.0],
    [34.0, 27.0],
    false,
)];

pub(in crate::entity_models) const MODEL_LAYER_COW: &str = "minecraft:cow#main";
pub(in crate::entity_models) const MODEL_LAYER_COW_BABY: &str = "minecraft:cow_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_WARM_COW: &str = "minecraft:warm_cow#main";
pub(in crate::entity_models) const MODEL_LAYER_WARM_COW_BABY: &str = "minecraft:warm_cow_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_COW: &str = "minecraft:cold_cow#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_COW_BABY: &str = "minecraft:cold_cow_baby#main";

/// The adult/warm/cold cow head part pose (vanilla `PartPose.offset(0, 4, -8)`).
const ADULT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The adult/warm/cold cow body part pose (`PartPose.offsetAndRotation(0, 5, 2, π/2, 0, 0)`).
const ADULT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 5.0, 2.0],
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

/// Builds the four adult-cow legs (hind-first, vanilla order) under the vanilla `QuadrupedModel`
/// child names. The adult, warm, and cold layouts share the same leg poses and cubes.
fn adult_legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        (
            "right_hind_leg",
            leg([-4.0, 12.0, 7.0], &ADULT_COW_RIGHT_LEG),
        ),
        ("left_hind_leg", leg([4.0, 12.0, 7.0], &ADULT_COW_LEFT_LEG)),
        (
            "right_front_leg",
            leg([-4.0, 12.0, -5.0], &ADULT_COW_RIGHT_LEG),
        ),
        (
            "left_front_leg",
            leg([4.0, 12.0, -5.0], &ADULT_COW_LEFT_LEG),
        ),
    ]
}

/// Builds a unified cow root for `variant`/`baby`, mirroring the vanilla layer choice
/// (temperate/warm/cold adult coats, or the baby layout) with the vanilla `QuadrupedModel` child
/// names (`head`, `body`, the four legs; the cold head also parents `right_horn`/`left_horn`).
fn cow_tree(variant: CowModelVariant, baby: bool) -> ModelPart {
    if baby {
        let children = vec![
            (
                "head",
                ModelPart::leaf(
                    PartPose {
                        offset: [0.0, 13.569, -5.1667],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_COW_HEAD.to_vec(),
                ),
            ),
            ("body", leg([3.0, 19.0, -5.0], &BABY_COW_BODY)),
            (
                "right_front_leg",
                leg([-2.5, 18.0, -3.5], &BABY_COW_RIGHT_FRONT_LEG),
            ),
            (
                "left_front_leg",
                leg([2.5, 18.0, -3.5], &BABY_COW_LEFT_FRONT_LEG),
            ),
            (
                "right_hind_leg",
                leg([-2.5, 18.0, 3.5], &BABY_COW_RIGHT_HIND_LEG),
            ),
            (
                "left_hind_leg",
                leg([2.5, 18.0, 3.5], &BABY_COW_LEFT_HIND_LEG),
            ),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let (head, body) = match variant {
        CowModelVariant::Warm => (
            ModelPart::leaf(ADULT_HEAD_POSE, WARM_COW_HEAD.to_vec()),
            leg_body(&ADULT_COW_BODY),
        ),
        CowModelVariant::Cold => (
            ModelPart::new(
                ADULT_HEAD_POSE,
                COLD_COW_HEAD.to_vec(),
                vec![
                    (
                        "right_horn",
                        ModelPart::leaf(COLD_COW_RIGHT_HORN_POSE, COLD_COW_RIGHT_HORN.to_vec()),
                    ),
                    (
                        "left_horn",
                        ModelPart::leaf(COLD_COW_LEFT_HORN_POSE, COLD_COW_LEFT_HORN.to_vec()),
                    ),
                ],
            ),
            leg_body(&COLD_COW_BODY),
        ),
        CowModelVariant::Temperate => (
            ModelPart::leaf(ADULT_HEAD_POSE, ADULT_COW_HEAD.to_vec()),
            leg_body(&ADULT_COW_BODY),
        ),
    };
    let mut children = vec![("head", head), ("body", body)];
    children.extend(adult_legs());
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the cow body part at the shared adult body pose carrying `cubes`.
fn leg_body(cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(ADULT_BODY_POSE, cubes.to_vec())
}

/// Mutable cow model, mirroring vanilla `CowModel` (a `QuadrupedModel`). The unified tree is built
/// for the selected `variant`/`baby` layout with the vanilla child names. `setup_anim` looks the
/// head ([`apply_head_look`] on `head`) and swings the four legs ([`apply_quadruped_leg_swing`]).
pub(in crate::entity_models) struct CowModel {
    root: ModelPart,
}

impl CowModel {
    pub(in crate::entity_models) fn new(variant: CowModelVariant, baby: bool) -> Self {
        Self {
            root: cow_tree(variant, baby),
        }
    }
}

impl EntityModel for CowModel {
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
