//! Fox night travel follows an actual occupied village POI.

use super::*;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const BED: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

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

fn actor(world: &ServerWorld, kind: i32) -> i32 {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == kind
        })
        .unwrap()
}

#[test]
fn a_fox_travels_across_sections_toward_an_occupied_village_at_night_with_no_ai_control() {
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
            "forceload add -32 -32 128 48",
            "fill -16 64 -16 112 68 32 minecraft:stone",
            "fill -16 65 -16 112 68 32 minecraft:air",
            // Walls keep a wandering fox off the generated terrain outside.
            "fill -16 65 -16 112 67 -16 minecraft:barrier",
            "fill -16 65 32 112 67 32 minecraft:barrier",
            "fill -16 65 -16 -16 67 32 minecraft:barrier",
            "fill 112 65 -16 112 67 32 minecraft:barrier",
            "tp observer 96.5 65.0 28.5",
            "setblock 7 65 8 minecraft:red_bed[part=foot,facing=east]",
            "setblock 8 65 8 minecraft:red_bed[part=head,facing=east]",
            "summon minecraft:villager 8.5 65.0 10.5 {PersistenceRequired:1b}",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        let resident = actor(&world, VANILLA_ENTITY_TYPE_VILLAGER_ID);
        let resident_uuid = world.stored_entity_for_test(DIM, resident).unwrap().uuid();
        let mut slept = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..1200 {
            world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, resident).unwrap();
            let sleeping = body.sleeping_pos() == Some(BED)
                && body.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID;
            let occupied = world.block_state(DIM, BED).and_then(|state| {
                bbb_server_types::block_queries::bool_property(state, "occupied")
            });
            if sleeping && occupied == Some(true) {
                slept = true;
                break;
            }
        }
        assert!(
            slept,
            "actual resident sleeps in and occupies the village bed"
        );
        let dimension = level_key(DIM);
        assert_eq!(
            world
                .poi_record(&dimension, [8, 65, 8])
                .unwrap()
                .free_tickets,
            0
        );
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {resident_uuid} {{NoAI:1b}}"),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            "summon minecraft:fox 72.5 65.0 8.5 {NoAI:1b,PersistenceRequired:1b}",
        );
        let id = actor(&world, VANILLA_ENTITY_TYPE_FOX_ID);
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0"),
        );
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
        assert!(world.stored_entity_for_test(DIM, id).unwrap().on_ground());
        world.tick_rate_mut().set_frozen(true);
        if no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:1b}}"),
            );
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0.2"),
        );
        let start = world.stored_entity_for_test(DIM, id).unwrap().position();
        assert_eq!(world.sections_to_village(&dimension, [4, 4, 0]), Some(4));
        let mut reached = false;
        let mut best_distance = f64::INFINITY;
        world.tick_rate_mut().set_frozen(false);
        // The village stroll starts about once per 100 goal ticks and a fox
        // spends some of the night sitting, so the walk is given several runs.
        for _ in 0..if no_ai { 3_600 } else { 36_000 } {
            world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.is_alive_with_health());
            assert!(
                matches!(body.data(),crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai()==no_ai)
            );
            let position = body.position();
            let distance = (position.x - 8.5).hypot(position.z - 8.5);
            best_distance = best_distance.min(distance);
            if no_ai {
                assert_eq!(position, start);
            } else if distance < 24.0 && position.x.floor() as i32 >> 4 <= 1 {
                reached = true;
                break;
            }
        }
        assert_eq!(
            reached, !no_ai,
            "NoAI={no_ai},best_distance={best_distance}"
        );
        assert_eq!(
            world
                .poi_record(&dimension, [8, 65, 8])
                .unwrap()
                .free_tickets,
            0
        );
        assert_eq!(
            world.stored_entity_for_test(DIM, resident).unwrap().uuid(),
            resident_uuid
        );
    }
}
