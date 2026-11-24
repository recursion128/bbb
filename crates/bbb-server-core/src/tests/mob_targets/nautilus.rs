use super::melee::{health, ALICE};
use super::*;
use crate::tests::entities::{console_command, console_command_answer, restore_entity_writes};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_server_types::geometry::Vec3Ops;

const START: Vec3d = Vec3d {
    x: 3.5,
    y: 68.0,
    z: 5.5,
};
const TARGET: Vec3d = Vec3d {
    x: 10.5,
    y: 68.0,
    z: 5.5,
};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = world.game_time() as u64;
    let tick = console_command(sender, receiver, world, operation_id, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn pool() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.difficulty = Difficulty::Normal;
    let mut candidate = restore_candidate(0, None, Vec::new());
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(config, &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    command(
        &sender,
        &receiver,
        &mut world,
        "fill 0 65 0 15 74 15 minecraft:water",
    );
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 72.0,
            z: 12.5,
        },
    );
    (sender, receiver, world)
}

fn frozen_target(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    kind: i32,
) -> (i32, Uuid) {
    let (id, uuid) = spawn(world, kind, TARGET);
    command(
        sender,
        receiver,
        world,
        &format!("data merge entity {uuid} {{NoAI:1b,NoGravity:1b}}"),
    );
    (id, uuid)
}

fn mob_health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn remaining_cooldown(world: &ServerWorld, id: i32) -> i32 {
    remaining_memory(
        world,
        id,
        crate::entity::ai::MemoryModuleType::ChargeCooldownTicks,
    )
}

fn remaining_memory(
    world: &ServerWorld,
    id: i32,
    kind: crate::entity::ai::MemoryModuleType,
) -> i32 {
    let Some(NbtSummaryValue::Compound(record)) =
        world.stored_entity_command_data_for_test(DIMENSION, id)
    else {
        panic!("a living record");
    };
    let brain = record.iter().find(|entry| entry.name == "Brain").unwrap();
    let packed = crate::entity::ai::BrainPacked::decode(&brain.value).unwrap();
    match packed
        .memories
        .iter()
        .find(|entry| entry.memory_type == kind)
        .map(|entry| &entry.value)
    {
        Some(crate::entity::ai::MemoryValue::Int(ticks)) => *ticks,
        _ => 0,
    }
}

fn restore_pool(world: &mut ServerWorld) -> ServerWorld {
    let request = world
        .save_request(1_786_000_005_000, 206, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    install_forced_ticket(&mut candidate);
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut config = datapack_config();
    config.difficulty = Difficulty::Normal;
    let mut restored = spawn_pinned_restore(config, &candidate);
    install_floor(&mut restored);
    let water = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
    )
    .unwrap();
    for x in 0..16 {
        for y in 65..75 {
            for z in 0..16 {
                assert!(restored.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    water,
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
    }
    restored
}

fn reset_charge_positions(world: &mut ServerWorld, body: i32, target: i32) {
    for (id, position) in [(body, START), (target, TARGET)] {
        let entity = world.stored_entity_mut_for_test(DIMENSION, id).unwrap();
        entity.set_pos(position);
        entity.set_delta_movement(Vec3d::default());
    }
}

#[test]
fn wild_nautiluses_charge_through_water_and_damage_and_push_the_living_attacker() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = pool();
        let (target, target_uuid) = frozen_target(
            &sender,
            &receiver,
            &mut world,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        );
        let (id, uuid) = spawn(&mut world, kind, START);
        let before = mob_health(&world, target);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
        );
        tick_until(&mut world, 50, |world| mob_health(world, target) < before);
        assert_eq!(mob_health(&world, target), before - 3.0);
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert!(
            body.position().x > START.x + 4.0,
            "the charge crossed the pool: {:?}",
            body.position()
        );
        assert!((body.position().z - START.z).abs() < 0.1);
        let target_body = world.stored_entity_for_test(DIMENSION, target).unwrap();
        assert!(
            target_body.delta_movement().x > 0.0,
            "charge knockback: {:?}",
            target_body.delta_movement()
        );
        assert_eq!(target_of(&world, id), None, "collision ends the charge");
        reset_charge_positions(&mut world, id, target);
        tick_until(&mut world, 140, |world| {
            mob_health(world, target) < before - 3.0
        });
        for _ in 0..12 {
            world.tick(Vec::new());
        }
        assert!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .living()
                .unwrap()
                .last_hurt_by_mob()
                .is_none(),
            "the short hurt memory has expired while nautilus anger remains"
        );
        let cooldown = remaining_cooldown(&world, id);
        assert!(cooldown > 40 && cooldown < 80);
        let mut restored = restore_pool(&mut world);
        let id = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .unwrap();
        let target = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, target_uuid)
            .unwrap();
        assert_eq!(remaining_cooldown(&restored, id), cooldown);
        reset_charge_positions(&mut restored, id, target);
        let before = mob_health(&restored, target);
        for _ in 0..35 {
            restored.tick(Vec::new());
        }
        assert_eq!(
            mob_health(&restored, target),
            before,
            "reload preserves the charge cooldown"
        );
        assert!(
            remaining_cooldown(&restored, id) > 0 && remaining_cooldown(&restored, id) < cooldown
        );
        tick_until(&mut restored, 100, |world| {
            mob_health(world, target) < before
        });
        assert_eq!(
            mob_health(&restored, target),
            before - 3.0,
            "the longer anger survives save and reload"
        );
    }
}

