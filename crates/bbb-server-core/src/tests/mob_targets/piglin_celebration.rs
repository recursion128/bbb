//! A real target's external death refreshes hunting cooldown and starts an interruptible celebration.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_synched_data::VANILLA_PIGLIN_DANCING_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

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
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}
fn looks_at(body: &crate::entity::Entity, target: &crate::entity::Entity) -> bool {
    let from = body.position();
    let to = target.position();
    let yaw = (to.z - from.z).atan2(to.x - from.x).to_degrees() as f32 - 90.0;
    let error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (from.y + f64::from(body.eye_height()) - to.y - f64::from(target.eye_height()))
        .atan2(separation(from, to))
        .to_degrees() as f32;
    error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
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
fn hunt_ttl(world: &ServerWorld, id: i32) -> i64 {
    let saved = memory(world, id, "minecraft:hunted_recently")
        .expect("canonical hunting cooldown is saved");
    assert_eq!(field(&saved, "value"), Some(&NbtSummaryValue::Byte(1)));
    match field(&saved, "ttl") {
        Some(NbtSummaryValue::Long(ttl)) => *ttl,
        _ => panic!("{saved:?}"),
    }
}
fn dancing(output: &TickResult, id: i32, value: bool) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|field| field.data_id == VANILLA_PIGLIN_DANCING_DATA_ID && field.value == EntityDataValueKind::Boolean(value))))
}
fn celebration_sound(output: &TickResult) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::Sound(sound), .. }
        if matches!(sound.sound, SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.piglin.celebrate"))))
}

