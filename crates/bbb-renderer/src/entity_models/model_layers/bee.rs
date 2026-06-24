use super::{PartPose, BEE_YELLOW, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

use std::f32::consts::PI;

// Vanilla 26.1 `AdultBeeModel.createBodyLayer` (atlas 64×64). The empty `bone` pivot parents the
// body (which carries the stinger and the two antennae), the two wings, and the three leg planes.
// Each cube carries both render paths' data: the colored debug tint (`BEE_YELLOW`, a single
// representative yellow approximating the striped texture) and the textured `uv_size` / `texOffs` /
// `mirror`. The wings keep the BASE box `uv_size` (`[9, 0, 6]`) while the geometry inflates by the
// vanilla `CubeDeformation(0.001)` (`min -= 0.001` / `size += 0.002`).
pub(in crate::entity_models) const BEE_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.5, -4.0, -5.0],
    [7.0, 7.0, 10.0],
    BEE_YELLOW,
    [7.0, 7.0, 10.0],
    [0.0, 0.0],
    false,
)];

// The stinger is a zero-thickness plane (x size 0).
pub(in crate::entity_models) const BEE_STINGER: [ModelCube; 1] = [ModelCube::new(
    [0.0, -1.0, 5.0],
    [0.0, 1.0, 2.0],
    BEE_YELLOW,
    [0.0, 1.0, 2.0],
    [26.0, 7.0],
    false,
)];

pub(in crate::entity_models) const BEE_LEFT_ANTENNA: [ModelCube; 1] = [ModelCube::new(
    [1.5, -2.0, -3.0],
    [1.0, 2.0, 3.0],
    BEE_YELLOW,
    [1.0, 2.0, 3.0],
    [2.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BEE_RIGHT_ANTENNA: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -2.0, -3.0],
    [1.0, 2.0, 3.0],
    BEE_YELLOW,
    [1.0, 2.0, 3.0],
    [2.0, 3.0],
    false,
)];

// The wings are zero-height planes inflated by the vanilla `CubeDeformation(0.001)`, so the colored
// box bakes `min -= 0.001` / `size += 0.002` while the `uv_size` keeps the BASE box. Both share
// `texOffs(0, 18)`; only the left wing's UV is mirrored.
pub(in crate::entity_models) const BEE_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [-9.001, -0.001, -0.001],
    [9.002, 0.002, 6.002],
    BEE_YELLOW,
    [9.0, 0.0, 6.0],
    [0.0, 18.0],
    false,
)];

pub(in crate::entity_models) const BEE_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [-0.001, -0.001, -0.001],
    [9.002, 0.002, 6.002],
    BEE_YELLOW,
    [9.0, 0.0, 6.0],
    [0.0, 18.0],
    true,
)];

// The three leg pairs are zero-depth planes (z size 0).
pub(in crate::entity_models) const BEE_FRONT_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-5.0, 0.0, 0.0],
    [7.0, 2.0, 0.0],
    BEE_YELLOW,
    [7.0, 2.0, 0.0],
    [26.0, 1.0],
    false,
)];

pub(in crate::entity_models) const BEE_MIDDLE_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-5.0, 0.0, 0.0],
    [7.0, 2.0, 0.0],
    BEE_YELLOW,
    [7.0, 2.0, 0.0],
    [26.0, 3.0],
    false,
)];

pub(in crate::entity_models) const BEE_BACK_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-5.0, 0.0, 0.0],
    [7.0, 2.0, 0.0],
    BEE_YELLOW,
    [7.0, 2.0, 0.0],
    [26.0, 5.0],
    false,
)];

