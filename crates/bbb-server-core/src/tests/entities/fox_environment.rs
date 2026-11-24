//! Autonomous fox fluid and powder-snow escape through actual world ticks.

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

fn flags(body: &crate::entity::Entity) -> i8 {
    crate::entity::tracking::synched_values(body)
        .into_iter()
        .find_map(|value| match value.value.value {
            EntityDataValueKind::Byte(flags)
                if value.value.data_id == VANILLA_FOX_FLAGS_DATA_ID =>
            {
                Some(flags)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn resting_foxes_escape_water_lava_and_powder_snow_with_no_ai_control() {
    for (material, no_ai) in [
        ("water", false),
        ("lava", false),
        ("powder_snow", false),
        ("water", true),
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
            "forceload add 0 0 15 15",
            "fill 0 64 0 15 70 15 minecraft:stone",
            "fill 5 65 5 10 70 10 minecraft:air",
            "tp observer 12.5 71.0 8.5",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        let top = if material == "powder_snow" { 65 } else { 67 };
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("fill 5 65 5 10 {top} 10 minecraft:{material}"),
        );
        command(&sender, &receiver, &mut world,
            "summon minecraft:fox 7.5 65.0 7.5 {NoAI:1b,PersistenceRequired:1b,Sleeping:1b,Sitting:1b,Crouching:1b}");
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
        if material == "lava" {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("effect give {uuid} minecraft:fire_resistance 600"),
            );
        }
        if !no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:0b}}"),
            );
        }
        let start = world.stored_entity_for_test(DIM, id).unwrap().position();
        assert_eq!(
            flags(world.stored_entity_for_test(DIM, id).unwrap()) & 37,
            37
        );
        let (mut contact, mut rose, mut cleared) = (false, false, false);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..160 {
            world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.is_alive_with_health());
            assert!(
                matches!(body.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
            );
            contact |= match material {
                "water" => body.is_in_water(),
                "lava" => body.is_in_lava(),
                _ => body.is_in_powder_snow() || body.was_in_powder_snow(),
            };
            cleared |= flags(body) & 37 == 0;
            let needed = if material == "powder_snow" {
                1.05
            } else {
                0.75
            };
            rose |= body.position().y > start.y + needed;
            if no_ai {
                assert!((body.position().y - start.y).abs() < 0.01);
            } else if contact && rose && (material == "powder_snow" || cleared) {
                break;
            }
        }
        if no_ai {
            assert!(!rose && !cleared);
        } else {
            assert!(
                contact && rose,
                "{material}: contact={contact}, rose={rose}"
            );
            if material != "powder_snow" {
                assert!(cleared);
            }
        }
    }
}
