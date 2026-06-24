use super::{PartPose, DRAGON_BODY, DRAGON_MEMBRANE, PART_POSE_ZERO};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 `EnderDragonModel.createBodyLayer` (atlas 256×256). The mesh root holds the head
// (parenting the jaw), the five neck segments and twelve tail segments (each the shared 10×10×10
// spine box plus its 2×4×6 dorsal scale), and the body (parenting the two wings — each a bone box
// plus a 56×0×56 membrane plane and a wing tip — and the four three-segment legs). The whole
// `EnderDragonModel.setupAnim` is procedural: every neck/tail segment is re-placed from the
// `DragonFlightHistory` path each frame, the wings flap (`flapTime`), the jaw opens, and the root
// gets the `bounce` y / fixed `z = -48` / `xRot` adjustments. All of that is deferred (like the
// guardian's procedural tail), so the model renders at this straight bind layout. `EnderDragonRenderer`
// applies the flight-history yaw, a pitch, a fixed `translate(0, 0, 1)`, and the standard flip /
// y-offset (captured by `ender_dragon_model_root_transform`). The dying dissolve, the emissive eyes
// layer, the crystal-healing beam, and the texture-backed path are deferred, so the colored debug
// path renders the body dark and the wing membranes a lighter tint. Each unified cube carries both
// the colored debug tint (`DRAGON_BODY` / `DRAGON_MEMBRANE`) and the textured `uv_size` / `texOffs`.

// ----- Head + jaw -----

// `head` (offset (0, 20, -62)): the upper lip, the upper head, and the mirrored scale/nostril pairs.
pub(in crate::entity_models) const DRAGON_HEAD_CUBES: [ModelCube; 6] = [
    ModelCube::new(
        [-6.0, -1.0, -24.0],
        [12.0, 5.0, 16.0],
        DRAGON_BODY,
        [12.0, 5.0, 16.0],
        [176.0, 44.0],
        false,
    ),
    ModelCube::new(
        [-8.0, -8.0, -10.0],
        [16.0, 16.0, 16.0],
        DRAGON_BODY,
        [16.0, 16.0, 16.0],
        [112.0, 30.0],
        false,
    ),
    ModelCube::new(
        [-5.0, -12.0, -4.0],
        [2.0, 4.0, 6.0],
        DRAGON_BODY,
        [2.0, 4.0, 6.0],
        [0.0, 0.0],
        true,
    ),
    ModelCube::new(
        [-5.0, -3.0, -22.0],
        [2.0, 2.0, 4.0],
        DRAGON_BODY,
        [2.0, 2.0, 4.0],
        [112.0, 0.0],
        true,
    ),
    ModelCube::new(
        [3.0, -12.0, -4.0],
        [2.0, 4.0, 6.0],
        DRAGON_BODY,
        [2.0, 4.0, 6.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [3.0, -3.0, -22.0],
        [2.0, 2.0, 4.0],
        DRAGON_BODY,
        [2.0, 2.0, 4.0],
        [112.0, 0.0],
        false,
    ),
];

// `jaw` (offset (0, 4, -8)): the lower jaw box.
pub(in crate::entity_models) const DRAGON_JAW_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-6.0, 0.0, -16.0],
    [12.0, 4.0, 16.0],
    DRAGON_BODY,
    [12.0, 4.0, 16.0],
    [176.0, 65.0],
    false,
)];

// ----- Shared spine segment (necks and tails) -----

// The 10×10×10 vertebra box plus its 2×4×6 dorsal scale.
pub(in crate::entity_models) const DRAGON_SPINE_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-5.0, -5.0, -5.0],
        [10.0, 10.0, 10.0],
        DRAGON_BODY,
        [10.0, 10.0, 10.0],
        [192.0, 104.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -9.0, -3.0],
        [2.0, 4.0, 6.0],
        DRAGON_BODY,
        [2.0, 4.0, 6.0],
        [48.0, 0.0],
        false,
    ),
];

// ----- Body + wings + legs -----

