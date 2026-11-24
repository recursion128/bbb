//! The live scoreboard behind `data/minecraft/scoreboard.dat`, over a whole
//! world.

use super::entities::{console_command, console_command_answer};
use super::*;

/// `MinecraftServer.scoreboard` (`MinecraftServer.java:270`) restored from
/// `data/minecraft/scoreboard.dat` and packed back out
/// (`ScoreboardSaveData.java:29-38`), with the live model in between.
#[test]
fn the_scoreboard_survives_a_restore_and_reaches_the_next_save_request() {
    let packed = bbb_server_persistence::Java26ScoreboardPacked {
        objectives: vec![bbb_server_persistence::Java26ScoreboardObjective {
            name: "kills".into(),
            criteria: "playerKillCount".to_string(),
            display_name: bbb_protocol::NbtSummaryValue::string("Kills"),
            render_type: bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
            display_auto_update: false,
            format: None,
        }],
        scores: vec![bbb_server_persistence::Java26ScoreboardPackedScore {
            owner: "alice".into(),
            objective: "kills".into(),
            score: 12,
            locked: false,
            display: None,
            format: None,
        }],
        display_slots: vec![(
            bbb_protocol::scoreboard_display_slots::ScoreboardDisplaySlot::Sidebar,
            "kills".into(),
        )],
        teams: vec![bbb_server_persistence::Java26ScoreboardTeam {
            name: "red".into(),
            display_name: Some(bbb_protocol::NbtSummaryValue::string("Red")),
            color: Some(bbb_protocol::chat_formattings::ChatFormatting::Red),
            allow_friendly_fire: false,
            see_friendly_invisibles: true,
            member_name_prefix: None,
            member_name_suffix: None,
            name_tag_visibility:
                bbb_facts::scoreboard_team_options::TeamVisibility::HideForOtherTeams,
            death_message_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            collision_rule: bbb_facts::scoreboard_team_options::TeamCollisionRule::PushOwnTeam,
            players: vec!["alice".into()],
        }],
    };
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.scoreboard = packed.clone();
    let mut world = crate::tests::restore_stored(creative_fixture_config(), &candidate);
    let request = world
        .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
        .expect("the fixture world builds a save request");
    assert_eq!(request.scoreboard, packed);

    let mut raw = packed;
    raw.objectives[0].name = bbb_protocol::JavaString::from(vec![0xd800]);
    raw.scores[0].objective = raw.objectives[0].name.clone();
    raw.display_slots[0].1 = raw.objectives[0].name.clone();
    raw.scores[0].owner = bbb_protocol::JavaString::from(vec![0xdc00]);
    raw.teams[0].name = bbb_protocol::JavaString::from(vec![0xd801]);
    raw.teams[0].players = vec![raw.scores[0].owner.clone()];
    candidate.scoreboard = raw;
    let mut world = crate::tests::restore_stored(creative_fixture_config(), &candidate);
    let saved = world
        .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap()
        .scoreboard;
    assert_eq!(saved.objectives[0].name.units(), [0xd800]);
    assert_eq!(saved.scores[0].objective.units(), [0xd800]);
    assert_eq!(saved.display_slots[0].1.units(), [0xd800]);
    assert_eq!(saved.scores[0].owner.units(), [0xdc00]);
    assert_eq!(saved.teams[0].name.units(), [0xd801]);
    assert_eq!(saved.teams[0].players[0].units(), [0xdc00]);
}

