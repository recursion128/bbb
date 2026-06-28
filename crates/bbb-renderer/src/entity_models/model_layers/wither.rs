use super::{
    head_look_at_rest, head_look_pose, PartPose, PART_POSE_ZERO, WITHER_BODY, WITHER_HEAD,
};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_WITHER: &str = "minecraft:wither#main";
pub(in crate::entity_models) const MODEL_LAYER_WITHER_ARMOR: &str = "minecraft:wither#armor";

// Vanilla 26.1 `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` (atlas 64×64). The mesh root
// holds six sibling parts: the shoulders bar, the ribcage (its spine plus three rib bars), the
// hanging tail, the center head, and the two side heads. The ribcage and tail carry their baked
// rest rotation; the tail's bind position is `(-2, 6.9 + cos(0.20420352) * 10, -0.5 +
// sin(0.20420352) * 10)`, derived from the ribcage's bind pitch (the `anim = 0` rest of the
// breathing sway below). The center head follows the plain head look
// (`centerHead.yRot/xRot = state.yRot/xRot`), reproduced via `head_look_pose`; the ribcage and tail
// breathe with `cos(ageInTicks * 0.1)` via [`wither_breathing_poses`]. The two side heads track the
// client-tick-lerped `WitherRenderState.xHeadRots/yHeadRots` arrays. The `wither.png`
// texture is wired here, and the wither swaps to `wither_invulnerable.png` during its spawn charge
// (`WitherBossRenderer.getTextureLocation`, see [`super::super::wither_model_root_transform`] and
// `wither_textured_layer_passes`); the `WITHER_ARMOR` powered energy-swirl overlay (`wither_armor.png`,
// the same `EnergySwirlLayer` as the charged creeper) is the inflated [`WitherModel::new_armor`] tree,
// emitted by the dispatch-owned energy-swirl helper. Each cube carries the colored debug tint and the textured
// `uv_size` / `texOffs`.

// `shoulders`: the 20×3×3 bar, texOffs(0,16).
pub(in crate::entity_models) const WITHER_SHOULDERS_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-10.0, 3.9, -0.5],
    [20.0, 3.0, 3.0],
    WITHER_BODY,
    [20.0, 3.0, 3.0],
    [0.0, 16.0],
    false,
)];

// `ribcage`: the 3×10×3 spine (texOffs(0,22)) plus three 11×2×2 rib bars (`texOffs(24,22)`, stacked).
pub(in crate::entity_models) const WITHER_RIBCAGE_CUBES: [ModelCube; 4] = [
    ModelCube::new(
        [0.0, 0.0, 0.0],
        [3.0, 10.0, 3.0],
        WITHER_BODY,
        [3.0, 10.0, 3.0],
        [0.0, 22.0],
        false,
    ),
    ModelCube::new(
        [-4.0, 1.5, 0.5],
        [11.0, 2.0, 2.0],
        WITHER_BODY,
        [11.0, 2.0, 2.0],
        [24.0, 22.0],
        false,
    ),
    ModelCube::new(
        [-4.0, 4.0, 0.5],
        [11.0, 2.0, 2.0],
        WITHER_BODY,
        [11.0, 2.0, 2.0],
        [24.0, 22.0],
        false,
    ),
    ModelCube::new(
        [-4.0, 6.5, 0.5],
        [11.0, 2.0, 2.0],
        WITHER_BODY,
        [11.0, 2.0, 2.0],
        [24.0, 22.0],
        false,
    ),
];

// `tail`: the 3×6×3 hanging spine segment, texOffs(12,22).
pub(in crate::entity_models) const WITHER_TAIL_CUBES: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, 0.0],
    [3.0, 6.0, 3.0],
    WITHER_BODY,
    [3.0, 6.0, 3.0],
    [12.0, 22.0],
    false,
)];

// `center_head`: the 8×8×8 skull, texOffs(0,0).
pub(in crate::entity_models) const WITHER_CENTER_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -4.0, -4.0],
    [8.0, 8.0, 8.0],
    WITHER_HEAD,
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    false,
)];

// The shared 6×6×6 side head (both side heads reuse it, differing only in pivot), texOffs(32,0).
pub(in crate::entity_models) const WITHER_SIDE_HEAD_CUBES: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -4.0, -4.0],
    [6.0, 6.0, 6.0],
    WITHER_HEAD,
    [6.0, 6.0, 6.0],
    [32.0, 0.0],
    false,
)];

