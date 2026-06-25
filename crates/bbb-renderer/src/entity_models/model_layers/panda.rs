use super::{
    apply_head_look, apply_quadruped_leg_swing, PartPose, PANDA_BLACK, PANDA_WHITE, PART_POSE_ZERO,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `PandaModel.createBodyLayer` (atlas 64×64). `PandaModel extends QuadrupedModel`, so the
// six root parts follow the quadruped layout — `head` (carrying the muzzle and two ears), the pitched
// `body`, and the four legs (all sharing one 6×9×6 box) — and the base `QuadrupedModel.setupAnim` turns
// the head by the look angles and swings the four legs off the walk cycle. The `isUnhappy` head shake /
// leg paddle and the `isSneezing` head dip are projected and applied ([`apply_panda_emotes`]); the
// remaining `PandaModel.setupAnim` poses (the `sitAmount` sitting fold with its eating / scared variants,
// the `lieOnBackAmount` belly roll, and the `rollAmount` somersault) read un-projected client-tick
// interpolation `AnimationState`s and stay deferred, so an idle panda renders at this bind pose plus the
// head look and leg swing. The black
// patches (eye rings, shoulders, legs) come from the deferred texture; the colored debug path uses a
// white body / head / muzzle and black ears / legs, the two tones the geometry separates. Panda uses a
// plain `MobRenderer` / `LivingEntityRenderer.setupRotations`. Each unified cube carries both render
// paths' data: the colored tint and the textured `uv_size` / `texOffs` / `mirror`.

// `head` cubes: the 13×10×9 skull (white, texOffs(0,6)) and the 7×5×2 muzzle (white, texOffs(45,16)),
// plus the two 5×4×1 ears (black) that share one UV region texOffs(52,25) unmirrored (both ears sample
// identical pixels in vanilla).
pub(in crate::entity_models) const PANDA_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-6.5, -5.0, -4.0],
        [13.0, 10.0, 9.0],
        PANDA_WHITE,
        [13.0, 10.0, 9.0],
        [0.0, 6.0],
        false,
    ),
    ModelCube::new(
        [-3.5, 0.0, -6.0],
        [7.0, 5.0, 2.0],
        PANDA_WHITE,
        [7.0, 5.0, 2.0],
        [45.0, 16.0],
        false,
    ),
    ModelCube::new(
        [3.5, -8.0, -1.0],
        [5.0, 4.0, 1.0],
        PANDA_BLACK,
        [5.0, 4.0, 1.0],
        [52.0, 25.0],
        false,
    ),
    ModelCube::new(
        [-8.5, -8.0, -1.0],
        [5.0, 4.0, 1.0],
        PANDA_BLACK,
        [5.0, 4.0, 1.0],
        [52.0, 25.0],
        false,
    ),
];

// `body`: the 19×26×13 trunk (pitched onto its belly by the pose), white, texOffs(0,25).
pub(in crate::entity_models) const PANDA_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-9.5, -13.0, -6.5],
    [19.0, 26.0, 13.0],
    PANDA_WHITE,
    [19.0, 26.0, 13.0],
    [0.0, 25.0],
    false,
)];

// The shared 6×9×6 leg box (all four adult legs reuse one `CubeListBuilder`, texOffs(40,0), no mirror),
// black.
pub(in crate::entity_models) const PANDA_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, -3.0],
    [6.0, 9.0, 6.0],
    PANDA_BLACK,
    [6.0, 9.0, 6.0],
    [40.0, 0.0],
    false,
)];

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

