//! Real session, village and omen shared by raid scenarios.

use super::*;
use crate::admin::AdminResult;
use crate::entity::Entity;
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::world::{TickResult, WorldCommand};
use bbb_protocol::packets::PlayServerbound;

pub(super) const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn answer(sender: &WorldCommandSender, output: &TickResult) {
    for event in &output.events {
        assert!(
            !matches!(
                event,
                WorldEvent::Close {
                    connection_id: HERO,
                    ..
                }
            ),
            "{event:?}"
        );
        let packet = match event {
            WorldEvent::Send {
                connection_id: HERO,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } => Some(PlayServerbound::KeepAlive { id: *id }),
            WorldEvent::Send {
                connection_id: HERO,
                packet: ClientboundPlayPacket::ChunkBatchFinished { .. },
            } => Some(PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick: 64.0,
            }),
            _ => None,
        };
        if let Some(packet) = packet {
            sender
                .try_send(WorldCommand::Play {
                    connection_id: HERO,
                    packet,
                })
                .unwrap();
        }
    }
}
pub(super) fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let output = world.tick(receiver.drain());
    answer(sender, &output);
    output
}
pub(super) fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let output = crate::tests::console_command(sender, receiver, world, operation, text);
    let report = output
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => Some(&outcome.result),
            _ => None,
        })
        .expect("the real console answers");
    assert!(
        matches!(report, Ok(AdminResult::Command(report)) if report.result.is_some()),
        "{text}: {report:?}"
    );
    answer(sender, &output);
    output
}
pub(super) fn members(world: &ServerWorld, raid: i32, wave: i32) -> Vec<&Entity> {
    let level = world.level_by_id(&DIMENSION.into());
    raid_manager(world).runtime[&raid].raiders[&wave]
        .keys()
        .filter_map(|uuid| {
            level
                .entities
                .entity_id_for_uuid(*uuid)
                .and_then(|id| level.entities.get(id))
        })
        .filter(|body| body.is_alive_with_health())
        .collect()
}

pub(super) fn pre_wave() -> (
    ServerWorld,
    WorldCommandSender,
    WorldCommandReceiver,
    i32,
    f32,
) {
    let (mut world, origin) = hero_world_in_a_village();
    let (sender, receiver) = crate::tests::mailbox();
    world.tick_rate_mut().set_frozen(true);
    sender
        .try_send(WorldCommand::Play {
            connection_id: HERO,
            packet: PlayServerbound::PlayerLoaded,
        })
        .unwrap();
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "time set noon",
        "weather clear",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let min_x = origin.x.div_euclid(16) * 16 - 128;
    let min_z = origin.z.div_euclid(16) * 16 - 128;
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "forceload add {} {} {} {}",
            min_x,
            min_z,
            min_x + 255,
            min_z + 255
        ),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "effect give hero minecraft:raid_omen 60 0 true",
    );
    let mut trigger_events = Vec::new();
    world.create_or_extend_raid(HERO, origin, &mut trigger_events);
    assert!(
        !trigger_events
            .iter()
            .any(|event| matches!(event, WorldEvent::ConsoleMessage(_))),
        "{trigger_events:?}"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "effect clear hero minecraft:raid_omen",
    );
    let raid_id = *raid_manager(&world)
        .saved
        .raids
        .keys()
        .next()
        .expect("the actual omen trigger created a raid");
    assert_eq!(raid_manager(&world).saved.raids.len(), 1);
    let health = world.player_health_for_test(HERO);
    let row = raid_manager(&world).row(raid_id).unwrap();
    assert_eq!(row.groups_spawned, 0);
    assert_eq!(raid_manager(&world).total_raiders_alive(raid_id), 0);
    assert!(!world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .any(crate::entity::raider::holds_current_raid));
    world.tick_rate_mut().set_frozen(false);
    (world, sender, receiver, raid_id, health)
}

pub(super) fn natural_wave() -> (
    ServerWorld,
    WorldCommandSender,
    WorldCommandReceiver,
    i32,
    i32,
    f32,
) {
    let (mut world, sender, receiver, raid_id, health) = pre_wave();
    let mut wave = None;
    for _ in 0..600 {
        tick(&sender, &receiver, &mut world);
        let row = raid_manager(&world).row(raid_id).unwrap();
        if row.groups_spawned > 0 {
            wave = Some(row.groups_spawned);
            break;
        }
    }
    let wave = wave.expect("the unmodified raid cooldown naturally spawns its first wave");
    (world, sender, receiver, raid_id, wave, health)
}