/// Vanilla `LayerDefinitions` bakes `ModelLayers.WITHER_ARMOR` with
/// `WitherBossModel.createBodyLayer(INNER_ARMOR_DEFORMATION)` = `new CubeDeformation(0.5F)`: the same
/// six-part tree one notch larger, so the `WitherArmorLayer` energy swirl floats just outside the body.
/// `CubeDeformation` grows each box (`min -= g`, `size += 2·g`) while keeping the base `texOffs`/`uv_size`.
const WITHER_INNER_ARMOR_DEFORMATION: f32 = 0.5;

const fn inflate_wither_cube(cube: ModelCube) -> ModelCube {
    let g = WITHER_INNER_ARMOR_DEFORMATION;
    ModelCube::new(
        [cube.min[0] - g, cube.min[1] - g, cube.min[2] - g],
        [
            cube.size[0] + 2.0 * g,
            cube.size[1] + 2.0 * g,
            cube.size[2] + 2.0 * g,
        ],
        cube.color,
        cube.uv_size,
        cube.tex,
        cube.mirror,
    )
}

const WITHER_ARMOR_SHOULDERS_CUBES: [ModelCube; 1] =
    [inflate_wither_cube(WITHER_SHOULDERS_CUBES[0])];
const WITHER_ARMOR_RIBCAGE_CUBES: [ModelCube; 4] = [
    inflate_wither_cube(WITHER_RIBCAGE_CUBES[0]),
    inflate_wither_cube(WITHER_RIBCAGE_CUBES[1]),
    inflate_wither_cube(WITHER_RIBCAGE_CUBES[2]),
    inflate_wither_cube(WITHER_RIBCAGE_CUBES[3]),
];
const WITHER_ARMOR_TAIL_CUBES: [ModelCube; 1] = [inflate_wither_cube(WITHER_TAIL_CUBES[0])];
const WITHER_ARMOR_CENTER_HEAD_CUBES: [ModelCube; 1] =
    [inflate_wither_cube(WITHER_CENTER_HEAD_CUBES[0])];
const WITHER_ARMOR_SIDE_HEAD_CUBES: [ModelCube; 1] =
    [inflate_wither_cube(WITHER_SIDE_HEAD_CUBES[0])];

