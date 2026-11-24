//! What a landed melee hit leaves on the player: the cave spider's poison,
//! the wither skeleton's wither, the husk's hunger, a burning zombie's fire,
//! a bee's stinger and a spear charge that only shoves and unseats.

use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::entity::{BoatData, EntityData};
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
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

fn near(dx: f64) -> Vec3d {
    Vec3d {
        x: PLAYER_POSITION.x + dx,
        ..PLAYER_POSITION
    }
}

/// Ticks until the first hit lands on Alice and one more, so the effect the
/// hit queued has reached her.
fn first_hit(world: &mut ServerWorld) {
    let before = health(world);
    tick_until(world, 400, |world| health(world) < before);
    world.tick(Vec::new());
}

#[test]
fn hoglins_and_zoglins_approach_and_attack_players_with_age_specific_damage_and_knockback() {
    for kind in [VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID] {
        for (baby, resistance) in [(false, 0.0), (false, 1.0), (true, 0.0)] {
            let (sender, receiver, mut world) = fixture();
            let knockback = if resistance == 0.0 { 5.0 } else { 0.6 };
            let start = near(5.0);
            let (source, uuid) = spawn(&mut world, kind, start);
            let age = if kind == VANILLA_ENTITY_TYPE_HOGLIN_ID {
                format!(
                    "Age:{},IsImmuneToZombification:1b",
                    if baby { -24000 } else { 0 }
                )
            } else {
                format!("IsBaby:{}b", i32::from(baby))
            };
            for (index, line) in [
                format!("data merge entity {uuid} {{NoAI:1b,{age}}}"),
                format!("attribute {uuid} minecraft:attack_knockback base set {knockback}"),
                format!("attribute alice minecraft:knockback_resistance base set {resistance}"),
                "team add no_collision".to_owned(),
                "team modify no_collision collisionRule never".to_owned(),
                format!("team join no_collision {uuid}"),
            ]
            .iter()
            .enumerate()
            {
                let tick =
                    console_command(&sender, &receiver, &mut world, 100 + index as u64, line);
                assert!(
                    console_command_answer(&tick).0.is_some(),
                    "{line}: {:?}",
                    console_command_answer(&tick)
                );
            }
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::MovePlayer(MovePlayer {
                    position: None,
                    rotation: None,
                    on_ground: false,
                    horizontal_collision: false,
                }),
            );
            for (id, line) in [
                (200, format!("data merge entity {uuid} {{NoAI:0b}}")),
                (
                    201,
                    format!("attribute {uuid} minecraft:attack_damage base set 1"),
                ),
            ] {
                let tick = console_command(&sender, &receiver, &mut world, id, &line);
                assert!(console_command_answer(&tick).0.is_some());
            }
            let hits = super::melee::hits(&mut world, 180);
            assert!(hits.len() >= 2, "{kind}: {hits:?}");
            assert_eq!(hits[0].1, if baby { 1.0 } else { 0.5 });
            let damage_within = |ticks| {
                hits.iter()
                    .take_while(|(tick, _, _)| *tick <= hits[0].0 + ticks)
                    .map(|(_, damage, _)| damage)
                    .sum::<f32>()
            };
            assert_eq!(
                damage_within(35),
                if baby { 3.0 } else { 0.5 },
                "{kind}, baby={baby}: {hits:?}"
            );
            if !baby {
                assert_eq!(damage_within(85), 1.5, "{kind}: {hits:?}");
            }
            assert!(
                world
                    .stored_entity_for_test(DIMENSION, source)
                    .unwrap()
                    .position()
                    .x
                    < start.x - 1.0
            );
            for (_, _, motions) in &hits[..2] {
                let motion = motions
                    .first()
                    .expect("the player receives the hit impulse");
                if resistance >= 0.6 {
                    assert_eq!(*motion, Vec3d::default());
                } else if baby {
                    assert!(
                        (motion.x.hypot(motion.z) - 0.4).abs() < 1e-6,
                        "{kind}: {motion:?}"
                    );
                    assert!((0.0..=0.4 + 1e-6).contains(&motion.y), "{kind}: {motion:?}");
                } else {
                    let horizontal = motion.x.hypot(motion.z);
                    assert!(
                        (0.6 - 1e-6..3.9 + 1e-6).contains(&horizontal),
                        "{kind}: {motion:?}"
                    );
                    assert!((0.0..=2.5).contains(&motion.y), "{kind}: {motion:?}");
                }
            }
        }
    }
}

