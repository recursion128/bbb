use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    VANILLA_ENTITY_TYPE_CAMEL_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CACTUS_ID, VANILLA_ITEM_RABBIT_FOOT_ID, VANILLA_ITEM_SPIDER_EYE_ID,
    VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::packets::ItemStackSummary;

const ALICE: ConnectionId = ConnectionId(1);

#[test]
fn camel_food_look_turns_while_sitting_or_carrying_a_passenger_but_respects_controllers() {
    use crate::entity::tracking::ServerEntity;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

    for (kind, food, sitting, rider, controlled) in [
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ITEM_CACTUS_ID,
            true,
            "",
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            VANILLA_ITEM_RABBIT_FOOT_ID,
            false,
            "",
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ITEM_CACTUS_ID,
            false,
            "player",
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ITEM_CACTUS_ID,
            false,
            "saddled",
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            VANILLA_ITEM_RABBIT_FOOT_ID,
            false,
            "mob",
            true,
        ),
    ] {
        let (sender, receiver, mut world) = fixture();
        let start = Vec3d {
            x: 5.5,
            y: 65.0,
            z: 8.5,
        };
        world.set_player_position_for_test(ALICE, start);
        join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
        world.set_player_position_for_test(ConnectionId(2), Vec3d { x: 8.5, ..start });
        world.set_inventory_slot_for_test(
            ConnectionId(2),
            0,
            ItemStackSummary {
                item_id: Some(food),
                count: 1,
                component_patch: Default::default(),
            },
        );
        let (id, uuid) = spawn(&mut world, kind, start);
        let mut commands = Vec::new();
        if sitting {
            commands.push(format!("data merge entity {uuid} {{LastPoseTick:-100L}}"));
        }
        if rider == "saddled" {
            commands.push(format!(
                "item replace entity {uuid} saddle with minecraft:saddle"
            ));
        }
        if rider == "mob" {
            let (_, passenger) = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, start);
            commands.push(format!("data merge entity {passenger} {{NoAI:1b}}"));
            commands.push(format!("ride {passenger} mount {uuid}"));
        } else if !rider.is_empty() {
            commands.push(format!("ride alice mount {uuid}"));
        }
        for (index, command) in commands.iter().enumerate() {
            let tick = console_command(&sender, &receiver, &mut world, 30 + index as u64, command);
            assert!(console_command_answer(&tick).0.is_some(), "{command}");
        }
        let initial = world.stored_entity_for_test(DIMENSION, id).unwrap();
        let mut tracker = ServerEntity::new(initial, 1, true);
        let initial_head = initial.y_head_rot();
        for _ in 0..30 {
            world.tick(receiver.drain());
        }
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        let head = body.y_head_rot();
        if controlled {
            assert_eq!(head, initial_head, "{kind} {rider}");
        } else {
            assert!(head < -30.0, "{kind} {rider}: {head}");
            let mut body = body.clone();
            assert!(tracker.send_changes(&mut body).iter().any(|packet| matches!(
                packet,
                ClientboundPlayPacket::RotateHead(rotation) if rotation.id == id && rotation.y_head_rot == head
            )));
        }
    }
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    for (id, command) in ["time set midnight", "gamerule spawn_mobs false"]
        .iter()
        .enumerate()
    {
        let tick = console_command(&sender, &receiver, &mut world, 10 + id as u64, command);
        assert!(console_command_answer(&tick).0.is_some());
    }
    (sender, receiver, world)
}

