use super::*;

#[test]
fn border_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::InitializeBorder(
        bbb_protocol::packets::InitializeBorder {
            new_center_x: 1.0,
            new_center_z: 2.0,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 40,
            new_absolute_max_size: 500,
            warning_blocks: 6,
            warning_time: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderCenter(
        bbb_protocol::packets::SetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderLerpSize(
        bbb_protocol::packets::SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderSize(
        bbb_protocol::packets::SetBorderSize { size: 250.0 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderWarningDelay(
        bbb_protocol::packets::SetBorderWarningDelay { warning_delay: 9 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderWarningDistance(
        bbb_protocol::packets::SetBorderWarningDistance { warning_blocks: 8 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );

    let border = world.world_border();
    assert_eq!(border.center_x, 3.0);
    assert_eq!(border.center_z, 4.0);
    assert_eq!(border.size, 250.0);
    assert_eq!(border.lerp_target, 250.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.absolute_max_size, 500);
    assert_eq!(border.warning_blocks, 8);
    assert_eq!(border.warning_time, 9);

    let world_counters = world.counters();
    assert_eq!(world_counters.world_border_initializes_received, 1);
    assert_eq!(world_counters.world_border_center_updates_received, 1);
    assert_eq!(world_counters.world_border_lerp_size_updates_received, 1);
    assert_eq!(world_counters.world_border_size_updates_received, 1);
    assert_eq!(
        world_counters.world_border_warning_delay_updates_received,
        1
    );
    assert_eq!(
        world_counters.world_border_warning_distance_updates_received,
        1
    );
}

#[test]
fn scoreboard_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(11);
    tx.try_send(NetEvent::Play(PlayClientbound::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "kills".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Add,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Kills".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: Some(vec![9]),
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Steve".to_string(),
            objective_name: "kills".to_string(),
            score: 4,
            display: Some("Four".to_string()),
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Alex".to_string(),
            objective_name: "kills".to_string(),
            score: 1,
            display: None,
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerTeam(
        bbb_protocol::packets::SetPlayerTeam {
            name: "red".to_string(),
            method: bbb_protocol::packets::PlayerTeamMethod::Add,
            parameters: Some(bbb_protocol::packets::PlayerTeamParameters {
                display_name: "Red Team".to_string(),
                options: 0b11,
                nametag_visibility: bbb_protocol::packets::TeamVisibility::Always,
                collision_rule: bbb_protocol::packets::TeamCollisionRule::Never,
                color: bbb_protocol::packets::ChatFormatting::Red,
                player_prefix: "[R]".to_string(),
                player_suffix: "!".to_string(),
            }),
            players: vec!["Steve".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ResetScore(
        bbb_protocol::packets::ResetScore {
            owner: "Alex".to_string(),
            objective_name: Some("kills".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "missing".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Change,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Missing".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: None,
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::List,
            objective_name: Some("missing".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Nobody".to_string(),
            objective_name: "missing".to_string(),
            score: 9,
            display: None,
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerTeam(
        bbb_protocol::packets::SetPlayerTeam {
            name: "missing".to_string(),
            method: bbb_protocol::packets::PlayerTeamMethod::Join,
            parameters: None,
            players: vec!["Nobody".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ResetScore(
        bbb_protocol::packets::ResetScore {
            owner: "Nobody".to_string(),
            objective_name: Some("missing".to_string()),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        11
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.set_objective_packets, 2);
    assert_eq!(world_counters.set_objective_updates_applied, 1);
    assert_eq!(world_counters.set_objective_updates_ignored, 1);
    assert_eq!(world_counters.set_display_objective_packets, 2);
    assert_eq!(world_counters.set_display_objective_updates_applied, 1);
    assert_eq!(world_counters.set_display_objective_updates_ignored, 1);
    assert_eq!(world_counters.set_score_packets, 3);
    assert_eq!(world_counters.set_score_updates_applied, 2);
    assert_eq!(world_counters.set_score_updates_ignored, 1);
    assert_eq!(world_counters.set_player_team_packets, 2);
    assert_eq!(world_counters.set_player_team_updates_applied, 1);
    assert_eq!(world_counters.set_player_team_updates_ignored, 1);
    assert_eq!(world_counters.reset_score_packets, 2);
    assert_eq!(world_counters.reset_score_updates_applied, 1);
    assert_eq!(world_counters.reset_score_updates_ignored, 1);

    let scoreboard = world.scoreboard();
    let objective = scoreboard.objectives.get("kills").unwrap();
    assert_eq!(objective.display_name, "Kills");
    assert_eq!(objective.render_type, "integer");
    assert_eq!(objective.number_format, Some(vec![9]));
    assert_eq!(
        scoreboard.display_slots.get("sidebar").map(String::as_str),
        Some("kills")
    );

    let steve_scores = scoreboard.scores.get("Steve").unwrap();
    let steve_kills = steve_scores.get("kills").unwrap();
    assert_eq!(steve_kills.value, 4);
    assert_eq!(steve_kills.display.as_deref(), Some("Four"));
    assert!(!scoreboard.scores.contains_key("Alex"));

    let team = scoreboard.teams.get("red").unwrap();
    assert!(team.players.contains("Steve"));
    let parameters = team.parameters.as_ref().unwrap();
    assert_eq!(parameters.display_name, "Red Team");
    assert_eq!(parameters.color, "red");
}

#[test]
fn hud_session_events_update_world_and_world_counters() {
    let boss_id = Uuid::from_u128(1);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::Add {
                name: "Ender Dragon".to_string(),
                progress: 0.75,
                color: bbb_protocol::packets::BossBarColor::Purple,
                overlay: bbb_protocol::packets::BossBarOverlay::Progress,
                flags: bbb_protocol::packets::BossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 0.25 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: Uuid::from_u128(99),
            operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 1.0 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TabList(
        bbb_protocol::packets::TabList {
            header: Some("Welcome".to_string()),
            footer: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChangeDifficulty(
        bbb_protocol::packets::ChangeDifficulty {
            difficulty: bbb_protocol::packets::Difficulty::Hard,
            locked: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );

    let boss = world.boss_bars().get(&boss_id).unwrap();
    assert_eq!(boss.name, "Ender Dragon");
    assert_eq!(boss.progress, 0.25);
    assert_eq!(boss.color, "purple");
    assert_eq!(boss.overlay, "progress");
    assert!(boss.darken_screen);
    assert!(boss.create_world_fog);
    assert_eq!(world.tab_list().header.as_deref(), Some("Welcome"));
    assert_eq!(world.tab_list().footer, None);
    assert_eq!(world.difficulty().difficulty, "hard");
    assert!(world.difficulty().difficulty_locked);

    let world_counters = world.counters();
    assert_eq!(world_counters.boss_event_packets, 3);
    assert_eq!(world_counters.boss_bars_tracked, 1);
    assert_eq!(world_counters.boss_events_ignored, 1);
    assert_eq!(world_counters.tab_list_packets, 1);
    assert_eq!(world_counters.change_difficulty_packets, 1);
}

#[test]
fn player_info_events_update_world_and_world_counters() {
    let profile_id = Uuid::from_u128(1);
    let removed_profile_id = Uuid::from_u128(2);
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerInfoUpdate(
        bbb_protocol::packets::PlayerInfoUpdate {
            actions: vec![
                bbb_protocol::packets::PlayerInfoAction::AddPlayer,
                bbb_protocol::packets::PlayerInfoAction::InitializeChat,
                bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
                bbb_protocol::packets::PlayerInfoAction::UpdateListed,
                bbb_protocol::packets::PlayerInfoAction::UpdateLatency,
                bbb_protocol::packets::PlayerInfoAction::UpdateDisplayName,
                bbb_protocol::packets::PlayerInfoAction::UpdateListOrder,
                bbb_protocol::packets::PlayerInfoAction::UpdateHat,
            ],
            entries: vec![
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id,
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: profile_id,
                        name: "Ada".to_string(),
                        properties: vec![bbb_protocol::packets::GameProfileProperty {
                            name: "textures".to_string(),
                            value: "skin".to_string(),
                            signature: Some("signature".to_string()),
                        }],
                    }),
                    listed: true,
                    latency: 42,
                    game_mode: bbb_protocol::packets::GameType::Creative,
                    display_name: Some("Ada Lovelace".to_string()),
                    show_hat: true,
                    list_order: 3,
                    chat_session: Some(bbb_protocol::packets::PlayerInfoChatSession {
                        session_id: Uuid::from_u128(3),
                        expires_at_epoch_millis: 99,
                        public_key: vec![1, 2],
                        key_signature: vec![3, 4],
                    }),
                },
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id: removed_profile_id,
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: removed_profile_id,
                        name: "Removed".to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 7,
                    game_mode: bbb_protocol::packets::GameType::Survival,
                    display_name: None,
                    show_hat: false,
                    list_order: 0,
                    chat_session: None,
                },
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerInfoRemove(
        bbb_protocol::packets::PlayerInfoRemove {
            profile_ids: vec![removed_profile_id],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let entry = world.player_info_entry(profile_id).unwrap();
    assert_eq!(entry.profile.uuid, profile_id);
    assert_eq!(entry.profile.name, "Ada");
    assert_eq!(entry.profile.properties.len(), 1);
    assert!(entry.listed);
    assert_eq!(entry.latency, 42);
    assert_eq!(entry.game_mode, "creative");
    assert_eq!(entry.display_name.as_deref(), Some("Ada Lovelace"));
    assert!(entry.show_hat);
    assert_eq!(entry.list_order, 3);
    assert!(entry.chat_session_present);
    assert!(world.listed_players().contains(&profile_id));
    assert!(world.player_info_entry(removed_profile_id).is_none());
    assert!(!world.listed_players().contains(&removed_profile_id));

    let world_counters = world.counters();
    assert_eq!(world_counters.player_info_update_packets, 1);
    assert_eq!(world_counters.player_info_remove_packets, 1);
    assert_eq!(world_counters.player_info_entries_tracked, 1);
    assert_eq!(world_counters.listed_players_tracked, 1);
}

#[test]
fn server_presentation_events_update_world_and_world_counters() {
    let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::ServerData(
        bbb_protocol::packets::ServerData {
            motd: "Native test server".to_string(),
            icon_bytes: Some(vec![1, 2, 3, 4]),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPush(
        bbb_protocol::packets::ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: Some("Install pack?".to_string()),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackResponse {
        id: pack_id,
        action: bbb_protocol::packets::ResourcePackResponseAction::Declined,
    })
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: None },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: Some(pack_id) },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );

    let server_data = world.server_data().unwrap();
    assert_eq!(server_data.motd, "Native test server");
    assert_eq!(server_data.icon_byte_len(), Some(4));
    assert!(world.resource_packs().is_empty());

    let world_counters = world.counters();
    assert_eq!(world_counters.server_data_packets, 1);
    assert_eq!(world_counters.resource_pack_push_packets, 1);
    assert_eq!(world_counters.resource_pack_response_packets, 1);
    assert_eq!(world_counters.resource_pack_response_updates_applied, 1);
    assert_eq!(world_counters.resource_pack_response_updates_ignored, 0);
    assert_eq!(world_counters.resource_pack_required_declines, 1);
    assert_eq!(world_counters.resource_pack_pop_packets, 2);
    assert_eq!(world_counters.resource_pack_pop_updates_applied, 1);
    assert_eq!(world_counters.resource_pack_pop_updates_ignored, 1);
    assert_eq!(world_counters.resource_packs_tracked, 0);
}

#[test]
fn entity_status_events_update_world_and_world_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::Cooldown(
        bbb_protocol::packets::Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id: 99,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateMobEffect(
        bbb_protocol::packets::UpdateMobEffect {
            entity_id,
            effect_id: 3,
            amplifier: 2,
            duration_ticks: 400,
            flags: bbb_protocol::packets::MobEffectFlags {
                raw: 0b1011,
                ambient: true,
                visible: true,
                show_icon: false,
                blend: true,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id,
            effect_id: 99,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    let cooldown = world.cooldown("minecraft:ender_pearl").unwrap();
    assert_eq!(cooldown.duration, 20);

    let damage = world.entity_last_damage(entity_id).unwrap();
    assert_eq!(damage.source_type_id, 5);
    assert_eq!(damage.source_cause_id, -1);
    assert_eq!(damage.source_direct_id, 42);
    assert_eq!(
        damage.source_position,
        Some(bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        })
    );

    let effect = world.entity_effect(entity_id, 3).unwrap();
    assert_eq!(effect.amplifier, 2);
    assert_eq!(effect.duration_ticks, 400);
    assert!(effect.ambient);
    assert!(effect.visible);
    assert!(!effect.show_icon);
    assert!(effect.blend);
    assert!(world.entity_effect(entity_id, 99).is_none());

    let world_counters = world.counters();
    assert_eq!(world_counters.cooldown_packets, 1);
    assert_eq!(world_counters.cooldowns_tracked, 1);
    assert_eq!(world_counters.damage_event_packets, 2);
    assert_eq!(world_counters.damage_events_applied, 1);
    assert_eq!(world_counters.damage_events_ignored, 1);
    assert_eq!(world_counters.update_mob_effect_packets, 1);
    assert_eq!(world_counters.update_mob_effects_ignored, 0);
    assert_eq!(world_counters.remove_mob_effect_packets, 1);
    assert_eq!(world_counters.remove_mob_effects_ignored, 1);
    assert_eq!(world_counters.active_mob_effects_tracked, 1);
}

#[test]
fn move_vehicle_event_updates_world_and_queues_ack() {
    let (event_tx, mut event_rx) = mpsc::channel(1);
    let (command_tx, mut command_rx) = mpsc::channel(1);
    let commands = Some(command_tx);
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(99));
    world.apply_add_entity(protocol_add_entity(10));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveVehicle(
            bbb_protocol::packets::MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &commands),
        1
    );

    assert_eq!(counters.move_vehicle_commands_queued, 1);
    assert_eq!(world.counters().vehicle_moves_received, 1);
    assert_eq!(world.counters().vehicle_moves_applied, 1);
    assert_eq!(world.counters().vehicle_moves_acked, 1);
    assert_eq!(world.counters().vehicle_moves_snapped, 1);
    assert_eq!(world.counters().vehicle_moves_ignored, 0);
    let vehicle = world.probe_entity(10).unwrap();
    assert_eq!(
        vehicle.position,
        bbb_world::EntityVec3 {
            x: 5.0,
            y: 66.0,
            z: -7.0,
        }
    );
    match command_rx.try_recv().unwrap() {
        NetCommand::MoveVehicle(command) => {
            assert_eq!(command.position.x, 5.0);
            assert_eq!(command.position.y, 66.0);
            assert_eq!(command.position.z, -7.0);
            assert_eq!(command.y_rot, 45.0);
            assert_eq!(command.x_rot, -5.0);
            assert!(!command.on_ground);
        }
        other => panic!("expected move vehicle command, got {other:?}"),
    }
}

#[test]
fn move_vehicle_ignored_counters_update_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(1);

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveVehicle(
            bbb_protocol::packets::MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            },
        )))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().vehicle_moves_received, 1);
    assert_eq!(world.counters().vehicle_moves_applied, 0);
    assert_eq!(world.counters().vehicle_moves_acked, 0);
    assert_eq!(world.counters().vehicle_moves_snapped, 0);
    assert_eq!(world.counters().vehicle_moves_ignored, 1);
}

#[test]
fn minecart_along_track_event_updates_world_state_and_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(1);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(10, 85));

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 10,
                lerp_steps: vec![MinecartStep {
                    position: ProtocolVec3d {
                        x: 2.0,
                        y: 64.25,
                        z: -3.0,
                    },
                    movement: ProtocolVec3d {
                        x: 0.3,
                        y: 0.0,
                        z: -0.3,
                    },
                    y_rot: 90.0,
                    x_rot: 5.0,
                    weight: 1.0,
                }],
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        1
    );

    let entity = world.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        bbb_world::EntityVec3 {
            x: 2.0,
            y: 64.25,
            z: -3.0,
        }
    );
    assert_eq!(entity.y_rot, 90.0);
    assert_eq!(entity.x_rot, 5.0);
    assert_eq!(world.counters().minecart_moves_received, 1);
    assert_eq!(world.counters().minecart_moves_applied, 1);
    assert_eq!(world.counters().minecart_lerp_steps_received, 1);
    assert_eq!(world.counters().minecart_lerp_steps_tracked, 1);
}

#[test]
fn minecart_along_track_ignored_counters_update_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(2);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(20));

    let step = MinecartStep {
        position: ProtocolVec3d {
            x: 2.0,
            y: 64.25,
            z: -3.0,
        },
        movement: ProtocolVec3d {
            x: 0.3,
            y: 0.0,
            z: -0.3,
        },
        y_rot: 90.0,
        x_rot: 5.0,
        weight: 1.0,
    };

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 999,
                lerp_steps: vec![step],
            },
        )))
        .unwrap();
    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 20,
                lerp_steps: vec![step],
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        2
    );

    assert_eq!(world.counters().minecart_moves_received, 2);
    assert_eq!(world.counters().minecart_moves_applied, 0);
    assert_eq!(world.counters().minecart_moves_ignored, 2);
    assert_eq!(world.counters().minecart_lerp_steps_received, 2);
    assert_eq!(world.counters().minecart_lerp_steps_tracked, 0);
}

#[test]
fn login_tracks_local_player_id_in_world() {
    let (tx, mut rx) = mpsc::channel(2);
    let respawn_info = protocol_play_login(9).common_spawn_info;
    tx.try_send(NetEvent::Play(PlayClientbound::Login(protocol_play_login(
        9,
    ))))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: respawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.local_player_id(), Some(9));
    assert_eq!(world.counters().play_logins_received, 1);
    assert_eq!(world.counters().respawns_received, 1);
}

#[test]
fn respawn_event_resets_local_player_runtime_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: protocol_play_login(9).common_spawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));
    world.apply_add_entity(protocol_add_entity(9));
    world.apply_add_entity(protocol_add_entity(55));
    world.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 4.0,
        food: 7,
        saturation: 0.5,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.25,
        level: 3,
        total: 40,
    });
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 55 }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: 65.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        },
        on_ground: true,
        horizontal_collision: true,
        fall_distance: 8.0,
        sneaking: true,
        swimming: true,
        y_rot: 90.0,
        x_rot: 20.0,
        last_teleport_id: 77,
    });
    world.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
    world.set_local_using_item(true);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 9,
        values: vec![EntityDataValue {
            data_id: 0,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0x02),
        }],
    }));
    assert!(world.apply_update_mob_effect(protocol_update_mob_effect(9, 3)));

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.local_player_id(), Some(9));
    assert_eq!(world.counters().respawns_received, 1);
    assert!(world.local_player().health.is_none());
    assert!(world.local_player().experience.is_none());
    assert_eq!(world.local_player_pose(), None);
    assert_eq!(
        world.local_player().camera,
        bbb_world::CameraState::default()
    );
    assert_eq!(
        world.local_player().interaction,
        bbb_world::LocalPlayerInteractionState::default()
    );
    let entity = world.probe_entity(9).unwrap();
    assert!(entity.data_values.is_empty());
    assert!(entity.mob_effects.is_empty());
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn player_position_and_rotation_events_update_world_pose() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerPosition(
        PlayerPositionUpdate {
            id: 23,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.125,
                y: 0.0,
                z: 0.25,
            },
            y_rot: 90.0,
            x_rot: 15.0,
            relatives_mask: 0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerRotation(
        PlayerRotationUpdate {
            y_rot: 10.0,
            relative_y: true,
            x_rot: -5.0,
            relative_x: false,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let pose = world.local_player_pose().unwrap();
    assert_eq!(
        pose.position,
        ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: -5.0,
        }
    );
    assert_eq!(
        pose.delta_movement,
        ProtocolVec3d {
            x: 0.125,
            y: 0.0,
            z: 0.25,
        }
    );
    assert_eq!(pose.y_rot, 100.0);
    assert_eq!(pose.x_rot, -5.0);
    assert_eq!(pose.last_teleport_id, 23);

    let world_counters = world.counters();
    assert_eq!(world_counters.player_position_packets, 1);
    assert_eq!(world_counters.player_rotation_packets, 1);
}

#[test]
fn local_player_events_update_world_state_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(10);
    tx.try_send(NetEvent::Play(PlayClientbound::Login(protocol_play_login(
        9,
    ))))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerAbilities(
        bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetHealth(
        bbb_protocol::packets::PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetExperience(
        bbb_protocol::packets::PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetHeldSlot(
        bbb_protocol::packets::SetHeldSlot { slot: 5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDefaultSpawnPosition(
        bbb_protocol::packets::SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetSimulationDistance(
        bbb_protocol::packets::SetSimulationDistance { distance: 12 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCamera(
        bbb_protocol::packets::SetCamera { camera_id: 9 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCamera(
        bbb_protocol::packets::SetCamera { camera_id: 123 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        9
    );

    let local = world.local_player();
    assert_eq!(local.health.unwrap().health, 7.5);
    assert_eq!(local.experience.unwrap().level, 8);
    assert_eq!(local.selected_hotbar_slot, 5);
    assert_eq!(local.simulation_distance, Some(12));
    assert_eq!(
        local.default_spawn.as_ref().map(|spawn| spawn.pos),
        Some(BlockPos {
            x: -5,
            y: 70,
            z: 12,
        })
    );
    assert_eq!(
        local.camera,
        bbb_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.player_abilities_packets, 1);
    assert_eq!(world_counters.player_health_packets, 1);
    assert_eq!(world_counters.player_experience_packets, 1);
    assert_eq!(world_counters.held_slot_packets, 1);
    assert_eq!(world_counters.held_slot_updates_applied, 1);
    assert_eq!(world_counters.held_slot_updates_ignored, 0);
    assert_eq!(world_counters.default_spawn_position_packets, 1);
    assert_eq!(world_counters.simulation_distance_packets, 1);
    assert_eq!(world_counters.set_camera_packets, 2);
    assert_eq!(world_counters.set_camera_updates_applied, 1);
    assert_eq!(world_counters.set_camera_updates_ignored, 1);
}

#[test]
fn world_time_and_weather_update_world_counters_and_clear_color() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::SetTime(
        bbb_protocol::packets::PlayTime {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 7,
            param: 0.5,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 3,
            param: 3.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 11,
            param: 1.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 12,
            param: 1.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TickingState(
        bbb_protocol::packets::TickingState {
            tick_rate: 0.25,
            frozen: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TickingStep(
        bbb_protocol::packets::TickingStep { tick_steps: 7 },
    )))
    .unwrap();

    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        7
    );

    assert_eq!(
        world.world_time(),
        Some(&bbb_world::WorldTimeState {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }
            .into()],
            day_time: 6000,
        })
    );
    assert_eq!(world.weather().rain_level, 0.5);
    assert_eq!(world.gameplay().game_type, 3);
    assert_eq!(world.gameplay().game_type_name, "spectator");
    assert_eq!(world.gameplay().previous_game_type, Some(0));
    assert!(!world.gameplay().show_death_screen);
    assert!(world.gameplay().do_limited_crafting);
    assert_eq!(
        world.ticking(),
        bbb_world::WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );

    assert_eq!(world.counters().world_time_packets, 1);
    assert_eq!(world.counters().game_event_packets, 4);
    assert_eq!(world.counters().ticking_state_packets, 1);
    assert_eq!(world.counters().ticking_step_packets, 1);

    let world_color = clear_color_for_world(&world, false);
    let expected_world_color = clear_color_for_day_time(6000, 0.5, 0.0);
    assert_eq!(world_color, expected_world_color);

    let day = clear_color_for_day_time(6000, 0.0, 0.0);
    let night = clear_color_for_day_time(18000, 0.0, 0.0);
    let storm = clear_color_for_day_time(6000, 1.0, 1.0);
    assert!(day.b > night.b);
    assert!(storm.r < day.r);
    assert!(storm.g < day.g);
    assert!(storm.b < day.b);
}
