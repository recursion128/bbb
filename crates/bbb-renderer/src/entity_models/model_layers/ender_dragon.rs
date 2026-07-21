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
// y-offset (captured by `ender_dragon_model_root_transform`). The dying dissolve and nearest-crystal
// healing beam are deferred; the base texture path and emissive eyes pass are wired. The colored
// debug path renders the body dark and the wing membranes a lighter tint. Each unified cube carries
// both the colored debug tint (`DRAGON_BODY` / `DRAGON_MEMBRANE`) and the textured `uv_size` /
// `texOffs`.

pub(in crate::entity_models) const MODEL_LAYER_ENDER_DRAGON: &str = "minecraft:ender_dragon#main";

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

    // Vanilla `EnderDragonModel.setupAnim`, formula for formula: the jaw opens and the root
    // bounces with the wing-beat phase, every neck/tail segment is re-placed along the
    // `DragonFlightHistory` path (rotating each segment and chaining the next pivot 10 units
    // down the rotated spine), the wings flap, and the legs sway with the bounce. Sources
    // without the projected animation state (older fixtures) keep the bind layout.
    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        let Some(dragon) = instance.render_state.ender_dragon_animation else {
            return;
        };
        const DEG: f32 = std::f32::consts::PI / 180.0;
        let flap_time = dragon.flap_time * std::f32::consts::TAU;
        let mut bounce = (flap_time - 1.0).sin() + 1.0;
        bounce = (bounce * bounce + bounce * 2.0) * 0.05;
        self.root.pose.offset[1] = (bounce - 2.0) * 16.0;
        self.root.pose.offset[2] = -48.0;
        self.root.pose.rotation[0] = bounce * 2.0 * DEG;
        self.root.child_mut("0").child_mut("0").pose.rotation[0] = (flap_time.sin() + 1.0) * 0.2;

        let start = dragon.historical_pos(6);
        let rot2 = wrap_degrees(dragon.historical_pos(5).y_rot - dragon.historical_pos(10).y_rot);
        let rot = wrap_degrees(dragon.historical_pos(5).y_rot + rot2 / 2.0);

        let [mut xx, mut yy, mut zz] = self.root.child_mut("1").pose.offset;
        for (i, name) in DRAGON_NECK_CHILD_NAMES.iter().enumerate() {
            let point = dragon.historical_pos(5 - i);
            let wave = (i as f32 * 0.45 + flap_time).cos() * 0.15;
            let neck = self.root.child_mut(name);
            neck.pose.rotation[1] = wrap_degrees(point.y_rot - start.y_rot) * DEG * 1.5;
            neck.pose.rotation[0] =
                wave + dragon.head_part_y_offset(i as i32, start, point) * DEG * 1.5 * 5.0;
            neck.pose.rotation[2] = -wrap_degrees(point.y_rot - rot) * DEG * 1.5;
            neck.pose.offset = [xx, yy, zz];
            xx -= neck.pose.rotation[1].sin() * neck.pose.rotation[0].cos() * 10.0;
            yy += neck.pose.rotation[0].sin() * 10.0;
            zz -= neck.pose.rotation[1].cos() * neck.pose.rotation[0].cos() * 10.0;
        }

        let current = dragon.historical_pos(0);
        let head = self.root.child_mut("0");
        head.pose.offset = [xx, yy, zz];
        head.pose.rotation[1] = wrap_degrees(current.y_rot - start.y_rot) * DEG;
        head.pose.rotation[0] =
            wrap_degrees(dragon.head_part_y_offset(6, start, current)) * DEG * 1.5 * 5.0;
        head.pose.rotation[2] = -wrap_degrees(current.y_rot - rot) * DEG;

        let body = self.root.child_mut("18");
        body.pose.rotation[2] = -rot2 * 1.5 * DEG;
        let wing_x_rot = 0.125 - flap_time.cos() * 0.2;
        let wing_y_rot = -0.25;
        let wing_z_rot = -(flap_time.sin() + 0.125) * 0.8;
        let wing_tip_z_rot = ((flap_time + 2.0).sin() + 0.5) * 0.75;
        let left_wing = body.child_mut("0");
        left_wing.pose.rotation = [wing_x_rot, wing_y_rot, wing_z_rot];
        left_wing.child_mut("0").pose.rotation[2] = wing_tip_z_rot;
        let right_wing = body.child_mut("3");
        right_wing.pose.rotation = [wing_x_rot, -wing_y_rot, -wing_z_rot];
        right_wing.child_mut("0").pose.rotation[2] = -wing_tip_z_rot;
        for (front_name, hind_name) in [("1", "2"), ("4", "5")] {
            let front_leg = body.child_mut(front_name);
            front_leg.pose.rotation[0] = 1.3 + bounce * 0.1;
            let front_tip = front_leg.child_mut("0");
            front_tip.pose.rotation[0] = -0.5 - bounce * 0.1;
            front_tip.child_mut("0").pose.rotation[0] = 0.75 + bounce * 0.1;
            let hind_leg = body.child_mut(hind_name);
            hind_leg.pose.rotation[0] = 1.0 + bounce * 0.1;
            let hind_tip = hind_leg.child_mut("0");
            hind_tip.pose.rotation[0] = 0.5 + bounce * 0.1;
            hind_tip.child_mut("0").pose.rotation[0] = 0.75 + bounce * 0.1;
        }

        let mut tail_x_rot = 0.0_f32;
        let tail_start = dragon.historical_pos(11);
        let [mut xx, mut yy, mut zz] = self.root.child_mut("6").pose.offset;
        for (i, name) in DRAGON_TAIL_CHILD_NAMES.iter().enumerate() {
            let point = dragon.historical_pos(12 + i);
            tail_x_rot += (i as f32 * 0.45 + flap_time).sin() * 0.05;
            let tail = self.root.child_mut(name);
            tail.pose.rotation[1] =
                (wrap_degrees(point.y_rot - tail_start.y_rot) * 1.5 + 180.0) * DEG;
            tail.pose.rotation[0] = tail_x_rot + (point.y - tail_start.y) as f32 * DEG * 1.5 * 5.0;
            tail.pose.rotation[2] = wrap_degrees(point.y_rot - rot) * DEG * 1.5;
            tail.pose.offset = [xx, yy, zz];
            yy += tail.pose.rotation[0].sin() * 10.0;
            zz -= tail.pose.rotation[1].cos() * tail.pose.rotation[0].cos() * 10.0;
            xx -= tail.pose.rotation[1].sin() * tail.pose.rotation[0].cos() * 10.0;
        }
    }
}

/// Root child names of the five neck segments (`neck0..neck4`) in vanilla order.
const DRAGON_NECK_CHILD_NAMES: [&str; 5] = ["1", "2", "3", "4", "5"];
/// Root child names of the twelve tail segments (`tail0..tail11`) in vanilla order.
const DRAGON_TAIL_CHILD_NAMES: [&str; 12] = [
    "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
];

/// Vanilla `Mth.wrapDegrees`.
fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
