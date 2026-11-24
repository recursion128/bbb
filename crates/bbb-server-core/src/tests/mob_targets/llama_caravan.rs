//! A naturally joined llama caravan follows a genuinely player-leashed leader.

use super::melee::{fixture, ALICE, PLAYER_POSITION};
use super::*;
use crate::entity::ai::mob::target::TargetRef;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID;
use bbb_server_types::geometry::Vec3Ops;

fn keep_alive(sender: &WorldCommandSender, output: &TickResult) {
    for event in &output.events {
        if let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::KeepAlive { id },
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
}
fn tick(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    let output = world.tick(receiver.drain());
    keep_alive(sender, &output);
}
fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let operation = world.game_time() as u64 + 10000;
    let output = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&output);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    keep_alive(sender, &output);
}
fn interact(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: i32,
) -> TickResult {
    let output = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    keep_alive(sender, &output);
    output
}
fn link(output: &TickResult, source: i32, destination: i32) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEntityLink(link), .. }
        if link.source_id == source && link.dest_id == destination))
}
fn caravan(world: &ServerWorld, id: i32) -> (Option<TargetRef>, Option<TargetRef>) {
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    let EntityData::GeneralLiving(data) = body.data() else {
        panic!("a real llama body");
    };
    let state = data.llama_caravan().unwrap();
    (state.head, state.tail)
}

#[test]
fn an_unleashed_llama_naturally_follows_and_leaves_a_player_led_caravan() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "forceload add -16 -16 32 32",
        "fill -16 64 -16 32 64 32 minecraft:stone",
        "time set noon",
        "weather clear",
        "item replace entity alice weapon.mainhand with minecraft:lead 2",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let leader_position = Vec3d {
        x: 10.5,
        y: 65.0,
        z: 8.5,
    };
    let (leader, leader_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_LLAMA_ID, leader_position);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {leader_uuid} {{NoAI:1b}}"),
    );
    world.set_player_position_for_test(ALICE, PLAYER_POSITION);
    tick(&sender, &receiver, &mut world);
    assert_eq!(world.publication().players[0].position, PLAYER_POSITION);
    let attached = interact(&sender, &receiver, &mut world, leader);
    let holder = world.player_entity_id_for_test(ALICE);
    assert!(link(&attached, leader, holder));
    assert_eq!(
        crate::entity::leash::leash_holder(
            world.stored_entity_for_test(DIMENSION, leader).unwrap()
        ),
        Some(holder)
    );
    let (follower, follower_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        Vec3d {
            x: 17.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let leader_ref = TargetRef {
        entity_id: leader,
        uuid: leader_uuid,
    };
    let follower_ref = TargetRef {
        entity_id: follower,
        uuid: follower_uuid,
    };
    let initial = world
        .stored_entity_for_test(DIMENSION, follower)
        .unwrap()
        .position()
        .distance_to_sqr(leader_position)
        .sqrt();
    let leader_health = world
        .stored_entity_for_test(DIMENSION, leader)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let follower_health = world
        .stored_entity_for_test(DIMENSION, follower)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut joined = false;
    let mut speed_seen = false;
    let mut approached = false;
    let mut best_distance = initial;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..450 {
        tick(&sender, &receiver, &mut world);
        let root = world.stored_entity_for_test(DIMENSION, leader).unwrap();
        assert_eq!(root.position(), leader_position);
        assert_eq!(root.living().unwrap().health(), leader_health);
        assert_eq!(crate::entity::leash::leash_holder(root), Some(holder));
        let body = world.stored_entity_for_test(DIMENSION, follower).unwrap();
        assert_eq!(body.living().unwrap().health(), follower_health);
        assert_eq!(crate::entity::leash::leash_holder(body), None);
        let distance = body.position().distance_to_sqr(root.position()).sqrt();
        best_distance = best_distance.min(distance);
        joined |= caravan(&world, follower).0 == Some(leader_ref)
            && caravan(&world, leader).1 == Some(follower_ref);
        let living = body.living().unwrap();
        speed_seen |= body.on_ground()
            && !body.is_in_water()
            && !body.is_in_lava()
            && (f64::from(living.speed())
                - 2.1 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
            .abs()
                < 1.0e-5;
        approached |= body.on_ground()
            && !body.is_in_water()
            && !body.is_in_lava()
            && initial - distance > 2.0
            && (1.5..=3.5).contains(&distance);
        if joined && speed_seen && approached {
            break;
        }
    }
    assert!(joined && speed_seen && approached,
        "natural caravan relation and grounded following: joined={joined}, speed={speed_seen}, approached={approached}, initial={initial}, best={best_distance}");
    assert_eq!(caravan(&world, follower).0, Some(leader_ref));
    assert_eq!(caravan(&world, leader).1, Some(follower_ref));
    let released = interact(&sender, &receiver, &mut world, leader);
    assert!(link(&released, leader, 0));
    assert_eq!(
        crate::entity::leash::leash_holder(
            world.stored_entity_for_test(DIMENSION, leader).unwrap()
        ),
        None
    );
    let mut left = false;
    for _ in 0..100 {
        tick(&sender, &receiver, &mut world);
        let root = world.stored_entity_for_test(DIMENSION, leader).unwrap();
        let body = world.stored_entity_for_test(DIMENSION, follower).unwrap();
        assert_eq!(root.position(), leader_position);
        assert_eq!(root.living().unwrap().health(), leader_health);
        assert_eq!(body.living().unwrap().health(), follower_health);
        assert_eq!(crate::entity::leash::leash_holder(body), None);
        if caravan(&world, follower).0.is_none() && caravan(&world, leader).1.is_none() {
            left = true;
            break;
        }
    }
    assert!(
        left,
        "releasing the actual root leash dissolves the natural caravan links"
    );
}
