use super::{apply_head_look, limb_swing_at_rest};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `AdultFelineModel.setupAnim` standing tail droop: while not sitting it sets
/// `tail2.xRot = 1.7278761` (`= π·0.55`), the base the walk wobble adds onto. At rest (zero walk
/// speed) the wobble term collapses to zero, so this is the resting `tail2` pitch — a real change
/// from the `0` bind rotation.
pub(in crate::entity_models) const FELINE_TAIL2_REST_X_ROT: f32 = 1.7278761;
const FELINE_TAIL2_CROUCH_WOBBLE_AMPLITUDE: f32 = 0.47123894;
pub(in crate::entity_models) const FELINE_SITTING_TAIL2_X_ROT: f32 = 2.670354;
pub(in crate::entity_models) const BABY_FELINE_SITTING_BODY_X_ROT_DELTA: f32 = -0.43633232;
pub(in crate::entity_models) const BABY_FELINE_SITTING_TAIL1_X_ROT_DELTA: f32 = 0.5454154;
pub(in crate::entity_models) const FELINE_RELAX_HEAD_X_ROT: f32 = -0.58177644;
pub(in crate::entity_models) const ADULT_FELINE_LIE_HEAD_Z_ROT: f32 = -1.2707963;
pub(in crate::entity_models) const ADULT_FELINE_LIE_HEAD_Y_ROT: f32 = 1.2707963;
pub(in crate::entity_models) const ADULT_FELINE_LIE_LEFT_FRONT_LEG_X_ROT: f32 = -1.2707963;
pub(in crate::entity_models) const ADULT_FELINE_LIE_RIGHT_FRONT_LEG_X_ROT: f32 = -0.47079635;

/// Vanilla `AdultFelineModel.setupAnim` lower-tail walk wobble while not sitting:
/// `tail2.xRot = 1.7278761 + amplitude·cos(walkAnimationPos)·walkAnimationSpeed`.
/// The base droop [`FELINE_TAIL2_REST_X_ROT`] is the same constant the [`setup_anim`] standing droop
/// applies; the wobble is a pure function of the projected `walk_animation_pos` /
/// `walk_animation_speed`. The standing, crouch, and sprint branches keep the same base droop and only
/// swap the wobble amplitude (`π/4`, `0.47123894`, and `π/10`).
fn feline_tail2_wobble_x_rot(
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    is_crouching: bool,
    is_sprinting: bool,
) -> f32 {
    let amplitude = if is_sprinting {
        std::f32::consts::PI / 10.0
    } else if is_crouching {
        FELINE_TAIL2_CROUCH_WOBBLE_AMPLITUDE
    } else {
        std::f32::consts::FRAC_PI_4
    };
    FELINE_TAIL2_REST_X_ROT + amplitude * walk_animation_pos.cos() * walk_animation_speed
}

/// Vanilla `AdultFelineModel` / `BabyFelineModel`, shared by the cat and ocelot layers and the cat
/// collar layers. `CatRenderer.setupRotations` lie-down roll lives in the feline root transform.
pub(in crate::entity_models) fn feline_model(
    layer: ModelLayer,
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    apply_feline_crouch_or_sprint_pose(
        &mut root,
        render_state.kind.feline().is_crouching,
        render_state.kind.feline().is_sprinting,
        render_state.age_scale,
    );
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    if render_state.kind.feline().is_sitting {
        apply_feline_sitting_pose(&mut root, baby, render_state.age_scale);
    } else {
        // Vanilla's not-sitting branch drops the lower tail to `1.7278761` and then adds the
        // branch-specific standing/crouch/sprint wobble. At rest the wobble collapses to zero,
        // leaving the standing droop. The baby's `tail2` is cubeless, so vanilla's identical
        // assignment there is invisible; we skip it.
        if !baby {
            root.child_mut("tail2").pose.rotation[0] = feline_tail2_wobble_x_rot(
                render_state.walk_animation_pos,
                render_state.walk_animation_speed,
                render_state.kind.feline().is_crouching,
                render_state.kind.feline().is_sprinting,
            );
        }
        // The four legs sweep only in vanilla's not-sitting branch.
        apply_feline_leg_swing(
            &mut root,
            render_state.walk_animation_pos,
            render_state.walk_animation_speed,
            render_state.kind.feline().is_sprinting,
        );
    }
    apply_feline_lie_down_pose(
        &mut root,
        baby,
        render_state.kind.feline().lie_down_amount,
        render_state.kind.feline().lie_down_amount_tail,
        render_state.age_scale,
    );
    apply_feline_relax_pose(&mut root, render_state.kind.feline().relax_state_one_amount);
    root
}

