use super::{
    apply_head_look, apply_quadruped_leg_swing, bind_part as part, bind_part_rot as rpart,
    model_cube as cube, ModelCubeDesc, ModelPartDesc, PANDA_BLACK, PANDA_WHITE,
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
// plain `MobRenderer` / `LivingEntityRenderer.setupRotations`.

// `head` cubes: the 13×10×9 skull and the 7×5×2 muzzle (white), plus the two 5×4×1 ears (black).
const PANDA_HEAD_CUBES: [ModelCubeDesc; 4] = [
    cube([-6.5, -5.0, -4.0], [13.0, 10.0, 9.0], PANDA_WHITE),
    cube([-3.5, 0.0, -6.0], [7.0, 5.0, 2.0], PANDA_WHITE),
    cube([3.5, -8.0, -1.0], [5.0, 4.0, 1.0], PANDA_BLACK),
    cube([-8.5, -8.0, -1.0], [5.0, 4.0, 1.0], PANDA_BLACK),
];

// `body`: the 19×26×13 trunk (pitched onto its belly), white.
const PANDA_BODY_CUBES: [ModelCubeDesc; 1] =
    [cube([-9.5, -13.0, -6.5], [19.0, 26.0, 13.0], PANDA_WHITE)];

// The shared 6×9×6 leg box (all four legs reuse it, differing only in pivot), black.
const PANDA_LEG_CUBES: [ModelCubeDesc; 1] = [cube([-3.0, 0.0, -3.0], [6.0, 9.0, 6.0], PANDA_BLACK)];

/// Vanilla `PandaModel.createBodyLayer` rest-pose hierarchy in `QuadrupedModel` order: `head` (0),
/// `body` (1, pitched `π/2`), then the right-hind / left-hind / right-front / left-front legs (2..=5).
/// Nine cubes.
pub(in crate::entity_models) const PANDA_PARTS: [ModelPartDesc; 6] = [
    part([0.0, 11.5, -17.0], &PANDA_HEAD_CUBES, &[]),
    rpart(
        [0.0, 10.0, 0.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        &PANDA_BODY_CUBES,
        &[],
    ),
    part([-5.5, 15.0, 9.0], &PANDA_LEG_CUBES, &[]),
    part([5.5, 15.0, 9.0], &PANDA_LEG_CUBES, &[]),
    part([-5.5, 15.0, -9.0], &PANDA_LEG_CUBES, &[]),
    part([5.5, 15.0, -9.0], &PANDA_LEG_CUBES, &[]),
];

/// The `head` is the first root part; the four legs are parts 2..=5 (the `QuadrupedModel` layout).
const PANDA_HEAD_PART_INDEX: usize = 0;
const PANDA_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// Mutable panda model, mirroring vanilla `PandaModel` (a `QuadrupedModel`). The six root parts hang off
/// a synthetic root, built from the baked [`PANDA_PARTS`] geometry. Colored-only: `setup_anim` runs the
/// shared `QuadrupedModel` head look ([`apply_head_look`]) and four-leg swing ([`apply_quadruped_leg_swing`]);
/// every panda-specific pose stays deferred.
pub(in crate::entity_models) struct PandaModel {
    root: ModelPart,
}

impl PandaModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::root_from_colored_descs(&PANDA_PARTS),
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
            self.root.child_at_mut(PANDA_HEAD_PART_INDEX),
            render_state.head_yaw,
            render_state.head_pitch,
        );
        apply_quadruped_leg_swing(
            &mut self.root,
            PANDA_LEG_PART_INDICES,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
        );
    }
}