// `head` cubes: the 7×6×5 skull (white, texOffs(0,0)) and the 4×2×1 muzzle (white, texOffs(24,6)), plus
// the two 3×3×1 ears (black) which — unlike the adult — carry DISTINCT UV regions: the -4.5 left-side ear
// at texOffs(24,0) and the +1.5 right-side ear at texOffs(33,0).
pub(in crate::entity_models) const BABY_PANDA_HEAD_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-3.5, -3.0, -5.0],
        [7.0, 6.0, 5.0],
        PANDA_WHITE,
        [7.0, 6.0, 5.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-2.0, 1.0, -6.0],
        [4.0, 2.0, 1.0],
        PANDA_WHITE,
        [4.0, 2.0, 1.0],
        [24.0, 6.0],
        false,
    ),
    ModelCube::new(
        [-4.5, -4.0, -3.5],
        [3.0, 3.0, 1.0],
        PANDA_BLACK,
        [3.0, 3.0, 1.0],
        [24.0, 0.0],
        false,
    ),
    ModelCube::new(
        [1.5, -4.0, -3.5],
        [3.0, 3.0, 1.0],
        PANDA_BLACK,
        [3.0, 3.0, 1.0],
        [33.0, 0.0],
        false,
    ),
];

// `body`: the 9×7×11 trunk (no pitch on the baby), white, texOffs(0,11).
pub(in crate::entity_models) const BABY_PANDA_BODY_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.5, -3.5, -5.5],
    [9.0, 7.0, 11.0],
    PANDA_WHITE,
    [9.0, 7.0, 11.0],
    [0.0, 11.0],
    false,
)];

// The baby 3×2×3 leg box, black. Unlike the adult (one shared `CubeListBuilder`), vanilla gives each baby
// leg its OWN `texOffs`, so the four legs need four distinct cube consts (geometry identical, UV differs):
// right_hind texOffs(0,34), left_hind texOffs(12,34), right_front texOffs(0,29), left_front texOffs(12,29).
pub(in crate::entity_models) const BABY_PANDA_RIGHT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 2.0, 3.0],
        PANDA_BLACK,
        [3.0, 2.0, 3.0],
        [0.0, 34.0],
        false,
    )];

pub(in crate::entity_models) const BABY_PANDA_LEFT_HIND_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 2.0, 3.0],
        PANDA_BLACK,
        [3.0, 2.0, 3.0],
        [12.0, 34.0],
        false,
    )];

pub(in crate::entity_models) const BABY_PANDA_RIGHT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 2.0, 3.0],
        PANDA_BLACK,
        [3.0, 2.0, 3.0],
        [0.0, 29.0],
        false,
    )];

pub(in crate::entity_models) const BABY_PANDA_LEFT_FRONT_LEG_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, 0.0, -1.5],
        [3.0, 2.0, 3.0],
        PANDA_BLACK,
        [3.0, 2.0, 3.0],
        [12.0, 29.0],
        false,
    )];

/// The baby panda head/body part poses (vanilla `BabyPandaModel.createBodyLayer`, body unpitched).
pub(in crate::entity_models) const BABY_PANDA_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 18.5, 2.5],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const BABY_PANDA_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, -3.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds a leaf leg part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes.to_vec(),
    )
}

/// Builds the four adult panda legs (hind-first, vanilla order) under the vanilla `QuadrupedModel` child
/// names. All four share the one 6×9×6 leg cube (texOffs(40,0)).
fn adult_panda_legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_hind_leg", leg([-5.5, 15.0, 9.0], &PANDA_LEG_CUBES)),
        ("left_hind_leg", leg([5.5, 15.0, 9.0], &PANDA_LEG_CUBES)),
        ("right_front_leg", leg([-5.5, 15.0, -9.0], &PANDA_LEG_CUBES)),
        ("left_front_leg", leg([5.5, 15.0, -9.0], &PANDA_LEG_CUBES)),
    ]
}

/// Builds the four baby panda legs (hind-first, vanilla order) under the vanilla `QuadrupedModel` child
/// names. Each carries its own per-leg UV (the four distinct baby leg cube consts).
fn baby_panda_legs() -> Vec<(&'static str, ModelPart)> {
    vec![
        (
            "right_hind_leg",
            leg([-3.0, 22.0, 6.5], &BABY_PANDA_RIGHT_HIND_LEG_CUBES),
        ),
        (
            "left_hind_leg",
            leg([3.0, 22.0, 6.5], &BABY_PANDA_LEFT_HIND_LEG_CUBES),
        ),
        (
            "right_front_leg",
            leg([-3.0, 22.0, -1.5], &BABY_PANDA_RIGHT_FRONT_LEG_CUBES),
        ),
        (
            "left_front_leg",
            leg([3.0, 22.0, -1.5], &BABY_PANDA_LEFT_FRONT_LEG_CUBES),
        ),
    ]
}

