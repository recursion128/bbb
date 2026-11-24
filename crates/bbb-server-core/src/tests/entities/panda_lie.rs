//! Autonomous Lazy Panda back resting and water interruption.

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

fn back_event(tick: &TickResult, id: i32, on_back: bool) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(|value|
            value.data_id == VANILLA_PANDA_FLAGS_DATA_ID
                && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 16 != 0) == on_back))))
}

#[test]
fn lazy_pandas_rest_on_their_backs_and_water_ends_rest_with_a_cooldown() {
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
    ] {
        command(&sender, &receiver, &mut world, line);
    }
    let mut pandas = Vec::new();
    for (x, z, gene, no_ai) in [
        (4.5, 4.5, "lazy", false),
        (10.5, 4.5, "lazy", true),
        (4.5, 12.5, "normal", false),
    ] {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "summon minecraft:panda {x} 65.0 {z} {{Age:0,MainGene:\"{gene}\",HiddenGene:\"normal\",NoAI:1b,PersistenceRequired:1b}}"
            ),
        );
        let id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world.stored_entity_for_test(DIM, *id).is_some_and(|body| {
                    body.entity_type_id() == VANILLA_ENTITY_TYPE_PANDA_ID
                        && (body.position().x - x).abs() < 0.01
                        && (body.position().z - z).abs() < 0.01
                })
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
        assert!(
            matches!(world.stored_entity_for_test(DIM, id).unwrap().data(),
            EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        pandas.push((id, uuid));
    }
    let id = pandas[0].0;
    let mut rested = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..12_000 {
        let tick = world.tick(receiver.drain());
        for (index, (other_id, uuid)) in pandas.iter().enumerate() {
            let body = world.stored_entity_for_test(DIM, *other_id).unwrap();
            assert_eq!(body.uuid(), *uuid);
            assert!(body.living().unwrap().health() > 0.0);
            if index > 0 {
                assert!(!back_event(&tick, *other_id, true), "negative Panda rested");
            }
        }
        rested |= back_event(&tick, id, true);
        if rested {
            break;
        }
    }
    assert!(rested, "Lazy Panda did not autonomously rest");
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "fill 3 65 3 5 67 5 minecraft:water",
    );
    let (mut wet, mut stopped) = (false, false);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        wet |= world.stored_entity_for_test(DIM, id).unwrap().is_in_water();
        stopped |= back_event(&tick, id, false);
        if wet && stopped {
            break;
        }
    }
    assert!(wet && stopped, "wet={wet},stopped={stopped}");
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "fill 0 65 0 15 75 15 minecraft:air",
    );
    let mut dry = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        assert!(
            !back_event(&tick, id, true),
            "rest restarted during cooldown"
        );
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        dry |= !body.is_in_water();
        assert_eq!(body.uuid(), pandas[0].1);
        assert!(body.living().unwrap().health() > 0.0);
    }
    assert!(dry, "Panda did not leave the cleared water");
}
