use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI};

use crate::entity_models::model_layers::{equine_model, EquineModelClass};

fn posed(
    layer: ModelLayer,
    class: EquineModelClass,
    instance: EntityModelInstance,
) -> crate::entity_models::model_layer::BakedRoot {
    equine_model(layer, class, &instance)
}

fn horse() -> EntityModelInstance {
    EntityModelInstance::horse(1, [0.0, 64.0, 0.0], 0.0, false)
}

fn pose(model: &mut ModelPart, name: &str) -> PartPose {
    model.part_mut(name).unwrap().pose
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

#[test]
fn equine_head_look_clamps_yaw_and_tilts_pitch() {
    let mut look = posed(
        ModelLayer::HORSE,
        EquineModelClass::Adult,
        horse().with(|s| (s.head_yaw, s.head_pitch) = (45.0, -25.0)),
    );
    let head = pose(&mut look, "head_parts");
    assert_close(head.rotation[1], 20.0_f32.to_radians());
    assert_close(head.rotation[0], FRAC_PI_6 + (-25.0_f32).to_radians());

    let mut look = posed(
        ModelLayer::HORSE,
        EquineModelClass::Adult,
        horse().with(|s| (s.head_yaw, s.head_pitch) = (-50.0, 0.0)),
    );
    assert_close(
        pose(&mut look, "head_parts").rotation[1],
        (-20.0_f32).to_radians(),
    );

    // The walk bob adds cos(pos * 0.8) * 0.15 * speed onto the pitch only when speed > 0.2.
    for (pos, speed, bob) in [
        (0.0, 1.0, 0.15),
        (0.0, 0.2, 0.0),
        (2.0, 0.5, (1.6_f32).cos() * 0.075),
    ] {
        let mut model = posed(
            ModelLayer::HORSE,
            EquineModelClass::Adult,
            horse().with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (pos, speed)),
        );
        assert_close(pose(&mut model, "head_parts").rotation[0], FRAC_PI_6 + bob);
    }
}

#[test]
fn baby_donkey_head_ignores_pitch_but_keeps_clamped_yaw() {
    // `BabyDonkeyModel.setupAnim` forces `state.xRot = -30`, so the idle head pitch is π/6 - π/6.
    let instance = EntityModelInstance::donkey(
        1,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (45.0, 35.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut model = posed(
        ModelLayer::DONKEY_BABY,
        EquineModelClass::BabyDonkey,
        instance,
    );
    let head = pose(&mut model, "head_parts");
    assert_close(head.rotation[0], 0.0);
    assert_close(head.rotation[1], 20.0_f32.to_radians());
}

#[test]
fn equine_head_blends_vanilla_eat_stand_and_feed_amounts() {
    let age = 1.2_f32;
    let mut instance = horse()
        .with(|s| (s.head_yaw, s.head_pitch) = (45.0, 10.0))
        .with(|s| s.age_in_ticks = age);
    instance.render_state.kind.equine_mut().eat_animation = 0.5;
    instance.render_state.kind.equine_mut().stand_animation = 0.25;
    instance.render_state.kind.equine_mut().feeding_animation = 0.4;
    let mut model = posed(ModelLayer::HORSE, EquineModelClass::Adult, instance);
    let head = pose(&mut model, "head_parts");

    let head_rot_x = 10.0_f32.to_radians();
    let idle = 0.5;
    let expected_x = 0.25 * (PI / 12.0 + head_rot_x)
        + 0.5 * (2.1816616 + age.sin() * 0.05)
        + idle * (FRAC_PI_6 + head_rot_x + 0.4 * age.sin() * 0.05);
    assert_close(head.rotation[0], expected_x);
    assert_close(head.rotation[1], (0.25 + idle) * 20.0_f32.to_radians());
    assert_close(head.offset[1], 4.0 + 2.5);
    assert_close(head.offset[2], -10.0);
}

#[test]
fn equine_standing_pose_matches_vanilla_body_and_leg_offsets() {
    let mut instance = horse();
    instance.render_state.kind.equine_mut().stand_animation = 1.0;
    let mut model = posed(ModelLayer::HORSE, EquineModelClass::Adult, instance);
    assert_close(pose(&mut model, "body").rotation[0], -FRAC_PI_4);
    let left_front = pose(&mut model, "left_front_leg");
    let right_front = pose(&mut model, "right_front_leg");
    assert_eq!(left_front.offset[1..], [2.0, -6.0]);
    assert_eq!(right_front.offset[1..], [2.0, -6.0]);
    assert_close(left_front.rotation[0], -FRAC_PI_3 - 1.0);
    assert_close(right_front.rotation[0], -FRAC_PI_3 + 1.0);
    assert_close(pose(&mut model, "left_hind_leg").rotation[0], PI / 12.0);

    // `BabyHorseModel` rears with its own 4/0 standing leg offsets.
    let mut baby = EntityModelInstance::horse(2, [0.0, 64.0, 0.0], 0.0, true);
    baby.render_state.kind.equine_mut().stand_animation = 1.0;
    let mut model = posed(ModelLayer::HORSE_BABY, EquineModelClass::BabyHorse, baby);
    assert_eq!(pose(&mut model, "left_front_leg").offset[1..], [12.0, -5.4]);
    assert_close(pose(&mut model, "head_parts").offset[1], 10.0 - 2.0);
}

#[test]
fn equine_tail_matches_vanilla_formula() {
    let mut rest = posed(ModelLayer::HORSE, EquineModelClass::Adult, horse());
    let tail = pose(&mut rest, "tail");
    assert_eq!(tail.offset, [0.0, -5.0, 2.0]);
    assert_close(tail.rotation[0], FRAC_PI_6);

    let mut walking = posed(
        ModelLayer::HORSE,
        EquineModelClass::Adult,
        horse().with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0)),
    );
    let tail = pose(&mut walking, "tail");
    assert_close(tail.rotation[0], FRAC_PI_6 + 0.75);
    assert_eq!(tail.offset, [0.0, -4.0, 4.0]);

    // `getTailXRotOffset` overrides the baby layers' baked tail angle even at rest.
    let baby = EntityModelInstance::horse(2, [0.0, 64.0, 0.0], 0.0, true)
        .with(|s| s.age_scale = 0.5)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut model = posed(ModelLayer::HORSE_BABY, EquineModelClass::BabyHorse, baby);
    let tail = pose(&mut model, "tail");
    assert_close(tail.rotation[0], -FRAC_PI_2 + FRAC_PI_6 + 0.75);
    assert_eq!(tail.offset, [0.0, -0.5, 8.0]);
    let baby_donkey = EntityModelInstance::donkey(
        3,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        true,
        false,
    );
    let mut model = posed(
        ModelLayer::MULE_BABY,
        EquineModelClass::BabyDonkey,
        baby_donkey,
    );
    assert_close(pose(&mut model, "tail").rotation[0], -FRAC_PI_4 + FRAC_PI_6);

    let age = 2.5_f32;
    let mut wag = horse().with(|s| s.age_in_ticks = age);
    wag.render_state.kind.equine_mut().animate_tail = true;
    let mut model = posed(ModelLayer::HORSE, EquineModelClass::Adult, wag);
    assert_close(pose(&mut model, "tail").rotation[1], (age * 0.7).cos());
}

