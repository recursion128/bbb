//! Melee goals on live ticks against a survival session: the hit, its
//! knockback and cooldown, the spider's leap and the killer bunny.

use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::builtin_enum_component_values::VANILLA_RABBIT_VARIANT_EVIL_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_SPIDER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};

pub(super) const ALICE: ConnectionId = ConnectionId(1);
pub(super) const PLAYER_POSITION: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};

/// A survival session on a stone floor at midnight, on Normal so a mob's hit
/// lands its own `ATTACK_DAMAGE` (`Player.java:723-725`).
pub(super) fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.difficulty = Difficulty::Normal;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_player_position_for_test(ALICE, PLAYER_POSITION);
    for (id, command) in [(10, "time set midnight"), (11, "gamerule spawn_mobs false")] {
        let tick = console_command(&sender, &receiver, &mut world, id, command);
        assert!(
            console_command_answer(&tick).0.is_some(),
            "{command}: {:?}",
            console_command_answer(&tick)
        );
    }
    (sender, receiver, world)
}

pub(super) fn health(world: &ServerWorld) -> f32 {
    world.player_health_for_test(ALICE)
}

fn motions_to_alice(tick: &TickResult) -> Vec<Vec3d> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::SetEntityMotion(motion),
            } => Some(motion.delta_movement),
            _ => None,
        })
        .collect()
}

/// Runs `ticks` ticks and answers the ticks (1-based) on which Alice lost
/// health, with what she lost and the motion the hit sent her, which the
/// session's tracking frame publishes on the tick after the hurt lands.
pub(super) fn hits(world: &mut ServerWorld, ticks: usize) -> Vec<(usize, f32, Vec<Vec3d>)> {
    let mut hits: Vec<(usize, f32, Vec<Vec3d>)> = Vec::new();
    let mut last = health(world);
    for index in 1..=ticks + 1 {
        let tick = world.tick(Vec::new());
        if let Some(hit) = hits.last_mut().filter(|hit| hit.0 + 1 == index) {
            hit.2 = motions_to_alice(&tick);
        }
        let now = health(world);
        if now < last && index <= ticks {
            hits.push((index, last - now, Vec::new()));
        }
        last = now;
    }
    hits
}

/// `ZombieAttackGoal` walks a zombie up to the player it targets and hits it
/// for `ATTACK_DAMAGE` 3 every `attackInterval` of 20 ticks
/// (`MeleeAttackGoal.java:21`, `:135-137`), each hit knocking the player away
/// from the zombie (`LivingEntity.java:1228-1236`).
#[test]
fn a_zombie_walks_up_and_hits_a_survival_player_on_its_cooldown() {
    let (_sender, _receiver, mut world) = fixture();
    let (zombie, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 5.0,
            ..PLAYER_POSITION
        },
    );
    let hits = hits(&mut world, 200);
    assert!(hits.len() >= 3, "the zombie keeps hitting: {hits:?}");
    assert_eq!(hits[0].1, 3.0, "zombie ATTACK_DAMAGE on Normal");
    for (_, _, motions) in &hits {
        let motion = motions.first().expect("markHurt sends the knockback");
        assert!(
            motion.z < 0.0 && motion.y > 0.0,
            "pushed away from the zombie to the south: {motion:?}"
        );
    }
    // A goal restarted after its path ran out resets the cooldown
    // (`MeleeAttackGoal.java:72`), so only the hits of one run are spaced.
    assert!(
        hits[..4]
            .windows(2)
            .all(|pair| (20..=21).contains(&(pair[1].0 - pair[0].0))),
        "a zombie in reach hits on the attack interval: {hits:?}"
    );
    let position = world
        .stored_entity_for_test(DIMENSION, zombie)
        .unwrap()
        .position();
    assert!(
        position.z - PLAYER_POSITION.z < 2.0,
        "the zombie closed in: {position:?}"
    );
}

