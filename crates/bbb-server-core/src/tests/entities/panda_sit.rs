//! Autonomous Panda sitting, food seeking, and water interruption.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_PANDA_FLAGS_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_PANDA_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

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

fn held(world: &ServerWorld, id: i32) -> Option<RuntimeItemStack> {
    world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::MainHand)
        .cloned()
}

fn sitting(world: &ServerWorld, id: i32) -> bool {
    matches!(world.stored_entity_for_test(DIM, id).unwrap().data(), EntityData::GeneralLiving(data)
        if data.panda_retained().unwrap().is_sitting())
}

fn sitting_event(tick: &TickResult, id: i32, sitting: bool) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        | WorldEvent::Broadcast { packet: ClientboundPlayPacket::SetEntityData(packet), .. }
        if packet.id == id && packet.values.iter().any(|value|
            value.data_id == VANILLA_PANDA_FLAGS_DATA_ID
                && matches!(value.value, EntityDataValueKind::Byte(flags) if (flags & 8 != 0) == sitting))))
}

fn setup(
    no_ai: bool,
    carrying: bool,
    seeking: bool,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
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
    if !seeking {
        command(
            &sender,
            &receiver,
            &mut world,
            "gamerule mob_griefing false",
        );
    }
    let equipment = if carrying {
        r#",equipment:{mainhand:{id:"minecraft:bamboo",count:7,components:{"minecraft:custom_data":{marker:17}}}}"#
    } else {
        ""
    };
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            r#"summon minecraft:panda 4.5 65.0 4.5 {{Age:0,MainGene:"lazy",HiddenGene:"normal",NoAI:1b,CanPickUpLoot:1b,PersistenceRequired:1b{equipment}}}"#
        ),
    );
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_PANDA_ID)
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
    if !seeking {
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0"),
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
    if seeking {
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
    assert!(!sitting(&world, id));
    assert_eq!(held(&world, id).is_some(), carrying);
    assert!(
        matches!(world.stored_entity_for_test(DIM, id).unwrap().data(), EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
    );
    (sender, receiver, world, id, uuid)
}

#[test]
fn held_food_starts_sitting_and_water_drops_the_whole_stack_before_a_cooldown() {
    for (no_ai, carrying) in [(false, true), (true, true), (false, false)] {
        let expected = !no_ai && carrying;
        let (sender, receiver, mut world, id, uuid) = setup(no_ai, carrying, false);
        let original = held(&world, id);
        let mut sat = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..60 {
            let tick = world.tick(receiver.drain());
            sat |= sitting_event(&tick, id, true);
            if expected && sat {
                break;
            }
        }
        assert_eq!(sat, expected, "NoAI={no_ai},carrying={carrying}");
        if !expected {
            continue;
        }
        let original = original.unwrap();
        assert_eq!(original.count(), 7);
        assert!(original.summary().component_patch.custom_data.is_some());
        assert_eq!(held(&world, id), Some(original.clone()));
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            "fill 3 65 3 5 67 5 minecraft:water",
        );
        world.tick_rate_mut().set_frozen(false);
        let mut water = false;
        let mut stood = false;
        for _ in 0..20 {
            let tick = world.tick(receiver.drain());
            water |= world.stored_entity_for_test(DIM, id).unwrap().is_in_water();
            stood |= sitting_event(&tick, id, false);
            if water && stood && held(&world, id).is_none() {
                break;
            }
        }
        assert!(water && stood && held(&world, id).is_none());
        let drops: Vec<_> = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
                EntityData::Item(item) => Some(item.item().clone()),
                _ => None,
            })
            .collect();
        assert_eq!(drops, vec![original]);
        world.tick_rate_mut().set_frozen(true);
        command(
            &sender,
            &receiver,
            &mut world,
            "fill 0 65 0 15 75 15 minecraft:air",
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("item replace entity {uuid} weapon.mainhand with minecraft:bamboo 3"),
        );
        world.tick_rate_mut().set_frozen(false);
        let mut dry = false;
        for _ in 0..100 {
            let tick = world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIM, id).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.living().unwrap().health() > 0.0);
            dry |= !body.is_in_water();
            assert!(!sitting(&world, id) && !sitting_event(&tick, id, true));
            assert_eq!(held(&world, id).unwrap().count(), 3);
        }
        assert!(dry);
    }
}

#[test]
fn an_empty_handed_panda_approaches_ground_food_picks_it_up_and_sits() {
    let (sender, receiver, mut world, id, uuid) = setup(false, false, true);
    command(
        &sender,
        &receiver,
        &mut world,
        r#"summon minecraft:item 7.5 65.0 4.5 {Item:{id:"minecraft:bamboo",count:7,components:{"minecraft:custom_data":{marker:23}}},PickupDelay:0s}"#,
    );
    let food = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .is_some_and(|body| body.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID)
        })
        .unwrap();
    let EntityData::Item(item) = world.stored_entity_for_test(DIM, food).unwrap().data() else {
        unreachable!()
    };
    let original = item.item().clone();
    let start = world.stored_entity_for_test(DIM, id).unwrap().position();
    let mut sat = false;
    let mut approached = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..400 {
        let tick = world.tick(receiver.drain());
        sat |= sitting_event(&tick, id, true);
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.uuid(), uuid);
        assert!(body.living().unwrap().health() > 0.0);
        let at = body.position();
        approached |= (at.x - start.x).hypot(at.z - start.z) > 0.5;
        if sat && approached && held(&world, id).is_some() {
            break;
        }
    }
    assert!(sat && approached, "sat={sat},approached={approached}");
    assert!(world.stored_entity_for_test(DIM, food).is_none());
    assert_eq!(held(&world, id), Some(original));
}
