//! `/item replace` and `/item modify` over live sessions.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
};
use bbb_protocol::packets::{DamageReductionSummary, DataComponentPatchSummary};

use super::*;

#[test]
fn item_modifier_slots_unpack_vehicle_loot_only_when_read_and_preserve_the_copies() {
    use bbb_server_datapack::loot::parse_loot_condition;
    use bbb_server_datapack::loot::{
        parse_loot_function, parse_loot_table, LootElement, LootElementValue,
    };
    use serde_json::json;

    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    for kind in ["chest_minecart", "oak_chest_boat"] {
        let [x, y, z] = ServerWorld::fixture_spawn_block();
        let position = [f64::from(x) + 2.25, f64::from(y) + 0.5, f64::from(z) + 3.75];
        let mut startup = (*startup_generation()).clone();
        let tables = &mut startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:loot_table")
            .unwrap()
            .elements;
        tables.push(LootElement {
            id: "test:vehicle_slots".into(),
            source_pack: "test".into(),
            value: LootElementValue::Table(parse_loot_table(&json!({
                "type":"minecraft:chest",
                "pools":[{"rolls":1,"bonus_rolls":1,"conditions":[
                    {"condition":"minecraft:weather_check","raining":true,"thundering":true},
                    {"condition":"minecraft:location_check","predicate":{
                        "dimension":"minecraft:overworld",
                        "position":{"x":position[0],"y":position[1],"z":position[2]}
                    }},
                    {"condition":"minecraft:inverted","term":{
                        "condition":"minecraft:entity_properties","entity":"this","predicate":{}
                    }}
                ],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
                {"rolls":1,"conditions":[
                    {"condition":"minecraft:weather_check","raining":false,"thundering":false}
                ],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}]
            }), None).unwrap()),
        });
        if kind == "oak_chest_boat" {
            tables.push(LootElement {
                id: "test:predicate_cargo".into(),
                source_pack: "test".into(),
                value: LootElementValue::Table(
                    parse_loot_table(
                        &json!({
                            "type":"minecraft:chest",
                            "pools":[{"rolls":1,"entries":[{
                                "type":"minecraft:item","name":"minecraft:diamond"
                            }]}]
                        }),
                        None,
                    )
                    .unwrap(),
                ),
            });
        }
        tables.sort_by(|left, right| left.id.cmp(&right.id));
        let slots = json!({"type":"minecraft:slots","slot_source":{
            "type":"minecraft:slot_range","source":"this","slots":"container.*"
        }});
        let contents = json!({"function":"minecraft:set_contents",
            "component":"minecraft:container","entries":[slots.clone()]});
        let mut outer_false = contents.clone();
        outer_false["conditions"] = json!([{"condition":"minecraft:random_chance","chance":0.0}]);
        let mut entry_false = contents.clone();
        entry_false["entries"][0]["conditions"] =
            json!([{"condition":"minecraft:random_chance","chance":0.0}]);
        let mut invalid = contents.clone();
        invalid["entries"][0]["slot_source"]["slots"] = json!("container.53");
        let predicate = |range: &str| {
            json!({
                "function":"minecraft:set_name","name":"Predicate",
                "conditions":[{"condition":"minecraft:entity_properties","entity":"this",
                    "predicate":{"slots":{(range):{"items":"minecraft:diamond","count":1}}}}]
            })
        };
        let modifiers = &mut startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == "minecraft:item_modifier")
            .unwrap()
            .elements;
        for (id, body) in [
            (
                "test:vehicle_plain",
                json!({"function":"minecraft:set_name","name":"Plain"}),
            ),
            (
                "test:vehicle_outer_false",
                json!([
                    {"function":"minecraft:set_name","name":"Outer"}, outer_false
                ]),
            ),
            (
                "test:vehicle_entry_false",
                json!([
                    {"function":"minecraft:set_name","name":"Entry"}, entry_false
                ]),
            ),
            (
                "test:vehicle_invalid_slot",
                json!([
                    {"function":"minecraft:set_name","name":"Invalid"}, invalid
                ]),
            ),
            ("test:vehicle_copy", contents),
            ("test:vehicle_predicate", predicate("container.*")),
            (
                "test:vehicle_predicate_invalid_slot",
                json!([
                    {"function":"minecraft:set_name","name":"Invalid predicate"},
                    predicate("container.53")
                ]),
            ),
        ] {
            modifiers.push(LootElement {
                id: id.into(),
                source_pack: "test".into(),
                value: LootElementValue::Modifier(parse_loot_function(&body, None).unwrap()),
            });
        }
        modifiers.sort_by(|left, right| left.id.cmp(&right.id));
        if kind == "oak_chest_boat" {
            let predicates = &mut startup
                .loot
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:predicate")
                .unwrap()
                .elements;
            for (id, count) in [("test:riding_cargo", 1), ("test:riding_cargo_two", 2)] {
                predicates.push(LootElement {
                    id: id.into(),
                    source_pack: "test".into(),
                    value: LootElementValue::Predicate(Box::new(parse_loot_condition(&json!({
                        "condition":"minecraft:entity_properties","entity":"this",
                        "predicate":{"vehicle":{"type":"minecraft:oak_chest_boat",
                            "slots":{"container.*":{"items":"minecraft:diamond","count":count}}
                        }}
                    }), None).unwrap())),
                });
            }
            predicates.sort_by(|left, right| left.id.cmp(&right.id));
        }
        let config = ServerWorldConfig {
            datapacks: Some(Arc::new(startup)),
            ..operator_config(&[("op", 1)])
        };
        let root = std::env::temp_dir().join(format!(
            "bbb-command-vehicle-slots-{}-{kind}",
            std::process::id()
        ));
        let storage = created_fixture_save(&root, "Vehicle slot copies");
        let mut candidate = storage.load_candidate().unwrap();
        candidate.weather = bbb_server_persistence::Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 100_000,
            thunder_time: 100_000,
            raining: true,
            thundering: true,
        };
        let mut world = ServerWorld::restore(config.clone(), &candidate).unwrap();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
        assert!(world.level_weather(DIM).is_raining());
        assert!(world.level_weather(DIM).is_thundering());
        assert!(!world
            .level_weather(Java26BuiltinDimension::Nether)
            .is_raining());
        assert!(!world
            .level_weather(Java26BuiltinDimension::Nether)
            .is_thundering());
        world.tick_rate_mut().set_frozen(true);
        let mut operation = 500;
        let mut run = |world: &mut ServerWorld, command: &str| {
            operation += 1;
            let tick = crate::tests::entities::console_command(
                &sender, &receiver, world, operation, command,
            );
            assert_eq!(
                crate::tests::entities::console_command_answer(&tick).0,
                Some(1),
                "{kind}: {command}"
            );
        };
        run(&mut world, "attribute op minecraft:luck base set 2");
        run(
            &mut world,
            "item replace entity op hotbar.0 with shulker_box",
        );
        run(&mut world, &format!(
            "summon minecraft:{kind} {} {} {} {{NoGravity:1b,LootTable:'test:vehicle_slots',LootTableSeed:37L}}",
            position[0], position[1], position[2]
        ));
        let vehicle = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .unwrap()
                    .data()
                    .entity_resource_id()
                    == format!("minecraft:{kind}")
            })
            .unwrap();
        let uuid = world.stored_entity_for_test(DIM, vehicle).unwrap().uuid();
        let record = |world: &ServerWorld, id| {
            let NbtSummaryValue::Compound(fields) =
                world.stored_entity_command_data_for_test(DIM, id).unwrap()
            else {
                panic!("the vehicle saves a compound")
            };
            fields
        };
        let field = |fields: &[bbb_protocol::NbtSummaryEntry], name: &str| {
            fields
                .iter()
                .find(|field| field.name == name)
                .map(|field| field.value.clone())
        };
        for modifier in [
            "test:vehicle_plain",
            "test:vehicle_outer_false",
            "test:vehicle_entry_false",
            "test:vehicle_invalid_slot",
            "test:vehicle_predicate_invalid_slot",
        ] {
            run(
                &mut world,
                &format!("execute as {uuid} run item modify entity op hotbar.0 {modifier}"),
            );
            let saved = record(&world, vehicle);
            assert_eq!(
                field(&saved, "LootTable"),
                Some(NbtSummaryValue::String("test:vehicle_slots".into())),
                "{modifier}"
            );
            assert_eq!(
                field(&saved, "LootTableSeed"),
                Some(NbtSummaryValue::Long(37))
            );
        }
        let predicate_read = format!(
            "execute as {uuid} in minecraft:the_nether run item modify entity op hotbar.0 test:vehicle_predicate"
        );
        run(&mut world, &predicate_read);
        assert_eq!(
            world.publication().players[0].inventory.slots[0]
                .component_patch
                .custom_name
                .as_deref(),
            Some("Predicate")
        );
        let saved = record(&world, vehicle);
        assert!(field(&saved, "LootTable").is_none());
        let predicate_cargo = field(&saved, "Items").expect("the predicate generated source cargo");
        run(&mut world, &predicate_read);
        assert_eq!(
            field(&record(&world, vehicle), "Items"),
            Some(predicate_cargo.clone())
        );
        run(
            &mut world,
            "item replace entity op hotbar.0 with shulker_box",
        );
        let copy = format!(
            "execute as {uuid} in minecraft:the_nether run item modify entity op hotbar.0 test:vehicle_copy"
        );
        run(&mut world, &copy);
        let copied = world.publication().players[0].inventory.slots[0].clone();
        let contents = &copied.component_patch.container;
        assert_eq!(contents.items().map(|item| item.count).sum::<i32>(), 1);
        assert!(contents
            .items()
            .all(|item| item.item_id == bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID));
        let saved = record(&world, vehicle);
        assert!(field(&saved, "LootTable").is_none());
        let cargo = field(&saved, "Items").expect("the source cargo is saved");
        assert_eq!(cargo, predicate_cargo);
        run(
            &mut world,
            "item replace entity op hotbar.0 with shulker_box",
        );
        run(&mut world, &copy);
        assert_eq!(world.publication().players[0].inventory.slots[0], copied);
        let saved = record(&world, vehicle);
        assert!(field(&saved, "LootTable").is_none());
        assert_eq!(field(&saved, "Items"), Some(cargo.clone()));

        if kind == "oak_chest_boat" {
            run(&mut world, "scoreboard objectives add cargo_probe dummy");
            let mut probe_operation = 1_000;
            for (tag, conditions) in [
                (
                    "command_cargo",
                    [
                        (
                            "as op in minecraft:the_nether if predicate test:riding_cargo",
                            1,
                        ),
                        (
                            "as op in minecraft:the_nether if predicate test:riding_cargo_two",
                            0,
                        ),
                        (
                            "as op in minecraft:the_nether unless predicate test:riding_cargo_two",
                            1,
                        ),
                    ],
                ),
                (
                    "selector_cargo",
                    [
                        ("as op if entity @a[name=op,predicate=test:riding_cargo]", 1),
                        (
                            "as op if entity @a[name=op,predicate=test:riding_cargo_two]",
                            0,
                        ),
                        (
                            "as op if entity @a[name=op,predicate=!test:riding_cargo_two]",
                            1,
                        ),
                    ],
                ),
            ] {
                run(&mut world, &format!(
                    "summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['{tag}'],LootTable:'test:predicate_cargo',LootTableSeed:37L}}",
                    position[0], position[1], position[2]
                ));
                let probe = world
                    .stored_entity_ids_for_test(DIM)
                    .into_iter()
                    .find(|id| {
                        matches!(field(&record(&world, *id), "Tags"),
                        Some(NbtSummaryValue::List(tags))
                            if tags.contains(&NbtSummaryValue::String(tag.into())))
                    })
                    .unwrap();
                let probe_body = world.stored_entity_for_test(DIM, probe).unwrap();
                let probe_uuid = probe_body.uuid();
                assert_eq!(
                    probe_body.data().entity_resource_id(),
                    "minecraft:oak_chest_boat"
                );
                let pending = record(&world, probe);
                assert_eq!(
                    field(&pending, "LootTable"),
                    Some(NbtSummaryValue::String("test:predicate_cargo".into()))
                );
                assert_eq!(
                    field(&pending, "LootTableSeed"),
                    Some(NbtSummaryValue::Long(37))
                );
                assert!(match field(&pending, "Items") {
                    None => true,
                    Some(NbtSummaryValue::List(items)) => items.is_empty(),
                    _ => false,
                });
                run(&mut world, &format!("ride op mount {probe_uuid}"));
                assert_eq!(
                    world
                        .external_player_vehicle_state_for_test(ConnectionId(1))
                        .2,
                    Some(probe)
                );
                assert_eq!(
                    field(&record(&world, probe), "LootTable"),
                    field(&pending, "LootTable")
                );
                for (condition, expected) in conditions {
                    probe_operation += 1;
                    let sentinel = crate::tests::entities::console_command(
                        &sender,
                        &receiver,
                        &mut world,
                        probe_operation,
                        &format!("scoreboard players set #{tag} cargo_probe -7"),
                    );
                    assert_eq!(
                        crate::tests::entities::console_command_answer(&sentinel).0,
                        Some(-7)
                    );
                    assert_eq!(
                        world
                            .scoreboard()
                            .score(&format!("#{tag}"), "cargo_probe")
                            .map(|score| score.value),
                        Some(-7)
                    );
                    probe_operation += 1;
                    let result = crate::tests::entities::console_command(
                        &sender,
                        &receiver,
                        &mut world,
                        probe_operation,
                        &format!("execute store result score #{tag} cargo_probe {condition}"),
                    );
                    if expected == 1 {
                        assert_eq!(
                            crate::tests::entities::console_command_answer(&result).0,
                            Some(1)
                        );
                    }
                    assert_eq!(
                        world
                            .scoreboard()
                            .score(&format!("#{tag}"), "cargo_probe")
                            .map(|score| score.value),
                        Some(expected)
                    );
                }
                assert_eq!(
                    world.stored_entity_for_test(DIM, probe).unwrap().uuid(),
                    probe_uuid
                );
                assert_eq!(
                    world
                        .external_player_vehicle_state_for_test(ConnectionId(1))
                        .2,
                    Some(probe)
                );
                let generated = record(&world, probe);
                assert!(field(&generated, "LootTable").is_none());
                let Some(NbtSummaryValue::List(items)) = field(&generated, "Items") else {
                    panic!("the predicate generated actual cargo");
                };
                let count = items
                    .iter()
                    .map(|item| {
                        let NbtSummaryValue::Compound(fields) = item else {
                            panic!("saved cargo item");
                        };
                        assert_eq!(
                            field(fields, "id"),
                            Some(NbtSummaryValue::String("minecraft:diamond".into()))
                        );
                        let Some(NbtSummaryValue::Int(count)) = field(fields, "count") else {
                            panic!("saved cargo count");
                        };
                        count
                    })
                    .sum::<i32>();
                assert_eq!(count, 1);
                run(&mut world, "ride op dismount");
            }
        }

        let request = world
            .save_request(1_786_000_107_000, 204, RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&candidate, &request).unwrap();
        let candidate = storage.load_candidate().unwrap();
        let mut restored = ServerWorld::restore(config, &candidate).unwrap();
        restored.tick_rate_mut().set_frozen(true);
        join_loaded(&sender, &receiver, &mut restored, 1, "op", 1);
        let vehicle = restored
            .live_entity_id_for_uuid_for_test(DIM, uuid)
            .unwrap();
        let saved = record(&restored, vehicle);
        assert!(field(&saved, "LootTable").is_none());
        assert_eq!(field(&saved, "Items"), Some(cargo));
        assert_eq!(restored.publication().players[0].inventory.slots[0], copied);
        storage.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn filtered_item_modifier_matches_effective_components_from_the_live_publication() {
    let mut startup = (*startup_generation()).clone();
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:enchantment")
        .unwrap();
    let original = registry
        .elements
        .iter()
        .position(|element| element.id == "minecraft:sharpness")
        .unwrap();
    registry
        .elements
        .swap(original, if original == 0 { 1 } else { 0 });
    for (registry_id, tag, member) in [
        (
            "minecraft:enchantment",
            "test:chosen",
            "minecraft:sharpness",
        ),
        ("minecraft:potion", "test:chosen", "minecraft:water"),
    ] {
        let tags = &mut startup.tags.registries;
        if !tags
            .iter()
            .any(|registry| registry.registry_id == registry_id)
        {
            tags.push(bbb_server_datapack::tag_loader::LoadedRegistryTags {
                registry_id: registry_id.into(),
                tags: BTreeMap::new(),
            });
        }
        tags.iter_mut()
            .find(|registry| registry.registry_id == registry_id)
            .unwrap()
            .tags
            .insert(tag.into(), vec![member.into()]);
    }
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..operator_config(&[("op", 1)])
    })
    .unwrap();
    let sharpness = world
        .synchronized_registries()
        .registry_element_index("minecraft:enchantment", "minecraft:sharpness")
        .unwrap();
    assert_ne!(sharpness, original as i32);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for (item, expected, count) in [
        ("diamond[minecraft:repair_cost=9]", 9, 7),
        ("diamond[minecraft:repair_cost=9]", 8, 3),
        ("diamond", 0, 7),
        ("diamond[!minecraft:repair_cost]", 0, 3),
    ] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity op weapon.mainhand with {item} 3"),
        );
        let modified = chat_command(&sender, &receiver, &mut world, 1,
            &format!("item modify entity op weapon.mainhand {{function:'minecraft:filtered',item_filter:{{components:{{'minecraft:repair_cost':{expected}}}}},on_pass:{{function:'minecraft:set_count',count:7}}}}"));
        assert_eq!(
            world.publication().players[0].inventory.slots[0].count,
            count,
            "{item}"
        );
        if count == 7 {
            assert!(modified.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::ContainerSetSlot(update), .. }
                    if update.item.count == 7)));
        }
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        r#"item replace entity op weapon.mainhand with diamond[minecraft:enchantments={"minecraft:sharpness":3}] 3"#,
    );
    assert!(world.publication().players[0].inventory.slots[0]
        .component_patch
        .enchantments
        .iter()
        .any(|entry| entry.holder_id == sharpness && entry.level == 3));
    chat_command(&sender, &receiver, &mut world, 1,
        "item modify entity op weapon.mainhand {function:'minecraft:filtered',item_filter:{components:{'minecraft:enchantments':{'minecraft:sharpness':3}}},on_pass:{function:'minecraft:set_count',count:7}}");
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 7);
    for (item, predicate, matched) in [
        ("diamond", "{'minecraft:enchantments':[]}", true),
        (
            "diamond[minecraft:enchantments={'minecraft:sharpness':3}]",
            "{'minecraft:enchantments':[{enchantments:'#test:chosen',levels:3}]}",
            true,
        ),
        (
            "diamond[!minecraft:enchantments]",
            "{'minecraft:enchantments':[]}",
            false,
        ),
        ("diamond_sword", "{'minecraft:damage':{damage:0}}", true),
        (
            "diamond_sword[minecraft:max_damage=20,minecraft:damage=7,minecraft:unbreakable={}]",
            "{'minecraft:damage':{damage:7,durability:13}}",
            true,
        ),
        (
            "diamond_sword[!minecraft:damage]",
            "{'minecraft:damage':{}}",
            false,
        ),
        (
            "potion[minecraft:potion_contents={potion:'minecraft:water',custom_color:123}]",
            "{'minecraft:potion_contents':['minecraft:awkward','minecraft:water']}",
            true,
        ),
        (
            "potion[minecraft:potion_contents={potion:'minecraft:water'}]",
            "{'minecraft:potion_contents':'#test:chosen'}",
            true,
        ),
        (
            "potion[minecraft:potion_contents={custom_color:123}]",
            "{'minecraft:potion_contents':'minecraft:water'}",
            false,
        ),
        (
            "enchanted_book",
            "{'minecraft:stored_enchantments':[]}",
            true,
        ),
        (
            "enchanted_book[!minecraft:stored_enchantments]",
            "{'minecraft:stored_enchantments':[]}",
            false,
        ),
        (
            "enchanted_book[minecraft:stored_enchantments={'minecraft:sharpness':3}]",
            "{'minecraft:stored_enchantments':[{enchantments:'minecraft:sharpness',levels:3}]}",
            true,
        ),
        (
            "enchanted_book[minecraft:stored_enchantments={'minecraft:sharpness':3}]",
            "{'minecraft:stored_enchantments':[{enchantments:'#test:chosen',levels:{min:2}}]}",
            true,
        ),
        (
            "enchanted_book[minecraft:stored_enchantments={'minecraft:sharpness':3}]",
            "{'minecraft:stored_enchantments':[{enchantments:'minecraft:sharpness',levels:4}]}",
            false,
        ),
    ] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity op weapon.mainhand with {item}"),
        );
        let modified = chat_command(&sender, &receiver, &mut world, 1,
            &format!("item modify entity op weapon.mainhand {{function:'minecraft:filtered',item_filter:{{predicates:{predicate}}},on_pass:{{function:'minecraft:set_name',name:'Matched'}}}}"));
        assert_eq!(
            world.publication().players[0].inventory.slots[0]
                .component_patch
                .custom_name
                .as_deref(),
            matched.then_some("Matched"),
            "{item} {predicate}"
        );
        if matched {
            assert!(modified.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::ContainerSetSlot(update), .. }
                    if update.item.component_patch.custom_name.as_deref() == Some("Matched"))));
        }
    }
}