fn hold(world: &mut ServerWorld, slot: usize, item_id: Option<i32>) {
    world.set_inventory_slot_for_test(
        ALICE,
        slot,
        ItemStackSummary {
            item_id,
            count: i32::from(item_id.is_some()),
            component_patch: Default::default(),
        },
    );
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

#[test]
fn herd_food_in_either_hand_attracts_adults_and_babies_to_their_own_stop_distance() {
    for (kind, food, multiplier, adult_distance, baby_distance) in [
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            VANILLA_ITEM_CACTUS_ID,
            2.5,
            3.5,
            2.5,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            VANILLA_ITEM_RABBIT_FOOT_ID,
            2.5,
            3.5,
            2.5,
        ),
        (
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            VANILLA_ITEM_SPIDER_EYE_ID,
            1.25,
            2.0,
            1.0,
        ),
    ] {
        for baby in [false, true] {
            if baby && kind == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                continue;
            }
            let (sender, receiver, mut world) = fixture();
            let player = Vec3d {
                x: 12.5,
                y: 65.0,
                z: 8.5,
            };
            world.set_player_position_for_test(ALICE, player);
            let slot = if baby || kind == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                40
            } else {
                0
            };
            hold(&mut world, slot, Some(food));
            let mut data = GeneralLivingData::new(kind).unwrap();
            if baby {
                data.ageable_retained_mut().unwrap().set_age(-24000);
            }
            let (id, uuid) = world.spawn_stored_entity_for_test(
                DIMENSION,
                kind,
                Vec3d { x: 4.5, ..player },
                EntityData::GeneralLiving(data),
            );
            if kind == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                let tick = console_command(
                    &sender,
                    &receiver,
                    &mut world,
                    30,
                    &format!("data merge entity {uuid} {{Age:-24000}}"),
                );
                assert!(console_command_answer(&tick).0.is_some());
            }
            let base_speed = world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                as f32;
            let mut tempted_speed = false;
            for _ in 0..160 {
                world.tick(receiver.drain());
                let speed = world
                    .stored_entity_for_test(DIMENSION, id)
                    .unwrap()
                    .living()
                    .unwrap()
                    .speed();
                tempted_speed |= (speed - multiplier * base_speed).abs() < 1.0e-5;
            }
            let close = position(&world, id);
            let stop = if baby { baby_distance } else { adult_distance };
            let distance = separation(close, player);
            assert!(
                tempted_speed,
                "type {kind}, baby={baby} uses food-following speed"
            );
            let stop_range = if kind == VANILLA_ENTITY_TYPE_ARMADILLO_ID {
                // Navigation can finish in the adjacent block before reaching
                // the Euclidean stop radius (`MoveToTargetSink.reachedTarget`).
                if baby {
                    stop - 0.55..1.5
                } else {
                    1.5..stop + 0.1
                }
            } else {
                stop - 0.55..stop + 0.1
            };
            assert!(
                stop_range.contains(&distance),
                "type {kind}, baby={baby}: distance={distance}, expected {stop}, mob={close:?}"
            );
            for _ in 0..25 {
                world.tick(receiver.drain());
            }
            assert!(separation(close, position(&world, id)) < 0.2);

            // Ending temptation still permits normal idle movement (ArmadilloAi.java:87-104).
            let idle_speed = if kind == VANILLA_ENTITY_TYPE_ARMADILLO_ID {
                1.0
            } else {
                2.0
            };
            for (held, ticks) in [(Some(VANILLA_ITEM_STONE_ID), 110), (None, 20)] {
                hold(&mut world, slot, held);
                let body = position(&world, id);
                world.set_player_position_for_test(
                    ALICE,
                    Vec3d {
                        x: body.x - 8.0,
                        ..body
                    },
                );
                let mut previous = body;
                for tick in 0..ticks {
                    world.tick(receiver.drain());
                    let current = position(&world, id);
                    let moved = separation(previous, current) > 0.01;
                    previous = current;
                    if tick < 10 || !moved {
                        continue;
                    }
                    let speed = world
                        .stored_entity_for_test(DIMENSION, id)
                        .unwrap()
                        .living()
                        .unwrap()
                        .speed();
                    assert!(
                        speed <= idle_speed * base_speed + 1.0e-5,
                        "type {kind}, baby={baby} stops following at temptation speed: {speed}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_camel_follows_another_food_holder_instead_of_its_own_player_passenger() {
    let (sender, receiver, mut world) = fixture();
    let start = Vec3d {
        x: 5.5,
        y: 65.0,
        z: 8.5,
    };
    world.set_player_position_for_test(ALICE, start);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.set_player_position_for_test(ConnectionId(2), Vec3d { x: 12.5, ..start });
    world.set_inventory_slot_for_test(
        ConnectionId(2),
        0,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_CACTUS_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_CAMEL_ID, start);
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        30,
        &format!("ride alice mount {uuid}"),
    );
    assert!(console_command_answer(&tick).0.is_some());
    hold(&mut world, 0, Some(VANILLA_ITEM_CACTUS_ID));
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    assert!(
        position(&world, id).x > start.x + 2.0,
        "the camel must ignore its nearest food holder riding it and approach bob: {:?}",
        position(&world, id)
    );
}
