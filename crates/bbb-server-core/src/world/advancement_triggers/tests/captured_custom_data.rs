use super::*;
use bbb_protocol::NbtSummaryValue;

fn custom_world(vehicle: bool) -> ServerWorld {
    let kind = if vehicle {
        "minecraft:chest_minecart"
    } else {
        "minecraft:sheep"
    };
    let criterion = |field: &str, value: Value, reward| {
        let mut predicate = json!({"type":kind});
        predicate[field] = json!({"minecraft:custom_data":value});
        if !vehicle {
            let components = predicate
                .as_object_mut()
                .unwrap()
                .entry("components")
                .or_insert_with(|| json!({}));
            components["minecraft:sheep/color"] = json!("red");
        }
        let conditions = if vehicle {
            json!({"player":{"vehicle":predicate}})
        } else {
            json!({"entity":[this_entity_properties(predicate)]})
        };
        json!({"criteria":{"matched":{
            "trigger":if vehicle {"minecraft:tick"} else {"minecraft:player_killed_entity"},
            "conditions":conditions}},"rewards":{"experience":reward}})
    };
    let mut definitions = vec![
        (
            "test:exact",
            criterion(
                "components",
                json!({"seal":7,"nested":{"kept":1,"extra":2}}),
                2,
            ),
        ),
        (
            "test:short_exact",
            criterion("components", json!({"seal":7,"nested":{"kept":1}}), 7),
        ),
        (
            "test:partial",
            criterion("predicates", json!("{seal:7b,nested:{kept:1b}}"), 3),
        ),
        (
            "test:wrong_partial",
            criterion("predicates", json!("{seal:8b}"), 11),
        ),
        ("test:empty", criterion("components", json!({}), 1)),
    ];
    if !vehicle {
        let mut wrong_color = criterion(
            "components",
            json!({"seal":7,"nested":{"kept":1,"extra":2}}),
            13,
        );
        wrong_color["criteria"]["matched"]["conditions"]["entity"][0]["predicate"]["components"]
            ["minecraft:sheep/color"] = json!("blue");
        definitions.push(("test:wrong_color", wrong_color));
    }
    let mut world = kill_world(&definitions);
    command(&mut world, "gamerule mob_drops false");
    assert!(!world.game_rule_bool("minecraft:mob_drops", true));
    world
}

fn matched(world: &ServerWorld, events: &[WorldEvent], id: &str) {
    assert!(world.player_advancement_is_done(CONNECTION, id));
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if update.progress.iter().any(|progress| progress.id == id
                && progress.criteria.iter().any(|criterion| criterion.name == "matched"
                    && criterion.obtained_epoch_millis.is_some()))
    )), "missing progress packet for {id}");
    assert!(world.player_advancements_to_data(CONNECTION).progress[id]
        .criteria()
        .iter()
        .any(|(name, progress)| name == "matched" && progress.is_done()));
}

fn rejected(world: &ServerWorld, id: &str) {
    assert!(!world.player_advancement_is_done(CONNECTION, id));
    assert!(world
        .player_advancements_to_data(CONNECTION)
        .progress
        .get(id)
        .is_none_or(|progress| progress
            .criteria()
            .iter()
            .all(|(_, criterion)| !criterion.is_done())));
}

fn saved(world: &ServerWorld, id: i32) -> Vec<bbb_protocol::NbtSummaryEntry> {
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
    else {
        unreachable!()
    };
    saved
}

fn assert_custom_data(world: &ServerWorld, id: i32) {
    assert!(saved(world, id).iter().any(|entry| entry.name == "data"
        && matches!(&entry.value, NbtSummaryValue::Compound(data)
            if data.iter().any(|field| field.name == "seal" && field.value == NbtSummaryValue::Byte(7)))));
}

#[test]
fn captured_custom_data_and_sheep_color_match_real_kill_components() {
    let mut world = custom_world(false);
    let (id, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    command(&mut world, &format!("data merge entity {uuid} {{NoAI:1b,NoGravity:1b,Color:14b,data:{{seal:7b,nested:{{kept:1b,extra:2b}}}}}}"));
    assert_custom_data(&world, id);
    assert!(saved(&world, id)
        .iter()
        .any(|field| field.name == "Color" && field.value == NbtSummaryValue::Byte(14)));
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
    for id in ["test:exact", "test:partial"] {
        matched(&world, &events, id);
    }
    for id in ["test:short_exact", "test:wrong_partial", "test:empty"] {
        rejected(&world, id);
    }
    rejected(&world, "test:wrong_color");
    assert_eq!(xp(&world), 5);
}

#[test]
fn captured_custom_data_on_a_real_vehicle_keeps_pending_loot_and_knows_empty_data() {
    let mut world = custom_world(true);
    let table = "minecraft:chests/simple_dungeon";
    command(&mut world, &format!("summon minecraft:chest_minecart ~ ~ ~ {{NoGravity:1b,LootTable:\"{table}\",LootTableSeed:23L,data:{{seal:7b,nested:{{kept:1b,extra:2b}}}}}}"));
    let (id, uuid) = {
        let store = &world.level(Java26BuiltinDimension::Overworld).entities;
        let body = store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .find(|body| {
                body.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
            })
            .unwrap();
        assert!(crate::entity::item_slots::container_has_loot(body));
        (body.id(), body.uuid())
    };
    assert_custom_data(&world, id);
    let mut events = command(&mut world, &format!("ride inventory_player mount {uuid}"));
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .external_player_vehicle(world.player_entity_id_for_test(CONNECTION)),
        Some(id)
    );
    for _ in 0..4 {
        if world.player_advancement_is_done(CONNECTION, "test:exact")
            && world.player_advancement_is_done(CONNECTION, "test:partial")
        {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    for id in ["test:exact", "test:partial"] {
        matched(&world, &events, id);
    }
    for id in ["test:short_exact", "test:wrong_partial", "test:empty"] {
        rejected(&world, id);
    }
    assert_eq!(xp(&world), 5);
    let mut events = command(
        &mut world,
        &format!("data modify entity {uuid} data set value {{}}"),
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
        .custom_data()
        .is_empty());
    for _ in 0..4 {
        if world.player_advancement_is_done(CONNECTION, "test:empty") {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    matched(&world, &events, "test:empty");
    assert_eq!(xp(&world), 6);
    assert!(crate::entity::item_slots::container_has_loot(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
    ));
    let saved = saved(&world, id);
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTable"
            && field.value == NbtSummaryValue::String(table.into())));
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTableSeed" && field.value == NbtSummaryValue::Long(23)));
}
