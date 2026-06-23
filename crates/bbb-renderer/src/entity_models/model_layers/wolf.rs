use super::{
    apply_head_look, apply_wolf_sitting_pose, head_first_part_index, quadruped_leg_swing_pose,
    wolf_angry_tail_pose, wolf_sitting_part_roles, wolf_tail_part_index, wolf_tail_swing_pose,
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

pub(in crate::entity_models) const WOLF_GRAY: [f32; 4] = [0.64, 0.66, 0.66, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_WOLF: &str = "minecraft:wolf#main";
pub(in crate::entity_models) const MODEL_LAYER_WOLF_BABY: &str = "minecraft:wolf_baby#main";

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_REAL_HEAD: [TexturedModelCubeDesc; 4] = [
    TexturedModelCubeDesc {
        min: [-2.0, -3.0, -2.0],
        size: [6.0, 6.0, 4.0],
        uv_size: [6.0, 6.0, 4.0],
        tex: [0.0, 0.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [16.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [16.0, 14.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-0.5, -0.001, -5.0],
        size: [3.0, 3.0, 4.0],
        uv_size: [3.0, 3.0, 4.0],
        tex: [0.0, 10.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_WOLF_TEXTURED_REAL_HEAD,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -3.0],
        size: [6.0, 9.0, 6.0],
        uv_size: [6.0, 9.0, 6.0],
        tex: [18.0, 14.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_UPPER_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -3.0, -3.0],
        size: [8.0, 6.0, 7.0],
        uv_size: [8.0, 6.0, 7.0],
        tex: [21.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_LEFT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_RIGHT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [0.0, 18.0],
        mirror: true,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_REAL_TAIL: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [0.0, 0.0, -1.0],
        size: [2.0, 8.0, 2.0],
        uv_size: [2.0, 8.0, 2.0],
        tex: [9.0, 18.0],
        mirror: false,
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_TAIL_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: PART_POSE_ZERO,
        cubes: &ADULT_WOLF_TEXTURED_REAL_TAIL,
        children: &[],
    }];

pub(in crate::entity_models) const ADULT_WOLF_TEXTURED_PARTS: [TexturedModelPartDesc; 8] = [
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[0].pose,
        cubes: &[],
        children: &ADULT_WOLF_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[1].pose,
        cubes: &ADULT_WOLF_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[2].pose,
        cubes: &ADULT_WOLF_TEXTURED_UPPER_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[3].pose,
        cubes: &ADULT_WOLF_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[4].pose,
        cubes: &ADULT_WOLF_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[5].pose,
        cubes: &ADULT_WOLF_TEXTURED_RIGHT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[6].pose,
        cubes: &ADULT_WOLF_TEXTURED_LEFT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: ADULT_WOLF_PARTS[7].pose,
        cubes: &[],
        children: &ADULT_WOLF_TEXTURED_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_HEAD: [TexturedModelCubeDesc; 2] = [
    TexturedModelCubeDesc {
        min: [-3.015, -3.275, -3.025],
        size: [6.05, 5.05, 5.05],
        uv_size: [6.0, 5.0, 5.0],
        tex: [0.0, 12.0],
        mirror: false,
    },
    TexturedModelCubeDesc {
        min: [-1.5, -0.24, -5.0],
        size: [3.0, 2.0, 2.0],
        uv_size: [3.0, 2.0, 2.0],
        tex: [17.0, 12.0],
        mirror: false,
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [0.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_EAR: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -1.0, -0.5],
        size: [2.0, 2.0, 1.0],
        uv_size: [2.0, 2.0, 1.0],
        tex: [20.0, 5.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_HEAD_CHILDREN: [TexturedModelPartDesc; 2] = [
    TexturedModelPartDesc {
        pose: BABY_WOLF_HEAD_CHILDREN[0].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_EAR,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_HEAD_CHILDREN[1].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-3.0, -2.0, -4.0],
        size: [6.0, 4.0, 8.0],
        uv_size: [6.0, 4.0, 8.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [0.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_HIND_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [8.0, 22.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_RIGHT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_LEFT_FRONT_LEG: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 3.0, 2.0],
        uv_size: [2.0, 3.0, 2.0],
        tex: [20.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_TAIL_R1: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, -5.7, -1.0],
        size: [2.0, 6.0, 2.0],
        uv_size: [2.0, 6.0, 2.0],
        tex: [22.0, 16.0],
        mirror: false,
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_TAIL_CHILDREN: [TexturedModelPartDesc; 1] =
    [TexturedModelPartDesc {
        pose: BABY_WOLF_TAIL_CHILDREN[0].pose,
        cubes: &BABY_WOLF_TEXTURED_TAIL_R1,
        children: &[],
    }];

pub(in crate::entity_models) const BABY_WOLF_TEXTURED_PARTS: [TexturedModelPartDesc; 7] = [
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[0].pose,
        cubes: &BABY_WOLF_TEXTURED_HEAD,
        children: &BABY_WOLF_TEXTURED_HEAD_CHILDREN,
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[1].pose,
        cubes: &BABY_WOLF_TEXTURED_BODY,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[2].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[3].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_HIND_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[4].pose,
        cubes: &BABY_WOLF_TEXTURED_RIGHT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[5].pose,
        cubes: &BABY_WOLF_TEXTURED_LEFT_FRONT_LEG,
        children: &[],
    },
    TexturedModelPartDesc {
        pose: BABY_WOLF_PARTS[6].pose,
        cubes: &[],
        children: &BABY_WOLF_TEXTURED_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_REAL_HEAD: [ModelCubeDesc; 4] = [
    ModelCubeDesc {
        min: [-2.0, -3.0, -2.0],
        size: [6.0, 6.0, 4.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [2.0, -5.0, 0.0],
        size: [2.0, 2.0, 1.0],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-0.5, -0.001, -5.0],
        size: [3.0, 3.0, 4.0],
        color: WOLF_GRAY,
    },
];

pub(in crate::entity_models) const ADULT_WOLF_HEAD_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_WOLF_REAL_HEAD,
    children: &[],
}];

pub(in crate::entity_models) const ADULT_WOLF_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -3.0],
    size: [6.0, 9.0, 6.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_UPPER_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -3.0, -3.0],
    size: [8.0, 6.0, 7.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [2.0, 8.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_REAL_TAIL: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 0.0, -1.0],
    size: [2.0, 8.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const ADULT_WOLF_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PART_POSE_ZERO,
    cubes: &ADULT_WOLF_REAL_TAIL,
    children: &[],
}];

// Vanilla 26.1 AdultWolfModel.createBodyLayer(CubeDeformation.NONE).
pub(in crate::entity_models) const ADULT_WOLF_PARTS: [ModelPartDesc; 8] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 13.5, -7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &[],
        children: &ADULT_WOLF_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 14.0, 2.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 14.0, -3.0],
            rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_UPPER_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 16.0, 7.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.5, 16.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.5, 16.0, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &ADULT_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.0, 12.0, 8.0],
            rotation: [0.62831855, 0.0, 0.0],
        },
        cubes: &[],
        children: &ADULT_WOLF_TAIL_CHILDREN,
    },
];

pub(in crate::entity_models) const BABY_WOLF_HEAD: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [-3.015, -3.275, -3.025],
        size: [6.05, 5.05, 5.05],
        color: WOLF_GRAY,
    },
    ModelCubeDesc {
        min: [-1.5, -0.24, -5.0],
        size: [3.0, 2.0, 2.0],
        color: WOLF_GRAY,
    },
];

pub(in crate::entity_models) const BABY_WOLF_EAR: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -1.0, -0.5],
    size: [2.0, 2.0, 1.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_HEAD_CHILDREN: [ModelPartDesc; 2] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [-2.0, -4.25, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_EAR,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [2.0, -4.25, -0.5],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_EAR,
        children: &[],
    },
];

pub(in crate::entity_models) const BABY_WOLF_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-3.0, -2.0, -4.0],
    size: [6.0, 4.0, 8.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_LEG: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 3.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_TAIL_R1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -5.7, -1.0],
    size: [2.0, 6.0, 2.0],
    color: WOLF_GRAY,
}];