#[test]
fn nautilus_charges_keep_their_line_and_hit_an_intercepting_living_entity() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = pool();
        let (target, target_uuid) = frozen_target(
            &sender,
            &receiver,
            &mut world,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        );
        let (id, uuid) = spawn(&mut world, kind, START);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
        );
        tick_until(&mut world, 8, |world| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                > START.x + 0.3
        });
        world
            .stored_entity_mut_for_test(DIMENSION, target)
            .unwrap()
            .set_pos(Vec3d { z: 9.5, ..TARGET });
        let (interceptor, interceptor_uuid) =
            spawn(&mut world, VANILLA_ENTITY_TYPE_VILLAGER_ID, TARGET);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {interceptor_uuid} {{NoAI:1b,NoGravity:1b}}"),
        );
        tick_until(&mut world, 40, |world| {
            mob_health(world, interceptor) < 20.0
        });
        assert_eq!(mob_health(&world, interceptor), 17.0);
        assert_eq!(mob_health(&world, target), 20.0);
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert!(
            (body.position().z - START.z).abs() < 0.1,
            "the velocity stayed on its initial line: {:?}",
            body.position()
        );
        let (sender, receiver, mut world) = pool();
        let (target, target_uuid) =
            frozen_target(&sender, &receiver, &mut world, VANILLA_ENTITY_TYPE_PIG_ID);
        command(
            &sender,
            &receiver,
            &mut world,
            "setblock 6 69 5 minecraft:stone",
        );
        let start = Vec3d { y: 68.2, ..START };
        world
            .stored_entity_mut_for_test(DIMENSION, target)
            .unwrap()
            .set_pos(Vec3d {
                y: start.y,
                ..TARGET
            });
        let (id, uuid) = spawn(&mut world, kind, start);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
        );
        tick_until(&mut world, 20, |world| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                > START.x + 1.5
        });
        for _ in 0..20 {
            world.tick(Vec::new());
        }
        assert_eq!(
            target_of(&world, id),
            Some(target_uuid),
            "the body is stuck with a visible target"
        );
        tick_until(&mut world, 75, |world| {
            target_of(world, id).is_none() && remaining_cooldown(world, id) > 0
        });
        assert_eq!(mob_health(&world, target), 10.0);
        assert!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                < 6.0,
            "the hanging obstacle stopped the body while its target remained visible"
        );
    }
}

