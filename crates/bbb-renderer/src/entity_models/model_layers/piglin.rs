use super::{
    apply_head_look, apply_humanoid_leg_swing_named, apply_humanoid_walk_named,
    piglin_ear_flap_pose, PartPose, PART_POSE_ZERO, PIGLIN_ADULT_EAR_ANGLE, PIGLIN_BABY_EAR_ANGLE,
};
use crate::entity_models::catalog::PiglinModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_PIGLIN: &str = "minecraft:piglin#main";
pub(in crate::entity_models) const MODEL_LAYER_PIGLIN_BABY: &str = "minecraft:piglin_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_PIGLIN_BRUTE: &str = "minecraft:piglin_brute#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIFIED_PIGLIN: &str =
    "minecraft:zombified_piglin#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIFIED_PIGLIN_BABY: &str =
    "minecraft:zombified_piglin_baby#main";

pub(in crate::entity_models) const PIGLIN_SKIN: [f32; 4] = [0.74, 0.44, 0.36, 1.0];
pub(in crate::entity_models) const PIGLIN_BRUTE_SKIN: [f32; 4] = [0.58, 0.35, 0.29, 1.0];
pub(in crate::entity_models) const ZOMBIFIED_PIGLIN_SKIN: [f32; 4] = [0.46, 0.62, 0.42, 1.0];

// Vanilla 26.1 AbstractPiglinModel/AdultPiglinModel/BabyPiglinModel.createBodyLayer(). Each cube
// carries both render paths' data: the colored debug tint and the textured `uv_size` / `texOffs` /
// `mirror`. The deformed sleeve/pants cubes inflate their colored geometry but keep the base box as
// `uv_size` (the squid-body precedent); the piglin clears the body `jacket` but keeps the arm sleeves
// and leg pants.
pub(in crate::entity_models) const ADULT_PIGLIN_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-5.0, -8.0, -4.0],
        [10.0, 8.0, 8.0],
        PIGLIN_SKIN,
        [10.0, 8.0, 8.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -4.0, -5.0],
        [4.0, 4.0, 1.0],
        PIGLIN_SKIN,
        [4.0, 4.0, 1.0],
        [31.0, 1.0],
        false,
    ),
    ModelCube::new(
        [2.0, -2.0, -5.0],
        [1.0, 2.0, 1.0],
        PIGLIN_SKIN,
        [1.0, 2.0, 1.0],
        [2.0, 4.0],
        false,
    ),
    ModelCube::new(
        [-3.0, -2.0, -5.0],
        [1.0, 2.0, 1.0],
        PIGLIN_SKIN,
        [1.0, 2.0, 1.0],
        [2.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -2.0],
    [1.0, 5.0, 4.0],
    PIGLIN_SKIN,
    [1.0, 5.0, 4.0],
    [51.0, 6.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -2.0],
    [1.0, 5.0, 4.0],
    PIGLIN_SKIN,
    [1.0, 5.0, 4.0],
    [39.0, 6.0],
    false,
)];

/// Adult piglin ear child poses (vanilla `AbstractPiglinModel.addHead`).
const ADULT_PIGLIN_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [4.5, -6.0, 0.0],
    rotation: [0.0, 0.0, -std::f32::consts::FRAC_PI_6],
};
const ADULT_PIGLIN_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-4.5, -6.0, 0.0],
    rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_6],
};

