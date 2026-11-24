//! What a mob's held weapon and mount change about its melee hit.

use super::melee::{fixture, hits, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};

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

/// Ticks until Alice dies and answers her death message.
fn death_message(world: &mut ServerWorld, limit: usize) -> String {
    for _ in 0..limit {
        let tick = world.tick(Vec::new());
        if let Some(message) = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::PlayerCombatKill(kill),
            } => Some(kill.message.text().to_string()),
            _ => None,
        }) {
            return message;
        }
    }
    panic!("Alice did not die within {limit} ticks");
}

/// `Mob.getAttackBoundingBox` (`Mob.java:1333-1352`) widens a rider's reach
/// to its mount's box: a zombie on a still horse hits a player beside the
/// horse that its own box could never reach.
#[test]
fn a_zombie_on_a_horse_reaches_across_the_horse() {
    let (sender, receiver, mut world) = fixture();
    let beside = Vec3d {
        x: PLAYER_POSITION.x + 1.75,
        ..PLAYER_POSITION
    };
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap();
    data.set_no_ai(true);
    let (_, horse_uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        beside,
        crate::entity::EntityData::GeneralLiving(data),
    );
    let (_, zombie_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, beside);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride {zombie_uuid} mount {horse_uuid}"),
    );
    let hits = hits(&mut world, 100);
    assert!(!hits.is_empty(), "the mounted zombie hits");
}

/// `Mob.doHurtTarget` hurts with the weapon's `damage_type`
/// (`Mob.java:1358`, `ItemStack.java:1126-1131`), which names the death.
#[test]
fn a_mob_weapons_damage_type_names_the_players_death() {
    let (sender, receiver, mut world) = fixture();
    let (_, zombie) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 3.0,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "item replace entity {zombie} weapon.mainhand with minecraft:stick[minecraft:damage_type=\"minecraft:spear\"]"
        ),
    );
    world.set_player_health_for_test(ALICE, 1.0);
    let message = death_message(&mut world, 200);
    assert!(message.contains("death.attack.spear"), "{message}");
}

/// `LivingEntity.stabAttack` (`LivingEntity.java:2837`) falls back to
/// `mob_attack` for a spear stripped of its `damage_type`.
#[test]
fn a_spear_without_a_damage_type_stabs_as_a_mob_attack() {
    let (sender, receiver, mut world) = fixture();
    let (_, zombie) = spawn(
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
            "item replace entity {zombie} weapon.mainhand with minecraft:iron_spear[!minecraft:damage_type]"
        ),
    );
    world.set_player_health_for_test(ALICE, 1.0);
    let message = death_message(&mut world, 300);
    assert!(message.contains("death.attack.mob"), "{message}");
}

/// A custom-named weapon picks the `.item` message (`DamageSource.java:80-83`).
#[test]
fn a_mobs_named_weapon_is_named_in_the_players_death() {
    let (sender, receiver, mut world) = fixture();
    let (_, zombie) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 3.0,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "item replace entity {zombie} weapon.mainhand with minecraft:stick[minecraft:custom_name=\"Excalibur\"]"
        ),
    );
    world.set_player_health_for_test(ALICE, 1.0);
    let message = death_message(&mut world, 200);
    assert!(message.contains("death.attack.mob.item"), "{message}");
    assert!(message.contains("Excalibur"), "{message}");
}
