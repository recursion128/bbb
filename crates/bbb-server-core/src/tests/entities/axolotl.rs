//! Axolotl water seeking, swimming, temptation and hunting through live ticks and saves.

use super::horse_interaction::record;
use super::*;
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use bbb_server_types::geometry::Vec3Ops;
use std::sync::atomic::{AtomicU64, Ordering};

static COMMAND_ID: AtomicU64 = AtomicU64::new(100);

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLAYER: Vec3d = Vec3d {
    x: 16.5,
    y: 99.0,
    z: 8.5,
};

fn config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Creative,
        difficulty: Difficulty::Normal,
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
    let operation_id = COMMAND_ID.fetch_add(1, Ordering::Relaxed);
    let result = console_command(sender, receiver, world, operation_id, text);
    let outcome = admin_outcome(&result);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    result
}

fn prepare(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    world.tick_rate_mut().set_frozen(true);
    join_loaded(sender, receiver, world, 1, "alice", 1);
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerAbilities { flying: true },
    );
    place_player(sender, receiver, world, 1, PLAYER, None);
    for text in [
        "gamerule spawn_mobs false",
        "weather clear",
        "fill 0 97 0 26 99 16 minecraft:stone",
        "fill 8 98 4 24 99 12 minecraft:water",
    ] {
        command(sender, receiver, world, text);
    }
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config());
    prepare(&sender, &receiver, &mut world);
    (sender, receiver, world)
}

fn summon(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> (i32, Uuid) {
    let before = world.stored_entity_ids_for_test(DIM);
    command(sender, receiver, world, text);
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| !before.contains(id))
        .unwrap();
    (id, world.stored_entity_for_test(DIM, id).unwrap().uuid())
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world.stored_entity_for_test(DIM, id).unwrap().position()
}

fn health(world: &ServerWorld, id: i32) -> Option<f32> {
    world
        .stored_entity_for_test(DIM, id)
        .and_then(|body| body.living().map(|living| living.health()))
}

fn cooldown(fields: &[NbtSummaryEntry]) -> Option<PackedMemory> {
    BrainPacked::decode(field(fields, "Brain"))
        .unwrap()
        .memories
        .into_iter()
        .find(|memory| memory.memory_type == MemoryModuleType::HasHuntingCooldown)
}

fn approach(receiver: &WorldCommandReceiver, world: &mut ServerWorld, id: i32, target: Vec3d) {
    let before = position(world, id).distance_to_sqr(target).sqrt();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..400 {
        world.tick(receiver.drain());
        if position(world, id).distance_to_sqr(target).sqrt() < before - 2.0 {
            world.tick_rate_mut().set_frozen(true);
            return;
        }
    }
    panic!(
        "the axolotl did not swim toward its visible target: position={:?}, target={target:?}",
        position(world, id)
    );
}

#[test]
fn a_baby_axolotl_finds_water_swims_for_food_then_follows_a_visible_adult() {
    let (sender, receiver, mut world) = fixture();
    let (baby, uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:axolotl 3.5 100 8.5 {Age:-24000,PersistenceRequired:1b}",
    );
    world.tick_rate_mut().set_frozen(false);
    let mut entered = false;
    for _ in 0..500 {
        world.tick(receiver.drain());
        let body = world.stored_entity_for_test(DIM, baby).unwrap();
        if body.is_in_water() {
            assert!(
                body.position().x > 7.0,
                "the axolotl travels off the shore into the pool"
            );
            entered = true;
            break;
        }
    }
    assert!(entered, "the shore axolotl did not find nearby water");
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:tropical_fish_bucket",
    );
    approach(&receiver, &mut world, baby, PLAYER);
    assert!(world
        .stored_entity_for_test(DIM, baby)
        .unwrap()
        .is_in_water());
    assert!(super::feeding::animal_record_int(&world, baby, "Age") < 0);
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
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 20.5 99 8.5"),
    );
    let (adult, _) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:axolotl 9.5 99 8.5 {Age:0,NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let empty_player = Vec3d { x: 24.5, ..PLAYER };
    place_player(&sender, &receiver, &mut world, 1, empty_player, None);
    let before_player = position(&world, baby).distance_to_sqr(empty_player);
    let adult_position = position(&world, adult);
    approach(&receiver, &mut world, baby, adult_position);
    assert!(
        position(&world, baby).distance_to_sqr(empty_player) > before_player + 4.0,
        "without food the baby swims away from the player toward its adult"
    );
}

