//! The melee goals that charge or warn before the hit: the spear charge, the
//! polar bear rearing up, the vex dash and the bee sting.

use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::{
    VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, VANILLA_POLAR_BEAR_STANDING_DATA_ID,
    VANILLA_VEX_FLAGS_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_VEX_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    command: &str,
) {
    let tick = console_command(sender, receiver, world, 20, command);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{command}: {:?}",
        console_command_answer(&tick)
    );
}

/// The values of `data_id` the tick published for `entity_id`.
fn published(tick: &TickResult, entity_id: i32, data_id: u8) -> Vec<EntityDataValueKind> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::SetEntityData(update),
            } if update.id == entity_id => Some(update),
            _ => None,
        })
        .flat_map(|update| &update.values)
        .filter(|value| value.data_id == data_id)
        .map(|value| value.value.clone())
        .collect()
}

/// Ticks until Alice is hurt and answers what she lost, with whether
/// `seen` held for a value of `data_id` published on or before that tick.
fn first_hit_after(
    world: &mut ServerWorld,
    limit: usize,
    entity_id: i32,
    data_id: u8,
    seen: impl Fn(&EntityDataValueKind) -> bool,
) -> (f32, bool) {
    let mut published_seen = false;
    let before = health(world);
    for _ in 0..limit {
        let tick = world.tick(Vec::new());
        published_seen |= published(&tick, entity_id, data_id).iter().any(&seen);
        if health(world) < before {
            return (before - health(world), published_seen);
        }
    }
    panic!("Alice was not hurt within {limit} ticks");
}

/// `SpearUseGoal` takes over from `ZombieAttackGoal` once the zombie holds a
/// spear (`Zombie.java:119`): within ten blocks it raises the spear
/// (`startUsingItem`, the using bit of `DATA_LIVING_ENTITY_FLAGS`) and runs
/// at the player, and the charge stabs for the zombie's base
/// `ATTACK_DAMAGE` 3 plus its closing speed (`KineticWeapon.java:101-136`).
#[test]
fn a_zombie_with_a_spear_charges_and_stabs_the_player() {
    let (sender, receiver, mut world) = fixture();
    let (zombie, zombie_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 7.0,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("item replace entity {zombie_uuid} weapon.mainhand with minecraft:iron_spear"),
    );
    let (lost, raised) = first_hit_after(
        &mut world,
        300,
        zombie,
        VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        |value| matches!(value, EntityDataValueKind::Byte(flags) if flags & 1 != 0),
    );
    assert!(raised, "the zombie raised its spear before the hit");
    assert!(
        lost >= 3.0,
        "the charge stabs for at least the base 3: {lost}"
    );
    let flags = world
        .stored_entity_for_test(DIMENSION, zombie)
        .unwrap()
        .living()
        .unwrap()
        .living_entity_flags();
    assert_ne!(flags & 1, 0, "the spear stays raised through the charge");
}

/// A spear stripped of its `attack_range` charges with
/// `AttackRange.defaultFor` (`AttackRange.java:55-57`) instead of never
/// striking.
#[test]
fn a_zombie_spear_without_an_attack_range_still_stabs() {
    let (sender, receiver, mut world) = fixture();
    let (zombie, zombie_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 7.0,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "item replace entity {zombie_uuid} weapon.mainhand with minecraft:iron_spear[!minecraft:attack_range]"
        ),
    );
    let (lost, _) = first_hit_after(
        &mut world,
        300,
        zombie,
        VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
        |_| true,
    );
    assert!(lost > 0.0);
}

/// An adult polar bear with a cub nearby takes the player
/// (`PolarBear.java:253-266`), rears up (`DATA_STANDING_ID`) once the
/// player is within reach of a swipe and only then hits for its
/// `ATTACK_DAMAGE` 6 (`PolarBear.java:303-322`).
#[test]
fn a_polar_bear_stands_up_before_it_swipes_at_the_player() {
    let (sender, receiver, mut world) = fixture();
    let (bear, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 4.0,
            ..PLAYER_POSITION
        },
    );
    let (_, cub) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 6.0,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {cub} {{Age:-24000,NoAI:1b}}"),
    );
    let (lost, stood) = first_hit_after(
        &mut world,
        400,
        bear,
        VANILLA_POLAR_BEAR_STANDING_DATA_ID,
        |value| *value == EntityDataValueKind::Boolean(true),
    );
    assert!(stood, "the bear stood up before it hit");
    assert_eq!(lost, 6.0, "polar bear ATTACK_DAMAGE");
}

