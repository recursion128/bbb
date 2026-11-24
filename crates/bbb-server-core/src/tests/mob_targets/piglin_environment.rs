//! Passive piglin flight from zombified mobs, baby nemeses and lit repellents.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
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
fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}
fn assert_cow_anger(world: &ServerWorld, id: i32, cow: Uuid) {
    let saved = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap();
    let anger = field(
        field(
            field(field(&saved, "Brain").unwrap(), "memories").unwrap(),
            "minecraft:angry_at",
        )
        .expect("the real cow-caused hurt established anger"),
        "value",
    )
    .unwrap();
    let bits = cow.as_u128();
    assert_eq!(
        anger,
        &NbtSummaryValue::IntArray(vec![
            (bits >> 96) as i32,
            (bits >> 64) as i32,
            (bits >> 32) as i32,
            bits as i32
        ])
    );
}

#[test]
fn a_near_zoglin_suppresses_cow_anger_and_its_removal_restores_actual_attacks() {
    let (sender, receiver, mut world) = arena();
    let (piglin, uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b,CannotHunt:1b",
    );
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        -1.5,
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
        &format!("damage {uuid} 1 minecraft:mob_attack by {cow_uuid}"),
    );
    assert_cow_anger(&world, piglin, cow_uuid);
    let (zoglin, zoglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        6.5,
        8.5,
        "NoAI:1b",
    );
    let initial = position(&world, piglin);
    let piglin_health = health(&world, piglin);
    let cow_health = health(&world, cow);
    world.tick_rate_mut().set_frozen(false);
    let mut fled = false;
    for _ in 0..120 {
        tick(&sender, &receiver, &mut world);
        let now = position(&world, piglin);
        assert_eq!(health(&world, piglin), piglin_health);
        if separation(now, position(&world, zoglin)) < 6.0 {
            assert_eq!(health(&world, cow), cow_health);
            if fleeing_speed(&world, piglin) && now.x > initial.x + 1.0 {
                fled = true;
                break;
            }
        }
    }
    assert!(
        fled,
        "the angry piglin really moves away from the nearby zoglin and its cow target"
    );
    assert_cow_anger(&world, piglin, cow_uuid);
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("kill {zoglin_uuid}"),
    );
    let before = position(&world, piglin);
    world.tick_rate_mut().set_frozen(false);
    let mut hit = false;
    for _ in 0..300 {
        tick(&sender, &receiver, &mut world);
        if health(&world, cow) < cow_health {
            hit = true;
            break;
        }
    }
    assert!(hit && position(&world, piglin).x < before.x - 3.0,
        "removing the zoglin restores pursuit and damage without a second provocation: piglin={:?}, cow_health={}", position(&world, piglin), health(&world, cow));
}

#[test]
fn a_baby_piglin_flees_a_fixed_wither_skeleton_without_any_hurt() {
    let (sender, receiver, mut world) = arena();
    let (baby, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b,IsBaby:1b,CannotHunt:1b",
    );
    let (skeleton, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
        12.5,
        8.5,
        "NoAI:1b",
    );
    let baby_health = health(&world, baby);
    let skeleton_health = health(&world, skeleton);
    let initial = position(&world, baby);
    let initial_distance = separation(initial, position(&world, skeleton));
    world.tick_rate_mut().set_frozen(false);
    let mut fled = false;
    for _ in 0..240 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(health(&world, baby), baby_health);
        assert_eq!(health(&world, skeleton), skeleton_health);
        let body = world.stored_entity_for_test(DIMENSION, baby).unwrap();
        if fleeing_speed(&world, baby)
            && body.on_ground()
            && separation(body.position(), position(&world, skeleton)) > initial_distance + 3.0
        {
            fled = true;
            break;
        }
    }
    assert!(
        fled,
        "the unharmed baby actively escapes its visible nemesis: baby={:?}, skeleton={:?}",
        position(&world, baby),
        position(&world, skeleton)
    );
}

#[test]
fn an_unlit_soul_campfire_only_repels_a_piglin_after_it_is_lit() {
    let (sender, receiver, mut world) = arena();
    let (piglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b,CannotHunt:1b",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "setblock 11 65 8 minecraft:soul_campfire[lit=false]",
    );
    let initial = position(&world, piglin);
    let initial_health = health(&world, piglin);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..100 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(health(&world, piglin), initial_health);
        assert!(
            !(fleeing_speed(&world, piglin) && position(&world, piglin).x < initial.x - 1.0),
            "an unlit soul campfire does not cause directed flight"
        );
    }
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "setblock 11 65 8 minecraft:soul_campfire[lit=true]",
    );
    let before_lit = position(&world, piglin);
    let campfire = Vec3d {
        x: 11.5,
        y: 65.0,
        z: 8.5,
    };
    world.tick_rate_mut().set_frozen(false);
    let mut walked = false;
    let mut escaped = false;
    for _ in 0..300 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(health(&world, piglin), initial_health);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        walked |= fleeing_speed(&world, piglin)
            && separation(body.position(), campfire) > separation(before_lit, campfire) + 3.0;
        if walked && body.on_ground() && separation(body.position(), campfire) >= 8.0 {
            escaped = true;
            break;
        }
    }
    assert!(
        escaped,
        "lighting the tagged campfire causes real speed1 movement to its eight-block separation"
    );
}
