//! Camel family navigation, idle attention and swimming on live world ticks.

use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID};
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;

const ALICE: ConnectionId = ConnectionId(1);

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let tick = console_command(sender, receiver, world, 1, text);
    assert!(console_command_answer(&tick).0.is_some(), "{text}");
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    command(&sender, &receiver, &mut world, "time set midnight");
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 85.0,
            z: 8.5,
        },
    );
    (sender, receiver, world)
}

#[track_caller]
fn until(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    limit: usize,
    mut done: impl FnMut(&ServerWorld) -> bool,
) {
    for _ in 0..limit {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::KeepAlive { id },
                ..
            } = event
            {
                enqueue(
                    sender,
                    WorldCommand::Play {
                        connection_id: ALICE,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        if done(world) {
            return;
        }
    }
    panic!("the observable behavior did not occur within {limit} ticks");
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn speed_matches(body: &crate::entity::Entity, modifier: f64) -> bool {
    let living = body.living().unwrap();
    (f64::from(living.speed())
        - modifier * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
    .abs()
        < 1.0e-5
}

fn looks_at_player(body: &crate::entity::Entity, player: Vec3d) -> bool {
    let position = body.position();
    let yaw = (player.z - position.z)
        .atan2(player.x - position.x)
        .to_degrees() as f32
        - 90.0;
    let yaw_error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (position.y + f64::from(body.eye_height()) - (player.y + 1.62))
        .atan2(separation(position, player))
        .to_degrees() as f32;
    yaw_error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
}

#[test]
fn a_baby_camel_follows_an_adult_at_its_family_speed() {
    let (sender, receiver, mut world) = fixture();
    let (adult, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 11.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    let (baby, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{Age:-24000}}"),
    );
    let initial = position(&world, baby);
    let mut followed_at_speed = false;
    world.tick_rate_mut().set_frozen(false);
    until(&sender, &receiver, &mut world, 250, |world| {
        let body = world.stored_entity_for_test(DIMENSION, baby).unwrap();
        followed_at_speed |= speed_matches(body, 2.5) && body.position().x > initial.x + 0.5;
        followed_at_speed
            && body.on_ground()
            && separation(body.position(), position(world, adult)) <= 4.6
    });
    let close = position(&world, baby);
    assert!(
        followed_at_speed && close.x > initial.x + 3.0,
        "the baby approaches its adult: {close:?}"
    );
    assert!((3.0..=4.6).contains(&separation(close, position(&world, adult))));
    assert!((close.y - 65.0).abs() < 0.001);
    assert_eq!(position(&world, adult).x, 11.5);
}

#[test]
fn camel_family_looks_and_walks_toward_a_non_food_player_then_wanders() {
    for kind in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let (sender, receiver, mut world) = fixture();
        world.set_inventory_slot_for_test(
            ALICE,
            0,
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: Default::default(),
            },
        );
        let player = Vec3d {
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
        let (camel, _) = spawn(&mut world, kind, initial);
        let mut walking_at_speed = false;
        let mut looking_ticks = 0;
        world.tick_rate_mut().set_frozen(false);
        until(&sender, &receiver, &mut world, 700, |world| {
            let body = world.stored_entity_for_test(DIMENSION, camel).unwrap();
            walking_at_speed |= speed_matches(body, 2.0) && body.position().x > initial.x + 1.0;
            looking_ticks = if looks_at_player(body, player) {
                looking_ticks + 1
            } else {
                0
            };
            walking_at_speed
                && looking_ticks >= 4
                && body.on_ground()
                && (2.5..=3.7).contains(&separation(body.position(), player))
        });
        let close = position(&world, camel);
        assert!(
            close.x > initial.x + 1.0 && (close.y - 65.0).abs() < 0.001,
            "kind {kind}: {close:?}"
        );
        world.set_player_position_for_test(
            ALICE,
            Vec3d {
                x: 8.5,
                y: 85.0,
                z: 8.5,
            },
        );
        for _ in 0..40 {
            world.tick(receiver.drain());
        }
        let after_player_left = position(&world, camel);
        until(&sender, &receiver, &mut world, 1000, |world| {
            let body = world.stored_entity_for_test(DIMENSION, camel).unwrap();
            body.on_ground()
                && speed_matches(body, 2.0)
                && separation(body.position(), after_player_left) > 2.0
        });
        let body = world.stored_entity_for_test(DIMENSION, camel).unwrap();
        assert!(!body.is_in_water() && !body.is_in_lava());
        assert!((body.position().y - 65.0).abs() < 0.001);
    }
}

#[test]
fn camel_family_swims_up_from_a_deep_pool_while_no_ai_camels_stay_at_the_bottom() {
    for kind in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let (sender, receiver, mut world) = fixture();
        command(
            &sender,
            &receiver,
            &mut world,
            "fill 0 65 0 15 69 15 minecraft:water",
        );
        let (swimmer, _) = spawn(
            &mut world,
            kind,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 8.5,
            },
        );
        let (still, uuid) = spawn(
            &mut world,
            kind,
            Vec3d {
                x: 11.5,
                y: 65.0,
                z: 8.5,
            },
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:1b}}"),
        );
        world.tick_rate_mut().set_frozen(false);
        until(&sender, &receiver, &mut world, 140, |world| {
            let body = world.stored_entity_for_test(DIMENSION, swimmer).unwrap();
            body.is_in_water() && body.position().y > 67.0
        });
        assert!(
            position(&world, still).y < 65.5,
            "kind {kind}: AI swimming supplies the upward motion"
        );
    }
}