/// Vanilla `AdultFelineModel`/`BabyFelineModel.setupAnim` walk leg swing: each leg's `xRot = cos(pos·
/// 0.6662 [+ π]) · 1.0 · speed`. The feline uses the MIRROR of the `QuadrupedModel` diagonal at the
/// shorter `1.0` amplitude (vs the `1.4` rule) — the left-hind & right-front legs swing in phase and the
/// right-hind & left-front a half-cycle out (the opposite of the standard right-hind/left-front pairing),
/// keyed by leg NAME. The base leg pose carries no `xRot`, so it is set (not accumulated). A no-op while
/// at rest. The `tail2` walk wobble vanilla adds on top is applied by [`feline_tail2_wobble_x_rot`].
fn apply_feline_leg_swing(
    root: &mut ModelPart,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    is_sprinting: bool,
) {
    if limb_swing_at_rest(walk_animation_speed) {
        return;
    }
    let phase = walk_animation_pos * 0.6662;
    let phases = if is_sprinting {
        [
            ("left_hind_leg", 0.0),
            ("right_hind_leg", 0.3),
            ("left_front_leg", std::f32::consts::PI + 0.3),
            ("right_front_leg", std::f32::consts::PI),
        ]
    } else {
        [
            ("left_hind_leg", 0.0),
            ("right_hind_leg", std::f32::consts::PI),
            ("left_front_leg", std::f32::consts::PI),
            ("right_front_leg", 0.0),
        ]
    };
    for (name, phase_offset) in phases {
        root.child_mut(name).pose.rotation[0] = (phase + phase_offset).cos() * walk_animation_speed;
    }
}

fn apply_feline_crouch_or_sprint_pose(
    root: &mut ModelPart,
    is_crouching: bool,
    is_sprinting: bool,
    age_scale: f32,
) {
    if is_crouching {
        root.child_mut("body").pose.offset[1] += age_scale;
        root.child_mut("head").pose.offset[1] += 2.0 * age_scale;
        root.child_mut("tail1").pose.offset[1] += age_scale;
        root.child_mut("tail2").pose.offset[1] -= 4.0 * age_scale;
        root.child_mut("tail2").pose.offset[2] += 2.0 * age_scale;
        root.child_mut("tail1").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
        root.child_mut("tail2").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
    } else if is_sprinting {
        let tail1_y = root.child_mut("tail1").pose.offset[1];
        root.child_mut("tail2").pose.offset[1] = tail1_y;
        root.child_mut("tail2").pose.offset[2] += 2.0 * age_scale;
        root.child_mut("tail1").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
        root.child_mut("tail2").pose.rotation[0] = std::f32::consts::FRAC_PI_2;
    }
}

/// Vanilla `AdultFelineModel.setupAnim` sitting branch, with every translated proportion multiplied
/// by the projected `state.ageScale`.
fn apply_adult_feline_sitting_pose(root: &mut ModelPart, age_scale: f32) {
    root.child_mut("body").pose.rotation[0] = std::f32::consts::FRAC_PI_4;
    root.child_mut("body").pose.offset[1] -= 4.0 * age_scale;
    root.child_mut("body").pose.offset[2] += 5.0 * age_scale;
    root.child_mut("head").pose.offset[1] -= 3.3 * age_scale;
    root.child_mut("head").pose.offset[2] += age_scale;
    root.child_mut("tail1").pose.offset[1] += 8.0 * age_scale;
    root.child_mut("tail1").pose.offset[2] -= 2.0 * age_scale;
    root.child_mut("tail2").pose.offset[1] += 2.0 * age_scale;
    root.child_mut("tail2").pose.offset[2] -= 0.8 * age_scale;
    root.child_mut("tail1").pose.rotation[0] = FELINE_TAIL2_REST_X_ROT;
    root.child_mut("tail2").pose.rotation[0] = FELINE_SITTING_TAIL2_X_ROT;
    root.child_mut("left_front_leg").pose.rotation[0] = -std::f32::consts::PI / 20.0;
    root.child_mut("left_front_leg").pose.offset[1] += 2.0 * age_scale;
    root.child_mut("left_front_leg").pose.offset[2] -= 2.0 * age_scale;
    root.child_mut("right_front_leg").pose.rotation[0] = -std::f32::consts::PI / 20.0;
    root.child_mut("right_front_leg").pose.offset[1] += 2.0 * age_scale;
    root.child_mut("right_front_leg").pose.offset[2] -= 2.0 * age_scale;
    root.child_mut("left_hind_leg").pose.rotation[0] = -std::f32::consts::FRAC_PI_2;
    root.child_mut("left_hind_leg").pose.offset[1] += 3.0 * age_scale;
    root.child_mut("left_hind_leg").pose.offset[2] -= 4.0 * age_scale;
    root.child_mut("right_hind_leg").pose.rotation[0] = -std::f32::consts::FRAC_PI_2;
    root.child_mut("right_hind_leg").pose.offset[1] += 3.0 * age_scale;
    root.child_mut("right_hind_leg").pose.offset[2] -= 4.0 * age_scale;
}

