use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};

use super::{
    apply_head_look, spider_leg_swing_pose, spider_leg_swing_roles, PartPose, PART_POSE_ZERO,
    SPIDER_DARK,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_SPIDER: &str = "minecraft:spider#main";
pub(in crate::entity_models) const MODEL_LAYER_CAVE_SPIDER: &str = "minecraft:cave_spider#main";

// Vanilla 26.1 `SpiderModel.createSpiderBodyLayer` cubes (atlas 64×32). Each unified cube carries the
// colored tint (`SPIDER_DARK`) and the textured `uv_size`/`texOffs`/`mirror`. The left legs reuse the
// right leg's `texOffs(18, 0)` mirrored.
pub(in crate::entity_models) const SPIDER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -4.0, -8.0],
    [8.0, 8.0, 8.0],
    SPIDER_DARK,
    [8.0, 8.0, 8.0],
    [32.0, 4.0],
    false,
)];

pub(in crate::entity_models) const SPIDER_BODY_0: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, -3.0],
    [6.0, 6.0, 6.0],
    SPIDER_DARK,
    [6.0, 6.0, 6.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SPIDER_BODY_1: [ModelCube; 1] = [ModelCube::new(
    [-5.0, -4.0, -6.0],
    [10.0, 8.0, 12.0],
    SPIDER_DARK,
    [10.0, 8.0, 12.0],
    [0.0, 12.0],
    false,
)];

pub(in crate::entity_models) const SPIDER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-15.0, -1.0, -1.0],
    [16.0, 2.0, 2.0],
    SPIDER_DARK,
    [16.0, 2.0, 2.0],
    [18.0, 0.0],
    false,
)];

pub(in crate::entity_models) const SPIDER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -1.0],
    [16.0, 2.0, 2.0],
    SPIDER_DARK,
    [16.0, 2.0, 2.0],
    [18.0, 0.0],
    true,
)];

/// The eight spider leg child names with their bind poses, in `SpiderModel.createSpiderBodyLayer`
/// order (right/left pairs from hind to front). Each carries the body-layer splay (yaw + roll) that
/// the swing accumulates onto. The right legs use [`SPIDER_RIGHT_LEG`], the left [`SPIDER_LEFT_LEG`].
const SPIDER_LEG_SPLAY: f32 = 0.58119464;
fn spider_legs() -> [(&'static str, PartPose, bool); 8] {
    [
        (
            "right_hind_leg",
            PartPose {
                offset: [-4.0, 15.0, 2.0],
                rotation: [0.0, FRAC_PI_4, -FRAC_PI_4],
            },
            false,
        ),
        (
            "left_hind_leg",
            PartPose {
                offset: [4.0, 15.0, 2.0],
                rotation: [0.0, -FRAC_PI_4, FRAC_PI_4],
            },
            true,
        ),
        (
            "right_middle_hind_leg",
            PartPose {
                offset: [-4.0, 15.0, 1.0],
                rotation: [0.0, FRAC_PI_8, -SPIDER_LEG_SPLAY],
            },
            false,
        ),
        (
            "left_middle_hind_leg",
            PartPose {
                offset: [4.0, 15.0, 1.0],
                rotation: [0.0, -FRAC_PI_8, SPIDER_LEG_SPLAY],
            },
            true,
        ),
        (
            "right_middle_front_leg",
            PartPose {
                offset: [-4.0, 15.0, 0.0],
                rotation: [0.0, -FRAC_PI_8, -SPIDER_LEG_SPLAY],
            },
            false,
        ),
        (
            "left_middle_front_leg",
            PartPose {
                offset: [4.0, 15.0, 0.0],
                rotation: [0.0, FRAC_PI_8, SPIDER_LEG_SPLAY],
            },
            true,
        ),
        (
            "right_front_leg",
            PartPose {
                offset: [-4.0, 15.0, -1.0],
                rotation: [0.0, -FRAC_PI_4, -FRAC_PI_4],
            },
            false,
        ),
        (
            "left_front_leg",
            PartPose {
                offset: [4.0, 15.0, -1.0],
                rotation: [0.0, FRAC_PI_4, FRAC_PI_4],
            },
            true,
        ),
    ]
}

/// Builds the spider tree with the vanilla `SpiderModel.createSpiderBodyLayer` child names: `head`,
/// the two body segments (`body0`/`body1`), and the eight legs in right/left pairs hind-to-front. The
/// render order matches the vanilla layer (head, body0, body1, legs).
fn spider_tree() -> ModelPart {
    let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(11);
    children.push((
        "head",
        ModelPart::leaf(
            PartPose {
                offset: [0.0, 15.0, -3.0],
                rotation: [0.0, 0.0, 0.0],
            },
            SPIDER_HEAD.to_vec(),
        ),
    ));
    children.push((
        "body0",
        ModelPart::leaf(
            PartPose {
                offset: [0.0, 15.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
            },
            SPIDER_BODY_0.to_vec(),
        ),
    ));
    children.push((
        "body1",
        ModelPart::leaf(
            PartPose {
                offset: [0.0, 15.0, 9.0],
                rotation: [0.0, 0.0, 0.0],
            },
            SPIDER_BODY_1.to_vec(),
        ),
    ));
    for (name, pose, left) in spider_legs() {
        let cubes = if left {
            SPIDER_LEFT_LEG.to_vec()
        } else {
            SPIDER_RIGHT_LEG.to_vec()
        };
        children.push((name, ModelPart::leaf(pose, cubes)));
    }
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable spider model, mirroring vanilla `SpiderModel` (shared by the cave spider, which differs
/// only by its smaller root transform). The unified tree is built once with named children.
/// `setup_anim` looks the head ([`apply_head_look`]) and sweeps/steps the eight legs
/// ([`spider_leg_swing_pose`] at the [`spider_leg_swing_roles`] names). Both the base and eyes
/// textured passes read this one posed tree.
pub(in crate::entity_models) struct SpiderModel {
    root: ModelPart,
}

impl SpiderModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: spider_tree(),
        }
    }
}

impl EntityModel for SpiderModel {
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
        let limb_swing = render_state.walk_animation_pos;
        let limb_swing_amount = render_state.walk_animation_speed;
        for (name, phase, side_sign) in spider_leg_swing_roles() {
            let leg = self.root.child_mut(name);
            leg.pose =
                spider_leg_swing_pose(leg.pose, phase, side_sign, limb_swing, limb_swing_amount);
        }
    }
}
