//! Baby piglin flight and outnumbered adult retreat through real damage and ticks.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID,
};

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
fn arena() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    (sender, receiver, world)
}
fn mob(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    kind: i32,
    x: f64,
    z: f64,
    flags: &str,
) -> (i32, Uuid) {
    let (id, uuid) = spawn(world, kind, Vec3d { x, y: 65.0, z });
    command(
        sender,
        receiver,
        world,
        &format!("data merge entity {uuid} {{IsImmuneToZombification:1b,{flags}}}"),
    );
    command(
        sender,
        receiver,
        world,
        &format!("attribute {uuid} minecraft:knockback_resistance base set 1"),
    );
    if kind == VANILLA_ENTITY_TYPE_PIGLIN_ID {
        command(
            sender,
            receiver,
            world,
            &format!("item replace entity {uuid} weapon.mainhand with minecraft:golden_sword"),
        );
        command(
            sender,
            receiver,
            world,
            &format!("attribute {uuid} minecraft:attack_damage base set 1"),
        );
    }
    (id, uuid)
}
fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}
fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}
fn fleeing_speed(world: &ServerWorld, id: i32) -> bool {
    let living = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap();
    (f64::from(living.speed()) - living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)).abs()
        < 1.0e-5
}
fn attack(output: &TickResult, id: i32) -> bool {
    output.events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::EntityAnimation(animation), .. } if animation.id == id && animation.action == 0))
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
fn memory(world: &ServerWorld, id: i32, name: &str) -> Option<NbtSummaryValue> {
    let record = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap();
    field(field(field(&record, "Brain")?, "memories")?, name).cloned()
}
fn assert_cooldown(world: &ServerWorld, id: i32) {
    let saved = memory(world, id, "minecraft:hunted_recently")
        .expect("retreat writes hunting cooldown to saved Brain NBT");
    assert_eq!(field(&saved, "value"), Some(&NbtSummaryValue::Byte(1)));
    assert!(
        matches!(field(&saved, "ttl"), Some(NbtSummaryValue::Long(ttl)) if (1..=2400).contains(ttl)),
        "{saved:?}"
    );
}

#[test]
fn a_hurt_baby_piglin_flees_pauses_without_ai_and_calls_an_adult_again_while_no_ai() {
    let (sender, receiver, mut world) = arena();
    let (baby, baby_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "IsBaby:1b,NoAI:0b",
    );
    let (ally, ally_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        3.5,
        15.5,
        "NoAI:1b",
    );
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        12.5,
        8.5,
        "NoAI:1b",
    );
    let before = position(&world, baby);
    let cow_health = health(&world, cow);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {baby_uuid} 1 minecraft:mob_attack by {cow_uuid}"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut ran = false;
    for _ in 0..4 {
        tick(&sender, &receiver, &mut world);
        ran |= fleeing_speed(&world, baby);
        assert_eq!(health(&world, cow), cow_health);
    }
    assert!(
        ran && separation(position(&world, baby), position(&world, cow))
            > separation(before, position(&world, cow)) + 0.1,
        "full knockback resistance leaves actual navigation to move the baby away"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {baby_uuid} {{NoAI:1b}}"),
    );
    for _ in 0..160 {
        tick(&sender, &receiver, &mut world);
    }
    let paused_at = position(&world, baby);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {baby_uuid} {{NoAI:0b}}"),
    );
    ran = false;
    for _ in 0..15 {
        tick(&sender, &receiver, &mut world);
        ran |= fleeing_speed(&world, baby);
        assert_eq!(health(&world, cow), cow_health);
    }
    assert!(
        ran && separation(position(&world, baby), position(&world, cow))
            > separation(paused_at, position(&world, cow)) + 1.0,
        "the baby resumes flight after more than 100 paused world ticks"
    );
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {baby_uuid} {{NoAI:1b}}"),
    );
    assert!(memory(&world, ally, "minecraft:angry_at").is_some());
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data remove entity {ally_uuid} Brain.memories.\"minecraft:angry_at\""),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {baby_uuid} 1 minecraft:mob_attack by {cow_uuid}"),
    );
    assert!(
        memory(&world, ally, "minecraft:angry_at").is_some(),
        "the NoAI baby broadcasts a fresh anger memory"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {ally_uuid} {{NoAI:0b}}"),
    );
    let ally_before = position(&world, ally);
    world.tick_rate_mut().set_frozen(false);
    let mut hit = false;
    for _ in 0..240 {
        tick(&sender, &receiver, &mut world);
        if health(&world, cow) < cow_health {
            hit = true;
            break;
        }
    }
    assert!(hit && position(&world, ally).x > ally_before.x + 2.0,
        "the adult approaches and attacks the cow while the baby stays NoAI: ally={:?}, cow_health={}", position(&world, ally), health(&world, cow));
}

