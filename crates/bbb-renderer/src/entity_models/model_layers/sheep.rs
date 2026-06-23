use super::{apply_quadruped_leg_swing_named, PartPose, PART_POSE_ZERO, SHEEP_WOOL};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::{EntityModel, ModelCube, ModelPart};

// Vanilla 26.1 SheepModel.createBodyLayer(). Each cube carries both render paths' data: the colored
// debug tint and the textured `uv_size` / `texOffs` / `mirror`.
pub(in crate::entity_models) const ADULT_SHEEP_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -4.0, -6.0],
    [6.0, 6.0, 8.0],
    SHEEP_WOOL,
    [6.0, 6.0, 8.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_SHEEP_BODY: [ModelCube; 1] = [ModelCube::new(
    [-4.0, -10.0, -7.0],
    [8.0, 16.0, 6.0],
    SHEEP_WOOL,
    [8.0, 16.0, 6.0],
    [28.0, 8.0],
    false,
)];

pub(in crate::entity_models) const ADULT_SHEEP_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.0, 0.0, -2.0],
    [4.0, 12.0, 4.0],
    SHEEP_WOOL,
    [4.0, 12.0, 4.0],
    [0.0, 16.0],
    false,
)];

// Vanilla 26.1 SheepFurModel.createFurLayer(): the wool cubes carry a `CubeDeformation`, so the
// colored geometry is inflated (head 7.2, body 11.5×19.5×9.5, leg 5×7×5) while the textured `uv_size`
// keeps the base box (the squid body precedent).
pub(in crate::entity_models) const ADULT_SHEEP_WOOL_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-3.6, -4.6, -4.6],
    [7.2, 7.2, 7.2],
    SHEEP_WOOL,
    [6.0, 6.0, 6.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_BODY: [ModelCube; 1] = [ModelCube::new(
    [-5.75, -11.75, -8.75],
    [11.5, 19.5, 9.5],
    SHEEP_WOOL,
    [8.0, 16.0, 6.0],
    [28.0, 8.0],
    false,
)];

pub(in crate::entity_models) const ADULT_SHEEP_WOOL_LEG: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -0.5, -2.5],
    [5.0, 7.0, 5.0],
    SHEEP_WOOL,
    [4.0, 6.0, 4.0],
    [0.0, 16.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_HEAD: [ModelCube; 1] = [ModelCube::new(
    [-2.5, -4.5, -3.5],
    [5.0, 5.0, 5.0],
    SHEEP_WOOL,
    [5.0, 5.0, 5.0],
    [0.0, 0.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_BODY: [ModelCube; 1] = [ModelCube::new(
    [-3.0, -2.0, -4.5],
    [6.0, 4.0, 9.0],
    SHEEP_WOOL,
    [6.0, 4.0, 9.0],
    [0.0, 10.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_RIGHT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 5.0, 2.0],
    SHEEP_WOOL,
    [2.0, 5.0, 2.0],
    [0.0, 23.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_LEFT_HIND_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 5.0, 2.0],
    SHEEP_WOOL,
    [2.0, 5.0, 2.0],
    [24.0, 12.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_RIGHT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 5.0, 2.0],
    SHEEP_WOOL,
    [2.0, 5.0, 2.0],
    [8.0, 23.0],
    false,
)];

pub(in crate::entity_models) const BABY_SHEEP_LEFT_FRONT_LEG: [ModelCube; 1] = [ModelCube::new(
    [-1.0, 0.0, -1.0],
    [2.0, 5.0, 2.0],
    SHEEP_WOOL,
    [2.0, 5.0, 2.0],
    [24.0, 5.0],
    false,
)];

/// The adult sheep head/body part poses (shared by the body and wool layers).
const ADULT_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 6.0, -8.0],
    rotation: [0.0, 0.0, 0.0],
};

const ADULT_BODY_POSE: PartPose = PartPose {
    offset: [0.0, 5.0, 2.0],
    rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
};

/// The baby sheep head part pose.
const BABY_HEAD_POSE: PartPose = PartPose {
    offset: [0.0, 15.5, -2.5],
    rotation: [0.0, 0.0, 0.0],
};

/// Vanilla `SheepRenderState.headEatPositionScale` / `headEatAngleScale`, the
/// per-frame eat-grass head animation projected from `Sheep.eatAnimationTick`.
/// `SheepModel`/`SheepFurModel.setupAnim` consume these to lower and tilt the
/// head part of the base, wool, and undercoat passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheepHeadEatPose {
    pub position_scale: f32,
    pub angle_scale: f32,
}

impl SheepHeadEatPose {
    /// Resting head pose used when the sheep is not eating grass.
    pub const NONE: Self = Self {
        position_scale: 0.0,
        angle_scale: 0.0,
    };

    /// Vanilla `Sheep.getHeadEatPositionScale`/`getHeadEatAngleScale` projected
    /// from the canonical `eatAnimationTick` and the renderer partial tick.
    pub fn from_eat_tick(eat_animation_tick: i32, partial_tick: f32) -> Self {
        Self {
            position_scale: sheep_head_eat_position_scale(eat_animation_tick, partial_tick),
            angle_scale: sheep_head_eat_angle_scale(eat_animation_tick, partial_tick),
        }
    }

    pub(in crate::entity_models) fn is_resting(self) -> bool {
        self == Self::NONE
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale` plateau angle: `(float)(Math.PI / 5)`.
const SHEEP_HEAD_EAT_PLATEAU_ANGLE: f32 = std::f32::consts::PI / 5.0;

/// Vanilla `Sheep.getHeadEatPositionScale(partialTick)`.
fn sheep_head_eat_position_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick <= 0 {
        0.0
    } else if (4..=36).contains(&eat_animation_tick) {
        1.0
    } else if eat_animation_tick < 4 {
        (eat_animation_tick as f32 - partial_tick) / 4.0
    } else {
        -(eat_animation_tick as f32 - 40.0 - partial_tick) / 4.0
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale(partialTick)`, restricted to the eating
/// branches (`eatAnimationTick > 0`). The non-eating branch of the vanilla
/// method folds in the entity look pitch (`getXRot(a) * PI/180`); that pitch is
/// projected separately as [`EntityRenderState::head_pitch`] and applied by
/// [`sheep_head_pose`], so a resting (non-eating) sheep returns `0.0` here and
/// the head pitch comes from the look projection instead.
fn sheep_head_eat_angle_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick > 4 && eat_animation_tick <= 36 {
        let scale = (eat_animation_tick as f32 - 4.0 - partial_tick) / 32.0;
        SHEEP_HEAD_EAT_PLATEAU_ANGLE + 0.21991149 * (scale * 28.7).sin()
    } else if eat_animation_tick > 0 {
        SHEEP_HEAD_EAT_PLATEAU_ANGLE
    } else {
        0.0
    }
}

/// Returns `true` when the sheep head is fully at rest — not eating and with no
/// head-look turn — so callers can borrow the static parts unchanged instead of
/// cloning to apply [`sheep_head_pose`].
pub(in crate::entity_models) fn sheep_head_at_rest(
    head_eat: SheepHeadEatPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> bool {
    head_eat.is_resting() && head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// Vanilla sheep head pose, composing `QuadrupedModel.setupAnim` (head look) with
/// the `SheepModel`/`SheepFurModel.setupAnim` overrides:
///
/// - `QuadrupedModel.setupAnim`: `head.xRot = xRot * π/180`, `head.yRot = yRot *
///   π/180` (`yRot` is the net head yaw `wrapDegrees(headRot - bodyRot)`).
/// - `SheepModel.setupAnim` (after super): `head.y += headEatPositionScale * 9.0
///   * ageScale` and `head.xRot = headEatAngleScale`, which *overrides* the pitch
///   set by the super call. Vanilla `Sheep.getHeadEatAngleScale` returns the look
///   pitch (`getXRot * π/180`) while not eating, so the head pitch is the look
///   pitch at rest and the eat curve while eating.
///
/// `BabySheepModel extends SheepModel`, so the baby head animates with `ageScale
/// = 0.5` (`LivingEntity.getAgeScale`). The base head pose has no rotation, so
/// the yaw/pitch are set (not accumulated), matching the vanilla `setupAnim`
/// assignments.
pub(in crate::entity_models) fn sheep_head_pose(
    head_pose: PartPose,
    baby: bool,
    head_eat: SheepHeadEatPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> PartPose {
    let age_scale = if baby { 0.5 } else { 1.0 };
    let x_rot = if head_eat.is_resting() {
        head_pitch_deg.to_radians()
    } else {
        head_eat.angle_scale
    };
    PartPose {
        offset: [
            head_pose.offset[0],
            head_pose.offset[1] + head_eat.position_scale * 9.0 * age_scale,
            head_pose.offset[2],
        ],
        rotation: [x_rot, head_yaw_deg.to_radians(), head_pose.rotation[2]],
    }
}

pub(in crate::entity_models) const MODEL_LAYER_SHEEP: &str = "minecraft:sheep#main";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_BABY: &str = "minecraft:sheep_baby#main";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_WOOL: &str = "minecraft:sheep#wool";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_BABY_WOOL: &str = "minecraft:sheep_baby#wool";
pub(in crate::entity_models) const MODEL_LAYER_SHEEP_WOOL_UNDERCOAT: &str =
    "minecraft:sheep#wool_undercoat";

/// Builds a leaf part at `offset` (no rotation) carrying `cubes`.
fn leg(offset: [f32; 3], cubes: &[ModelCube]) -> ModelPart {
    ModelPart::leaf(
        PartPose {
            offset,
            rotation: [0.0, 0.0, 0.0],
        },
        cubes.to_vec(),
    )
}

/// Builds the four adult-sheep legs (hind-first, shared by the body and wool layers) under the
/// vanilla `QuadrupedModel` child names, carrying `cubes`.
fn adult_legs(cubes: &'static [ModelCube]) -> Vec<(&'static str, ModelPart)> {
    vec![
        ("right_hind_leg", leg([-3.0, 12.0, 7.0], cubes)),
        ("left_hind_leg", leg([3.0, 12.0, 7.0], cubes)),
        ("right_front_leg", leg([-3.0, 12.0, -5.0], cubes)),
        ("left_front_leg", leg([3.0, 12.0, -5.0], cubes)),
    ]
}

/// Builds the unified adult sheep body tree (head, body, legs) under the vanilla child names.
fn adult_sheep_body_tree() -> ModelPart {
    let mut children = vec![
        (
            "head",
            ModelPart::leaf(ADULT_HEAD_POSE, ADULT_SHEEP_HEAD.to_vec()),
        ),
        (
            "body",
            ModelPart::leaf(ADULT_BODY_POSE, ADULT_SHEEP_BODY.to_vec()),
        ),
    ];
    children.extend(adult_legs(&ADULT_SHEEP_LEG));
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the unified adult sheep wool (fur) tree (head, body, legs) under the vanilla child names.
fn adult_sheep_wool_tree() -> ModelPart {
    let mut children = vec![
        (
            "head",
            ModelPart::leaf(ADULT_HEAD_POSE, ADULT_SHEEP_WOOL_HEAD.to_vec()),
        ),
        (
            "body",
            ModelPart::leaf(ADULT_BODY_POSE, ADULT_SHEEP_WOOL_BODY.to_vec()),
        ),
    ];
    children.extend(adult_legs(&ADULT_SHEEP_WOOL_LEG));
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Builds the unified baby sheep tree (body first, then head, then legs — vanilla declaration order),
/// shared by the baby body and fur layers (vanilla baby sheep has no separate fur layer).
fn baby_sheep_tree() -> ModelPart {
    let children = vec![
        ("body", leg([0.0, 17.0, 0.5], &BABY_SHEEP_BODY)),
        (
            "head",
            ModelPart::leaf(BABY_HEAD_POSE, BABY_SHEEP_HEAD.to_vec()),
        ),
        (
            "right_hind_leg",
            leg([-2.0, 19.0, 3.0], &BABY_SHEEP_RIGHT_HIND_LEG),
        ),
        (
            "left_hind_leg",
            leg([2.0, 19.0, 3.0], &BABY_SHEEP_LEFT_HIND_LEG),
        ),
        (
            "right_front_leg",
            leg([-2.0, 19.0, -2.0], &BABY_SHEEP_RIGHT_FRONT_LEG),
        ),
        (
            "left_front_leg",
            leg([2.0, 19.0, -2.0], &BABY_SHEEP_LEFT_FRONT_LEG),
        ),
    ];
    ModelPart::new(PART_POSE_ZERO, Vec::new(), children)
}

/// Vanilla `SheepModel.setupAnim`: `super.setupAnim` (the `QuadrupedModel` leg swing) then the
/// eat-grass head pose ([`sheep_head_pose`], folded with the head look). Shared by the body and fur
/// layers — both move together. The head pose is skipped while fully at rest ([`sheep_head_at_rest`]).
fn apply_sheep_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    if !sheep_head_at_rest(
        render_state.head_eat,
        render_state.head_yaw,
        render_state.head_pitch,
    ) {
        let head = root.child_mut("head");
        head.pose = sheep_head_pose(
            head.pose,
            baby,
            render_state.head_eat,
            render_state.head_yaw,
            render_state.head_pitch,
        );
    }
    apply_quadruped_leg_swing_named(
        root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
}

/// Mutable sheep body model, mirroring vanilla `SheepModel` (a `QuadrupedModel`). The unified tree is
/// built for `baby` with the vanilla child names; `setup_anim` runs the shared [`apply_sheep_anim`].
/// The base layer renders this with its texture (or baked colors); the dyed-undercoat layer renders
/// the same tree recolored with the wool tint.
pub(in crate::entity_models) struct SheepModel {
    root: ModelPart,
    baby: bool,
}

impl SheepModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let root = if baby {
            baby_sheep_tree()
        } else {
            adult_sheep_body_tree()
        };
        Self { root, baby }
    }
}

impl EntityModel for SheepModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_sheep_anim(&mut self.root, self.baby, instance);
    }
}

/// Mutable sheep fur model, mirroring vanilla `SheepFurModel`. The unified tree is the fluffy wool
/// layer for the adult; the baby reuses its body geometry (vanilla baby sheep has no separate fur
/// layer). `setup_anim` runs the same shared [`apply_sheep_anim`] so the wool moves with the body.
/// Rendered with the wool tint (colored) or the wool texture (textured); skipped when sheared or
/// invisible.
pub(in crate::entity_models) struct SheepFurModel {
    root: ModelPart,
    baby: bool,
}

impl SheepFurModel {
    pub(in crate::entity_models) fn new(baby: bool) -> Self {
        let root = if baby {
            baby_sheep_tree()
        } else {
            adult_sheep_wool_tree()
        };
        Self { root, baby }
    }
}

impl EntityModel for SheepFurModel {
    fn root(&self) -> &ModelPart {
        &self.root
    }

    fn root_mut(&mut self) -> &mut ModelPart {
        &mut self.root
    }

    fn setup_anim(&mut self, instance: &EntityModelInstance) {
        apply_sheep_anim(&mut self.root, self.baby, instance);
    }
}
