use super::{
    apply_head_look, apply_quadruped_leg_swing, model_cube as cube, ModelCubeDesc, PartPose,
    PANDA_BLACK, PANDA_WHITE, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `PandaModel.createBodyLayer` (atlas 64×64). `PandaModel extends QuadrupedModel`, so the
// six root parts follow the quadruped layout — `head` (carrying the muzzle and two ears), the pitched
// `body`, and the four legs (all sharing one 6×9×6 box) — and the base `QuadrupedModel.setupAnim` turns
// the head by the look angles and swings the four legs off the walk cycle. Every panda-specific pose in
// `PandaModel.setupAnim` (the `isUnhappy` head shake / leg paddle, the `isSneezing` head dip, the
// `sitAmount` sitting fold with its eating / scared variants, the `lieOnBackAmount` belly roll, and the
// `rollAmount` somersault) reads un-projected `PandaRenderState` fields / `AnimationState`s and stays
// deferred, so a resting panda renders at this bind pose plus the head look and leg swing. The black
// patches (eye rings, shoulders, legs) come from the deferred texture; the colored debug path uses a
// white body / head / muzzle and black ears / legs, the two tones the geometry separates. Panda uses a
// plain `MobRenderer` / `LivingEntityRenderer.setupRotations`. Colored-only: there is no textured path.

// `head` cubes: the 13×10×9 skull and the 7×5×2 muzzle (white), plus the two 5×4×1 ears (black).
pub(in crate::entity_models) const PANDA_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-6.5, -5.0, -4.0], [13.0, 10.0, 9.0], PANDA_WHITE),
    cube([-3.5, 0.0, -6.0], [7.0, 5.0, 2.0], PANDA_WHITE),
    cube([3.5, -8.0, -1.0], [5.0, 4.0, 1.0], PANDA_BLACK),
    cube([-8.5, -8.0, -1.0], [5.0, 4.0, 1.0], PANDA_BLACK),
];

// `body`: the 19×26×13 trunk (pitched onto its belly), white.
pub(in crate::entity_models) const PANDA_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-9.5, -13.0, -6.5], [19.0, 26.0, 13.0], PANDA_WHITE)];

// The shared 6×9×6 leg box (all four legs reuse it, differing only in pivot), black.
pub(in crate::entity_models) const PANDA_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-3.0, 0.0, -3.0], [6.0, 9.0, 6.0], PANDA_BLACK)];

/// The adult panda head/body part poses (vanilla `PandaModel.createBodyLayer`).
pub(in crate::entity_models) const PANDA_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 11.5, -17.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const PANDA_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 10.0, 0.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

// Vanilla `BabyPandaModel.createBodyLayer` (atlas 64×64). The `QuadrupedModel` baby convention lists the
// body FIRST then the head (so the head is part 1, not 0), and the baby body carries no `π/2` pitch.

// `head` cubes: the 7×6×5 skull and the 4×2×1 muzzle (white), plus the two 3×3×1 ears (black).
pub(in crate::entity_models) const BABY_PANDA_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-3.5, -3.0, -5.0], [7.0, 6.0, 5.0], PANDA_WHITE),
    cube([-2.0, 1.0, -6.0], [4.0, 2.0, 1.0], PANDA_WHITE),
    cube([-4.5, -4.0, -3.5], [3.0, 3.0, 1.0], PANDA_BLACK),
    cube([1.5, -4.0, -3.5], [3.0, 3.0, 1.0], PANDA_BLACK),
];

// `body`: the 9×7×11 trunk (no pitch on the baby), white.
pub(in crate::entity_models) const BABY_PANDA_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-4.5, -3.5, -5.5], [9.0, 7.0, 11.0], PANDA_WHITE)];

// The shared 3×2×3 baby leg box, black.
pub(in crate::entity_models) const BABY_PANDA_LEG_CUBES: [ModelCubeDesc; 1] =
    [cube([-1.5, 0.0, -1.5], [3.0, 2.0, 3.0], PANDA_BLACK)];

/// The baby panda head/body part poses (vanilla `BabyPandaModel.createBodyLayer`, body unpitched).
pub(in crate::entity_models) const BABY_PANDA_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.5, 2.5],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_PANDA_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a colored-only leg part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &'static [ModelCubeDesc]) -> ModelPart {
    ModelPart::leaf_colored(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes,
    )
}

/// Builds the four panda legs (hind-first, vanilla order) under the vanilla `QuadrupedModel` child
/// names. The adult and baby layouts share the same name set, differing only in pivot and cube.
fn panda_legs(
    cubes: &'static [ModelCubeDesc],
    offsets: [[f32; 3]; 4],
) -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_hind_leg", leg(offsets[0], cubes)),
        ("left_hind_leg", leg(offsets[1], cubes)),
        ("right_front_leg", leg(offsets[2], cubes)),
        ("left_front_leg", leg(offsets[3], cubes)),
    ]
}

/// Builds the unified panda tree for `baby`, keeping the vanilla declaration order (adult head-first,
/// baby body-first) so the colored render order stays byte-identical, under the vanilla child names.
fn panda_tree(baby: bool) -> ModelPart {
    let children = if baby {
        let mut children = vec![
            (
                "body",
                ModelPart::leaf_colored(BABY_PANDA_BODY_POSE, &BABY_PANDA_BODY_CUBES),
            ),
            (
                "head",
                ModelPart::leaf_colored(BABY_PANDA_HEAD_POSE, &BABY_PANDA_HEAD_CUBES),
            ),
        ];
        children.extend(panda_legs(
            &BABY_PANDA_LEG_CUBES,
            [
                [-3.0, 22.0, 6.5],
                [3.0, 22.0, 6.5],
                [-3.0, 22.0, -1.5],
                [3.0, 22.0, -1.5],
            ],
        ));
        children
    } else {
        let mut children = vec![
            (
                "head",
                ModelPart::leaf_colored(PANDA_HEAD_POSE, &PANDA_HEAD_CUBES),
            ),
            (
                "body",
                ModelPart::leaf_colored(PANDA_BODY_POSE, &PANDA_BODY_CUBES),
            ),
        ];
        children.extend(panda_legs(
            &PANDA_LEG_CUBES,
            [
                [-5.5, 15.0, 9.0],
                [5.5, 15.0, 9.0],
                [-5.5, 15.0, -9.0],
                [5.5, 15.0, -9.0],
            ],
        ));
        children
    };
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable panda model, mirroring vanilla `PandaModel` / `BabyPandaModel` (both `QuadrupedModel`s). The
/// unified tree is built for the selected `baby` layout with the vanilla child names. Colored-only:
/// `setup_anim` runs the shared `QuadrupedModel` head look ([`apply_head_look`] on `head`) and four-leg
/// swing ([`apply_quadruped_leg_swing`]); every panda-specific pose stays deferred.
pub(in crate::entity_models) struct PandaModel {
    root: ModelPart,
}

impl PandaModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: panda_tree(baby),
        }
    }
}

impl EntityModel for PandaModel {
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
        apply_quadruped_leg_swing(
            &mut self.root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
