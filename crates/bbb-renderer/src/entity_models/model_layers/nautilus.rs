use super::{apply_head_look, PartPose, NAUTILUS_BODY, NAUTILUS_SHELL, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 nautilus `ModelLayers`.
pub(in crate::entity_models) const MODEL_LAYER_NAUTILUS: &str = "minecraft:nautilus#main";
pub(in crate::entity_models) const MODEL_LAYER_NAUTILUS_BABY: &str = "minecraft:nautilus_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_NAUTILUS_SADDLE: &str = "minecraft:nautilus#saddle";
pub(in crate::entity_models) const MODEL_LAYER_NAUTILUS_ARMOR: &str =
    "minecraft:nautilus_armor#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_NAUTILUS: &str =
    "minecraft:zombie_nautilus#main";
pub(in crate::entity_models) const MODEL_LAYER_ZOMBIE_NAUTILUS_CORAL: &str =
    "minecraft:zombie_nautilus_coral#main";

// Vanilla 26.1 `NautilusModel.createBodyLayer` (atlas 128×128) — the new rideable nautilus mob.
// `NautilusModel extends EntityModel`: one cubeless `root` pivot parenting the `shell` (the spiral shell,
// three boxes) and the `body` (the trunk plus its 0-thickness rear fin) which in turn parents the three
// mouth boxes. `NautilusModel.setupAnim` steers the `body` by the look — `body.yRot/xRot` set from the
// look yaw/pitch CLAMPED to ±10° — then layers the looping `NautilusAnimation.SWIMMING` keyframe via
// `applyWalk` (always on, the idle baseline `walkAnimationSpeed + 0.2`). The clamped body look is
// reproduced; the SWIMMING keyframe undulation needs the keyframe machinery + an `AnimationState`, so it
// stays deferred (the nautilus renders at this bind pose plus the clamped look). Both the adult
// `createBodyMesh` and the smaller baby `createBabyBodyLayer` are modeled (same `root → shell + body →
// mouths` structure), along with the zombie warm coral variant and the adult saddle/body-armor
// equipment layers. Each unified cube carries both the colored debug tint
// (tan shell over a pale body) and the textured `uv_size` / `texOffs`. Nautilus uses a plain
// `MobRenderer`.

// `shell` cubes: the 14×10×16 dome, the 14×8×20 whorl, and a 14×8×0 rear fin plane.
pub(in crate::entity_models) const NAUTILUS_SHELL_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [-7.0, -10.0, -7.0],
        [14.0, 10.0, 16.0],
        NAUTILUS_SHELL,
        [14.0, 10.0, 16.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-7.0, 0.0, -7.0],
        [14.0, 8.0, 20.0],
        NAUTILUS_SHELL,
        [14.0, 8.0, 20.0],
        [0.0, 26.0],
        false,
    ),
    ModelCube::new(
        [-7.0, 0.0, 6.0],
        [14.0, 8.0, 0.0],
        NAUTILUS_SHELL,
        [14.0, 8.0, 0.0],
        [48.0, 26.0],
        false,
    ),
];

// Vanilla 26.1 `NautilusSaddleModel.createSaddleLayer`: starts from `NautilusModel.createBodyMesh`,
// replaces the `shell` child with the same dome box inflated by `CubeDeformation(0.2F)`, and preserves
// the existing `body` subtree through `PartDefinition.addOrReplaceChild`.
pub(in crate::entity_models) const NAUTILUS_SADDLE_SHELL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-7.2, -10.2, -7.2],
    [14.4, 10.4, 16.4],
    NAUTILUS_SHELL,
    [14.0, 10.0, 16.0],
    [0.0, 0.0],
    false,
)];

