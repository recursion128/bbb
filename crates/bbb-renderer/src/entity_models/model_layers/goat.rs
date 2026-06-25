use super::{
    apply_head_look, apply_quadruped_leg_swing, PartPose, GOAT_BEARD, GOAT_HORN, GOAT_WHITE,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_GOAT: &str = "minecraft:goat#main";
pub(in crate::entity_models) const MODEL_LAYER_GOAT_BABY: &str = "minecraft:goat_baby#main";

// Vanilla 26.1 ModelLayers.GOAT: GoatModel.createBodyLayer(). Each cube carries both render paths'
// data: the colored debug tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_GOAT_HEAD: [ModelCube; 3] = [
    ModelCube::new(
        [-6.0, -11.0, -10.0],
        [3.0, 2.0, 1.0],
        GOAT_WHITE,
        [3.0, 2.0, 1.0],
        [2.0, 61.0],
        false,
    ),
    ModelCube::new(
        [2.0, -11.0, -10.0],
        [3.0, 2.0, 1.0],
        GOAT_WHITE,
        [3.0, 2.0, 1.0],
        [2.0, 61.0],
        true,
    ),
    ModelCube::new(
        [-0.5, -3.0, -14.0],
        [0.0, 7.0, 5.0],
        GOAT_BEARD,
        [0.0, 7.0, 5.0],
        [23.0, 52.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_GOAT_LEFT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-0.01, -16.0, -10.0],
    [2.0, 7.0, 2.0],
    GOAT_HORN,
    [2.0, 7.0, 2.0],
    [12.0, 55.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_RIGHT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-2.99, -16.0, -10.0],
    [2.0, 7.0, 2.0],
    GOAT_HORN,
    [2.0, 7.0, 2.0],
    [12.0, 55.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -4.0, -8.0],
    [5.0, 7.0, 10.0],
    GOAT_WHITE,
    [5.0, 7.0, 10.0],
    [34.0, 46.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-4.0, -17.0, -7.0],
        [9.0, 11.0, 16.0],
        GOAT_WHITE,
        [9.0, 11.0, 16.0],
        [1.0, 1.0],
        false,
    ),
    ModelCube::new(
        [-5.0, -18.0, -8.0],
        [11.0, 14.0, 11.0],
        GOAT_WHITE,
        [11.0, 14.0, 11.0],
        [0.0, 28.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_GOAT_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 4.0, 0.0],
    [3.0, 6.0, 3.0],
    GOAT_WHITE,
    [3.0, 6.0, 3.0],
    [36.0, 29.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 4.0, 0.0],
    [3.0, 6.0, 3.0],
    GOAT_WHITE,
    [3.0, 6.0, 3.0],
    [49.0, 29.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [3.0, 10.0, 3.0],
    GOAT_WHITE,
    [3.0, 10.0, 3.0],
    [49.0, 2.0],
    false,
)];

pub(in crate::entity_models) const ADULT_GOAT_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [3.0, 10.0, 3.0],
    GOAT_WHITE,
    [3.0, 10.0, 3.0],
    [35.0, 2.0],
    false,
)];

/// The adult goat nose part pose (vanilla `PartPose.offsetAndRotation`).
pub(in crate::entity_models) const ADULT_GOAT_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -8.0, -8.0],
    rotation: [0.9599, 0.0, 0.0],
};

