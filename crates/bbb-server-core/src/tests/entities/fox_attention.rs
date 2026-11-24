//! Natural fox surveying and player gaze with disabled-AI controls.

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

fn sitting(body: &crate::entity::Entity) -> bool {
    crate::entity::tracking::synched_values(body)
        .into_iter()
        .find_map(|value| match value.value.value {
            EntityDataValueKind::Byte(flags)
                if value.value.data_id == VANILLA_FOX_FLAGS_DATA_ID =>
            {
                Some(flags & 1 != 0)
            }
            _ => None,
        })
        .unwrap()
}

fn sitting_event(tick: &TickResult, id: i32, sitting: bool) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(|value|
            value.data_id == VANILLA_FOX_FLAGS_DATA_ID
                && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 1 != 0) == sitting))))
}

#[test]
fn foxes_sit_survey_stand_and_follow_player_gaze_with_no_ai_controls() {
    for (no_ai, watch_player) in [(false, false), (true, false), (false, true), (true, true)] {
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
            "tp observer 30.5 65.0 4.5",
            "forceload add 0 0 31 31",
            "fill 0 64 0 31 75 31 minecraft:stone",
            "fill 0 65 0 31 75 31 minecraft:air",
            "summon minecraft:fox 4.5 65.0 4.5 {PersistenceRequired:1b}",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        if !watch_player {
            for line in [
                "fill 3 65 3 5 67 5 minecraft:stone",
                "fill 4 65 4 4 67 4 minecraft:air",
            ] {
                command(&sender, &receiver, &mut world, line);
            }
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
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..20 {
            world.tick(receiver.drain());
            if world.stored_entity_for_test(DIM, id).unwrap().on_ground() {
                break;
            }
        }
        world.tick_rate_mut().set_frozen(true);
        assert!(world.stored_entity_for_test(DIM, id).unwrap().on_ground());
        if !watch_player {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("attribute {uuid} minecraft:movement_speed base set 0.2"),
            );
        }
        if no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:1b,Sitting:0b}}"),
            );
        }
        assert!(
            matches!(world.stored_entity_for_test(DIM, id).unwrap().data(),
            crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        if watch_player {
            for (x, z) in [(19.5_f64, 4.5_f64), (4.5, 19.5)] {
                command(
                    &sender,
                    &receiver,
                    &mut world,
                    &format!("tp observer {x} 65.0 {z}"),
                );
                let mut aligned = 0;
                let mut watched = false;
                world.tick_rate_mut().set_frozen(false);
                for _ in 0..1200 {
                    world.tick(receiver.drain());
                    let body = world.stored_entity_for_test(DIM, id).unwrap();
                    assert_eq!(body.uuid(), uuid);
                    assert!(body.living().unwrap().health() > 0.0);
                    let at = body.position();
                    let dx = x - at.x;
                    let dz = z - at.z;
                    let yaw = (dz.atan2(dx).to_degrees() - 90.0) as f32;
                    let eye_delta = 65.0 + 1.62 - at.y - f64::from(body.eye_height());
                    let pitch = (-eye_delta.atan2(dx.hypot(dz)).to_degrees()) as f32;
                    // A same-height idle survey cannot produce this player-eye pitch.
                    aligned = if crate::entity::living::wrap_degrees(body.y_head_rot() - yaw).abs()
                        < 0.5
                        && (body.x_rot() - pitch).abs() < 0.5
                    {
                        aligned + 1
                    } else {
                        0
                    };
                    watched |= aligned >= 3;
                    if watched && !no_ai {
                        break;
                    }
                }
                world.tick_rate_mut().set_frozen(true);
                assert_eq!(watched, !no_ai, "NoAI={no_ai}, player=({x},{z})");
            }
        } else {
            let (mut sat, mut surveyed, mut stood) = (false, false, false);
            let mut first_yaw = None;
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..2400 {
                let tick = world.tick(receiver.drain());
                let body = world.stored_entity_for_test(DIM, id).unwrap();
                assert_eq!(body.uuid(), uuid);
                assert!(body.living().unwrap().health() > 0.0);
                let seated = sitting(body);
                if no_ai {
                    assert!(!seated, "NoAI fox sat down");
                    continue;
                }
                if seated && sitting_event(&tick, id, true) {
                    sat = true;
                }
                if sat && seated {
                    let first = *first_yaw.get_or_insert(body.y_head_rot());
                    surveyed |=
                        crate::entity::living::wrap_degrees(body.y_head_rot() - first).abs() > 15.0;
                }
                stood |= surveyed && !seated && sitting_event(&tick, id, false);
                if sat && surveyed && stood {
                    break;
                }
            }
            if !no_ai {
                assert!(
                    sat && surveyed && stood,
                    "sit={sat},survey={surveyed},stand={stood}"
                );
            }
        }
    }
}