#[test]
fn zoglins_pass_over_creepers_and_each_other_to_attack_and_throw_animals() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREEPER_ID};

    for baby in [false, true] {
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        let start = near(-4.0);
        let (cow, cow_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID, near(4.0));
        let (creeper, creeper_uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_CREEPER_ID,
            Vec3d {
                z: start.z + 3.0,
                ..start
            },
        );
        let (other, other_uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            Vec3d {
                z: start.z - 3.0,
                ..start
            },
        );
        let (source, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOGLIN_ID, start);
        for (index, line) in [
            "gamemode spectator alice".to_owned(),
            format!(
                "data merge entity {uuid} {{NoAI:1b,IsBaby:{}b}}",
                i32::from(baby)
            ),
            format!("data merge entity {cow_uuid} {{NoAI:1b}}"),
            format!("data merge entity {creeper_uuid} {{NoAI:1b}}"),
            format!("data merge entity {other_uuid} {{NoAI:1b}}"),
            format!("attribute {uuid} minecraft:attack_knockback base set 5"),
            "team add no_collision".to_owned(),
            "team modify no_collision collisionRule never".to_owned(),
            format!("team join no_collision {uuid}"),
        ]
        .iter()
        .enumerate()
        {
            let tick = console_command(&sender, &receiver, &mut world, 100 + index as u64, line);
            assert!(
                console_command_answer(&tick).0.is_some(),
                "{line}: {:?}",
                console_command_answer(&tick)
            );
        }
        let health = |world: &ServerWorld, id| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .health()
        };
        let before = health(&world, cow);
        let excluded = [
            (creeper, health(&world, creeper)),
            (other, health(&world, other)),
        ];
        for (id, line) in [
            (200, format!("data merge entity {uuid} {{NoAI:0b}}")),
            (
                201,
                format!("attribute {uuid} minecraft:attack_damage base set 1"),
            ),
        ] {
            let tick = console_command(&sender, &receiver, &mut world, id, &line);
            assert!(console_command_answer(&tick).0.is_some());
        }
        world.tick_rate_mut().set_frozen(false);
        tick_until(&mut world, 200, |world| health(world, cow) < before);
        assert_eq!(before - health(&world, cow), if baby { 1.0 } else { 0.5 });
        for (id, before) in excluded {
            assert_eq!(health(&world, id), before);
        }
        assert!(
            world
                .stored_entity_for_test(DIMENSION, source)
                .unwrap()
                .position()
                .x
                > start.x + 1.0
        );
        let motion = world
            .stored_entity_for_test(DIMENSION, cow)
            .unwrap()
            .delta_movement();
        let horizontal = motion.x.hypot(motion.z);
        if baby {
            assert!((horizontal - 0.4).abs() < 1e-6, "{motion:?}");
        } else {
            assert!((0.6 - 1e-6..3.9 + 1e-6).contains(&horizontal), "{motion:?}");
        }
    }
}

