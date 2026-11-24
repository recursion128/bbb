//! Adult hoglins reinforce allies and retreat together from a larger piglin group.

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
    must_succeed: bool,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let tick = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(
        !must_succeed || report.result.is_some(),
        "{text}: {report:?}"
    );
    keep_alive(sender, &tick);
    tick
}

fn keep_alive(sender: &WorldCommandSender, tick: &TickResult) {
    for event in &tick.events {
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
    let tick = world.tick(receiver.drain());
    keep_alive(sender, &tick);
    tick
}

fn arena() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text, true);
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
        true,
    );
    command(
        sender,
        receiver,
        world,
        &format!("attribute {uuid} minecraft:knockback_resistance base set 1"),
        true,
    );
    if kind == VANILLA_ENTITY_TYPE_HOGLIN_ID {
        command(
            sender,
            receiver,
            world,
            &format!("attribute {uuid} minecraft:attack_damage base set 1"),
            true,
        );
        command(
            sender,
            receiver,
            world,
            &format!("attribute {uuid} minecraft:attack_knockback base set 0"),
            true,
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
fn retreat_speed(world: &ServerWorld, id: i32) -> bool {
    let living = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap();
    (f64::from(living.speed()) - 1.3 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
        < 1.0e-5
}
fn attack(events: &[WorldEvent], id: i32) -> bool {
    events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. } if event.entity_id == id && event.event_id == 4))
}

#[test]
fn a_hurt_no_ai_adult_hoglin_calls_an_active_ally_to_attack_its_attacker() {
    let (sender, receiver, mut world) = arena();
    let (victim_id, victim) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        8.5,
        8.5,
        "NoAI:1b",
    );
    let (ally, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        11.5,
        15.5,
        "NoAI:0b",
    );
    let (cow, attacker) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        3.0,
        8.5,
        "NoAI:1b",
    );
    let initial = position(&world, ally);
    let cow_health = health(&world, cow);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {victim} 1 minecraft:mob_attack by {attacker}"),
        true,
    );
    world.tick_rate_mut().set_frozen(false);
    let mut hit = false;
    let mut ally_attacked = false;
    for _ in 0..180 {
        let output = tick(&sender, &receiver, &mut world);
        ally_attacked |= attack(&output.events, ally);
        if health(&world, cow) < cow_health {
            hit = true;
            break;
        }
    }
    assert!(
        hit && position(&world, ally).x < initial.x - 3.0,
        "the active ally approaches and hurts the fixed cow: ally={:?}, cow_health={}, victim={:?}, cow={:?}, target={:?}, ally_attacked={ally_attacked}",
        position(&world, ally),
        health(&world, cow),
        position(&world, victim_id),
        position(&world, cow),
        target_of(&world, ally),
    );
}

#[test]
fn only_an_outnumbering_visible_adult_piglin_group_makes_hoglins_flee_then_cow_hurt_restores_fighting(
) {
    let (sender, receiver, mut world) = arena();
    let (leader, leader_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b",
    );
    let (ally, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        -5.5,
        8.5,
        "NoAI:0b",
    );
    let (piglin, piglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        9.8,
        8.5,
        "NoAI:1b",
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        12.5,
        "NoAI:1b",
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        10.5,
        10.5,
        "NoAI:1b",
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        10.5,
        12.5,
        "NoAI:1b,IsBaby:1b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "fill -16 65 0 32 69 0 minecraft:stone",
        true,
    );
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        -3.5,
        "NoAI:1b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("effect give {piglin_uuid} minecraft:resistance 120 4"),
        true,
    );
    let protected_health = health(&world, piglin);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..25 {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {piglin_uuid} 100 minecraft:generic"),
            false,
        );
    }
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {leader_uuid} 1 minecraft:mob_attack by {piglin_uuid}"),
        true,
    );
    world.tick_rate_mut().set_frozen(false);
    let mut attempted = false;
    let mut ally_attacked = false;
    for _ in 0..100 {
        let output = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {piglin_uuid} 100 minecraft:generic"),
            false,
        );
        ally_attacked |= attack(&output.events, ally);
        assert!(
            !retreat_speed(&world, leader) && !retreat_speed(&world, ally),
            "equal adult numbers do not cause retreat"
        );
        if attack(&output.events, leader) {
            attempted = true;
            break;
        }
    }
    assert!(
        attempted && health(&world, piglin) == protected_health,
        "a targetable piglin takes no damage from the real hoglin attack attempt"
    );
    world.tick_rate_mut().set_frozen(true);
    mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        5.0,
        "NoAI:1b",
    );
    let threat = position(&world, piglin);
    world.tick_rate_mut().set_frozen(false);
    let mut leader_ran = false;
    let mut ally_ran = false;
    let mut leader_retreat_start = None;
    let mut ally_retreat_start = None;
    let mut ally_started_fleeing = false;
    for _ in 0..100 {
        let output = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {piglin_uuid} 100 minecraft:generic"),
            false,
        );
        if !ally_started_fleeing {
            ally_attacked |= attack(&output.events, ally);
        }
        ally_started_fleeing |= retreat_speed(&world, ally);
        if retreat_speed(&world, leader) {
            let start = *leader_retreat_start.get_or_insert(position(&world, leader));
            leader_ran |=
                separation(position(&world, leader), threat) > separation(start, threat) + 2.0;
        }
        if retreat_speed(&world, ally) {
            let start = *ally_retreat_start.get_or_insert(position(&world, ally));
            ally_ran |=
                separation(position(&world, ally), threat) > separation(start, threat) + 2.0;
        }
        if leader_ran && ally_ran {
            break;
        }
    }
    assert!(leader_ran && ally_ran && !ally_attacked,
        "the third visible adult triggers group retreat before the ally attacks: leader={:?}, ally={:?}, ally_attacked={ally_attacked}", position(&world, leader), position(&world, ally));
    world.tick_rate_mut().set_frozen(true);
    let close = position(&world, leader);
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        close.x + 1.3,
        close.z,
        "NoAI:1b",
    );
    let cow_health = health(&world, cow);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {leader_uuid} 1 minecraft:mob_attack by {cow_uuid}"),
        true,
    );
    world.tick_rate_mut().set_frozen(false);
    let mut fought_back = false;
    let mut leader_attacked = false;
    for _ in 0..100 {
        let output = tick(&sender, &receiver, &mut world);
        leader_attacked |= attack(&output.events, leader);
        if leader_attacked && health(&world, cow) < cow_health {
            fought_back = true;
            break;
        }
    }
    assert!(
        fought_back,
        "a non-piglin attacker restores actual fighting during group retreat"
    );
}