#[test]
fn an_axolotl_hunts_visible_aquatic_prey_and_keeps_its_hunting_cooldown_after_a_restart() {
    let (sender, receiver, mut world) = fixture();
    let (axolotl, uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:axolotl 10.5 99 8.5 {Age:0,PersistenceRequired:1b}",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "setblock 9 99 8 minecraft:stone",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "fill 9 98 10 12 103 10 minecraft:stone",
    );
    let (dry, dry_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cod 9.5 100 8.5 {NoAI:1b,NoGravity:1b,Air:6000s,PersistenceRequired:1b}",
    );
    let (hidden, hidden_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cod 10.5 99 11.5 {NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    let (cow, cow_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cow 10.5 99 7.5 {NoAI:1b,NoGravity:1b,Air:6000s,PersistenceRequired:1b}",
    );
    let (prey, prey_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cod 15.5 99 8.5 {NoAI:1b,NoGravity:1b,Health:2f,PersistenceRequired:1b}",
    );
    let (spared, spared_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cod 18.5 99 8.5 {NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    let spared_health = health(&world, spared);
    let untouched = [
        (dry, health(&world, dry)),
        (hidden, health(&world, hidden)),
        (cow, health(&world, cow)),
    ];
    world.tick_rate_mut().set_frozen(false);
    let mut selected_prey = false;
    let mut killed_prey = false;
    let mut hunted = None;
    for _ in 0..400 {
        world.tick(receiver.drain());
        let target = world.mob_target_for_test(DIM, axolotl);
        assert!(
            ![dry_uuid, hidden_uuid, cow_uuid]
                .iter()
                .any(|uuid| target == Some(*uuid)),
            "a nearer dry, hidden or unrelated mob is not valid prey"
        );
        assert_ne!(
            target,
            Some(spared_uuid),
            "finishing the nearer hunt does not start another fish hunt"
        );
        selected_prey |= target == Some(prey_uuid);
        killed_prey |= health(&world, prey).is_some_and(|value| value <= 0.0);
        if let Some(memory) = cooldown(&record(&world, axolotl)) {
            hunted = Some(memory);
            break;
        }
    }
    assert!(
        selected_prey && killed_prey,
        "the axolotl hunts and kills the visible aquatic fish"
    );
    for (id, before) in untouched {
        assert_eq!(health(&world, id), before, "invalid prey remains unharmed");
    }
    assert_eq!(
        health(&world, spared),
        spared_health,
        "the second visible fish survives the completed hunt"
    );
    let hunting_cooldown = hunted.expect("finishing a real hunt starts the cooldown");
    assert_eq!(hunting_cooldown.value, MemoryValue::Bool(true));
    assert!(hunting_cooldown
        .time_to_live
        .is_some_and(|ttl| (2300..=2400).contains(&ttl)));
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..120 {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(
        cooldown(&record(&world, axolotl)),
        Some(hunting_cooldown.clone())
    );
    let age = field(&record(&world, axolotl), "Age").clone();
    let request = world
        .save_request(1_786_000_011_000, 211, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "Age"), &age);
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    assert_eq!(cooldown(&saved), Some(hunting_cooldown.clone()));
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let axolotl = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let restored_record = record(&restored, axolotl);
    assert_eq!(field(&restored_record, "Age"), &age);
    assert_eq!(field(&restored_record, "NoAI"), &NbtSummaryValue::Byte(1));
    assert_eq!(cooldown(&restored_record), Some(hunting_cooldown));
    prepare(&sender, &receiver, &mut restored);
    assert!(restored
        .player_tracking_viewers_for_test(DIM, axolotl)
        .contains(&ConnectionId(1)));
    let spared = restored
        .live_entity_id_for_uuid_for_test(DIM, spared_uuid)
        .unwrap();
    assert_eq!(health(&restored, spared), spared_health);
    assert!(position(&restored, axolotl).distance_to_sqr(position(&restored, spared)) <= 64.0);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..100 {
        restored.tick(receiver.drain());
        assert_ne!(
            restored.mob_target_for_test(DIM, axolotl),
            Some(spared_uuid),
            "the restored cooldown prevents another fish hunt"
        );
    }
    assert_eq!(health(&restored, spared), spared_health);
    assert!(cooldown(&record(&restored, axolotl)).is_some());
    restored.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("tp {uuid} 10.5 99 8.5"),
    );
    let (hostile, _) = summon(
        &sender,
        &receiver,
        &mut restored,
        "summon minecraft:drowned 12.5 99 8.5 {NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    let before_hostile = health(&restored, hostile).unwrap();
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    let mut attacked_hostile = false;
    for _ in 0..300 {
        restored.tick(receiver.drain());
        assert_ne!(
            restored.mob_target_for_test(DIM, axolotl),
            Some(spared_uuid)
        );
        if health(&restored, hostile).is_some_and(|value| value <= before_hostile - 2.0) {
            attacked_hostile = true;
            break;
        }
    }
    assert!(
        attacked_hostile,
        "the hunting cooldown still permits attacking a hostile aquatic mob"
    );
    assert_eq!(health(&restored, spared), spared_health);
}

fn play_dead_ticks(world: &ServerWorld, id: i32) -> Option<i32> {
    BrainPacked::decode(field(&record(world, id), "Brain"))
        .unwrap()
        .memories
        .into_iter()
        .find_map(|memory| match (memory.memory_type, memory.value) {
            (MemoryModuleType::PlayDeadTicks, MemoryValue::Int(ticks)) => Some(ticks),
            _ => None,
        })
}

fn playing_dead(world: &ServerWorld, id: i32) -> bool {
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    crate::entity::tracking::synched_values(body)
        .iter()
        .any(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID
                && row.value.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true)
        })
}

fn playing_dead_update(tick: &TickResult, id: i32, expected: bool) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|row|
            row.data_id == bbb_protocol::entity_synched_data::VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID
            && row.value == bbb_protocol::packets::EntityDataValueKind::Boolean(expected)))
    })
}