/// The restored teams reach the command projection, which is what
/// `/spreadplayers respectTeams` reads (`SpreadPlayersCommand.java:140-152`).
#[test]
fn a_restored_teams_membership_reaches_the_command_projection() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.scoreboard = bbb_server_persistence::Java26ScoreboardPacked {
        teams: vec![bbb_server_persistence::Java26ScoreboardTeam {
            name: "red".into(),
            display_name: None,
            color: None,
            allow_friendly_fire: true,
            see_friendly_invisibles: true,
            member_name_prefix: None,
            member_name_suffix: None,
            name_tag_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            death_message_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            collision_rule: bbb_facts::scoreboard_team_options::TeamCollisionRule::Always,
            players: vec!["alice".into(), "bob".into()],
        }],
        ..Default::default()
    };
    let world = crate::tests::restore_stored(creative_fixture_config(), &candidate);
    let team_of = |name| world.scoreboard().players_team(name).map(|team| &team.name);
    assert_eq!(
        team_of("alice"),
        Some(&bbb_protocol::JavaString::from("red"))
    );
    assert_eq!(team_of("bob"), Some(&bbb_protocol::JavaString::from("red")));
    assert_eq!(team_of("cara"), None);

    let root = std::env::temp_dir().join(format!("bbb-saved-living-team-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-saved-living-team");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = creative_fixture_config();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "watcher", 0xbeef);
    let dimension = Java26BuiltinDimension::Overworld;
    let position = world.publication().players[0].position;
    for (operation, command, result) in [(1, "team add red", 1), (2, "team add blue", 2)] {
        let tick = console_command(&sender, &receiver, &mut world, operation, command);
        assert_eq!(console_command_answer(&tick).0, Some(result));
    }
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!(
            "summon minecraft:cow {} {} {} {{NoAI:1b,NoGravity:1b,Silent:1b,Tags:['saved_team_probe'],Team:'red'}}",
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
    let entity = world.stored_entity_for_test(dimension, cow).unwrap();
    let uuid = entity.uuid();
    let position = entity.position();
    let member = uuid.to_string();
    let passenger_summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        &format!(
            "summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Passengers:[{{id:'minecraft:cow',Team:'red',NoAI:1b,NoGravity:1b,Silent:1b,Tags:['saved_child_team_probe']}}]}}",
            position.x + 2.0, position.y, position.z
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
    let child_uuid = body.uuid();
    let child_member = child_uuid.to_string();
    let stand_summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        9,
        &format!(
            "summon minecraft:armor_stand {} {} {} {{Team:'red',NoGravity:1b,Silent:1b,ShowArms:1b,Tags:['saved_stand_team_probe']}}",
            position.x + 4.0, position.y, position.z
        ),
    );
    assert_eq!(console_command_answer(&stand_summoned).0, Some(1));
    let stand = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        })
        .unwrap();
    let stand_uuid = world
        .stored_entity_for_test(dimension, stand)
        .unwrap()
        .uuid();
    let stand_member = stand_uuid.to_string();
    for (operation, command) in [
        (12, format!("summon minecraft:oak_chest_boat {} {} {} {{NoGravity:1b,Tags:['stand_boat']}}",
            position.x + 8.0, position.y, position.z)),
        (13, format!("summon minecraft:armor_stand {} {} {} {{Team:'red',NoGravity:1b,Silent:1b,ShowArms:1b,Tags:['saved_stand_child']}}",
            position.x + 8.0, position.y, position.z)),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, operation, &command);
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let child_boat = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "stand_boat")
        })
        .unwrap();
    let child_boat_body = world.stored_entity_for_test(dimension, child_boat).unwrap();
    assert_eq!(
        child_boat_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    let child_boat_uuid = child_boat_body.uuid();
    let stand_child = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .unwrap()
                .entity_tags()
                .iter()
                .any(|tag| tag == "saved_stand_child")
        })
        .unwrap();
    let body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    let stand_child_uuid = body.uuid();
    let stand_child_member = stand_child_uuid.to_string();
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        14,
        &format!("ride {stand_child_uuid} mount {child_boat_uuid}"),
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert_eq!(
        world.stored_passengers_for_test(dimension, child_boat),
        &[stand_child]
    );
    let mut request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| { team.name == "red" && team.players.iter().any(|player| player == &member) }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &member)
    }));
    assert!(request.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &child_member)
    }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    }));
    assert!(request.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    }));
    let mut injected = false;
    let mut child_injected = false;
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    let mut stand_injected = false;
    let mut stand_child_injected = false;
    for write in &mut request.entity_writes {
        let NbtSummaryValue::List(records) = &mut write
            .document
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == "Entities")
            .unwrap()
            .value
        else {
            panic!("saved entity list")
        };
        for record in records {
            let NbtSummaryValue::Compound(fields) = record else {
                panic!("saved entity record")
            };
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(uuid))
            {
                assert!(!injected);
                assert!(!fields.iter().any(|entry| entry.name == "Team"));
                fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                injected = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_uuid))
            {
                assert!(!stand_injected);
                assert!(!fields.iter().any(|entry| entry.name == "Team"));
                fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                stand_injected = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(child_boat_uuid))
            {
                let NbtSummaryValue::List(passengers) = &mut fields
                    .iter_mut()
                    .find(|entry| entry.name == "Passengers")
                    .unwrap()
                    .value
                else {
                    panic!("actual stand boat saves passengers")
                };
                let child_fields = passengers
                    .iter_mut()
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
                assert!(!stand_child_injected);
                assert!(!child_fields.iter().any(|entry| entry.name == "Team"));
                child_fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                stand_child_injected = true;
            }
            if fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(boat_uuid))
            {
                let Some(NbtSummaryValue::List(passengers)) = fields
                    .iter_mut()
                    .find(|entry| entry.name == "Passengers")
                    .map(|entry| &mut entry.value)
                else {
                    panic!("the actual boat saves its passenger")
                };
                for passenger in passengers {
                    let NbtSummaryValue::Compound(child_fields) = passenger else {
                        panic!("saved passenger record")
                    };
                    if child_fields
                        .iter()
                        .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(child_uuid))
                    {
                        assert!(!child_injected);
                        assert!(!child_fields.iter().any(|entry| entry.name == "Team"));
                        child_fields.push(nbt("Team", NbtSummaryValue::string("blue")));
                        child_injected = true;
                    }
                }
            }
        }
    }
    assert!(injected);
    assert!(child_injected);
    assert!(stand_injected);
    assert!(stand_child_injected);
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert!(saved
        .scoreboard
        .teams
        .iter()
        .any(|team| { team.name == "red" && team.players.iter().any(|player| player == &member) }));
    let input_cow = saved
        .dimensions
        .values()
        .flat_map(|level| level.entity_chunks.values())
        .flat_map(|chunk| chunk.data.entities())
        .find(|record| record.uuid() == uuid)
        .unwrap();
    assert!(input_cow
        .raw()
        .iter()
        .any(|entry| { entry.name == "Team" && entry.value == NbtSummaryValue::string("blue") }));
    assert!(saved.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &child_member)
    }));
    let input_boat = saved
        .dimensions
        .values()
        .flat_map(|level| level.entity_chunks.values())
        .flat_map(|chunk| chunk.data.entities())
        .find(|record| record.uuid() == boat_uuid)
        .unwrap();
    let input_children = input_boat.passengers().collect::<Vec<_>>();
    let [input_child] = input_children.as_slice() else {
        panic!("one saved actual passenger")
    };
    assert_eq!(input_child.uuid(), child_uuid);
    assert!(input_child
        .raw()
        .iter()
        .any(|entry| { entry.name == "Team" && entry.value == NbtSummaryValue::string("blue") }));
    assert!(saved.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    }));
    let input_stand = saved
        .dimensions
        .values()
        .flat_map(|level| level.entity_chunks.values())
        .flat_map(|chunk| chunk.data.entities())
        .find(|record| record.uuid() == stand_uuid)
        .unwrap();
    assert!(input_stand
        .raw()
        .iter()
        .any(|entry| { entry.name == "Team" && entry.value == NbtSummaryValue::string("blue") }));
    assert!(saved.scoreboard.teams.iter().any(|team| team.name == "red"
        && team
            .players
            .iter()
            .any(|player| player == &stand_child_member)));
    assert!(
        !saved.scoreboard.teams.iter().any(|team| team.name == "blue"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member))
    );
    let input_boat = saved
        .dimensions
        .values()
        .flat_map(|level| level.entity_chunks.values())
        .flat_map(|chunk| chunk.data.entities())
        .find(|record| record.uuid() == child_boat_uuid)
        .unwrap();
    let input_child = input_boat
        .passengers()
        .find(|record| record.uuid() == stand_child_uuid)
        .unwrap();
    assert_eq!(input_child.id(), "minecraft:armor_stand");
    assert!(input_child
        .raw()
        .iter()
        .any(|entry| entry.name == "Team" && entry.value == NbtSummaryValue::string("blue")));
    let mut world = spawn_pinned_restore(config, &saved);
    let cow = world
        .live_entity_id_for_uuid_for_test(dimension, uuid)
        .unwrap();
    let entity = world.stored_entity_for_test(dimension, cow).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert_eq!(entity.uuid(), uuid);
    assert_eq!(entity.position(), position);
    let (joined, loaded) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "watcher", 0xbeef);
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &member)
    )));
    for (operation, condition, team) in [(4, "if", "blue"), (5, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=saved_team_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(dimension, cow)
        .unwrap()
    else {
        panic!("restored cow")
    };
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    assert!(record
        .iter()
        .any(|entry| { entry.name == "Silent" && entry.value == NbtSummaryValue::Byte(1) }));
    assert!(record.iter().any(|entry| {
        entry.name == "Tags"
            && entry.value
                == NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "saved_team_probe".to_string(),
                )])
    }));
    let request = world
        .save_request(1_786_000_004_001, 205, RegionCompression::Zlib)
        .unwrap();
    assert!(request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &member)
    }));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| { team.name == "red" && team.players.iter().any(|player| player == &member) }));
    let saved_cow = saved_entities(&request)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(uuid))
        })
        .unwrap();
    assert!(!saved_cow.iter().any(|entry| entry.name == "Team"));
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
    for (operation, condition, team) in [(7, "if", "blue"), (8, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=saved_child_team_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(child_record) = world
        .stored_entity_command_data_for_test(dimension, child)
        .unwrap()
    else {
        panic!("restored actual passenger")
    };
    assert!(!child_record.iter().any(|entry| entry.name == "Team"));
    assert!(child_record
        .iter()
        .any(|entry| { entry.name == "Silent" && entry.value == NbtSummaryValue::Byte(1) }));
    assert!(child_record.iter().any(|entry| {
        entry.name == "Tags"
            && entry.value
                == NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "saved_child_team_probe".to_string(),
                )])
    }));
    assert!(request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &child_member)
    }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &child_member)
    }));
    let saved_boat = saved_entities(&request)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(boat_uuid))
        })
        .unwrap();
    let Some(NbtSummaryValue::List(passengers)) = saved_boat
        .iter()
        .find(|entry| entry.name == "Passengers")
        .map(|entry| &entry.value)
    else {
        panic!("the restored boat saves its passenger")
    };
    let [NbtSummaryValue::Compound(saved_child)] = passengers.as_slice() else {
        panic!("one saved passenger")
    };
    assert!(saved_child
        .iter()
        .any(|entry| { entry.name == "UUID" && entry.value == nbt_uuid(child_uuid) }));
    assert!(!saved_child.iter().any(|entry| entry.name == "Team"));
    let stand = world
        .live_entity_id_for_uuid_for_test(dimension, stand_uuid)
        .unwrap();
    let body = world.stored_entity_for_test(dimension, stand).unwrap();
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert_eq!(body.uuid(), stand_uuid);
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    )));
    for (operation, condition, team) in [(10, "if", "blue"), (11, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=saved_stand_team_probe,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(stand_record) = world
        .stored_entity_command_data_for_test(dimension, stand)
        .unwrap()
    else {
        panic!("restored actual armor stand")
    };
    assert!(!stand_record.iter().any(|entry| entry.name == "Team"));
    for field in ["Silent", "NoGravity", "ShowArms"] {
        assert!(stand_record
            .iter()
            .any(|entry| { entry.name == field && entry.value == NbtSummaryValue::Byte(1) }));
    }
    assert!(stand_record.iter().any(|entry| {
        entry.name == "Tags"
            && entry.value
                == NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "saved_stand_team_probe".to_string(),
                )])
    }));
    assert!(request.scoreboard.teams.iter().any(|team| {
        team.name == "blue" && team.players.iter().any(|player| player == &stand_member)
    }));
    assert!(!request.scoreboard.teams.iter().any(|team| {
        team.name == "red" && team.players.iter().any(|player| player == &stand_member)
    }));
    let saved_stand = saved_entities(&request)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(stand_uuid))
        })
        .unwrap();
    assert!(!saved_stand.iter().any(|entry| entry.name == "Team"));
    let child_boat = world
        .live_entity_id_for_uuid_for_test(dimension, child_boat_uuid)
        .unwrap();
    let stand_child = world
        .live_entity_id_for_uuid_for_test(dimension, stand_child_uuid)
        .unwrap();
    assert_eq!(
        world
            .stored_entity_for_test(dimension, child_boat)
            .unwrap()
            .uuid(),
        child_boat_uuid
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, child_boat)
            .unwrap()
            .entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    let body = world
        .stored_entity_for_test(dimension, stand_child)
        .unwrap();
    assert_eq!(body.uuid(), stand_child_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
    );
    assert_eq!(
        world.stored_passengers_for_test(dimension, child_boat),
        &[stand_child]
    );
    assert!(joined.events.iter().chain(&loaded.events).any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "blue" && team.players.iter().any(|player| player == &stand_child_member)
    )));
    for (operation, condition, team) in [(15, "if", "blue"), (16, "unless", "red")] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation,
            &format!("execute {condition} entity @e[tag=saved_stand_child,team={team}]"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
    }
    let NbtSummaryValue::Compound(child_record) = world
        .stored_entity_command_data_for_test(dimension, stand_child)
        .unwrap()
    else {
        panic!("restored stand passenger")
    };
    assert!(!child_record.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(child_record
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(&child_record, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("saved_stand_child".into())))
    );
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "blue"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    assert!(!request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "red"
            && team
                .players
                .iter()
                .any(|player| player == &stand_child_member)));
    let saved_boat = saved_entities(&request)
        .into_iter()
        .find(|fields| {
            fields
                .iter()
                .any(|entry| entry.name == "UUID" && entry.value == nbt_uuid(child_boat_uuid))
        })
        .unwrap();
    let NbtSummaryValue::List(passengers) = field(&saved_boat, "Passengers") else {
        panic!("resaved stand passenger")
    };
    let saved_child = passengers
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
    assert!(!saved_child.iter().any(|entry| entry.name == "Team"));
    for name in ["Silent", "NoGravity", "ShowArms"] {
        assert!(saved_child
            .iter()
            .any(|entry| entry.name == name && entry.value == NbtSummaryValue::Byte(1)));
    }
    assert!(
        matches!(field(saved_child, "Tags"), NbtSummaryValue::List(tags)
        if tags.contains(&NbtSummaryValue::String("saved_stand_child".into())))
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// One `command.execute` request, the administration surface's route into the
/// same dispatcher a chat command reaches.
fn enqueue_command(sender: &WorldCommandSender, operation_id: u64, command: &str) {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(operation_id, command, AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
}

fn scoreboard_packets(tick: &TickResult) -> Vec<&ClientboundPlayPacket> {
    tick.events
        .iter()
        .filter_map(|event| match event.sent_packet() {
            Some(packet) => matches!(
                packet,
                ClientboundPlayPacket::SetObjective(_)
                    | ClientboundPlayPacket::SetScore(_)
                    | ClientboundPlayPacket::ResetScore(_)
                    | ClientboundPlayPacket::SetDisplayObjective(_)
                    | ClientboundPlayPacket::SetPlayerTeam(_)
            )
            .then_some(packet),
            None => None,
        })
        .collect()
}

/// `ServerScoreboard`'s fan-out reached through the live dispatcher: a
/// `/scoreboard` typed at the management surface broadcasts exactly what the
/// callbacks queue (`ServerScoreboard.java:51-114`), and only once the
/// objective a display slot shows is tracked.
#[test]
fn a_typed_scoreboard_command_broadcasts_what_the_callbacks_queue() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    enqueue_command(&sender, 1, "scoreboard objectives add kills dummy");
    let tick = world.tick(receiver.drain());
    assert_eq!(
        scoreboard_packets(&tick),
        Vec::<&ClientboundPlayPacket>::new()
    );

    enqueue_command(&sender, 2, "scoreboard players set bob kills 4");
    let tick = world.tick(receiver.drain());
    assert_eq!(
        scoreboard_packets(&tick),
        Vec::<&ClientboundPlayPacket>::new()
    );

    enqueue_command(&sender, 3, "scoreboard objectives setdisplay sidebar kills");
    let tick = world.tick(receiver.drain());
    let packets = scoreboard_packets(&tick);
    assert_eq!(packets.len(), 3, "{packets:?}");
    assert!(matches!(
        packets[0],
        ClientboundPlayPacket::SetObjective(objective) if objective.objective_name == "kills"
    ));
    assert!(matches!(
        packets[1],
        ClientboundPlayPacket::SetDisplayObjective(display)
            if display.objective_name.as_ref().is_some_and(|name| name == "kills")
    ));
    assert!(matches!(
        packets[2],
        ClientboundPlayPacket::SetScore(score) if score.owner == "bob" && score.score == 4
    ));

    // Tracked now, so the next write travels on its own.
    enqueue_command(&sender, 4, "scoreboard players set bob kills 9");
    let tick = world.tick(receiver.drain());
    assert_eq!(scoreboard_packets(&tick).len(), 1);

    // And the whole board reaches the session that joins afterwards
    // (`PlayerList.java:191`, `:220-237`).
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 2, "bob", 2);
    let replay = joined
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::SetObjective(_)
                        | ClientboundPlayPacket::SetScore(_)
                        | ClientboundPlayPacket::SetDisplayObjective(_),
                }
            )
        })
        .count();
    assert_eq!(replay, 3, "{joined:?}");
}