/// The three components the stack-summary projection gained last, as the `[...]`
/// patch of an item argument: one field of each record away from its codec
/// default (`KineticWeapon.java:42-55`, `BlocksAttacks.java:37-53`) beside the
/// unit `glider` (`DataComponents.java:198`).
const WEAPON_ITEM_PATCH: &str = "minecraft:kinetic_weapon={delay_ticks:3},\
     minecraft:blocks_attacks={block_delay_seconds:0.5},minecraft:glider={}";

/// `/item replace entity` over a live session: `setEntityItem`
/// (`ItemCommands.java:384-385`) writes one `Inventory` slot.
#[test]
fn item_replace_entity_writes_a_live_inventory_slot() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.5 with diamond 3",
    );
    assert!(
        system_chats(&written)
            .iter()
            .any(|(connection, text)| *connection == ConnectionId(1)
                && text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&written)
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(
        slots[5].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
    );
    assert_eq!(slots[5].count, 3);
}

/// `SetComponentsFunction.run` (`SetComponentsFunction.java:29-32`) over a live
/// session: the component the command sets reaches the slot the client is
/// sent and the save patch the storage layer encodes.
#[test]
fn item_modify_entity_set_components_reaches_the_slot_and_the_save() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op weapon.mainhand with diamond",
    );

    let modified = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item modify entity op weapon.mainhand {function:'minecraft:set_components',components:{'minecraft:custom_data':{bbb:1}}}",
    );
    let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "bbb".into(),
        value: NbtSummaryValue::Int(1),
    }]);
    assert!(
        modified.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.item.component_patch.custom_data.as_ref() == Some(&custom_data)
        )),
        "{:?}",
        modified.events
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(
        slots[0].component_patch.custom_data.as_ref(),
        Some(&custom_data)
    );

    // The save patch carries the same component, and the document the storage
    // would author out of it reads back as the same stack.
    let request = world
        .save_request(
            1_786_000_002_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(
        patch.created.inventory.runtime_slots()[0]
            .summary()
            .component_patch
            .custom_data
            .as_ref(),
        Some(&custom_data)
    );
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let Some(NbtSummaryValue::List(saved_inventory)) = parsed.inventory_input() else {
        panic!("saved inventory list");
    };
    let restored = saved_inventory
        .iter()
        .find(|value| {
            bbb_protocol::nbt_summary::value_field(value, "Slot") == Some(&NbtSummaryValue::Byte(0))
        })
        .expect("slot 0 survives the round trip");
    let restored = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        restored,
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .unwrap()
    .into_summary();
    assert_eq!(
        restored.component_patch.custom_data.as_ref(),
        Some(&custom_data)
    );
}

