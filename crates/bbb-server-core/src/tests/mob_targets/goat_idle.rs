//! Goat idle attention follows a real player and releases land navigation.

use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;

const ALICE: ConnectionId = ConnectionId(1);

fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn looks_at(body: &crate::entity::Entity, player: Vec3d) -> bool {
    let position = body.position();
    let wanted = (player.z - position.z)
        .atan2(player.x - position.x)
        .to_degrees() as f32
        - 90.0;
    let error = (body.y_head_rot() - wanted + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (position.y + f64::from(body.eye_height()) - (player.y + 1.62))
        .atan2(separation(position, player))
        .to_degrees() as f32;
    error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
}

fn base_speed(body: &crate::entity::Entity) -> bool {
    let living = body.living().unwrap();
    (f64::from(living.speed()) - living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)).abs()
        < 1.0e-5
}

#[test]
fn an_idle_goat_looks_and_walks_toward_a_non_food_player_then_resumes_wandering() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let mut player = Vec3d {
        x: 10.5,
        y: 65.0,
        z: 8.5,
    };
    world.set_player_position_for_test(ALICE, player);
    let initial = Vec3d {
        x: 5.5,
        y: 65.0,
        z: 8.5,
    };
    let (goat, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_GOAT_ID, initial);
    for text in [
        "gamerule spawn_mobs false".to_owned(),
        format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:10000}},\"minecraft:long_jump_cooling_down\":{{value:10000}}}}}}}}"),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, 1, &text);
        assert!(console_command_answer(&tick).0.is_some(), "{text}");
    }
    world.tick_rate_mut().set_frozen(false);
    let mut walking_at_base_speed = false;
    let mut looking_ticks = 0;
    tick_until(&mut world, 350, |world| {
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        let moved = body.position().x > initial.x + 1.0;
        walking_at_base_speed |= moved && base_speed(body);
        looking_ticks = if looks_at(body, player) {
            looking_ticks + 1
        } else {
            0
        };
        walking_at_base_speed
            && looking_ticks >= 4
            && body.on_ground()
            && (2.5..=3.7).contains(&separation(body.position(), player))
    });
    let first = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .position();
    assert!(first.x > initial.x + 1.0, "the goat approaches: {first:?}");
    assert!((first.y - 65.0).abs() < 0.001);

    player = Vec3d {
        x: first.x,
        z: first.z - 5.0,
        ..player
    };
    world.set_player_position_for_test(ALICE, player);
    walking_at_base_speed = false;
    looking_ticks = 0;
    tick_until(&mut world, 350, |world| {
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        let moved = body.position().z < first.z - 1.0;
        walking_at_base_speed |= moved && base_speed(body);
        looking_ticks = if looks_at(body, player) {
            looking_ticks + 1
        } else {
            0
        };
        walking_at_base_speed
            && looking_ticks >= 4
            && body.on_ground()
            && (2.5..=3.7).contains(&separation(body.position(), player))
    });
    let second = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .position();
    assert!(
        second.z < first.z - 1.0,
        "the goat turns its head and approaches the moved player: {second:?}"
    );
    assert!((second.y - 65.0).abs() < 0.001);

    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 0.5,
            y: 85.0,
            z: 0.5,
        },
    );
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    let after_player_left = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .position();
    tick_until(&mut world, 700, |world| {
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        body.on_ground() && base_speed(body) && separation(body.position(), after_player_left) > 2.0
    });
    let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
    assert!((body.position().y - 65.0).abs() < 0.001);
}