#[test]
fn zoglins_retaliate_against_ignored_attackers_then_resume_hunting() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREEPER_ID};

    for (attacker_type, switch) in [
        (VANILLA_ENTITY_TYPE_CREEPER_ID, true),
        (VANILLA_ENTITY_TYPE_ZOGLIN_ID, true),
        (VANILLA_ENTITY_TYPE_CREEPER_ID, false),
    ] {
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        let start = near(-4.0);
        let (cow, cow_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID, near(4.0));
        let (attacker, attacker_uuid) = spawn(&mut world, attacker_type, near(0.0));
        let (source, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOGLIN_ID, start);
        for (index, line) in [
            "gamemode spectator alice".to_owned(),
            format!("data merge entity {cow_uuid} {{NoAI:1b}}"),
            format!("data merge entity {attacker_uuid} {{NoAI:1b}}"),
            format!("attribute {uuid} minecraft:attack_damage base set 1"),
            format!("attribute {uuid} minecraft:attack_knockback base set 0"),
            "team add no_collision".to_owned(),
            "team modify no_collision collisionRule never".to_owned(),
            format!("team join no_collision {uuid}"),
        ]
        .iter()
        .enumerate()
        {
            let tick = console_command(&sender, &receiver, &mut world, 100 + index as u64, line);
            assert!(console_command_answer(&tick).0.is_some(), "{line}");
        }
        let health = |world: &ServerWorld, id| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .health()
        };
        let initial_cow_health = health(&world, cow);
        world.tick_rate_mut().set_frozen(false);
        tick_until(&mut world, 200, |world| {
            health(world, cow) < initial_cow_health
        });
        let cow_before = health(&world, cow);
        world.tick_rate_mut().set_frozen(true);
        for (id, line) in [
            (200, format!("tp {uuid} {} 65 8.5", start.x)),
            (
                201,
                format!(
                    "tp {attacker_uuid} {} 65 8.5",
                    if switch { 8.5 } else { 15.0 }
                ),
            ),
            (202, format!("tp {cow_uuid} 12.5 65 8.5")),
        ] {
            let tick = console_command(&sender, &receiver, &mut world, id, &line);
            assert!(console_command_answer(&tick).0.is_some(), "{line}");
        }
        let npc_melee = attacker_type == VANILLA_ENTITY_TYPE_ZOGLIN_ID;
        let commands = if npc_melee {
            vec![
                format!("data merge entity {attacker_uuid} {{NoAI:0b}}"),
                format!("attribute {attacker_uuid} minecraft:attack_damage base set 1"),
                format!("attribute {attacker_uuid} minecraft:attack_knockback base set 0"),
                format!("damage {attacker_uuid} 1 minecraft:mob_attack by {uuid}"),
            ]
        } else {
            vec![format!(
                "damage {uuid} 1 minecraft:mob_attack by {attacker_uuid}"
            )]
        };
        for (index, line) in commands.iter().enumerate() {
            let tick = console_command(&sender, &receiver, &mut world, 203 + index as u64, line);
            assert!(console_command_answer(&tick).0.is_some(), "{line}");
        }
        let attacker_before = health(&world, attacker);
        let source_before = health(&world, source);
        world.tick_rate_mut().set_frozen(false);
        let mut after_expiry = attacker_before;
        let mut attacker_stopped = false;
        for step in 0..340 {
            let tick = world.tick(receiver.drain());
            for event in &tick.events {
                if let WorldEvent::Send {
                    connection_id: ALICE,
                    packet: ClientboundPlayPacket::KeepAlive { id },
                } = event
                {
                    enqueue(
                        &sender,
                        WorldCommand::Play {
                            connection_id: ALICE,
                            packet: PlayServerbound::KeepAlive { id: *id },
                        },
                    )
                    .unwrap();
                }
            }
            if npc_melee && !attacker_stopped && health(&world, source) < source_before {
                world.tick_rate_mut().set_frozen(true);
                let line = format!("data merge entity {attacker_uuid} {{NoAI:1b}}");
                let tick = console_command(&sender, &receiver, &mut world, 300, &line);
                assert!(console_command_answer(&tick).0.is_some());
                world.tick_rate_mut().set_frozen(false);
                attacker_stopped = true;
            }
            if step == 179 && switch {
                assert!(
                    health(&world, attacker) < attacker_before,
                    "type {attacker_type}"
                );
                assert_eq!(health(&world, cow), cow_before);
            }
            if step == 259 {
                after_expiry = health(&world, attacker);
            }
        }
        assert!(
            !npc_melee || attacker_stopped,
            "the other zoglin landed a real melee hit"
        );
        assert_eq!(health(&world, attacker), after_expiry);
        if !switch {
            assert_eq!(health(&world, attacker), attacker_before);
        }
        assert!(
            health(&world, cow) < cow_before,
            "type {attacker_type}, switch {switch}"
        );
    }
}

