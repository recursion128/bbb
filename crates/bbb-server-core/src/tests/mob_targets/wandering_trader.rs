//! Public saved travel targets yield to a real merchant session and accept a new route.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID;
use bbb_protocol::packets::{ContainerCloseRequest, InteractEntity};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let output = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&output);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    keep_alive(sender, &output);
    output
}
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
fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let output = world.tick(receiver.drain());
    keep_alive(sender, &output);
    output
}
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}
fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}
fn has_saved_target(world: &ServerWorld, id: i32) -> bool {
    let record = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap();
    field(&record, "wander_target").is_some()
}
fn progress(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    trader: i32,
    destination: Vec3d,
) {
    let initial = separation(position(world, trader), destination);
    let mut best = initial;
    for _ in 0..600 {
        tick(sender, receiver, world);
        let body = world.stored_entity_for_test(DIMENSION, trader).unwrap();
        let distance = separation(body.position(), destination);
        best = best.min(distance);
        let living = body.living().unwrap();
        let speed_matches = (f64::from(living.speed())
            - 0.35 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
            < 1.0e-5;
        if speed_matches && body.on_ground() && distance < initial - 3.0 {
            return;
        }
    }
    panic!(
        "actual .35 travel did not approach saved target: initial={initial}, best={best}, at={:?}",
        position(world, trader)
    );
}

#[test]
fn a_traders_saved_route_yields_to_real_trading_then_a_new_route_moves_it_again() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "forceload add -32 -16 80 32",
        "fill -32 64 -16 80 64 32 minecraft:stone",
        "gamerule advance_time false",
        "time set noon",
        "weather clear",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 14.5,
        },
    );
    let (trader, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{wander_target:[I;32,65,8],DespawnDelay:48000}}"),
    );
    assert!(has_saved_target(&world, trader));
    world.tick_rate_mut().set_frozen(false);
    progress(
        &sender,
        &receiver,
        &mut world,
        trader,
        Vec3d {
            x: 32.5,
            y: 65.5,
            z: 8.5,
        },
    );
    let at = position(&world, trader);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            z: at.z + 2.0,
            ..at
        },
    );
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: trader,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    keep_alive(&sender, &opened);
    let container_id = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::OpenScreen(screen),
                ..
            } => Some(screen.container_id),
            _ => None,
        })
        .expect("real interaction opens the merchant screen");
    assert!(opened.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::MerchantOffers(offers), .. }
        if !offers.offers.is_empty())), "the trader publishes its natural offers");
    let player = world.player_entity_id_for_test(ALICE);
    let mut settled = false;
    for _ in 0..80 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, trader).unwrap();
        assert_eq!(body.merchant_trading_player(), Some(player));
        assert!(separation(body.position(), world.publication().players[0].position) < 4.0);
        let delta = body.delta_movement();
        if body.on_ground() && delta.x.hypot(delta.z) < 0.005 {
            settled = true;
            break;
        }
    }
    assert!(
        settled,
        "the actual trading body settles after its navigation stops"
    );
    assert!(
        !has_saved_target(&world, trader),
        "preempting travel clears the saved old target"
    );
    let paused = position(&world, trader);
    for _ in 0..40 {
        let output = tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, trader).unwrap();
        assert_eq!(body.merchant_trading_player(), Some(player));
        assert!(separation(body.position(), world.publication().players[0].position) < 4.0);
        assert!(
            separation(body.position(), paused) < 0.05,
            "real merchant session pauses movement: baseline={paused:?}, actual={:?}",
            body.position()
        );
        assert!(
            !output.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id: ALICE,
                    packet: ClientboundPlayPacket::ContainerClose(_),
                    ..
                }
            )),
            "the real merchant session remains open"
        );
    }
    world.tick_rate_mut().set_frozen(true);
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id }),
    );
    keep_alive(&sender, &closed);
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, trader)
            .unwrap()
            .merchant_trading_player(),
        None
    );
    let at = position(&world, trader);
    let new_x = at.x.floor() as i32 + 24;
    let new_z = at.z.floor() as i32;
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{wander_target:[I;{new_x},65,{new_z}]}}"),
    );
    assert!(has_saved_target(&world, trader));
    world.tick_rate_mut().set_frozen(false);
    progress(
        &sender,
        &receiver,
        &mut world,
        trader,
        Vec3d {
            x: f64::from(new_x) + 0.5,
            y: 65.5,
            z: f64::from(new_z) + 0.5,
        },
    );
}
