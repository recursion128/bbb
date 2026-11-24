//! Ghastling food temptation and visible player/adult following through real ticks.

use super::horse_interaction::record;
use super::*;
use bbb_server_types::geometry::Vec3Ops;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 100.0,
    z: 8.5,
};
const FOOD_PLAYER: Vec3d = Vec3d { x: 20.5, ..START };
const NEAR_PLAYER: Vec3d = Vec3d { x: 2.5, ..START };
const COW: Vec3d = Vec3d { x: 0.5, ..START };

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
    assert!(text.chars().count() <= 256);
    let tick = console_command(sender, receiver, world, 100, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn prepare(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    spectators: bool,
) {
    world.tick_rate_mut().set_frozen(true);
    for (connection, name, position) in [(1, "alice", FOOD_PLAYER), (2, "bob", NEAR_PLAYER)] {
        join_loaded(
            sender,
            receiver,
            world,
            connection,
            name,
            connection as u128,
        );
        let mode = if spectators { "spectator" } else { "creative" };
        command(sender, receiver, world, &format!("gamemode {mode} {name}"));
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

fn find(world: &ServerWorld, entity_type: i32) -> (i32, Uuid) {
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == entity_type
        })
        .unwrap();
    (id, world.stored_entity_for_test(DIM, id).unwrap().uuid())
}

fn fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world, false);
    command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:happy_ghast 8.5 100 8.5 {Age:-24000}",
    );
    let (id, uuid) = find(
        &world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    );
    (sender, receiver, world, id, uuid)
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world.stored_entity_for_test(DIM, id).unwrap().position()
}

fn approach(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: i32,
    target: Vec3d,
) -> Vec3d {
    let before = position(world, id).distance_to_sqr(target).sqrt();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..300 {
        world.tick(receiver.drain());
        let now = position(world, id);
        assert!(now.y > 90.0, "the ghastling remains airborne: {now:?}");
        if now.distance_to_sqr(target).sqrt() < before - 2.0 {
            world.tick_rate_mut().set_frozen(true);
            return now;
        }
    }
    panic!("the ghastling did not approach its visible target: before={before}, position={:?}, target={target:?}", position(world, id));
}

fn reset_position(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    uuid: Uuid,
) {
    command(
        sender,
        receiver,
        world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    command(sender, receiver, world, &format!("tp {uuid} 8.5 100 8.5"));
}

#[test]
fn a_ghastling_prefers_the_food_holder_then_follows_the_nearer_player_without_food() {
    let (sender, receiver, mut world, ghastling, uuid) = fixture();
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
    let before_near = position(&world, ghastling).distance_to_sqr(NEAR_PLAYER);
    let tempted = approach(&receiver, &mut world, ghastling, FOOD_PLAYER);
    assert!(
        tempted.distance_to_sqr(NEAR_PLAYER) > before_near,
        "food takes priority over ordinary following of the nearer harness holder"
    );
    assert!(super::feeding::animal_record_int(&world, ghastling, "Age") < 0);
    reset_position(&sender, &receiver, &mut world, uuid);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:air",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let before_far = position(&world, ghastling).distance_to_sqr(FOOD_PLAYER);
    let followed = approach(&receiver, &mut world, ghastling, NEAR_PLAYER);
    assert!(
        followed.distance_to_sqr(FOOD_PLAYER) > before_far,
        "without food the ghastling follows the nearer visible player"
    );
}

#[test]
fn a_ghastling_prefers_a_player_then_follows_a_friendly_adult_after_a_restart() {
    let (sender, receiver, mut world, ghastling, uuid) = fixture();
    command(&sender, &receiver, &mut world, "gamemode spectator bob");
    command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cow 0.5 100 8.5 {Age:0,NoAI:1b,NoGravity:1b}",
    );
    let (cow, cow_uuid) = find(
        &world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    assert_eq!(position(&world, cow), COW);
    let before_cow = position(&world, ghastling).distance_to_sqr(COW);
    let followed = approach(&receiver, &mut world, ghastling, FOOD_PLAYER);
    assert!(
        followed.distance_to_sqr(COW) > before_cow,
        "an ordinary visible player takes priority over the nearer friendly adult"
    );
    command(&sender, &receiver, &mut world, "gamemode spectator alice");
    reset_position(&sender, &receiver, &mut world, uuid);
    let age = field(&record(&world, ghastling), "Age").clone();
    assert!(matches!(age, NbtSummaryValue::Int(value) if value < 0));
    let request = world
        .save_request(1_786_000_008_000, 208, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "Age"), &age);
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let ghastling = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let cow = restored
        .live_entity_id_for_uuid_for_test(DIM, cow_uuid)
        .unwrap();
    assert_eq!(field(&record(&restored, ghastling), "Age"), &age);
    assert_eq!(super::feeding::animal_record_int(&restored, cow, "Age"), 0);
    prepare(&sender, &receiver, &mut restored, true);
    assert!(restored
        .player_tracking_viewers_for_test(DIM, ghastling)
        .contains(&ConnectionId(1)));
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let before_player = position(&restored, ghastling).distance_to_sqr(FOOD_PLAYER);
    let target = position(&restored, cow);
    let followed = approach(&receiver, &mut restored, ghastling, target);
    assert!(
        followed.distance_to_sqr(FOOD_PLAYER) > before_player,
        "spectators do not prevent following the visible adult cow after a restart"
    );
}