/// `Player.getSlot`'s cursor arm (`Player.java:1705-1718`) over a live
/// session: `containerMenu.setCarried` is what the client sees as
/// `SetCursorItem`.
#[test]
fn item_replace_entity_writes_the_live_cursor() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op player.cursor with diamond 3",
    );
    assert!(
        written.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetCursorItem(update),
                ..
            } if update.item.item_id
                == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
                && update.item.count == 3
        )),
        "{written:?}"
    );

    // The same slot reads back through `getItemInSlot`
    // (`ItemCommands.java:492-499`).
    let copied = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.7 from entity op player.cursor",
    );
    assert!(
        system_chats(&copied)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&copied)
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(
        slots[7].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
    );
    assert_eq!(slots[7].count, 3);
}

/// Every `/item replace` into the grid publishes the result slot twice and the
/// grid slot once.
///
/// The write reaches `slotsChanged` from the container
/// (`TransientCraftingContainer.java:64-67`) and again from the `SlotAccess`
/// (`Player.java:1729-1732`); each runs `slotChangedCraftingGrid`, which sends
/// the result slot whatever it holds (`CraftingMenu.java:70-72`). The grid slot
/// itself is the `broadcastChanges` `setEntityItem` closes with
/// (`ItemCommands.java:386-388`), and it is one packet because the two result
/// publications left the remote copy of slot 0 current.
#[test]
fn item_replace_entity_publishes_the_crafting_result_for_every_grid_write() {
    let mut config = operator_config(&[("op", 1)]);
    config.datapacks = Some(startup_generation());
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    // Four oak planks fill the shaped 2x2 `#minecraft:planks` pattern of
    // `minecraft:crafting_table` (data/minecraft/recipe/crafting_table.json),
    // so only the fourth write leaves a result that differs from the third.
    let mut published = Vec::new();
    for slot in 0..4 {
        let tick = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity op player.crafting.{slot} with minecraft:oak_planks"),
        );
        published.push(
            tick.events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::ContainerSetSlot(update),
                    } => Some(update.slot),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(
        published,
        [vec![0, 0, 1], vec![0, 0, 2], vec![0, 0, 3], vec![0, 0, 4]],
        "{published:?}"
    );
    assert_eq!(
        world
            .player_crafting_result_for_test(ConnectionId(1))
            .item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_CRAFTING_TABLE_ID),
    );
}