#[test]
fn outnumbered_adults_retreat_with_cannot_hunt_peers_but_not_brutes_and_counterattack_a_cow() {
    let (sender, receiver, mut world) = arena();
    let (leader, leader_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b,CannotHunt:1b",
    );
    let (peer, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        2.5,
        15.5,
        "NoAI:0b,CannotHunt:1b",
    );
    let (brute, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        14.5,
        15.5,
        "NoAI:1b",
    );
    let brute_cooldown = memory(&world, brute, "minecraft:hunted_recently");
    let (attacker, attacker_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        11.5,
        8.5,
        "NoAI:1b",
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        6.5,
        4.5,
        "NoAI:1b",
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        10.5,
        4.5,
        "NoAI:1b",
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..25 {
        tick(&sender, &receiver, &mut world);
    }
    world.tick_rate_mut().set_frozen(true);
    let attacker_health = health(&world, attacker);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {leader_uuid} 1 minecraft:mob_attack by {attacker_uuid}"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut fought = false;
    for _ in 0..180 {
        tick(&sender, &receiver, &mut world);
        if health(&world, attacker) < attacker_health {
            fought = true;
            break;
        }
    }
    assert!(
        fought,
        "the brute counts toward equal numbers, allowing actual retaliation"
    );
    world.tick_rate_mut().set_frozen(true);
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        8.5,
        12.5,
        "NoAI:1b",
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..25 {
        tick(&sender, &receiver, &mut world);
    }
    world.tick_rate_mut().set_frozen(true);
    let leader_before = position(&world, leader);
    let peer_before = position(&world, peer);
    let threat = position(&world, attacker);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {leader_uuid} 1 minecraft:mob_attack by {attacker_uuid}"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut leader_ran = false;
    let mut peer_ran = false;
    for _ in 0..80 {
        tick(&sender, &receiver, &mut world);
        leader_ran |= fleeing_speed(&world, leader)
            && separation(position(&world, leader), threat)
                > separation(leader_before, threat) + 2.0;
        peer_ran |= fleeing_speed(&world, peer)
            && separation(position(&world, peer), threat) > separation(peer_before, threat) + 2.0;
        if leader_ran && peer_ran {
            break;
        }
    }
    assert!(
        leader_ran && peer_ran,
        "the fourth visible adult hoglin makes both ordinary piglins flee: leader={:?}, peer={:?}",
        position(&world, leader),
        position(&world, peer)
    );
    assert_cooldown(&world, leader);
    assert_cooldown(&world, peer);
    assert_eq!(
        memory(&world, brute, "minecraft:hunted_recently"),
        brute_cooldown,
        "the brute is counted but receives no retreat cooldown broadcast"
    );
    world.tick_rate_mut().set_frozen(true);
    let close = position(&world, leader);
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        close.x + 2.0,
        close.z + 2.0,
        "NoAI:1b",
    );
    let cow_health = health(&world, cow);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {leader_uuid} 2 minecraft:mob_attack by {cow_uuid}"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut fought_back = false;
    let mut leader_attacked = false;
    for _ in 0..160 {
        let output = tick(&sender, &receiver, &mut world);
        leader_attacked |= attack(&output, leader);
        if leader_attacked && health(&world, cow) < cow_health {
            fought_back = true;
            break;
        }
    }
    assert!(fought_back && separation(position(&world, leader), close) > 0.5,
        "a different-species attacker clears avoidance and the leader really counters: leader={:?}, cow_health={}", position(&world, leader), health(&world, cow));
}
