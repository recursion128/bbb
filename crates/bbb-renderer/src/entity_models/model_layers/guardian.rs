use super::{head_look_pose, PartPose, GUARDIAN_BODY, GUARDIAN_EYE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `GuardianModel.createBodyLayer` (atlas 64×64). The whole model hangs off a
// single `head` part (`PartPose.ZERO`) carrying the body shell, twelve spikes, the eye, and the
// three-segment tail. The `guardian.png` texture is wired here; the elder guardian (the same mesh
// scaled 2.35 via a `MeshTransformer`, `guardian_elder.png`) and the attack beam stay deferred. Each
// cube carries the colored debug tint and the textured `uv_size` / `texOffs`.

/// Vanilla `GuardianModel.ELDER_GUARDIAN_SCALE = MeshTransformer.scaling(2.35F)`.
pub(in crate::entity_models) const GUARDIAN_ELDER_SCALE: f32 = 2.35;

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

// `head`: the main body box (`texOffs(0,0)`), the left side plate (`texOffs(0,28)`) and its mirrored
// right plate (same texOffs, `mirror()`), and the bottom/top plates (both `texOffs(16,40)`).
pub(in crate::entity_models) const GUARDIAN_HEAD: [ModelCube; 5] = [
    ModelCube::new(
        [-6.0, 10.0, -8.0],
        [12.0, 12.0, 16.0],
        GUARDIAN_BODY,
        [12.0, 12.0, 16.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-8.0, 10.0, -6.0],
        [2.0, 12.0, 12.0],
        GUARDIAN_BODY,
        [2.0, 12.0, 12.0],
        [0.0, 28.0],
        false,
    ),
    ModelCube::new(
        [6.0, 10.0, -6.0],
        [2.0, 12.0, 12.0],
        GUARDIAN_BODY,
        [2.0, 12.0, 12.0],
        [0.0, 28.0],
        true,
    ),
    ModelCube::new(
        [-6.0, 8.0, -6.0],
        [12.0, 2.0, 12.0],
        GUARDIAN_BODY,
        [12.0, 2.0, 12.0],
        [16.0, 40.0],
        false,
    ),
    ModelCube::new(
        [-6.0, 22.0, -6.0],
        [12.0, 2.0, 12.0],
        GUARDIAN_BODY,
        [12.0, 2.0, 12.0],
        [16.0, 40.0],
        false,
    ),
];

// `spike`: a shared 2×9×2 box (`texOffs(0,0) addBox(-1, -4.5, -1, 2, 9, 2)`), instanced twelve
// times with [`guardian_spike_bind_pose`].
pub(in crate::entity_models) const GUARDIAN_SPIKE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, -4.5, -1.0],
    [2.0, 9.0, 2.0],
    GUARDIAN_BODY,
    [2.0, 9.0, 2.0],
    [0.0, 0.0],
    false,
)];

// `eye`: `texOffs(8,0) addBox(-1, 15, 0, 2, 2, 1)` at `PartPose.offset(0, 0, -8.25)`.
pub(in crate::entity_models) const GUARDIAN_EYE_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 15.0, 0.0],
    [2.0, 2.0, 1.0],
    GUARDIAN_EYE,
    [2.0, 2.0, 1.0],
    [8.0, 0.0],
    false,
)];
pub(in crate::entity_models) const GUARDIAN_EYE_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, -8.25],
    rotation: [0.0, 0.0, 0.0],
};

