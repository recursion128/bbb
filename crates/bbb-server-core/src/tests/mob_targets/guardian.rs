use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID,
    VANILLA_ENTITY_TYPE_SQUID_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, PlayerAbilities};

fn guardian(world: &mut ServerWorld, kind: i32) -> i32 {
    let id = spawn(
        world,
        kind,
        Vec3d {
            x: 3.5,
            y: 67.0,
            ..PLAYER_POSITION
        },
    )
    .0;
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_no_gravity(true);
    id
}

fn beam_targets(tick: &TickResult, guardian: i32) -> Vec<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } if update.id == guardian => Some(update),
            _ => None,
        })
        .flat_map(|update| &update.values)
        .filter_map(|value| {
            if value.data_id == VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID {
                if let EntityDataValueKind::Int(id) = value.value {
                    return Some(id);
                }
            }
            None
        })
        .collect()
}

fn is_magic_hit(event: &WorldEvent, guardian: i32) -> bool {
    let magic = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:indirect_magic")
        .unwrap()
        .registry_id;
    matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hurt), .. }
        if hurt.source_type_id == magic && hurt.source_cause_id == guardian && hurt.source_direct_id == guardian)
}

#[test]
fn guardians_charge_a_visible_beam_then_hurt_a_survival_player() {
    for (kind, damage) in [
        (VANILLA_ENTITY_TYPE_GUARDIAN_ID, 6.0),
        (VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, 8.0),
    ] {
        let (sender, receiver, mut world) = fixture();
        let id = guardian(&mut world, kind);
        let player = world.player_entity_id_for_test(ALICE);
        let before = health(&world);
        let mut targets = Vec::new();
        let mut events = Vec::new();
        let mut merged = false;
        for _ in 0..180 {
            let tick = world.tick(Vec::new());
            targets.extend(beam_targets(&tick, id));
            events.extend(tick.events);
            if !merged && targets.contains(&player) {
                let uuid = world.stored_entity_for_test(DIMENSION, id).unwrap().uuid();
                let tick = console_command(
                    &sender,
                    &receiver,
                    &mut world,
                    12,
                    &format!("data merge entity {uuid} {{Silent:1b}}"),
                );
                assert!(console_command_answer(&tick).0.is_some());
                targets.extend(beam_targets(&tick, id));
                events.extend(tick.events);
                assert_eq!(targets.last(), Some(&player));
                merged = true;
            }
            if health(&world) < before {
                break;
            }
        }
        assert!(merged);
        assert_eq!(health(&world), before - damage);
        assert!(targets.contains(&player));
        assert!(events.iter().any(|event| is_magic_hit(event, id)));
        assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(update), .. } if update.entity_id == id && update.event_id == 21)));
        for _ in 0..4 {
            targets.extend(beam_targets(&world.tick(Vec::new()), id));
        }
        assert_eq!(targets.last(), Some(&0));
    }
}

#[test]
fn a_guardian_beam_hurts_a_stored_squid_and_names_its_attacker() {
    let (_sender, _receiver, mut world) = fixture();
    world.set_player_mode_and_abilities_for_test(
        ALICE,
        GameType::Creative,
        PlayerAbilities::for_game_type(GameType::Creative),
    );
    let id = guardian(&mut world, VANILLA_ENTITY_TYPE_GUARDIAN_ID);
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_SQUID_ID).unwrap();
    data.set_no_ai(true);
    let victim = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_SQUID_ID,
            Vec3d {
                x: 12.5,
                y: 67.0,
                ..PLAYER_POSITION
            },
            EntityData::GeneralLiving(data),
        )
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, victim)
        .unwrap()
        .set_no_gravity(true);
    let victim_health = |world: &ServerWorld| {
        world
            .stored_entity_for_test(DIMENSION, victim)
            .unwrap()
            .living()
            .unwrap()
            .health()
    };
    let before = victim_health(&world);
    let mut targets = Vec::new();
    let mut events = Vec::new();
    for _ in 0..180 {
        let tick = world.tick(Vec::new());
        targets.extend(beam_targets(&tick, id));
        events.extend(tick.events);
        if victim_health(&world) < before {
            break;
        }
    }
    assert_eq!(victim_health(&world), before - 6.0);
    assert!(targets.contains(&victim));
    assert!(events.iter().any(|event| is_magic_hit(event, id)));
}

#[test]
fn an_opaque_wall_cancels_a_guardian_beam_before_it_hurts() {
    let (_sender, _receiver, mut world) = fixture();
    let id = guardian(&mut world, VANILLA_ENTITY_TYPE_GUARDIAN_ID);
    let player = world.player_entity_id_for_test(ALICE);
    let mut charging = false;
    for _ in 0..100 {
        let tick = world.tick(Vec::new());
        if beam_targets(&tick, id).contains(&player) {
            charging = true;
            break;
        }
    }
    assert!(charging);
    let before = health(&world);
    for y in 65..=70 {
        for z in 6..=10 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x: 6, y, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    let mut cleared = false;
    for _ in 0..120 {
        let tick = world.tick(Vec::new());
        cleared |= beam_targets(&tick, id).contains(&0);
    }
    assert!(cleared);
    assert_eq!(health(&world), before);
}

fn spiked_guardian(
    world: &mut ServerWorld,
    kind: i32,
    position: Vec3d,
    moving: bool,
) -> (i32, Uuid) {
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    data.guardian_state_mut().unwrap().moving = moving;
    world.spawn_stored_entity_for_test(DIMENSION, kind, position, EntityData::GeneralLiving(data))
}

fn thorns_hit(event: &WorldEvent, guardian: i32, victim: i32) -> bool {
    let thorns = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:thorns")
        .unwrap()
        .registry_id;
    matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hurt), .. }
        if hurt.entity_id == victim && hurt.source_type_id == thorns
            && hurt.source_cause_id == guardian && hurt.source_direct_id == guardian)
}