/// The board a `/scoreboard` run wrote reaches the next save request, which is
/// what makes the objective outlive the process.
#[test]
fn a_typed_scoreboard_command_reaches_the_next_save_request() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    enqueue_command(&sender, 1, "scoreboard objectives add kills dummy");
    world.tick(receiver.drain());
    enqueue_command(&sender, 2, "scoreboard players set bob kills 4");
    world.tick(receiver.drain());

    let request = world
        .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
        .expect("the world builds a save request");
    assert_eq!(request.scoreboard.objectives.len(), 1);
    assert_eq!(request.scoreboard.objectives[0].name, "kills");
    assert_eq!(request.scoreboard.scores.len(), 1);
    assert_eq!(request.scoreboard.scores[0].owner, "bob");
    assert_eq!(request.scoreboard.scores[0].score, 4);
}

/// `ServerPlayer.doTick`'s recorded criteria (`ServerPlayer.java:680-708`) and
/// `die`'s death count (`:911`): a value is written when it moves, a displayed
/// objective broadcasts the write, and a respawned `ServerPlayer` starts its
/// baselines over (`:232-237`) and so files even unchanged values.
#[test]
fn session_criteria_scores_follow_the_tick_death_and_respawn() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for (operation, command) in [
        "scoreboard objectives add hp health",
        "scoreboard objectives add air air",
        "scoreboard objectives add deaths deathCount",
        "scoreboard objectives setdisplay sidebar hp",
    ]
    .into_iter()
    .enumerate()
    {
        enqueue_command(&sender, operation as u64 + 1, command);
        world.tick(receiver.drain());
    }
    let score = |world: &ServerWorld, objective: &str| {
        world
            .scoreboard()
            .score("alice", objective)
            .map(|score| score.value)
    };
    // The objectives arrived after the join tick recorded every value.
    assert_eq!(score(&world, "hp"), None);
    assert_eq!(score(&world, "air"), None);

    let damaged = chat_command(&sender, &receiver, &mut world, 1, "damage alice 3.5");
    assert_eq!(score(&world, "hp"), Some(17));
    assert!(scoreboard_packets(&damaged).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetScore(score)
            if score.owner == "alice" && score.objective_name == "hp" && score.score == 17
    )));

    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    assert_eq!(score(&world, "deaths"), Some(1));
    assert_eq!(score(&world, "hp"), Some(0));
    assert_eq!(score(&world, "air"), None);

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    world.tick(receiver.drain());
    assert_eq!(score(&world, "hp"), Some(20));
    assert_eq!(score(&world, "air"), Some(300));
    assert_eq!(score(&world, "deaths"), Some(1));
}