fn mob_health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

#[test]
fn adult_hoglins_interrupt_breeding_to_retaliate_against_a_landed_npc_hit() {
    for breeding in [false, true] {
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        let (source, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_HOGLIN_ID, near(0.0));
        let (partner, partner_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_HOGLIN_ID, near(7.0));
        let (attacker, attacker_uuid) =
            spawn(&mut world, VANILLA_ENTITY_TYPE_ZOGLIN_ID, near(-1.5));
        for line in [
            "gamemode spectator alice".to_owned(),
            format!("data merge entity {uuid} {{IsImmuneToZombification:1b,InLove:{}}}", if breeding { 600 } else { 0 }),
            format!("data merge entity {partner_uuid} {{NoAI:1b,IsImmuneToZombification:1b,InLove:600}}"),
            format!("data merge entity {attacker_uuid} {{NoAI:1b}}"),
            format!("attribute {uuid} minecraft:attack_damage base set 1"),
            format!("attribute {uuid} minecraft:attack_knockback base set 0"),
            format!("attribute {attacker_uuid} minecraft:attack_damage base set 1"),
            format!("attribute {attacker_uuid} minecraft:attack_knockback base set 0"),
            "team add no_collision".to_owned(),
            "team modify no_collision collisionRule never".to_owned(),
            format!("team join no_collision {uuid}"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        if breeding {
            let start = world
                .stored_entity_for_test(DIMENSION, source)
                .unwrap()
                .position()
                .x;
            world.tick_rate_mut().set_frozen(false);
            tick_until(&mut world, 30, |world| {
                world
                    .stored_entity_for_test(DIMENSION, source)
                    .unwrap()
                    .position()
                    .x
                    > start + 0.25
            });
            world.tick_rate_mut().set_frozen(true);
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {attacker_uuid} {{NoAI:0b}}"),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {attacker_uuid} 1 minecraft:mob_attack by {uuid}"),
        );
        let source_before = mob_health(&world, source);
        let partner_before = mob_health(&world, partner);
        let attacker_before = mob_health(&world, attacker);
        world.tick_rate_mut().set_frozen(false);
        tick_until(&mut world, 120, |world| {
            mob_health(world, source) < source_before
        });
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {attacker_uuid} {{NoAI:1b}}"),
        );
        world.tick_rate_mut().set_frozen(false);
        tick_until(&mut world, 120, |world| {
            mob_health(world, attacker) < attacker_before
        });
        assert_eq!(mob_health(&world, partner), partner_before);
        let NbtSummaryValue::Compound(record) = world
            .stored_entity_command_data_for_test(DIMENSION, source)
            .unwrap()
        else {
            panic!("hoglin record");
        };
        assert!(record
            .iter()
            .any(|field| field.name == "InLove" && field.value == NbtSummaryValue::Int(0)));
        assert_eq!(
            world
                .stored_entity_ids_for_test(DIMENSION)
                .into_iter()
                .filter(|id| {
                    world
                        .stored_entity_for_test(DIMENSION, *id)
                        .unwrap()
                        .entity_type_id()
                        == VANILLA_ENTITY_TYPE_HOGLIN_ID
                })
                .count(),
            2
        );
    }
}