/// `VexChargeAttackGoal` flies the vex at the player's eyes with the
/// charging bit of `DATA_FLAGS_ID` set and hits on contact for its
/// `ATTACK_DAMAGE` 4 (`Vex.java:214-270`).
#[test]
fn a_vex_charges_the_player_and_hits_on_contact() {
    let (_sender, _receiver, mut world) = fixture();
    let (vex, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_VEX_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 5.0,
            y: PLAYER_POSITION.y + 1.0,
            ..PLAYER_POSITION
        },
    );
    let charging = |value: &EntityDataValueKind| matches!(value, EntityDataValueKind::Byte(flags) if flags & 1 != 0);
    let charged = (0..400).any(|_| {
        let tick = world.tick(Vec::new());
        published(&tick, vex, VANILLA_VEX_FLAGS_DATA_ID)
            .iter()
            .any(charging)
    });
    assert!(charged, "the vex charged before the hit");
    // Move control turns before travel, so retain the source of that turn.
    let source = world
        .stored_entity_for_test(DIMENSION, vex)
        .unwrap()
        .position();
    let player = world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ALICE)
        .unwrap()
        .position;
    let dx = player.x - source.x;
    let dz = player.z - source.z;
    let distance = dx.hypot(dz);
    assert!(
        distance > 2.0,
        "sample the charge before contact: {distance}"
    );
    world
        .stored_entity_mut_for_test(DIMENSION, vex)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: dz / distance * 0.6,
            y: 0.0,
            z: -dx / distance * 0.6,
        });
    world.tick(Vec::new());
    let entity = world.stored_entity_for_test(DIMENSION, vex).unwrap();
    let expected_yaw = -dx.atan2(dz).to_degrees();
    let yaw_error = (f64::from(entity.y_rot()) - expected_yaw + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        yaw_error.abs() < 1.0,
        "the charging vex faces Alice despite its sideways motion: {} versus {expected_yaw}",
        entity.y_rot()
    );
    let movement = entity.delta_movement();
    let motion_yaw = -movement.x.atan2(movement.z).to_degrees();
    let motion_error = (motion_yaw - expected_yaw + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        motion_error.abs() > 30.0,
        "motion differs from target bearing"
    );
    let (lost, _) = first_hit_after(&mut world, 400, vex, VANILLA_VEX_FLAGS_DATA_ID, charging);
    assert_eq!(lost, 4.0, "vex ATTACK_DAMAGE");
}

/// A bee the player hit stings back (`Bee.java:226-248`): poison for 10
/// seconds on Normal, `HasStung` set, and the stung-out bee dies within
/// 1200 ticks (`Bee.java:376-381`).
#[test]
fn an_angry_bee_stings_poisons_the_player_and_then_dies() {
    let (sender, receiver, mut world) = fixture();
    let (bee, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_BEE_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 2.0,
            ..PLAYER_POSITION
        },
    );
    world.tick(Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bee }),
    );
    let before = health(&world);
    tick_until(&mut world, 400, |world| health(world) < before);
    assert_eq!(before - health(&world), 2.0, "bee ATTACK_DAMAGE");
    world.tick(Vec::new());
    let poison = world
        .player_active_effect_for_test(ALICE, "minecraft:poison")
        .expect("the sting poisons on Normal");
    assert!(
        (190..=200).contains(&poison.duration()),
        "ten seconds of poison: {}",
        poison.duration()
    );
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, bee)
    else {
        panic!("the bee is still there");
    };
    assert!(fields
        .iter()
        .any(|entry| entry.name == "HasStung" && entry.value == NbtSummaryValue::Byte(1)));
    // The bee outlives the session's keep-alive window, so Alice answers.
    for _ in 0..1300 {
        let tick = world.tick(Vec::new());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                play(
                    &sender,
                    &receiver,
                    &mut world,
                    1,
                    PlayServerbound::KeepAlive { id: *id },
                );
            }
        }
        if world
            .stored_entity_for_test(DIMENSION, bee)
            .is_none_or(|bee| bee.living().is_some_and(|living| living.health() <= 0.0))
        {
            return;
        }
    }
    panic!("the stung bee lived past its 1200 ticks");
}
