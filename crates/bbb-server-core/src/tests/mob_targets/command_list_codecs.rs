use super::*;
use crate::tests::entities::{console_command, restore_entity_writes};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_COW_ID,
};

fn fixture() -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let mut candidate = restore_candidate(100, None, Vec::new());
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    let (sender, receiver) = mailbox();
    (world, sender, receiver)
}

fn command(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    text: &str,
) -> TickResult {
    static NEXT_OPERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let operation = NEXT_OPERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tick = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert_eq!(
        report.result,
        Some(if text == "tick unfreeze" { 0 } else { 1 }),
        "{text}: {report:?}"
    );
    tick
}

fn cow(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    x: f64,
) -> (i32, Uuid) {
    let (id, uuid) = spawn(
        world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x,
            y: 120.0,
            z: 8.5,
        },
    );
    command(
        world,
        sender,
        receiver,
        &format!("data merge entity {uuid} {{NoAI:true,NoGravity:true,PersistenceRequired:true}}"),
    );
    assert!(!world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .is_first_tick());
    (id, uuid)
}

fn record(world: &ServerWorld, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(fields) = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("compound record")
    };
    fields
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &fields
        .iter()
        .find(|field| field.name == name)
        .unwrap()
        .value
}

fn reload(world: &mut ServerWorld) -> ServerWorld {
    let request = world
        .save_request(1_786_000_008_000, 212, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    install_forced_ticket(&mut candidate);
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    restored.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    restored
}

#[test]
fn changed_attribute_list_length_reaches_live_scale_and_cold_reload() {
    let (mut world, sender, receiver) = fixture();
    let (id, uuid) = cow(&mut world, &sender, &receiver, 8.5);
    let NbtSummaryValue::List(old) = field(&record(&world, id), "attributes").clone() else {
        panic!("attributes")
    };
    assert_ne!(old.len(), 2, "fixture must change the list length");
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    let (position, width, height) = (body.position(), body.bb_width(), body.bb_height());
    command(&mut world, &sender, &receiver, "tick freeze");
    command(
        &mut world,
        &sender,
        &receiver,
        &format!(
            "data modify entity {uuid} attributes set value [{{id:\"minecraft:scale\",base:1.5d}},{{id:\"minecraft:max_health\",base:20.0d}}]"
        ),
    );
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(
        body.living()
            .unwrap()
            .attributes()
            .value(VANILLA_ATTRIBUTE_SCALE_ID),
        Some(1.5)
    );
    assert_eq!(
        body.bb_height(),
        height,
        "command load leaves scale dirty until living tick"
    );
    assert_eq!(body.uuid(), uuid);
    command(&mut world, &sender, &receiver, "tick unfreeze");
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
    assert_eq!(
        (body.bb_width(), body.bb_height()),
        (width * 1.5, height * 1.5)
    );
    assert!(
        (body.position().y + f64::from(body.bb_height()) / 2.0
            - position.y
            - f64::from(height) / 2.0)
            .abs()
            < 1.0e-6
    );
    let stable_position = body.position();
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position(),
        stable_position
    );
    let mut restored = reload(&mut world);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    assert_eq!(
        restored
            .stored_entity_for_test(DIMENSION, restored_id)
            .unwrap()
            .position(),
        stable_position
    );
    restored.tick(Vec::new());
    let body = restored
        .stored_entity_for_test(DIMENSION, restored_id)
        .unwrap();
    assert_eq!(
        (body.bb_width(), body.bb_height()),
        (width * 1.5, height * 1.5)
    );
    let again = reload(&mut restored);
    let again_id = again
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    assert_eq!(
        field(&record(&again, again_id), "attributes"),
        field(&record(&restored, restored_id), "attributes")
    );
}

