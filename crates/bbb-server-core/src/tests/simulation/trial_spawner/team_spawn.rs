use super::*;

#[test]
fn a_trial_spawner_detects_a_player_and_spawns_its_pinned_mob_into_the_store() {
    let mut record = pinned_record([4.5, 65.0, 6.5]);
    let NbtSummaryValue::Compound(config) = &mut record
        .iter_mut()
        .find(|entry| entry.name == "normal_config")
        .unwrap()
        .value
    else {
        panic!("the pinned normal config is a compound")
    };
    let NbtSummaryValue::List(potentials) = &mut config
        .iter_mut()
        .find(|entry| entry.name == "spawn_potentials")
        .unwrap()
        .value
    else {
        panic!("the pinned spawn potentials are a list")
    };
    let NbtSummaryValue::Compound(potential) = &mut potentials[0] else {
        panic!("the pinned potential is a compound")
    };
    let NbtSummaryValue::Compound(data) = &mut potential
        .iter_mut()
        .find(|entry| entry.name == "data")
        .unwrap()
        .value
    else {
        panic!("the pinned potential data is a compound")
    };
    let NbtSummaryValue::Compound(entity) = &mut data
        .iter_mut()
        .find(|entry| entry.name == "entity")
        .unwrap()
        .value
    else {
        panic!("the pinned entity is a compound")
    };
    entity.extend([
        entry("Team", NbtSummaryValue::String("blue".into())),
        entry(
            "Tags",
            NbtSummaryValue::List(vec![NbtSummaryValue::String("trial_team_root".into())]),
        ),
        entry("NoAI", NbtSummaryValue::Byte(1)),
        entry(
            "Passengers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("minecraft:chicken".into())),
                entry("Team", NbtSummaryValue::String("red".into())),
                entry(
                    "Tags",
                    NbtSummaryValue::List(vec![NbtSummaryValue::String("trial_team_child".into())]),
                ),
            ])]),
        ),
    ]);
    let mut candidate =
        restore_candidate(0, Some(trial_spawner_chunk("inactive", record)), Vec::new());
    candidate.scoreboard.teams = ["blue", "red"]
        .into_iter()
        .map(|name| bbb_server_persistence::Java26ScoreboardTeam {
            name: name.into(),
            display_name: None,
            color: None,
            allow_friendly_fire: true,
            see_friendly_invisibles: true,
            member_name_prefix: None,
            member_name_suffix: None,
            name_tag_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            death_message_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            collision_rule: bbb_facts::scoreboard_team_options::TeamCollisionRule::Always,
            players: Vec::new(),
        })
        .collect();
    let (sender, receiver, mut world) = open_trial_spawner_world(survival_config(), &candidate);

    // `TrialSpawnerState`: a resolvable display entity leaves
    // INACTIVE, and `TrialSpawnerBlockEntity.setState` writes the property.
    world.tick(receiver.drain());
    assert_eq!(
        trial_spawner_state(&mut world),
        TrialSpawnerState::WaitingForPlayers
    );

    // `TrialSpawnerStateData.tryDetectPlayers` (`:119`) runs on the phase
    // `(pos.asLong() + gameTime) % 20 == 0`, which is game_time 4 here, and
    // `:110-120` adds the 40-tick buffer the first detection arms.
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 4);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    let detected = trial_spawner_data(&mut world);
    assert_eq!(detected.registered_players().len(), 1);
    assert_eq!(detected.next_mob_spawns_at(), 44);

    for _ in 0..39 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 43);
    assert_eq!(
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| {
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                    .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            })
            .count(),
        0,
        "nothing spawns before the countdown is due"
    );

    let spawned = world.tick(receiver.drain());
    let flushed = world.tick(receiver.drain());
    assert_eq!(world.game_time(), 45);

    let (zombie, uuid, position) = stored_zombie(&mut world);
    assert_eq!(
        position,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 6.5,
        }
    );
    let data = trial_spawner_data(&mut world);
    assert_eq!(
        data.current_mobs().iter().copied().collect::<Vec<_>>(),
        vec![uuid]
    );
    assert_eq!(data.total_mobs_spawned(), 1);
    assert_eq!(data.next_mob_spawns_at(), 44 + 40);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);

    let mut events = trial_spawner_events(&spawned);
    events.extend(trial_spawner_events(&flushed));
    assert_eq!(
        events,
        vec![(3011, at(), 0), (3012, BlockPos { x: 4, y: 65, z: 6 }, 0),]
    );
    // The fixture's walk over the floor to the spawner is the session's own
    // STEP.
    let alice = world.player_entity_id_for_test(ConnectionId(1));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    let game_events: Vec<_> = game_events
        .iter()
        .filter(|event| event.source_entity_id != Some(alice))
        .collect();
    assert_eq!(game_events.len(), 1);
    assert_eq!(game_events[0].game_event, VanillaGameEvent::ENTITY_PLACE);
    assert_eq!(game_events[0].pos, BlockPos { x: 4, y: 65, z: 6 });
    assert_eq!(game_events[0].source_entity_id, Some(zombie));

    let children = world.stored_passengers_for_test(Java26BuiltinDimension::Overworld, zombie);
    let [child] = children else {
        panic!("the actual trial root has its one configured passenger")
    };
    let child = *child;
    let child_body = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, child)
        .unwrap();
    assert_eq!(child_body.entity_type_id(), VANILLA_ENTITY_TYPE_CHICKEN_ID);
    let child_uuid = child_body.uuid();
    assert_ne!(child_uuid, uuid);
    let root_member = uuid.to_string();
    let child_member = child_uuid.to_string();
    for (team, other, member) in [
        ("blue", "red", &root_member),
        ("red", "blue", &child_member),
    ] {
        assert!(spawned.events.iter().chain(&flushed.events).any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(packet) }
            if packet.name == team && packet.players.iter().any(|player| player == member)
        )));
        assert!(!spawned.events.iter().chain(&flushed.events).any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(packet) }
            if packet.name == other && packet.players.iter().any(|player| player == member)
        )));
    }
    for (id, tag, team, other) in [
        (zombie, "trial_team_root", "blue", "red"),
        (child, "trial_team_child", "red", "blue"),
    ] {
        let selected = crate::tests::entities::console_command(
            &sender, &receiver, &mut world, 200,
            &format!("execute if entity @e[tag={tag},team={team}] unless entity @e[tag={tag},team={other}]"),
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&selected).0,
            Some(1)
        );
        let NbtSummaryValue::Compound(record) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
        else {
            panic!("the actual spawned entity has canonical data")
        };
        assert!(!record.iter().any(|entry| entry.name == "Team"));
        assert!(
            matches!(saved_field(&record, "Tags"), Some(NbtSummaryValue::List(tags))
            if tags.contains(&NbtSummaryValue::String(tag.into())))
        );
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    for (team, other, member) in [
        ("blue", "red", &root_member),
        ("red", "blue", &child_member),
    ] {
        assert!(
            request
                .scoreboard
                .teams
                .iter()
                .any(|entry| entry.name == team
                    && entry.players.iter().any(|player| player == member))
        );
        assert!(!request.scoreboard.teams.iter().any(
            |entry| entry.name == other && entry.players.iter().any(|player| player == member)
        ));
    }
}