// `body` (offset (0, 3, 8)): the 24×24×64 torso plus the three dorsal scales.
pub(in crate::entity_models) const DRAGON_BODY_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [-12.0, 1.0, -16.0],
        [24.0, 24.0, 64.0],
        DRAGON_BODY,
        [24.0, 24.0, 64.0],
        [0.0, 0.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -5.0, -10.0],
        [2.0, 6.0, 12.0],
        DRAGON_BODY,
        [2.0, 6.0, 12.0],
        [220.0, 53.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -5.0, 10.0],
        [2.0, 6.0, 12.0],
        DRAGON_BODY,
        [2.0, 6.0, 12.0],
        [220.0, 53.0],
        false,
    ),
    ModelCube::new(
        [-1.0, -5.0, 30.0],
        [2.0, 6.0, 12.0],
        DRAGON_BODY,
        [2.0, 6.0, 12.0],
        [220.0, 53.0],
        false,
    ),
];

// The wings: each a 56×8×8 bone plus a 56×0×56 membrane plane, parenting a 56×4×4 tip bone plus its
// own membrane. Left wings extend +X, right wings extend -X (vanilla's mirror is true geometry here).
pub(in crate::entity_models) const DRAGON_LEFT_WING_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [0.0, -4.0, -4.0],
        [56.0, 8.0, 8.0],
        DRAGON_BODY,
        [56.0, 8.0, 8.0],
        [112.0, 88.0],
        true,
    ),
    ModelCube::new(
        [0.0, 0.0, 2.0],
        [56.0, 0.0, 56.0],
        DRAGON_MEMBRANE,
        [56.0, 0.0, 56.0],
        [-56.0, 88.0],
        true,
    ),
];
pub(in crate::entity_models) const DRAGON_LEFT_WING_TIP_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [0.0, -2.0, -2.0],
        [56.0, 4.0, 4.0],
        DRAGON_BODY,
        [56.0, 4.0, 4.0],
        [112.0, 136.0],
        true,
    ),
    ModelCube::new(
        [0.0, 0.0, 2.0],
        [56.0, 0.0, 56.0],
        DRAGON_MEMBRANE,
        [56.0, 0.0, 56.0],
        [-56.0, 144.0],
        true,
    ),
];
pub(in crate::entity_models) const DRAGON_RIGHT_WING_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-56.0, -4.0, -4.0],
        [56.0, 8.0, 8.0],
        DRAGON_BODY,
        [56.0, 8.0, 8.0],
        [112.0, 88.0],
        false,
    ),
    ModelCube::new(
        [-56.0, 0.0, 2.0],
        [56.0, 0.0, 56.0],
        DRAGON_MEMBRANE,
        [56.0, 0.0, 56.0],
        [-56.0, 88.0],
        false,
    ),
];
pub(in crate::entity_models) const DRAGON_RIGHT_WING_TIP_CUBES: [ModelCube; 2] = [
    ModelCube::new(
        [-56.0, -2.0, -2.0],
        [56.0, 4.0, 4.0],
        DRAGON_BODY,
        [56.0, 4.0, 4.0],
        [112.0, 136.0],
        false,
    ),
    ModelCube::new(
        [-56.0, 0.0, 2.0],
        [56.0, 0.0, 56.0],
        DRAGON_MEMBRANE,
        [56.0, 0.0, 56.0],
        [-56.0, 144.0],
        false,
    ),
];

// The legs: front and hind, each a leg → leg-tip → foot chain. The cubes are centred, so the left
// and right legs of each pair share geometry and reuse the same child hierarchies (only the body
// pivot X differs).
pub(in crate::entity_models) const DRAGON_FRONT_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -4.0, -4.0],
    [8.0, 24.0, 8.0],
    DRAGON_BODY,
    [8.0, 24.0, 8.0],
    [112.0, 104.0],
    false,
)];
pub(in crate::entity_models) const DRAGON_FRONT_LEG_TIP_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -1.0, -3.0],
    [6.0, 24.0, 6.0],
    DRAGON_BODY,
    [6.0, 24.0, 6.0],
    [226.0, 138.0],
    false,
)];
pub(in crate::entity_models) const DRAGON_FRONT_FOOT_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, 0.0, -12.0],
    [8.0, 4.0, 16.0],
    DRAGON_BODY,
    [8.0, 4.0, 16.0],
    [144.0, 104.0],
    false,
)];
pub(in crate::entity_models) const DRAGON_HIND_LEG_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-8.0, -4.0, -8.0],
    [16.0, 32.0, 16.0],
    DRAGON_BODY,
    [16.0, 32.0, 16.0],
    [0.0, 0.0],
    false,
)];
pub(in crate::entity_models) const DRAGON_HIND_LEG_TIP_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-6.0, -2.0, 0.0],
    [12.0, 32.0, 12.0],
    DRAGON_BODY,
    [12.0, 32.0, 12.0],
    [196.0, 0.0],
    false,
)];
pub(in crate::entity_models) const DRAGON_HIND_FOOT_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-9.0, 0.0, -20.0],
    [18.0, 6.0, 24.0],
    DRAGON_BODY,
    [18.0, 6.0, 24.0],
    [112.0, 0.0],
    false,
)];