/// `ItemInput.createItemStack` (`ItemInput.java:21-25`) over a live session:
/// the patch the item argument read reaches the slot the client is sent and
/// the save patch the storage layer encodes.
#[test]
fn item_replace_entity_with_carries_a_component_patch_to_the_slot_and_the_save() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.0 with diamond[minecraft:damage=5]",
    );
    let damage = VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    assert!(
        written.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.item.component_patch.damage == Some(5)
                && update.item.component_patch.added_type_ids == vec![damage]
        )),
        "{:?}",
        written.events
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(slots[0].component_patch.damage, Some(5));
    assert_eq!(slots[0].component_patch.added_type_ids, vec![damage]);

    // The save patch carries the same component, and the document the storage
    // would author out of it reads back as the same stack.
    let request = world
        .save_request(
            1_786_000_003_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(
        patch.created.inventory.runtime_slots()[0]
            .summary()
            .component_patch
            .damage,
        Some(5)
    );
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let Some(NbtSummaryValue::List(saved_inventory)) = parsed.inventory_input() else {
        panic!("saved inventory list");
    };
    let restored = saved_inventory
        .iter()
        .find(|value| {
            bbb_protocol::nbt_summary::value_field(value, "Slot") == Some(&NbtSummaryValue::Byte(0))
        })
        .expect("slot 0 survives the round trip");
    let restored = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        restored,
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .unwrap()
    .into_summary();
    assert_eq!(restored.component_patch.damage, Some(5));
}