#[test]
fn wild_nautiluses_retaliate_against_a_survival_player_with_damage_and_motion_packets() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = pool();
        world.set_player_position_for_test(ALICE, TARGET);
        let (id, uuid) = spawn(&mut world, kind, START);
        let before = health(&world);
        let tick = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by alice"),
        );
        let mut events = tick.events;
        for _ in 0..50 {
            events.extend(world.tick(Vec::new()).events);
            if health(&world) < before {
                break;
            }
        }
        events.extend(world.tick(Vec::new()).events);
        assert_eq!(health(&world), before - 3.0);
        let player_id = world.player_entity_id_for_test(ALICE);
        assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hit), .. } if hit.entity_id == player_id && hit.source_cause_id == id)));
        assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEntityMotion(motion) } if motion.delta_movement.x > 0.0)));
    }
}

#[test]
fn tame_and_baby_nautiluses_do_not_charge_and_taming_stops_a_running_charge() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        for state in ["{Age:-24000}", "{Owner:[I;0,0,0,1]}"] {
            if kind == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID && state == "{Age:-24000}" {
                continue;
            }
            let (sender, receiver, mut world) = pool();
            let (target, target_uuid) = frozen_target(
                &sender,
                &receiver,
                &mut world,
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
            );
            let (id, uuid) = spawn(&mut world, kind, START);
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {state}"),
            );
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
            );
            let before = mob_health(&world, target);
            for _ in 0..35 {
                world.tick(Vec::new());
            }
            assert_eq!(mob_health(&world, target), before);
            assert_eq!(target_of(&world, id), None);
        }
        let (sender, receiver, mut world) = pool();
        let (target, target_uuid) = frozen_target(
            &sender,
            &receiver,
            &mut world,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        );
        let (id, uuid) = spawn(&mut world, kind, START);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
        );
        tick_until(&mut world, 8, |world| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                > START.x + 0.3
        });
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{Owner:[I;0,0,0,1]}}"),
        );
        for _ in 0..30 {
            world.tick(Vec::new());
        }
        assert_eq!(target_of(&world, id), None);
        assert_eq!(mob_health(&world, target), 20.0);
    }
}

#[test]
fn nautiluses_follow_food_in_either_hand_stop_at_their_age_distance_and_save_the_cooldown() {
    use crate::entity::ai::MemoryModuleType::TemptationCooldownTicks;
    for (kind, baby, slot) in [
        (VANILLA_ENTITY_TYPE_NAUTILUS_ID, false, "mainhand"),
        (VANILLA_ENTITY_TYPE_NAUTILUS_ID, true, "offhand"),
        (VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, false, "offhand"),
    ] {
        let (sender, receiver, mut world) = pool();
        world.set_player_position_for_test(ALICE, TARGET);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("item replace entity alice weapon.{slot} with minecraft:cod"),
        );
        let (id, uuid) = spawn(&mut world, kind, START);
        if baby || kind == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{Age:-24000}}"),
            );
        }
        if kind == VANILLA_ENTITY_TYPE_NAUTILUS_ID && !baby {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{InLove:600}}"),
            );
        }
        let close = if baby { 2.5 } else { 3.5 };
        let mut events = Vec::new();
        for _ in 0..180 {
            events.extend(world.tick(Vec::new()).events);
            if world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .distance_to_sqr(TARGET)
                < close * close
            {
                break;
            }
        }
        for _ in 0..15 {
            world.tick(Vec::new());
        }
        let position = world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position();
        let distance = position.distance_to_sqr(TARGET).sqrt();
        assert!(
            position.x > START.x + 2.5,
            "food guides the body through water: {position:?}"
        );
        assert!(
            distance < close && distance > close - 0.8,
            "{kind}, baby={baby}: stops near {close}: {position:?}, distance={distance}"
        );
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::MoveEntity(packet), .. } if packet.id == id)));
        command(&sender, &receiver, &mut world, "clear alice");
        tick_until(&mut world, 30, |world| {
            remaining_memory(world, id, TemptationCooldownTicks) > 0
        });
        let cooldown = remaining_memory(&world, id, TemptationCooldownTicks);
        assert!(
            cooldown > 80 && cooldown <= 100,
            "removing food ends following"
        );
        let restored = restore_pool(&mut world);
        let restored_id = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .unwrap();
        assert_eq!(
            remaining_memory(&restored, restored_id, TemptationCooldownTicks),
            cooldown
        );
    }
}