// The three-segment tail (`tail0` at `PartPose.ZERO`, `tail1`/`tail2` nested).
pub(in crate::entity_models) const GUARDIAN_TAIL0: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 14.0, 7.0],
    [4.0, 4.0, 8.0],
    GUARDIAN_BODY,
    [4.0, 4.0, 8.0],
    [40.0, 0.0],
    false,
)];
pub(in crate::entity_models) const GUARDIAN_TAIL1: [ModelCube; 1] = [ModelCube::new(
    [0.0, 14.0, 0.0],
    [3.0, 3.0, 7.0],
    GUARDIAN_BODY,
    [3.0, 3.0, 7.0],
    [0.0, 54.0],
    false,
)];
pub(in crate::entity_models) const GUARDIAN_TAIL1_POSE: PartPose = PartPose {
    offset: [-1.5, 0.5, 14.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const GUARDIAN_TAIL2: [ModelCube; 2] = [
    ModelCube::new(
        [0.0, 14.0, 0.0],
        [2.0, 2.0, 6.0],
        GUARDIAN_BODY,
        [2.0, 2.0, 6.0],
        [41.0, 32.0],
        false,
    ),
    ModelCube::new(
        [1.0, 10.5, 3.0],
        [1.0, 9.0, 9.0],
        GUARDIAN_BODY,
        [1.0, 9.0, 9.0],
        [25.0, 19.0],
        false,
    ),
];
pub(in crate::entity_models) const GUARDIAN_TAIL2_POSE: PartPose = PartPose {
    offset: [0.5, 0.5, 6.0],
    rotation: [0.0, 0.0, 0.0],
};

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

/// The spike rest pose — the age pulse at `ageInTicks = 0` with no withdrawal (`cos(i)`) — used to
/// build the bind tree before `setupAnim` re-poses the spikes with the live age phase and withdrawal.
pub(in crate::entity_models) fn guardian_spike_bind_pose(i: usize) -> PartPose {
    guardian_spike_pose(i, 0.0, 0.0)
}

/// The twelve spikes are the head's first twelve children (built before the eye and tail), so they
/// carry the index child names `"0"`..=`"11"`.
const GUARDIAN_SPIKE_CHILD_NAMES: [&str; 12] =
    ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"];

/// The eye is the head's thirteenth child (after the twelve spikes `"0"`..=`"11"`), index `"12"`.
const GUARDIAN_EYE_CHILD_NAME: &str = "12";

/// `tail0` is the head's fourteenth child (twelve spikes `"0"`..=`"11"`, then the eye `"12"`, then
/// `tail0` `"13"`), and `tail1`/`tail2` are each the single (index-`"0"`) child of the segment above.
const GUARDIAN_TAIL0_CHILD_NAME: &str = "13";
const GUARDIAN_TAIL_NESTED_CHILD_NAME: &str = "0";

/// Vanilla `GuardianModel.setupAnim` tail-segment `yRot` scales: `sin(swim) * π * {0.05, 0.1,
/// 0.15}` for `tail0`/`tail1`/`tail2`, so each deeper segment sways a little harder.
const GUARDIAN_TAIL_YROT_SCALE: [f32; 3] = [0.05, 0.1, 0.15];

/// Builds the guardian's `head` part tree: the body shell carries the twelve spikes, the eye, and
/// the three-segment tail chain (`tail0` → `tail1` → `tail2`) as children, in vanilla emit order.
fn guardian_head_part() -> ModelPart {
    let mut children: Vec<(&'static str, ModelPart)> = (0..GUARDIAN_SPIKE_X.len())
        .map(|i| {
            (
                GUARDIAN_SPIKE_CHILD_NAMES[i],
                ModelPart::leaf(guardian_spike_bind_pose(i), GUARDIAN_SPIKE.to_vec()),
            )
        })
        .collect();
    children.push((
        GUARDIAN_EYE_CHILD_NAME,
        ModelPart::leaf(GUARDIAN_EYE_POSE, GUARDIAN_EYE_CUBE.to_vec()),
    ));

    let tail2 = ModelPart::leaf(GUARDIAN_TAIL2_POSE, GUARDIAN_TAIL2.to_vec());
    let tail1 = ModelPart::new(
        GUARDIAN_TAIL1_POSE,
        GUARDIAN_TAIL1.to_vec(),
        vec![(GUARDIAN_TAIL_NESTED_CHILD_NAME, tail2)],
    );
    let tail0 = ModelPart::new(
        PART_POSE_ZERO,
        GUARDIAN_TAIL0.to_vec(),
        vec![(GUARDIAN_TAIL_NESTED_CHILD_NAME, tail1)],
    );
    children.push((GUARDIAN_TAIL0_CHILD_NAME, tail0));

    ModelPart::new(PART_POSE_ZERO, GUARDIAN_HEAD.to_vec(), children)
}

/// Mutable guardian model, mirroring vanilla `GuardianModel`. The whole guardian hangs off the
/// single `head` root part (body shell + twelve spikes + eye + three-segment tail), so the head IS
/// the model root. The elder variant is the same tree at the 2.35× scaled root transform (applied at
/// the call site). Colored-only: `setup_anim` turns the head — and with it the whole guardian — to
/// the look angles, pulses the twelve spikes in and out with the entity age, and sways the
/// three-segment tail with the in-water swim accumulator (eye tracking and the attack beam stay
/// deferred).
pub(in crate::entity_models) struct GuardianModel {
    root: ModelPart,
}

impl GuardianModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: guardian_head_part(),
        }
    }
}

impl EntityModel for GuardianModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `GuardianModel.setupAnim` sets `head.yRot/xRot` from the plain look; every part is
        // a child of `head`, so the whole guardian turns with it. The head's bind pose is ZERO, so a
        // level gaze collapses to the bind pose and the look applies every frame.
        self.root.pose = head_look_pose(
            self.root.pose,
            instance.render_state.head_yaw,
            instance.render_state.head_pitch,
        );
        // Vanilla `setupAnim` also pulses each of the twelve spikes in and out by
        // `getSpikeOffset(i, ageInTicks, withdrawal) = 1 + cos(ageInTicks · 1.5 + i) · 0.01 -
        // withdrawal`, where `withdrawal = (1 - state.spikesAnimation) · 0.55` retracts the whole
        // crown — `0` while idle in water (fully extended), ~`0.55` while swimming. The spikes are the
        // head's first twelve children, in build order, so they re-pose with the live age + withdrawal.
        let age_pulse = instance.render_state.age_in_ticks * 1.5;
        let withdrawal = (1.0 - instance.render_state.guardian_spikes_animation) * 0.55;
        for i in 0..GUARDIAN_SPIKE_X.len() {
            self.root.child_mut(GUARDIAN_SPIKE_CHILD_NAMES[i]).pose =
                guardian_spike_pose(i, age_pulse, withdrawal);
        }
        // Vanilla `GuardianModel.setupAnim`: `float swim = state.tailAnimation; tailParts[i].yRot =
        // sin(swim) * π * {0.05, 0.1, 0.15}`. The three tail segments are a `tail0 → tail1 → tail2`
        // chain; each bind pose is ZERO-rotation, so the sway is the segment's whole `yRot`. The
        // accumulator (`Guardian.aiStep`) runs `2.0`/tick out of water (a frantic flop), snaps up and
        // eases toward `0.5`/tick while moving in water, and settles toward `0.125`/tick while idle.
        let swim = instance.render_state.guardian_tail_animation;
        let sway = swim.sin() * std::f32::consts::PI;
        let tail0 = self.root.child_mut(GUARDIAN_TAIL0_CHILD_NAME);
        tail0.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[0];
        let tail1 = tail0.child_mut(GUARDIAN_TAIL_NESTED_CHILD_NAME);
        tail1.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[1];
        let tail2 = tail1.child_mut(GUARDIAN_TAIL_NESTED_CHILD_NAME);
        tail2.pose.rotation[1] = sway * GUARDIAN_TAIL_YROT_SCALE[2];
    }
}