/// Vanilla `BabyFelineModel.setupAnim` sitting branch. Baby feline applies these deltas unscaled after
/// the shared head-look and crouch/sprint setup.
fn apply_baby_feline_sitting_pose(root: &mut ModelPart) {
    root.child_mut("body").pose.rotation[0] += BABY_FELINE_SITTING_BODY_X_ROT_DELTA;
    root.child_mut("body").pose.offset[1] += 1.25;
    root.child_mut("head").pose.offset[2] += 0.75;
    root.child_mut("tail1").pose.rotation[0] += BABY_FELINE_SITTING_TAIL1_X_ROT_DELTA;
    root.child_mut("tail1").pose.offset[1] += 4.0;
    root.child_mut("tail1").pose.offset[2] -= 0.9;
    root.child_mut("left_hind_leg").pose.offset[2] -= 0.9;
    root.child_mut("right_hind_leg").pose.offset[2] -= 0.9;
}

fn apply_feline_sitting_pose(root: &mut ModelPart, baby: bool, age_scale: f32) {
    if baby {
        apply_baby_feline_sitting_pose(root);
    } else {
        apply_adult_feline_sitting_pose(root, age_scale);
    }
}

fn rot_lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

/// Vanilla `AdultFelineModel.setupAnim` cat lie-down branch. The state values are cat-only in vanilla
/// (`CatRenderer.extractRenderState`); ocelots leave them at `0.0`, so this remains a no-op there.
fn apply_adult_feline_lie_down_pose(
    root: &mut ModelPart,
    lie_down_amount: f32,
    lie_down_amount_tail: f32,
    age_scale: f32,
) {
    if lie_down_amount <= 0.0 {
        return;
    }

    {
        let head = root.child_mut("head");
        head.pose.rotation[2] = rot_lerp(
            lie_down_amount,
            head.pose.rotation[2],
            ADULT_FELINE_LIE_HEAD_Z_ROT,
        );
        head.pose.rotation[1] = rot_lerp(
            lie_down_amount,
            head.pose.rotation[1],
            ADULT_FELINE_LIE_HEAD_Y_ROT,
        );
    }
    root.child_mut("left_front_leg").pose.rotation[0] = ADULT_FELINE_LIE_LEFT_FRONT_LEG_X_ROT;
    {
        let right_front_leg = root.child_mut("right_front_leg");
        right_front_leg.pose.rotation[0] = ADULT_FELINE_LIE_RIGHT_FRONT_LEG_X_ROT;
        right_front_leg.pose.rotation[2] = -0.2;
        right_front_leg.pose.offset[0] += age_scale;
    }
    root.child_mut("left_hind_leg").pose.rotation[0] = -0.4;
    {
        let right_hind_leg = root.child_mut("right_hind_leg");
        right_hind_leg.pose.rotation[0] = 0.5;
        right_hind_leg.pose.rotation[2] = -0.5;
        right_hind_leg.pose.offset[0] += 0.8 * age_scale;
        right_hind_leg.pose.offset[1] += 2.0 * age_scale;
    }
    {
        let tail1 = root.child_mut("tail1");
        tail1.pose.rotation[0] = rot_lerp(lie_down_amount_tail, tail1.pose.rotation[0], 0.8);
    }
    {
        let tail2 = root.child_mut("tail2");
        tail2.pose.rotation[0] = rot_lerp(lie_down_amount_tail, tail2.pose.rotation[0], -0.4);
    }
}

