use super::{PartPose, EVOKER_FANGS_BASE, EVOKER_FANGS_JAW, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_EVOKER_FANGS: &str = "minecraft:evoker_fangs#main";

// Vanilla 26.1 `EvokerFangsModel.createBodyLayer` (atlas 64×32). The mesh root holds the base block
// at `offset(-5, 24, -5)`, which parents the two jaws (a shared 4×14×8 box). The bind-pose jaw
// rotations are exactly the `setupAnim` closed-jaw rest at `biteProgress = 0`: `upperJaw.zRot =
// π - 0.35π = 0.65π = 2.042035` and `lowerJaw.zRot = π + 0.35π = 1.35π = 4.2411504` (the lower jaw
// also carries `yRot = π`). `EvokerFangsModel.setupAnim` drives the jaw snap, the rise out of the
// ground, and the final vanish from the `biteProgress` ramp (see [`EvokerFangsModel::setup_anim`]).
// `EvokerFangsRenderer` applies the standard flip and `-1.501` y-offset but a distinct `Ry(90 - yRot)`
// yaw, captured by `evoker_fangs_model_root_transform`. Each unified cube carries the colored debug
// tint (`EVOKER_FANGS_BASE` / `EVOKER_FANGS_JAW`) and the textured `uv_size` / `texOffs`; both jaws
// share the one jaw box (texOffs 40,0), differing only by pivot and rotation.

// `base`: the 10×12×10 block.
pub(in crate::entity_models) const EVOKER_FANGS_BASE_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [10.0, 12.0, 10.0],
    EVOKER_FANGS_BASE,
    [10.0, 12.0, 10.0],
    [0.0, 0.0],
    false,
);

// The shared 4×14×8 jaw box (both jaws reuse it, differing only in pivot and rotation).
pub(in crate::entity_models) const EVOKER_FANGS_JAW_CUBE: ModelCube = ModelCube::new(
    [0.0, 0.0, 0.0],
    [4.0, 14.0, 8.0],
    EVOKER_FANGS_JAW,
    [4.0, 14.0, 8.0],
    [40.0, 0.0],
    false,
);

const EVOKER_FANGS_BASE_POSE: PartPose = PartPose {
    offset: [-5.0, 24.0, -5.0],
    rotation: [0.0, 0.0, 0.0],
};
const EVOKER_FANGS_UPPER_JAW_POSE: PartPose = PartPose {
    offset: [6.5, 0.0, 1.0],
    rotation: [0.0, 0.0, 2.042035],
};
const EVOKER_FANGS_LOWER_JAW_POSE: PartPose = PartPose {
    offset: [3.5, 0.0, 9.0],
    rotation: [0.0, std::f32::consts::PI, 4.2411504],
};

/// Evoker-fangs model mirroring vanilla `EvokerFangsModel`: `base` → {`upper_jaw`, `lower_jaw`}, with
/// `setup_anim` driving the bite from the projected `biteProgress`. Each cube carries the colored tint
/// and textured UV.
pub(in crate::entity_models) struct EvokerFangsModel {
    root: ModelPart,
}

impl EvokerFangsModel {
    pub(in crate::entity_models) fn new() -> Self {
        let base = ModelPart::new(
            EVOKER_FANGS_BASE_POSE,
            vec![EVOKER_FANGS_BASE_CUBE],
            vec![
                (
                    "upper_jaw",
                    ModelPart::leaf(EVOKER_FANGS_UPPER_JAW_POSE, vec![EVOKER_FANGS_JAW_CUBE]),
                ),
                (
                    "lower_jaw",
                    ModelPart::leaf(EVOKER_FANGS_LOWER_JAW_POSE, vec![EVOKER_FANGS_JAW_CUBE]),
                ),
            ],
        );
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), vec![("base", base)]),
        }
    }
}

impl EntityModel for EvokerFangsModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        use std::f32::consts::PI;

        // Vanilla `EvokerFangsRenderer` skips the whole model while `biteProgress == 0`
        // (the fang is still underground); hide the subtree to match exactly.
        let bite_progress = instance.render_state.evoker_fangs_bite_progress;
        self.root.visible = bite_progress != 0.0;
        if !self.root.visible {
            return;
        }

        // Vanilla `EvokerFangsModel.setupAnim`: a cubic ease-out `biteAmount` snaps the
        // jaws shut over the first half of the bite (`biteAmount` runs `1 → 0` as
        // `biteProgress` runs `0 → 0.5`), `base.y` lifts the fang out of the ground, and
        // a `preScale` shrinks the whole model to nothing over the final 10%.
        let mut bite_amount = (bite_progress * 2.0).min(1.0);
        bite_amount = 1.0 - bite_amount * bite_amount * bite_amount;
        let base = self.root.child_mut("base");
        base.child_mut("upper_jaw").pose.rotation[2] = PI - bite_amount * 0.35 * PI;
        base.child_mut("lower_jaw").pose.rotation[2] = PI + bite_amount * 0.35 * PI;
        base.pose.offset[1] -= (bite_progress + (bite_progress * 2.7).sin()) * 7.2;

        let pre_scale = if bite_progress > 0.9 {
            (1.0 - bite_progress) / 0.1
        } else {
            1.0
        };
        self.root.pose.offset[1] = 24.0 - 20.0 * pre_scale;
        self.root.scale = [pre_scale; 3];
    }
}
