use super::{PartPose, PART_POSE_ZERO, TURTLE_GREEN, TURTLE_SHELL};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

pub(in crate::entity_models) const MODEL_LAYER_TURTLE: &str = "minecraft:turtle#main";
pub(in crate::entity_models) const MODEL_LAYER_TURTLE_BABY: &str = "minecraft:turtle_baby#main";

// Vanilla 26.1 `AdultTurtleModel.createBodyLayer` (atlas 128×64). The head, body (shell +
// belly), and four legs are direct children of the mesh root; the `egg_belly` overlay shell (one
// extra cube at the body pose) is emitted when the synced `hasEgg` state is set, and vanilla then
// drops the whole model `root.y--` by one unit. The legs are repositioned per frame by
// `QuadrupedModel.setupAnim` + `TurtleModel.setupAnim`, so their poses are built from the offset
// constants and the animation curves below. Each cube carries both render paths' data: the colored
// debug tint and the textured `uv_size` / `texOffs` / `mirror` (no turtle cube is mirrored, and
// `CubeDeformation.NONE` keeps `uv_size == size`).
pub(in crate::entity_models) const TURTLE_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -1.0, -3.0],
    [6.0, 5.0, 6.0],
    TURTLE_GREEN,
    [6.0, 5.0, 6.0],
    [3.0, 0.0],
    false,
)];

// Body: the `texOffs(7, 37)` shell box plus the `texOffs(31, 1)` belly box, both under the
// body's `Rx(π/2)` rotation.
pub(in crate::entity_models) const TURTLE_BODY: [ModelCube; 2] = [
    ModelCube::new(
        [-9.5, 3.0, -10.0],
        [19.0, 20.0, 6.0],
        TURTLE_SHELL,
        [19.0, 20.0, 6.0],
        [7.0, 37.0],
        false,
    ),
    ModelCube::new(
        [-5.5, 3.0, -13.0],
        [11.0, 18.0, 3.0],
        TURTLE_GREEN,
        [11.0, 18.0, 3.0],
        [31.0, 1.0],
        false,
    ),
];

// `egg_belly` (`texOffs(70, 33)`): a thin 9×18×1 overlay shell shown only while `hasEgg`. It
// shares the body's pose ([`TURTLE_BODY_POSE`], offset `[0, 11, -10]`, `Rx(π/2)`).
pub(in crate::entity_models) const TURTLE_EGG_BELLY: [ModelCube; 1] = [ModelCube::new(
    [-4.5, 3.0, -14.0],
    [9.0, 18.0, 1.0],
    TURTLE_SHELL,
    [9.0, 18.0, 1.0],
    [70.0, 33.0],
    false,
)];