#[test]
fn adult_hoglins_keep_a_nearer_retaliation_target_and_ignore_hoglin_attackers() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID;

    for other_type in [
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
    ] {
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        let (source, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_HOGLIN_ID, near(0.0));
        let (nearer, nearer_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_CREEPER_ID, near(-4.0));
        let (other, other_uuid) = spawn(
            &mut world,
            other_type,
            near(if other_type == VANILLA_ENTITY_TYPE_HOGLIN_ID {
                2.0
            } else {
                7.0
            }),
        );
        for line in [
            "gamemode spectator alice".to_owned(),
            format!("data merge entity {uuid} {{NoAI:1b,IsImmuneToZombification:1b}}"),
            format!("data merge entity {nearer_uuid} {{NoAI:1b}}"),
            format!("data merge entity {other_uuid} {{NoAI:1b,IsImmuneToZombification:1b}}"),
            format!("attribute {uuid} minecraft:attack_damage base set 1"),
            format!("attribute {uuid} minecraft:attack_knockback base set 0"),
            "team add no_collision".to_owned(),
            "team modify no_collision collisionRule never".to_owned(),
            format!("team join no_collision {uuid}"),
            format!("damage {uuid} 1 minecraft:mob_attack by {nearer_uuid}"),
            format!("damage {uuid} 2 minecraft:mob_attack by {other_uuid}"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        let nearer_before = mob_health(&world, nearer);
        let other_before = mob_health(&world, other);
        let position = world
            .stored_entity_for_test(DIMENSION, source)
            .unwrap()
            .position();
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..240 {
            world.tick(Vec::new());
        }
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, source)
                .unwrap()
                .position(),
            position
        );
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:0b}}"),
        );
        world.tick_rate_mut().set_frozen(false);
        tick_until(&mut world, 120, |world| {
            mob_health(world, nearer) < nearer_before
        });
        assert_eq!(mob_health(&world, other), other_before);
    }
}

fn effect_duration(world: &ServerWorld, effect: &str) -> i32 {
    world
        .player_active_effect_for_test(ALICE, effect)
        .unwrap_or_else(|| panic!("Alice has {effect}"))
        .duration()
}

/// `CaveSpider.doHurtTarget`: seven seconds of poison on Normal.
#[test]
fn a_cave_spider_bite_poisons_for_seven_seconds_on_normal() {
    let (_sender, _receiver, mut world) = fixture();
    spawn(&mut world, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, near(3.0));
    first_hit(&mut world);
    let poison = effect_duration(&world, "minecraft:poison");
    assert!((130..=140).contains(&poison), "7 s of poison: {poison}");
}

/// `WitherSkeleton.doHurtTarget`: wither for 200 ticks at any difficulty.
#[test]
fn a_wither_skeleton_hit_withers_for_ten_seconds() {
    let (_sender, _receiver, mut world) = fixture();
    spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
        near(3.0),
    );
    first_hit(&mut world);
    let wither = effect_duration(&world, "minecraft:wither");
    assert!((190..=200).contains(&wither), "10 s of wither: {wither}");
}

/// `Husk.doHurtTarget`: an empty-handed husk's hunger lasts `140 *
/// (int)effectiveDifficulty` ticks; a fresh chunk on Normal is 1.5.
#[test]
fn an_empty_handed_husk_hit_starves_by_the_local_difficulty() {
    let (_sender, _receiver, mut world) = fixture();
    spawn(&mut world, VANILLA_ENTITY_TYPE_HUSK_ID, near(3.0));
    first_hit(&mut world);
    let hunger = effect_duration(&world, "minecraft:hunger");
    assert!(
        (130..=140).contains(&hunger),
        "140 ticks of hunger: {hunger}"
    );
}