/// Builds the unified panda tree for `baby`, keeping the vanilla declaration order (adult head-first,
/// baby body-first) so the render order stays byte-identical, under the vanilla child names.
fn panda_tree(baby: bool) -> ModelPart {
    let children = if baby {
        let mut children = vec![
            (
                "body",
                ModelPart::leaf(BABY_PANDA_BODY_POSE, BABY_PANDA_BODY_CUBES.to_vec()),
            ),
            (
                "head",
                ModelPart::leaf(BABY_PANDA_HEAD_POSE, BABY_PANDA_HEAD_CUBES.to_vec()),
            ),
        ];
        children.extend(baby_panda_legs());
        children
    } else {
        let mut children = vec![
            (
                "head",
                ModelPart::leaf(PANDA_HEAD_POSE, PANDA_HEAD_CUBES.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(PANDA_BODY_POSE, PANDA_BODY_CUBES.to_vec()),
            ),
        ];
        children.extend(adult_panda_legs());
        children
    };
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Mutable panda model, mirroring vanilla `PandaModel` / `BabyPandaModel` (both `QuadrupedModel`s). The
/// unified tree is built for the selected `baby` layout with the vanilla child names. `setup_anim` runs
/// the shared `QuadrupedModel` head look ([`apply_head_look`] on `head`) and four-leg swing
/// ([`apply_quadruped_leg_swing`]); every panda-specific pose stays deferred.
/// Vanilla `PandaModel.setupAnim`'s `isUnhappy` and `isSneezing` branches, applied after the inherited
/// `QuadrupedModel` head look + four-leg swing. An unhappy panda shakes its head (`yRot = zRot =
/// 0.35·sin(0.6·age)`, overwriting the look yaw) and paddles its front legs (`xRot = ∓0.75·sin(0.3·age)`,
/// overwriting the walk swing); a sneezing panda dips its head (`xRot` ramps to `-π/4` over `sneezeTime`
/// 0..14, then holds at `-π/4` for 15..19 — vanilla's `(sneezeTime-15)/5` is integer division, so the
/// "ease back" term is always 0). Both run on the reset bind pose, so the resting (`else`) head `zRot`
/// stays 0 with no action.
fn apply_panda_emotes(
    root: &mut ModelPart,
    unhappy: bool,
    sneezing: bool,
    sneeze_time: i32,
    age_in_ticks: f32,
) {
    use std::f32::consts::PI;
    if unhappy {
        let head_shake = 0.35 * (0.6 * age_in_ticks).sin();
        {
            let head = root.child_mut("head");
            head.pose.rotation[1] = head_shake;
            head.pose.rotation[2] = head_shake;
        }
        let paddle = 0.75 * (0.3 * age_in_ticks).sin();
        root.child_mut("right_front_leg").pose.rotation[0] = -paddle;
        root.child_mut("left_front_leg").pose.rotation[0] = paddle;
    }
    if sneezing {
        let head = root.child_mut("head");
        if sneeze_time < 15 {
            head.pose.rotation[0] = -PI / 4.0 * sneeze_time as f32 / 14.0;
        } else if sneeze_time < 20 {
            head.pose.rotation[0] = -PI / 4.0;
        }
    }
}

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
        // Vanilla `PandaModel.setupAnim` overrides the head/front legs for the unhappy shake and the
        // sneeze head dip after the inherited quadruped pose.
        apply_panda_emotes(
            &mut self.root,
            render_state.panda_unhappy,
            render_state.panda_sneezing,
            render_state.panda_sneeze_time,
            render_state.age_in_ticks,
        );
    }
}
