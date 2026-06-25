use super::{
    head_look_yaw_pose, hoglin_ear_sway_pose, hoglin_leg_swing_pose, PartPose, HOGLIN_RED,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_HOGLIN: &str = "minecraft:hoglin#main";
pub(in crate::entity_models) const MODEL_LAYER_HOGLIN_BABY: &str = "minecraft:hoglin_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOGLIN: &str = "minecraft:zoglin#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOGLIN_BABY: &str = "minecraft:zoglin_baby#main";

pub(in crate::entity_models) const HOGLIN_HEAD_X_ROT: f32 = 0.87266463;
pub(in crate::entity_models) const HOGLIN_EAR_Z_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
pub(in crate::entity_models) const BABY_HOGLIN_HEAD_X_ROT: f32 = 0.8727;
pub(in crate::entity_models) const BABY_HOGLIN_EAR_Z_ROT: f32 = 0.8727;

// Vanilla 26.1 `HoglinModel.createBodyLayer` cubes (hoglin atlas 128×64). Each unified cube carries
// the colored tint (`HOGLIN_RED`) and the textured `uv_size`/`texOffs`/`mirror` in one struct. The
// mane keeps its inflated colored geometry against the base textured `uv_size` (the squid precedent).
pub(in crate::entity_models) const ADULT_HOGLIN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -7.0, -13.0],
    [16.0, 14.0, 26.0],
    HOGLIN_RED,
    [16.0, 14.0, 26.0],
    [1.0, 1.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_MANE: [ModelCube; 1] = [ModelCube::new(
    [-0.001, -0.001, -9.001],
    [0.002, 10.002, 19.002],
    HOGLIN_RED,
    [0.0, 10.0, 19.0],
    [90.0, 33.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-7.0, -3.0, -19.0],
    [14.0, 6.0, 19.0],
    HOGLIN_RED,
    [14.0, 6.0, 19.0],
    [61.0, 1.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-6.0, -1.0, -2.0],
    [6.0, 1.0, 4.0],
    HOGLIN_RED,
    [6.0, 1.0, 4.0],
    [1.0, 1.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, -2.0],
    [6.0, 1.0, 4.0],
    HOGLIN_RED,
    [6.0, 1.0, 4.0],
    [1.0, 6.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -11.0, -1.0],
    [2.0, 11.0, 2.0],
    HOGLIN_RED,
    [2.0, 11.0, 2.0],
    [10.0, 13.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_HORN: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -11.0, -1.0],
    [2.0, 11.0, 2.0],
    HOGLIN_RED,
    [2.0, 11.0, 2.0],
    [1.0, 13.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, -3.0],
    [6.0, 14.0, 6.0],
    HOGLIN_RED,
    [6.0, 14.0, 6.0],
    [66.0, 42.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, -3.0],
    [6.0, 14.0, 6.0],
    HOGLIN_RED,
    [6.0, 14.0, 6.0],
    [41.0, 42.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.5, 0.0, -2.5],
    [5.0, 11.0, 5.0],
    HOGLIN_RED,
    [5.0, 11.0, 5.0],
    [21.0, 45.0],
    false,
)];

pub(in crate::entity_models) const ADULT_HOGLIN_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.5, 0.0, -2.5],
    [5.0, 11.0, 5.0],
    HOGLIN_RED,
    [5.0, 11.0, 5.0],
    [0.0, 45.0],
    false,
)];

// Vanilla 26.1 `BabyHoglinModel.createBodyLayer` cubes (baby atlas 64×64). The head and body each
// carry several cubes; the body cubes are inflated (the colored geometry keeps the inflated box
// against the base textured `uv_size`).
pub(in crate::entity_models) const BABY_HOGLIN_HEAD: [ModelCube; 3] = [
    ModelCube::new(
        [-5.0, -2.2605, -10.547],
        [10.0, 4.0, 12.0],
        HOGLIN_RED,
        [10.0, 4.0, 12.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-7.0, -4.0981, -8.4879],
        [2.0, 5.0, 2.0],
        HOGLIN_RED,
        [2.0, 5.0, 2.0],
        [44.0, 29.0],
        false,
    ),
    ModelCube::new(
        [5.0, -4.0981, -8.4879],
        [2.0, 5.0, 2.0],
        HOGLIN_RED,
        [2.0, 5.0, 2.0],
        [52.0, 29.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_HOGLIN_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-4.02, -14.02, -7.02],
        [8.04, 8.04, 14.04],
        HOGLIN_RED,
        [8.0, 8.0, 14.0],
        [0.0, 16.0],
        false,
    ),
    ModelCube::new(
        [-0.02, -18.02, -8.02],
        [0.04, 6.04, 11.04],
        HOGLIN_RED,
        [0.0, 6.0, 11.0],
        [24.0, 39.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_HOGLIN_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-5.1, -0.5, -2.0],
    [6.0, 1.0, 4.0],
    HOGLIN_RED,
    [6.0, 1.0, 4.0],
    [32.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BABY_HOGLIN_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-0.9, -0.5, -2.0],
    [6.0, 1.0, 4.0],
    HOGLIN_RED,
    [6.0, 1.0, 4.0],
    [32.0, 0.0],
    true,
)];