/// `GiveCommand.giveItem` (`GiveCommand.java:48-103`) over a live session: the
/// prototype's patch is what `Inventory.add` stores and the save writes.
#[test]
fn give_carries_a_component_patch_into_the_live_inventory_and_the_save() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "bbb".into(),
        value: NbtSummaryValue::Int(1),
    }]);
    let given = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        r#"give op diamond[minecraft:custom_data={bbb:1},minecraft:custom_name={text:"Named"},minecraft:item_name={text:"Base"},minecraft:enchantments={"minecraft:sharpness":1}] 7"#,
    );
    assert!(
        given.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.item.component_patch.custom_data.as_ref() == Some(&custom_data)
        )),
        "{:?}",
        given.events
    );
    let display = given
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if chat.content.text().contains("commands.give.success.single") => {
                Some(chat.content.component())
            }
            _ => None,
        })
        .expect("the player receives give feedback");
    let display = item_feedback_argument(display, 1);
    assert_item_feedback_name(display, "Named", 1, true);
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(
        slots[0].component_patch.custom_data.as_ref(),
        Some(&custom_data)
    );

    assert_eq!(slots[0].count, 7);
    let request = world
        .save_request(
            1_786_000_004_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(
        patch.created.inventory.runtime_slots()[0]
            .summary()
            .component_patch
            .custom_data
            .as_ref(),
        Some(&custom_data)
    );
}