// Vanilla 26.1 `NautilusArmorModel.createBodyLayer`: starts from `NautilusModel.createBodyMesh`,
// replaces the adult `shell` child with the three shell boxes; the dome and whorl are inflated by
// `CubeDeformation(0.01F)`, while the zero-thickness rear fin is not inflated.
pub(in crate::entity_models) const NAUTILUS_ARMOR_SHELL_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [-7.01, -10.01, -7.01],
        [14.02, 10.02, 16.02],
        NAUTILUS_SHELL,
        [14.0, 10.0, 16.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-7.01, -0.01, -7.01],
        [14.02, 8.02, 20.02],
        NAUTILUS_SHELL,
        [14.0, 8.0, 20.0],
        [0.0, 26.0],
        false,
    ),
    ModelCube::new(
        [-7.0, 0.0, 6.0],
        [14.0, 8.0, 0.0],
        NAUTILUS_SHELL,
        [14.0, 8.0, 0.0],
        [48.0, 26.0],
        false,
    ),
];

// `body` cubes: the 10×8×14 trunk and a 10×8×0 rear fin plane.
pub(in crate::entity_models) const NAUTILUS_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -4.51, -3.0],
        [10.0, 8.0, 14.0],
        NAUTILUS_BODY,
        [10.0, 8.0, 14.0],
        [0.0, 54.0],
        false,
    ),
    ModelCube::new(
        [-5.0, -4.51, 7.0],
        [10.0, 8.0, 0.0],
        NAUTILUS_BODY,
        [10.0, 8.0, 0.0],
        [0.0, 76.0],
        false,
    ),
];

// The three mouth boxes. Upper/lower use the vanilla `CubeDeformation(-0.001)` (min += 0.001,
// size -= 0.002); the inner mouth is undeformed. The UV-unwrap dimensions stay the UN-inflated
// addBox dims (10×4×4), so the deformed cubes' `uv_size` differs from their (inflated) `size`.
pub(in crate::entity_models) const NAUTILUS_UPPER_MOUTH_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.999, -1.999, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
    [10.0, 4.0, 4.0],
    [54.0, 54.0],
    false,
)];
pub(in crate::entity_models) const NAUTILUS_INNER_MOUTH_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -0.5],
    [6.0, 4.0, 4.0],
    NAUTILUS_BODY,
    [6.0, 4.0, 4.0],
    [54.0, 70.0],
    false,
)];
pub(in crate::entity_models) const NAUTILUS_LOWER_MOUTH_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.999, -1.979, 0.001],
    [9.998, 3.998, 3.998],
    NAUTILUS_BODY,
    [10.0, 4.0, 4.0],
    [54.0, 62.0],
    false,
)];

/// `root` pivot pose: `PartPose.offset(0, 29, -6)`.
pub(in crate::entity_models) const NAUTILUS_ROOT_POSE: PartPose = PartPose {
    offset: [0.0, 29.0, -6.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `shell` part pose: `PartPose.offset(0, -13, 5)`.
pub(in crate::entity_models) const NAUTILUS_SHELL_POSE: PartPose = PartPose {
    offset: [0.0, -13.0, 5.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, -8.5, 12.3)`.
pub(in crate::entity_models) const NAUTILUS_BODY_POSE: PartPose = PartPose {
    offset: [0.0, -8.5, 12.3],
    rotation: [0.0, 0.0, 0.0],
};
/// `upper_mouth` part pose: `PartPose.offset(0, -2.51, 7)`.
pub(in crate::entity_models) const NAUTILUS_UPPER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -2.51, 7.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `inner_mouth` part pose: `PartPose.offset(0, -0.51, 7.5)`.
pub(in crate::entity_models) const NAUTILUS_INNER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -0.51, 7.5],
    rotation: [0.0, 0.0, 0.0],
};
/// `lower_mouth` part pose: `PartPose.offset(0, 1.49, 7)`.
pub(in crate::entity_models) const NAUTILUS_LOWER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, 1.49, 7.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `NautilusModel.createBabyBodyLayer` (atlas 64×64): the same `root → shell + body → three
// mouths` hierarchy as the adult, scaled down to the hatchling proportions.

// Baby `shell` cubes: the 7×4×7 dome, the 7×4×9 whorl, and a 7×4×0 rear fin plane.
pub(in crate::entity_models) const BABY_NAUTILUS_SHELL_CUBES: [ModelCube; 3] = [
    ModelCube::new(
        [-6.0, -4.0, -1.0],
        [7.0, 4.0, 7.0],
        NAUTILUS_SHELL,
        [7.0, 4.0, 7.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-6.0, 0.0, -1.0],
        [7.0, 4.0, 9.0],
        NAUTILUS_SHELL,
        [7.0, 4.0, 9.0],
        [0.0, 11.0],
        false,
    ),
    ModelCube::new(
        [-6.0, 0.0, 5.0],
        [7.0, 4.0, 0.0],
        NAUTILUS_SHELL,
        [7.0, 4.0, 0.0],
        [23.0, 11.0],
        false,
    ),
];

// Baby `body` cubes: the 5×4×7 trunk and a 5×4×0 rear fin plane.
pub(in crate::entity_models) const BABY_NAUTILUS_BODY_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-2.5, -3.01, -1.0],
        [5.0, 4.0, 7.0],
        NAUTILUS_BODY,
        [5.0, 4.0, 7.0],
        [0.0, 24.0],
        false,
    ),
    ModelCube::new(
        [-2.5, -3.01, 4.1],
        [5.0, 4.0, 0.0],
        NAUTILUS_BODY,
        [5.0, 4.0, 0.0],
        [0.0, 35.0],
        false,
    ),
];

