//! Fox offspring follow adults through actual commands and entity ticks.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_FOX_FLAGS_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_server_types::geometry::Vec3Ops;

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
fn baby_fox_follows_an_adult_and_clears_rest_posture_with_no_ai_control() {
    for no_ai in [false, true] {
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
            "tp observer 28.5 65.0 28.5",
            "forceload add 0 0 31 31",
            "fill 0 64 0 31 75 31 minecraft:stone",
            "fill 0 65 0 31 75 31 minecraft:air",
            "summon minecraft:fox 10.5 65.0 16.5 {Age:-24000,NoAI:1b,PersistenceRequired:1b}",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        let baby_id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID)
            })
            .unwrap();
        let baby_uuid = world.stored_entity_for_test(DIM, baby_id).unwrap().uuid();
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {baby_uuid} minecraft:movement_speed base set 0"),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {baby_uuid} {{NoAI:0b}}"),
        );
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..20 {
            world.tick(receiver.drain());
            if world
                .stored_entity_for_test(DIM, baby_id)
                .unwrap()
                .on_ground()
            {
                break;
            }
        }
        world.tick_rate_mut().set_frozen(true);
        assert!(world
            .stored_entity_for_test(DIM, baby_id)
            .unwrap()
            .on_ground());
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "data merge entity {baby_uuid} {{NoAI:{}b,Sitting:1b,Crouching:1b}}",
                i32::from(no_ai)
            ),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {baby_uuid} minecraft:movement_speed base set 0.2"),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            "summon minecraft:fox 16.5 65.0 16.5 {Age:0,NoAI:1b,PersistenceRequired:1b}",
        );
        let adult_id = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                *id != baby_id
                    && world
                        .stored_entity_for_test(DIM, *id)
                        .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID)
            })
            .unwrap();
        let adult = world.stored_entity_for_test(DIM, adult_id).unwrap();
        let adult_uuid = adult.uuid();
        let destination = adult.position();
        let baby = world.stored_entity_for_test(DIM, baby_id).unwrap();
        let start = baby.position();
        assert_eq!(flags(baby) & 5, 5);
        assert!(start.distance_to_sqr(destination) >= 25.0);
        let mut approached = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..160 {
            world.tick(receiver.drain());
            let adult = world.stored_entity_for_test(DIM, adult_id).unwrap();
            assert_eq!(adult.uuid(), adult_uuid);
            assert!(adult.is_alive() && adult.living().unwrap().health() > 0.0);
            assert_eq!(adult.position(), destination);
            let baby = world.stored_entity_for_test(DIM, baby_id).unwrap();
            assert_eq!(baby.uuid(), baby_uuid);
            assert!(baby.is_alive() && baby.living().unwrap().health() > 0.0);
            assert!(
                matches!(baby.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.is_no_ai() == no_ai)
            );
            if no_ai {
                assert!(baby.position().distance_to_sqr(start) < 0.0001);
                assert_eq!(flags(baby) & 5, 5);
            } else if baby.position().distance_to_sqr(destination) < 3.5 * 3.5 {
                assert_eq!(flags(baby) & (1 | 4 | 32), 0);
                approached = true;
                break;
            }
        }
        assert_eq!(approached, !no_ai, "NoAI={no_ai}");
    }
}
