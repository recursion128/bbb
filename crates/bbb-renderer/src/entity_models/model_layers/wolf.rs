use super::{
    apply_head_look, apply_wolf_sitting_pose, quadruped_leg_swing_pose, wolf_angry_tail_pose,
    wolf_sitting_part_roles, wolf_tail_swing_pose, PartPose, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const WOLF_GRAY: [f32; 4] = [0.64, 0.66, 0.66, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_WOLF: &str = "minecraft:wolf#main";
pub(in crate::entity_models) const MODEL_LAYER_WOLF_BABY: &str = "minecraft:wolf_baby#main";

// Vanilla 26.1 `AdultWolfModel.createBodyLayer(CubeDeformation.NONE)` cubes (atlas 64×32). Each
// unified cube carries the colored tint (`WOLF_GRAY`) and the textured `uv_size`/`texOffs`/`mirror`.
// The right legs reuse the left leg's `texOffs(0, 18)` mirrored.
pub(in crate::entity_models) const ADULT_WOLF_REAL_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-2.0, -3.0, -2.0],
        [6.0, 6.0, 4.0],
        WOLF_GRAY,
        [6.0, 6.0, 4.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -5.0, 0.0],
        [2.0, 2.0, 1.0],
        WOLF_GRAY,
        [2.0, 2.0, 1.0],
        [16.0, 14.0],
        false,
    ),
    ModelCube::new(
        [2.0, -5.0, 0.0],
        [2.0, 2.0, 1.0],
        WOLF_GRAY,
        [2.0, 2.0, 1.0],
        [16.0, 14.0],
        false,
    ),
    ModelCube::new(
        [-0.5, -0.001, -5.0],
        [3.0, 3.0, 4.0],
        WOLF_GRAY,
        [3.0, 3.0, 4.0],
        [0.0, 10.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_WOLF_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -3.0],
    [6.0, 9.0, 6.0],
    WOLF_GRAY,
    [6.0, 9.0, 6.0],
    [18.0, 14.0],
    false,
)];

pub(in crate::entity_models) const ADULT_WOLF_UPPER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, -3.0],
    [8.0, 6.0, 7.0],
    WOLF_GRAY,
    [8.0, 6.0, 7.0],
    [21.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_WOLF_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -1.0],
    [2.0, 8.0, 2.0],
    WOLF_GRAY,
    [2.0, 8.0, 2.0],
    [0.0, 18.0],
    false,
)];

pub(in crate::entity_models) const ADULT_WOLF_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -1.0],
    [2.0, 8.0, 2.0],
    WOLF_GRAY,
    [2.0, 8.0, 2.0],
    [0.0, 18.0],
    true,
)];

pub(in crate::entity_models) const ADULT_WOLF_REAL_TAIL: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -1.0],
    [2.0, 8.0, 2.0],
    WOLF_GRAY,
    [2.0, 8.0, 2.0],
    [9.0, 18.0],
    false,
)];

// Vanilla 26.1 `BabyWolfModel.createBodyLayer` cubes (atlas 32×32). The baby head carries its two
// cubes directly (a slightly inflated skull box + snout) and parents the two ears; its body and four
// legs each have their own `texOffs`, and its tail tip carries a single cube.
pub(in crate::entity_models) const BABY_WOLF_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-3.015, -3.275, -3.025],
        [6.05, 5.05, 5.05],
        WOLF_GRAY,
        [6.0, 5.0, 5.0],
        [0.0, 12.0],
        false,
    ),
    ModelCube::new(
        [-1.5, -0.24, -5.0],
        [3.0, 2.0, 2.0],
        WOLF_GRAY,
        [3.0, 2.0, 2.0],
        [17.0, 12.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_WOLF_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -0.5],
    [2.0, 2.0, 1.0],
    WOLF_GRAY,
    [2.0, 2.0, 1.0],
    [0.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -0.5],
    [2.0, 2.0, 1.0],
    WOLF_GRAY,
    [2.0, 2.0, 1.0],
    [20.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -4.0],
    [6.0, 4.0, 8.0],
    WOLF_GRAY,
    [6.0, 4.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 2.0],
    WOLF_GRAY,
    [2.0, 3.0, 2.0],
    [0.0, 22.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 2.0],
    WOLF_GRAY,
    [2.0, 3.0, 2.0],
    [8.0, 22.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 2.0],
    WOLF_GRAY,
    [2.0, 3.0, 2.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 3.0, 2.0],
    WOLF_GRAY,
    [2.0, 3.0, 2.0],
    [20.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_WOLF_TAIL_R1: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -5.7, -1.0],
    [2.0, 6.0, 2.0],
    WOLF_GRAY,
    [2.0, 6.0, 2.0],
    [22.0, 16.0],
    false,
)];