/// `[!<component>]` (`ItemParser.java:150-160`) over a live session: the
/// removal reaches the slot and survives save and reload.
#[test]
fn item_replace_entity_with_removes_a_prototype_component() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.0 with wooden_sword[!minecraft:damage]",
    );
    let damage = VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(slots[0].component_patch.removed_type_ids, vec![damage]);
    assert!(slots[0].component_patch.added_type_ids.is_empty());

    let request = world
        .save_request(
            1_786_000_005_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let Some(NbtSummaryValue::List(saved_inventory)) = parsed.inventory_input() else {
        panic!("saved inventory list");
    };
    let restored = saved_inventory
        .iter()
        .find(|value| {
            bbb_protocol::nbt_summary::value_field(value, "Slot") == Some(&NbtSummaryValue::Byte(0))
        })
        .expect("slot 0 survives the round trip");
    let restored = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        restored,
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .unwrap()
    .into_summary();
    assert_eq!(restored.component_patch.removed_type_ids, vec![damage]);
}

/// `Player.getSlot`'s crafting arm (`Player.java:1720-1735`) writes the 2x2
/// grid the `InventoryMenu` owns, which is menu slot 1 for grid index 0: the
/// result slot is added first (`InventoryMenu.java:52-53`).
#[test]
fn item_replace_entity_writes_the_live_crafting_grid() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op player.crafting.0 with diamond 2",
    );
    assert!(
        written.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
                ..
            } if update.slot == 1
                && update.item.item_id
                    == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
                && update.item.count == 2
        )),
        "{written:?}"
    );

    let copied = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.8 from entity op player.crafting.0",
    );
    assert!(
        system_chats(&copied)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&copied)
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(slots[8].count, 2);
}

/// `Player.getSlot`'s ender-chest arm (`Player.java:1741-1742`) over a live
/// session. Nothing is published: the container has no open menu, so the write
/// only shows through a later read.
#[test]
fn item_replace_entity_writes_the_live_ender_chest() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op enderchest.5 with diamond 7",
    );
    assert!(
        system_chats(&written)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&written)
    );

    let copied = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op container.9 from entity op enderchest.5",
    );
    assert!(
        system_chats(&copied)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&copied)
    );
    let slots = &world.publication().players[0].inventory.slots;
    assert_eq!(
        slots[9].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
    );
    assert_eq!(slots[9].count, 7);
}

/// The container is 27 slots (`PlayerEnderChestContainer.java:16-18`), so 227
/// is past its end and `Player.getSlot` falls through to `super.getSlot`
/// (`Player.java:1742`), which has no equipment slot of that number either.
#[test]
fn item_replace_entity_refuses_a_slot_past_the_ender_chest() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let refused = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op enderchest.26 with diamond 1",
    );
    assert!(
        system_chats(&refused)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&refused)
    );
    // `enderchest.27` is not a name `SlotRanges` registers at all
    // (`SlotRanges.java:24`), so the argument itself refuses it.
    let unknown = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity op enderchest.27 with diamond 1",
    );
    assert!(
        !system_chats(&unknown)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{:?}",
        system_chats(&unknown)
    );
}