/// `head` part pose: `PartPose.offset(0, 20, -62)`.
pub(in crate::entity_models) const DRAGON_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 20.0, -62.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `jaw` part pose: `PartPose.offset(0, 4, -8)`.
pub(in crate::entity_models) const DRAGON_JAW_POSE: PartPose = PartPose {
    offset: [0.0, 4.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};
/// `body` part pose: `PartPose.offset(0, 3, 8)`.
pub(in crate::entity_models) const DRAGON_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 3.0, 8.0],
    rotation: [0.0, 0.0, 0.0],
};

/// A neck/tail spine segment at `offset`, carrying a fresh clone of the shared spine cubes.
fn dragon_spine_part(offset: [f32; 3]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        DRAGON_SPINE_CUBES.to_vec(),
    )
}

/// A front leg → leg-tip → foot chain hung at the body pivot `offset`. The cubes are centred, so the
/// left and right front legs reuse this builder (only the pivot X differs).
fn dragon_front_leg(offset: [f32; 3]) -> ModelPart {
    let foot = ModelPart::new(
        PartPose {
            offset: [0.0, 23.0, 0.0],
            rotation: [0.75, 0.0, 0.0],
        },
        DRAGON_FRONT_FOOT_CUBES.to_vec(),
        Vec::new(),
    );
    let leg_tip = ModelPart::new(
        PartPose {
            offset: [0.0, 20.0, -1.0],
            rotation: [-0.5, 0.0, 0.0],
        },
        DRAGON_FRONT_LEG_TIP_CUBES.to_vec(),
        vec![("0", foot)],
    );
    ModelPart::new(
        PartPose {
            offset,
            rotation: [1.3, 0.0, 0.0],
        },
        DRAGON_FRONT_LEG_CUBES.to_vec(),
        vec![("0", leg_tip)],
    )
}

/// A hind leg → leg-tip → foot chain hung at the body pivot `offset`. The cubes are centred, so the
/// left and right hind legs reuse this builder (only the pivot X differs).
fn dragon_hind_leg(offset: [f32; 3]) -> ModelPart {
    let foot = ModelPart::new(
        PartPose {
            offset: [0.0, 31.0, 4.0],
            rotation: [0.75, 0.0, 0.0],
        },
        DRAGON_HIND_FOOT_CUBES.to_vec(),
        Vec::new(),
    );
    let leg_tip = ModelPart::new(
        PartPose {
            offset: [0.0, 32.0, -4.0],
            rotation: [0.5, 0.0, 0.0],
        },
        DRAGON_HIND_LEG_TIP_CUBES.to_vec(),
        vec![("0", foot)],
    );
    ModelPart::new(
        PartPose {
            offset,
            rotation: [1.0, 0.0, 0.0],
        },
        DRAGON_HIND_LEG_CUBES.to_vec(),
        vec![("0", leg_tip)],
    )
}

/// A wing → wing-tip chain. `wing_cubes` / `tip_cubes` differ between the left (+X) and right (-X)
/// wings; the tip hangs at the bone's far end (`tip_offset`).
fn dragon_wing(
    offset: [f32; 3],
    wing_cubes: &[ModelCube],
    tip_offset: [f32; 3],
    tip_cubes: &[ModelCube],
) -> ModelPart {
    let tip = ModelPart::new(
        PartPose {
            offset: tip_offset,
            rotation: [0.0, 0.0, 0.0],
        },
        tip_cubes.to_vec(),
        Vec::new(),
    );
    ModelPart::new(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        wing_cubes.to_vec(),
        vec![("0", tip)],
    )
}

