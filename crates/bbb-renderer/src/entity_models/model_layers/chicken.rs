use super::{
    humanoid_leg_swing_pose, limb_swing_at_rest, PartPose, CHICKEN_BEAK, CHICKEN_LEG, CHICKEN_RED,
    CHICKEN_WHITE, CHICKEN_WING, PART_POSE_ZERO,
};
use crate::entity_models::catalog::ChickenModelVariant;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use std::f32::consts::FRAC_PI_2;

// Vanilla 26.1 ChickenModel / ColdChickenModel / BabyChickenModel layers (atlas 64x32, babies 16x16).
// Each unified cube carries both render paths' data: the colored debug tint and the textured
// `uv_size` / `texOffs` / `mirror`. No `CubeDeformation`, so `uv_size == size` and no cube mirrors.

// --- Adult / cold head + body cubes ---
pub(in crate::entity_models) const ADULT_CHICKEN_BEAK: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -4.0, -4.0],
    [4.0, 2.0, 2.0],
    CHICKEN_BEAK,
    [4.0, 2.0, 2.0],
    [14.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_CHICKEN_RED_THING: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -3.0],
    [2.0, 2.0, 2.0],
    CHICKEN_RED,
    [2.0, 2.0, 2.0],
    [14.0, 4.0],
    false,
)];

pub(in crate::entity_models) const ADULT_CHICKEN_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -6.0, -2.0],
    [4.0, 6.0, 3.0],
    CHICKEN_WHITE,
    [4.0, 6.0, 3.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_CHICKEN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -4.0, -3.0],
    [6.0, 8.0, 6.0],
    CHICKEN_WHITE,
    [6.0, 8.0, 6.0],
    [0.0, 9.0],
    false,
)];

pub(in crate::entity_models) const COLD_CHICKEN_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-2.0, -6.0, -2.0],
        [4.0, 6.0, 3.0],
        CHICKEN_WHITE,
        [4.0, 6.0, 3.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-3.0, -7.0, -2.015],
        [6.0, 3.0, 4.0],
        CHICKEN_WING,
        [6.0, 3.0, 4.0],
        [44.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const COLD_CHICKEN_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-3.0, -4.0, -3.0],
        [6.0, 8.0, 6.0],
        CHICKEN_WHITE,
        [6.0, 8.0, 6.0],
        [0.0, 9.0],
        false,
    ),
    ModelCube::new(
        [0.0, 3.0, -1.0],
        [0.0, 3.0, 5.0],
        CHICKEN_WING,
        [0.0, 3.0, 5.0],
        [38.0, 9.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_CHICKEN_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -3.0],
    [3.0, 5.0, 3.0],
    CHICKEN_LEG,
    [3.0, 5.0, 3.0],
    [26.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_CHICKEN_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -3.0],
    [1.0, 4.0, 6.0],
    CHICKEN_WING,
    [1.0, 4.0, 6.0],
    [24.0, 13.0],
    false,
)];

pub(in crate::entity_models) const ADULT_CHICKEN_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -3.0],
    [1.0, 4.0, 6.0],
    CHICKEN_WING,
    [1.0, 4.0, 6.0],
    [24.0, 13.0],
    false,
)];