#[test]
fn a_piglin_celebrates_its_externally_killed_target_refreshes_hunting_and_fights_after_hurt() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (piglin, uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        0.5,
        8.5,
        "NoAI:0b",
    );
    let (hoglin, hoglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        12.5,
        8.5,
        "NoAI:1b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("item replace entity {uuid} weapon.mainhand with minecraft:golden_sword"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:attack_damage base set 1"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:mob_attack by {hoglin_uuid}"),
    );
    let initial = position(&world, piglin);
    let hoglin_health = health(&world, hoglin);
    world.tick_rate_mut().set_frozen(false);
    let mut pursued = false;
    for _ in 0..120 {
        tick(&sender, &receiver, &mut world);
        if position(&world, piglin).x > initial.x + 1.0 {
            pursued = true;
            break;
        }
    }
    assert!(
        pursued,
        "the piglin pursues its actual living hoglin target"
    );
    assert_eq!(health(&world, hoglin), hoglin_health);
    let corpse_at = position(&world, hoglin);
    assert!(
        separation(position(&world, piglin), corpse_at) >= 7.0,
        "death must leave room for either celebration reach"
    );
    world.tick_rate_mut().set_frozen(true);
    command(&sender, &receiver, &mut world, &format!("data merge entity {uuid} {{Brain:{{memories:{{\"minecraft:hunted_recently\":{{value:1b,ttl:40L}}}}}}}}"));
    assert_eq!(hunt_ttl(&world, piglin), 40);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("kill {hoglin_uuid}"),
    );
    let died_at = world.game_time();
    let before_celebrating = position(&world, piglin);
    let piglin_health = health(&world, piglin);
    world.tick_rate_mut().set_frozen(false);
    let mut refreshed = false;
    let mut heard = false;
    let mut moved = false;
    let mut danced = false;
    for _ in 0..120 {
        let output = tick(&sender, &receiver, &mut world);
        heard |= celebration_sound(&output);
        danced |= dancing(&output, piglin, true);
        refreshed |= (500..=2400).contains(&hunt_ttl(&world, piglin));
        assert_eq!(health(&world, piglin), piglin_health);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        let living = body.living().unwrap();
        let base = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        let speed = f64::from(living.speed());
        moved |= ((speed - base).abs() < 1.0e-5 || (speed - 0.6 * base).abs() < 1.0e-5)
            && separation(body.position(), corpse_at)
                < separation(before_celebrating, corpse_at) - 2.0;
        if heard && refreshed && moved {
            break;
        }
    }
    assert!(
        heard && refreshed && moved,
        "external target death refreshes cooldown and starts audible celebration movement"
    );
    assert!(memory(&world, piglin, "minecraft:angry_at").is_none());
    world.tick_rate_mut().set_frozen(true);
    let before_hurt = position(&world, piglin);
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        before_hurt.x,
        before_hurt.z + 4.0,
        "NoAI:1b",
    );
    let cow_health = health(&world, cow);
    assert!(
        world.game_time() - died_at < 300,
        "hurt interrupts a still-live celebration"
    );
    let hurt = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 2 minecraft:mob_attack by {cow_uuid}"),
    );
    let mut dance_cleared = dancing(&hurt, piglin, false);
    world.tick_rate_mut().set_frozen(false);
    let mut swung = false;
    let mut hit = false;
    for _ in 0..180 {
        let output = tick(&sender, &receiver, &mut world);
        dance_cleared |= dancing(&output, piglin, false);
        swung |= output.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::EntityAnimation(animation), .. }
            if animation.id == piglin && animation.action == 0));
        if swung && health(&world, cow) < cow_health {
            hit = true;
            break;
        }
    }
    assert!(
        hit && separation(position(&world, piglin), before_hurt) > 1.0,
        "real hurt interrupts celebration and the piglin approaches and damages its cow attacker"
    );
    assert!(
        !danced || dance_cleared,
        "a naturally observed dance is cleared in observer metadata after hurt"
    );
    world.tick_rate_mut().set_frozen(true);
    let cow_corpse = position(&world, cow);
    let killed = command(&sender, &receiver, &mut world, &format!("kill {cow_uuid}"));
    let cow_died_at = world.game_time();
    let (peer, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        cow_corpse.x + 5.0,
        cow_corpse.z,
        "NoAI:1b,CannotHunt:1b",
    );
    let peer_health = health(&world, peer);
    let after_hurt_health = health(&world, piglin);
    let mut heard_again = celebration_sound(&killed);
    let mut baseline = None;
    let mut quiet = 0;
    let mut looked = 0;
    let mut strolled = false;
    world.tick_rate_mut().set_frozen(false);
    while world.game_time() - cow_died_at < 280 {
        let output = tick(&sender, &receiver, &mut world);
        heard_again |= celebration_sound(&output);
        assert!(
            !dancing(&output, piglin, true),
            "a cow death does not start dancing"
        );
        assert_eq!(health(&world, piglin), after_hurt_health);
        assert_eq!(health(&world, peer), peer_health);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        let target = world.stored_entity_for_test(DIMENSION, peer).unwrap();
        looked += usize::from(looks_at(body, target));
        let delta = body.delta_movement();
        if let Some(base_pos) = baseline {
            let living = body.living().unwrap();
            let base = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
            strolled |= body.on_ground()
                && (f64::from(living.speed()) - 0.6 * base).abs() < 1.0e-5
                && separation(body.position(), base_pos) > 0.5;
        } else {
            let at = body.block_position();
            let dx = f64::from(at.x) - cow_corpse.x.floor();
            let dz = f64::from(at.z) - cow_corpse.z.floor();
            quiet = if body.on_ground() && delta.x.hypot(delta.z) < 0.01 && dx * dx + dz * dz < 4.0
            {
                quiet + 1
            } else {
                0
            };
            if quiet >= 3 {
                baseline = Some(body.position());
            }
        }
        if heard_again && looked >= 4 && strolled {
            break;
        }
    }
    assert!(heard_again && looked >= 4 && strolled,
        "second celebration peer look and fresh local stroll: heard={heard_again}, looked={looked}, baseline={baseline:?}, strolled={strolled}, at={:?}", position(&world, piglin));
    assert!(memory(&world, piglin, "minecraft:angry_at").is_none());
}
