use super::{
    apply_half_amplitude_leg_swing, apply_head_look, witch_nose_bob_pose, PartPose, PART_POSE_ZERO,
    WITCH_HAT_COLOR, WITCH_ROBE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_WITCH: &str = "minecraft:witch#main";

// Vanilla 26.1 WitchModel.createBodyLayer(), with LayerDefinitions' MeshTransformer.scaling(0.9375F)
// applied by the emitter root transform. Each cube carries both render paths' data: the colored debug
// tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const WITCH_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -10.0, -4.0],
    [8.0, 10.0, 8.0],
    WITCH_ROBE,
    [8.0, 10.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const WITCH_HAT: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [10.0, 2.0, 10.0],
    WITCH_HAT_COLOR,
    [10.0, 2.0, 10.0],
    [0.0, 64.0],
    false,
)];

pub(in crate::entity_models) const WITCH_HAT_2: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [7.0, 4.0, 7.0],
    WITCH_HAT_COLOR,
    [7.0, 4.0, 7.0],
    [0.0, 76.0],
    false,
)];

pub(in crate::entity_models) const WITCH_HAT_3: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [4.0, 4.0, 4.0],
    WITCH_HAT_COLOR,
    [4.0, 4.0, 4.0],
    [0.0, 87.0],
    false,
)];

pub(in crate::entity_models) const WITCH_HAT_4: [ModelCube; 1] = [ModelCube::new(
    [-0.25, -0.25, -0.25],
    [1.5, 2.5, 1.5],
    WITCH_HAT_COLOR,
    [1.0, 2.0, 1.0],
    [0.0, 95.0],
    false,
)];

pub(in crate::entity_models) const WITCH_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -6.0],
    [2.0, 4.0, 2.0],
    WITCH_ROBE,
    [2.0, 4.0, 2.0],
    [24.0, 0.0],
    false,
)];

pub(in crate::entity_models) const WITCH_MOLE: [ModelCube; 1] = [ModelCube::new(
    [0.25, 3.25, -6.5],
    [0.5, 0.5, 0.5],
    WITCH_ROBE,
    [1.0, 1.0, 1.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const WITCH_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -3.0],
    [8.0, 12.0, 6.0],
    WITCH_ROBE,
    [8.0, 12.0, 6.0],
    [16.0, 20.0],
    false,
)];

pub(in crate::entity_models) const WITCH_JACKET: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -0.5, -3.5],
    [9.0, 21.0, 7.0],
    WITCH_ROBE,
    [8.0, 20.0, 6.0],
    [0.0, 38.0],
    false,
)];

pub(in crate::entity_models) const WITCH_ARMS: [ModelCube; 3] = [
    ModelCube::new(
        [-8.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        WITCH_ROBE,
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        false,
    ),
    ModelCube::new(
        [4.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        WITCH_ROBE,
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        true,
    ),
    ModelCube::new(
        [-4.0, 2.0, -2.0],
        [8.0, 4.0, 4.0],
        WITCH_ROBE,
        [8.0, 4.0, 4.0],
        [40.0, 38.0],
        false,
    ),
];

pub(in crate::entity_models) const WITCH_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    WITCH_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    false,
)];

pub(in crate::entity_models) const WITCH_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    WITCH_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];

/// The nested witch-hat child part poses (`hat` -> `hat2` -> `hat3` -> `hat4`, the drooping tip).
pub(in crate::entity_models) const WITCH_HAT_POSE: PartPose = PartPose {
    offset: [-5.0, -10.03125, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const WITCH_HAT_2_POSE: PartPose = PartPose {
    offset: [1.75, -4.0, 2.0],
    rotation: [-0.05235988, 0.0, 0.02617994],
};
pub(in crate::entity_models) const WITCH_HAT_3_POSE: PartPose = PartPose {
    offset: [1.75, -4.0, 2.0],
    rotation: [-0.10471976, 0.0, 0.05235988],
};
pub(in crate::entity_models) const WITCH_HAT_4_POSE: PartPose = PartPose {
    offset: [1.75, -2.0, 2.0],
    rotation: [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
};

/// The nose child part pose (which parents the mole) and the mole child part pose.
pub(in crate::entity_models) const WITCH_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const WITCH_MOLE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// The arms and right/left leg part poses.
pub(in crate::entity_models) const WITCH_ARMS_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, -1.0],
    rotation: [-0.75, 0.0, 0.0],
};
pub(in crate::entity_models) const WITCH_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const WITCH_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf part at `pose` carrying `cubes`.
fn leaf(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the witch `head` part: the head cube parents the nested hat chain (`hat` -> `hat2` -> `hat3`
/// -> `hat4`) and the nose (which parents the mole), in vanilla render order.
fn witch_head() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        WITCH_HEAD.to_vec(),
        vec![
            (
                "hat",
                ModelPart::new(
                    WITCH_HAT_POSE,
                    WITCH_HAT.to_vec(),
                    vec![(
                        "hat2",
                        ModelPart::new(
                            WITCH_HAT_2_POSE,
                            WITCH_HAT_2.to_vec(),
                            vec![(
                                "hat3",
                                ModelPart::new(
                                    WITCH_HAT_3_POSE,
                                    WITCH_HAT_3.to_vec(),
                                    vec![("hat4", leaf(WITCH_HAT_4_POSE, &WITCH_HAT_4))],
                                ),
                            )],
                        ),
                    )],
                ),
            ),
            (
                "nose",
                ModelPart::new(
                    WITCH_NOSE_POSE,
                    WITCH_NOSE.to_vec(),
                    vec![("mole", leaf(WITCH_MOLE_POSE, &WITCH_MOLE))],
                ),
            ),
        ],
    )
}

/// Builds the witch `body` part: the body cube parents the jacket.
fn witch_body() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        WITCH_BODY.to_vec(),
        vec![("jacket", leaf(PART_POSE_ZERO, &WITCH_JACKET))],
    )
}

/// Builds the unified witch tree under the vanilla `WitchModel` child names (`head`, `body`, the
/// combined `arms`, `right_leg`, `left_leg`).
fn witch_tree() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", witch_head()),
            ("body", witch_body()),
            ("arms", leaf(WITCH_ARMS_POSE, &WITCH_ARMS)),
            ("right_leg", leaf(WITCH_RIGHT_LEG_POSE, &WITCH_RIGHT_LEG)),
            ("left_leg", leaf(WITCH_LEFT_LEG_POSE, &WITCH_LEFT_LEG)),
        ],
    )
}

/// Mutable witch model, mirroring vanilla `WitchModel`. The unified tree is built with the vanilla
/// child names: `head` parents the hat chain, the nose, and the mole; `body` parents the jacket; the
/// combined `arms` and the two legs hang off the root. `setup_anim` looks the head ([`apply_head_look`]
/// on `head`), swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]), then bobs the nose continuously ([`witch_nose_bob_pose`],
/// driven by `ageInTicks` and the entity id), and finally applies the `isHoldingItem` nose hold pose —
/// reached as the head's `nose` child so it inherits the head look.
pub(in crate::entity_models) struct WitchModel {
    root: ModelPart,
}

impl WitchModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self { root: witch_tree() }
    }
}

impl EntityModel for WitchModel {
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
        apply_half_amplitude_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        let nose = self.root.child_mut("head").child_mut("nose");
        nose.pose = witch_nose_bob_pose(nose.pose, render_state.age_in_ticks, instance.entity_id);
        if render_state.witch_holding_item {
            nose.pose.offset = [0.0, 1.0, -1.5];
            nose.pose.rotation[0] = -0.9;
        }
    }
}
