use super::*;
use bbb_protocol::NbtSummaryValue;

#[test]
fn captured_entity_nbt_from_a_real_kill_matches_the_saved_body_and_publishes_progress() {
    let criterion = |color, reward| {
        json!({
            "criteria":{"killed":{"trigger":"minecraft:player_killed_entity",
                "conditions":{"entity":[this_entity_properties(json!({"type":"minecraft:sheep",
                    "nbt":format!("{{Color:{color}b,Tags:[\"captured\"]}}")
                }))]}}},
            "rewards":{"experience":reward}
        })
    };
    let mut world = kill_world(&[
        ("test:captured_red", criterion(14, 3)),
        ("test:captured_green", criterion(13, 7)),
    ]);
    // Keep the reward observable without random sheep experience pickups.
    command(&mut world, "gamerule mob_drops false");
    let (id, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    command(
        &mut world,
        &format!("data merge entity {uuid} {{Color:14b,Tags:[\"captured\"],NoAI:1b,NoGravity:1b}}"),
    );
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(saved
        .iter()
        .any(|field| field.name == "Color" && field.value == NbtSummaryValue::Byte(14)));
    assert!(saved.iter().any(|field| matches!(&field.value,
        NbtSummaryValue::List(tags) if field.name == "Tags" && tags.iter().any(|tag| *tag == NbtSummaryValue::String("captured".into()))
    )));
    let events = command(
        &mut world,
        &format!("damage {uuid} 40 minecraft:generic by inventory_player"),
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        0.0
    );
    assert!(world.player_advancement_is_done(CONNECTION, "test:captured_red"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:captured_green"));
    assert_eq!(xp(&world), 3);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if *connection_id == CONNECTION && update.progress.iter().any(|progress|
                progress.id == "test:captured_red" && progress.criteria.iter().any(|criterion|
                    criterion.name == "killed" && criterion.obtained_epoch_millis.is_some()))
    )));
    let written = world.player_advancements_to_data(CONNECTION);
    assert!(written.progress["test:captured_red"]
        .criteria()
        .iter()
        .any(|(name, progress)| name == "killed" && progress.is_done()));
    assert!(written
        .progress
        .get("test:captured_green")
        .is_none_or(|progress| progress
            .criteria()
            .iter()
            .all(|(_, criterion)| !criterion.is_done())));
}

#[test]
fn captured_entity_nbt_of_a_real_vehicle_keeps_its_unopened_loot_table() {
    let criterion = |table, reward| {
        json!({
            "criteria":{"riding":{"trigger":"minecraft:tick",
                "conditions":{"player":{"vehicle":{"type":"minecraft:chest_minecart",
                    "nbt":format!("{{LootTable:\"{table}\",LootTableSeed:23L}}")}}}}},
            "rewards":{"experience":reward}
        })
    };
    let table = "minecraft:chests/simple_dungeon";
    let mut world = kill_world(&[
        ("test:captured_vehicle", criterion(table, 2)),
        (
            "test:other_vehicle",
            criterion("minecraft:chests/abandoned_mineshaft", 11),
        ),
    ]);
    command(&mut world, &format!("summon minecraft:chest_minecart ~ ~ ~ {{NoGravity:1b,LootTable:\"{table}\",LootTableSeed:23L}}"));
    let (id, uuid) = {
        let entities = &world.level(Java26BuiltinDimension::Overworld).entities;
        let body = entities
            .ids()
            .into_iter()
            .filter_map(|id| entities.get(id))
            .find(|body| {
                body.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
            })
            .expect("actual summoned chest minecart");
        assert!(crate::entity::item_slots::container_has_loot(body));
        (body.id(), body.uuid())
    };
    let mut events = command(&mut world, &format!("ride inventory_player mount {uuid}"));
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .external_player_vehicle(world.player_entity_id_for_test(CONNECTION)),
        Some(id)
    );
    for _ in 0..4 {
        if world.player_advancement_is_done(CONNECTION, "test:captured_vehicle") {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    assert!(world.player_advancement_is_done(CONNECTION, "test:captured_vehicle"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:other_vehicle"));
    assert_eq!(xp(&world), 2);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if *connection_id == CONNECTION && update.progress.iter().any(|progress|
                progress.id == "test:captured_vehicle" && progress.criteria.iter().any(|criterion|
                    criterion.name == "riding" && criterion.obtained_epoch_millis.is_some()))
    )));
    assert!(crate::entity::item_slots::container_has_loot(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
    ));
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTable"
            && field.value == NbtSummaryValue::String(table.into())));
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTableSeed" && field.value == NbtSummaryValue::Long(23)));
    assert!(
        world.player_advancements_to_data(CONNECTION).progress["test:captured_vehicle"]
            .criteria()
            .iter()
            .any(|(name, progress)| name == "riding" && progress.is_done())
    );
}
