use super::{
    head_look_at_rest, head_look_pose, limb_swing_at_rest, ravager_leg_swing_pose, PartPose,
    PART_POSE_ZERO, RAVAGER_GRAY,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_RAVAGER: &str = "minecraft:ravager#main";

// Vanilla 26.1 `RavagerModel.createBodyLayer` cubes (atlas 128×128, `CubeDeformation.NONE`). Each
// unified cube carries the colored tint (`RAVAGER_GRAY`) and the textured `uv_size`/`texOffs`/`mirror`
// in one struct. The left horn/legs reuse their right counterpart's `texOffs` mirrored.
pub(in crate::entity_models) const RAVAGER_NECK: [ModelCube; 1] = [ModelCube::new(
    [-5.0, -1.0, -18.0],
    [10.0, 10.0, 18.0],
    RAVAGER_GRAY,
    [10.0, 10.0, 18.0],
    [68.0, 73.0],
    false,
)];

pub(in crate::entity_models) const RAVAGER_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-8.0, -20.0, -14.0],
        [16.0, 20.0, 16.0],
        RAVAGER_GRAY,
        [16.0, 20.0, 16.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -6.0, -18.0],
        [4.0, 8.0, 4.0],
        RAVAGER_GRAY,
        [4.0, 8.0, 4.0],
        [0.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const RAVAGER_RIGHT_HORN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -14.0, -2.0],
    [2.0, 14.0, 4.0],
    RAVAGER_GRAY,
    [2.0, 14.0, 4.0],
    [74.0, 55.0],
    false,
)];

pub(in crate::entity_models) const RAVAGER_LEFT_HORN: [ModelCube; 1] = [ModelCube::new(
    [0.0, -14.0, -2.0],
    [2.0, 14.0, 4.0],
    RAVAGER_GRAY,
    [2.0, 14.0, 4.0],
    [74.0, 55.0],
    true,
)];

pub(in crate::entity_models) const RAVAGER_MOUTH: [ModelCube; 1] = [ModelCube::new(
    [-8.0, 0.0, -16.0],
    [16.0, 3.0, 16.0],
    RAVAGER_GRAY,
    [16.0, 3.0, 16.0],
    [0.0, 36.0],
    false,
)];

pub(in crate::entity_models) const RAVAGER_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-7.0, -10.0, -7.0],
        [14.0, 16.0, 20.0],
        RAVAGER_GRAY,
        [14.0, 16.0, 20.0],
        [0.0, 55.0],
        false,
    ),
    ModelCube::new(
        [-6.0, 6.0, -7.0],
        [12.0, 13.0, 18.0],
        RAVAGER_GRAY,
        [12.0, 13.0, 18.0],
        [0.0, 91.0],
        false,
    ),
];

pub(in crate::entity_models) const RAVAGER_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 37.0, 8.0],
    RAVAGER_GRAY,
    [8.0, 37.0, 8.0],
    [96.0, 0.0],
    false,
)];

pub(in crate::entity_models) const RAVAGER_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 37.0, 8.0],
    RAVAGER_GRAY,
    [8.0, 37.0, 8.0],
    [96.0, 0.0],
    true,
)];

pub(in crate::entity_models) const RAVAGER_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 37.0, 8.0],
    RAVAGER_GRAY,
    [8.0, 37.0, 8.0],
    [64.0, 0.0],
    false,
)];

pub(in crate::entity_models) const RAVAGER_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -4.0],
    [8.0, 37.0, 8.0],
    RAVAGER_GRAY,
    [8.0, 37.0, 8.0],
    [64.0, 0.0],
    true,
)];

/// The four leg child names in `RavagerModel.createBodyLayer` order: the two hind legs then the two
/// front legs. [`ravager_leg_swing_pose`] resolves each leg's phase from its bind offset, so the
/// names only need to be distinct and render in the layer's order.
const RAVAGER_LEG_NAMES: [&str; 4] = [
    "right_hind_leg",
    "left_hind_leg",
    "right_front_leg",
    "left_front_leg",
];

/// Builds the ravager tree with the vanilla `RavagerModel.createBodyLayer` names: `neck` (parenting
/// `head` → `right_horn`/`left_horn`/`mouth`), `body`, and the four legs. The head is nested under
/// the neck, so a head look re-poses the whole head subtree (horns + mouth) automatically.
fn ravager_tree() -> ModelPart {
    let head = ModelPart::new(
        PartPose {
            offset: [0.0, 16.0, -17.0],
            rotation: [0.0, 0.0, 0.0],
        },
        RAVAGER_HEAD.to_vec(),
        vec![
            (
                "right_horn",
                ModelPart::leaf(
                    PartPose {
                        offset: [-10.0, -14.0, -8.0],
                        rotation: [1.0995574, 0.0, 0.0],
                    },
                    RAVAGER_RIGHT_HORN.to_vec(),
                ),
            ),
            (
                "left_horn",
                ModelPart::leaf(
                    PartPose {
                        offset: [8.0, -14.0, -8.0],
                        rotation: [1.0995574, 0.0, 0.0],
                    },
                    RAVAGER_LEFT_HORN.to_vec(),
                ),
            ),
            (
                "mouth",
                ModelPart::leaf(
                    PartPose {
                        offset: [0.0, -2.0, 2.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    RAVAGER_MOUTH.to_vec(),
                ),
            ),
        ],
    );
    let neck = ModelPart::new(
        PartPose {
            offset: [0.0, -7.0, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        RAVAGER_NECK.to_vec(),
        vec![("head", head)],
    );
    let body = ModelPart::leaf(
        PartPose {
            offset: [0.0, 1.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        RAVAGER_BODY.to_vec(),
    );
    let legs = [
        ([-8.0, -13.0, 18.0], RAVAGER_RIGHT_HIND_LEG.to_vec()),
        ([8.0, -13.0, 18.0], RAVAGER_LEFT_HIND_LEG.to_vec()),
        ([-8.0, -13.0, -5.0], RAVAGER_RIGHT_FRONT_LEG.to_vec()),
        ([8.0, -13.0, -5.0], RAVAGER_LEFT_FRONT_LEG.to_vec()),
    ];
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(6);
    children.push(("neck", neck));
    children.push(("body", body));
    for (&name, (offset, cubes)) in RAVAGER_LEG_NAMES.iter().zip(legs) {
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

/// Mutable ravager model, mirroring vanilla `RavagerModel`. The unified tree is built once with the
/// vanilla `RavagerModel.createBodyLayer` child names: `neck` (parenting `head` → horns/mouth),
/// `body`, and the four legs. `setup_anim` swings the four legs ([`ravager_leg_swing_pose`]) and
/// looks the head — which, being nested under the neck, is reached as `neck.child_mut("head")` so its
/// horn/mouth descendants inherit the look automatically. The neck/mouth attack/stun/roar poses defer.
pub(in crate::entity_models) struct RavagerModel {
    root: ModelPart,
}

impl RavagerModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ravager_tree(),
        }
    }
}

impl EntityModel for RavagerModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        if !limb_swing_at_rest(render_state.walk_animation_speed) {
            for name in RAVAGER_LEG_NAMES {
                let leg = self.root.child_mut(name);
                leg.pose = ravager_leg_swing_pose(
                    leg.pose,
                    render_state.walk_animation_pos,
                    render_state.walk_animation_speed,
                );
            }
        }
        if !head_look_at_rest(render_state.head_yaw, render_state.head_pitch) {
            let head = self.root.child_mut("neck").child_mut("head");
            head.pose = head_look_pose(head.pose, render_state.head_yaw, render_state.head_pitch);
        }
    }
}
