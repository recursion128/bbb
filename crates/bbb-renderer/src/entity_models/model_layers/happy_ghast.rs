use super::{ghast_tentacle_x_rot, PartPose, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Happy ghasts are a warm cream jelly; the colored fallback paints every cube the same pale
// cream so the silhouette reads even without the texture.
pub(in crate::entity_models) const HAPPY_GHAST_CREAM: [f32; 4] = [0.96, 0.92, 0.74, 1.0];

pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST: &str = "minecraft:happy_ghast#main";
pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST_BABY: &str =
    "minecraft:happy_ghast_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST_HARNESS: &str =
    "minecraft:happy_ghast_harness#main";
pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST_BABY_HARNESS: &str =
    "minecraft:happy_ghast_baby_harness#main";
pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST_ROPES: &str =
    "minecraft:happy_ghast_ropes#main";
pub(in crate::entity_models) const MODEL_LAYER_HAPPY_GHAST_BABY_ROPES: &str =
    "minecraft:happy_ghast_baby_ropes#main";

/// Vanilla `LayerDefinitions`: both happy-ghast rope model layers use
/// `HappyGhastModel.createBodyLayer(..., new CubeDeformation(0.2F))`.
pub(in crate::entity_models) const HAPPY_GHAST_ROPES_DEFORMATION: f32 = 0.2;

/// The nine happy-ghast tentacle lengths, baked verbatim from vanilla 26.1
/// `HappyGhastModel.createBodyLayer` (each `addBox(-1, 0, -1, 2, len, 2)`). Unlike the regular
/// ghast (random lengths), the happy ghast hard-codes them.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_LENGTHS: [f32; 9] =
    [5.0, 7.0, 4.0, 5.0, 5.0, 7.0, 8.0, 8.0, 5.0];

/// The nine tentacle root offsets under the body. Keeping the vanilla parent relationship is
/// required because equipping any BODY item scales the complete body subtree by 0.9375.
pub(in crate::entity_models) const HAPPY_GHAST_TENTACLE_OFFSETS: [[f32; 3]; 9] = [
    [-3.75, 7.0, -5.0],
    [1.25, 7.0, -5.0],
    [6.25, 7.0, -5.0],
    [-6.25, 7.0, 0.0],
    [-1.25, 7.0, 0.0],
    [3.75, 7.0, 0.0],
    [-3.75, 7.0, 5.0],
    [1.25, 7.0, 5.0],
    [6.25, 7.0, 5.0],
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

/// Baby-only inset body cube (`inner_body`): vanilla uses the same logical 16px UV dimensions with
/// a `-0.5` deformation and offsets it by eight pixels below the outer body origin.
pub(in crate::entity_models) const HAPPY_GHAST_BABY_INNER_BODY_CUBE: [ModelCube; 1] =
    [ModelCube::new(
        [-7.5, -15.5, -7.5],
        [15.0, 15.0, 15.0],
        HAPPY_GHAST_CREAM,
        [16.0, 16.0, 16.0],
        [0.0, 32.0],
        false,
    )];

pub(in crate::entity_models) const HAPPY_GHAST_BABY_INNER_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 8.0, 0.0],
    rotation: [0.0; 3],
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
/// [`HAPPY_GHAST_TENTACLE_OFFSETS`]`[i]` (`y = 7.0` under the body) with no rotation. The `xRot` wave is `0.4` at
/// age 0 and overwritten each frame by `setup_anim`.
pub(in crate::entity_models) fn happy_ghast_tentacle_pose(index: usize) -> PartPose {
    PartPose {
        offset: HAPPY_GHAST_TENTACLE_OFFSETS[index],
        rotation: [0.0, 0.0, 0.0],
    }
}

/// Mutable happy ghast model, mirroring vanilla `HappyGhastModel`. The unified tree is built once
/// with the original `body -> inner_body/tentacle*` hierarchy. `setup_anim` reuses
/// `GhastModel.animateTentacles` verbatim ([`ghast_tentacle_x_rot`], never at rest). The harness
/// body-item squeeze (`0.9375` scale when equipped) applies to the body subtree; age scaling lives
/// in the root transform (`happy_ghast_model_root_transform`).
pub(in crate::entity_models) struct HappyGhastModel {
    root: ModelPart,
}

impl HappyGhastModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self::with_deformation(baby, 0.0)
    }

    pub(in crate::entity_models) fn new_ropes(baby: bool) -> Self {
        Self::with_deformation(baby, HAPPY_GHAST_ROPES_DEFORMATION)
    }

    fn with_deformation(baby: bool, deformation: f32) -> Self {
        let mut body_children: Vec<(&'static str, ModelPart)> = Vec::with_capacity(10);
        if baby {
            body_children.push((
                "inner_body",
                ModelPart::leaf(
                    HAPPY_GHAST_BABY_INNER_BODY_POSE,
                    deformed_cubes(&HAPPY_GHAST_BABY_INNER_BODY_CUBE, deformation),
                ),
            ));
        }
        for (i, &name) in HAPPY_GHAST_TENTACLE_NAMES.iter().enumerate() {
            body_children.push((
                name,
                ModelPart::leaf(
                    happy_ghast_tentacle_pose(i),
                    vec![deformed_cube(happy_ghast_tentacle_cube(i), deformation)],
                ),
            ));
        }
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![(
                    "body",
                    ModelPart::new(
                        HAPPY_GHAST_BODY_POSE,
                        deformed_cubes(&HAPPY_GHAST_BODY_CUBE, deformation),
                        body_children,
                    ),
                )],
            ),
        }
    }
}