pub(in crate::entity_models) const BEE_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_STINGER_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_LEFT_ANTENNA_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_RIGHT_ANTENNA_POSE: PartPose = PartPose {
    offset: [0.0, -2.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.5, -4.0, -3.0],
    rotation: [0.0, -0.2618, 0.0],
};
pub(in crate::entity_models) const BEE_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.5, -4.0, -3.0],
    rotation: [0.0, 0.2618, 0.0],
};
pub(in crate::entity_models) const BEE_FRONT_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_MIDDLE_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BACK_LEGS_POSE: PartPose = PartPose {
    offset: [1.5, 3.0, 2.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BabyBeeModel.createBodyLayer` (atlas 32×32). The `bone` pivot itself carries two
// small cubes; there are no antennae, and the wings sit at a different bind rotation. Each cube
// carries both render paths' data (no `CubeDeformation`, so each `uv_size` matches its box `size`).
// The left wing carries the vanilla negative `texOffs(-3, 9)` with a mirrored box.
pub(in crate::entity_models) const BEE_BABY_BONE: [ModelCube; 2] = [
    ModelCube::new(
        [1.0, -1.6667, -2.1633],
        [1.0, 2.0, 2.0],
        BEE_YELLOW,
        [1.0, 2.0, 2.0],
        [6.0, 12.0],
        false,
    ),
    ModelCube::new(
        [-2.0, -1.6667, -2.1933],
        [1.0, 2.0, 2.0],
        BEE_YELLOW,
        [1.0, 2.0, 2.0],
        [0.0, 12.0],
        false,
    ),
];

pub(in crate::entity_models) const BEE_BABY_BODY: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -2.0, -2.5],
    [4.0, 4.0, 5.0],
    BEE_YELLOW,
    [4.0, 4.0, 5.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_STINGER: [ModelCube; 1] = [ModelCube::new(
    [0.0, -0.5, 0.0],
    [0.0, 1.0, 1.0],
    BEE_YELLOW,
    [0.0, 1.0, 1.0],
    [13.0, 2.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_RIGHT_WING: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, 0.0],
    [3.0, 0.0, 3.0],
    BEE_YELLOW,
    [3.0, 0.0, 3.0],
    [3.0, 9.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_LEFT_WING: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [3.0, 0.0, 3.0],
    BEE_YELLOW,
    [3.0, 0.0, 3.0],
    [-3.0, 9.0],
    true,
)];

pub(in crate::entity_models) const BEE_BABY_FRONT_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, 0.0],
    [3.0, 1.0, 0.0],
    BEE_YELLOW,
    [3.0, 1.0, 0.0],
    [13.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_MIDDLE_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, 0.0],
    [3.0, 1.0, 0.0],
    BEE_YELLOW,
    [3.0, 1.0, 0.0],
    [13.0, 1.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_BACK_LEGS: [ModelCube; 1] = [ModelCube::new(
    [-1.5, 0.0, 0.0],
    [3.0, 1.0, 0.0],
    BEE_YELLOW,
    [3.0, 1.0, 0.0],
    [13.0, 2.0],
    false,
)];

pub(in crate::entity_models) const BEE_BABY_BONE_POSE: PartPose = PartPose {
    offset: [0.0, 19.6667, -1.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 1.3333, 2.3567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_STINGER_POSE: PartPose = PartPose {
    offset: [0.0, 0.5, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_RIGHT_WING_POSE: PartPose = PartPose {
    offset: [-1.0, -0.6667, 0.8567],
    rotation: [0.2182, 0.3491, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_LEFT_WING_POSE: PartPose = PartPose {
    offset: [1.0, -0.6667, 0.8567],
    rotation: [0.2182, -0.3491, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_FRONT_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 1.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_MIDDLE_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 2.8567],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const BEE_BABY_BACK_LEGS_POSE: PartPose = PartPose {
    offset: [0.0, 3.3333, 3.8567],
    rotation: [0.0, 0.0, 0.0],
};

// The flying middle-leg angle (vanilla sets all three legs to `π/4` in the airborne branch before
// the bob overrides the front/back pair).
pub(in crate::entity_models) const BEE_MID_LEG_FLYING_X_ROT: f32 = PI / 4.0;

/// Vanilla `BeeModel.setupAnim` wing flap: `zRot = cos(ageInTicks · 120.32113°) · π · 0.15`. The
/// left wing mirrors this (`leftWing.zRot = -rightWing.zRot`).
pub(in crate::entity_models) fn bee_wing_z_rot(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 120.32113 * (PI / 180.0)).cos() * PI * 0.15
}

/// The shared `bobUpAndDown` speed term: `cos(ageInTicks · 0.18)`.
pub(in crate::entity_models) fn bee_bob_speed(age_in_ticks: f32) -> f32 {
    (age_in_ticks * 0.18).cos()
}

/// `bone.xRot = 0.1 + speed · π · 0.025`.
pub(in crate::entity_models) fn bee_bone_x_rot(age_in_ticks: f32) -> f32 {
    0.1 + bee_bob_speed(age_in_ticks) * PI * 0.025
}

/// The vertical bob added to the bone pivot: `bone.y -= cos(ageInTicks · 0.18) · 0.9`.
pub(in crate::entity_models) fn bee_bone_y_delta(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * 0.9
}

/// `frontLeg.xRot = -speed · π · 0.1 + π/8`.
pub(in crate::entity_models) fn bee_front_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.1 + PI / 8.0
}

/// `backLeg.xRot = -speed · π · 0.05 + π/4`.
pub(in crate::entity_models) fn bee_back_leg_x_rot(age_in_ticks: f32) -> f32 {
    -bee_bob_speed(age_in_ticks) * PI * 0.05 + PI / 4.0
}

/// Adult-only antenna bob (`AdultBeeModel.bobUpAndDown`): `antenna.xRot = speed · π · 0.03`.
pub(in crate::entity_models) fn bee_antenna_x_rot(age_in_ticks: f32) -> f32 {
    bee_bob_speed(age_in_ticks) * PI * 0.03
}

/// Builds the adult bee's `bone` pivot subtree (`AdultBeeModel.createBodyLayer`): the empty `bone`
/// parents the body (which carries the stinger and the two antennae), the two wings, and the three
/// leg planes — in the vanilla emit order, preserved for byte-identical meshes. Each cube carries
/// both the colored tint and the textured UV, so one tree drives both render paths.
fn bee_adult_bone() -> ModelPart {
    let body = ModelPart::new(
        BEE_BODY_POSE,
        BEE_BODY.to_vec(),
        vec![
            (
                "stinger",
                ModelPart::leaf(BEE_STINGER_POSE, BEE_STINGER.to_vec()),
            ),
            (
                "left_antenna",
                ModelPart::leaf(BEE_LEFT_ANTENNA_POSE, BEE_LEFT_ANTENNA.to_vec()),
            ),
            (
                "right_antenna",
                ModelPart::leaf(BEE_RIGHT_ANTENNA_POSE, BEE_RIGHT_ANTENNA.to_vec()),
            ),
        ],
    );
    ModelPart::new(
        BEE_BONE_POSE,
        Vec::new(),
        vec![
            ("body", body),
            (
                "right_wing",
                ModelPart::leaf(BEE_RIGHT_WING_POSE, BEE_RIGHT_WING.to_vec()),
            ),
            (
                "left_wing",
                ModelPart::leaf(BEE_LEFT_WING_POSE, BEE_LEFT_WING.to_vec()),
            ),
            (
                "front_legs",
                ModelPart::leaf(BEE_FRONT_LEGS_POSE, BEE_FRONT_LEGS.to_vec()),
            ),
            (
                "middle_legs",
                ModelPart::leaf(BEE_MIDDLE_LEGS_POSE, BEE_MIDDLE_LEGS.to_vec()),
            ),
            (
                "back_legs",
                ModelPart::leaf(BEE_BACK_LEGS_POSE, BEE_BACK_LEGS.to_vec()),
            ),
        ],
    )
}

/// Builds the baby bee's `bone` pivot subtree (`BabyBeeModel.createBodyLayer`): the `bone` itself
/// carries two small cubes, the body has only the stinger (no antennae), and the wings/legs sit at
/// the baby binds — in the vanilla emit order.
fn bee_baby_bone() -> ModelPart {
    let body = ModelPart::new(
        BEE_BABY_BODY_POSE,
        BEE_BABY_BODY.to_vec(),
        vec![(
            "stinger",
            ModelPart::leaf(BEE_BABY_STINGER_POSE, BEE_BABY_STINGER.to_vec()),
        )],
    );
    ModelPart::new(
        BEE_BABY_BONE_POSE,
        BEE_BABY_BONE.to_vec(),
        vec![
            ("body", body),
            (
                "right_wing",
                ModelPart::leaf(BEE_BABY_RIGHT_WING_POSE, BEE_BABY_RIGHT_WING.to_vec()),
            ),
            (
                "left_wing",
                ModelPart::leaf(BEE_BABY_LEFT_WING_POSE, BEE_BABY_LEFT_WING.to_vec()),
            ),
            (
                "front_legs",
                ModelPart::leaf(BEE_BABY_FRONT_LEGS_POSE, BEE_BABY_FRONT_LEGS.to_vec()),
            ),
            (
                "middle_legs",
                ModelPart::leaf(BEE_BABY_MIDDLE_LEGS_POSE, BEE_BABY_MIDDLE_LEGS.to_vec()),
            ),
            (
                "back_legs",
                ModelPart::leaf(BEE_BABY_BACK_LEGS_POSE, BEE_BABY_BACK_LEGS.to_vec()),
            ),
        ],
    )
}

/// Applies vanilla `BeeModel.setupAnim` to the unified tree. While airborne the wings flap on
/// `ageInTicks` and (when not angry) `bobUpAndDown` rocks the `bone` pivot, the front/back legs and —
/// on adults — the antennae; all three legs first splay to `π/4`, so the middle leg holds that angle
/// while the bob overrides the front/back pair. On the ground the model rests at its bind pose. The
/// stinger cube is hidden once the bee has stung (`stinger.visible = hasStinger`).
fn apply_bee_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let age = instance.render_state.age_in_ticks;
    let flying = !instance.render_state.on_ground;
    // Vanilla gates `bobUpAndDown` on `!isAngry && !isOnGround`: an angry airborne bee still flaps
    // its wings and splays its legs to `π/4`, but its body, front/back legs and antennae hold still.
    let bob = flying && !instance.render_state.bee_angry;
    let has_stinger = instance.render_state.bee_has_stinger;

    // Bone pivot (root child): the airborne bob rocks it forward and lifts/drops it.
    let bone = root.child_mut("bone");
    if bob {
        bone.pose.offset[1] += bee_bone_y_delta(age);
        bone.pose.rotation = [bee_bone_x_rot(age), 0.0, 0.0];
    }

    // Body subtree: the stinger is shown only while carried; the adult antennae bob with the body.
    {
        let body = bone.child_mut("body");
        body.child_mut("stinger").visible = has_stinger;
        if !baby {
            let antenna_x_rot = if bob { bee_antenna_x_rot(age) } else { 0.0 };
            body.child_mut("left_antenna").pose.rotation = [antenna_x_rot, 0.0, 0.0];
            body.child_mut("right_antenna").pose.rotation = [antenna_x_rot, 0.0, 0.0];
        }
    }

    // Wings (bone children): the flap overrides the bind yaw to 0 and drives `zRot`, mirrored on the
    // left, while the bind pitch (0 on adults, `0.2182` on babies) is preserved.
    if flying {
        let wing_z_rot = bee_wing_z_rot(age);
        let right_wing = bone.child_mut("right_wing");
        right_wing.pose.rotation = [right_wing.pose.rotation[0], 0.0, wing_z_rot];
        let left_wing = bone.child_mut("left_wing");
        left_wing.pose.rotation = [left_wing.pose.rotation[0], 0.0, -wing_z_rot];
    }

    // Legs (bone children): airborne, all three splay to `π/4`; the non-angry bob then overrides the
    // front/back pair, while an angry bee holds all three at `π/4`. On the ground they rest at `0`.
    let (front_x, mid_x, back_x) = if flying {
        (
            if bob {
                bee_front_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
            BEE_MID_LEG_FLYING_X_ROT,
            if bob {
                bee_back_leg_x_rot(age)
            } else {
                BEE_MID_LEG_FLYING_X_ROT
            },
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    bone.child_mut("front_legs").pose.rotation = [front_x, 0.0, 0.0];
    bone.child_mut("middle_legs").pose.rotation = [mid_x, 0.0, 0.0];
    bone.child_mut("back_legs").pose.rotation = [back_x, 0.0, 0.0];

    // Vanilla `BeeModel.setupAnim` applies the barrel roll last: a rolling bee tips its `bone` pivot
    // onto its back by interpolating the (already bob-posed) pitch toward `3.0915928` (≈ π − 0.05).
    let roll = instance.render_state.bee_roll_amount;
    if roll > 0.0 {
        bone.pose.rotation[0] = rot_lerp_rad(roll, bone.pose.rotation[0], 3.0915928);
    }
}

/// Vanilla `Mth.rotLerpRad(a, from, to)`: lerps `from` toward `to` along the shortest signed angular
/// path (the `to - from` delta is wrapped into `[-π, π)` before scaling by `a`).
fn rot_lerp_rad(a: f32, from: f32, to: f32) -> f32 {
    let mut diff = to - from;
    while diff < -PI {
        diff += 2.0 * PI;
    }
    while diff >= PI {
        diff -= 2.0 * PI;
    }
    from + a * diff
}

/// Mutable bee model, mirroring vanilla `AdultBeeModel` / `BabyBeeModel`. The unified tree is built
/// once with named children: a synthetic root parenting the `bone` pivot selected by `baby`
/// ([`bee_adult_bone`] / [`bee_baby_bone`]), with the `bone` carrying the body (→ `stinger`, and on
/// adults the two antennae), the two wings, and the three leg planes, in the emit order (preserved
/// for byte-identical meshes). Each cube carries both the colored tint and the textured UV, so one
/// tree drives both render paths; `setup_anim` runs [`apply_bee_anim`]. The same posed tree drives
/// the colored fallback and the cutout textured layer; the adult/baby texture and the rolled-up fall
/// pose (`rollAmount`) live outside the model.
pub(in crate::entity_models) struct BeeModel {
    root: ModelPart,
    baby: bool,
}

impl BeeModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let bone = if baby {
            bee_baby_bone()
        } else {
            bee_adult_bone()
        };
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("bone", bone)]),
            baby,
        }
    }
}

impl EntityModel for BeeModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_bee_anim(&mut self.root, self.baby, instance);
    }
}
