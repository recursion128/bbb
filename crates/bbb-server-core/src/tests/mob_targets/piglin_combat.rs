//! Piglin anger reinforcement and qualified hoglin hunting through live ticks.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
};

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
fn assert_anger(world: &ServerWorld, id: i32, target: Uuid) {
    let saved =
        memory(world, id, "minecraft:angry_at").expect("anger is visible in saved Brain NBT");
    let bits = target.as_u128();
    assert_eq!(
        field(&saved, "value"),
        Some(&NbtSummaryValue::IntArray(vec![
            (bits >> 96) as i32,
            (bits >> 64) as i32,
            (bits >> 32) as i32,
            bits as i32
        ]))
    );
}
fn assert_hunt_cooldown(world: &ServerWorld, id: i32) {
    let saved = memory(world, id, "minecraft:hunted_recently")
        .expect("the visible cohort gets a saved hunting cooldown");
    assert_eq!(field(&saved, "value"), Some(&NbtSummaryValue::Byte(1)));
    assert!(
        matches!(field(&saved, "ttl"), Some(NbtSummaryValue::Long(ttl)) if (1..=2400).contains(ttl)),
        "{saved:?}"
    );
}
fn clear_hunt_cooldown(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    uuid: Uuid,
) {
    command(sender, receiver, world, &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:hunted_recently\":{{value:1b,ttl:9999L}}}}}}}}"));
    command(
        sender,
        receiver,
        world,
        &format!("data remove entity {uuid} Brain.memories.\"minecraft:hunted_recently\""),
    );
}

#[test]
fn a_hurt_no_ai_piglin_alerts_a_brute_and_an_adult_behind_a_wall() {
    let (sender, receiver, mut world) = arena();
    let (source, source_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b",
    );
    let (brute, brute_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        11.5,
        15.5,
        "NoAI:0b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("item replace entity {brute_uuid} weapon.mainhand with minecraft:golden_axe"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {brute_uuid} minecraft:attack_damage base set 1"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "fill -16 65 0 32 69 0 minecraft:stone",
    );
    let (hidden, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        -3.5,
        "NoAI:1b",
    );
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        3.0,
        8.5,
        "NoAI:1b",
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..25 {
        tick(&sender, &receiver, &mut world);
    }
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {source_uuid} {{NoAI:1b}}"),
    );
    let before = position(&world, brute);
    let cow_health = health(&world, cow);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {source_uuid} 1 minecraft:mob_attack by {cow_uuid}"),
    );
    assert_anger(&world, source, cow_uuid);
    assert_anger(&world, hidden, cow_uuid);
    world.tick_rate_mut().set_frozen(false);
    let mut hit = false;
    for _ in 0..240 {
        tick(&sender, &receiver, &mut world);
        if health(&world, cow) < cow_health {
            hit = true;
            break;
        }
    }
    assert!(
        hit && position(&world, brute).x < before.x - 3.0,
        "the brute ally approaches and hurts the fixed cow: brute={:?}, source={:?}, cow_health={}",
        position(&world, brute),
        position(&world, source),
        health(&world, cow)
    );
    world.tick_rate_mut().set_frozen(true);
    command(&sender, &receiver, &mut world, &format!("kill {cow_uuid}"));
    let before_nemesis = position(&world, brute);
    let (skeleton, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
        before_nemesis.x,
        before_nemesis.z + 6.0,
        "NoAI:1b",
    );
    let skeleton_health = health(&world, skeleton);
    world.tick_rate_mut().set_frozen(false);
    let mut hit_nemesis = false;
    for _ in 0..300 {
        tick(&sender, &receiver, &mut world);
        if health(&world, skeleton) < skeleton_health {
            hit_nemesis = true;
            break;
        }
    }
    let after_nemesis = position(&world, brute);
    assert!(
        hit_nemesis && (after_nemesis.x - before_nemesis.x).hypot(after_nemesis.z - before_nemesis.z) > 1.0,
        "after its anger target dies the brute approaches and attacks a nemesis: brute={after_nemesis:?}, skeleton_health={}",
        health(&world, skeleton),
    );
}

#[test]
fn piglin_hunting_respects_flags_and_visible_peer_cooldown_then_starts_a_group_hunt() {
    let (sender, receiver, mut world) = arena();
    let (hunter, hunter_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        5.5,
        8.5,
        "NoAI:0b,CannotHunt:1b",
    );
    let (peer, peer_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        5.5,
        13.5,
        "NoAI:1b",
    );
    let (brute, brute_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        8.5,
        15.5,
        "NoAI:1b",
    );
    let (hoglin, hoglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        12.5,
        8.5,
        "NoAI:1b,CannotBeHunted:0b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("item replace entity {hunter_uuid} weapon.mainhand with minecraft:golden_sword"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {hunter_uuid} minecraft:attack_damage base set 1"),
    );
    for uuid in [hunter_uuid, peer_uuid, brute_uuid] {
        clear_hunt_cooldown(&sender, &receiver, &mut world, uuid);
    }
    let initial_health = health(&world, hoglin);
    for gate in 0..3 {
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..120 {
            tick(&sender, &receiver, &mut world);
            assert_eq!(
                health(&world, hoglin),
                initial_health,
                "hunting gate {gate} protects the hoglin"
            );
        }
        world.tick_rate_mut().set_frozen(true);
        if gate == 0 {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {hunter_uuid} {{CannotHunt:0b}}"),
            );
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {hoglin_uuid} {{CannotBeHunted:1b}}"),
            );
        } else if gate == 1 {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {hoglin_uuid} {{CannotBeHunted:0b}}"),
            );
            command(&sender, &receiver, &mut world, &format!("data merge entity {peer_uuid} {{Brain:{{memories:{{\"minecraft:hunted_recently\":{{value:1b,ttl:1200L}}}}}}}}"));
        }
    }
    clear_hunt_cooldown(&sender, &receiver, &mut world, peer_uuid);
    let initial = position(&world, hunter);
    world.tick_rate_mut().set_frozen(false);
    let mut hit = false;
    for _ in 0..300 {
        tick(&sender, &receiver, &mut world);
        if health(&world, hoglin) < initial_health {
            hit = true;
            break;
        }
    }
    assert!(
        hit && position(&world, hunter).x > initial.x + 1.0,
        "an eligible adult initiates hunting and hurts the hoglin: hunter={:?}, hoglin_health={}",
        position(&world, hunter),
        health(&world, hoglin)
    );
    for id in [hunter, peer, brute] {
        assert_hunt_cooldown(&world, id);
    }
    assert_anger(&world, hunter, hoglin_uuid);
    assert_anger(&world, peer, hoglin_uuid);
    assert!(
        memory(&world, brute, "minecraft:angry_at").is_none(),
        "a brute shares hunting cooldown without joining the hoglin hunt"
    );
}
