use super::*;

/// `/summon` reaches the armor stand runtime, so the stand `createEntity`
/// builds is the entity a selector resolves and `/kill` hurts
/// (`SummonCommand.java:105-111`, `KillCommand.java:22-34`).
#[test]
fn a_summoned_armor_stand_answers_a_selector_and_dies_to_kill() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:armor_stand 11.5 64 11.5",
    );
    assert_eq!(
        console_command_answer(&summoned),
        (Some(1), vec!["commands.summon.success".to_owned()])
    );

    let ids = world.stored_entity_ids_for_test(dimension);
    let [entity_id] = ids.as_slice() else {
        panic!("one summoned stand, not {ids:?}")
    };
    let stand = world
        .stored_entity_for_test(dimension, *entity_id)
        .expect("the summoned stand is filed in the level");
    assert_eq!(
        stand.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert_eq!(
        stand.position(),
        Vec3d {
            x: 11.5,
            y: 64.0,
            z: 11.5
        }
    );
    assert_eq!(stand.y_rot(), 0.0);

    // `@e` without options, which is the selector form this crate resolves.
    let killed = console_command(&sender, &receiver, &mut world, 2, "kill @e");
    assert_eq!(
        console_command_answer(&killed),
        (Some(1), vec!["commands.kill.success.single".to_owned()])
    );
    assert!(world.stored_entity_ids_for_test(dimension).is_empty());

    let root = std::env::temp_dir().join(format!("bbb-command-living-team-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-command-living-team");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig::default();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "watcher", 0xbeef);
    let position = world.publication().players[0].position;
    for (operation, command, result) in [(100, "team add red", 1), (101, "team add blue", 2)] {
        let tick = console_command(&sender, &receiver, &mut world, operation, command);
        assert_eq!(console_command_answer(&tick).0, Some(result));
    }
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        102,
        &format!(
            "summon minecraft:cow {} {} {} {{NoAI:1b,NoGravity:1b,Tags:['team_nbt_probe'],Team:'red'}}",
            position.x + 2.0, position.y, position.z
        ),
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let cow = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(dimension, cow).unwrap().uuid();
    let member = uuid.to_string();
    assert!(summoned.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "red" && team.method == bbb_protocol::packets::PlayerTeamMethod::Join.into()
            && team.players.iter().any(|player| player == &member)
    )));
    let selected = console_command(
        &sender,
        &receiver,
        &mut world,
        103,
        "execute if entity @e[tag=team_nbt_probe,team=red]",
    );
    assert_eq!(console_command_answer(&selected).0, Some(1));
    let merged = console_command(
        &sender,
        &receiver,
        &mut world,
        104,
        &format!("data merge entity {uuid} {{Team:'blue'}}"),
    );
    assert_eq!(console_command_answer(&merged).0, Some(1));
    assert!(merged.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.method == bbb_protocol::packets::PlayerTeamMethod::Join.into()
            && team.players.iter().any(|player| player == &member)
    )));
    for (operation, condition, team) in [(105, "if", "blue"), (106, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=team_nbt_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    for (operation, command) in [
        (107, format!("team join red {uuid}")),
        (108, format!("data merge entity {uuid} {{Silent:1b}}")),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    for (operation, condition, team) in [(109, "if", "red"), (110, "unless", "blue")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=team_nbt_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, cow)
        .unwrap()
    else {
        panic!("cow record")
    };
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    assert!(record
        .iter()
        .any(|entry| entry.name == "Silent" && entry.value == NbtSummaryValue::Byte(1)));
    assert_eq!(
        world.stored_entity_for_test(dimension, cow).unwrap().uuid(),
        uuid
    );

    let passenger_summoned = console_command(
        &sender, &receiver, &mut world, 120,
        &format!(
            "summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Passengers:[{{id:'minecraft:cow',Team:'red',NoAI:1b,NoGravity:1b,Tags:['team_child']}}]}}",
            position.x + 4.0, position.y, position.z
        ),
    );
    assert_eq!(console_command_answer(&passenger_summoned).0, Some(1));
    let boat = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
        })
        .unwrap();
    let boat_uuid = world
        .stored_entity_for_test(dimension, boat)
        .unwrap()
        .uuid();
    let [child] = world.stored_passengers_for_test(dimension, boat) else {
        panic!("one actual passenger")
    };
    let child = *child;
    let body = world.stored_entity_for_test(dimension, child).unwrap();
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert!(body.entity_tags().iter().any(|tag| tag == "team_child"));
    let child_uuid = body.uuid();
    let child_member = child_uuid.to_string();
    assert!(passenger_summoned.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "red" && team.method == bbb_protocol::packets::PlayerTeamMethod::Join.into()
            && team.players.iter().any(|player| player == &child_member)
    )));
    assert!(
        matches!(world.stored_entity_command_data_for_test(dimension, child),
        Some(NbtSummaryValue::Compound(record)) if !record.iter().any(|entry| entry.name == "Team"))
    );
    let selected = console_command(
        &sender,
        &receiver,
        &mut world,
        121,
        "execute if entity @e[tag=team_child,team=red]",
    );
    assert_eq!(console_command_answer(&selected).0, Some(1));
    for (operation, command) in [
        (122, format!("team join blue {child_uuid}")),
        (123, format!("data merge entity {child_uuid} {{Silent:1b}}")),
        (124, "execute if entity @e[tag=team_child,team=blue]".into()),
        (
            125,
            "execute unless entity @e[tag=team_child,team=red]".into(),
        ),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }

    let stand_summoned = console_command(
        &sender, &receiver, &mut world, 130,
        &format!(
            "summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['summoned_stand_boat'],Passengers:[{{id:'minecraft:armor_stand',Team:'red',NoGravity:1b,Silent:1b,ShowArms:1b,Tags:['summoned_stand_child']}}]}}",
            position.x + 8.0, position.y, position.z
        ),
    );
    assert_eq!(console_command_answer(&stand_summoned).0, Some(1));
    let stand_boat = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "summoned_stand_boat")
        })
        .unwrap();
    let stand_boat_body = world.stored_entity_for_test(dimension, stand_boat).unwrap();
    assert_eq!(
        stand_boat_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    let stand_boat_uuid = stand_boat_body.uuid();
    let [stand_child] = world.stored_passengers_for_test(dimension, stand_boat) else {
        panic!("one actual stand passenger")
    };
    let stand_child = *stand_child;
    let body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert!(body
        .entity_tags()
        .iter()
        .any(|tag| tag == "summoned_stand_child"));
    let stand_child_uuid = body.uuid();
    let stand_member = stand_child_uuid.to_string();
    assert!(stand_summoned.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "red" && team.method == bbb_protocol::packets::PlayerTeamMethod::Join.into()
            && team.players.iter().any(|player| player == &stand_member)
    )));
    for (operation, condition, team) in [(131, "if", "red"), (132, "unless", "blue")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=summoned_stand_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_record) = world
        .stored_entity_command_data_for_test(dimension, stand_child)
        .unwrap()
    else {
        panic!("summoned stand record")
    };
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    for name in ["NoGravity", "Silent", "ShowArms"] {
        assert!(stand_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(!stand_record
        .iter()
        .any(|entry| entry.name == "CustomNameVisible" && entry.value == NbtSummaryValue::Byte(1)));
    for (operation, command) in [
        (133, format!("team join blue {stand_child_uuid}")),
        (
            134,
            format!("data merge entity {stand_child_uuid} {{CustomNameVisible:1b}}"),
        ),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    for (operation, condition, team) in [(135, "if", "blue"), (136, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=summoned_stand_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_record) = world
        .stored_entity_command_data_for_test(dimension, stand_child)
        .unwrap()
    else {
        panic!("merged stand record")
    };
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    assert!(stand_record
        .iter()
        .any(|entry| entry.name == "CustomNameVisible" && entry.value == NbtSummaryValue::Byte(1)));

    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red" && team.players.iter().any(|player| player == &member)));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue" && team.players.iter().any(|player| player == &member)));
    assert!(request.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    ));
    assert!(request.scoreboard.teams.iter().any(
        |team| team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    ));
    assert!(!request.scoreboard.teams.iter().any(
        |team| team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    ));
    let saved_boat = saved_entities(&request)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_boat_uuid))
        })
        .unwrap();
    let NbtSummaryValue::List(passengers) = field(&saved_boat, "Passengers") else {
        panic!("saved stand passengers")
    };
    let saved_stand = passengers
        .iter()
        .find_map(|value| match value {
            NbtSummaryValue::Compound(fields)
                if fields.iter().any(|entry| {
                    entry.name == "UUID" && entry.value == nbt_uuid(stand_child_uuid)
                }) =>
            {
                Some(fields)
            }
            _ => None,
        })
        .unwrap();
    assert!(!saved_stand.iter().any(|entry| entry.name == "Team"));
    for name in ["NoGravity", "Silent", "ShowArms", "CustomNameVisible"] {
        assert!(saved_stand
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = storage.load_candidate().unwrap();
    assert!(saved
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red" && team.players.iter().any(|player| player == &member)));
    let mut world = spawn_pinned_restore(config, &saved);
    let (joined, loaded) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "watcher", 0xbeef);
    let cow = world
        .live_entity_id_for_uuid_for_test(dimension, uuid)
        .unwrap();
    assert_eq!(
        world
            .stored_entity_for_test(dimension, cow)
            .unwrap()
            .entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "red" && team.players.iter().any(|player| player == &member)
    )));
    for (operation, condition, team) in [(111, "if", "red"), (112, "unless", "blue")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=team_nbt_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, cow)
        .unwrap()
    else {
        panic!("cow record")
    };
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    let boat = world
        .live_entity_id_for_uuid_for_test(dimension, boat_uuid)
        .unwrap();
    let child = world
        .live_entity_id_for_uuid_for_test(dimension, child_uuid)
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(dimension, boat), &[child]);
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    )));
    for (operation, condition, team) in [(126, "if", "blue"), (127, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=team_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    assert!(
        matches!(world.stored_entity_command_data_for_test(dimension, child),
        Some(NbtSummaryValue::Compound(record)) if !record.iter().any(|entry| entry.name == "Team"))
    );
    let stand_boat = world
        .live_entity_id_for_uuid_for_test(dimension, stand_boat_uuid)
        .unwrap();
    let stand_child = world
        .live_entity_id_for_uuid_for_test(dimension, stand_child_uuid)
        .unwrap();
    assert_eq!(
        world.stored_passengers_for_test(dimension, stand_boat),
        &[stand_child]
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, stand_boat)
            .unwrap()
            .uuid(),
        stand_boat_uuid
    );
    let body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(body.uuid(), stand_child_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert!(body
        .entity_tags()
        .iter()
        .any(|tag| tag == "summoned_stand_child"));
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    )));
    for (operation, condition, team) in [(137, "if", "blue"), (138, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=summoned_stand_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_record) = world
        .stored_entity_command_data_for_test(dimension, stand_child)
        .unwrap()
    else {
        panic!("restored stand record")
    };
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    for name in ["NoGravity", "Silent", "ShowArms", "CustomNameVisible"] {
        assert!(stand_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