/// `Zombie.doHurtTarget`: a burning, empty-handed zombie sets its target
/// alight with chance `0.3 * effectiveDifficulty` for `2 * (int)difficulty`
/// seconds. On Hard with the chunk fully inhabited the difficulty is
/// `3 * (0.75 + 1.0) = 5.25`, past certainty, so ten seconds of fire.
#[test]
fn a_burning_zombie_sets_the_player_alight() {
    let (sender, receiver, mut world) = fixture();
    command(&sender, &receiver, &mut world, "difficulty hard");
    world.add_inhabited_time_for_test(DIMENSION, 0, 0, 3_600_000);
    let (_, zombie) = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, near(3.0));
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {zombie} {{Fire:1000s}}"),
    );
    first_hit(&mut world);
    let fire = world.player_fire_ticks_for_test(ALICE);
    assert!((190..=200).contains(&fire), "10 s of fire: {fire}");
}

/// The values of the stinger count Alice saw for her own body.
fn stinger_counts(tick: &TickResult, alice: i32) -> Vec<EntityDataValueKind> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::SetEntityData(update),
            } if update.id == alice => Some(update),
            _ => None,
        })
        .flat_map(|update| &update.values)
        .filter(|value| value.data_id == VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID)
        .map(|value| value.value.clone())
        .collect()
}

/// `Bee.doHurtTarget` leaves a stinger in the player (`Bee.java:232`), which
/// `LivingEntity.tick` sheds after `20 * (30 - 1)` ticks
/// (`LivingEntity.java:2707-2716`).
#[test]
fn a_bee_sting_leaves_a_stinger_that_falls_out() {
    let (sender, receiver, mut world) = fixture();
    let alice = world.player_entity_id_for_test(ALICE);
    let (bee, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_BEE_ID, near(2.0));
    world.tick(Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: bee }),
    );
    let mut stung_at = None;
    for index in 0..1200 {
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
        let counts = stinger_counts(&tick, alice);
        if counts.contains(&EntityDataValueKind::Int(1)) {
            stung_at = Some(index);
        }
        if counts.contains(&EntityDataValueKind::Int(0)) {
            let stung_at = stung_at.expect("the stinger went in before it fell out");
            assert!(
                (575..=585).contains(&(index - stung_at)),
                "one stinger sheds after 580 ticks: {}",
                index - stung_at
            );
            return;
        }
    }
    panic!("the stinger never went in and fell out: {stung_at:?}");
}

/// A mob's spear strike that does not deal damage still knocks back and
/// unseats the player it reaches (`LivingEntity.stabAttack`,
/// `LivingEntity.java:2825-2867`). Without a hurt the knockback only reaches
/// the players tracking her (`ServerEntity.java:172-189`), here Bob.
#[test]
fn a_spear_stab_without_damage_shoves_and_unseats_the_player() {
    let (sender, receiver, mut world) = fixture();
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.set_player_position_for_test(ConnectionId(2), near(4.0));
    let alice = world.player_entity_id_for_test(ALICE);
    let (boat, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        PLAYER_POSITION,
        EntityData::Boat(BoatData::new()),
    );
    world.tick(Vec::new());
    assert!(world.attach_external_player_for_test(ALICE, boat));
    world.tick(Vec::new());
    let before = health(&world);
    world.with_level_for_test(DIMENSION, |level| {
        level.stab_player(crate::level_event::PendingPlayerStab {
            player: Uuid::from_u128(1),
            hurt: None,
            knockback: Some(crate::level_event::PendingHitImpulse::Knockback {
                power: 0.4,
                xd: 0.0,
                zd: 1.0,
            }),
            dismount: true,
        })
    });
    let mut shoved = false;
    for _ in 0..2 {
        let tick = world.tick(receiver.drain());
        shoved |= tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::SetEntityMotion(motion),
                } if motion.id == alice
            )
        });
    }
    assert!(
        world
            .external_player_vehicle_state_for_test(ALICE)
            .2
            .is_none(),
        "the stab unseated Alice"
    );
    assert!(shoved, "Bob saw the stab push Alice");
    assert_eq!(health(&world), before, "the stab dealt no damage");
}
