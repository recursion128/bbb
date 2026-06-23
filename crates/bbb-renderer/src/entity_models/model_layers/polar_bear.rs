use super::{
    apply_head_look, apply_quadruped_leg_swing_named, PartPose, PART_POSE_ZERO, POLAR_BEAR_WHITE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_POLAR_BEAR: &str = "minecraft:polar_bear#main";
pub(in crate::entity_models) const MODEL_LAYER_POLAR_BEAR_BABY: &str =
    "minecraft:polar_bear_baby#main";

// Vanilla 26.1 ModelLayers.POLAR_BEAR: PolarBearModel.createBodyLayer() (the `scaling(1.2)` lives in
// the root transform). Each cube carries both render paths' data: the colored debug tint and the
// textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_POLAR_BEAR_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-3.5, -3.0, -3.0],
        [7.0, 7.0, 7.0],
        POLAR_BEAR_WHITE,
        [7.0, 7.0, 7.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.5, 1.0, -6.0],
        [5.0, 3.0, 3.0],
        POLAR_BEAR_WHITE,
        [5.0, 3.0, 3.0],
        [0.0, 44.0],
        false,
    ),
    ModelCube::new(
        [-4.5, -4.0, -1.0],
        [2.0, 2.0, 1.0],
        POLAR_BEAR_WHITE,
        [2.0, 2.0, 1.0],
        [26.0, 0.0],
        false,
    ),
    ModelCube::new(
        [2.5, -4.0, -1.0],
        [2.0, 2.0, 1.0],
        POLAR_BEAR_WHITE,
        [2.0, 2.0, 1.0],
        [26.0, 0.0],
        true,
    ),
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -13.0, -7.0],
        [14.0, 14.0, 11.0],
        POLAR_BEAR_WHITE,
        [14.0, 14.0, 11.0],
        [0.0, 19.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -25.0, -7.0],
        [12.0, 12.0, 10.0],
        POLAR_BEAR_WHITE,
        [12.0, 12.0, 10.0],
        [39.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 10.0, 8.0],
    POLAR_BEAR_WHITE,
    [4.0, 10.0, 8.0],
    [50.0, 22.0],
    false,
)];

pub(in crate::entity_models) const ADULT_POLAR_BEAR_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 10.0, 6.0],
    POLAR_BEAR_WHITE,
    [4.0, 10.0, 6.0],
    [50.0, 40.0],
    false,
)];

/// The adult polar bear head/body part poses.
const ADULT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 10.0, -16.0],
    rotation: [0.0, 0.0, 0.0],
};

const ADULT_BODY_POSE: PartPose = PartPose {
    offset: [-2.0, 9.0, 12.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_POLAR_BEAR_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -3.5, -6.0],
    [8.0, 7.0, 12.0],
    POLAR_BEAR_WHITE,
    [8.0, 7.0, 12.0],
    [0.0, 9.0],
    false,
)];

pub(in crate::entity_models) const BABY_POLAR_BEAR_HEAD: [ModelCube; 4] = [
    ModelCube::new(
        [-3.0, -2.625, -4.25],
        [6.0, 5.0, 4.0],
        POLAR_BEAR_WHITE,
        [6.0, 5.0, 4.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 0.375, -6.25],
        [4.0, 2.0, 2.0],
        POLAR_BEAR_WHITE,
        [4.0, 2.0, 2.0],
        [20.0, 3.0],
        false,
    ),
    ModelCube::new(
        [-4.0, -3.625, -2.75],
        [2.0, 2.0, 1.0],
        POLAR_BEAR_WHITE,
        [2.0, 2.0, 1.0],
        [20.0, 0.0],
        false,
    ),
    ModelCube::new(
        [2.0, -3.625, -2.75],
        [2.0, 2.0, 1.0],
        POLAR_BEAR_WHITE,
        [2.0, 2.0, 1.0],
        [26.0, 0.0],
        false,
    ),
];

pub(in crate::entity_models) const BABY_POLAR_BEAR_RIGHT_HIND_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, -0.5, -1.5],
        [3.0, 3.0, 3.0],
        POLAR_BEAR_WHITE,
        [3.0, 3.0, 3.0],
        [0.0, 34.0],
        false,
    )];

pub(in crate::entity_models) const BABY_POLAR_BEAR_LEFT_HIND_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, -0.5, -1.5],
        [3.0, 3.0, 3.0],
        POLAR_BEAR_WHITE,
        [3.0, 3.0, 3.0],
        [12.0, 34.0],
        false,
    )];

pub(in crate::entity_models) const BABY_POLAR_BEAR_RIGHT_FRONT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, -0.5, -1.5],
        [3.0, 3.0, 3.0],
        POLAR_BEAR_WHITE,
        [3.0, 3.0, 3.0],
        [0.0, 28.0],
        false,
    )];

pub(in crate::entity_models) const BABY_POLAR_BEAR_LEFT_FRONT_LEG: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, -0.5, -1.5],
        [3.0, 3.0, 3.0],
        POLAR_BEAR_WHITE,
        [3.0, 3.0, 3.0],
        [12.0, 28.0],
        false,
    )];

/// Builds a leaf part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes.to_vec(),
    )
}

