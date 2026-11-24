//! A real player-led llama walks below the elastic leash threshold.

use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID;
use bbb_server_types::geometry::Vec3Ops;

const ALICE: ConnectionId = ConnectionId(1);
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

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

fn center_distance(world: &ServerWorld, llama: i32) -> (f64, f64) {
    let player = world.publication().players[0].position;
    let holder = bbb_server_types::interaction::player_bounding_box(player).unwrap();
    let body = world.stored_entity_for_test(DIMENSION, llama).unwrap();
    let distance = body
        .bounding_box()
        .center()
        .distance_to_sqr(holder.center())
        .sqrt();
    // Leashable.java:185-187; the elastic branch also subtracts both actual widths.
    let elastic_threshold = 6.0 - (holder.max_x - holder.min_x) - f64::from(body.bb_width());
    (distance, elastic_threshold)
}
fn dropped_lead(world: &ServerWorld) -> bool {
    world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .any(|id| {
            let Some(body) = world.stored_entity_for_test(DIMENSION, id) else {
                return false;
            };
            let EntityData::Item(item) = body.data() else {
                return false;
            };
            item.item().item_resource_id() == Some("minecraft:lead")
        })
}

#[test]
fn a_player_led_llama_walks_close_to_its_holder_without_elastic_pulling() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamerule spawn_mobs false",
        "forceload add -16 -16 32 32",
        "fill -16 64 -16 32 64 32 minecraft:stone",
        "time set noon",
        "weather clear",
        "item replace entity alice weapon.mainhand with minecraft:lead 2",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (llama, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_LLAMA_ID).unwrap()),
    );
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 10.0,
            y: 65.0,
            z: 8.5,
        },
    );
    tick(&sender, &receiver, &mut world);
    let attached = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: llama,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    keep_alive(&sender, &attached);
    let holder_id = world.player_entity_id_for_test(ALICE);
    assert_eq!(
        crate::entity::leash::leash_holder(world.stored_entity_for_test(DIMENSION, llama).unwrap()),
        Some(holder_id)
    );
    assert!(attached.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::SetEntityLink(link), .. }
        if link.source_id == llama && link.dest_id == holder_id)), "the real lead attachment is published");
    let holder_position = Vec3d {
        x: 12.0,
        y: 65.0,
        z: 8.5,
    };
    world.set_player_position_for_test(ALICE, holder_position);
    tick(&sender, &receiver, &mut world);
    assert_eq!(world.publication().players[0].position, holder_position);
    let initial = center_distance(&world, llama).0;
    assert!(initial > 3.0);
    let health = world
        .stored_entity_for_test(DIMENSION, llama)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut walked = false;
    let mut best_distance = initial;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..300 {
        let (distance_before, threshold_before) = center_distance(&world, llama);
        assert!(distance_before < 4.0 && distance_before < threshold_before,
            "the public player input stays below elastic admission: distance={distance_before}, threshold={threshold_before}");
        let output = tick(&sender, &receiver, &mut world);
        assert_eq!(world.publication().players[0].position, holder_position);
        let (distance, threshold) = center_distance(&world, llama);
        assert!(
            distance < 4.0 && distance < threshold,
            "no elastic pull is admitted: distance={distance}, threshold={threshold}"
        );
        assert!(!dropped_lead(&world));
        let body = world.stored_entity_for_test(DIMENSION, llama).unwrap();
        assert_eq!(crate::entity::leash::leash_holder(body), Some(holder_id));
        assert_eq!(body.living().unwrap().health(), health);
        assert!(!output.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::SetEntityLink(link), .. }
            if link.source_id == llama && link.dest_id == 0)));
        best_distance = best_distance.min(distance);
        let living = body.living().unwrap();
        let speed_matches = (f64::from(living.speed())
            - 2.0 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
            < 1.0e-5;
        if speed_matches
            && body.on_ground()
            && !body.is_in_water()
            && !body.is_in_lava()
            && distance < initial - 1.0
        {
            walked = true;
            break;
        }
    }
    assert!(walked, "actual grounded navigation approaches the close holder at speed2: initial={initial}, best={best_distance}");
}
