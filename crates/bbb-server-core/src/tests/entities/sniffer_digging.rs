use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_SNIFFER_DIGGING_SOUND_EVENT_ID, VANILLA_SNIFFER_STATE_DATA_ID,
    VANILLA_SNIFFER_STATE_DIGGING_ID, VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID,
    VANILLA_SNIFFER_STATE_SNIFFING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
// EntityType.java:906-913: the normal Sniffer passenger attachment is explicit.
const NORMAL_PASSENGER_HEIGHT: f64 = 2.09375;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let tick = console_command(sender, receiver, world, 40, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
}

fn state_update(tick: &TickResult, id: i32) -> Option<i32> {
    for event in &tick.events {
        let WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(data),
            ..
        } = event
        else {
            continue;
        };
        if data.id != id {
            continue;
        }
        for row in &data.values {
            if row.data_id == VANILLA_SNIFFER_STATE_DATA_ID {
                if let EntityDataValueKind::EnumId { id, .. } = row.value {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn memory_record(world: &ServerWorld, id: i32, name: &str) -> NbtSummaryValue {
    let data = world.stored_entity_command_data_for_test(DIM, id).unwrap();
    let brain = crate::loot_runtime::nbt_field(&data, "Brain").expect("saved Brain");
    let memories = crate::loot_runtime::nbt_field(brain, "memories").expect("saved memories");
    crate::loot_runtime::nbt_field(memories, name)
        .expect("saved memory")
        .clone()
}

fn cooldown(world: &ServerWorld, id: i32) -> i64 {
    let memory = memory_record(world, id, "minecraft:sniff_cooldown");
    let Some(NbtSummaryValue::Long(ttl)) = crate::loot_runtime::nbt_field(&memory, "ttl") else {
        panic!("saved cooldown lifetime");
    };
    *ttl
}

fn assert_passenger_height(world: &ServerWorld, sniffer: i32, chicken: i32, height: f64) {
    assert_eq!(world.stored_passengers_for_test(DIM, sniffer), &[chicken]);
    let vehicle = world.stored_entity_for_test(DIM, sniffer).unwrap();
    let passenger = world.stored_entity_for_test(DIM, chicken).unwrap();
    let actual = passenger.position().y - vehicle.position().y;
    assert!(
        (actual - height).abs() < 1.0e-5,
        "passenger height {actual}, expected {height}"
    );
}

#[test]
fn sniffer_digging_draws_gift_loot_without_mob_drops_and_saves_explored_ground() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 30_000;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let table = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .iter_mut()
        .find(|entry| entry.id == "minecraft:gameplay/sniffer_digging")
        .unwrap();
    table.value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type":"minecraft:gift", "pools":[{"rolls":1,
                "conditions":[{"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:sniffer"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:diamond","functions":[{"function":"minecraft:set_count","count":2}]}]
            }]
        }), None).unwrap()
    );
    let root = std::env::temp_dir().join(format!("bbb-sniffer-digging-{}", uuid::Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Sniffer digging");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamerule mob_drops false",
        "gamerule spawn_mobs false",
        "gamemode spectator observer",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let origin = world.publication().players[0].position;
    let dirt = block_state_id("minecraft:dirt");
    for dx in -20..=25 {
        for dz in -20..=20 {
            world.set_raw_block_state_for_test(
                DIM,
                BlockPos {
                    x: origin.x.floor() as i32 + dx,
                    y: origin.y.floor() as i32 - 1,
                    z: origin.z.floor() as i32 + dz,
                },
                dirt,
            );
        }
    }
    command(
        &sender,
        &receiver,
        &mut world,
        &format!(
            "summon minecraft:sniffer {} {} {} {{PersistenceRequired:1b,Brain:{{memories:{{\"minecraft:sniff_cooldown\":{{value:{{}},ttl:120L}}}}}},Passengers:[{{id:chicken,UUID:[I;0,0,0,43],NoAI:1b,NoGravity:1b}}]}}",
            origin.x + 6.0,
            origin.y,
            origin.z
        ),
    );
    let sniffer = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_SNIFFER_ID
        })
        .unwrap();
    let body = world.stored_entity_for_test(DIM, sniffer).unwrap();
    let uuid = body.uuid();
    let normal_height = body.bounding_box().max_y - body.bounding_box().min_y;
    let chicken_uuid = Uuid::from_u128(43);
    let chicken = world
        .live_entity_id_for_uuid_for_test(DIM, chicken_uuid)
        .unwrap();
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..60 {
        world.tick(receiver.drain());
    }
    assert!((1..120).contains(&cooldown(&world, sniffer)));
    assert!(world.stored_items_for_test(DIM).is_empty());
    assert_passenger_height(&world, sniffer, chicken, NORMAL_PASSENGER_HEIGHT);
    let mut sniffed = false;
    let mut digging_event = false;
    let mut dug = false;
    for _ in 0..3500 {
        let tick = world.tick(receiver.drain());
        sniffed |= state_update(&tick, sniffer) == Some(VANILLA_SNIFFER_STATE_SNIFFING_ID);
        digging_event |= tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. } if event.entity_id == sniffer && event.event_id == VANILLA_SNIFFER_DIGGING_SOUND_EVENT_ID));
        if state_update(&tick, sniffer) == Some(VANILLA_SNIFFER_STATE_DIGGING_ID) {
            dug = true;
            break;
        }
    }
    assert!(
        sniffed && dug && digging_event,
        "sniffed={sniffed}, digging={dug}, event={digging_event}"
    );
    let body = world.stored_entity_for_test(DIM, sniffer).unwrap();
    let digging_height = body.bounding_box().max_y - body.bounding_box().min_y;
    assert!((normal_height - digging_height - 0.4).abs() < 1.0e-5);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    world.tick(receiver.drain());
    assert_passenger_height(&world, sniffer, chicken, digging_height);
    let mut gift = None;
    for _ in 0..130 {
        world.tick(receiver.drain());
        gift = world
            .stored_items_for_test(DIM)
            .into_iter()
            .find(|item| item.2 == "minecraft:diamond");
        if gift.is_some() {
            break;
        }
    }
    let gift = gift.expect("the digging deadline drops the data pack gift while NoAI");
    assert_eq!(gift.3, 2);
    assert!(gift.4 > 0);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let mut completed = false;
    for _ in 0..300 {
        let tick = world.tick(receiver.drain());
        if state_update(&tick, sniffer) == Some(VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID) {
            completed = true;
            break;
        }
    }
    assert!(completed, "the sniffer rises and finishes digging");
    world.tick(receiver.drain());
    assert_passenger_height(&world, sniffer, chicken, NORMAL_PASSENGER_HEIGHT);
    let explored = memory_record(&world, sniffer, "minecraft:sniffer_explored_positions");
    let saved_cooldown = cooldown(&world, sniffer);
    assert!((9000..9600).contains(&saved_cooldown));
    let Some(NbtSummaryValue::List(positions)) = crate::loot_runtime::nbt_field(&explored, "value")
    else {
        panic!("saved explored positions");
    };
    assert_eq!(positions.len(), 1);
    assert_eq!(
        crate::loot_runtime::nbt_field(&positions[0], "dimension"),
        Some(&NbtSummaryValue::String("minecraft:overworld".into()))
    );
    let body = world.stored_entity_for_test(DIM, sniffer).unwrap();
    assert!((body.bounding_box().max_y - body.bounding_box().min_y - normal_height).abs() < 1.0e-5);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(config, &candidate);
    let id = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(
        memory_record(&restored, id, "minecraft:sniffer_explored_positions"),
        explored
    );
    assert_eq!(cooldown(&restored, id), saved_cooldown);
    join_loaded(&sender, &receiver, &mut restored, 1, "observer", 1);
    for _ in 0..600 {
        let tick = restored.tick(receiver.drain());
        assert_ne!(
            state_update(&tick, id),
            Some(VANILLA_SNIFFER_STATE_DIGGING_ID)
        );
    }
    assert!(cooldown(&restored, id) < saved_cooldown);
    assert_eq!(
        restored
            .stored_items_for_test(DIM)
            .iter()
            .filter(|item| item.2 == "minecraft:diamond" && item.3 == 2)
            .count(),
        1
    );
    let chicken = restored
        .live_entity_id_for_uuid_for_test(DIM, chicken_uuid)
        .unwrap();
    assert_passenger_height(&restored, id, chicken, NORMAL_PASSENGER_HEIGHT);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