fn deformed_cubes(cubes: &[ModelCube], deformation: f32) -> Vec<ModelCube> {
    cubes
        .iter()
        .copied()
        .map(|cube| deformed_cube(cube, deformation))
        .collect()
}

fn deformed_cube(cube: ModelCube, deformation: f32) -> ModelCube {
    ModelCube {
        min: [
            cube.min[0] - deformation,
            cube.min[1] - deformation,
            cube.min[2] - deformation,
        ],
        size: [
            cube.size[0] + deformation * 2.0,
            cube.size[1] + deformation * 2.0,
            cube.size[2] + deformation * 2.0,
        ],
        ..cube
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
        let body = self.root.child_mut("body");
        if instance.render_state.happy_ghast_body_item {
            body.scale = [0.9375; 3];
        }
        for (i, &name) in HAPPY_GHAST_TENTACLE_NAMES.iter().enumerate() {
            body.child_mut(name).pose.rotation[0] = ghast_tentacle_x_rot(i, age_in_ticks);
        }
    }
}

pub(in crate::entity_models) const HAPPY_GHAST_HARNESS_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -16.0, -8.0],
    [16.0, 16.0, 16.0],
    [1.0; 4],
    [16.0, 16.0, 16.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const HAPPY_GHAST_GOGGLES_CUBE: [ModelCube; 1] = [ModelCube::new(
    [-8.15, -2.65, -2.65],
    [16.3, 5.3, 5.3],
    [1.0; 4],
    [16.0, 5.0, 5.0],
    [0.0, 32.0],
    false,
)];

pub(in crate::entity_models) struct HappyGhastHarnessModel {
    root: ModelPart,
}

impl HappyGhastHarnessModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    (
                        "harness",
                        ModelPart::leaf(
                            PartPose {
                                offset: [0.0, 24.0, 0.0],
                                rotation: [0.0; 3],
                            },
                            HAPPY_GHAST_HARNESS_CUBE.to_vec(),
                        ),
                    ),
                    (
                        "goggles",
                        ModelPart::leaf(
                            PartPose {
                                offset: [0.0, 9.0, -5.5],
                                rotation: [-0.7854, 0.0, 0.0],
                            },
                            HAPPY_GHAST_GOGGLES_CUBE.to_vec(),
                        ),
                    ),
                ],
            ),
        }
    }
}

impl EntityModel for HappyGhastHarnessModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        if instance.render_state.happy_ghast_ridden {
            let goggles = self.root.child_mut("goggles");
            goggles.pose.offset[1] = 14.0;
            goggles.pose.rotation[0] = 0.0;
        }
    }
}