/// Vanilla `createBodyLayer` rest poses (`addOrReplaceChild` order: shoulders, ribcage, tail,
/// center_head, right_head, left_head).
pub(in crate::entity_models) const WITHER_SHOULDERS_POSE: PartPose = PART_POSE_ZERO;
/// The `ribcage` bind pose (offset `(-2, 6.9, -0.5)`, pitched `0.20420352`); it breathes via
/// [`wither_breathing_poses`].
pub(in crate::entity_models) const WITHER_RIBCAGE_POSE: PartPose = PartPose {
    offset: [-2.0, 6.9, -0.5],
    rotation: [0.20420352, 0.0, 0.0],
};
/// The `tail` bind pose; its hang position and pitch breathe with the ribcage via
/// [`wither_breathing_poses`].
pub(in crate::entity_models) const WITHER_TAIL_POSE: PartPose = PartPose {
    offset: [-2.0, 16.692228, 1.5278729],
    rotation: [0.83252203, 0.0, 0.0],
};
/// The `center_head` bind pose; it tracks the plain head look.
pub(in crate::entity_models) const WITHER_CENTER_HEAD_POSE: PartPose = PART_POSE_ZERO;
/// The `right_head` bind pose; `setup_anim` applies vanilla side-head target tracking over it.
pub(in crate::entity_models) const WITHER_RIGHT_HEAD_POSE: PartPose = PartPose {
    offset: [-8.0, 4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
/// The `left_head` bind pose; `setup_anim` applies vanilla side-head target tracking over it.
pub(in crate::entity_models) const WITHER_LEFT_HEAD_POSE: PartPose = PartPose {
    offset: [10.0, 4.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `WitherBossModel.setupAnim` breathing sway, driven entirely by the projected
/// `ageInTicks`: `anim = cos(ageInTicks * 0.1)` pitches the ribcage to
/// `(0.065 + 0.05 * anim) * PI`, re-hangs the tail from that new pitch
/// (`tail.setPos(-2, 6.9 + cos(ribcage.xRot) * 10, -0.5 + sin(ribcage.xRot) * 10)`), and pitches the
/// tail to `(0.265 + 0.1 * anim) * PI`. At `anim = 0` it collapses to the baked
/// [`WITHER_RIBCAGE_POSE`] / [`WITHER_TAIL_POSE`] rest poses, so the sway oscillates symmetrically
/// about the layer pose. Returns the
/// `(ribcage, tail)` poses; the ribcage keeps its bind offset `(-2, 6.9, -0.5)` and only its `xRot`
/// moves. Because `ageInTicks` advances every frame, the wither never sits perfectly still.
pub(in crate::entity_models) fn wither_breathing_poses(age_in_ticks: f32) -> (PartPose, PartPose) {
    use std::f32::consts::PI;
    let anim = (age_in_ticks * 0.1).cos();
    let ribcage_x_rot = (0.065 + 0.05 * anim) * PI;
    let ribcage = PartPose {
        offset: [-2.0, 6.9, -0.5],
        rotation: [ribcage_x_rot, 0.0, 0.0],
    };
    let tail = PartPose {
        offset: [
            -2.0,
            6.9 + ribcage_x_rot.cos() * 10.0,
            -0.5 + ribcage_x_rot.sin() * 10.0,
        ],
        rotation: [(0.265 + 0.1 * anim) * PI, 0.0, 0.0],
    };
    (ribcage, tail)
}

/// Mutable wither model, mirroring vanilla `WitherBossModel`. Its six sibling parts hang off a
/// synthetic root (vanilla `WitherBossModel`'s `root`); each is built from the baked per-part pose
/// and `ModelCube` geometry. `setup_anim` mutates the named parts exactly as
/// `WitherBossModel.setupAnim` does, and the trait renders the tree in one pass.
pub(in crate::entity_models) struct WitherModel {
    root: ModelPart,
}

impl WitherModel {
    pub(in crate::entity_models) fn new() -> Self {
        Self::with_cubes(
            &WITHER_SHOULDERS_CUBES,
            &WITHER_RIBCAGE_CUBES,
            &WITHER_TAIL_CUBES,
            &WITHER_CENTER_HEAD_CUBES,
            &WITHER_SIDE_HEAD_CUBES,
        )
    }

    /// The inflated `WITHER_ARMOR` tree (vanilla `INNER_ARMOR_DEFORMATION` = `CubeDeformation(0.5)`),
    /// driven by the same `setup_anim` so the `WitherArmorLayer` energy swirl tracks the body pose.
    pub(in crate::entity_models) fn new_armor() -> Self {
        Self::with_cubes(
            &WITHER_ARMOR_SHOULDERS_CUBES,
            &WITHER_ARMOR_RIBCAGE_CUBES,
            &WITHER_ARMOR_TAIL_CUBES,
            &WITHER_ARMOR_CENTER_HEAD_CUBES,
            &WITHER_ARMOR_SIDE_HEAD_CUBES,
        )
    }

    fn with_cubes(
        shoulders: &[ModelCube],
        ribcage: &[ModelCube],
        tail: &[ModelCube],
        center_head: &[ModelCube],
        side_head: &[ModelCube],
    ) -> Self {
        let root = ModelPart::new(
            PART_POSE_ZERO,
            Vec::new(),
            vec![
                (
                    "shoulders",
                    ModelPart::leaf(WITHER_SHOULDERS_POSE, shoulders.to_vec()),
                ),
                (
                    "ribcage",
                    ModelPart::leaf(WITHER_RIBCAGE_POSE, ribcage.to_vec()),
                ),
                ("tail", ModelPart::leaf(WITHER_TAIL_POSE, tail.to_vec())),
                (
                    "center_head",
                    ModelPart::leaf(WITHER_CENTER_HEAD_POSE, center_head.to_vec()),
                ),
                (
                    "right_head",
                    ModelPart::leaf(WITHER_RIGHT_HEAD_POSE, side_head.to_vec()),
                ),
                (
                    "left_head",
                    ModelPart::leaf(WITHER_LEFT_HEAD_POSE, side_head.to_vec()),
                ),
            ],
        );
        Self { root }
    }
}

impl EntityModel for WitherModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        // Vanilla `WitherBossModel.setupAnim`: side heads track the copied wither head rotation
        // arrays, the ribcage and tail breathe with `ageInTicks` ([`wither_breathing_poses`]), then
        // the center head tracks the ordinary look angles.
        let render_state = instance.render_state;
        let body_rot = render_state.body_rot;
        for (name, index) in [("right_head", 0_usize), ("left_head", 1_usize)] {
            let head = self.root.child_mut(name);
            head.pose.rotation[0] = render_state.wither_x_head_rots[index].to_radians();
            head.pose.rotation[1] =
                (render_state.wither_y_head_rots[index] - body_rot).to_radians();
        }

        let (ribcage_pose, tail_pose) = wither_breathing_poses(instance.render_state.age_in_ticks);
        self.root.child_mut("ribcage").pose = ribcage_pose;
        self.root.child_mut("tail").pose = tail_pose;

        let head_yaw = render_state.head_yaw;
        let head_pitch = render_state.head_pitch;
        if !head_look_at_rest(head_yaw, head_pitch) {
            let center_head = self.root.child_mut("center_head");
            center_head.pose = head_look_pose(center_head.pose, head_yaw, head_pitch);
        }
    }
}
