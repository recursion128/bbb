use super::{ModelCubeDesc, PartPose, GUARDIAN_BODY, GUARDIAN_EYE};

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
