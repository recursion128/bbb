use std::f32::consts::PI;

use super::{PartPose, BELL_GOLD, PART_POSE_ZERO};
use crate::entity_models::catalog::BellShakeDirection;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

/// Vanilla `ModelLayers.BELL` (`register("bell")`).
pub(in crate::entity_models) const MODEL_LAYER_BELL: &str = "minecraft:bell#main";

const fn bell_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, BELL_GOLD, size, tex, false)
}

// Vanilla 26.1 `BellModel.createBodyLayer` (atlas 32×32): the 6×7×6 `bell_body` box at
// (-3, -6, -3) texOffs(0, 0) pivoting at offset(8, 12, 8) — the swing hinge under the bar —
// with the 8×2×8 `bell_base` lip box at (4, 4, 4) texOffs(0, 13) as its child at
// offset(-8, -12, -8), so the lip swings with the body. Like the chest, the bell mesh is
// authored Y-up in block-local space (`BellRenderer.submit` applies no transform at all).
pub(in crate::entity_models) const BELL_BODY_CUBE: ModelCube =
    bell_cube([-3.0, -6.0, -3.0], [6.0, 7.0, 6.0], [0.0, 0.0]);
pub(in crate::entity_models) const BELL_BASE_CUBE: ModelCube =
    bell_cube([4.0, 4.0, 4.0], [8.0, 2.0, 8.0], [0.0, 13.0]);

/// Vanilla `PartPose.offset(8, 12, 8)` — the `bell_body` swing pivot.
pub(in crate::entity_models) const BELL_BODY_POSE: PartPose = PartPose {
    offset: [8.0, 12.0, 8.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Vanilla `PartPose.offset(-8, -12, -8)` — `bell_base` cancelling the body pivot.
pub(in crate::entity_models) const BELL_BASE_POSE: PartPose = PartPose {
    offset: [-8.0, -12.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `BellModel.setupAnim` swing angle:
/// `Mth.sin(ticks / π) / (4 + ticks / 3)` — a decaying oscillation over the shake
/// counter (`BellRenderState.ticks = blockEntity.ticks + partialTicks`).
pub(in crate::entity_models) fn bell_shake_base_rotation(ticks: f32) -> f32 {
    (ticks / PI).sin() / (4.0 + ticks / 3.0)
}

/// Vanilla 26.1 `BellModel`: the bell body + base lip. `setup_anim` transcribes
/// `BellModel.setupAnim`: with a shake direction, the base rotation swings the
/// `bell_body` about its pivot — NORTH `xRot = -r`, SOUTH `xRot = r`, EAST
/// `zRot = -r`, WEST `zRot = r` (the vanilla switch handles only the four
/// horizontal directions; DOWN/UP leave the bell still).
pub(in crate::entity_models) struct BellModel {
    root: ModelPart,
}

impl BellModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "bell_body",
                    ModelPart::new(
                        BELL_BODY_POSE,
                        vec![BELL_BODY_CUBE],
                        vec![(
                            "bell_base",
                            ModelPart::leaf(BELL_BASE_POSE, vec![BELL_BASE_CUBE]),
                        )],
                    ),
                )],
            ),
        }
    }
}

impl EntityModel for BellModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let mut x_rot = 0.0;
        let mut z_rot = 0.0;
        if let Some(direction) = instance.render_state.bell_shake_direction {
            let base_rot = bell_shake_base_rotation(instance.render_state.bell_ticks);
            match direction {
                BellShakeDirection::North => x_rot = -base_rot,
                BellShakeDirection::South => x_rot = base_rot,
                BellShakeDirection::East => z_rot = -base_rot,
                BellShakeDirection::West => z_rot = base_rot,
                BellShakeDirection::Down | BellShakeDirection::Up => {}
            }
        }
        let body = self.root.child_mut("bell_body");
        body.pose.rotation[0] = x_rot;
        body.pose.rotation[2] = z_rot;
    }
}