pub(in crate::entity_models) const BABY_HOGLIN_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    HOGLIN_RED,
    [3.0, 6.0, 3.0],
    [0.0, 47.0],
    false,
)];

pub(in crate::entity_models) const BABY_HOGLIN_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    HOGLIN_RED,
    [3.0, 6.0, 3.0],
    [12.0, 47.0],
    false,
)];

pub(in crate::entity_models) const BABY_HOGLIN_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    HOGLIN_RED,
    [3.0, 6.0, 3.0],
    [0.0, 38.0],
    false,
)];

pub(in crate::entity_models) const BABY_HOGLIN_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 6.0, 3.0],
    HOGLIN_RED,
    [3.0, 6.0, 3.0],
    [12.0, 38.0],
    false,
)];

/// The four leg child names, shared by both layers. [`hoglin_leg_swing_pose`] resolves each leg's
/// phase from its bind offset, so the differing front/hind order of the adult and baby layers does
/// not matter — the names only need to be distinct and render in the layer's order.
const HOGLIN_LEG_NAMES: [&str; 4] = [
    "right_front_leg",
    "left_front_leg",
    "right_hind_leg",
    "left_hind_leg",
];

/// Builds the adult hoglin tree with the vanilla `HoglinModel.createBodyLayer` names: `body`
/// (parenting `mane`), `head` (parenting `right_ear`/`left_ear`/`right_horn`/`left_horn`), and the
/// four legs. The adult layer lists the body first, the head second.
fn adult_hoglin_tree() -> ModelPart {
    let body = ModelPart::new(
        PartPose {
            offset: [0.0, 7.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        ADULT_HOGLIN_BODY.to_vec(),
        vec![(
            "mane",
            ModelPart::leaf(
                PartPose {
                    offset: [0.0, -14.0, -7.0],
                    rotation: [0.0, 0.0, 0.0],
                },
                ADULT_HOGLIN_MANE.to_vec(),
            ),
        )],
    );
    let head = ModelPart::new(
        PartPose {
            offset: [0.0, 2.0, -12.0],
            rotation: [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        ADULT_HOGLIN_HEAD.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [-6.0, -2.0, -3.0],
                        rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
                    },
                    ADULT_HOGLIN_RIGHT_EAR.to_vec(),
                ),
            ),
            (
                "left_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [6.0, -2.0, -3.0],
                        rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
                    },
                    ADULT_HOGLIN_LEFT_EAR.to_vec(),
                ),
            ),
            (
                "right_horn",
                ModelPart::leaf(
                    PartPose {
                        offset: [-7.0, 2.0, -12.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    ADULT_HOGLIN_RIGHT_HORN.to_vec(),
                ),
            ),
            (
                "left_horn",
                ModelPart::leaf(
                    PartPose {
                        offset: [7.0, 2.0, -12.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    ADULT_HOGLIN_LEFT_HORN.to_vec(),
                ),
            ),
        ],
    );
    let legs = [
        ([-4.0, 10.0, -8.5], ADULT_HOGLIN_RIGHT_FRONT_LEG.to_vec()),
        ([4.0, 10.0, -8.5], ADULT_HOGLIN_LEFT_FRONT_LEG.to_vec()),
        ([-5.0, 13.0, 10.0], ADULT_HOGLIN_RIGHT_HIND_LEG.to_vec()),
        ([5.0, 13.0, 10.0], ADULT_HOGLIN_LEFT_HIND_LEG.to_vec()),
    ];
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(6);
    children.push(("body", body));
    children.push(("head", head));
    for (&name, (offset, cubes)) in HOGLIN_LEG_NAMES.iter().zip(legs) {
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
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the baby hoglin tree with the same names; the baby layer lists the `head` first, the
/// `body` second, and its head parents only the two ears (no horns). The legs follow.
fn baby_hoglin_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [0.0, 13.0, -7.0],
            rotation: [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        },
        BABY_HOGLIN_HEAD.to_vec(),
        vec![
            (
                "right_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [-5.0, -1.0, -1.5],
                        rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
                    },
                    BABY_HOGLIN_RIGHT_EAR.to_vec(),
                ),
            ),
            (
                "left_ear",
                ModelPart::leaf(
                    PartPose {
                        offset: [5.0, -1.0, -1.5],
                        rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
                    },
                    BABY_HOGLIN_LEFT_EAR.to_vec(),
                ),
            ),
        ],
    );
    let body = ModelPart::leaf(
        PartPose {
            offset: [0.0, 24.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        BABY_HOGLIN_BODY.to_vec(),
    );
    let legs = [
        ([-2.5, 18.0, 4.5], BABY_HOGLIN_RIGHT_HIND_LEG.to_vec()),
        ([2.5, 18.0, 4.5], BABY_HOGLIN_LEFT_HIND_LEG.to_vec()),
        ([-2.5, 18.0, -4.5], BABY_HOGLIN_RIGHT_FRONT_LEG.to_vec()),
        ([2.5, 18.0, -4.5], BABY_HOGLIN_LEFT_FRONT_LEG.to_vec()),
    ];
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(6);
    children.push(("head", head));
    children.push(("body", body));
    for (&name, (offset, cubes)) in HOGLIN_LEG_NAMES.iter().zip(legs) {
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
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable hoglin model, mirroring vanilla `HoglinModel` (shared by the zoglin). The unified tree is
/// built once with the vanilla `HoglinModel.createBodyLayer` child names, selected by `baby`.
/// `setup_anim` runs the yaw-only head look ([`head_look_yaw_pose`]), sways the two ears (head
/// children `right_ear`/`left_ear` via [`hoglin_ear_sway_pose`], whose `±2π/9` rest also overrides
/// the baby layer's wider baked angle), and swings the four legs ([`hoglin_leg_swing_pose`]). The
/// family recolor/texture and root scale are supplied by the caller; the headbutt head ram is applied
/// on top ([`apply_hoglin_headbutt`]) from the projected attack timer.
pub(in crate::entity_models) struct HoglinModel {
    root: ModelPart,
    baby: bool,
}

impl HoglinModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: if baby {
                baby_hoglin_tree()
            } else {
                adult_hoglin_tree()
            },
            baby,
        }
    }
}

/// Vanilla `HoglinModel.animateHeadbutt` (and `BabyHoglinModel`'s override): the head SET to
/// `lerp(headbuttLerpFactor, 0.87266463, -π/9)`, where `headbuttLerpFactor = 1 - |10 - 2·tick| / 10`
/// ramps `0 → 1 → 0` across the 10-tick attack (peaking at `tick = 5`). At rest (`tick = 0`) the factor
/// is `0`, so the head holds its baked down-tilt `0.87266463`; mid-ram it rises to `-π/9`. The baby
/// additionally lifts `head.y += factor·2.5`. Vanilla SETs the head pitch absolutely, so this supersedes
/// the baked rest tilt and the yaw-only look's preserved `xRot`.
fn apply_hoglin_headbutt(head: &mut ModelPart, attack_animation_tick: i32, baby: bool) {
    use std::f32::consts::PI;
    let factor = 1.0 - (10 - 2 * attack_animation_tick).abs() as f32 / 10.0;
    head.pose.rotation[0] = HOGLIN_HEAD_X_ROT + factor * (-PI / 9.0 - HOGLIN_HEAD_X_ROT);
    if baby {
        head.pose.offset[1] += factor * 2.5;
    }
}

impl EntityModel for HoglinModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        let baby = self.baby;
        // Yaw-only head look, the headbutt head ram (always applied — at rest it re-sets the baked down
        // tilt), then sway the two ears (head children). Vanilla overrides the baked ear rest angle to
        // `±2π/9` every frame, so the sway is applied unconditionally (a no-op for the adult layer at
        // rest, an override for the baby layer's wider baked angle).
        let head = self.root.child_mut("head");
        head.pose = head_look_yaw_pose(head.pose, render_state.head_yaw);
        apply_hoglin_headbutt(head, render_state.hoglin_attack_animation_tick, baby);
        let right_ear = head.child_mut("right_ear");
        right_ear.pose = hoglin_ear_sway_pose(right_ear.pose, false, limb_swing, limb_swing_amount);
        let left_ear = head.child_mut("left_ear");
        left_ear.pose = hoglin_ear_sway_pose(left_ear.pose, true, limb_swing, limb_swing_amount);
        for name in HOGLIN_LEG_NAMES {
            let leg = self.root.child_mut(name);
            leg.pose = hoglin_leg_swing_pose(leg.pose, limb_swing, limb_swing_amount);
        }
    }
}