// The three baby mouth boxes. Upper/lower use the vanilla `CubeDeformation(-0.001)` (min += 0.001,
// size -= 0.002, on the same 5×2×2 box); the inner mouth is an undeformed 3×2×2. The deformed
// cubes' `uv_size` stays the UN-inflated addBox dims (5×2×2).
pub(in crate::entity_models) const BABY_NAUTILUS_UPPER_MOUTH_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-2.499, -0.999, 0.001],
        [4.998, 1.998, 1.998],
        NAUTILUS_BODY,
        [5.0, 2.0, 2.0],
        [24.0, 24.0],
        false,
    )];
pub(in crate::entity_models) const BABY_NAUTILUS_INNER_MOUTH_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-1.5, -1.0, -1.0],
        [3.0, 2.0, 2.0],
        NAUTILUS_BODY,
        [3.0, 2.0, 2.0],
        [24.0, 32.0],
        false,
    )];
pub(in crate::entity_models) const BABY_NAUTILUS_LOWER_MOUTH_CUBES: [ModelCube; 1] =
    [ModelCube::new(
        [-2.499, -0.999, 0.001],
        [4.998, 1.998, 1.998],
        NAUTILUS_BODY,
        [5.0, 2.0, 2.0],
        [24.0, 28.0],
        false,
    )];

