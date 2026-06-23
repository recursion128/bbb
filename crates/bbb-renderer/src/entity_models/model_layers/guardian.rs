use super::{head_look_pose, ModelCubeDesc, PartPose, GUARDIAN_BODY, GUARDIAN_EYE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

// Vanilla 26.1 `GuardianModel.createBodyLayer` (atlas 64×64). The whole model hangs off a
// single `head` part (`PartPose.ZERO`) carrying the body shell, twelve spikes, the eye, and the
// three-segment tail. The elder guardian is the same mesh scaled by 2.35 via a `MeshTransformer`.

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

// `head`: the main body box plus two mirrored side plates and the bottom/top plates
// (`texOffs(0,0)/(0,28)/(16,40)`).
pub(in crate::entity_models) const GUARDIAN_HEAD: [ModelCubeDesc; 5] = [
    ModelCubeDesc {
        min: [-6.0, 10.0, -8.0],
        size: [12.0, 12.0, 16.0],
        color: GUARDIAN_BODY,
    },
    ModelCubeDesc {
        min: [-8.0, 10.0, -6.0],
        size: [2.0, 12.0, 12.0],
        color: GUARDIAN_BODY,
    },
    ModelCubeDesc {
        min: [6.0, 10.0, -6.0],
        size: [2.0, 12.0, 12.0],
        color: GUARDIAN_BODY,
    },
    ModelCubeDesc {
        min: [-6.0, 8.0, -6.0],
        size: [12.0, 2.0, 12.0],
        color: GUARDIAN_BODY,
    },
    ModelCubeDesc {
        min: [-6.0, 22.0, -6.0],
        size: [12.0, 2.0, 12.0],
        color: GUARDIAN_BODY,
    },
];

// `spike`: a shared 2×9×2 box (`texOffs(0,0) addBox(-1, -4.5, -1, 2, 9, 2)`), instanced twelve
// times with [`guardian_spike_bind_pose`].
pub(in crate::entity_models) const GUARDIAN_SPIKE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, -4.5, -1.0],
    size: [2.0, 9.0, 2.0],
    color: GUARDIAN_BODY,
}];

// `eye`: `texOffs(8,0) addBox(-1, 15, 0, 2, 2, 1)` at `PartPose.offset(0, 0, -8.25)`.
pub(in crate::entity_models) const GUARDIAN_EYE_CUBE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 15.0, 0.0],
    size: [2.0, 2.0, 1.0],
    color: GUARDIAN_EYE,
}];
pub(in crate::entity_models) const GUARDIAN_EYE_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, -8.25],
    rotation: [0.0, 0.0, 0.0],
};

// The three-segment tail (`tail0` at `PartPose.ZERO`, `tail1`/`tail2` nested).
pub(in crate::entity_models) const GUARDIAN_TAIL0: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-2.0, 14.0, 7.0],
    size: [4.0, 4.0, 8.0],
    color: GUARDIAN_BODY,
}];
pub(in crate::entity_models) const GUARDIAN_TAIL1: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [0.0, 14.0, 0.0],
    size: [3.0, 3.0, 7.0],
    color: GUARDIAN_BODY,
}];
pub(in crate::entity_models) const GUARDIAN_TAIL1_POSE: PartPose = PartPose {
    offset: [-1.5, 0.5, 14.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const GUARDIAN_TAIL2: [ModelCubeDesc; 2] = [
    ModelCubeDesc {
        min: [0.0, 14.0, 0.0],
        size: [2.0, 2.0, 6.0],
        color: GUARDIAN_BODY,
    },
    ModelCubeDesc {
        min: [1.0, 10.5, 3.0],
        size: [1.0, 9.0, 9.0],
        color: GUARDIAN_BODY,
    },
];
pub(in crate::entity_models) const GUARDIAN_TAIL2_POSE: PartPose = PartPose {
    offset: [0.5, 0.5, 6.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `GuardianModel.createBodyLayer` places spike `i` at `getSpike{X,Y,Z}(i, 0, 0)` with
/// rotation `PI * SPIKE_{X,Y,Z}_ROT[i]`, where `getSpikeOffset(i, 0, 0) = 1 + cos(i) * 0.01`
/// and the Y base adds 16. This is the spike rest pose; the `setupAnim` age pulse
/// (`cos(ageInTicks · 1.5 + i)`) and the `spikesAnimation` withdrawal are deferred.
pub(in crate::entity_models) fn guardian_spike_bind_pose(i: usize) -> PartPose {
    let offset = 1.0 + (i as f32).cos() * 0.01;
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

/// Builds the guardian's `head` part tree: the body shell carries the twelve spikes, the eye, and
/// the three-segment tail chain (`tail0` → `tail1` → `tail2`) as children, in vanilla emit order.
fn guardian_head_part() -> ModelPart {
    let mut children: Vec<ModelPart> = (0..GUARDIAN_SPIKE_X.len())
        .map(|i| ModelPart::leaf_colored(guardian_spike_bind_pose(i), &GUARDIAN_SPIKE))
        .collect();
    children.push(ModelPart::leaf_colored(
        GUARDIAN_EYE_POSE,
        &GUARDIAN_EYE_CUBE,
    ));

    let tail2 = ModelPart::leaf_colored(GUARDIAN_TAIL2_POSE, &GUARDIAN_TAIL2);
    let tail1 = ModelPart::colored(GUARDIAN_TAIL1_POSE, &GUARDIAN_TAIL1, vec![tail2]);
    let tail0 = ModelPart::colored(PART_POSE_ZERO, &GUARDIAN_TAIL0, vec![tail1]);
    children.push(tail0);

    ModelPart::colored(PART_POSE_ZERO, &GUARDIAN_HEAD, children)
}

/// Mutable guardian model, mirroring vanilla `GuardianModel`. The whole guardian hangs off the
/// single `head` root part (body shell + twelve spikes + eye + three-segment tail), so the head IS
/// the model root. The elder variant is the same tree at the 2.35× scaled root transform (applied at
/// the call site). Colored-only: `setup_anim` turns the head — and with it the whole guardian — to
/// the look angles (the spike pulse, eye tracking, tail sway, and attack beam stay deferred).
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
    }
}