#[test]
fn statistic_criteria_follow_live_mining_kills_resets_and_departure() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    super::mining::miner_holding(
        &sender,
        &receiver,
        &mut world,
        super::mining::instant_stone_tool("minecraft:diamond_pickaxe", true),
        true,
    );
    for command in [
        "scoreboard objectives add mined minecraft.mined:minecraft.stone",
        "scoreboard objectives add mined2 minecraft.mined:minecraft.stone",
        "scoreboard objectives add used minecraft.used:minecraft.diamond_pickaxe",
        "scoreboard objectives add kills minecraft.killed:minecraft.pig",
        "scoreboard objectives add total totalKillCount",
        "scoreboard objectives add total2 totalKillCount",
        "scoreboard objectives add players playerKillCount",
        "scoreboard objectives add blue teamkill.blue",
        "scoreboard objectives add by_red killedByTeam.red",
        "scoreboard objectives modify by_red displayautoupdate true",
        "team add killers",
        "team modify killers color red",
        "team join killers miner",
        "team add victims",
        "team modify victims color blue",
        "scoreboard objectives add since_death minecraft.custom:minecraft.time_since_death",
        "scoreboard objectives add left minecraft.custom:minecraft.leave_game",
        "scoreboard objectives modify mined displayautoupdate true",
        "scoreboard objectives setdisplay sidebar mined",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let score = |world: &ServerWorld, owner: &str, objective: &str| {
        world
            .scoreboard()
            .score(owner, objective)
            .map(|score| score.value)
    };
    let broke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: BlockPos { x: 0, y: 63, z: 0 },
            direction: Direction::Up,
            sequence: 1,
        }),
    );
    for objective in ["mined", "mined2", "used"] {
        assert_eq!(score(&world, "miner", objective), Some(1));
    }
    assert!(scoreboard_packets(&broke).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetScore(update)
            if update.owner == "miner" && update.objective_name == "mined" && update.score == 1
                && update.display.is_some()
    )));
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "scoreboard objectives add late minecraft.mined:minecraft.stone",
    );
    assert_eq!(score(&world, "miner", "late"), None);

    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:pig 1 64 1",
    );
    let pig = world
        .entity_reads()
        .stored_entities()
        .find(|row| row.entity.data().entity_resource_id() == "minecraft:pig")
        .unwrap()
        .entity
        .uuid()
        .to_string();
    for command in [
        "team join victims @e[type=minecraft:pig]",
        "scoreboard objectives setdisplay sidebar by_red",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage @e[type=minecraft:pig,limit=1] 40 minecraft:player_attack by miner",
    );
    for objective in ["total", "total2", "blue"] {
        assert_eq!(score(&world, "miner", objective), Some(1));
    }
    assert_eq!(score(&world, "miner", "players"), None);
    assert_eq!(score(&world, &pig, "by_red"), Some(1));
    assert!(scoreboard_packets(&killed).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetScore(update)
            if update.owner == pig && update.objective_name == "by_red" && update.score == 1
                && update.display.is_some()
    )));
    assert_eq!(score(&world, "miner", "kills"), Some(1));
    chat_command(&sender, &receiver, &mut world, 1, "kill miner");
    assert_eq!(score(&world, "miner", "since_death"), Some(0));

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "scoreboard objectives setdisplay sidebar left",
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    let departed = world.tick(receiver.drain());
    assert_eq!(score(&world, "miner", "left"), Some(1));
    assert!(scoreboard_packets(&departed).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetScore(update)
            if update.owner == "miner" && update.objective_name == "left" && update.score == 1
    )));
    let request = world.save_request(0, 0, RegionCompression::Zlib).unwrap();
    assert!(request
        .scoreboard
        .scores
        .iter()
        .any(|score| score.owner == pig
            && score.objective == "by_red"
            && score.score == 1
            && score.display.is_some()));
    assert!(request
        .scoreboard
        .scores
        .iter()
        .any(|score| score.owner == "miner"
            && score.objective == "mined"
            && score.score == 1
            && score.display.is_some()));
    assert!(request
        .scoreboard
        .scores
        .iter()
        .any(|score| score.owner == "miner"
            && score.objective == "since_death"
            && score.score == 0));
}