/// Static ender-dragon model mirroring vanilla `EnderDragonModel` at its `createBodyLayer` straight
/// bind layout: the mesh root holds the head (parenting the jaw), the five neck and twelve tail spine
/// segments, and the body (parenting the two wings and four legs). The whole `setupAnim` is
/// procedural and deferred to `ender_dragon_model_root_transform`, so the model has no per-frame
/// `setup_anim`. Each cube carries the colored tint and the textured UV.
pub(in crate::entity_models) struct EnderDragonModel {
    root: ModelPart,
}

impl EnderDragonModel {
    pub(in crate::entity_models) fn new() -> Self {
        // `head` (offset (0, 20, -62)) parents the `jaw` (offset (0, 4, -8)).
        let head = ModelPart::new(
            DRAGON_HEAD_POSE,
            DRAGON_HEAD_CUBES.to_vec(),
            vec![(
                "0",
                ModelPart::leaf(DRAGON_JAW_POSE, DRAGON_JAW_CUBES.to_vec()),
            )],
        );

        // `body` (offset (0, 3, 8)) parents the two wings and the four legs.
        let body = ModelPart::new(
            DRAGON_BODY_POSE,
            DRAGON_BODY_CUBES.to_vec(),
            vec![
                (
                    "0",
                    dragon_wing(
                        [12.0, 2.0, -6.0],
                        &DRAGON_LEFT_WING_CUBES,
                        [56.0, 0.0, 0.0],
                        &DRAGON_LEFT_WING_TIP_CUBES,
                    ),
                ),
                ("1", dragon_front_leg([12.0, 17.0, -6.0])),
                ("2", dragon_hind_leg([16.0, 13.0, 34.0])),
                (
                    "3",
                    dragon_wing(
                        [-12.0, 2.0, -6.0],
                        &DRAGON_RIGHT_WING_CUBES,
                        [-56.0, 0.0, 0.0],
                        &DRAGON_RIGHT_WING_TIP_CUBES,
                    ),
                ),
                ("4", dragon_front_leg([-12.0, 17.0, -6.0])),
                ("5", dragon_hind_leg([-16.0, 13.0, 34.0])),
            ],
        );

        // The mesh root: head, the five neck segments (`offset(0, 20, -12 - i·10)`), the twelve tail
        // segments (`offset(0, 10, 60 + i·10)`), and the body.
        Self {
            root: ModelPart::new(
                PART_POSE_ZERO,
                Vec::new(),
                vec![
                    ("0", head),
                    ("1", dragon_spine_part([0.0, 20.0, -12.0])),
                    ("2", dragon_spine_part([0.0, 20.0, -22.0])),
                    ("3", dragon_spine_part([0.0, 20.0, -32.0])),
                    ("4", dragon_spine_part([0.0, 20.0, -42.0])),
                    ("5", dragon_spine_part([0.0, 20.0, -52.0])),
                    ("6", dragon_spine_part([0.0, 10.0, 60.0])),
                    ("7", dragon_spine_part([0.0, 10.0, 70.0])),
                    ("8", dragon_spine_part([0.0, 10.0, 80.0])),
                    ("9", dragon_spine_part([0.0, 10.0, 90.0])),
                    ("10", dragon_spine_part([0.0, 10.0, 100.0])),
                    ("11", dragon_spine_part([0.0, 10.0, 110.0])),
                    ("12", dragon_spine_part([0.0, 10.0, 120.0])),
                    ("13", dragon_spine_part([0.0, 10.0, 130.0])),
                    ("14", dragon_spine_part([0.0, 10.0, 140.0])),
                    ("15", dragon_spine_part([0.0, 10.0, 150.0])),
                    ("16", dragon_spine_part([0.0, 10.0, 160.0])),
                    ("17", dragon_spine_part([0.0, 10.0, 170.0])),
                    ("18", body),
                ],
            ),
        }
    }
}

impl EntityModel for EnderDragonModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, _instance: &EntityModelInstance) {}
}
