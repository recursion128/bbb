use super::{
    apply_half_amplitude_leg_swing, apply_head_look, PartPose, PART_POSE_ZERO, VILLAGER_ROBE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_VILLAGER: &str = "minecraft:villager#main";
pub(in crate::entity_models) const MODEL_LAYER_VILLAGER_BABY: &str = "minecraft:villager_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_WANDERING_TRADER: &str =
    "minecraft:wandering_trader#main";

// Vanilla 26.1 VillagerModel.createBodyModel(), with LayerDefinitions' MeshTransformer.scaling(0.9375F)
// applied by the emitter root transform. Each cube carries both render paths' data: the colored debug
// tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_VILLAGER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -10.0, -4.0],
    [8.0, 10.0, 8.0],
    VILLAGER_ROBE,
    [8.0, 10.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.51, -10.51, -4.51],
    [9.02, 11.02, 9.02],
    VILLAGER_ROBE,
    [8.0, 10.0, 8.0],
    [32.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_HAT_RIM: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -8.0, -6.0],
    [16.0, 16.0, 1.0],
    VILLAGER_ROBE,
    [16.0, 16.0, 1.0],
    [30.0, 47.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -1.0, -6.0],
    [2.0, 4.0, 2.0],
    VILLAGER_ROBE,
    [2.0, 4.0, 2.0],
    [24.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -3.0],
    [8.0, 12.0, 6.0],
    VILLAGER_ROBE,
    [8.0, 12.0, 6.0],
    [16.0, 20.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_JACKET: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -0.5, -3.5],
    [9.0, 21.0, 7.0],
    VILLAGER_ROBE,
    [8.0, 20.0, 6.0],
    [0.0, 38.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_ARMS: [ModelCube; 3] = [
    ModelCube::new(
        [-8.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        VILLAGER_ROBE,
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        false,
    ),
    ModelCube::new(
        [4.0, -2.0, -2.0],
        [4.0, 8.0, 4.0],
        VILLAGER_ROBE,
        [4.0, 8.0, 4.0],
        [44.0, 22.0],
        true,
    ),
    ModelCube::new(
        [-4.0, 2.0, -2.0],
        [8.0, 4.0, 4.0],
        VILLAGER_ROBE,
        [8.0, 4.0, 4.0],
        [40.0, 38.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_VILLAGER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    VILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    false,
)];

pub(in crate::entity_models) const ADULT_VILLAGER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    VILLAGER_ROBE,
    [4.0, 12.0, 4.0],
    [0.0, 22.0],
    true,
)];

/// Adult villager arms-container part pose (vanilla `VillagerModel.createBodyModel`).
pub(in crate::entity_models) const ADULT_VILLAGER_ARMS_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, -1.0],
    rotation: [-0.75, 0.0, 0.0],
};

/// Adult villager hat-rim child part pose (pitched `-π/2`).
pub(in crate::entity_models) const ADULT_VILLAGER_HAT_RIM_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