// Vanilla 26.1 ModelLayers.GOAT_BABY: BabyGoatModel.createBodyLayer().
pub(in crate::entity_models) const BABY_GOAT_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    GOAT_WHITE,
    [2.0, 5.0, 2.0],
    [29.0, 12.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    GOAT_WHITE,
    [2.0, 5.0, 2.0],
    [21.0, 12.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    GOAT_WHITE,
    [2.0, 5.0, 2.0],
    [21.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 5.0, 2.0],
    GOAT_WHITE,
    [2.0, 5.0, 2.0],
    [29.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-3.0, -2.3, -4.5],
        [6.0, 5.0, 9.0],
        GOAT_WHITE,
        [6.0, 5.0, 9.0],
        [0.0, 10.0],
        false,
    ),
    ModelCube::new(
        [-2.5, -2.2, -4.0],
        [5.0, 4.0, 8.0],
        GOAT_WHITE,
        [5.0, 4.0, 8.0],
        [0.0, 24.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_GOAT_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -3.8126, -5.1548],
    [4.0, 4.0, 6.0],
    GOAT_WHITE,
    [4.0, 4.0, 6.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_HORN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -4.5, 0.0],
    [1.0, 2.0, 1.0],
    GOAT_HORN,
    [1.0, 2.0, 1.0],
    [24.0, 0.0],
    true,
)];

pub(in crate::entity_models) const BABY_GOAT_LEFT_HORN: [ModelCube; 1] = [ModelCube::new(
    [2.0, -4.5, 0.0],
    [1.0, 2.0, 1.0],
    GOAT_HORN,
    [1.0, 2.0, 1.0],
    [24.0, 0.0],
    true,
)];

pub(in crate::entity_models) const BABY_GOAT_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -0.5, -0.5],
    [2.0, 1.0, 1.0],
    GOAT_WHITE,
    [2.0, 1.0, 1.0],
    [0.0, 12.0],
    true,
)];

pub(in crate::entity_models) const BABY_GOAT_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [0.0, -0.5, -0.5],
    [2.0, 1.0, 1.0],
    GOAT_WHITE,
    [2.0, 1.0, 1.0],
    [0.0, 12.0],
    false,
)];

pub(in crate::entity_models) const BABY_GOAT_HEAD_MAIN: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.5, -4.0],
    [4.0, 4.0, 6.0],
    GOAT_WHITE,
    [4.0, 4.0, 6.0],
    [0.0, 0.0],
    false,
)];

/// The baby goat horn child part pose (shared `PartPose.offsetAndRotation`).
pub(in crate::entity_models) const BABY_GOAT_HORN_POSE: PartPose = PartPose {
    offset: [-1.5, -1.5, -1.0],
    rotation: [-0.3926991, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_GOAT_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.7, -2.3126, 0.1452],
    rotation: [0.0, -0.5236, 0.0],
};

pub(in crate::entity_models) const BABY_GOAT_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.7, -2.3126, 0.1452],
    rotation: [0.0, 0.5236, 0.0],
};

