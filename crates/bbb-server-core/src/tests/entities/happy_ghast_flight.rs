//! Native happy ghast temptation, autonomous flight gates and growth.

use super::horse_interaction::record;
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::geometry::Vec3Ops;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 100.0,
    z: 8.5,
};
const ALICE: Vec3d = Vec3d { x: 20.5, ..START };
const BOB: Vec3d = Vec3d { x: 0.5, ..START };

fn config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Creative,
        keep_alive_interval_ticks: 10_000,
        ..datapack_config()
    }
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 100, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    for (connection, name, position) in [(1, "alice", ALICE), (2, "bob", BOB)] {
        join_loaded(
            sender,
            receiver,
            world,
            connection,
            name,
            connection as u128,
        );
        command(
            sender,
            receiver,
            world,
            &format!("gamemode creative {name}"),
        );
        play(
            sender,
            receiver,
            world,
            connection,
            PlayServerbound::PlayerAbilities { flying: true },
        );
        place_player(sender, receiver, world, connection, position, None);
    }
    command(sender, receiver, world, "gamerule spawn_mobs false");
}

fn fixture(
    baby: bool,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    command(
        &sender,
        &receiver,
        &mut world,
        if baby {
            "summon minecraft:happy_ghast 8.5 100 8.5 {Age:-20}"
        } else {
            "summon minecraft:happy_ghast 8.5 100 8.5"
        },
    );
    let ghast = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, ghast).unwrap().uuid();
    (sender, receiver, world, ghast, uuid)
}

fn position(world: &ServerWorld, ghast: i32) -> Vec3d {
    world.stored_entity_for_test(DIM, ghast).unwrap().position()
}

fn approach(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    ghast: i32,
    player: Vec3d,
) -> Vec3d {
    let before = position(world, ghast).distance_to_sqr(player).sqrt();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..240 {
        world.tick(receiver.drain());
        let now = position(world, ghast);
        assert!(now.y > 90.0, "the happy ghast remains airborne: {now:?}");
        if now.distance_to_sqr(player).sqrt() < before - 2.0 {
            world.tick_rate_mut().set_frozen(true);
            return now;
        }
    }
    panic!("the held item did not attract the happy ghast: start distance={before}, end={:?}, player={player:?}", position(world, ghast));
}

#[test]
fn a_happy_ghast_flies_for_harnesses_then_food_and_restores_its_flight_gates() {
    let (sender, receiver, mut world, ghast, uuid) = fixture(false);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:white_harness",
    );
    approach(&receiver, &mut world, ghast, ALICE);
    let body = world.stored_entity_for_test(DIM, ghast).unwrap();
    let yaw = f64::from(body.y_head_rot()).to_radians();
    let dx = ALICE.x - body.position().x;
    let dz = ALICE.z - body.position().z;
    assert!(
        (-yaw.sin() * dx + yaw.cos() * dz) / dx.hypot(dz) > 0.5,
        "the attracted happy ghast turns its head towards the player"
    );

    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 8.5 100 8.5"),
    );
    command(&sender, &receiver, &mut world, &format!("item replace entity {uuid} armor.body with minecraft:white_harness[custom_data={{kept:17}}]"));
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:snowball",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity bob weapon.mainhand with minecraft:white_harness",
    );
    let harness = world
        .stored_entity_for_test(DIM, ghast)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::Body)
        .unwrap()
        .clone();
    let stopped = position(&world, ghast);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(
        position(&world, ghast).distance_to_sqr(stopped) < 1e-10,
        "NoAI prevents autonomous flight"
    );
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b,still_timeout:200,Motion:[0d,0d,0d]}}"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..30 {
        world.tick(receiver.drain());
    }
    assert!(
        position(&world, ghast).distance_to_sqr(stopped) < 1e-10,
        "the positive still timeout prevents autonomous flight"
    );
    world.tick_rate_mut().set_frozen(true);
    let still = field(&record(&world, ghast), "still_timeout").clone();
    assert!(matches!(still, NbtSummaryValue::Int(value) if value > 0));
    let request = world
        .save_request(1_786_000_007_000, 207, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "still_timeout"), &still);
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let ghast = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(field(&record(&restored, ghast), "still_timeout"), &still);
    assert_eq!(
        restored
            .stored_entity_for_test(DIM, ghast)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Body)
            .unwrap()
            .summary(),
        harness.summary()
    );
    prepare(&sender, &receiver, &mut restored);
    assert!(restored
        .player_tracking_viewers_for_test(DIM, ghast)
        .contains(&ConnectionId(1)));
    command(
        &sender,
        &receiver,
        &mut restored,
        "item replace entity alice weapon.mainhand with minecraft:snowball",
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        "item replace entity bob weapon.mainhand with minecraft:white_harness",
    );
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        restored.tick(receiver.drain());
    }
    assert!(
        position(&restored, ghast).distance_to_sqr(stopped) < 1e-10,
        "the restored still timeout remains effective"
    );
    restored.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{still_timeout:0}}"),
    );
    let away_from_harness = position(&restored, ghast).distance_to_sqr(BOB);
    let moved = approach(&receiver, &mut restored, ghast, ALICE);
    assert!(
        moved.distance_to_sqr(BOB) > away_from_harness,
        "an equipped adult follows the food holder instead of the nearer harness holder"
    );
}

#[test]
fn a_ghastling_grows_into_an_adult_that_flies_towards_a_harness_holder() {
    let (sender, receiver, mut world, ghast, _) = fixture(true);
    let body = world.stored_entity_for_test(DIM, ghast).unwrap();
    let small_height = body.bounding_box().max_y - body.bounding_box().min_y;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..60 {
        world.tick(receiver.drain());
        if super::feeding::animal_record_int(&world, ghast, "Age") >= 0 {
            break;
        }
    }
    assert!(super::feeding::animal_record_int(&world, ghast, "Age") >= 0);
    world.tick_rate_mut().set_frozen(true);
    let body = world.stored_entity_for_test(DIM, ghast).unwrap();
    assert!(body.bounding_box().max_y - body.bounding_box().min_y > small_height);
    let target = Vec3d {
        x: body.position().x + 12.0,
        ..body.position()
    };
    place_player(&sender, &receiver, &mut world, 1, target, None);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:white_harness",
    );
    approach(&receiver, &mut world, ghast, target);
}