/// Adult villager nose child part pose (under the head).
pub(in crate::entity_models) const ADULT_VILLAGER_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 BabyVillagerModel.createBodyModel() (atlas 64×64).
pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_HAND: [ModelCube; 2] = [
    ModelCube::new(
        [-1.0, -2.4925, -1.8401],
        [2.0, 4.0, 2.0],
        VILLAGER_ROBE,
        [2.0, 4.0, 2.0],
        [36.0, 15.0],
        false,
    ),
    ModelCube::new(
        [5.0, -2.4925, -1.8401],
        [2.0, 4.0, 2.0],
        VILLAGER_ROBE,
        [2.0, 4.0, 2.0],
        [16.0, 15.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_VILLAGER_MIDDLE_ARM: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -0.9924, -0.9825],
    [4.0, 2.0, 2.0],
    VILLAGER_ROBE,
    [4.0, 2.0, 2.0],
    [24.0, 17.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 3.0, 2.0],
    VILLAGER_ROBE,
    [2.0, 3.0, 2.0],
    [8.0, 23.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -0.5, -1.0],
    [2.0, 3.0, 2.0],
    VILLAGER_ROBE,
    [2.0, 3.0, 2.0],
    [0.0, 23.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -8.0, -3.5],
    [8.0, 8.0, 7.0],
    VILLAGER_ROBE,
    [8.0, 8.0, 7.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_HAT: [ModelCube; 1] = [ModelCube::new(
    [-4.3, -4.3, -3.8],
    [8.6, 8.6, 7.6],
    VILLAGER_ROBE,
    [8.0, 8.0, 7.0],
    [0.0, 30.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_HAT_RIM: [ModelCube; 1] = [ModelCube::new(
    [-7.0, -0.5, -6.0],
    [14.0, 1.0, 12.0],
    VILLAGER_ROBE,
    [14.0, 1.0, 12.0],
    [0.0, 45.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_NOSE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -0.5],
    [2.0, 2.0, 1.0],
    VILLAGER_ROBE,
    [2.0, 2.0, 1.0],
    [23.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_BODY: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.75, -1.5],
    [4.0, 5.0, 3.0],
    VILLAGER_ROBE,
    [4.0, 5.0, 3.0],
    [0.0, 15.0],
    false,
)];

pub(in crate::entity_models) const BABY_VILLAGER_BB_MAIN: [ModelCube; 1] = [ModelCube::new(
    [-2.7, -8.2, -1.7],
    [4.4, 6.4, 3.4],
    VILLAGER_ROBE,
    [4.0, 6.0, 3.0],
    [16.0, 21.0],
    false,
)];

/// Baby villager arms-container, leg, head, body, and bb_main part poses
/// (vanilla `BabyVillagerModel.createBodyModel`).
pub(in crate::entity_models) const BABY_VILLAGER_ARMS_POSE: PartPose = PartPose {
    offset: [0.0, 17.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.0, 21.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.0, 21.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.75, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_BB_MAIN_POSE: PartPose = PartPose {
    offset: [0.5, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Baby villager arms-container child part poses (the right-hand pair and the middle arm).
pub(in crate::entity_models) const BABY_VILLAGER_RIGHT_HAND_POSE: PartPose = PartPose {
    offset: [-3.0, 1.4025, -0.9599],
    rotation: [-1.0472, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_MIDDLE_ARM_POSE: PartPose = PartPose {
    offset: [0.0, 0.9024, -1.8175],
    rotation: [-1.0472, 0.0, 0.0],
};

/// Baby villager head child part poses (the hat, the hat rim, the nose).
pub(in crate::entity_models) const BABY_VILLAGER_HAT_POSE: PartPose = PartPose {
    offset: [0.0, -4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_HAT_RIM_POSE: PartPose = PartPose {
    offset: [0.0, -4.5, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BABY_VILLAGER_NOSE_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf part at `pose` carrying `cubes`.
fn leaf(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds the adult villager `head` part: the head cube parents the hat (which parents the hat rim)
/// and the nose, in vanilla render order.
fn adult_head() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        ADULT_VILLAGER_HEAD.to_vec(),
        vec![
            (
                "hat",
                ModelPart::new(
                    PART_POSE_ZERO,
                    ADULT_VILLAGER_HAT.to_vec(),
                    vec![(
                        "hat_rim",
                        leaf(ADULT_VILLAGER_HAT_RIM_POSE, &ADULT_VILLAGER_HAT_RIM),
                    )],
                ),
            ),
            ("nose", leaf(ADULT_VILLAGER_NOSE_POSE, &ADULT_VILLAGER_NOSE)),
        ],
    )
}

/// Builds the adult villager `body` part: the body cube parents the jacket.
fn adult_body() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        ADULT_VILLAGER_BODY.to_vec(),
        vec![("jacket", leaf(PART_POSE_ZERO, &ADULT_VILLAGER_JACKET))],
    )
}

/// Builds the unified adult villager / wandering trader tree under the vanilla `VillagerModel` child
/// names (`head`, `body`, the combined `arms`, `right_leg`, `left_leg`).
fn adult_villager_tree() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            ("head", adult_head()),
            ("body", adult_body()),
            ("arms", leaf(ADULT_VILLAGER_ARMS_POSE, &ADULT_VILLAGER_ARMS)),
            (
                "right_leg",
                leaf(
                    PartPose {
                        offset: [-2.0, 12.0, 0.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    &ADULT_VILLAGER_RIGHT_LEG,
                ),
            ),
            (
                "left_leg",
                leaf(
                    PartPose {
                        offset: [2.0, 12.0, 0.0],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    &ADULT_VILLAGER_LEFT_LEG,
                ),
            ),
        ],
    )
}

/// Builds the baby villager `head` part: the head cube parents the hat, hat rim, and nose, in vanilla
/// render order.
fn baby_head() -> ModelPart {
    ModelPart::new(
        BABY_VILLAGER_HEAD_POSE,
        BABY_VILLAGER_HEAD.to_vec(),
        vec![
            ("hat", leaf(BABY_VILLAGER_HAT_POSE, &BABY_VILLAGER_HAT)),
            (
                "hat_rim",
                leaf(BABY_VILLAGER_HAT_RIM_POSE, &BABY_VILLAGER_HAT_RIM),
            ),
            ("nose", leaf(BABY_VILLAGER_NOSE_POSE, &BABY_VILLAGER_NOSE)),
        ],
    )
}

/// Builds the unified baby villager tree under the vanilla `VillagerModel` child names. The baby
/// `createBodyModel` lists the parts in a different order (arms container, legs, head, body, bb_main),
/// preserved here so the colored render order stays byte-identical, while the leg swing and head look
/// resolve their parts by name.
fn baby_villager_tree() -> ModelPart {
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "arms",
                ModelPart::new(
                    BABY_VILLAGER_ARMS_POSE,
                    Vec::new(),
                    vec![
                        (
                            "right_hand",
                            leaf(BABY_VILLAGER_RIGHT_HAND_POSE, &BABY_VILLAGER_RIGHT_HAND),
                        ),
                        (
                            "middle_arm",
                            leaf(BABY_VILLAGER_MIDDLE_ARM_POSE, &BABY_VILLAGER_MIDDLE_ARM),
                        ),
                    ],
                ),
            ),
            (
                "right_leg",
                leaf(BABY_VILLAGER_RIGHT_LEG_POSE, &BABY_VILLAGER_RIGHT_LEG),
            ),
            (
                "left_leg",
                leaf(BABY_VILLAGER_LEFT_LEG_POSE, &BABY_VILLAGER_LEFT_LEG),
            ),
            ("head", baby_head()),
            ("body", leaf(BABY_VILLAGER_BODY_POSE, &BABY_VILLAGER_BODY)),
            (
                "bb_main",
                leaf(BABY_VILLAGER_BB_MAIN_POSE, &BABY_VILLAGER_BB_MAIN),
            ),
        ],
    )
}

/// Builds the unified villager tree for the selected `baby` layout under the vanilla child names.
fn villager_tree(baby: bool) -> ModelPart {
    if baby {
        baby_villager_tree()
    } else {
        adult_villager_tree()
    }
}

/// Mutable villager model, mirroring vanilla `VillagerModel`/`BabyVillagerModel`. The unified tree is
/// built for the selected `baby` layout with the vanilla child names. `setup_anim` looks the head
/// ([`apply_head_look`] on `head`) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]). The combined `arms` part and the unhappy head shake
/// defer.
pub(in crate::entity_models) struct VillagerModel {
    root: ModelPart,
}

impl VillagerModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: villager_tree(baby),
        }
    }
}

impl EntityModel for VillagerModel {
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
    }
}

/// Mutable wandering trader model, mirroring vanilla `WanderingTraderRenderer`, which reuses the adult
/// `VillagerModel` layer. The unified tree is built with the vanilla child names (`head`, `body`, the
/// combined `arms`, `right_leg`, `left_leg`). `setup_anim` looks the head ([`apply_head_look`] on
/// `head`) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]). The held-item arm pose and the combined `arms` part
/// defer.
pub(in crate::entity_models) struct WanderingTraderModel {
    root: ModelPart,
}

impl WanderingTraderModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: adult_villager_tree(),
        }
    }
}

impl EntityModel for WanderingTraderModel {
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
    }
}
