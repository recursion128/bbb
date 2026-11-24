//! Fox shelter seeking through actual commands, weather, movement and sky light.

use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;

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

#[test]
fn exposed_foxes_seek_storm_shelter_with_no_ai_and_clear_night_controls() {
    for (storm, no_ai) in [(true, false), (true, true), (false, false)] {
        let expected = storm && !no_ai;
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
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        world.with_level_for_test(DIM, |level| {
            for (x, z) in [(0, 0), (0, 31), (31, 0), (31, 31)] {
                assert!(level.has_ticking_column_at(BlockPos { x, y: 65, z }));
            }
        });
        for line in [
            "fill 0 64 0 31 75 31 minecraft:stone",
            "fill 0 65 0 31 75 31 minecraft:air",
            "fill 0 67 0 31 67 31 minecraft:stone",
            "setblock 16 67 16 minecraft:air",
            "summon minecraft:fox 16.5 65.0 16.5 {NoAI:1b,PersistenceRequired:1b}",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
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
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..20 {
                world.tick(receiver.drain());
                if world.stored_entity_for_test(DIM, id).unwrap().on_ground() {
                    break;
                }
            }
            world.tick_rate_mut().set_frozen(true);
            assert!(world.stored_entity_for_test(DIM, id).unwrap().on_ground());
        }
        if storm {
            command(&sender, &receiver, &mut world, "weather thunder");
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..200 {
                world.tick(receiver.drain());
                if world.with_level_for_test(DIM, |level| level.is_thundering()) {
                    break;
                }
            }
            world.tick_rate_mut().set_frozen(true);
            assert!(world.with_level_for_test(DIM, |level| level.is_thundering()));
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0.2"),
        );
        let start = world.stored_entity_for_test(DIM, id).unwrap().position();
        let feet = world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .block_position();
        assert!(world.with_level_for_test(DIM, |level| level.can_see_sky(feet)));
        let (mut moved, mut covered, mut fast) = (false, false, false);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..600 {
            world.tick(receiver.drain());
            let (at, feet, speed) = {
                let body = world.stored_entity_for_test(DIM, id).unwrap();
                assert_eq!(body.uuid(), uuid);
                assert!(body.living().unwrap().health() > 0.0);
                (
                    body.position(),
                    body.block_position(),
                    body.living().unwrap().speed(),
                )
            };
            let distance = (at.x - start.x).hypot(at.z - start.z);
            moved |= distance > 0.5;
            covered |= !world.with_level_for_test(DIM, |level| level.can_see_sky(feet));
            fast |= speed > 0.225;
            if no_ai {
                assert!(distance < 0.01, "NoAI fox moved: {at:?}");
            } else if !storm {
                assert!(speed <= 0.21, "clear-night shelter speed: {speed}");
            }
            if expected && moved && covered && fast {
                break;
            }
        }
        if expected {
            assert!(
                moved && covered && fast,
                "moved={moved},covered={covered},fast={fast}"
            );
        } else {
            assert!(!fast, "storm={storm},NoAI={no_ai}");
        }
    }
}
