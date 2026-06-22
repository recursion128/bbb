use super::{ModelCubeDesc, ModelPartDesc, PartPose, SQUID_BLUE};

// Vanilla 26.1 `SquidModel.createBodyLayer` (atlas 64×32). The body carries a
// `CubeDeformation(0.02)`, so its colored cube is the base box inflated by 0.02 on
// every side (the colored path bakes the inflated min/size directly).
pub(in crate::entity_models) const SQUID_BODY: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-6.02, -8.02, -6.02],
    size: [12.04, 16.04, 12.04],
    color: SQUID_BLUE,
}];

// The eight tentacles share one `CubeListBuilder` (`texOffs(48, 0)`, no deformation):
// box(-1, 0, -1, 2×18×2).
pub(in crate::entity_models) const SQUID_TENTACLE: [ModelCubeDesc; 1] = [ModelCubeDesc {
    min: [-1.0, 0.0, -1.0],
    size: [2.0, 18.0, 2.0],
    color: SQUID_BLUE,
}];

/// The body part pose: `PartPose.offset(0, 8, 0)`.
pub(in crate::entity_models) const SQUID_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 8.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the squid body layer parts (body + the eight tentacles) with the vanilla
/// `SquidModel.setupAnim` tentacle sweep already applied (`tentacle.xRot =
/// tentacleAngle` on every tentacle). The tentacle ring is procedural: tentacle `i`
/// sits at `(cos(i·2π/8)·5, 15, sin(i·2π/8)·5)` and is yawed by `-i·2π/8 + π/2` so its
/// flat face points outward, exactly as `SquidModel.createBodyLayer` lays them out.
pub(in crate::entity_models) fn squid_model_parts(tentacle_angle: f32) -> Vec<ModelPartDesc> {
    let mut parts = Vec::with_capacity(9);
    parts.push(ModelPartDesc {
        pose: SQUID_BODY_POSE,
        cubes: &SQUID_BODY,
        children: &[],
    });
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::TAU / 8.0;
        let x = angle.cos() * 5.0;
        let z = angle.sin() * 5.0;
        let y_rot = -(i as f32) * std::f32::consts::TAU / 8.0 + std::f32::consts::FRAC_PI_2;
        parts.push(ModelPartDesc {
            pose: PartPose {
                offset: [x, 15.0, z],
                rotation: [tentacle_angle, y_rot, 0.0],
            },
            cubes: &SQUID_TENTACLE,
            children: &[],
        });
    }
    parts
}