pub(in crate::entity_models) const BABY_WOLF_TAIL_CHILDREN: [ModelPartDesc; 1] = [ModelPartDesc {
    pose: PartPose {
        offset: [0.0, -0.6, 0.2],
        rotation: [-3.1, 0.0, 0.0],
    },
    cubes: &BABY_WOLF_TAIL_R1,
    children: &[],
}];

// Vanilla 26.1 BabyWolfModel.createBodyLayer().
pub(in crate::entity_models) const BABY_WOLF_PARTS: [ModelPartDesc; 7] = [
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 18.25, -4.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_HEAD,
        children: &BABY_WOLF_HEAD_CHILDREN,
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_BODY,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 21.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [-1.5, 21.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [1.5, 21.0, -3.0],
            rotation: [0.0, 0.0, 0.0],
        },
        cubes: &BABY_WOLF_LEG,
        children: &[],
    },
    ModelPartDesc {
        pose: PartPose {
            offset: [0.0, 19.0, 3.0],
            rotation: [-0.5236, 0.0, 0.0],
        },
        cubes: &[],
        children: &BABY_WOLF_TAIL_CHILDREN,
    },
];

/// Selects the colored + textured trees for an adult or baby wolf, zipped into the unified tree by
/// [`WolfModel::new`]. The adult layer lists head/body/mane at `0`/`1`/`2`, the four legs at
/// `[3, 4, 5, 6]`, and the tail at `7`; the baby layer drops the mane (legs `[2, 3, 4, 5]`, tail `6`).
fn wolf_part_trees(baby: bool) -> (&'static [ModelPartDesc], &'static [TexturedModelPartDesc]) {
    if baby {
        (&BABY_WOLF_PARTS, &BABY_WOLF_TEXTURED_PARTS)
    } else {
        (&ADULT_WOLF_PARTS, &ADULT_WOLF_TEXTURED_PARTS)
    }
}