#[test]
fn a_targetless_ghastling_resumes_idle_flight_after_a_no_ai_restart() {
    let (sender, receiver, mut world, ghastling, uuid) = fixture();
    for name in ["alice", "bob"] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("gamemode spectator {name}"),
        );
    }
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    let stopped = position(&world, ghastling);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert!(
        position(&world, ghastling).distance_to_sqr(stopped) < 1e-10,
        "NoAI prevents targetless flight"
    );
    world.tick_rate_mut().set_frozen(true);
    let age = field(&record(&world, ghastling), "Age").clone();
    assert!(matches!(age, NbtSummaryValue::Int(value) if value < 0));
    let request = world
        .save_request(1_786_000_009_000, 209, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "Age"), &age);
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let ghastling = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(position(&restored, ghastling), stopped);
    let restored_record = record(&restored, ghastling);
    assert_eq!(field(&restored_record, "Age"), &age);
    assert_eq!(field(&restored_record, "NoAI"), &NbtSummaryValue::Byte(1));
    prepare(&sender, &receiver, &mut restored, true);
    for connection in [ConnectionId(1), ConnectionId(2)] {
        assert!(restored
            .player_tracking_viewers_for_test(DIM, ghastling)
            .contains(&connection));
    }
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    let mut wandered = false;
    for _ in 0..300 {
        restored.tick(receiver.drain());
        let now = position(&restored, ghastling);
        assert!(now.y > 90.0, "idle flight remains airborne: {now:?}");
        if now.distance_to_sqr(stopped) > 1.0 {
            wandered = true;
            break;
        }
    }
    assert!(
        wandered,
        "the targetless ghastling did not resume idle flight: {:?}",
        position(&restored, ghastling)
    );
}

