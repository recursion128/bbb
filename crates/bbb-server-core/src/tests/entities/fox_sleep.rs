//! Daylight shelter sleep and night wake through actual entity tracking.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_FOX_FLAGS_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::packets::EntityDataValueKind;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    line: &str,
) {
    let operation = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation, line);
    let outcome = admin_outcome(&tick);
    assert!(!outcome.replayed, "{line}");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{line}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{line}: {report:?}");
}

fn sleeping_event(tick: &TickResult, id: i32, sleeping: bool) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(|value|
            value.data_id == VANILLA_FOX_FLAGS_DATA_ID
                && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 32 != 0) == sleeping))))
}

#[test]
fn sheltered_foxes_sleep_by_day_and_wake_at_night_with_no_ai_sky_and_threat_controls() {
    for (no_ai, roof, chicken) in [
        (false, true, false),
        (true, true, false),
        (false, false, false),
        (false, true, true),
    ] {
        let expected = !no_ai && roof && !chicken;
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Creative;
        config.keep_alive_interval_ticks = 100_000;
        let mut world = spawn_pinned_world(config);
        world.tick_rate_mut().set_frozen(true);
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        for line in [
            "gamerule spawn_mobs false",
            "gamerule advance_time false",
            "gamerule random_tick_speed 0",
            "time set noon",
            "weather clear",
            "tp observer 12.5 65.0 12.5",
            "fill 0 64 0 15 75 15 minecraft:stone",
            "fill 0 65 0 15 75 15 minecraft:air",
            "fill 0 64 0 15 64 15 minecraft:grass_block",
            "fill 3 65 3 5 67 5 minecraft:stone",
            "fill 4 65 4 4 66 4 minecraft:air",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        if !roof {
            command(
                &sender,
                &receiver,
                &mut world,
                "setblock 4 67 4 minecraft:air",
            );
        }
        if chicken {
            command(
                &sender,
                &receiver,
                &mut world,
                "summon minecraft:chicken 7.5 65.0 4.5 {NoAI:1b,PersistenceRequired:1b}",
            );
        }
        command(
            &sender,
            &receiver,
            &mut world,
            "summon minecraft:fox 4.5 65.0 4.5 {NoAI:1b,PersistenceRequired:1b}",
        );
        let id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID)
            })
            .unwrap();
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0"),
        );
        if !no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:0b}}"),
            );
        }
        let mut slept = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..400 {
            let tick = world.tick(receiver.drain());
            slept |= sleeping_event(&tick, id, true);
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.living().unwrap().health() > 0.0);
            if expected && slept {
                break;
            }
        }
        assert_eq!(
            slept, expected,
            "NoAI={no_ai},roof={roof},chicken={chicken}"
        );
        if expected {
            world.tick_rate_mut().set_frozen(true);
            command(&sender, &receiver, &mut world, "time set midnight");
            world.tick_rate_mut().set_frozen(false);
            let woke = (0..40).any(|_| {
                let tick = world.tick(receiver.drain());
                sleeping_event(&tick, id, false)
            });
            assert!(woke, "sleeping Fox did not wake at night");
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.living().unwrap().health() > 0.0);
        }
    }
}