/// Builds a unified polar bear root for `baby` with the vanilla `QuadrupedModel` child names, keeping
/// the vanilla declaration order (adult: head, body, legs; baby: body, head, legs) so the render
/// order — and the textured atlas vertex ranges — stay byte-identical.
fn polar_bear_tree(baby: bool) -> ModelPart {
    let children = if baby {
        vec![
            ("body", leg([0.0, 17.5, 0.0], &BABY_POLAR_BEAR_BODY)),
            (
                "head",
                ModelPart::leaf(
                    PartPose {
                        offset: [0.0, 18.625, -5.75],
                        rotation: [0.0, 0.0, 0.0],
                    },
                    BABY_POLAR_BEAR_HEAD.to_vec(),
                ),
            ),
            (
                "right_hind_leg",
                leg([-2.5, 21.5, 4.5], &BABY_POLAR_BEAR_RIGHT_HIND_LEG),
            ),
            (
                "left_hind_leg",
                leg([2.5, 21.5, 4.5], &BABY_POLAR_BEAR_LEFT_HIND_LEG),
            ),
            (
                "right_front_leg",
                leg([-2.5, 21.5, -4.5], &BABY_POLAR_BEAR_RIGHT_FRONT_LEG),
            ),
            (
                "left_front_leg",
                leg([2.5, 21.5, -4.5], &BABY_POLAR_BEAR_LEFT_FRONT_LEG),
            ),
        ]
    } else {
        vec![
            (
                "head",
                ModelPart::leaf(ADULT_HEAD_POSE, ADULT_POLAR_BEAR_HEAD.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(ADULT_BODY_POSE, ADULT_POLAR_BEAR_BODY.to_vec()),
            ),
            (
                "right_hind_leg",
                leg([-4.5, 14.0, 6.0], &ADULT_POLAR_BEAR_HIND_LEG),
            ),
            (
                "left_hind_leg",
                leg([4.5, 14.0, 6.0], &ADULT_POLAR_BEAR_HIND_LEG),
            ),
            (
                "right_front_leg",
                leg([-3.5, 14.0, -8.0], &ADULT_POLAR_BEAR_FRONT_LEG),
            ),
            (
                "left_front_leg",
                leg([3.5, 14.0, -8.0], &ADULT_POLAR_BEAR_FRONT_LEG),
            ),
        ]
    };
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Which polar bear model part a standing-pose delta applies to. Vanilla
/// `PolarBearModel.setupAnim` rears the bear by moving the head, body, and both
/// front legs; the hind legs stay put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum PolarBearStandPart {
    Head,
    Body,
    FrontLeg,
}

/// The named parts the standing rear-up pose moves (head, body, both front legs), with their roles.
/// The swing/standing pose resolves by name, so this is the same for the adult and baby layouts.
pub(in crate::entity_models) const fn polar_bear_standing_part_roles(
) -> [(&'static str, PolarBearStandPart); 4] {
    [
        ("head", PolarBearStandPart::Head),
        ("body", PolarBearStandPart::Body),
        ("right_front_leg", PolarBearStandPart::FrontLeg),
        ("left_front_leg", PolarBearStandPart::FrontLeg),
    ]
}

/// Applies the vanilla `PolarBearModel.setupAnim` standing delta to one part
/// pose. `stand_scale` is `PolarBear.getStandingAnimationScale`; vanilla squares
/// it into `standScale`. `bodyAgeScale` (`state.ageScale`, 1.0 adult / 0.5 baby)
/// scales only the body and front-leg translation terms. Both front legs share
/// the same base y/z, so applying the delta to each matches vanilla's
/// `leftFrontLeg.y = rightFrontLeg.y` / `leftFrontLeg.z = rightFrontLeg.z`.
pub(in crate::entity_models) fn apply_polar_bear_standing_pose(
    pose: &mut PartPose,
    part: PolarBearStandPart,
    baby: bool,
    stand_scale: f32,
) {
    let scale = stand_scale * stand_scale;
    let body_age_scale = if baby { 0.5 } else { 1.0 };
    match part {
        PolarBearStandPart::Body => {
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.35;
            pose.offset[1] += scale * body_age_scale * 2.0;
        }
        PolarBearStandPart::FrontLeg => {
            pose.offset[1] -= scale * body_age_scale * 20.0;
            pose.offset[2] += scale * body_age_scale * 4.0;
            pose.rotation[0] -= scale * std::f32::consts::PI * 0.45;
        }
        PolarBearStandPart::Head => {
            pose.offset[1] -= scale * 24.0;
            pose.offset[2] += scale * 13.0;
            pose.rotation[0] += scale * std::f32::consts::PI * 0.15;
        }
    }
}

/// Mutable polar bear model, mirroring vanilla `PolarBearModel` (a `QuadrupedModel`). The unified tree
/// is built for the selected `baby` layout with the vanilla child names ([`polar_bear_tree`]).
/// `setup_anim` runs the `QuadrupedModel` head look ([`apply_head_look`] on `head`) and four-leg swing
/// ([`apply_quadruped_leg_swing_named`]), then — when standing — adds the rear-up deltas on top
/// ([`apply_polar_bear_standing_pose`] over [`polar_bear_standing_part_roles`]), driven by the
/// projected `standScale`. The per-size scale (adult only) lives in the root transform.
pub(in crate::entity_models) struct PolarBearModel {
    root: ModelPart,
    baby: bool,
}

impl PolarBearModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: polar_bear_tree(baby),
            baby,
        }
    }
}

impl EntityModel for PolarBearModel {
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
        apply_quadruped_leg_swing_named(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
        let stand_scale = render_state.polar_bear_stand_scale;
        if stand_scale != 0.0 {
            for (name, part) in polar_bear_standing_part_roles() {
                apply_polar_bear_standing_pose(
                    &mut self.root.child_mut(name).pose,
                    part,
                    self.baby,
                    stand_scale,
                );
            }
        }
    }
}