/// `Mob.doHurtTarget` ends in `postPiercingAttack` (`Mob.java:1374`), so a
/// Lunge weapon's `change_item_damage` wears the zombie's weapon one extra
/// point per swing.
#[test]
fn a_zombie_swing_runs_its_weapons_post_piercing_effects() {
    let mut worn = Vec::new();
    for weapon in [
        "minecraft:iron_sword",
        "minecraft:iron_sword[enchantments={\"minecraft:lunge\":1}]",
    ] {
        let (sender, receiver, mut world) = fixture();
        let (zombie, uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                z: PLAYER_POSITION.z + 1.5,
                ..PLAYER_POSITION
            },
        );
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            20,
            &format!("item replace entity {uuid} weapon.mainhand with {weapon}"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1), "{weapon}");
        let mut hit = false;
        for _ in 0..100 {
            let before = health(&world);
            world.tick(Vec::new());
            if health(&world) < before {
                hit = true;
                break;
            }
        }
        assert!(hit, "{weapon}");
        world.tick(Vec::new());
        worn.push(
            world
                .stored_entity_for_test(DIMENSION, zombie)
                .unwrap()
                .living()
                .unwrap()
                .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
                .unwrap()
                .summary()
                .component_patch
                .damage
                .unwrap_or(0),
        );
    }
    assert_eq!(worn[1], worn[0] + 1, "{worn:?}");
}

/// `LeapAtTargetGoal(0.4)` launches a spider off the ground at a target two to
/// four blocks away (`LeapAtTargetGoal.java:27-31`, `:40`).
#[test]
fn a_spider_leaps_at_the_player_it_targets() {
    let (_sender, _receiver, mut world) = fixture();
    let (spider, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SPIDER_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 3.5,
            ..PLAYER_POSITION
        },
    );
    tick_until(&mut world, 200, |world| {
        world
            .stored_entity_for_test(DIMENSION, spider)
            .unwrap()
            .position()
            .y
            > PLAYER_POSITION.y + 0.3
    });
    let hits = hits(&mut world, 100);
    assert!(
        hits.iter().any(|(_, lost, _)| *lost == 2.0),
        "the spider then bites for its ATTACK_DAMAGE 2: {hits:?}"
    );
}

/// `Rabbit.setVariant(EVIL)` gives the killer bunny `MeleeAttackGoal` and a
/// player target (`Rabbit.java:368-372`); its hit carries the variant's
/// `+5` attack modifier on the base 3 (`Rabbit.java:373-374`).
#[test]
fn a_killer_bunny_attacks_the_player() {
    let (sender, receiver, mut world) = fixture();
    let (_, rabbit_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 1.0,
            ..PLAYER_POSITION
        },
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        12,
        &format!("data merge entity {rabbit_uuid} {{RabbitType:{VANILLA_RABBIT_VARIANT_EVIL_ID}}}"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let hits = hits(&mut world, 200);
    assert!(
        hits.first().is_some_and(|(_, lost, _)| *lost == 8.0),
        "the killer bunny bites for 8: {hits:?}"
    );
}

/// A slime past its tiny size hurts a player it touches for its size
/// (`Slime.playerTouch`, `Slime.java:228-242`; `ATTACK_DAMAGE` is the size,
/// `:99`), and `SlimeAttackGoal` keeps it hopping at the player.
#[test]
fn a_slime_hurts_the_player_it_touches() {
    let (sender, receiver, mut world) = fixture();
    let (_, slime_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 2.5,
            ..PLAYER_POSITION
        },
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        12,
        &format!("data merge entity {slime_uuid} {{Size:1}}"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    let hits = hits(&mut world, 200);
    assert!(
        hits.first().is_some_and(|(_, lost, _)| *lost == 2.0),
        "a size-2 slime hits for 2: {hits:?}"
    );
}

/// `PhantomAttackPlayerTargetGoal` picks the player, `PhantomAttackStrategyGoal`
/// sends the phantom into a swoop and `PhantomSweepAttackGoal` bites on
/// contact (`Phantom.java:216-296`, `:490-505`). A size-0 phantom keeps the
/// monster `ATTACK_DAMAGE` 2: setting the size it already has never reaches
/// `updatePhantomSizeInfo` (`Phantom.java:83-90`).
#[test]
fn a_phantom_swoops_down_and_bites_the_player() {
    let (sender, receiver, mut world) = fixture();
    // `/summon` runs `finalizeSpawn`, which anchors the phantom.
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        12,
        &format!(
            "summon minecraft:phantom {} {} {}",
            PLAYER_POSITION.x,
            PLAYER_POSITION.y + 12.0,
            PLAYER_POSITION.z + 6.0
        ),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    tick_until(&mut world, 600, |world| health(world) < 20.0);
    assert_eq!(health(&world), 18.0, "the phantom bites for 2");
}

mod panda;