#[test]
fn kill_criteria_use_current_team_colors_and_only_player_kill_credit() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(mining_config());
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    for command in [
        "gamemode survival @a",
        "scoreboard objectives add total totalKillCount",
        "scoreboard objectives add players playerKillCount",
        "scoreboard objectives add blue teamkill.blue",
        "scoreboard objectives add green teamkill.green",
        "scoreboard objectives add by_red killedByTeam.red",
        "scoreboard objectives add by_yellow killedByTeam.yellow",
        "scoreboard objectives setdisplay sidebar players",
        "team add killers",
        "team modify killers color red",
        "team join killers miner",
        "team add victims",
        "team modify victims color blue",
        "team join victims bob",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let score = |world: &ServerWorld, owner: &str, objective: &str| {
        world
            .scoreboard()
            .score(owner, objective)
            .map(|score| score.value)
    };
    for (victim_color, killer_color, expected) in [("blue", "red", 1), ("green", "yellow", 2)] {
        for command in [
            format!("team modify victims color {victim_color}"),
            format!("team modify killers color {killer_color}"),
        ] {
            chat_command(&sender, &receiver, &mut world, 1, &command);
        }
        let killed = chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "damage bob 40 minecraft:player_attack by miner",
        );
        assert_eq!(score(&world, "miner", "total"), Some(expected));
        assert_eq!(score(&world, "miner", "players"), Some(expected));
        assert_eq!(score(&world, "miner", "blue"), Some(1));
        assert_eq!(score(&world, "bob", "by_red"), Some(1));
        assert!(scoreboard_packets(&killed).iter().any(|packet| matches!(
            packet, ClientboundPlayPacket::SetScore(update)
                if update.owner == "miner" && update.objective_name == "players" && update.score == expected
        )));
        play(
            &sender,
            &receiver,
            &mut world,
            2,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );
        play(
            &sender,
            &receiver,
            &mut world,
            2,
            PlayServerbound::PlayerLoaded,
        );
    }
    assert_eq!(score(&world, "miner", "green"), Some(1));
    assert_eq!(score(&world, "bob", "by_yellow"), Some(1));
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:player_attack by bob",
    );
    assert_eq!(score(&world, "bob", "total"), None);
    assert_eq!(score(&world, "bob", "players"), None);
    assert_eq!(score(&world, "bob", "green"), None);
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::PlayerLoaded,
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:pig 1 64 1",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage bob 40 minecraft:mob_attack by @e[type=minecraft:pig,limit=1]",
    );
    assert_eq!(score(&world, "bob", "by_yellow"), Some(1));
    assert_eq!(score(&world, "miner", "total"), Some(2));
    let request = world.save_request(0, 0, RegionCompression::Zlib).unwrap();
    assert!(request
        .scoreboard
        .scores
        .iter()
        .any(|score| score.owner == "miner" && score.objective == "players" && score.score == 2));
}