pub(in crate::entity_models) const BABY_GOAT_HEAD_MAIN_POSE: PartPose = PartPose {
    offset: [0.0, -1.3126, -1.1548],
    rotation: [0.0, 0.0, 0.0],
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

/// Builds the adult goat tree with the vanilla `GoatModel` child names: `head` (children
/// `left_horn`, `right_horn`, `nose`), `body`, and the four legs.
fn adult_goat_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [1.0, 14.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        ADULT_GOAT_HEAD.to_vec(),
        vec![
            (
                "left_horn",
                ModelPart::leaf(PART_POSE_ZERO, ADULT_GOAT_LEFT_HORN.to_vec()),
            ),
            (
                "right_horn",
                ModelPart::leaf(PART_POSE_ZERO, ADULT_GOAT_RIGHT_HORN.to_vec()),
            ),
            (
                "nose",
                ModelPart::leaf(ADULT_GOAT_NOSE_POSE, ADULT_GOAT_NOSE.to_vec()),
            ),
        ],
    );
    let body = leg([0.0, 24.0, 0.0], &ADULT_GOAT_BODY);
    let children = vec![
        ("head", head),
        ("body", body),
        (
            "left_hind_leg",
            leg([1.0, 14.0, 4.0], &ADULT_GOAT_LEFT_HIND_LEG),
        ),
        (
            "right_hind_leg",
            leg([-3.0, 14.0, 4.0], &ADULT_GOAT_RIGHT_HIND_LEG),
        ),
        (
            "left_front_leg",
            leg([1.0, 14.0, -6.0], &ADULT_GOAT_LEFT_FRONT_LEG),
        ),
        (
            "right_front_leg",
            leg([-3.0, 14.0, -6.0], &ADULT_GOAT_RIGHT_FRONT_LEG),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the baby goat tree with the vanilla `BabyGoatModel` child names: the four legs, `body`,
/// then `head` (children `right_horn`, `left_horn`, `right_ear`, `left_ear`, `head`).
fn baby_goat_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [0.0, 15.5, -3.0],
            rotation: [0.4363, 0.0, 0.0],
        },
        BABY_GOAT_HEAD.to_vec(),
        vec![
            (
                "right_horn",
                ModelPart::leaf(BABY_GOAT_HORN_POSE, BABY_GOAT_RIGHT_HORN.to_vec()),
            ),
            (
                "left_horn",
                ModelPart::leaf(BABY_GOAT_HORN_POSE, BABY_GOAT_LEFT_HORN.to_vec()),
            ),
            (
                "right_ear",
                ModelPart::leaf(BABY_GOAT_RIGHT_EAR_POSE, BABY_GOAT_RIGHT_EAR.to_vec()),
            ),
            (
                "left_ear",
                ModelPart::leaf(BABY_GOAT_LEFT_EAR_POSE, BABY_GOAT_LEFT_EAR.to_vec()),
            ),
            (
                "head",
                ModelPart::leaf(BABY_GOAT_HEAD_MAIN_POSE, BABY_GOAT_HEAD_MAIN.to_vec()),
            ),
        ],
    );
    let children = vec![
        (
            "left_hind_leg",
            leg([1.5, 19.5, 3.0], &BABY_GOAT_LEFT_HIND_LEG),
        ),
        (
            "right_hind_leg",
            leg([-1.5, 19.5, 3.0], &BABY_GOAT_RIGHT_HIND_LEG),
        ),
        (
            "right_front_leg",
            leg([-1.5, 19.5, -2.0], &BABY_GOAT_RIGHT_FRONT_LEG),
        ),
        (
            "left_front_leg",
            leg([1.5, 19.5, -2.0], &BABY_GOAT_LEFT_FRONT_LEG),
        ),
        ("body", leg([0.0, 17.8, 0.0], &BABY_GOAT_BODY)),
        ("head", head),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable goat model, mirroring vanilla `GoatModel` (a `QuadrupedModel`). The unified tree is built
/// with the vanilla child names for the selected `baby` layout. `setup_anim` looks the head
/// ([`apply_head_look`] on `head`), swings the four legs ([`apply_quadruped_leg_swing`]), and
/// toggles each horn (a `head` child) via the [`ModelPart::visible`] flag from the `left_horn`/
/// `right_horn` flags — vanilla hides the screaming-goat-only horns a polled goat lacks. A ramming goat
/// then tilts its head down (`goat_ramming_x_head_rot`, the projected `Goat.getRammingXHeadRot()`).
pub(in crate::entity_models) struct GoatModel {
    root: ModelPart,
    left_horn: bool,
    right_horn: bool,
}

impl GoatModel {
    pub(in crate::entity_models) fn new(baby: bool, left_horn: bool, right_horn: bool) -> Self {
        let root = if baby {
            baby_goat_tree()
        } else {
            adult_goat_tree()
        };
        Self {
            root,
            left_horn,
            right_horn,
        }
    }
}

impl EntityModel for GoatModel {
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
        let head = self.root.child_mut("head");
        head.child_mut("left_horn").visible = self.left_horn;
        head.child_mut("right_horn").visible = self.right_horn;
        // Vanilla `GoatModel.setupAnim`: a ramming goat tilts its head down
        // (`if rammingXHeadRot != 0 { head.xRot = rammingXHeadRot }`), overwriting the head-look pitch.
        if render_state.goat_ramming_x_head_rot != 0.0 {
            head.pose.rotation[0] = render_state.goat_ramming_x_head_rot;
        }
    }
}
