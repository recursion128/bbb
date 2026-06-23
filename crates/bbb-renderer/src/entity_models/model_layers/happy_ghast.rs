use super::{ghast_tentacle_x_rot, PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Happy ghasts are a warm cream jelly; the colored fallback paints every cube the same pale
// cream so the silhouette reads even without the texture.
pub(in crate::entity_models) const HAPPY_GHAST_CREAM: [f32; 4] = [0.96, 0.92, 0.74, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST: &str = "minecraft:happy_ghast#main";

/// The nine happy-ghast tentacle lengths, baked verbatim from vanilla 26.1
/// `HappyGhastModel.createBodyLayer` (each `addBox(-1, 0, -1, 2, len, 2)`). Unlike the regular
/// ghast (random lengths), the happy ghast hard-codes them.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_LENGTHS: [f32; 9] =
    [5.0, 7.0, 4.0, 5.0, 5.0, 7.0, 8.0, 8.0, 5.0];

/// The nine tentacle root offsets `[xo, 23.0, yo]`. Vanilla parents the tentacles under the
/// body (`PartPose.offset(0, 16, 0)`) at `PartPose.offset(xo, 7.0, yo)`, so the world-space
/// offset is `[xo, 16 + 7, yo]`. The body carries no rotation (and, for an unharnessed happy
/// ghast, no scale), so flattening the tentacles to absolute offsets is exact.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_OFFSETS: [[f32; 3]; 9] = [
    [-3.75, 23.0, -5.0],
    [1.25, 23.0, -5.0],
    [6.25, 23.0, -5.0],
    [-6.25, 23.0, 0.0],
    [-1.25, 23.0, 0.0],
    [3.75, 23.0, 0.0],
    [-3.75, 23.0, 5.0],
    [1.25, 23.0, 5.0],
    [6.25, 23.0, 5.0],
];

/// Vanilla `HappyGhastModel.createBodyLayer` body cube: a 16×16×16 box at `texOffs(0, 0)`. The
/// unified cube carries the colored tint (`HAPPY_GHAST_CREAM`) and the textured `uv_size`/`texOffs`
/// in one struct.
pub(in crate::entity_models) const HAPPY_GHAST_BODY_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -8.0, -8.0],
    [16.0, 16.0, 16.0],
    HAPPY_GHAST_CREAM,
    [16.0, 16.0, 16.0],
    [0.0, 0.0],
    false,
)];

/// The happy ghast body pose: `PartPose.offset(0, 16, 0)`.
pub(in crate::entity_models) const HAPPY_GHAST_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 16.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `HappyGhastModel.createBodyLayer` tentacle child names, in `tentacle0..tentacle8` order;
/// `child_mut` needs `&'static` names, so the procedural ring draws its names from this const array.
const HAPPY_GHAST_TENTACLE_NAMES: [&str; 9] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
    "tentacle8",
];

/// Bind cube of tentacle `i`: vanilla `addBox(-1, 0, -1, 2, len, 2)` at `texOffs(0, 0)` (reused for
/// the body and every tentacle, so each samples the same top-left region of the 64×64 texture). The
/// length comes from [`HAPPY_GHAST_TENTACLE_LENGTHS`]; `uv_size == size` (no deformation).
pub(in crate::entity_models) fn happy_ghast_tentacle_cube(index: usize) -> ModelCube {
    let len = HAPPY_GHAST_TENTACLE_LENGTHS[index];
    ModelCube::new(
        [-1.0, 0.0, -1.0],
        [2.0, len, 2.0],
        HAPPY_GHAST_CREAM,
        [2.0, len, 2.0],
        [0.0, 0.0],
        false,
    )
}

/// Bind pose of tentacle `i`: vanilla `HappyGhastModel.createBodyLayer` hangs it at
/// [`HAPPY_GHAST_TENTACLE_OFFSETS`]`[i]` (`y = 23.0`) with no rotation. The `xRot` wave is `0.4` at
/// age 0 and overwritten each frame by `setup_anim`.
pub(in crate::entity_models) fn happy_ghast_tentacle_pose(index: usize) -> PartPose {
    PartPose {
        offset: HAPPY_GHAST_TENTACLE_OFFSETS[index],
        rotation: [0.0, 0.0, 0.0],
    }
}

/// Mutable happy ghast model, mirroring vanilla `HappyGhastModel`. The unified tree is built once
/// with named children: `body` plus `tentacle0..tentacle8`. `setup_anim` reuses
/// `GhastModel.animateTentacles` verbatim ([`ghast_tentacle_x_rot`], never at rest). The harness
/// body-item squeeze (`0.9375` scale when equipped) is deferred with the equipment layer, so an
/// unharnessed happy ghast renders at full scale; the bob/scale lives in the root transform
/// (`happy_ghast_model_root_transform`).
pub(in crate::entity_models) struct HappyGhastModel {
    root: ModelPart,
}

impl HappyGhastModel {
    pub(in crate::entity_models) fn new() -> Self {
        let mut children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(10);
        children.push((
            "body",
            ModelPart::leaf(HAPPY_GHAST_BODY_POSE, HAPPY_GHAST_BODY_CUBE.to_vec()),
        ));
        for (i, &name) in HAPPY_GHAST_TENTACLE_NAMES.iter().enumerate() {
            children.push((
                name,
                ModelPart::leaf(
                    happy_ghast_tentacle_pose(i),
                    vec![happy_ghast_tentacle_cube(i)],
                ),
            ));
        }
        Self {
            root: ModelPart::new(PART_POSE_ZERO, Vec::new(), children),
        }
    }
}

impl EntityModel for HappyGhastModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let age_in_ticks = instance.render_state.age_in_ticks;
        for (i, &name) in HAPPY_GHAST_TENTACLE_NAMES.iter().enumerate() {
            self.root.child_mut(name).pose.rotation[0] = ghast_tentacle_x_rot(i, age_in_ticks);
        }
    }
}
