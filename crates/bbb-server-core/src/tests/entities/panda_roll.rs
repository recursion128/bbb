//! Autonomous Panda roll metadata and motion on a real ledge.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_PANDA_FLAGS_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
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

fn rolling_event(tick: &TickResult, id: i32, rolling: bool) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(|value|
            value.data_id == VANILLA_PANDA_FLAGS_DATA_ID
                && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 4 != 0) == rolling))))
}

#[test]
fn babies_and_playful_pandas_autonomously_roll_off_a_ledge_and_finish_the_motion() {
    for (age, gene, no_ai, expected) in [
        (-24000, "normal", false, true),
        (0, "playful", false, true),
        (0, "normal", false, false),
        (-24000, "normal", true, false),
    ] {
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
            "time set midnight",
            "weather clear",
            "tp observer 12.5 65.0 12.5",
            "fill 0 64 0 15 75 15 minecraft:stone",
            "fill 0 65 0 15 75 15 minecraft:air",
            "setblock 4 68 4 minecraft:stone",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        command(&sender, &receiver, &mut world, &format!(
            "summon minecraft:panda 4.5 69.0 4.5 {{Age:{age},MainGene:\"{gene}\",HiddenGene:\"normal\",NoAI:1b,PersistenceRequired:1b}}"
        ));
        let id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_PANDA_ID)
            })
            .unwrap();
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        let uuid = body.uuid();
        let start = body.position();
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
        assert!(
            matches!(world.stored_entity_for_test(DIM, id).unwrap().data(),
            EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        let mut started = false;
        let mut finished = false;
        let mut moved = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..200 {
            let tick = world.tick(receiver.drain());
            finished |= started && rolling_event(&tick, id, false);
            started |= rolling_event(&tick, id, true);
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.living().unwrap().health() > 0.0);
            let at = body.position();
            moved |=
                started && at.y > start.y + 0.1 && (at.x - start.x).hypot(at.z - start.z) > 0.15;
            if expected && started && finished && moved {
                break;
            }
        }
        assert_eq!(started, expected, "Age={age},gene={gene},NoAI={no_ai}");
        if expected {
            assert!(
                finished && moved,
                "Age={age},gene={gene}: completed={finished},hop={moved}"
            );
        }
    }
}