// --- Baby cubes (the beak is baked into the body; the legs/wings are flat planes) ---
pub(in crate::entity_models) const BABY_CHICKEN_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-2.0, -2.25, -0.75],
        [4.0, 4.0, 4.0],
        CHICKEN_WHITE,
        [4.0, 4.0, 4.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -0.25, -1.75],
        [2.0, 1.0, 1.0],
        CHICKEN_BEAK,
        [2.0, 1.0, 1.0],
        [10.0, 8.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_CHICKEN_LEFT_LEG: [ModelCube; 2] = [
    ModelCube::new(
        [-0.5, 0.0, 0.0],
        [1.0, 2.0, 0.0],
        CHICKEN_LEG,
        [1.0, 2.0, 0.0],
        [2.0, 2.0],
        false,
    ),
    ModelCube::new(
        [-0.5, 2.0, -1.0],
        [1.0, 0.0, 1.0],
        CHICKEN_LEG,
        [1.0, 0.0, 1.0],
        [0.0, 1.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_LEG: [ModelCube; 2] = [
    ModelCube::new(
        [-0.5, 0.0, 0.0],
        [1.0, 2.0, 0.0],
        CHICKEN_LEG,
        [1.0, 2.0, 0.0],
        [0.0, 2.0],
        false,
    ),
    ModelCube::new(
        [-0.5, 2.0, -1.0],
        [1.0, 0.0, 1.0],
        CHICKEN_LEG,
        [1.0, 0.0, 1.0],
        [0.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -1.0],
    [1.0, 0.0, 2.0],
    CHICKEN_WING,
    [1.0, 0.0, 2.0],
    [6.0, 8.0],
    false,
)];

pub(in crate::entity_models) const BABY_CHICKEN_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [1.0, 0.0, 2.0],
    CHICKEN_WING,
    [1.0, 0.0, 2.0],
    [4.0, 8.0],
    false,
)];

// --- Part poses (shared adult/cold layout; the baby uses its own squat layout) ---
pub(in crate::entity_models) const CHICKEN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CHICKEN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, 0.0],
    rotation: [FRAC_PI_2, 0.0, 0.0],
};
pub(in crate::entity_models) const CHICKEN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 19.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CHICKEN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 19.0, 1.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CHICKEN_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-4.0, 13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const CHICKEN_LEFT_WING_POSE: PartPose = PartPose {
    offset: [4.0, 13.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_CHICKEN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 20.25, -1.25],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_CHICKEN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 22.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 22.0, 0.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_CHICKEN_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [2.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_CHICKEN_LEFT_WING_POSE: PartPose = PartPose {
    offset: [-2.0, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const MODEL_LAYER_CHICKEN: &str = "minecraft:chicken#main";
pub(in crate::entity_models) const MODEL_LAYER_CHICKEN_BABY: &str = "minecraft:chicken_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_COLD_CHICKEN: &str = "minecraft:cold_chicken#main";

/// Builds the adult / cold chicken head with its `beak` and `red_thing` children (the cold head carries
/// an extra fluff cube in its own cube list, but the child layout is identical).
fn chicken_head(head_cubes: Vec<ModelCube>) -> ModelPart {
    ModelPart::new(
        CHICKEN_HEAD_POSE,
        head_cubes,
        vec![
            (
                "beak",
                ModelPart::leaf(PART_POSE_ZERO, ADULT_CHICKEN_BEAK.to_vec()),
            ),
            (
                "red_thing",
                ModelPart::leaf(PART_POSE_ZERO, ADULT_CHICKEN_RED_THING.to_vec()),
            ),
        ],
    )
}

/// Builds the adult / cold chicken tree (vanilla `ChickenModel.createBodyLayer` names: head with
/// beak + red_thing, body, right_leg, left_leg, right_wing, left_wing). The cold variant only swaps the
/// head/body cube lists for their fluffed counterparts.
fn adult_chicken_tree(head_cubes: Vec<ModelCube>, body_cubes: Vec<ModelCube>) -> ModelPart {
    let children: Vec<(&'static str, ModelPart)> = vec![
        ("head", chicken_head(head_cubes)),
        ("body", ModelPart::leaf(CHICKEN_BODY_POSE, body_cubes)),
        (
            "right_leg",
            ModelPart::leaf(CHICKEN_RIGHT_LEG_POSE, ADULT_CHICKEN_LEG.to_vec()),
        ),
        (
            "left_leg",
            ModelPart::leaf(CHICKEN_LEFT_LEG_POSE, ADULT_CHICKEN_LEG.to_vec()),
        ),
        (
            "right_wing",
            ModelPart::leaf(CHICKEN_RIGHT_WING_POSE, ADULT_CHICKEN_RIGHT_WING.to_vec()),
        ),
        (
            "left_wing",
            ModelPart::leaf(CHICKEN_LEFT_WING_POSE, ADULT_CHICKEN_LEFT_WING.to_vec()),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the headless baby chicken tree (vanilla `BabyChickenModel.createBodyLayer` names: body with
/// the baked beak, right_leg, left_leg, right_wing, left_wing).
fn baby_chicken_tree() -> ModelPart {
    let children: Vec<(&'static str, ModelPart)> = vec![
        (
            "body",
            ModelPart::leaf(BABY_CHICKEN_BODY_POSE, BABY_CHICKEN_BODY.to_vec()),
        ),
        (
            "left_leg",
            ModelPart::leaf(BABY_CHICKEN_LEFT_LEG_POSE, BABY_CHICKEN_LEFT_LEG.to_vec()),
        ),
        (
            "right_leg",
            ModelPart::leaf(BABY_CHICKEN_RIGHT_LEG_POSE, BABY_CHICKEN_RIGHT_LEG.to_vec()),
        ),
        (
            "right_wing",
            ModelPart::leaf(
                BABY_CHICKEN_RIGHT_WING_POSE,
                BABY_CHICKEN_RIGHT_WING.to_vec(),
            ),
        ),
        (
            "left_wing",
            ModelPart::leaf(BABY_CHICKEN_LEFT_WING_POSE, BABY_CHICKEN_LEFT_WING.to_vec()),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the unified chicken tree for `variant`/`baby`, mirroring the vanilla layer choice (cold
/// chickens carry their fluff layer; babies use the squat headless layout).
fn chicken_tree(variant: ChickenModelVariant, baby: bool) -> ModelPart {
    match (variant, baby) {
        (_, true) => baby_chicken_tree(),
        (ChickenModelVariant::Cold, false) => {
            adult_chicken_tree(COLD_CHICKEN_HEAD.to_vec(), COLD_CHICKEN_BODY.to_vec())
        }
        (_, false) => adult_chicken_tree(ADULT_CHICKEN_HEAD.to_vec(), ADULT_CHICKEN_BODY.to_vec()),
    }
}

/// Mutable chicken model, mirroring vanilla `ChickenModel`. The unified tree is built once for the
/// selected `variant`/`baby` layout ([`chicken_tree`]). `setup_anim` swings the `left_leg`/`right_leg`
/// with the `HumanoidModel` phase ([`humanoid_leg_swing_pose`]) and flaps the `right_wing`/`left_wing`
/// with the `flap`/`flapSpeed` phase (vanilla `flapAngle = (sin(flap) + 1) * flapSpeed`). The chicken
/// has no head look.
pub(in crate::entity_models) struct ChickenModel {
    root: ModelPart,
}

impl ChickenModel {
    pub(in crate::entity_models) fn new(variant: ChickenModelVariant, baby: bool) -> Self {
        Self {
            root: chicken_tree(variant, baby),
        }
    }
}

impl EntityModel for ChickenModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        // Vanilla `ChickenModel.setupAnim`: `flapAngle = (sin(flap) + 1) * flapSpeed`,
        // applied as `rightWing.zRot = flapAngle` / `leftWing.zRot = -flapAngle`. The
        // adult, cold, and headless-baby layers all carry the named wings. With
        // `flapSpeed == 0` (a grounded/still chicken) the angle is `0`, so the wings
        // hold the bind pose.
        let flap_angle = (render_state.chicken_flap.sin() + 1.0) * render_state.chicken_flap_speed;
        self.root.child_mut("right_wing").pose.rotation[2] = flap_angle;
        self.root.child_mut("left_wing").pose.rotation[2] = -flap_angle;
        // `humanoid_leg_swing_pose` resolves each leg's phase from its `x` sign, so the named legs work
        // for the adult, cold, and headless-baby layouts alike. A standing chicken leaves the legs at
        // their bind pose.
        if !limb_swing_at_rest(render_state.walk_animation_speed) {
            for name in ["right_leg", "left_leg"] {
                let leg = self.root.child_mut(name);
                leg.pose = humanoid_leg_swing_pose(
                    leg.pose,
                    render_state.walk_animation_pos,
                    render_state.walk_animation_speed,
                );
            }
        }
    }
}