#[test]
fn food_following_suppresses_anger_charges_until_the_food_is_removed() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = pool();
        world.set_player_position_for_test(ALICE, TARGET);
        command(
            &sender,
            &receiver,
            &mut world,
            "item replace entity alice weapon.mainhand with minecraft:cod",
        );
        let (id, uuid) = spawn(&mut world, kind, START);
        let before = health(&world);
        for _ in 0..60 {
            world.tick(Vec::new());
        }
        assert_eq!(target_of(&world, id), None);
        assert!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                > START.x + 2.0
        );
        command(&sender, &receiver, &mut world, "clear alice");
        let temptation = crate::entity::ai::MemoryModuleType::TemptationCooldownTicks;
        tick_until(&mut world, 30, |world| {
            remaining_memory(world, id, temptation) > 0
        });
        command(
            &sender,
            &receiver,
            &mut world,
            "item replace entity alice weapon.mainhand with minecraft:cod",
        );
        for _ in 0..25 {
            world.tick(Vec::new());
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by alice"),
        );
        for _ in 0..40 {
            world.tick(Vec::new());
        }
        assert!(remaining_memory(&world, id, temptation) > 0);
        assert_eq!(
            health(&world),
            before,
            "food still suppresses charging during the following cooldown"
        );
        assert_eq!(target_of(&world, id), None);
        for _ in 0..80 {
            world.tick(Vec::new());
        }
        assert_eq!(
            health(&world),
            before,
            "following food continues to suppress anger after the cooldown"
        );
        command(&sender, &receiver, &mut world, "clear alice");
        tick_until(&mut world, 75, |world| health(world) < before);
        assert_eq!(
            health(&world),
            before - 3.0,
            "anger remains after following ends"
        );
    }
}

#[test]
fn nautiluses_autonomously_swim_without_food_or_an_attack_target() {
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (_, _, mut world) = pool();
        let (id, _) = spawn(&mut world, kind, START);
        tick_until(&mut world, 400, |world| {
            let position = world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position();
            (position.x - START.x).hypot(position.z - START.z) > 1.0
        });
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert!(body.is_in_water());
        assert_eq!(target_of(&world, id), None);
    }
}

#[test]
fn a_nautilus_ignores_food_held_by_its_own_unsaddled_rider() {
    use crate::entity::ai::MemoryModuleType::TemptationCooldownTicks;
    for kind in [
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = pool();
        world.set_player_position_for_test(ALICE, START);
        let (id, uuid) = spawn(&mut world, kind, START);
        let owner = world.publication().players[0].uuid.as_u128().to_be_bytes();
        let owner = owner
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&part| i32::from_be_bytes(part).to_string())
            .collect::<Vec<_>>()
            .join(",");
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{Owner:[I;{owner}]}}"),
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(bbb_protocol::packets::InteractEntity {
                entity_id: id,
                hand: bbb_protocol::packets::InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        let rider = world.player_entity_id_for_test(ALICE);
        assert_eq!(
            world
                .external_player_relation_in_dimension_for_test(DIMENSION, rider)
                .0,
            Some(id)
        );
        command(
            &sender,
            &receiver,
            &mut world,
            "item replace entity alice weapon.mainhand with minecraft:cod",
        );
        for _ in 0..30 {
            world.tick(Vec::new());
        }
        command(&sender, &receiver, &mut world, "clear alice");
        for _ in 0..30 {
            world.tick(Vec::new());
        }
        assert_eq!(
            remaining_memory(&world, id, TemptationCooldownTicks),
            0,
            "the rider never starts a food-following session"
        );
    }
}