/// `ServerLevel.EntityCallbacks.onDestroyed` (`ServerLevel.java:1894-1899`):
/// a non-player entity that is killed leaves its team and loses every score
/// (`Scoreboard.java:342-347`); a player keeps both.
#[test]
fn a_killed_entity_loses_its_scores_and_leaves_its_team() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stand = Uuid::from_u128(77).to_string();
    for (operation, command) in [
        "scoreboard objectives add points dummy".to_string(),
        "team add red".to_string(),
        "execute at alice run summon armor_stand ~ ~ ~ {UUID:[I;0,0,0,77],NoGravity:1b}"
            .to_string(),
        format!("scoreboard players set {stand} points 3"),
        "scoreboard players set alice points 5".to_string(),
        format!("team join red {stand}"),
        "team join red alice".to_string(),
    ]
    .iter()
    .enumerate()
    {
        console_command(
            &sender,
            &receiver,
            &mut world,
            operation as u64 + 10,
            command,
        );
    }
    assert_eq!(
        world.scoreboard().score(&stand, "points").map(|s| s.value),
        Some(3)
    );
    assert_eq!(world.scoreboard().team_members("red").count(), 2);

    console_command(&sender, &receiver, &mut world, 30, &format!("kill {stand}"));
    world.tick(receiver.drain());

    assert_eq!(world.scoreboard().score(&stand, "points"), None);
    assert_eq!(
        world.scoreboard().score("alice", "points").map(|s| s.value),
        Some(5)
    );
    assert_eq!(
        world.scoreboard().team_members("red").collect::<Vec<_>>(),
        ["alice"]
    );
}

