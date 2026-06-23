use super::{
    ModelCubeDesc, ModelPartDesc, PartPose, TexturedModelCubeDesc, TexturedModelPartDesc,
    SQUID_BLUE,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelPart};

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

// Textured counterparts of [`SQUID_BODY`] / [`SQUID_TENTACLE`]. The body's
// `CubeDeformation(0.02)` inflates the geometry but not the UVs, so `uv_size` keeps the
// base 12×16×12 box while `size` is the inflated 12.04×16.04×12.04.
pub(in crate::entity_models) const SQUID_TEXTURED_BODY: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-6.02, -8.02, -6.02],
        size: [12.04, 16.04, 12.04],
        uv_size: [12.0, 16.0, 12.0],
        tex: [0.0, 0.0],
        mirror: false,
    }];

pub(in crate::entity_models) const SQUID_TEXTURED_TENTACLE: [TexturedModelCubeDesc; 1] =
    [TexturedModelCubeDesc {
        min: [-1.0, 0.0, -1.0],
        size: [2.0, 18.0, 2.0],
        uv_size: [2.0, 18.0, 2.0],
        tex: [48.0, 0.0],
        mirror: false,
    }];

/// Textured counterpart of [`squid_model_parts`]: the same procedural body + tentacle
/// ring with the `tentacleAngle` sweep, built as textured parts for the hand-emitted
/// squid render path.
pub(in crate::entity_models) fn squid_textured_model_parts(
    tentacle_angle: f32,
) -> Vec<TexturedModelPartDesc> {
    let mut parts = Vec::with_capacity(9);
    parts.push(TexturedModelPartDesc {
        pose: SQUID_BODY_POSE,
        cubes: &SQUID_TEXTURED_BODY,
        children: &[],
    });
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::TAU / 8.0;
        let x = angle.cos() * 5.0;
        let z = angle.sin() * 5.0;
        let y_rot = -(i as f32) * std::f32::consts::TAU / 8.0 + std::f32::consts::FRAC_PI_2;
        parts.push(TexturedModelPartDesc {
            pose: PartPose {
                offset: [x, 15.0, z],
                rotation: [tentacle_angle, y_rot, 0.0],
            },
            cubes: &SQUID_TEXTURED_TENTACLE,
            children: &[],
        });
    }
    parts
}

/// The squid root's first child is the static body; its eight tentacles follow as children `1..=8`.
const SQUID_TENTACLE_CHILD_INDICES: std::ops::RangeInclusive<usize> = 1..=8;

/// Applies the vanilla `SquidModel.setupAnim` tentacle sweep to the unified tree: every tentacle's
/// `xRot` is set to the lerped `tentacleAngle`, while the body and the tentacles' fixed yaw layout
/// stay at the bind pose. The angle is `0` at rest, so a still squid is byte-identical to its bind
/// tree (the `*_PARTS` builders bake `xRot = 0` and `setup_anim` overwrites it each frame).
fn apply_squid_tentacle_sweep(root: &mut ModelPart, tentacle_angle: f32) {
    for index in SQUID_TENTACLE_CHILD_INDICES {
        root.child_at_mut(index).pose.rotation[0] = tentacle_angle;
    }
}

/// Mutable squid model, mirroring vanilla `SquidModel`. The unified tree is built once from the
/// procedural body + eight-tentacle ring ([`squid_model_parts`] / [`squid_textured_model_parts`] at
/// the rest `tentacleAngle = 0`); `setup_anim` runs [`apply_squid_tentacle_sweep`]. The same posed
/// tree drives the colored fallback (recolored to the squid / glow-squid tint) and the textured base
/// layer; the swim body tilt, baby scale, and glow texture live in the squid root transform / texture
/// selection, not the model.
pub(in crate::entity_models) struct SquidModel {
    root: ModelPart,
}

impl SquidModel {
    pub(in crate::entity_models) fn new() -> Self {
        let colored = squid_model_parts(0.0);
        let textured = squid_textured_model_parts(0.0);
        Self {
            root: ModelPart::root_from_descs(&colored, &textured),
        }
    }
}

impl EntityModel for SquidModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_squid_tentacle_sweep(&mut self.root, instance.render_state.squid_tentacle_angle);
    }
}
