//! Natural Panda sneezes, adult reactions and datapack gift results.

use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_ITEM_ENTITY_ITEM_DATA_ID, VANILLA_PANDA_FLAGS_DATA_ID,
    VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
use bbb_protocol::item_types::VANILLA_ITEM_SLIME_BALL_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

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

fn data_event(tick: &TickResult, id: i32, predicate: impl Fn(&EntityDataValue) -> bool) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(&predicate))
    })
}

fn sneeze_event(tick: &TickResult, id: i32, sneezing: bool) -> bool {
    data_event(tick, id, |value| {
        value.data_id == VANILLA_PANDA_FLAGS_DATA_ID
            && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 2 != 0) == sneezing)
    })
}

#[test]
fn baby_panda_sneezes_make_adults_jump_and_honor_mob_drops_with_real_gift_components() {
    let table = serde_json::json!({
        "type": "minecraft:gift",
        "pools": [{"rolls": 1, "conditions": [
            {"condition": "minecraft:entity_properties", "entity": "this", "predicate": {
                "type": "minecraft:panda", "flags": {"is_baby": true}
            }},
            {"condition": "minecraft:location_check", "predicate": {"position": {
                "x": {"min": 3.0, "max": 6.0}, "y": {"min": 64.0, "max": 68.0},
                "z": {"min": 3.0, "max": 6.0}
            }}}
        ], "entries": [{"type": "minecraft:item", "name": "minecraft:slime_ball", "functions": [
            {"function": "minecraft:set_components", "components": {
                "minecraft:custom_data": {"sneeze_gift": "panda"}
            }}
        ]}]}]
    });
    let mut config = datapack_config_with_loot_table("minecraft:gameplay/panda_sneeze", &table);
    config.game_type = GameType::Creative;
    config.keep_alive_interval_ticks = 100_000;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for line in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule random_tick_speed 0",
        "gamerule mob_drops false",
        "time set midnight",
        "weather clear",
        "tp observer 12.5 65.0 12.5",
        "fill 0 64 0 15 75 15 minecraft:stone",
        "fill 0 65 0 15 75 15 minecraft:air",
        "fill 3 65 3 6 68 6 minecraft:stone",
        "fill 4 65 4 5 68 5 minecraft:air",
    ] {
        command(&sender, &receiver, &mut world, line);
    }
    let mut pandas = Vec::new();
    for (index, (x, z, age, gene)) in [
        (4.5, 4.5, -48_000, "weak"),
        (12.5, 4.5, -48_000, "weak"),
        (9.5, 4.5, 0, "weak"),
        (4.5, 9.5, 0, "normal"),
    ]
    .into_iter()
    .enumerate()
    {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "summon minecraft:panda {x} 65.0 {z} {{Age:{age},MainGene:\"{gene}\",HiddenGene:\"{gene}\",NoAI:1b,PersistenceRequired:1b}}"
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
        if index >= 2 {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:0b}}"),
            );
        }
        pandas.push((id, uuid));
    }
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..20 {
        world.tick(receiver.drain());
        if pandas[2..]
            .iter()
            .all(|(id, _)| world.stored_entity_for_test(DIM, *id).unwrap().on_ground())
        {
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    let adult_y: Vec<_> = pandas[2..]
        .iter()
        .map(|(id, _)| {
            let body = world.stored_entity_for_test(DIM, *id).unwrap();
            assert!(body.on_ground());
            body.position().y
        })
        .collect();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {} {{NoAI:0b}}", pandas[0].1),
    );
    let custom_data = Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "sneeze_gift".into(),
        value: NbtSummaryValue::String("panda".into()),
    }]));
    let id = pandas[0].0;
    for drops in [false, true] {
        if drops {
            world.tick_rate_mut().set_frozen(true);
            command(&sender, &receiver, &mut world, "gamerule mob_drops true");
        }
        let (mut started, mut progressed, mut reset, mut finished) = (false, false, false, false);
        let mut jumped = [false; 2];
        let mut gift_sync = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..12_000 {
            let tick = world.tick(receiver.drain());
            for (index, (other_id, uuid)) in pandas.iter().enumerate() {
                let body = world.stored_entity_for_test(DIM, *other_id).unwrap();
                assert_eq!(body.uuid(), *uuid);
                assert!(body.living().unwrap().health() > 0.0);
                assert!(matches!(body.data(), EntityData::GeneralLiving(data)
                    if data.is_no_ai() == (index == 1) && data.is_baby() == (index < 2)));
                if index > 0 {
                    assert!(
                        !sneeze_event(&tick, *other_id, true),
                        "negative Panda sneezed"
                    );
                }
                if index >= 2 && finished {
                    jumped[index - 2] |= body.position().y > adult_y[index - 2] + 0.1;
                }
            }
            started |= sneeze_event(&tick, id, true);
            progressed |= data_event(&tick, id, |value| {
                value.data_id == VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID
                    && matches!(value.value, EntityDataValueKind::Int(counter) if counter > 0)
            });
            reset |= progressed
                && data_event(&tick, id, |value| {
                    value.data_id == VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Int(0))
                });
            finished |= started && progressed && sneeze_event(&tick, id, false);
            let items = world.stored_items_for_test(DIM);
            if !drops {
                assert!(items.is_empty(), "gift dropped with mob_drops=false");
                if finished && reset && jumped.into_iter().all(|value| value) {
                    break;
                }
            } else if let Some((item_id, _, item, count, _, _)) = items.first() {
                assert_eq!(item, "minecraft:slime_ball");
                assert_eq!(*count, 1);
                let stack = world.stored_item_stack_for_test(DIM, *item_id).unwrap();
                assert_eq!(stack.summary().component_patch.custom_data, custom_data);
                gift_sync |= data_event(&tick, *item_id, |value| {
                    value.data_id == VANILLA_ITEM_ENTITY_ITEM_DATA_ID
                        && matches!(&value.value, EntityDataValueKind::ItemStack(stack)
                            if stack.item_id == Some(VANILLA_ITEM_SLIME_BALL_ID) && stack.count == 1
                                && stack.component_patch.custom_data == custom_data)
                });
                if finished && reset && gift_sync {
                    break;
                }
            }
        }
        assert!(
            started && progressed && reset && finished,
            "drops={drops}: incomplete sneeze"
        );
        if drops {
            assert!(gift_sync, "missing actual gift tracking");
        } else {
            assert!(
                jumped.into_iter().all(|value| value),
                "adult reactions={jumped:?}"
            );
        }
    }
}