#[test]
fn a_hurt_ghastling_keeps_panicking_through_a_restart_seeks_water_then_resumes_temptation() {
    use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue};

    let panic_memory = |fields: &[NbtSummaryEntry]| {
        BrainPacked::decode(field(fields, "Brain"))
            .unwrap()
            .memories
            .into_iter()
            .find(|memory| memory.memory_type == MemoryModuleType::IsPanicking)
    };
    let (sender, receiver, mut world, ghastling, uuid) = fixture();
    command(&sender, &receiver, &mut world, "gamemode spectator bob");
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:snowball",
    );
    assert!(panic_memory(&record(&world, ghastling)).is_none());
    let hurt = world
        .hurt_stored_entity_by_command_for_test(DIM, ghastling, "minecraft:mob_attack", 1.0, None)
        .unwrap();
    assert!(hurt.applied && hurt.succeeded);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    let panicking = panic_memory(&record(&world, ghastling)).unwrap();
    assert_eq!(panicking.value, MemoryValue::Bool(true));
    assert_eq!(panicking.time_to_live, None);
    assert!(
        position(&world, ghastling).distance_to_sqr(START) > 1.0,
        "damage starts airborne panic movement"
    );
    world.tick_rate_mut().set_frozen(true);
    reset_position(&sender, &receiver, &mut world, uuid);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..160 {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(position(&world, ghastling), START);
    assert_eq!(
        panic_memory(&record(&world, ghastling)),
        Some(panicking.clone())
    );
    let age = field(&record(&world, ghastling), "Age").clone();
    assert!(matches!(age, NbtSummaryValue::Int(value) if value < 0));
    let request = world
        .save_request(1_786_000_010_000, 210, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "Age"), &age);
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    assert_eq!(panic_memory(&saved), Some(panicking.clone()));
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let ghastling = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(position(&restored, ghastling), START);
    let restored_record = record(&restored, ghastling);
    assert_eq!(field(&restored_record, "Age"), &age);
    assert_eq!(field(&restored_record, "NoAI"), &NbtSummaryValue::Byte(1));
    assert_eq!(panic_memory(&restored_record), Some(panicking));
    prepare(&sender, &receiver, &mut restored, false);
    assert!(restored
        .player_tracking_viewers_for_test(DIM, ghastling)
        .contains(&ConnectionId(1)));
    command(&sender, &receiver, &mut restored, "gamemode spectator bob");
    command(
        &sender,
        &receiver,
        &mut restored,
        "item replace entity alice weapon.mainhand with minecraft:snowball",
    );
    // A non-colliding front rim contains the water without obstructing the flight into it.
    command(
        &sender,
        &receiver,
        &mut restored,
        "fill 3 98 6 7 99 10 minecraft:stone",
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        "fill 7 99 6 7 99 10 minecraft:oak_sign",
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        "fill 4 99 7 6 99 9 minecraft:water",
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b,Fire:200s}}"),
    );
    let before_food = position(&restored, ghastling).distance_to_sqr(FOOD_PLAYER);
    restored.tick_rate_mut().set_frozen(false);
    let mut extinguished_at = None;
    let mut entered_water = false;
    let mut moved_after_water = false;
    for _ in 0..90 {
        restored.tick(receiver.drain());
        entered_water |= restored
            .stored_entity_for_test(DIM, ghastling)
            .unwrap()
            .is_in_water();
        let now = position(&restored, ghastling);
        let fields = record(&restored, ghastling);
        assert!(
            panic_memory(&fields).is_some(),
            "panic outlasts the first destination"
        );
        if let Some(wet) = extinguished_at {
            moved_after_water |= now.distance_to_sqr(wet) > 1.0;
        } else if matches!(field(&fields, "Fire"), NbtSummaryValue::Short(fire) if *fire <= 0) {
            assert!(
                now.x < START.x - 1.0 && now.distance_to_sqr(FOOD_PLAYER) > before_food,
                "the burning ghastling seeks the pool away from the food holder: {now:?}"
            );
            extinguished_at = Some(now);
        }
    }
    assert!(entered_water, "the ghastling never entered water");
    assert!(
        extinguished_at.is_some(),
        "water did not extinguish the ghastling before its fire expired"
    );
    assert!(
        moved_after_water,
        "panic chooses another flight after reaching water"
    );
    for _ in 0..100 {
        restored.tick(receiver.drain());
    }
    restored.tick_rate_mut().set_frozen(true);
    assert!(panic_memory(&record(&restored, ghastling)).is_none());
    reset_position(&sender, &receiver, &mut restored, uuid);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    approach(&receiver, &mut restored, ghastling, FOOD_PLAYER);
}