#[test]
fn container_minecart_accepts_longer_items_list_and_saves_both_slots() {
    let (mut world, sender, receiver) = fixture();
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        Vec3d {
            x: 8.5,
            y: 120.0,
            z: 8.5,
        },
        EntityData::ChestMinecart(crate::entity::MinecartContainerData::new_chest()),
    );
    command(&mut world, &sender, &receiver, "tick freeze");
    command(
        &mut world,
        &sender,
        &receiver,
        &format!(
            "data merge entity {uuid} {{Items:[{{Slot:0b,id:\"minecraft:stone\",count:2}},{{Slot:4b,id:\"minecraft:diamond\",count:3}}]}}"
        ),
    );
    let NbtSummaryValue::List(items) = field(&record(&world, id), "Items").clone() else {
        panic!("items")
    };
    assert_eq!(items.len(), 2);
    for (slot, resource, count) in [(0, "minecraft:stone", 2), (4, "minecraft:diamond", 3)] {
        let row = items
            .iter()
            .find_map(|row| match row {
                NbtSummaryValue::Compound(fields)
                    if field(fields, "Slot") == &NbtSummaryValue::Byte(slot) =>
                {
                    Some(fields)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(field(row, "id"), &NbtSummaryValue::String(resource.into()));
        assert_eq!(field(row, "count"), &NbtSummaryValue::Int(count));
    }
    let restored = reload(&mut world);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    assert_eq!(
        field(&record(&restored, restored_id), "Items"),
        &NbtSummaryValue::List(items)
    );
}

#[test]
fn numeric_command_fields_reach_mob_rules_metadata_and_vehicle_ticks() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_BOGGED_SHEARED_DATA_ID, VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    };
    use bbb_protocol::packets::EntityDataValueKind;

    let (mut world, sender, receiver) = fixture();
    command(&mut world, &sender, &receiver, "tick freeze");
    let (golem, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d {
            x: 6.5,
            y: 120.0,
            z: 8.5,
        },
    );
    command(
        &mut world,
        &sender,
        &receiver,
        &format!("data merge entity {uuid} {{PlayerCreated:1,NoAI:1b,NoGravity:1b}}"),
    );
    let golem = world
        .stored_entity_for_test(DIMENSION, golem)
        .unwrap()
        .clone();
    world.with_level_for_test(DIMENSION, |level| {
        let store = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
        let sensing = Default::default();
        assert!(
            crate::entity::ai::mob::target::TargetView::new(&golem, &store, level, &sensing)
                .body()
                .player_created()
        );
    });
    for (kind, fields, row, expected) in [
        (
            VANILLA_ENTITY_TYPE_BOGGED_ID,
            "sheared:1",
            VANILLA_BOGGED_SHEARED_DATA_ID,
            EntityDataValueKind::Boolean(true),
        ),
        (
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
            "Pumpkin:256",
            VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID,
            EntityDataValueKind::Byte(0),
        ),
    ] {
        let (id, uuid) = spawn(
            &mut world,
            kind,
            Vec3d {
                x: 8.5,
                y: 120.0,
                z: 8.5,
            },
        );
        command(
            &mut world,
            &sender,
            &receiver,
            &format!("data merge entity {uuid} {{{fields},NoAI:1b,NoGravity:1b}}"),
        );
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert!(crate::entity::tracking::synched_values(body)
            .iter()
            .any(|value| value.value.data_id == row && value.value.value == expected));
    }

    let (villager, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        Vec3d {
            x: 10.5,
            y: 120.0,
            z: 8.5,
        },
    );
    command(
        &mut world,
        &sender,
        &receiver,
        &format!("data merge entity {uuid} {{FoodLevel:12,LastGossipDecay:-30000,Gossips:[{{Target:[I;0,0,0,1],Type:'trading',Value:10}}],NoAI:1b,NoGravity:1b}}"),
    );
    assert!(crate::entity::villager_life::can_breed(
        world.stored_entity_for_test(DIMENSION, villager).unwrap()
    ));

    let (cart, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        Vec3d {
            x: 12.5,
            y: 120.0,
            z: 8.5,
        },
        EntityData::TntMinecart(crate::entity::TntMinecartData::new()),
    );
    command(
        &mut world,
        &sender,
        &receiver,
        &format!("data merge entity {uuid} {{fuse:35s,explosion_power:5,explosion_speed_factor:2s,NoGravity:1b}}"),
    );
    let EntityData::TntMinecart(data) = world
        .stored_entity_for_test(DIMENSION, cart)
        .unwrap()
        .data()
    else {
        panic!("TNT minecart")
    };
    assert_eq!(data.fuse(), 35);
    assert_eq!(data.explosion_power_base(), 5.0);
    assert_eq!(data.explosion_speed_factor(), 2.0);

    let (copper, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        Vec3d {
            x: 14.5,
            y: 120.0,
            z: 8.5,
        },
    );
    command(
        &mut world,
        &sender,
        &receiver,
        &format!("data merge entity {uuid} {{next_weather_age:-2,NoAI:1b,NoGravity:1b}}"),
    );
    command(&mut world, &sender, &receiver, "tick unfreeze");
    world.tick(Vec::new());
    let copper_record = record(&world, copper);
    assert_eq!(
        crate::entity::save::optional_long(&copper_record, "next_weather_age"),
        Some(-2)
    );
    assert_eq!(
        field(&copper_record, "weather_state"),
        &NbtSummaryValue::string("unaffected")
    );
    let NbtSummaryValue::List(gossips) = field(&record(&world, villager), "Gossips").clone() else {
        panic!("gossips")
    };
    let [NbtSummaryValue::Compound(gossip)] = gossips.as_slice() else {
        panic!("one gossip")
    };
    assert_eq!(field(gossip, "Value"), &NbtSummaryValue::Int(8));
    let EntityData::TntMinecart(data) = world
        .stored_entity_for_test(DIMENSION, cart)
        .unwrap()
        .data()
    else {
        panic!("TNT minecart")
    };
    assert!(data.fuse() < 35);
}