/// The four leg part indices in the wolf body layers (adult `[3, 4, 5, 6]`, baby `[2, 3, 4, 5]`).
const fn wolf_leg_part_indices(baby: bool) -> [usize; 4] {
    if baby {
        [2, 3, 4, 5]
    } else {
        [3, 4, 5, 6]
    }
}

/// Mutable wolf model, mirroring vanilla `WolfModel` / `BabyWolfModel`. The unified tree is zipped from
/// the colored and textured trees selected by `baby` ([`wolf_part_trees`]): the head (carrying the
/// ears), body, mane (adult only), the four legs, and the tail (carrying its tip). `setup_anim` runs the
/// shared `WolfModel.setupAnim`: the head follows the look, then either the `setSittingPose` fold (body
/// tilt + leg tuck + tail lift) or the `QuadrupedModel` diagonal leg swing, then the tail `xRot`
/// (`tailAngle`) + wag `yRot` (angry → the raised constant). The collar dye overlay is a second textured
/// pass on the same posed tree; the water-shake roll is deferred.
pub(in crate::entity_models) struct WolfModel {
    root: ModelPart,
    baby: bool,
    angry: bool,
}

impl WolfModel {
    pub(in crate::entity_models) fn new(baby: bool, angry: bool) -> Self {
        let (colored, textured) = wolf_part_trees(baby);
        Self {
            root: ModelPart::root_from_descs(colored, textured),
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
        let baby = self.baby;

        // The head (root child 0) carries the ears, so the look turns the whole head subtree.
        apply_head_look(
            self.root.child_at_mut(head_first_part_index()),
            render_state.head_yaw,
            render_state.head_pitch,
        );

        // A sitting wolf folds (`setSittingPose`) instead of swinging its legs; the tail rotation below
        // then layers onto the sitting offset lift (both helpers preserve the offset).
        if render_state.wolf_sitting {
            for (index, role) in wolf_sitting_part_roles(baby) {
                apply_wolf_sitting_pose(&mut self.root.child_at_mut(index).pose, role, baby);
            }
        } else {
            for index in wolf_leg_part_indices(baby) {
                let leg = self.root.child_at_mut(index);
                leg.pose = quadruped_leg_swing_pose(
                    leg.pose,
                    render_state.walk_animation_pos,
                    render_state.walk_animation_speed,
                );
            }
        }

        // The tail `xRot` is SET to `tailAngle` (wag `yRot` on top) in both poses; an angry wolf holds
        // the raised constant with no wag.
        let tail = self.root.child_at_mut(wolf_tail_part_index(baby));
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