/// `/item replace block` over a live chest: `setBlockItem`
/// (`ItemCommands.java:361-362`) writes one slot of the block entity.
#[test]
fn item_replace_block_writes_a_live_chest_slot() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let properties = std::collections::BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("type".to_string(), "single".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    let chest = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &properties,
    )
    .expect("minecraft:chest is a vanilla 26.1 block");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let written = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace block 1 64 1 container.0 with diamond 3",
    );
    assert_eq!(management_report(&written).0, Some(1));
    let items = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|block_entity| block_entity.chest())
        .expect("the write created the chest's block entity")
        .items();
    assert_eq!(
        items[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
    );
    assert_eq!(items[0].count(), 3);
}

/// `/item replace … from block` copies one container slot onto another
/// (`ItemCommands.java:427-429`).
#[test]
fn item_replace_block_from_block_copies_a_live_chest_slot() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let properties = std::collections::BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("type".to_string(), "single".to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    let chest = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &properties,
    )
    .expect("minecraft:chest is a vanilla 26.1 block");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let written = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        r#"item replace block 1 64 1 container.0 with diamond[minecraft:item_name={text:"Crate Diamond"},minecraft:rarity="uncommon",minecraft:enchantments={"minecraft:sharpness":1}] 3"#,
    );
    assert_eq!(management_report(&written).0, Some(1));
    let copied = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace block 1 64 1 container.1 from block 1 64 1 container.0",
    );
    assert_eq!(management_report(&copied).0, Some(1));
    let feedback = copied
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => match &outcome.result {
                Ok(AdminResult::Command(report)) => report.messages.first(),
                _ => None,
            },
            _ => None,
        })
        .expect("the management command receives copy feedback");
    assert_item_feedback_name(
        item_feedback_argument(feedback.component(), 3),
        "Crate Diamond",
        3,
        false,
    );
    let items = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|block_entity| block_entity.chest())
        .expect("the write created the chest's block entity")
        .items();
    assert_eq!(
        items[1].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
    );
    assert_eq!(items[1].count(), 3);
}

/// The patch of the one `ContainerSetSlot` a tick sent for menu slot `slot`:
/// the stack the session is synchronised with, and the copy the client holds.
///
/// `Inventory` slot `n` is menu slot `n + 36` for the nine hotbar slots: the
/// menu adds the result, the 2x2 grid and the four armor slots first
/// (`InventoryMenu.java:52-59`) and `addStandardInventorySlots` adds the 27
/// extended slots before the hotbar (`AbstractContainerMenu.java:86-91`).
fn slot_patch_sent_to(tick: &TickResult, connection: u64, slot: i16) -> DataComponentPatchSummary {
    tick.events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
            } if *connection_id == ConnectionId(connection) && update.slot == slot => {
                Some(update.item.component_patch.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no ContainerSetSlot for slot {slot}: {:?}", tick.events))
        .into_inner()
}

/// The three components as `WEAPON_ITEM_PATCH` writes them: the field the
/// command set beside the values `KineticWeapon.CODEC` (`KineticWeapon.java:44`)
/// and `BlocksAttacks.CODEC` (`BlocksAttacks.java:39-47`) default, and the unit
/// compound `glider` writes (`Unit.java:11`).
fn assert_the_weapon_components(patch: &DataComponentPatchSummary) {
    let weapon = patch
        .kinetic_weapon
        .as_ref()
        .expect("the stack carries its kinetic weapon");
    assert_eq!(weapon.contact_cooldown_ticks, 10);
    assert_eq!(weapon.delay_ticks, 3);
    assert!(weapon.damage_conditions.is_none());
    assert_eq!(weapon.damage_multiplier_bits, 1.0f32.to_bits());
    let attacks = patch
        .blocks_attacks
        .as_ref()
        .expect("the stack carries its block attacks");
    assert_eq!(f32::from_bits(attacks.block_delay_seconds_bits), 0.5);
    assert_eq!(attacks.disable_cooldown_scale_bits, 1.0f32.to_bits());
    assert_eq!(
        attacks.damage_reductions,
        [DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0f32.to_bits(),
            damage_types: None,
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }]
    );
    assert_eq!(attacks.item_damage.threshold_bits, 1.0f32.to_bits());
    for type_id in [
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    ] {
        assert!(
            patch.added_type_ids.contains(&type_id),
            "{type_id} is not in {:?}",
            patch.added_type_ids
        );
    }
    assert!(patch.removed_type_ids.is_empty(), "{patch:?}");
}

/// The same three components through the filesystem: the storage layer authors
/// the player document `/give` produced, and a restart on that directory seeds
/// a session whose slot still carries them.
#[test]
fn the_weapon_and_glider_components_survive_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-weapon-components-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-weapon-components");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let given = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("give op diamond[{WEAPON_ITEM_PATCH}]"),
    );
    let patch = slot_patch_sent_to(&given, 1, 36);
    assert_the_weapon_components(&patch);

    let request = world
        .save_request(1_786_000_106_000, 203, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    assert_eq!(
        world.publication().players[0].inventory.slots[0].component_patch,
        patch
    );
}