/// Baby `root` pivot pose: `PartPose.offset(-0.5, 28, -0.5)`.
pub(in crate::entity_models) const BABY_NAUTILUS_ROOT_POSE: PartPose = PartPose {
    offset: [-0.5, 28.0, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `shell` part pose: `PartPose.offset(3, -8, -2)`.
pub(in crate::entity_models) const BABY_NAUTILUS_SHELL_POSE: PartPose = PartPose {
    offset: [3.0, -8.0, -2.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `body` part pose: `PartPose.offset(0.5, -5, 3)`.
pub(in crate::entity_models) const BABY_NAUTILUS_BODY_POSE: PartPose = PartPose {
    offset: [0.5, -5.0, 3.0],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `upper_mouth` part pose: `PartPose.offset(0, -2.01, 3.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_UPPER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -2.01, 3.9],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `inner_mouth` part pose: `PartPose.offset(0, -1.01, 4.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_INNER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -1.01, 4.9],
    rotation: [0.0, 0.0, 0.0],
};
/// Baby `lower_mouth` part pose: `PartPose.offset(0, -0.01, 3.9)`.
pub(in crate::entity_models) const BABY_NAUTILUS_LOWER_MOUTH_POSE: PartPose = PartPose {
    offset: [0.0, -0.01, 3.9],
    rotation: [0.0, 0.0, 0.0],
};

/// Builds the cubeless `root` pivot parenting the named `shell` and `body` for the adult or baby
/// geometry. The cube-bearing `body` carries its three mouth boxes as index-named children (only the
/// `body` itself is steered by name in [`NautilusModel::setup_anim`]; the mouths ride along). Both the
/// adult and baby share the identical `root → shell + body → mouths` structure.
fn nautilus_root(baby: bool) -> ModelPart {
    if baby {
        let body = ModelPart::new(
            BABY_NAUTILUS_BODY_POSE,
            BABY_NAUTILUS_BODY_CUBES.to_vec(),
            vec![
                (
                    "0",
                    ModelPart::leaf(
                        BABY_NAUTILUS_UPPER_MOUTH_POSE,
                        BABY_NAUTILUS_UPPER_MOUTH_CUBES.to_vec(),
                    ),
                ),
                (
                    "1",
                    ModelPart::leaf(
                        BABY_NAUTILUS_INNER_MOUTH_POSE,
                        BABY_NAUTILUS_INNER_MOUTH_CUBES.to_vec(),
                    ),
                ),
                (
                    "2",
                    ModelPart::leaf(
                        BABY_NAUTILUS_LOWER_MOUTH_POSE,
                        BABY_NAUTILUS_LOWER_MOUTH_CUBES.to_vec(),
                    ),
                ),
            ],
        );
        ModelPart::new(
            BABY_NAUTILUS_ROOT_POSE,
            Vec::new(),
            vec![
                (
                    "shell",
                    ModelPart::leaf(BABY_NAUTILUS_SHELL_POSE, BABY_NAUTILUS_SHELL_CUBES.to_vec()),
                ),
                ("body", body),
            ],
        )
    } else {
        ModelPart::new(
            NAUTILUS_ROOT_POSE,
            Vec::new(),
            vec![
                (
                    "shell",
                    ModelPart::leaf(NAUTILUS_SHELL_POSE, NAUTILUS_SHELL_CUBES.to_vec()),
                ),
                ("body", nautilus_adult_body()),
            ],
        )
    }
}

/// The adult `body` subtree (`NautilusModel.createBodyMesh`): the cube-bearing body parenting its three
/// index-named mouth boxes. Shared by the plain adult nautilus and the zombie nautilus coral variant.
fn nautilus_adult_body() -> ModelPart {
    ModelPart::new(
        NAUTILUS_BODY_POSE,
        NAUTILUS_BODY_CUBES.to_vec(),
        vec![
            (
                "0",
                ModelPart::leaf(
                    NAUTILUS_UPPER_MOUTH_POSE,
                    NAUTILUS_UPPER_MOUTH_CUBES.to_vec(),
                ),
            ),
            (
                "1",
                ModelPart::leaf(
                    NAUTILUS_INNER_MOUTH_POSE,
                    NAUTILUS_INNER_MOUTH_CUBES.to_vec(),
                ),
            ),
            (
                "2",
                ModelPart::leaf(
                    NAUTILUS_LOWER_MOUTH_POSE,
                    NAUTILUS_LOWER_MOUTH_CUBES.to_vec(),
                ),
            ),
        ],
    )
}

// Vanilla 26.1 `ZombieNautilusCoralModel.createBodyLayer` (atlas 128×128): the adult `NautilusModel`
// body PLUS a `corals` subtree under `shell` — four coral clusters of textured-only zero-thickness
// cross-planes. These render textured-only (the zombie nautilus is texture-backed, so the colored
// path is skipped), so `color` is the unused [`NAUTILUS_SHELL`] placeholder; `uv_size` equals `size`.
const fn coral_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, NAUTILUS_SHELL, size, tex, false)
}

/// A textured-only coral cross-plane leaf at `offset`/`rotation` carrying one `cube`.
fn coral_part(offset: [f32; 3], rotation: [f32; 3], cube: ModelCube) -> ModelPart {
    ModelPart::leaf(PartPose { offset, rotation }, vec![cube])
}

/// Builds the `corals` subtree (vanilla `ZombieNautilusCoralModel.createBodyLayer`), parented under the
/// adult `shell`. The four clusters keep the vanilla child order; the cross-planes are billboards
/// (`y`-rotated ±π/4, the pink pair `z`-rotated π/2). The `corals.visible = bodyArmorItem.isEmpty()`
/// gate is applied in [`NautilusModel::setup_anim`].
fn nautilus_corals() -> ModelPart {
    let yellow = ModelPart::new(
        PartPose {
            offset: [0.0, -11.0, 11.0],
            rotation: [0.0, 0.0, 0.0],
        },
        Vec::new(),
        vec![
            (
                "yellow_coral_second",
                coral_part(
                    [0.0, 0.0, 2.0],
                    [0.0, -0.7854, 0.0],
                    coral_cube([-4.5, -3.5, 0.0], [6.0, 8.0, 0.0], [0.0, 85.0]),
                ),
            ),
            (
                "yellow_coral_first",
                coral_part(
                    [0.0, 0.0, 0.0],
                    [0.0, 0.7854, 0.0],
                    coral_cube([-4.5, -3.5, 0.0], [6.0, 8.0, 0.0], [0.0, 85.0]),
                ),
            ),
        ],
    );
    let pink = ModelPart::new(
        PartPose {
            offset: [-12.5, -18.0, 11.0],
            rotation: [0.0, 0.0, 0.0],
        },
        vec![coral_cube([-4.5, 4.5, 0.0], [6.0, 0.0, 8.0], [-8.0, 94.0])],
        vec![(
            "pink_coral_second",
            coral_part(
                [-1.5, 4.5, 4.0],
                [0.0, 0.0, 1.5708],
                coral_cube([-3.0, 0.0, -4.0], [6.0, 0.0, 8.0], [-8.0, 94.0]),
            ),
        )],
    );
    let blue = ModelPart::new(
        PartPose {
            offset: [-14.0, 0.0, 5.5],
            rotation: [0.0, 0.0, 0.0],
        },
        Vec::new(),
        vec![
            (
                "blue_second",
                coral_part(
                    [0.0, 0.0, -2.0],
                    [0.0, 0.7854, 0.0],
                    coral_cube([-3.5, -5.5, 0.0], [5.0, 10.0, 0.0], [0.0, 102.0]),
                ),
            ),
            (
                "blue_first",
                coral_part(
                    [0.0, 0.0, 0.0],
                    [0.0, -0.7854, 0.0],
                    coral_cube([-3.5, -5.5, 0.0], [5.0, 10.0, 0.0], [0.0, 102.0]),
                ),
            ),
        ],
    );
    let red = ModelPart::new(
        PartPose {
            offset: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        Vec::new(),
        vec![
            (
                "red_coral_second",
                coral_part(
                    [-0.5, -1.0, 1.5],
                    [0.0, -0.829, 0.0],
                    coral_cube([-2.5, -5.5, 0.0], [4.0, 10.0, 0.0], [0.0, 112.0]),
                ),
            ),
            (
                "red_coral_first",
                coral_part(
                    [0.0, 0.0, 0.0],
                    [0.0, 0.7854, 0.0],
                    coral_cube([-4.5, -5.5, 0.0], [6.0, 10.0, 0.0], [0.0, 112.0]),
                ),
            ),
        ],
    );
    ModelPart::new(
        PartPose {
            offset: [8.0, 4.5, -8.0],
            rotation: [0.0, 0.0, 0.0],
        },
        Vec::new(),
        vec![
            ("yellow_coral", yellow),
            ("pink_coral", pink),
            ("blue_coral", blue),
            ("red_coral", red),
        ],
    )
}

/// Builds the zombie nautilus coral root: the adult `root → shell + body` tree with the `corals`
/// subtree added under `shell` (vanilla `ZombieNautilusCoralModel.createBodyLayer`).
fn nautilus_coral_root() -> ModelPart {
    let shell = ModelPart::new(
        NAUTILUS_SHELL_POSE,
        NAUTILUS_SHELL_CUBES.to_vec(),
        vec![("corals", nautilus_corals())],
    );
    ModelPart::new(
        NAUTILUS_ROOT_POSE,
        Vec::new(),
        vec![("shell", shell), ("body", nautilus_adult_body())],
    )
}

/// Builds the adult nautilus saddle layer tree: the full adult body mesh with the shell replaced by
/// the single inflated `NautilusSaddleModel` shell cube. Vanilla supplies no baby saddle model.
fn nautilus_saddle_root() -> ModelPart {
    ModelPart::new(
        NAUTILUS_ROOT_POSE,
        Vec::new(),
        vec![
            (
                "shell",
                ModelPart::leaf(NAUTILUS_SHELL_POSE, NAUTILUS_SADDLE_SHELL_CUBES.to_vec()),
            ),
            ("body", nautilus_adult_body()),
        ],
    )
}

/// Builds the adult nautilus armor layer tree: the full adult body mesh with the shell replaced by
/// the lightly inflated `NautilusArmorModel` shell. Vanilla supplies no baby armor model.
fn nautilus_armor_root() -> ModelPart {
    ModelPart::new(
        NAUTILUS_ROOT_POSE,
        Vec::new(),
        vec![
            (
                "shell",
                ModelPart::leaf(NAUTILUS_SHELL_POSE, NAUTILUS_ARMOR_SHELL_CUBES.to_vec()),
            ),
            ("body", nautilus_adult_body()),
        ],
    )
}

/// Vanilla `NautilusModel.applyBodyRotation` clamps the look yaw/pitch to ±10° before steering the body.
const NAUTILUS_LOOK_CLAMP_DEGREES: f32 = 10.0;

/// Mutable nautilus model, mirroring vanilla `NautilusModel` (`baby` selecting the adult
/// `createBodyMesh` or the smaller `createBabyBodyLayer` geometry). The named hierarchy
/// (`root → shell + body → upper_mouth/inner_mouth/lower_mouth`) hangs off a synthetic root, built
/// from the baked geometry. `setup_anim` steers the body by the clamped look
/// ([`apply_head_look`] on the `body` part, with the yaw/pitch pre-clamped to ±10°) via `child_mut`;
/// the SWIMMING keyframe undulation and every other pose stay deferred.
pub(in crate::entity_models) struct NautilusModel {
    root: ModelPart,
    has_corals: bool,
}

impl NautilusModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("root", nautilus_root(baby))],
            ),
            has_corals: false,
        }
    }

    /// The zombie nautilus `WARM` coral variant (vanilla `ZombieNautilusCoralModel`): the adult body
    /// with the `corals` subtree. Always adult and shares [`NautilusModel::setup_anim`] (the corals
    /// hang off `shell`, away from the steered `body`, so they ride the body rotation unchanged).
    pub(in crate::entity_models) fn new_coral() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("root", nautilus_coral_root())],
            ),
            has_corals: true,
        }
    }

    /// Vanilla `NautilusSaddleModel(ModelLayers.NAUTILUS_SADDLE)`: the adult saddle equipment layer.
    /// It keeps the adult body subtree from `NautilusModel.createBodyMesh` and replaces the shell with
    /// the inflated saddle shell; no baby saddle model exists.
    pub(in crate::entity_models) fn new_saddle() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("root", nautilus_saddle_root())],
            ),
            has_corals: false,
        }
    }

    /// Vanilla `NautilusArmorModel(ModelLayers.NAUTILUS_ARMOR)`: the adult body-armor equipment layer.
    /// It keeps the adult body subtree from `NautilusModel.createBodyMesh` and replaces the shell with
    /// the lightly inflated armor shell; no baby armor model exists.
    pub(in crate::entity_models) fn new_armor() -> Self {
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![("root", nautilus_armor_root())],
            ),
            has_corals: false,
        }
    }
}

impl EntityModel for NautilusModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `NautilusModel.applyBodyRotation`: the body (root → body) turns to the look, clamped
        // to ±10° on each axis. `apply_head_look` sets `xRot`/`yRot` from the (pre-clamped) degrees.
        let render_state = &instance.render_state;
        let body = self.root.child_mut("root").child_mut("body");
        apply_head_look(
            body,
            render_state
                .head_yaw
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
            render_state
                .head_pitch
                .clamp(-NAUTILUS_LOOK_CLAMP_DEGREES, NAUTILUS_LOOK_CLAMP_DEGREES),
        );
        if self.has_corals {
            self.root
                .child_mut("root")
                .child_mut("shell")
                .child_mut("corals")
                .visible = render_state.nautilus_body_armor.is_none();
        }
    }
}