#[test]
fn a_hurt_axolotl_plays_dead_heals_and_resumes_its_saved_countdown_after_a_restart() {
    let (sender, receiver, mut world) = fixture();
    let (axolotl, uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:axolotl 14.5 99 8.5 {Age:0,PersistenceRequired:1b}",
    );
    let (_, attacker) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:cow 20.5 99 8.5 {NoAI:1b,NoGravity:1b,PersistenceRequired:1b}",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:tropical_fish_bucket",
    );
    world.tick_rate_mut().set_frozen(false);
    world.tick(receiver.drain());
    world.tick_rate_mut().set_frozen(true);
    let mut triggered = false;
    for _ in 0..48 {
        if health(&world, axolotl).unwrap() < 14.0 {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{Health:14f}}"),
            );
        }
        let damage = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 3 minecraft:mob_attack by {attacker}"),
        );
        assert!(
            health(&world, axolotl).unwrap() < 14.0,
            "the entity-caused damage is real"
        );
        let mut pose_packet = playing_dead_update(&damage, axolotl, true);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..20 {
            let tick = world.tick(receiver.drain());
            pose_packet |= playing_dead_update(&tick, axolotl, true);
            if playing_dead(&world, axolotl) && pose_packet {
                triggered = true;
                break;
            }
        }
        if triggered {
            break;
        }
        assert!(
            !playing_dead(&world, axolotl),
            "playing-dead metadata never reached the player"
        );
        for _ in 0..21 {
            world.tick(receiver.drain());
        }
        world.tick_rate_mut().set_frozen(true);
    }
    assert!(
        triggered,
        "repeated real nonlethal hits never triggered playing dead"
    );
    world.tick_rate_mut().set_frozen(true);
    let food_target = Vec3d {
        x: position(&world, axolotl).x + 6.0,
        y: 99.0,
        z: position(&world, axolotl).z,
    };
    place_player(&sender, &receiver, &mut world, 1, food_target, None);
    let (enemy, enemy_uuid) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:drowned 12.5 99 8.5 {NoGravity:1b,PersistenceRequired:1b}",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {enemy_uuid} minecraft:movement_speed base set 0"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {enemy_uuid} minecraft:attack_damage base set 0"),
    );
    let before_healing = health(&world, axolotl).unwrap();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    let resting = position(&world, axolotl);
    for _ in 0..40 {
        world.tick(receiver.drain());
        assert!(playing_dead(&world, axolotl));
        assert_ne!(
            world.mob_target_for_test(DIM, enemy),
            Some(uuid),
            "enemies ignore the playing-dead axolotl"
        );
    }
    assert!(
        position(&world, axolotl).distance_to_sqr(resting) < 0.25,
        "playing dead suspends swimming toward the food holder"
    );
    assert!(
        health(&world, axolotl).unwrap() > before_healing,
        "playing dead regeneration restores health"
    );
    world.tick_rate_mut().set_frozen(true);
    let before_second_hit = health(&world, axolotl).unwrap();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:mob_attack by {attacker}"),
    );
    assert!(
        health(&world, axolotl).unwrap() < before_second_hit,
        "playing dead does not make the axolotl invulnerable"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("kill {enemy_uuid}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Motion:[0d,0d,0d]}}"),
    );
    let paused = play_dead_ticks(&world, axolotl).unwrap();
    assert!(paused > 0 && playing_dead(&world, axolotl));
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    world.tick_rate_mut().set_frozen(true);
    assert_eq!(play_dead_ticks(&world, axolotl), Some(paused));
    assert!(
        playing_dead(&world, axolotl),
        "NoAI preserves the active pose"
    );
    let saved_effect = world
        .stored_entity_for_test(DIM, axolotl)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:regeneration")
        .unwrap()
        .clone();
    let request = world
        .save_request(1_786_000_012_000, 212, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    assert_eq!(field(&saved, "NoAI"), &NbtSummaryValue::Byte(1));
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config(), &candidate);
    let axolotl = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(play_dead_ticks(&restored, axolotl), Some(paused));
    assert!(
        !playing_dead(&restored, axolotl),
        "the live pose is not a saved field"
    );
    assert_eq!(
        field(&record(&restored, axolotl), "NoAI"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(
        restored
            .stored_entity_for_test(DIM, axolotl)
            .unwrap()
            .living()
            .unwrap()
            .active_effect("minecraft:regeneration"),
        Some(&saved_effect)
    );
    prepare(&sender, &receiver, &mut restored);
    place_player(&sender, &receiver, &mut restored, 1, food_target, None);
    command(
        &sender,
        &receiver,
        &mut restored,
        "item replace entity alice weapon.mainhand with minecraft:tropical_fish_bucket",
    );
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    restored.tick_rate_mut().set_frozen(false);
    let mut rebuilt = false;
    for _ in 0..20 {
        let tick = restored.tick(receiver.drain());
        rebuilt |= playing_dead_update(&tick, axolotl, true);
        if rebuilt && playing_dead(&restored, axolotl) {
            break;
        }
    }
    assert!(rebuilt && playing_dead(&restored, axolotl));
    let regen = restored
        .stored_entity_for_test(DIM, axolotl)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:regeneration")
        .unwrap();
    assert!(
        regen.duration() <= saved_effect.duration(),
        "a restart does not restart regeneration without a fresh attacker memory"
    );
    let mut recovered = false;
    for _ in 0..240 {
        restored.tick(receiver.drain());
        if play_dead_ticks(&restored, axolotl).is_none() && !playing_dead(&restored, axolotl) {
            recovered = true;
            break;
        }
    }
    restored.tick_rate_mut().set_frozen(true);
    assert!(
        recovered,
        "the restored playing-dead countdown eventually ends"
    );
    approach(&receiver, &mut restored, axolotl, food_target);
}

fn player_finishes_an_axolotl_hunt(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    axolotl: i32,
    kind: &str,
    existing_seconds: Option<i32>,
) -> Vec<WorldEvent> {
    let (victim, victim_uuid) = summon(
        sender,
        receiver,
        world,
        &format!(
            "summon minecraft:{kind} 14.5 99 8.5 {{NoAI:1b,NoGravity:1b,PersistenceRequired:1b}}"
        ),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut acquired = false;
    for _ in 0..160 {
        world.tick(receiver.drain());
        if world.mob_target_for_test(DIM, axolotl) == Some(victim_uuid) {
            acquired = true;
            break;
        }
    }
    assert!(acquired, "the axolotl actually starts hunting the victim");
    world.tick_rate_mut().set_frozen(true);
    if let Some(seconds) = existing_seconds {
        command(
            sender,
            receiver,
            world,
            &format!("effect give alice minecraft:regeneration {seconds}"),
        );
    }
    command(
        sender,
        receiver,
        world,
        "effect give alice minecraft:mining_fatigue 60",
    );
    let kill = command(
        sender,
        receiver,
        world,
        &format!("damage {victim_uuid} 100 minecraft:player_attack by alice"),
    );
    assert_eq!(health(world, victim), Some(0.0));
    assert_eq!(
        world
            .stored_entity_for_test(DIM, victim)
            .unwrap()
            .living()
            .unwrap()
            .last_damage_source_entity(),
        Some(world.player_entity_id_for_test(ConnectionId(1)))
    );
    let mut events = kill.events;
    world.tick_rate_mut().set_frozen(false);
    let mut supported = false;
    for _ in 0..60 {
        events.extend(world.tick(receiver.drain()).events);
        if world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:mining_fatigue")
            .is_none()
        {
            supported = true;
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(
        supported,
        "the nearby player who finished the hunt receives support"
    );
    events
}

#[test]
fn an_axolotl_rewards_a_players_real_hunt_kills_with_capped_regeneration_and_fatigue_removal() {
    let (sender, receiver, mut world) = fixture();
    let (axolotl, _) = summon(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:axolotl 10.5 99 8.5 {Age:0,PersistenceRequired:1b}",
    );
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let regen_id = bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
        "minecraft:regeneration",
    )
    .unwrap()
    .id;
    let fatigue_id = bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
        "minecraft:mining_fatigue",
    )
    .unwrap()
    .id;
    assert!(world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:regeneration")
        .is_none());
    for (existing_seconds, kind, expected_duration) in [
        (None, "cod", Some(100)),
        (Some(119), "drowned", Some(2400)),
        (Some(180), "drowned", None),
    ] {
        let events = player_finishes_an_axolotl_hunt(
            &sender,
            &receiver,
            &mut world,
            axolotl,
            kind,
            existing_seconds,
        );
        let regen = world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:regeneration")
            .unwrap();
        assert_eq!(regen.amplifier(), 0);
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::RemoveMobEffect(remove) }
            if remove.entity_id == player && remove.effect_id == fatigue_id)));
        if let Some(expected) = expected_duration {
            assert!(events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::UpdateMobEffect(update) }
                if update.entity_id == player && update.effect_id == regen_id && update.duration_ticks == expected && update.amplifier == 0)));
        } else {
            assert!(
                regen.duration() > 2400,
                "support does not shorten an existing effect above the cap"
            );
        }
    }
}