/// Vanilla `BabyFelineModel.setupAnim` cat lie-down branch. Baby values are unscaled and apply after
/// the shared head look / crouch-sprint / sitting setup, matching vanilla order.
fn apply_baby_feline_lie_down_pose(
    root: &mut ModelPart,
    lie_down_amount: f32,
    lie_down_amount_tail: f32,
) {
    if lie_down_amount <= 0.0 {
        return;
    }

    root.child_mut("body").pose.offset[0] += 1.0;
    {
        let head = root.child_mut("head");
        head.pose.rotation[0] = rot_lerp(
            lie_down_amount,
            head.pose.rotation[0],
            std::f32::consts::PI / 18.0,
        );
        head.pose.rotation[2] = rot_lerp(
            lie_down_amount,
            head.pose.rotation[2],
            -std::f32::consts::PI * 5.0 / 12.0,
        );
        head.pose.offset[0] += 1.5;
        head.pose.offset[1] += 0.75;
        head.pose.offset[2] -= 0.5;
    }
    {
        let right_front_leg = root.child_mut("right_front_leg");
        right_front_leg.pose.rotation[0] = -std::f32::consts::FRAC_PI_4;
        right_front_leg.pose.offset[0] += 3.5;
        right_front_leg.pose.offset[1] -= 0.5;
    }
    {
        let left_front_leg = root.child_mut("left_front_leg");
        left_front_leg.pose.rotation[0] = -std::f32::consts::FRAC_PI_2;
        left_front_leg.pose.offset[0] += 1.5;
        left_front_leg.pose.offset[1] -= 1.0;
        left_front_leg.pose.offset[2] -= 2.0;
    }
    {
        let right_hind_leg = root.child_mut("right_hind_leg");
        right_hind_leg.pose.rotation[0] = std::f32::consts::PI * 2.0 / 9.0;
        right_hind_leg.pose.rotation[1] = std::f32::consts::PI / 9.0;
        right_hind_leg.pose.rotation[2] = -std::f32::consts::PI / 9.0;
        right_hind_leg.pose.offset[0] += 2.5;
        right_hind_leg.pose.offset[1] -= 0.25;
        right_hind_leg.pose.offset[2] += 0.5;
    }
    {
        let left_hind_leg = root.child_mut("left_hind_leg");
        left_hind_leg.pose.offset[0] += 1.5;
        left_hind_leg.pose.offset[2] -= 1.0;
    }
    {
        let tail1 = root.child_mut("tail1");
        tail1.pose.rotation[0] += rot_lerp(
            lie_down_amount_tail,
            tail1.pose.rotation[0],
            -std::f32::consts::PI / 6.0,
        );
        tail1.pose.rotation[1] += rot_lerp(lie_down_amount_tail, tail1.pose.rotation[1], 0.0);
        tail1.pose.rotation[2] += rot_lerp(
            lie_down_amount_tail,
            tail1.pose.rotation[2],
            -std::f32::consts::PI / 18.0,
        );
        tail1.pose.offset[0] += 1.0;
        tail1.pose.offset[1] += 0.5;
        tail1.pose.offset[2] -= 0.25;
    }
}

fn apply_feline_lie_down_pose(
    root: &mut ModelPart,
    baby: bool,
    lie_down_amount: f32,
    lie_down_amount_tail: f32,
    age_scale: f32,
) {
    if baby {
        apply_baby_feline_lie_down_pose(root, lie_down_amount, lie_down_amount_tail);
    } else {
        apply_adult_feline_lie_down_pose(root, lie_down_amount, lie_down_amount_tail, age_scale);
    }
}

/// Vanilla adult/baby feline relax branch: after lie-down, lerp the head pitch toward the relaxed
/// `-0.58177644` target by `relaxStateOneAmount`.
fn apply_feline_relax_pose(root: &mut ModelPart, relax_state_one_amount: f32) {
    if relax_state_one_amount <= 0.0 {
        return;
    }
    let head = root.child_mut("head");
    head.pose.rotation[0] = rot_lerp(
        relax_state_one_amount,
        head.pose.rotation[0],
        FELINE_RELAX_HEAD_X_ROT,
    );
}
