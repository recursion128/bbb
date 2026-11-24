use super::{head_look_pose, PartPose};
use crate::entity_models::catalog::GuardianEyeTrackingRenderState;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

// Vanilla 26.1 `GuardianModel.createBodyLayer` (atlas 64×64). The whole model hangs off a
// single `head` part (`PartPose.ZERO`) carrying the body shell, twelve spikes, the eye, and the
// three-segment tail. The `guardian.png` texture is wired here; the elder guardian (the same mesh
// scaled 2.35 via a `MeshTransformer`, `guardian_elder.png`) shares the body tree, while the attack
// beam is custom geometry submitted after the body. Each cube carries the colored debug tint and the
// textured `uv_size` / `texOffs`.

// `GuardianModel.SPIKE_{X,Y,Z}_ROT` (multiplied by π at bind) and `SPIKE_{X,Y,Z}` base offsets.
pub(in crate::entity_models) const GUARDIAN_SPIKE_X_ROT: [f32; 12] = [
    1.75, 0.25, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.25, 0.75, 0.0, 0.0,
];
pub(in crate::entity_models) const GUARDIAN_SPIKE_Y_ROT: [f32; 12] = [
    0.0, 0.0, 0.0, 0.0, 0.25, 1.75, 1.25, 0.75, 0.0, 0.0, 0.0, 0.0,
];
pub(in crate::entity_models) const GUARDIAN_SPIKE_Z_ROT: [f32; 12] = [
    0.0, 0.0, 0.25, 1.75, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.75, 1.25,
];
pub(in crate::entity_models) const GUARDIAN_SPIKE_X: [f32; 12] = [
    0.0, 0.0, 8.0, -8.0, -8.0, 8.0, 8.0, -8.0, 0.0, 0.0, 8.0, -8.0,
];
pub(in crate::entity_models) const GUARDIAN_SPIKE_Y: [f32; 12] = [
    -8.0, -8.0, -8.0, -8.0, 0.0, 0.0, 0.0, 0.0, 8.0, 8.0, 8.0, 8.0,
];
pub(in crate::entity_models) const GUARDIAN_SPIKE_Z: [f32; 12] = [
    8.0, -8.0, 0.0, 0.0, -8.0, -8.0, 8.0, 8.0, 8.0, -8.0, 0.0, 0.0,
];

/// Vanilla `GuardianModel` spike `i` at `getSpike{X,Y,Z}(i, ageInTicks, withdrawal)` with rotation
/// `PI * SPIKE_{X,Y,Z}_ROT[i]`, where `getSpikeOffset(i, ageInTicks, withdrawal) =
/// 1 + cos(ageInTicks · 1.5 + i) · 0.01 - withdrawal` and the Y base adds 16. So the spikes slowly
/// pulse in and out with the entity age, and the whole spike crown retracts by `withdrawal` (vanilla
/// `setupAnim`'s `(1 - spikesAnimation) · 0.55`) — fully extended (`withdrawal = 0`) while idle in
/// water, pulled ~0.55 in while swimming. `createBodyLayer` bakes the bind pose at `ageInTicks = 0`,
/// `withdrawal = 0` (`cos(i)`), and `setupAnim` re-poses each spike every frame with the live phase.
pub(in crate::entity_models) fn guardian_spike_pose(
    i: usize,
    age_pulse: f32,
    withdrawal: f32,
) -> PartPose {
    let offset = 1.0 + (age_pulse + i as f32).cos() * 0.01 - withdrawal;
    PartPose {
        offset: [
            GUARDIAN_SPIKE_X[i] * offset,
            16.0 + GUARDIAN_SPIKE_Y[i] * offset,
            GUARDIAN_SPIKE_Z[i] * offset,
        ],
        rotation: [
            std::f32::consts::PI * GUARDIAN_SPIKE_X_ROT[i],
            std::f32::consts::PI * GUARDIAN_SPIKE_Y_ROT[i],
            std::f32::consts::PI * GUARDIAN_SPIKE_Z_ROT[i],
        ],
    }
}

const GUARDIAN_SPIKE_NAMES: [&str; 12] = [
    "spike0", "spike1", "spike2", "spike3", "spike4", "spike5", "spike6", "spike7", "spike8",
    "spike9", "spike10", "spike11",
];

/// Vanilla `GuardianModel.setupAnim` tail-segment `yRot` scales: `sin(swim) * π * {0.05, 0.1,
/// 0.15}` for `tail0`/`tail1`/`tail2`, so each deeper segment sways a little harder.
const GUARDIAN_TAIL_YROT_SCALE: [f32; 3] = [0.05, 0.1, 0.15];

