//! Natural fox hunting with a disabled-AI control.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_FOX_FLAGS_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_FOX_ID};
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
fn fox_naturally_stalks_charges_pounces_and_hurts_prey_with_no_ai_control() {
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
            "summon minecraft:chicken 20.5 65.0 16.5 {NoAI:1b,PersistenceRequired:1b}",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        command(
            &sender,
            &receiver,
            &mut world,
            if no_ai {
                "summon minecraft:fox 11.5 65.0 16.5 {NoAI:1b,PersistenceRequired:1b}"
            } else {
                "summon minecraft:fox 11.5 65.0 16.5 {PersistenceRequired:1b}"
            },
        );
        let id_of = |kind| {
            world
                .stored_entity_ids_for_test(DIM)
                .into_iter()
                .find(|id| {
                    world
                        .stored_entity_for_test(DIM, *id)
                        .is_some_and(|body| body.entity_type_id() == kind)
                })
                .unwrap()
        };
        let fox_id = id_of(VANILLA_ENTITY_TYPE_FOX_ID);
        let prey_id = id_of(VANILLA_ENTITY_TYPE_CHICKEN_ID);
        let fox_uuid = world.stored_entity_for_test(DIM, fox_id).unwrap().uuid();
        let prey_uuid = world.stored_entity_for_test(DIM, prey_id).unwrap().uuid();
        for line in [
            format!("attribute {prey_uuid} minecraft:max_health base set 1000"),
            format!("data merge entity {prey_uuid} {{Health:1000.0f}}"),
        ] {
            command(&sender, &receiver, &mut world, &line);
        }
        assert!(
            matches!(world.stored_entity_for_test(DIM, fox_id).unwrap().data(), crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        let start = world
            .stored_entity_for_test(DIM, fox_id)
            .unwrap()
            .position();
        let initial_hp = world
            .stored_entity_for_test(DIM, prey_id)
            .unwrap()
            .living()
            .unwrap()
            .health();
        assert!(initial_hp > 100.0);
        let (
            mut approached,
            mut charged,
            mut pounced,
            mut rose,
            mut forward,
            mut pitched,
            mut hurt,
            mut cleaned,
        ) = (false, false, false, false, false, false, false, false);
        let mut launch = start;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..600 {
            world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, fox_id).unwrap();
            assert_eq!(body.uuid(), fox_uuid);
            assert!(body.living().unwrap().health() > 0.0);
            let at = body.position();
            let pose = flags(body);
            let prey = world.stored_entity_for_test(DIM, prey_id).unwrap();
            assert_eq!(prey.uuid(), prey_uuid);
            let hp = prey.living().unwrap().health();
            assert!(hp > 0.0);
            if no_ai {
                assert_eq!(pose & (4 | 8 | 16), 0);
                assert_eq!(hp, initial_hp);
                continue;
            }
            approached |= at.x > start.x + 0.5;
            charged |= pose & (4 | 8) == (4 | 8);
            if pose & 16 != 0 {
                assert!(charged, "pounce without observed charge");
                if !pounced {
                    launch = at;
                }
                pounced = true;
                rose |= at.y > start.y + 0.5;
                forward |= at.x > launch.x + 0.5;
                pitched |= body.x_rot().abs() > 15.0;
            }
            hurt |= hp < initial_hp;
            cleaned |= pounced && pose & (4 | 8 | 16) == 0;
            if approached && charged && pounced && rose && forward && pitched && hurt && cleaned {
                break;
            }
        }
        if !no_ai {
            assert!(approached && charged && pounced && rose && forward && pitched && hurt && cleaned,
                "approach={approached},charge={charged},pounce={pounced},rise={rose},forward={forward},pitch={pitched},hurt={hurt},cleanup={cleaned}");
        }
    }
}
