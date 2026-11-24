use super::horse_interaction::record;
use super::*;
use bbb_protocol::entity_synched_data::VANILLA_ARMADILLO_STATE_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
use bbb_protocol::packets::{
    EntityDataEnumSerializer, EntityDataValueKind, PlayerCommand, PlayerCommandAction,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const ALICE: ConnectionId = ConnectionId(1);

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: u64,
    line: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, id, line);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{line}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    for x in 0..16 {
        for z in 0..16 {
            assert!(world.set_block(
                DIM,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        100,
        "gamerule spawn_mobs false",
    );
    command(&sender, &receiver, &mut world, 101, "time set midnight");
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 8.5,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_ARMADILLO_ID).unwrap(),
        ),
    );
    (sender, receiver, world, id, uuid)
}

fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let tick = world.tick(receiver.drain());
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: *connection_id,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
    tick
}

fn has_pose(tick: &TickResult, entity: i32, expected: i32) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet:ClientboundPlayPacket::SetEntityData(data), .. }
        if data.id == entity && data.values.iter().any(|value|
            value.data_id == VANILLA_ARMADILLO_STATE_DATA_ID && value.value == EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::ArmadilloState, id: expected,
            })
    ))
}

fn pose(world: &ServerWorld, id: i32) -> String {
    record(world, id)
        .into_iter()
        .find(|entry| entry.name == "state")
        .and_then(|entry| match entry.value {
            NbtSummaryValue::String(value) => Some(value.try_to_string().unwrap()),
            _ => None,
        })
        .unwrap()
}

fn danger_ttl(world: &ServerWorld, id: i32) -> Option<i64> {
    let saved = record(world, id);
    let compound = |entries: &[bbb_protocol::NbtSummaryEntry], name: &str| {
        entries
            .iter()
            .find(|field| field.name == name)
            .and_then(|field| match &field.value {
                NbtSummaryValue::Compound(fields) => Some(fields.clone()),
                _ => None,
            })
    };
    let brain = compound(&saved, "Brain")?;
    let memories = compound(&brain, "memories")?;
    let danger = compound(&memories, "minecraft:danger_detected_recently")?;
    danger
        .into_iter()
        .find(|field| field.name == "ttl")
        .and_then(|field| match field.value {
            NbtSummaryValue::Long(ttl) => Some(ttl),
            _ => None,
        })
}

#[test]
fn armadillos_roll_for_unseen_undead_and_restore_saved_danger_before_unrolling() {
    let (sender, receiver, mut world, armadillo, uuid) = fixture();
    let (_, zombie_uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 11.5,
            y: 65.0,
            z: 8.5,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
        ),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        102,
        &format!("data merge entity {zombie_uuid} {{NoAI:1b}}"),
    );
    for z in 7..=9 {
        for y in 65..=66 {
            assert!(world.set_block(
                DIM,
                BlockPos { x: 9, y, z },
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    world.tick_rate_mut().set_frozen(false);
    let mut rolling = false;
    let mut scared = false;
    let mut peeked = false;
    for _ in 0..500 {
        let result = tick(&sender, &receiver, &mut world);
        rolling |= has_pose(&result, armadillo, 1);
        scared |= has_pose(&result, armadillo, 2);
        peeked |= result.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. }
                if event.entity_id == armadillo && event.event_id == 64
            )
        });
    }
    assert!(
        rolling && scared && peeked,
        "rolling={rolling}, scared={scared}, peeked={peeked}"
    );
    assert_eq!(pose(&world, armadillo), "scared");
    let saved = record(&world, armadillo);
    let remaining = danger_ttl(&world, armadillo).unwrap();
    assert!((76..=80).contains(&remaining));
    command(
        &sender,
        &receiver,
        &mut world,
        200,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    for _ in 0..20 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(danger_ttl(&world, armadillo), Some(remaining));
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        201,
        &format!("kill {zombie_uuid}"),
    );
    world.load_stored_entity_record_for_test(DIM, armadillo, &saved);
    assert_eq!(danger_ttl(&world, armadillo), Some(remaining));
    world.tick_rate_mut().set_frozen(false);
    let mut unrolling = false;
    let mut idle = false;
    for _ in 0..120 {
        let result = tick(&sender, &receiver, &mut world);
        unrolling |= has_pose(&result, armadillo, 3);
        idle |= has_pose(&result, armadillo, 0);
    }
    assert!(unrolling && idle);
    assert_eq!(pose(&world, armadillo), "idle");
    assert_eq!(danger_ttl(&world, armadillo), None);
}

#[test]
fn armadillos_roll_for_real_sprinting_players_and_relax_after_they_stop() {
    let (sender, receiver, mut world, armadillo, _) = fixture();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..10 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(pose(&world, armadillo), "idle");
    let player = world.player_entity_id_for_test(ALICE);
    let mut rolled = false;
    for action in [
        PlayerCommandAction::StartSprinting,
        PlayerCommandAction::StopSprinting,
    ] {
        let result = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerCommand(PlayerCommand {
                entity_id: player,
                action,
                data: 0,
            }),
        );
        rolled |= has_pose(&result, armadillo, 1);
        for _ in 0..20 {
            let result = tick(&sender, &receiver, &mut world);
            rolled |= has_pose(&result, armadillo, 1);
        }
    }
    assert!(rolled);
    for _ in 0..100 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(pose(&world, armadillo), "idle");
}

#[test]
fn rolled_armadillos_reduce_damage_with_no_ai_and_unroll_for_environmental_hurt() {
    let (sender, receiver, mut world, armadillo, uuid) = fixture();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..5 {
        tick(&sender, &receiver, &mut world);
    }
    let health = |world: &ServerWorld| {
        world
            .stored_entity_for_test(DIM, armadillo)
            .unwrap()
            .living()
            .unwrap()
            .health()
    };
    let before = health(&world);
    command(
        &sender,
        &receiver,
        &mut world,
        200,
        &format!("damage {uuid} 1 minecraft:mob_attack by alice"),
    );
    assert_eq!(health(&world), before - 1.0);
    assert_eq!(pose(&world, armadillo), "rolling");
    for _ in 0..25 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(pose(&world, armadillo), "scared");
    command(
        &sender,
        &receiver,
        &mut world,
        201,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        202,
        &format!("damage {uuid} 5 minecraft:mob_attack by alice"),
    );
    assert_eq!(health(&world), before - 3.0);
    assert_eq!(pose(&world, armadillo), "scared");
    command(
        &sender,
        &receiver,
        &mut world,
        203,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    for _ in 0..25 {
        tick(&sender, &receiver, &mut world);
    }
    command(
        &sender,
        &receiver,
        &mut world,
        204,
        &format!("damage {uuid} 5 minecraft:in_fire"),
    );
    assert_eq!(health(&world), before - 5.0);
    assert_eq!(pose(&world, armadillo), "idle");
    assert_eq!(danger_ttl(&world, armadillo), None);
}