/// Vanilla `GuardianModel`.
pub(in crate::entity_models) fn guardian_model(
    elder: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if elder {
        ModelLayer::ELDER_GUARDIAN
    } else {
        ModelLayer::GUARDIAN
    };
    let mut root = layer.bake();
    let guardian_state = *instance.render_state.kind.guardian();
    // Vanilla `GuardianModel.setupAnim` sets `head.yRot/xRot` from the plain look; every part is
    // a child of `head`, so the whole guardian turns with it. The head's bind pose is ZERO, so a
    // level gaze collapses to the bind pose and the look applies every frame.
    let head = root.child_mut("head");
    head.pose = head_look_pose(
        head.pose,
        instance.render_state.head_yaw,
        instance.render_state.head_pitch,
    );
    // Vanilla `setupAnim` also pulses each of the twelve spikes in and out by
    // `getSpikeOffset(i, ageInTicks, withdrawal) = 1 + cos(ageInTicks · 1.5 + i) · 0.01 -
    // withdrawal`, where `withdrawal = (1 - state.spikesAnimation) · 0.55` retracts the whole
    // crown — `0` while idle in water (fully extended), ~`0.55` while swimming. The spikes are the
    // head's first twelve children, in build order, so they re-pose with the live age + withdrawal.
    let age_pulse = instance.render_state.age_in_ticks * 1.5;
    let withdrawal = (1.0 - guardian_state.spikes_animation) * 0.55;
    for (i, &spike_name) in GUARDIAN_SPIKE_NAMES
        .iter()
        .enumerate()
        .take(GUARDIAN_SPIKE_X.len())
    {
        head.child_mut(spike_name).pose = guardian_spike_pose(i, age_pulse, withdrawal);
    }
    // Vanilla `GuardianModel.setupAnim` keeps the extracted Vec3 values in world space until
    // this model-owned formula. Targets above the guardian use the eye's bind y; targets at or
    // below it move the eye one model unit down. Horizontal motion follows the signed square
    // root of the guardian view vector dotted with the eye-to-target direction rotated +π/2.
    if let Some(tracking) = guardian_state.eye_tracking {
        let [eye_x, eye_y] = guardian_eye_offset(tracking);
        let eye = head.child_mut("eye");
        eye.pose.offset[0] = eye_x;
        eye.pose.offset[1] = eye_y;
    }
    // Vanilla `GuardianModel.setupAnim`: `float swim = state.tailAnimation; tailParts[i].yRot =
    // sin(swim) * π * {0.05, 0.1, 0.15}`. The three tail segments are a `tail0 → tail1 → tail2`
    // chain; each bind pose is ZERO-rotation, so the sway is the segment's whole `yRot`. The
    // accumulator (`Guardian.aiStep`) runs `2.0`/tick out of water (a frantic flop), snaps up and
    // eases toward `0.5`/tick while moving in water, and settles toward `0.125`/tick while idle.
    let swim = guardian_state.tail_animation;
    let sway = swim.sin() * std::f32::consts::PI;
    let tail0 = head.child_mut("tail0");
    tail0.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[0];
    let tail1 = tail0.child_mut("tail1");
    tail1.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[1];
    let tail2 = tail1.child_mut("tail2");
    tail2.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[2];
    root
}

fn guardian_eye_offset(tracking: GuardianEyeTrackingRenderState) -> [f32; 2] {
    let eye_y = if tracking.look_at_position[1] > tracking.eye_position[1] {
        0.0
    } else {
        1.0
    };
    let delta_x = tracking.eye_position[0] - tracking.look_at_position[0];
    let delta_z = tracking.eye_position[2] - tracking.look_at_position[2];
    let length = (delta_x * delta_x + delta_z * delta_z).sqrt();
    let (delta_x, delta_z) = if length < f64::from(1.0e-5_f32) {
        (0.0, 0.0)
    } else {
        (delta_x / length, delta_z / length)
    };
    // `Vec3.yRot(π/2)` uses Vanilla's float trig before the double-vector multiply.
    let cos = f64::from(std::f32::consts::FRAC_PI_2.cos());
    let sin = f64::from(std::f32::consts::FRAC_PI_2.sin());
    let rotated_x = delta_x * cos + delta_z * sin;
    let rotated_z = delta_z * cos - delta_x * sin;
    let dot = tracking.look_direction[0] * rotated_x + tracking.look_direction[2] * rotated_z;
    let sign = if dot > 0.0 {
        1.0
    } else if dot < 0.0 {
        -1.0
    } else {
        0.0
    };
    let eye_x = (dot.abs() as f32).sqrt() * 2.0 * sign;
    [eye_x, eye_y]
}