#[test]
fn equine_legs_follow_vanilla_gait_and_slow_in_water() {
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    for in_water in [false, true] {
        let multiplier = if in_water { 0.2 } else { 1.0 };
        let leg_anim = (multiplier * pos * 0.6662 + PI).cos() * speed;
        let mut model = posed(
            ModelLayer::HORSE,
            EquineModelClass::Adult,
            horse()
                .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (pos, speed))
                .with(|s| s.in_water = in_water),
        );
        assert_close(
            pose(&mut model, "left_hind_leg").rotation[0],
            -0.5 * leg_anim,
        );
        assert_close(
            pose(&mut model, "right_hind_leg").rotation[0],
            0.5 * leg_anim,
        );
        assert_close(
            pose(&mut model, "left_front_leg").rotation[0],
            0.8 * leg_anim,
        );
        assert_close(
            pose(&mut model, "right_front_leg").rotation[0],
            -0.8 * leg_anim,
        );
        assert_eq!(
            pose(&mut model, "left_front_leg").offset,
            [4.0, 14.0, -10.0]
        );
    }
}

#[test]
fn donkey_chests_and_saddle_lines_follow_render_state() {
    for has_chest in [false, true] {
        let instance = EntityModelInstance::donkey(
            1,
            [0.0, 64.0, 0.0],
            0.0,
            DonkeyModelFamily::Donkey,
            false,
            has_chest,
        );
        let mut model = posed(ModelLayer::DONKEY, EquineModelClass::Adult, instance);
        let root = &mut model;
        assert_eq!(root.part_mut("left_chest").unwrap().visible, has_chest);
        assert_eq!(root.part_mut("right_chest").unwrap().visible, has_chest);
    }
    for ridden in [false, true] {
        let mut instance = horse();
        instance.render_state.kind.equine_mut().saddle_ridden = ridden;
        let mut model = posed(ModelLayer::HORSE_SADDLE, EquineModelClass::Saddle, instance);
        let root = &mut model;
        assert_eq!(root.part_mut("left_saddle_line").unwrap().visible, ridden);
        assert_eq!(root.part_mut("right_saddle_line").unwrap().visible, ridden);
    }
}

#[test]
fn rebuilt_model_starts_from_the_bind_pose() {
    let mut instance = horse();
    instance.render_state.kind.equine_mut().stand_animation = 1.0;
    let mut rearing = posed(ModelLayer::HORSE, EquineModelClass::Adult, instance);
    rearing.part_mut("tail").unwrap().visible = false;
    drop(rearing);
    // The next model reuses the dropped tree, which must not keep the rearing pose.
    let mut rebuilt = ModelLayer::HORSE.bake();
    assert_eq!(pose(&mut rebuilt, "body").rotation, [0.0; 3]);
    assert!(rebuilt.part_mut("tail").unwrap().visible);
}