/// `TextDisplay.readAdditionalSaveData` (`Display.java:773-786`) resolves the
/// text against the display, on `/summon` and again on `/data merge`.
#[test]
fn a_text_display_resolves_its_score_text_on_summon_and_data_merge() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let display = Uuid::from_u128(78);
    for (operation, command) in [
        "scoreboard objectives add points dummy".to_string(),
        "scoreboard players set alice points 5".to_string(),
        "execute at alice run summon text_display ~ ~ ~ {UUID:[I;0,0,0,78],text:{score:{name:\"alice\",objective:\"points\"}}}".to_string(),
    ]
    .iter()
    .enumerate()
    {
        let tick = console_command(&sender, &receiver, &mut world, operation as u64 + 10, command);
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    let text = |world: &ServerWorld| {
        let id = world
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, display)
            .unwrap();
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap();
        crate::entity::display::text_display(entity)
            .unwrap()
            .text()
            .component()
            .clone()
    };
    let resolved = |value: &str| bbb_protocol::NbtSummaryValue::string(value);
    assert_eq!(text(&world), resolved("5"));

    console_command(
        &sender,
        &receiver,
        &mut world,
        20,
        "scoreboard players set alice points 9",
    );
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        21,
        &format!("data merge entity {display} {{text:{{score:{{name:\"alice\",objective:\"points\"}}}}}}"),
    );
    assert!(console_command_answer(&tick).0.is_some());
    assert_eq!(text(&world), resolved("9"));
}
