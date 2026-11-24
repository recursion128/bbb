use super::*;

#[test]
fn a_mob_death_reads_and_saves_its_attackers_pending_vehicle_cargo() {
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[{
        "rolls":1,"conditions":[{"condition":"minecraft:inverted","term":{
            "condition":"minecraft:entity_properties","entity":"this","predicate":{}
        }}],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    let vehicle_condition = |item| {
        serde_json::json!({
            "condition":"minecraft:entity_properties","entity":"attacker",
            "predicate":{"type":"minecraft:pig","vehicle":{
                "type":"minecraft:oak_chest_boat",
                "slots":{"container.*":{"items":item,"count":1}}
            }}
        })
    };
    let death = serde_json::json!({"type":"minecraft:entity","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[vehicle_condition("minecraft:diamond")],
            "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[vehicle_condition("minecraft:stone")],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-death-vehicle-slots-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Death vehicle slots","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (name, table) in [("pending_vehicle", cargo), ("pending_vehicle_death", death)] {
        std::fs::write(
            pack.join(format!("data/test/loot_table/{name}.json")),
            serde_json::to_vec(&table).unwrap(),
        )
        .unwrap();
    }
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    };
    std::fs::remove_dir_all(root).unwrap();
    let mut world = spawn_pinned_world(config.clone());
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for (index, command) in [
        "gamemode spectator observer",
        "summon minecraft:oak_chest_boat 0.5 64 0.5 {UUID:[I;0,0,0,4001],NoGravity:1b,LootTable:'test:pending_vehicle',LootTableSeed:37L}",
        "summon minecraft:pig -2.5 64 0.5 {UUID:[I;0,0,0,4002],NoAI:1b,NoGravity:1b}",
        "summon minecraft:cow 3.5 64 0.5 {UUID:[I;0,0,0,4003],NoAI:1b,NoGravity:1b,DeathLootTable:'test:pending_vehicle_death'}",
        "ride 00000000-0000-0000-0000-000000000fa2 mount 00000000-0000-0000-0000-000000000fa1",
    ].into_iter().enumerate() {
        let tick = console_command(&sender, &receiver, &mut world, index as u64 + 1, command);
        assert_eq!(console_command_answer(&tick).0, Some(1), "{command}");
    }
    let dimension = Java26BuiltinDimension::Overworld;
    let boat_uuid = Uuid::from_u128(4001);
    let pig_uuid = Uuid::from_u128(4002);
    let cow_uuid = Uuid::from_u128(4003);
    let boat = world
        .live_entity_id_for_uuid_for_test(dimension, boat_uuid)
        .unwrap();
    let pig = world
        .live_entity_id_for_uuid_for_test(dimension, pig_uuid)
        .unwrap();
    let cow = world
        .live_entity_id_for_uuid_for_test(dimension, cow_uuid)
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(dimension, boat), &[pig]);
    let pig_body = world.stored_entity_for_test(dimension, pig).unwrap();
    assert_eq!(pig_body.uuid(), pig_uuid);
    assert!(pig_body.is_alive_with_health());
    let boat_body = world.stored_entity_for_test(dimension, boat).unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert!(crate::entity::item_slots::container_has_loot(boat_body));
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        assert!(crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let before = command_record(&world, boat_uuid);
    assert_eq!(
        nbt_field(&before, "LootTable"),
        &NbtSummaryValue::String("test:pending_vehicle".into())
    );
    assert_eq!(
        nbt_field(&before, "LootTableSeed"),
        &NbtSummaryValue::Long(37)
    );
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    assert!(items(&world).is_empty());
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        &format!("damage {cow_uuid} 40 minecraft:generic by {pig_uuid}"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert!(!world
        .stored_entity_for_test(dimension, cow)
        .unwrap()
        .is_alive_with_health());
    let mut drops: Vec<_> = items(&world)
        .into_iter()
        .map(|item| (item.2, item.3))
        .collect();
    drops.sort();
    assert_eq!(
        drops,
        vec![
            ("minecraft:apple".into(), 1),
            ("minecraft:diamond".into(), 1)
        ]
    );

    let saved = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &saved.entity_writes);
    let restored = spawn_pinned_restore(config, &candidate);
    for current in [&world, &restored] {
        let boat = current
            .live_entity_id_for_uuid_for_test(dimension, boat_uuid)
            .unwrap();
        let pig = current
            .live_entity_id_for_uuid_for_test(dimension, pig_uuid)
            .unwrap();
        assert_eq!(current.stored_passengers_for_test(dimension, boat), &[pig]);
        let pig_body = current.stored_entity_for_test(dimension, pig).unwrap();
        assert_eq!(pig_body.uuid(), pig_uuid);
        assert!(pig_body.is_alive_with_health());
        let boat_body = current.stored_entity_for_test(dimension, boat).unwrap();
        assert_eq!(boat_body.uuid(), boat_uuid);
        assert_eq!(boat_body.removal_reason(), None);
        assert!(!crate::entity::item_slots::container_has_loot(boat_body));
        let record = command_record(current, boat_uuid);
        assert!(!record.iter().any(|entry| entry.name == "LootTable"));
        let NbtSummaryValue::List(cargo) = nbt_field(&record, "Items") else {
            panic!("the unpacked cargo saves its items");
        };
        let [NbtSummaryValue::Compound(stack)] = cargo.as_slice() else {
            panic!("the actual boat keeps exactly one cargo stack: {cargo:?}");
        };
        assert_eq!(
            nbt_field(stack, "id"),
            &NbtSummaryValue::String("minecraft:diamond".into())
        );
        assert_eq!(nbt_field(stack, "count"), &NbtSummaryValue::Int(1));
    }
}