fn item_feedback_argument(message: &NbtSummaryValue, index: usize) -> &NbtSummaryValue {
    let NbtSummaryValue::Compound(fields) = message else {
        panic!("feedback component");
    };
    let NbtSummaryValue::List(arguments) = &fields
        .iter()
        .find(|field| field.name == "with")
        .unwrap()
        .value
    else {
        panic!("feedback arguments");
    };
    &arguments[index]
}

fn assert_item_feedback_name(display: &NbtSummaryValue, name: &str, count: i32, italic: bool) {
    let display = bbb_protocol::RichTextComponent::try_from_component_nbt(display.clone()).unwrap();
    assert!(display.text().contains(name), "{}", display.text());
    let style = bbb_protocol::component_node_style_from_nbt_value(display.component());
    assert_eq!(
        style.color_name,
        Some(bbb_protocol::chat_formattings::ChatFormatting::Aqua)
    );
    let Some(bbb_protocol::ComponentHoverEvent::ShowItem {
        id,
        count: actual,
        components: Some(NbtSummaryValue::Compound(fields)),
    }) = style.hover_event
    else {
        panic!("actual stack hover is required");
    };
    assert_eq!(id, "minecraft:diamond");
    assert_eq!(actual, count);
    assert!(fields
        .iter()
        .any(|field| field.name == "minecraft:enchantments"));
    assert!(fields
        .iter()
        .any(|field| field.name == "minecraft:item_name"));
    if italic {
        assert!(fields
            .iter()
            .any(|field| field.name == "minecraft:custom_name"));
        assert!(display
            .styled()
            .iter()
            .any(|run| run.text == name && run.style.italic == Some(true)));
        assert!(!display.text().contains("Base"));
    }
}

#[test]
fn item_set_name_reads_live_entities_and_storage() {
    let mut startup = (*startup_generation()).clone();
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:predicate")
        .unwrap();
    let index = registry
        .elements
        .binary_search_by(|element| element.id.as_str().cmp("test:name"))
        .unwrap_err();
    registry.elements.insert(index, bbb_server_datapack::loot::LootElement {
        id: "test:name".into(),
        source_pack: "test".into(),
        value: bbb_server_datapack::loot::LootElementValue::Predicate(Box::new(
            bbb_server_datapack::loot::parse_loot_condition(&serde_json::json!([
                {"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":"red"}},
                {"condition":"minecraft:entity_properties","entity":"this","predicate":{
                    "nbt":"{SelectedItem:{id:'minecraft:diamond',count:1},Dimension:'minecraft:overworld'}"}},
                {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":4}},
                {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                    "target":{"type":"minecraft:fixed","name":"global"},"score":"points"},"range":9},
                {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
                {"condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:gameplay/water_evaporates","value":false},
                {"condition":"minecraft:location_check","predicate":{"light":{"light":{"max":7}},"can_see_sky":true}}
            ]), None).unwrap()
        )),
    });
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..operator_config(&[("op", 1), ("other", 2)])
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let (_, stand) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 64.0,
            z: 1.0,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    for command in [
        "item replace entity op weapon.mainhand with diamond".to_owned(),
        format!("tag {stand} add marker"),
        r#"data modify storage test:name message set value "stored""#.to_owned(),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, &command);
    }
    let modified = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        r#"item modify entity op weapon.mainhand {function:"minecraft:set_name",entity:"this",name:[{selector:"@s"},{text:":"},{nbt:"message",storage:"test:name",plain:true},{text:":"},{nbt:"Tags[0]",entity:"@e[type=minecraft:armor_stand,limit=1]",plain:true}]}"#,
    );
    assert!(
        system_chats(&modified)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{modified:?}"
    );
    assert_eq!(
        world.publication().players[0].inventory.slots[0]
            .component_patch
            .custom_name
            .as_deref(),
        Some(r#"op:"stored":"marker""#),
    );
    join_loaded(&sender, &receiver, &mut world, 2, "other", 2);
    for command in [
        "execute in minecraft:the_nether run tp other 0.5 64 0.5",
        "tp op 0.5 200 0.5",
        "team add red",
        "team join red op",
        "scoreboard objectives add points dummy",
        "scoreboard players set op points 4",
        "scoreboard players set global points 9",
        "time of minecraft:overworld pause",
        "time of minecraft:overworld set 18000",
        "time of minecraft:the_end pause",
        "time of minecraft:the_end set 107",
        "weather clear",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let modify = r#"execute in minecraft:the_nether run item modify entity op weapon.mainhand {function:"minecraft:set_name",entity:"this",name:[{text:"night:"},{selector:"@a[predicate=test:name]"}]}"#;
    for (command, name, count) in [
        ("time of minecraft:the_end set 107", "night:op", 1),
        (
            "item replace entity op weapon.mainhand with diamond 2",
            "night:",
            2,
        ),
        (
            "item replace entity op weapon.mainhand with diamond",
            "night:op",
            1,
        ),
        ("time of minecraft:the_end set 108", "night:", 1),
        ("time of minecraft:the_end set 107", "night:op", 1),
        ("time of minecraft:overworld set 6000", "night:", 1),
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
        let modified = chat_command(&sender, &receiver, &mut world, 1, modify);
        assert!(
            system_chats(&modified)
                .iter()
                .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
            "{modified:?}"
        );
        let held = world.player_inventory_slot_for_test(ConnectionId(1), 0);
        assert_eq!(held.count, count, "{command}");
        assert_eq!(
            held.component_patch.custom_name.as_deref(),
            Some(name),
            "{command}"
        );
    }
}
