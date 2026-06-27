use std::f32::consts::{FRAC_PI_2, TAU};

use super::{PartPose, PART_POSE_ZERO, SQUID_BLUE};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_SQUID: &str = "minecraft:squid#main";
pub(in crate::entity_models) const MODEL_LAYER_SQUID_BABY: &str = "minecraft:squid_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_GLOW_SQUID: &str = "minecraft:glow_squid#main";
pub(in crate::entity_models) const MODEL_LAYER_GLOW_SQUID_BABY: &str =
    "minecraft:glow_squid_baby#main";

// Vanilla 26.1 `SquidModel.createBodyLayer` (atlas 64×32). The body carries a `CubeDeformation(0.02)`,
// so its cube is the base box inflated by 0.02 on every side (the colored path bakes the inflated
// min/size directly, while the textured `uv_size` keeps the base 12×16×12 box). Each cube carries both
// render paths' data: the colored debug tint (`SQUID_BLUE`) and the textured `uv_size` / `texOffs`.
pub(in crate::entity_models) const SQUID_BODY: [ModelCube; 1] = [ModelCube::new(
    [-6.02, -8.02, -6.02],
    [12.04, 16.04, 12.04],
    SQUID_BLUE,
    [12.0, 16.0, 12.0],
    [0.0, 0.0],
    false,
)];

// The eight tentacles share one `CubeListBuilder` (`texOffs(48, 0)`, no deformation): box(-1, 0, -1,
// 2×18×2), so `uv_size == size`.
pub(in crate::entity_models) const SQUID_TENTACLE: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 18.0, 2.0],
    SQUID_BLUE,
    [2.0, 18.0, 2.0],
    [48.0, 0.0],
    false,
)];

/// The body part pose: `PartPose.offset(0, 8, 0)`.
pub(in crate::entity_models) const SQUID_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 8.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `SquidModel.createBodyLayer` tentacle child names, in ring order `tentacle0..tentacle7`.
/// `child_mut` needs `&'static` names, so the procedural ring draws its names from this const array.
const SQUID_TENTACLE_NAMES: [&str; 8] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
];

/// Bind pose of tentacle `i`: vanilla `SquidModel.createBodyLayer` lays the ring at
/// `(cos(i·2π/8)·5, 15, sin(i·2π/8)·5)`, yawed `-i·2π/8 + π/2` so the flat face points outward. The
/// `xRot` sweep is `0` at bind and overwritten each frame by `setup_anim`.
pub(in crate::entity_models) fn squid_tentacle_pose(i: usize) -> PartPose {
    let angle = i as f32 * TAU / 8.0;
    PartPose {
        offset: [angle.cos() * 5.0, 15.0, angle.sin() * 5.0],
        rotation: [0.0, -(i as f32) * TAU / 8.0 + FRAC_PI_2, 0.0],
    }
}

/// Mutable squid model, mirroring vanilla `SquidModel`. The unified tree (body + the eight-tentacle
/// ring) is built once with named children; `setup_anim` runs the vanilla `SquidModel.setupAnim`
/// tentacle sweep (`tentacle.xRot = tentacleAngle` on every tentacle). The same posed tree drives the
/// colored fallback (recolored to the squid / glow-squid tint) and the textured base layer; the swim
/// body tilt, baby scale, and glow texture live in the squid root transform / texture selection, not
/// the model.
pub(in crate::entity_models) struct SquidModel {
    root: ModelPart,
}

impl SquidModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(9);
        children.push((
            "body",
            ModelPart::leaf(SQUID_BODY_POSE, SQUID_BODY.to_vec()),
        ));
        for (i, &name) in SQUID_TENTACLE_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(squid_tentacle_pose(i), SQUID_TENTACLE.to_vec()),
            ));
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
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
        // Vanilla `SquidModel.setupAnim` sets `tentacle.xRot = tentacleAngle` on every tentacle, while
        // the body and the tentacles' fixed yaw layout stay at the bind pose. The angle is `0` at rest,
        // so a still squid is identical to its bind tree.
        let tentacle_angle = instance.render_state.squid_tentacle_angle;
        for &name in &SQUID_TENTACLE_NAMES {
            self.root.child_mut(name).pose.rotation[0] = tentacle_angle;
        }
    }
}