/// The four leg child names, in `WolfModel.createBodyLayer` order (hind pair then front pair).
/// [`quadruped_leg_swing_pose`] resolves each leg's phase from its bind offset, so the names only need
/// to be distinct and render in the layer's order.
const WOLF_LEG_NAMES: [&str; 4] = [
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

/// Builds the adult wolf tree with the vanilla `AdultWolfModel.createBodyLayer` names: a cubeless
/// `head` parenting `real_head` (skull + ears + snout), `body`, `upper_body` (mane), the four legs,
/// and a cubeless `tail` parenting `real_tail`.
fn adult_wolf_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [-1.0, 13.5, -7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        Vec::new(),
        vec![(
            "real_head",
            ModelPart::leaf(PART_POSE_ZERO, ADULT_WOLF_REAL_HEAD.to_vec()),
        )],
    );
    let tail = ModelPart::new(
        PartPose {
            offset: [-1.0, 12.0, 8.0],
            rotation: [0.62831855, 0.0, 0.0],
        },
        Vec::new(),
        vec![(
            "real_tail",
            ModelPart::leaf(PART_POSE_ZERO, ADULT_WOLF_REAL_TAIL.to_vec()),
        )],
    );
    let legs = [
        ([-2.5, 16.0, 7.0], ADULT_WOLF_RIGHT_LEG.to_vec()),
        ([0.5, 16.0, 7.0], ADULT_WOLF_LEFT_LEG.to_vec()),
        ([-2.5, 16.0, -4.0], ADULT_WOLF_RIGHT_LEG.to_vec()),
        ([0.5, 16.0, -4.0], ADULT_WOLF_LEFT_LEG.to_vec()),
    ];
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(8);
    children.push(("head", head));
    children.push((
        "body",
        ModelPart::leaf(
            PartPose {
                offset: [0.0, 14.0, 2.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
            ADULT_WOLF_BODY.to_vec(),
        ),
    ));
    children.push((
        "upper_body",
        ModelPart::leaf(
            PartPose {
                offset: [-1.0, 14.0, -3.0],
                rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
            },
            ADULT_WOLF_UPPER_BODY.to_vec(),
        ),
    ));
    for (&name, (offset, cubes)) in WOLF_LEG_NAMES.iter().zip(legs) {
        children.push((
            name,
            ModelPart::leaf(
                PartPose {
                    offset,
                    rotation: [0.0, 0.0, 0.0],
                },
                cubes,
            ),
        ));
    }
    children.push(("tail", tail));
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the baby wolf tree with the same names minus the mane: the `head` carries its two cubes and
/// parents the two ears, then `body`, the four legs, and a cubeless `tail` parenting its tip.
fn baby_wolf_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [0.0, 18.25, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        BABY_WOLF_HEAD.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [-2.0, -4.25, -0.5],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_WOLF_RIGHT_EAR.to_vec(),
                ),
            ),
            (
                "left_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [2.0, -4.25, -0.5],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_WOLF_LEFT_EAR.to_vec(),
                ),
            ),
        ],
    );
    let tail = ModelPart::new(
        PartPose {
            offset: [0.0, 19.0, 3.0],
            rotation: [-0.5236, 0.0, 0.0],
        },
        Vec::new(),
        vec![(
            "tail_r1",
            ModelPart::leaf(
                PartPose {
                    offset: [0.0, -0.6, 0.2],
                    rotation: [-3.1, 0.0, 0.0],
                },
                BABY_WOLF_TAIL_R1.to_vec(),
            ),
        )],
    );
    let legs = [
        ([-1.5, 21.0, 3.0], BABY_WOLF_RIGHT_HIND_LEG.to_vec()),
        ([1.5, 21.0, 3.0], BABY_WOLF_LEFT_HIND_LEG.to_vec()),
        ([-1.5, 21.0, -3.0], BABY_WOLF_RIGHT_FRONT_LEG.to_vec()),
        ([1.5, 21.0, -3.0], BABY_WOLF_LEFT_FRONT_LEG.to_vec()),
    ];
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(7);
    children.push(("head", head));
    children.push((
        "body",
        ModelPart::leaf(
            PartPose {
                offset: [0.0, 19.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
            },
            BABY_WOLF_BODY.to_vec(),
        ),
    ));
    for (&name, (offset, cubes)) in WOLF_LEG_NAMES.iter().zip(legs) {
        children.push((
            name,
            ModelPart::leaf(
                PartPose {
                    offset,
                    rotation: [0.0, 0.0, 0.0],
                },
                cubes,
            ),
        ));
    }
    children.push(("tail", tail));
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable wolf model, mirroring vanilla `WolfModel` / `BabyWolfModel`. The unified tree is built once
/// with named children selected by `baby`: the `head` (carrying the ears/real head), `body`,
/// `upper_body` mane (adult only), the four legs, and the `tail` (carrying its tip). `setup_anim` runs
/// the shared `WolfModel.setupAnim`: the head follows the look, then either the `setSittingPose` fold
/// (body tilt + leg tuck + tail lift) or the `QuadrupedModel` diagonal leg swing, then the tail `xRot`
/// (`tailAngle`) + wag `yRot` (angry → the raised constant). The collar dye overlay is a second
/// textured pass on the same posed tree; the water-shake roll is deferred.
pub(in crate::entity_models) struct WolfModel {
    root: ModelPart,
    baby: bool,
    angry: bool,
}

impl WolfModel {
    pub(in crate::entity_models) fn new(baby: bool, angry: bool) -> Self {
        Self {
            root: if baby {
                baby_wolf_tree()
            } else {
                adult_wolf_tree()
            },
            baby,
            angry,
        }
    }
}

impl EntityModel for WolfModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;

        // The head carries the ears, so the look turns the whole head subtree.
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );

        // A sitting wolf folds (`setSittingPose`) instead of swinging its legs; the tail rotation below
        // then layers onto the sitting offset lift (both helpers preserve the offset). The baby's
        // `setSittingPose` adds a further body tilt.
        let baby = self.baby;
        if render_state.wolf_sitting {
            for (name, role) in wolf_sitting_part_roles() {
                apply_wolf_sitting_pose(&mut self.root.child_mut(name).pose, role, baby);
            }
        } else {
            for name in WOLF_LEG_NAMES {
                let leg = self.root.child_mut(name);
                leg.pose = quadruped_leg_swing_pose(
                    leg.pose,
                    render_state.walk_animation_pos,
                    render_state.walk_animation_speed,
                );
            }
        }

        // The tail `xRot` is SET to `tailAngle` (wag `yRot` on top) in both poses; an angry wolf holds
        // the raised constant with no wag.
        let tail = self.root.child_mut("tail");
        tail.pose = if self.angry {
            wolf_angry_tail_pose(tail.pose)
        } else {
            wolf_tail_swing_pose(
                tail.pose,
                render_state.wolf_tail_angle,
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
            )
        };
    }
}