pub(in crate::entity_models) const ADULT_PIGLIN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -2.0],
    [8.0, 12.0, 4.0],
    PIGLIN_SKIN,
    [8.0, 12.0, 4.0],
    [16.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [40.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -2.0, -2.0],
    [4.0, 12.0, 4.0],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [32.0, 48.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-3.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [40.0, 32.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_SLEEVE: [ModelCube; 1] = [ModelCube::new(
    [-1.25, -2.25, -2.25],
    [4.5, 12.5, 4.5],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [48.0, 48.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [16.0, 48.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_RIGHT_PANTS: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [0.0, 32.0],
    false,
)];

pub(in crate::entity_models) const ADULT_PIGLIN_LEFT_PANTS: [ModelCube; 1] = [ModelCube::new(
    [-2.25, -0.25, -2.25],
    [4.5, 12.5, 4.5],
    PIGLIN_SKIN,
    [4.0, 12.0, 4.0],
    [0.0, 48.0],
    false,
)];

/// Adult piglin part poses (vanilla `AdultPiglinModel.createBodyLayer`).
const ADULT_PIGLIN_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_PIGLIN_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_PIGLIN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const ADULT_PIGLIN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_PIGLIN_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -3.0, -1.0],
    [6.0, 5.0, 3.0],
    PIGLIN_SKIN,
    [6.0, 5.0, 3.0],
    [0.0, 13.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIGLIN_HEAD: [ModelCube; 2] = [
    ModelCube::new(
        [-1.5, -3.0, -4.5],
        [3.0, 3.0, 1.0],
        PIGLIN_SKIN,
        [3.0, 3.0, 1.0],
        [21.0, 30.0],
        false,
    ),
    ModelCube::new(
        [-4.5, -6.0, -3.5],
        [9.0, 6.0, 7.0],
        PIGLIN_SKIN,
        [9.0, 6.0, 7.0],
        [0.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -3.0, -2.0],
    [1.0, 6.0, 4.0],
    PIGLIN_SKIN,
    [1.0, 6.0, 4.0],
    [0.0, 21.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_EAR: [ModelCube; 1] = [ModelCube::new(
    [-0.5, -3.0, -2.0],
    [1.0, 6.0, 4.0],
    PIGLIN_SKIN,
    [1.0, 6.0, 4.0],
    [18.0, 13.0],
    false,
)];

/// Baby piglin ear-holder child poses, and the rotated ear poses nested under them (vanilla
/// `BabyPiglinModel.createBodyLayer`). The flapping ear holder (`left_ear`/`right_ear`) parents the
/// rotated ear cube; the flap pose is applied to the holder, the cube hangs off it.
const BABY_PIGLIN_LEFT_EAR_HOLDER_POSE: PartPose = PartPose {
    offset: [4.2, -4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_RIGHT_EAR_HOLDER_POSE: PartPose = PartPose {
    offset: [-4.2, -4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_LEFT_EAR_POSE: PartPose = PartPose {
    offset: [1.0, 1.75, 0.0],
    rotation: [0.0, 0.0, -0.6109],
};
const BABY_PIGLIN_RIGHT_EAR_POSE: PartPose = PartPose {
    offset: [-1.0, 1.75, 0.0],
    rotation: [0.0, 0.0, 0.6109],
};

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.5],
    [2.0, 5.0, 3.0],
    PIGLIN_SKIN,
    [2.0, 5.0, 3.0],
    [28.0, 13.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_ARM: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.5],
    [2.0, 5.0, 3.0],
    PIGLIN_SKIN,
    [2.0, 5.0, 3.0],
    [10.0, 30.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIGLIN_RIGHT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 4.0, 3.0],
    PIGLIN_SKIN,
    [3.0, 4.0, 3.0],
    [22.0, 23.0],
    false,
)];

pub(in crate::entity_models) const BABY_PIGLIN_LEFT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, -1.5],
    [3.0, 4.0, 3.0],
    PIGLIN_SKIN,
    [3.0, 4.0, 3.0],
    [10.0, 23.0],
    false,
)];

/// Baby piglin part poses (vanilla `BabyPiglinModel.createBodyLayer`).
const BABY_PIGLIN_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.0, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [4.0, 15.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-4.0, 15.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.5, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const BABY_PIGLIN_LEFT_LEG_POSE: PartPose = PartPose {
    offset: [1.5, 20.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Whether a piglin renders the baby layer. The brute reuses the adult model even as a baby, so only
/// the (zombified) piglin uses the smaller baby layout. Drives the part tree, ear children, and the
/// default ear angle.
fn piglin_baby_layout(family: PiglinModelFamily, baby: bool) -> bool {
    baby && family != PiglinModelFamily::PiglinBrute
}

/// `AbstractPiglinModel.getDefaultEarAngleInDegrees()` (in radians): `5°` baby, `30°` adult/brute.
fn piglin_default_ear_angle(baby_layout: bool) -> f32 {
    if baby_layout {
        PIGLIN_BABY_EAR_ANGLE
    } else {
        PIGLIN_ADULT_EAR_ANGLE
    }
}

/// Builds a leaf part at `pose` carrying `cubes`.
fn part(pose: PartPose, cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(pose, cubes.to_vec())
}

/// Builds an arm (or leg) part at `pose` carrying `cubes` plus its single inflated `overlay` child
/// (sleeve/pants) at the zero pose, mirroring `PlayerModel`'s nested overlay parts.
fn limb_with_overlay(
    pose: PartPose,
    cubes: &[ModelCube],
    overlay_name: &'static str,
    overlay: &[ModelCube],
) -> ModelPart {
    ModelPart::new(
        pose,
        cubes.to_vec(),
        vec![(overlay_name, part(PART_POSE_ZERO, overlay))],
    )
}

/// Builds the unified piglin root for `baby_layout`, with the vanilla `HumanoidModel` child names
/// (`head`, `body`, `right_arm`, `left_arm`, `right_leg`, `left_leg`). The adult head parents the two
/// ears directly (`left_ear`/`right_ear`); the baby head parents an empty `hat` plus the two flapping
/// ear holders (`left_ear`/`right_ear`), each carrying the rotated ear cube. The adult limbs nest the
/// inflated sleeve/pants overlays.
fn piglin_tree(baby_layout: bool) -> ModelPart {
    if baby_layout {
        let head = ModelPart::new(
            BABY_PIGLIN_HEAD_POSE,
            BABY_PIGLIN_HEAD.to_vec(),
            vec![
                ("hat", part(PART_POSE_ZERO, &[])),
                (
                    "left_ear",
                    ModelPart::new(
                        BABY_PIGLIN_LEFT_EAR_HOLDER_POSE,
                        Vec::new(),
                        vec![(
                            "ear",
                            part(BABY_PIGLIN_LEFT_EAR_POSE, &BABY_PIGLIN_LEFT_EAR),
                        )],
                    ),
                ),
                (
                    "right_ear",
                    ModelPart::new(
                        BABY_PIGLIN_RIGHT_EAR_HOLDER_POSE,
                        Vec::new(),
                        vec![(
                            "ear",
                            part(BABY_PIGLIN_RIGHT_EAR_POSE, &BABY_PIGLIN_RIGHT_EAR),
                        )],
                    ),
                ),
            ],
        );
        // Vanilla `BabyPiglinModel.createBodyLayer` lists the parts body-first (body, head, the two
        // arms, the two legs); the names make the layout order irrelevant to the animators but the
        // emit order is preserved to keep the mesh byte-identical.
        let children = vec![
            ("body", part(BABY_PIGLIN_BODY_POSE, &BABY_PIGLIN_BODY)),
            ("head", head),
            (
                "left_arm",
                part(BABY_PIGLIN_LEFT_ARM_POSE, &BABY_PIGLIN_LEFT_ARM),
            ),
            (
                "right_arm",
                part(BABY_PIGLIN_RIGHT_ARM_POSE, &BABY_PIGLIN_RIGHT_ARM),
            ),
            (
                "right_leg",
                part(BABY_PIGLIN_RIGHT_LEG_POSE, &BABY_PIGLIN_RIGHT_LEG),
            ),
            (
                "left_leg",
                part(BABY_PIGLIN_LEFT_LEG_POSE, &BABY_PIGLIN_LEFT_LEG),
            ),
        ];
        return ModelPart::new(PART_POSE_ZERO, Vec::new(), children);
    }
    let head = ModelPart::new(
        PART_POSE_ZERO,
        ADULT_PIGLIN_HEAD.to_vec(),
        vec![
            (
                "left_ear",
                part(ADULT_PIGLIN_LEFT_EAR_POSE, &ADULT_PIGLIN_LEFT_EAR),
            ),
            (
                "right_ear",
                part(ADULT_PIGLIN_RIGHT_EAR_POSE, &ADULT_PIGLIN_RIGHT_EAR),
            ),
        ],
    );
    let children = vec![
        ("head", head),
        ("body", part(PART_POSE_ZERO, &ADULT_PIGLIN_BODY)),
        (
            "right_arm",
            limb_with_overlay(
                ADULT_PIGLIN_RIGHT_ARM_POSE,
                &ADULT_PIGLIN_RIGHT_ARM,
                "sleeve",
                &ADULT_PIGLIN_RIGHT_SLEEVE,
            ),
        ),
        (
            "left_arm",
            limb_with_overlay(
                ADULT_PIGLIN_LEFT_ARM_POSE,
                &ADULT_PIGLIN_LEFT_ARM,
                "sleeve",
                &ADULT_PIGLIN_LEFT_SLEEVE,
            ),
        ),
        (
            "right_leg",
            limb_with_overlay(
                ADULT_PIGLIN_RIGHT_LEG_POSE,
                &ADULT_PIGLIN_RIGHT_LEG,
                "pants",
                &ADULT_PIGLIN_RIGHT_PANTS,
            ),
        ),
        (
            "left_leg",
            limb_with_overlay(
                ADULT_PIGLIN_LEFT_LEG_POSE,
                &ADULT_PIGLIN_LEFT_LEG,
                "pants",
                &ADULT_PIGLIN_LEFT_PANTS,
            ),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable piglin model, mirroring vanilla `AbstractPiglinModel extends HumanoidModel` (the piglin,
/// piglin brute, and zombified piglin). The unified tree is built for the `family`/`baby` layout with
/// the vanilla child names. `setup_anim` runs `super.setupAnim` — the head look ([`apply_head_look`]
/// on `head`) and the humanoid walk (leg + arm swing/bob, [`apply_humanoid_walk_named`]) — except the
/// zombified piglin keeps its arms at rest (the held-out `animateZombieArms` pose defers), so it swings
/// only the legs ([`apply_humanoid_leg_swing_named`]); then it always flaps the two ears
/// ([`piglin_ear_flap_pose`], head children). The family recolor/texture is supplied by the caller; the
/// dance/attack/crossbow/admire arm poses and held items defer.
pub(in crate::entity_models) struct PiglinModel {
    root: ModelPart,
    family: PiglinModelFamily,
    baby_layout: bool,
}

impl PiglinModel {
    pub(in crate::entity_models) fn new(family: PiglinModelFamily, baby: bool) -> Self {
        let baby_layout = piglin_baby_layout(family, baby);
        Self {
            root: piglin_tree(baby_layout),
            family,
            baby_layout,
        }
    }
}

impl EntityModel for PiglinModel {
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
        let age_in_ticks = render_state.age_in_ticks;
        apply_head_look(
            self.root.child_mut("head"),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        if self.family == PiglinModelFamily::ZombifiedPiglin {
            // The zombified piglin's held-out arms (deferred) replace the inherited swing, so only the
            // legs swing.
            apply_humanoid_leg_swing_named(&mut self.root, limb_swing, limb_swing_amount);
        } else {
            apply_humanoid_walk_named(&mut self.root, limb_swing, limb_swing_amount, age_in_ticks);
        }
        // Flap the two ears (head children) every frame.
        let default_ear_angle = piglin_default_ear_angle(self.baby_layout);
        let head = self.root.child_mut("head");
        let left = head.child_mut("left_ear");
        left.pose = piglin_ear_flap_pose(
            left.pose,
            true,
            default_ear_angle,
            age_in_ticks,
            limb_swing,
            limb_swing_amount,
        );
        let right = head.child_mut("right_ear");
        right.pose = piglin_ear_flap_pose(
            right.pose,
            false,
            default_ear_angle,
            age_in_ticks,
            limb_swing,
            limb_swing_amount,
        );
    }
}
