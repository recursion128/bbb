use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

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

/// Applies the vanilla `QuadrupedModel.setupAnim` head look plus `TurtleModel.setupAnim` leg swing to
/// the unified tree: the head tracks the look, the body holds its fixed shell tilt, the legs swing
/// (the land `yRot` walk swing or the water hind-`xRot`/front-`zRot` paddle), and the adult `egg_belly`
/// overlay is shown only when `hasEgg`. The `root.y--` egg drop lives in the root transform.
fn apply_turtle_anim(root: &mut ModelPart, baby: bool, instance: &EntityModelInstance) {
    let turtle_state = *instance.render_state.kind.turtle();
    let pos = instance.render_state.walk_animation_pos;
    let speed = instance.render_state.walk_animation_speed;
    let on_land = !instance.render_state.in_water && instance.render_state.on_ground;
    let has_egg = !baby && turtle_state.has_egg;
    let laying = turtle_state.laying_egg;
    let head_pitch = instance.render_state.head_pitch.to_radians();
    let head_yaw = instance.render_state.head_yaw.to_radians();

    root.child_mut("head").pose.rotation = [head_pitch, head_yaw, 0.0];

    // The adult tree carries the `egg_belly` (toggled by `hasEgg`); the baby has no such part.
    if !baby {
        root.child_mut("egg_belly").visible = has_egg;
        if has_egg {
            root.pose.offset[1] -= 1.0;
        }
    }
    for &(name, front, right) in TURTLE_LEGS.iter() {
        root.child_mut(name).pose.rotation =
            turtle_leg_rotation(pos, speed, on_land, front, right, laying);
    }
}

/// Vanilla `TurtleModel`.
pub(in crate::entity_models) fn turtle_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::TURTLE_BABY
    } else {
        ModelLayer::TURTLE
    }
    .bake();
    apply_turtle_anim(&mut root, baby, instance);
    root
}