#[test]
fn a_hurt_nautilus_flees_through_water_while_food_is_offered() {
    let (sender, receiver, mut world) = pool();
    world.set_player_position_for_test(ALICE, TARGET);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:cod",
    );
    let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_NAUTILUS_ID, START);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:mob_attack by alice"),
    );
    let before = health(&world);
    let mut furthest = START.distance_to_sqr(TARGET);
    for _ in 0..80 {
        world.tick(Vec::new());
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        furthest = furthest.max(body.position().distance_to_sqr(TARGET));
    }
    assert!(
        furthest > START.distance_to_sqr(TARGET) + 8.0,
        "panic swims away despite the offered food: {furthest}"
    );
    assert_eq!(health(&world), before);
    command(&sender, &receiver, &mut world, "clear alice");
    // Bring the frightened animal back within charging range of its attacker.
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(START);
    tick_until(&mut world, 240, |world| health(world) < before);
    assert_eq!(health(&world), before - 3.0);
}

#[test]
fn nautilus_charge_armor_stand_hits_use_live_damage_tags_and_mob_griefing() {
    for (tag, mob_griefing, breaks, wobbles) in [
        (None, true, false, false),
        (Some("minecraft:can_break_armor_stand"), true, false, true),
        (
            Some("minecraft:always_kills_armor_stands"),
            true,
            true,
            false,
        ),
        (
            Some("minecraft:always_kills_armor_stands"),
            false,
            false,
            false,
        ),
    ] {
        let (sender, receiver, mut world) = pool();
        if let Some(tag) = tag {
            let config = datapack_config();
            let startup = config.datapacks.as_ref().unwrap();
            let tags = startup.tags.registries.iter().flat_map(|registry| {
                registry.tags.iter().map(|(name, members)| {
                    let mut members = members.clone();
                    if registry.registry_id == "minecraft:damage_type" && name == tag {
                        members.push("minecraft:mob_attack".to_owned());
                    }
                    (registry.registry_id.clone(), name.clone(), members)
                })
            });
            world.install_level_registry_tags(Arc::new(
                crate::registry_tags::RegistryTagSnapshot::new(tags),
            ));
        }
        if !mob_griefing {
            command(
                &sender,
                &receiver,
                &mut world,
                "gamerule mob_griefing false",
            );
        }
        let (target, target_uuid) = frozen_target(
            &sender,
            &receiver,
            &mut world,
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        );
        let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_NAUTILUS_ID, START);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic by {target_uuid}"),
        );
        tick_until(&mut world, 8, |world| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .position()
                .x
                > START.x + 0.3
        });
        world
            .stored_entity_mut_for_test(DIMENSION, target)
            .unwrap()
            .set_pos(Vec3d { z: 9.5, ..TARGET });
        command(
            &sender,
            &receiver,
            &mut world,
            "summon minecraft:armor_stand 10.5 68 5.5",
        );
        let stand = world
            .stored_entity_ids_for_test(DIMENSION)
            .into_iter()
            .find(|entity_id| {
                world
                    .stored_entity_for_test(DIMENSION, *entity_id)
                    .unwrap()
                    .entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
            })
            .unwrap();
        let stand_uuid = world
            .stored_entity_for_test(DIMENSION, stand)
            .unwrap()
            .uuid();
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                r#"data merge entity {stand_uuid} {{NoGravity:1b,CustomName:{{text:"Charged",color:"gold",bold:1b,extra:[{{text:" stand",italic:1b}}]}},equipment:{{head:{{id:"minecraft:iron_helmet",count:1}}}}}}"#
            ),
        );
        let body = world.stored_entity_for_test(DIMENSION, stand).unwrap();
        let name = body.custom_name().unwrap().component().clone();
        let particle_position = Vec3d {
            y: body.position().y + f64::from(body.bb_height()) * (2.0 / 3.0),
            ..body.position()
        };
        let particle_offset = Vec3d {
            x: f64::from(body.bb_width() / 4.0),
            y: f64::from(body.bb_height() / 4.0),
            z: f64::from(body.bb_width() / 4.0),
        };
        let mut events = Vec::new();
        for _ in 0..40 {
            events.extend(world.tick(Vec::new()).events);
            if world
                .stored_entity_for_test(DIMENSION, stand)
                .is_none_or(|stand| stand.delta_movement().x > 0.0)
            {
                break;
            }
        }
        let hit_event = events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. }
                | WorldEvent::Broadcast { packet: ClientboundPlayPacket::EntityEvent(event), .. }
                if event.entity_id == stand && event.event_id == 32
        ));
        assert_eq!(hit_event, wobbles, "{tag:?}, mob_griefing={mob_griefing}");
        if breaks {
            assert!(world.stored_entity_for_test(DIMENSION, stand).is_none());
            let items = world.stored_items_for_test(DIMENSION);
            for item in ["minecraft:armor_stand", "minecraft:iron_helmet"] {
                assert!(
                    items.iter().any(|drop| drop.2 == item && drop.3 == 1),
                    "{items:?}"
                );
            }
            let drop_id = items
                .iter()
                .find(|drop| drop.2 == "minecraft:armor_stand")
                .unwrap()
                .0;
            let drop = world
                .stored_item_stack_for_test(DIMENSION, drop_id)
                .unwrap();
            assert_eq!(
                drop.component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    &world.item_stack_resolver(),
                )
                .unwrap(),
                Some(name),
            );
            let particle = events
                .iter()
                .find_map(|event| match event {
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::LevelParticles(particle),
                        ..
                    }
                    | WorldEvent::Broadcast {
                        packet: ClientboundPlayPacket::LevelParticles(particle),
                        ..
                    } if particle.particle.particle_type_id
                        == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID =>
                    {
                        Some(particle)
                    }
                    _ => None,
                })
                .expect("breaking the stand sends oak planks particles");
            assert_eq!(particle.count, 10);
            assert_eq!(particle.position, particle_position);
            assert_eq!(particle.offset, particle_offset);
            assert_eq!(particle.max_speed, 0.05);
            assert_eq!(
                bbb_protocol::codec::Decoder::new(&particle.particle.raw_options)
                    .read_var_i32()
                    .unwrap(),
                block_state_id("minecraft:oak_planks"),
            );
            assert!(events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                    | WorldEvent::Broadcast { packet: ClientboundPlayPacket::Sound(sound), .. }
                    if sound.sound == bbb_protocol::packets::SoundEventHolder::Reference {
                        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:entity.armor_stand.break").unwrap()
                    }
            )));
            assert!(world
                .positioned_game_events_for_test(DIMENSION)
                .iter()
                .any(|event| {
                    event.recorded.game_event == crate::level_event::VanillaGameEvent::ENTITY_DIE
                        && event.recorded.source_entity_id == Some(stand)
                }));
        } else {
            let stand = world.stored_entity_for_test(DIMENSION, stand).unwrap();
            assert!(stand.is_alive());
            if mob_griefing {
                assert!(
                    stand.delta_movement().x > 0.0,
                    "{tag:?}: charge knockback {:?}",
                    stand.delta_movement()
                );
            } else {
                assert_eq!(stand.delta_movement().x, 0.0);
                assert_eq!(stand.position().x, TARGET.x);
            }
        }
        if wobbles {
            assert!(world
                .positioned_game_events_for_test(DIMENSION)
                .iter()
                .any(|event| {
                    event.recorded.game_event == crate::level_event::VanillaGameEvent::ENTITY_DAMAGE
                        && event.recorded.source_entity_id == Some(id)
                }));
        }
        assert_eq!(mob_health(&world, target), 20.0);
    }
}