/// Vanilla `AdultTurtleModel.setupAnim` `root.y--`: the model-local one-unit drop applied to the
/// whole turtle while the `egg_belly` is shown.
pub(in crate::entity_models) const TURTLE_EGG_ROOT_DROP_POSE: PartPose = PartPose {
    offset: [0.0, -1.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, 0.0],
    [4.0, 1.0, 10.0],
    TURTLE_GREEN,
    [4.0, 1.0, 10.0],
    [1.0, 23.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, 0.0],
    [4.0, 1.0, 10.0],
    TURTLE_GREEN,
    [4.0, 1.0, 10.0],
    [1.0, 12.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-13.0, 0.0, -2.0],
    [13.0, 1.0, 5.0],
    TURTLE_GREEN,
    [13.0, 1.0, 5.0],
    [27.0, 30.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -2.0],
    [13.0, 1.0, 5.0],
    TURTLE_GREEN,
    [13.0, 1.0, 5.0],
    [27.0, 24.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 19.0, -10.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 11.0, -10.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-3.5, 22.0, 11.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [3.5, 22.0, 11.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-5.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [5.0, 21.0, -4.0],
    rotation: [0.0, 0.0, 0.0],
};

// Vanilla 26.1 `BabyTurtleModel.createBodyLayer` (atlas 16×16). Smaller geometry, zero-height
// leg planes, but the same root layout and shared `TurtleModel.setupAnim`. The hind-leg planes use
// the vanilla negative `texOffs(-1, …)` exactly as `BabyTurtleModel` bakes them.
pub(in crate::entity_models) const TURTLE_BABY_BODY: [ModelCube; 1] = [ModelCube::new(
    [-2.0, -1.0, -2.0],
    [4.0, 2.0, 4.0],
    TURTLE_SHELL,
    [4.0, 2.0, 4.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-1.5, -2.0, -3.0],
    [3.0, 3.0, 3.0],
    TURTLE_GREEN,
    [3.0, 3.0, 3.0],
    [0.0, 6.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -0.5],
    [2.0, 0.0, 1.0],
    TURTLE_GREEN,
    [2.0, 0.0, 1.0],
    [-1.0, 0.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -0.5],
    [2.0, 0.0, 1.0],
    TURTLE_GREEN,
    [2.0, 0.0, 1.0],
    [-1.0, 1.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -0.5],
    [2.0, 0.0, 1.0],
    TURTLE_GREEN,
    [2.0, 0.0, 1.0],
    [8.0, 6.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [0.0, 0.0, -0.5],
    [2.0, 0.0, 1.0],
    TURTLE_GREEN,
    [2.0, 0.0, 1.0],
    [8.0, 7.0],
    false,
)];

pub(in crate::entity_models) const TURTLE_BABY_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 22.9, -1.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BABY_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 22.9, 1.0],
    rotation: [0.0, 0.0, 0.0],
};

pub(in crate::entity_models) const TURTLE_BABY_RIGHT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 23.9, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_LEFT_HIND_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 23.9, 2.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_RIGHT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [-2.0, 23.9, -0.5],
    rotation: [0.0, 0.0, 0.0],
};
pub(in crate::entity_models) const TURTLE_BABY_LEFT_FRONT_LEG_POSE: PartPose = PartPose {
    offset: [2.0, 23.9, -0.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `QuadrupedModel.setupAnim` leg swing: `leg.xRot = cos(pos·0.6662 + phase)·1.4·speed`
/// with the diagonal `phase = π` for the left-hind and right-front legs. This is the base pose
/// that `TurtleModel.setupAnim` then augments (land) or partly overrides (water).
pub(in crate::entity_models) fn turtle_quadruped_leg_x_rot(
    pos: f32,
    speed: f32,
    phase_pi: bool,
) -> f32 {
    let phase = if phase_pi { std::f32::consts::PI } else { 0.0 };
    (pos * 0.6662 + phase).cos() * 1.4 * speed
}

/// Vanilla `TurtleModel.setupAnim` land leg yaw swing. The hind legs swing
/// `±cos(pos·5)·3·speed`; the front legs swing `±cos(layEgg·pos·5)·8·speed·layEggAmplitude`,
/// where a turtle that `isLayingEgg` sets `layEgg = 4` (the front legs paddle four times faster)
/// and `layEggAmplitude = 2` (twice as wide) to mime digging the nest, while the hind legs are
/// untouched. Both multipliers are `1` when not laying, recovering the plain walk. The sign is
/// negated for the right legs.
pub(in crate::entity_models) fn turtle_land_leg_y_rot(
    pos: f32,
    speed: f32,
    front: bool,
    right: bool,
    laying: bool,
) -> f32 {
    let sign = if right { -1.0 } else { 1.0 };
    if front {
        let lay_egg = if laying { 4.0 } else { 1.0 };
        let lay_amplitude = if laying { 2.0 } else { 1.0 };
        sign * (lay_egg * pos * 5.0).cos() * 8.0 * speed * lay_amplitude
    } else {
        sign * (pos * 5.0).cos() * 3.0 * speed
    }
}

/// Vanilla `TurtleModel.setupAnim` water paddle swing: `swing = cos(pos·0.6662·0.6)·0.5·speed`.
/// The hind legs use it on `xRot` (overriding the quadruped base), the front legs on `zRot`.
pub(in crate::entity_models) fn turtle_water_swing(pos: f32, speed: f32) -> f32 {
    (pos * 0.6662 * 0.6).cos() * 0.5 * speed
}

/// The full per-leg rotation `[xRot, yRot, zRot]` for one turtle leg, composing the
/// `QuadrupedModel` base swing with the `TurtleModel` land/water branch. `front`/`right`
/// identify the leg; `on_land` selects the branch (`!isInWater && onGround`); `laying` applies
/// the egg-laying front-leg amplitude (land branch only, matching vanilla).
pub(in crate::entity_models) fn turtle_leg_rotation(
    pos: f32,
    speed: f32,
    on_land: bool,
    front: bool,
    right: bool,
    laying: bool,
) -> [f32; 3] {
    let base_x = turtle_quadruped_leg_x_rot(pos, speed, front == right);
    if on_land {
        // Land: the quadruped `xRot` swing remains and the turtle adds the `yRot` walk swing.
        [
            base_x,
            turtle_land_leg_y_rot(pos, speed, front, right, laying),
            0.0,
        ]
    } else {
        // Water: the hind legs' `xRot` is replaced by the paddle swing; the front legs keep the
        // quadruped `xRot` and add the paddle swing on `zRot`.
        let swing = turtle_water_swing(pos, speed);
        if front {
            [base_x, 0.0, if right { -swing } else { swing }]
        } else {
            [if right { swing } else { -swing }, 0.0, 0.0]
        }
    }
}

/// The four turtle leg children, in tree order `[right hind, left hind, right front, left front]`,
/// with each leg's name and its `(front, right)` flags for [`turtle_leg_rotation`].
const TURTLE_LEGS: [(&str, bool, bool); 4] = [
    ("right_hind_leg", false, true),
    ("left_hind_leg", false, false),
    ("right_front_leg", true, true),
    ("left_front_leg", true, false),
];

/// Builds the four turtle leg children (named, in tree order) for either the adult or baby tree.
fn turtle_leg_children(baby: bool) -> Vec<(&'static str, ModelPart)> {
    if baby {
        vec![
            (
                "right_hind_leg",
                ModelPart::leaf(
                    TURTLE_BABY_RIGHT_HIND_LEG_POSE,
                    TURTLE_BABY_RIGHT_HIND_LEG.to_vec(),
                ),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(
                    TURTLE_BABY_LEFT_HIND_LEG_POSE,
                    TURTLE_BABY_LEFT_HIND_LEG.to_vec(),
                ),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(
                    TURTLE_BABY_RIGHT_FRONT_LEG_POSE,
                    TURTLE_BABY_RIGHT_FRONT_LEG.to_vec(),
                ),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(
                    TURTLE_BABY_LEFT_FRONT_LEG_POSE,
                    TURTLE_BABY_LEFT_FRONT_LEG.to_vec(),
                ),
            ),
        ]
    } else {
        vec![
            (
                "right_hind_leg",
                ModelPart::leaf(TURTLE_RIGHT_HIND_LEG_POSE, TURTLE_RIGHT_HIND_LEG.to_vec()),
            ),
            (
                "left_hind_leg",
                ModelPart::leaf(TURTLE_LEFT_HIND_LEG_POSE, TURTLE_LEFT_HIND_LEG.to_vec()),
            ),
            (
                "right_front_leg",
                ModelPart::leaf(TURTLE_RIGHT_FRONT_LEG_POSE, TURTLE_RIGHT_FRONT_LEG.to_vec()),
            ),
            (
                "left_front_leg",
                ModelPart::leaf(TURTLE_LEFT_FRONT_LEG_POSE, TURTLE_LEFT_FRONT_LEG.to_vec()),
            ),
        ]
    }
}

/// Builds the unified turtle root (named children) for either the adult or baby model. The child
/// order is the vanilla emit order — `head`, `body`, the adult `egg_belly` overlay (bound visible,
/// toggled per frame by `hasEgg`), then the four legs — preserved for byte-identical meshes.
fn turtle_root(baby: bool) -> ModelPart {
    let mut children: Vec<(&'static str, ModelPart)> = if baby {
        vec![
            (
                "head",
                ModelPart::leaf(TURTLE_BABY_HEAD_POSE, TURTLE_BABY_HEAD.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(TURTLE_BABY_BODY_POSE, TURTLE_BABY_BODY.to_vec()),
            ),
        ]
    } else {
        vec![
            (
                "head",
                ModelPart::leaf(TURTLE_HEAD_POSE, TURTLE_HEAD.to_vec()),
            ),
            (
                "body",
                ModelPart::leaf(TURTLE_BODY_POSE, TURTLE_BODY.to_vec()),
            ),
            (
                "egg_belly",
                ModelPart::leaf(TURTLE_BODY_POSE, TURTLE_EGG_BELLY.to_vec()),
            ),
        ]
    };
    children.extend(turtle_leg_children(baby));
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Applies the vanilla `QuadrupedModel.setupAnim` head look plus `TurtleModel.setupAnim` leg swing to
/// the unified tree: the head tracks the look, the body holds its fixed shell tilt, the legs swing
/// (the land `yRot` walk swing or the water hind-`xRot`/front-`zRot` paddle), and the adult `egg_belly`
/// overlay is shown only when `hasEgg`. The `root.y--` egg drop lives in the root transform.
fn apply_turtle_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let pos = instance.render_state.walk_animation_pos;
    let speed = instance.render_state.walk_animation_speed;
    let on_land = !instance.render_state.in_water && instance.render_state.on_ground;
    let has_egg = !baby && instance.render_state.turtle_has_egg;
    let laying = instance.render_state.turtle_laying_egg;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();

    root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];

    // The adult tree carries the `egg_belly` (toggled by `hasEgg`); the baby has no such part.
    if !baby {
        root.child_mut("egg_belly").visible = has_egg;
    }
    for &(name, front, right) in TURTLE_LEGS.iter() {
        root.child_mut(name).pose.rotation =
            turtle_leg_rotation(pos, speed, on_land, front, right, laying);
    }
}

/// Mutable turtle model, mirroring vanilla `AdultTurtleModel` / `BabyTurtleModel` (a `QuadrupedModel`).
/// The unified tree is built once with named children (selected by `baby`): the root parents `head`,
/// `body`, the adult `egg_belly` overlay, then the four legs (the emit order, preserved for
/// byte-identical meshes). Each cube carries both the colored tint and the textured UV, so one tree
/// drives both render paths; `setup_anim` runs [`apply_turtle_anim`]. The `root.y--` egg drop and the
/// adult/baby texture live outside the model.
pub(in crate::entity_models) struct TurtleModel {
    root: ModelPart,
    baby: bool,
}

impl TurtleModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        Self {
            root: turtle_root(baby),
            baby,
        }
    }
}

impl EntityModel for TurtleModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_turtle_anim(&mut self.root, self.baby, instance);
    }
}