#[test]
fn a_player_melee_hit_meets_stationary_guardian_spikes_even_when_invulnerable() {
    for kind in [
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
    ] {
        for moving in [false, true] {
            let (sender, receiver, mut world) = fixture();
            let id = spiked_guardian(
                &mut world,
                kind,
                Vec3d {
                    z: PLAYER_POSITION.z + 1.5,
                    ..PLAYER_POSITION
                },
                moving,
            )
            .0;
            let entity = world.stored_entity_mut_for_test(DIMENSION, id).unwrap();
            entity.set_invulnerable(true);
            let before = health(&world);
            let player = world.player_entity_id_for_test(ALICE);
            let tick = play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::Attack(bbb_protocol::packets::AttackEntity { entity_id: id }),
            );
            assert_eq!(health(&world), before - if moving { 0.0 } else { 2.0 });
            assert_eq!(
                tick.events
                    .iter()
                    .any(|event| thorns_hit(event, id, player)),
                !moving
            );
            let entity = world.stored_entity_for_test(DIMENSION, id).unwrap();
            assert_eq!(
                entity.living().unwrap().health(),
                entity.living().unwrap().max_health()
            );
        }
    }
}

#[test]
fn a_golems_melee_hit_on_guardian_spikes_hurts_the_attacking_body() {
    let (_sender, _receiver, mut world) = fixture();
    world.set_player_mode_and_abilities_for_test(
        ALICE,
        GameType::Creative,
        PlayerAbilities::for_game_type(GameType::Creative),
    );
    let guardian = spiked_guardian(
        &mut world,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        Vec3d {
            x: 5.5,
            ..PLAYER_POSITION
        },
        false,
    )
    .0;
    let golem = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d {
            x: 6.5,
            ..PLAYER_POSITION
        },
    )
    .0;
    let mut events = Vec::new();
    for _ in 0..100 {
        events.extend(world.tick(Vec::new()).events);
        if world
            .stored_entity_for_test(DIMENSION, golem)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < 100.0
        {
            break;
        }
    }
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, golem)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        98.0
    );
    assert!(
        world
            .stored_entity_for_test(DIMENSION, guardian)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < 30.0
    );
    assert!(events
        .iter()
        .any(|event| thorns_hit(event, guardian, golem)));
}

#[test]
fn thorns_and_nonliving_direct_sources_do_not_trigger_guardian_spikes() {
    for indirect in [false, true] {
        let (sender, receiver, mut world) = fixture();
        let (guardian, uuid) = spiked_guardian(
            &mut world,
            VANILLA_ENTITY_TYPE_GUARDIAN_ID,
            Vec3d {
                x: 5.5,
                ..PLAYER_POSITION
            },
            false,
        );
        let (direct_id, direct) = if indirect {
            world.spawn_stored_entity_for_test(
                DIMENSION,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
                PLAYER_POSITION,
                EntityData::Arrow(crate::entity::ArrowData::new(
                    crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
                )),
            )
        } else {
            spawn(&mut world, VANILLA_ENTITY_TYPE_SQUID_ID, PLAYER_POSITION)
        };
        let before = health(&world);
        let command = if indirect {
            format!("damage {uuid} 1 minecraft:mob_attack by {direct} from alice")
        } else {
            format!("damage {uuid} 1 minecraft:thorns by {direct}")
        };
        let tick = console_command(&sender, &receiver, &mut world, 21, &command);
        assert!(
            console_command_answer(&tick).0.is_some(),
            "{command}: {:?}",
            console_command_answer(&tick)
        );
        assert_eq!(health(&world), before);
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, guardian)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            29.0
        );
        if !indirect {
            let squid = world.stored_entity_for_test(DIMENSION, direct_id).unwrap();
            assert_eq!(
                squid.living().unwrap().health(),
                squid.living().unwrap().max_health()
            );
        }
    }
}

#[test]
fn a_reloaded_damage_tag_can_disable_guardian_spikes_for_player_melee() {
    let (sender, receiver, mut world) = fixture();
    let mut startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:damage_type")
        .unwrap()
        .tags
        .insert(
            "minecraft:avoids_guardian_thorns".to_string(),
            vec!["minecraft:player_attack".to_string()],
        );
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: crate::world::ReloadedDatapacks::Built(std::sync::Arc::new(startup)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let id = spiked_guardian(
        &mut world,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 1.5,
            ..PLAYER_POSITION
        },
        false,
    )
    .0;
    let before = health(&world);
    let player = world.player_entity_id_for_test(ALICE);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(bbb_protocol::packets::AttackEntity { entity_id: id }),
    );
    assert_eq!(health(&world), before);
    assert!(!tick
        .events
        .iter()
        .any(|event| thorns_hit(event, id, player)));
    assert!(
        world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < 30.0
    );
}
