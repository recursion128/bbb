use super::{PartPose, ARROW_HEAD, ARROW_SHAFT, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_ARROW: &str = "minecraft:arrow#main";

// Vanilla 26.1 `ArrowModel.createBodyLayer` (atlas 32×32). The mesh root holds three sibling planes:
// the `back` arrowhead (a 0×5×5 YZ plane at `offset(-11, 0, 0)`, pitched π/4, `withScale(0.8)`), and
// two crossed fletching planes (`cross_1`/`cross_2`, each a 16×4×0 XY plane pitched π/4 and 3π/4).
// The whole mesh is baked through `mesh.transformed(pose -> pose.scaled(0.9))`; that 0.9 lives in
// `arrow_model_root_transform`, while the `back` part's `withScale(0.8)` is baked into its cube
// (a 0×5×5 box → a 0×4×4 box; the UV box stays the integer `texOffs(0,0)` dims [0,5,5]).
// `ArrowModel.setupAnim` only adds the impact-shake `root.zRot` wobble from
// `ArrowRenderState.shake`; bbb projects that from world `AbstractArrow.shakeTime`.
// `ArrowRenderer` orients the arrow along its flight with `Ry(yRot - 90)` then `Rz(xRot)` (no flip).
//
// The fletching plane's `addBox(..., 1.0F, 0.8F)` carries a `texScaleV = 0.8`: vanilla shrinks the V
// coverage of the cross plane's UV box to `0.8×`. bbb's `ModelCube` has no texScale field, but for a
// 0-depth plane only the front face has area and its V extent is exactly `uv_size[1]` (the textured
// emit's `v2 = y_tex + depth + height` with `depth = 0`), so the `0.8` is baked as
// `uv_size[1] = 4 × 0.8 = 3.2`. The `arrow.png` texture is wired here; the tipped/spectral variants
// stay bound by the entity kind's texture choice.

// `back`: the 0×5×5 arrowhead plane, `withScale(0.8)` baked into the centred YZ box → 0×4×4.
pub(in crate::entity_models) const ARROW_BACK_CUBE: ModelCube = ModelCube::new(
    [0.0, -2.0, -2.0],
    [0.0, 4.0, 4.0],
    ARROW_HEAD,
    [0.0, 5.0, 5.0],
    [0.0, 0.0],
    false,
);

// The shared 16×4×0 cross plane (both fletching planes reuse it, differing only in pitch). The
// `texScaleV = 0.8` is baked into the UV height (`4 × 0.8 = 3.2`); see the header.
pub(in crate::entity_models) const ARROW_CROSS_CUBE: ModelCube = ModelCube::new(
    [-12.0, -2.0, 0.0],
    [16.0, 4.0, 0.0],
    ARROW_SHAFT,
    [16.0, 3.2, 0.0],
    [0.0, 0.0],
    false,
);

pub(in crate::entity_models) const ARROW_BACK_POSE: PartPose = PartPose {
    offset: [-11.0, 0.0, 0.0],
    rotation: [std::f32::consts::FRAC_PI_4, 0.0, 0.0],
};
pub(in crate::entity_models) const ARROW_CROSS_1_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [std::f32::consts::FRAC_PI_4, 0.0, 0.0],
};
pub(in crate::entity_models) const ARROW_CROSS_2_POSE: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [3.0 * std::f32::consts::FRAC_PI_4, 0.0, 0.0],
};

/// Static arrow model mirroring vanilla `ArrowModel`: a root holding the `back` arrowhead plane and
/// the two crossed `cross_1`/`cross_2` fletching planes. `setup_anim` applies vanilla's impact shake
/// to the root z-rotation. Each cube carries the colored tint and the textured UV.
pub(in crate::entity_models) struct ArrowModel {
    root: ModelPart,
}

/// Vanilla `ArrowModel.setupAnim`: `pow = -sin(shake * 3) * shake`, then
/// `root.zRot += pow * π / 180`.
pub(in crate::entity_models) fn arrow_shake_z_rot(shake: f32) -> f32 {
    if shake > 0.0 {
        -f32::sin(shake * 3.0) * shake * std::f32::consts::PI / 180.0
    } else {
        0.0
    }
}

impl ArrowModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "back",
                        ModelPart::leaf(ARROW_BACK_POSE, vec![ARROW_BACK_CUBE]),
                    ),
                    (
                        "cross_1",
                        ModelPart::leaf(ARROW_CROSS_1_POSE, vec![ARROW_CROSS_CUBE]),
                    ),
                    (
                        "cross_2",
                        ModelPart::leaf(ARROW_CROSS_2_POSE, vec![ARROW_CROSS_CUBE]),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for ArrowModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        self.root.pose.rotation[2] += arrow_shake_z_rot(instance.render_state.arrow_shake);
    }
}
