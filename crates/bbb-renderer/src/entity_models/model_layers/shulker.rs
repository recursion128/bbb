use super::{PartPose, PART_POSE_ZERO, SHULKER_HEAD, SHULKER_SHELL};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `ShulkerModel.createBodyLayer` (atlas 64×64). The mesh root holds three sibling
// parts: the 16×12×16 lid and the 16×8×16 base (both at `offset(0, 24, 0)`), and the 6×6×6 head at
// `offset(0, 12, 0)`. The closed rest pose equals this bind pose — `ShulkerModel.setupAnim` sets the
// lid back to `y = 16 + sin((0.5 + peekAmount) * π) * 8`, which is exactly `24` when `peekAmount = 0`.
// The peek open/close and the head look are now driven from the projected peek and head angles (see
// [`emit_shulker_model`](crate::entity_models::colored::runtime)). The head look uses the vanilla
// non-standard `head.yRot = (yHeadRot − 180 − yBodyRot)`, which equals the projected `head_yaw − 180`;
// it is vanilla-correct for a floor shulker because bbb's standard root differs from the shulker's
// `bodyRot + 180` root by a 180° rotation the 180°-symmetric square shell hides. The
// `ShulkerRenderer.setupRotations` non-floor attach-face rotation (`attachFace.getOpposite()`, the
// identity for a floor shulker) and the `bodyRot + 180` body-yaw inversion read the entity-side
// `attachFace`/yaw state, which the native scene does not yet project, so the floor rest orientation
// is emitted. The sixteen dye-color variants share this one UV layout (only the texture image
// differs); the colored debug path renders a purple shell tint plus a yellow head tint.

// `lid`: the 16×12×16 upper shell at texOffs(0, 0). Each unified cube carries the colored tint and
// the textured `uv_size` / `texOffs`.
pub(in crate::entity_models) const SHULKER_LID_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -16.0, -8.0],
    [16.0, 12.0, 16.0],
    SHULKER_SHELL,
    [16.0, 12.0, 16.0],
    [0.0, 0.0],
    false,
)];

// `base`: the 16×8×16 lower shell at texOffs(0, 28).
pub(in crate::entity_models) const SHULKER_BASE_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -8.0, -8.0],
    [16.0, 8.0, 16.0],
    SHULKER_SHELL,
    [16.0, 8.0, 16.0],
    [0.0, 28.0],
    false,
)];

// `head`: the 6×6×6 yellow head at texOffs(0, 52).
pub(in crate::entity_models) const SHULKER_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, 0.0, -3.0],
    [6.0, 6.0, 6.0],
    SHULKER_HEAD,
    [6.0, 6.0, 6.0],
    [0.0, 52.0],
    false,
)];

/// `lid` and `base` part poses: both `PartPose.offset(0, 24, 0)`.
pub(in crate::entity_models) const SHULKER_SHELL_POSE: PartPose = PartPose {
    offset: [0.0, 24.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// `head` part pose: `PartPose.offset(0, 12, 0)`.
pub(in crate::entity_models) const SHULKER_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `ShulkerModel.setupAnim` lid pose from the client peek amount (and `ageInTicks`
/// for the open-lid bob). With `bs = (0.5 + peek)·π`:
/// `lid.y = 16 + sin(bs)·8` (plus `sin(ageInTicks·0.1)·0.7` once `bs > π`, i.e. the lid is
/// past half-open) and `lid.yRot = (−1 + sin(bs))⁴ · π · 0.125` once `peek > 0.3` (else `0`).
/// Returns `(lid_y, lid_yrot)`. At `peek = 0` this is `(24, 0)` — the closed/bind pose.
pub(in crate::entity_models) fn shulker_lid_pose(peek: f32, age_in_ticks: f32) -> (f32, f32) {
    let bs = (0.5 + peek) * std::f32::consts::PI;
    let extra = if bs > std::f32::consts::PI {
        (age_in_ticks * 0.1).sin() * 0.7
    } else {
        0.0
    };
    let lid_y = 16.0 + bs.sin() * 8.0 + extra;
    let lid_yrot = if peek > 0.3 {
        let q = -1.0 + bs.sin();
        q * q * q * q * std::f32::consts::PI * 0.125
    } else {
        0.0
    };
    (lid_y, lid_yrot)
}

/// Mutable shulker model, mirroring vanilla `ShulkerModel`. Its three named sibling parts (`lid`,
/// `base`, `head`) hang off a synthetic root, each built from the baked geometry. `setup_anim` opens
/// the lid from the synced peek and turns the head to the look angles via `child_mut`.
pub(in crate::entity_models) struct ShulkerModel {
    root: ModelPart,
}

impl ShulkerModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "lid",
                        ModelPart::leaf(SHULKER_SHELL_POSE, SHULKER_LID_CUBES.to_vec()),
                    ),
                    (
                        "base",
                        ModelPart::leaf(SHULKER_SHELL_POSE, SHULKER_BASE_CUBES.to_vec()),
                    ),
                    (
                        "head",
                        ModelPart::leaf(SHULKER_HEAD_POSE, SHULKER_HEAD_CUBES.to_vec()),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for ShulkerModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `ShulkerModel.setupAnim`: the lid opens with the synced peek ([`shulker_lid_pose`])
        // and the head tracks the look angles, while the base holds still. All three are absolute
        // sets — at `peek = 0` the lid returns to its `y = 24` bind offset, and the head yaw carries
        // the vanilla `−180` cancel (so it never equals the bind yaw), so they apply every frame.
        let (lid_y, lid_yrot) = shulker_lid_pose(
            instance.render_state.shulker_peek,
            instance.render_state.age_in_ticks,
        );
        let lid = self.root.child_mut("lid");
        lid.pose.offset[1] = lid_y;
        lid.pose.rotation[1] = lid_yrot;

        let head = self.root.child_mut("head");
        head.pose.rotation[0] = instance.render_state.head_pitch.to_radians();
        head.pose.rotation[1] = (instance.render_state.head_yaw - 180.0).to_radians();
    }
}
