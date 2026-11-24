use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `EnderDragonModel`: `setup_anim` replays the projected flight history and flap state.
pub(in crate::entity_models) fn ender_dragon_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::ENDER_DRAGON.bake();
    let Some(dragon) = instance.render_state.kind.ender_dragon().animation else {
        return root;
    };
    const DEG: f32 = std::f32::consts::PI / 180.0;
    let flap_time = dragon.flap_time * std::f32::consts::TAU;
    let mut bounce = (flap_time - 1.0).sin() + 1.0;
    bounce = (bounce * bounce + bounce * 2.0) * 0.05;
    root.pose.offset[1] = (bounce - 2.0) * 16.0;
    root.pose.offset[2] = -48.0;
    root.pose.rotation[0] = bounce * 2.0 * DEG;
    root.child_mut("head").child_mut("jaw").pose.rotation[0] = (flap_time.sin() + 1.0) * 0.2;

    let start = dragon.historical_pos(6);
    let rot2 = wrap_degrees(dragon.historical_pos(5).y_rot - dragon.historical_pos(10).y_rot);
    let rot = wrap_degrees(dragon.historical_pos(5).y_rot + rot2 / 2.0);

    let [mut xx, mut yy, mut zz] = root.child_mut("neck0").pose.offset;
    for (i, name) in DRAGON_NECK_CHILD_NAMES.iter().enumerate() {
        let point = dragon.historical_pos(5 - i);
        let wave = (i as f32 * 0.45 + flap_time).cos() * 0.15;
        let neck = root.child_mut(name);
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
    let head = root.child_mut("head");
    head.pose.offset = [xx, yy, zz];
    head.pose.rotation[1] = wrap_degrees(current.y_rot - start.y_rot) * DEG;
    head.pose.rotation[0] =
        wrap_degrees(dragon.head_part_y_offset(6, start, current)) * DEG * 1.5 * 5.0;
    head.pose.rotation[2] = -wrap_degrees(current.y_rot - rot) * DEG;

    let body = root.child_mut("body");
    body.pose.rotation[2] = -rot2 * 1.5 * DEG;
    let wing_x_rot = 0.125 - flap_time.cos() * 0.2;
    let wing_y_rot = -0.25;
    let wing_z_rot = -(flap_time.sin() + 0.125) * 0.8;
    let wing_tip_z_rot = ((flap_time + 2.0).sin() + 0.5) * 0.75;
    let left_wing = body.child_mut("left_wing");
    left_wing.pose.rotation = [wing_x_rot, wing_y_rot, wing_z_rot];
    left_wing.child_mut("left_wing_tip").pose.rotation[2] = wing_tip_z_rot;
    let right_wing = body.child_mut("right_wing");
    right_wing.pose.rotation = [wing_x_rot, -wing_y_rot, -wing_z_rot];
    right_wing.child_mut("right_wing_tip").pose.rotation[2] = -wing_tip_z_rot;
    for [leg, tip, foot, rot] in [
        [
            "left_front_leg",
            "left_front_leg_tip",
            "left_front_foot",
            "front",
        ],
        [
            "left_hind_leg",
            "left_hind_leg_tip",
            "left_hind_foot",
            "hind",
        ],
        [
            "right_front_leg",
            "right_front_leg_tip",
            "right_front_foot",
            "front",
        ],
        [
            "right_hind_leg",
            "right_hind_leg_tip",
            "right_hind_foot",
            "hind",
        ],
    ] {
        let (leg_x, tip_x) = if rot == "front" {
            (1.3 + bounce * 0.1, -0.5 - bounce * 0.1)
        } else {
            (1.0 + bounce * 0.1, 0.5 + bounce * 0.1)
        };
        let leg = body.child_mut(leg);
        leg.pose.rotation[0] = leg_x;
        let tip = leg.child_mut(tip);
        tip.pose.rotation[0] = tip_x;
        tip.child_mut(foot).pose.rotation[0] = 0.75 + bounce * 0.1;
    }

    let mut tail_x_rot = 0.0_f32;
    let tail_start = dragon.historical_pos(11);
    let [mut xx, mut yy, mut zz] = root.child_mut("tail0").pose.offset;
    for (i, name) in DRAGON_TAIL_CHILD_NAMES.iter().enumerate() {
        let point = dragon.historical_pos(12 + i);
        tail_x_rot += (i as f32 * 0.45 + flap_time).sin() * 0.05;
        let tail = root.child_mut(name);
        tail.pose.rotation[1] = (wrap_degrees(point.y_rot - tail_start.y_rot) * 1.5 + 180.0) * DEG;
        tail.pose.rotation[0] = tail_x_rot + (point.y - tail_start.y) as f32 * DEG * 1.5 * 5.0;
        tail.pose.rotation[2] = wrap_degrees(point.y_rot - rot) * DEG * 1.5;
        tail.pose.offset = [xx, yy, zz];
        yy += tail.pose.rotation[0].sin() * 10.0;
        zz -= tail.pose.rotation[1].cos() * tail.pose.rotation[0].cos() * 10.0;
        xx -= tail.pose.rotation[1].sin() * tail.pose.rotation[0].cos() * 10.0;
    }
    root
}

const DRAGON_NECK_CHILD_NAMES: [&str; 5] = ["neck0", "neck1", "neck2", "neck3", "neck4"];
const DRAGON_TAIL_CHILD_NAMES: [&str; 12] = [
    "tail0", "tail1", "tail2", "tail3", "tail4", "tail5", "tail6", "tail7", "tail8", "tail9",
    "tail10", "tail11",
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
