use super::*;
use crate::runtime::clear_color_for_day_time;
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::packets::{
    AddEntity, BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo, PlayLogin, SetPassengers,
    Vec3d as ProtocolVec3d,
};
use bbb_world::{BlockPos, WorldStore};
use tokio::sync::mpsc;
use uuid::Uuid;

#[test]
fn block_changed_ack_updates_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_block_changed_ack(
        &mut counters,
        bbb_protocol::packets::BlockChangedAck { sequence: 17 },
    );

    assert_eq!(counters.block_changed_ack_packets, 1);
    assert_eq!(counters.last_block_changed_ack_sequence, Some(17));
}

#[test]
fn transfer_event_updates_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Transfer(bbb_protocol::packets::Transfer {
        host: "next.example.com".to_string(),
        port: 25566,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        counters.last_transfer,
        Some(bbb_control::TransferTarget {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );
    assert_eq!(counters.transfer_packets, 1);
}

#[test]
fn take_item_entity_event_updates_snapshot_counter() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::TakeItemEntity(
        bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 3,
        },
    ))
    .unwrap();
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.take_item_entity_packets, 1);
    assert_eq!(world.counters().take_item_entities_received, 1);
    assert_eq!(world.counters().take_item_entities_applied, 0);
}

#[test]
fn clear_titles_event_updates_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::SetTitlesAnimation(
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetTitleText(
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetSubtitleText(
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ClearTitles(bbb_protocol::packets::ClearTitles {
        reset_times: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::ClearTitles(bbb_protocol::packets::ClearTitles {
        reset_times: true,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.title, bbb_control::TitleState::default());
    assert_eq!(counters.clear_titles_packets, 2);
    assert_eq!(counters.title_text_packets, 1);
    assert_eq!(counters.subtitle_text_packets, 1);
    assert_eq!(counters.titles_animation_packets, 1);
}

#[test]
fn command_suggestions_event_updates_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::CommandSuggestions(
        bbb_protocol::packets::CommandSuggestions {
            id: 7,
            start: 1,
            length: 4,
            suggestions: vec![
                bbb_protocol::packets::CommandSuggestion {
                    text: "give".to_string(),
                    tooltip: Some("Run give".to_string()),
                },
                bbb_protocol::packets::CommandSuggestion {
                    text: "gamemode".to_string(),
                    tooltip: None,
                },
            ],
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.command_suggestion_packets, 1);
    assert_eq!(world.counters().command_suggestion_packets, 1);
    assert_eq!(world.counters().command_suggestion_entries_tracked, 2);

    let result = world.command_suggestions_by_id(7).unwrap();
    assert_eq!(result.start, 1);
    assert_eq!(result.length, 4);
    assert_eq!(result.suggestions.len(), 2);
    assert_eq!(result.suggestions[0].text, "give");
    assert_eq!(result.suggestions[0].tooltip.as_deref(), Some("Run give"));
    assert_eq!(world.last_command_suggestions(), Some(result));
}

#[test]
fn block_destruction_event_updates_world_and_counter() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.block_destruction_packets, 1);
    assert_eq!(world.counters().block_destructions_received, 1);
    assert_eq!(world.counters().block_destructions_tracked, 1);
    assert_eq!(world.block_destruction(4).unwrap().progress, 6);
}

#[test]
fn block_and_level_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::BlockEvent(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    }))
    .unwrap();
    tx.try_send(NetEvent::LevelEvent(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(counters.block_event_packets, 1);
    assert_eq!(counters.level_event_packets, 1);

    let world_counters = world.counters();
    assert_eq!(world_counters.block_events_received, 1);
    assert_eq!(world_counters.block_events_tracked, 1);
    assert_eq!(world_counters.level_events_received, 1);
    assert_eq!(world_counters.level_events_tracked, 1);

    let block_event = world.block_events().first().unwrap();
    assert_eq!(
        block_event.pos,
        BlockPos {
            x: 12,
            y: 65,
            z: -5
        }
    );
    assert_eq!(block_event.b0, 2);
    assert_eq!(block_event.b1, 9);
    assert_eq!(block_event.block_id, 54);

    let level_event = world.level_events().first().unwrap();
    assert_eq!(level_event.event_type, 1001);
    assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
    assert_eq!(level_event.data, 42);
    assert!(level_event.global);
}

#[test]
fn border_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::InitializeBorder(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderCenter(
        bbb_protocol::packets::SetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderLerpSize(
        bbb_protocol::packets::SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderSize(
        bbb_protocol::packets::SetBorderSize { size: 250.0 },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderWarningDelay(
        bbb_protocol::packets::SetBorderWarningDelay { warning_delay: 9 },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderWarningDistance(
        bbb_protocol::packets::SetBorderWarningDistance { warning_blocks: 8 },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );
    assert_eq!(counters.initialize_border_packets, 1);
    assert_eq!(counters.set_border_center_packets, 1);
    assert_eq!(counters.set_border_lerp_size_packets, 1);
    assert_eq!(counters.set_border_size_packets, 1);
    assert_eq!(counters.set_border_warning_delay_packets, 1);
    assert_eq!(counters.set_border_warning_distance_packets, 1);

    let border = world.world_border();
    assert_eq!(border.center_x, 3.0);
    assert_eq!(border.center_z, 4.0);
    assert_eq!(border.size, 250.0);
    assert_eq!(border.lerp_target, 250.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.absolute_max_size, 500);
    assert_eq!(border.warning_blocks, 8);
    assert_eq!(border.warning_time, 9);
}

#[test]
fn scoreboard_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "kills".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Add,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Kills".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: Some(vec![9]),
            }),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
        owner: "Steve".to_string(),
        objective_name: "kills".to_string(),
        score: 4,
        display: Some("Four".to_string()),
        number_format: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
        owner: "Alex".to_string(),
        objective_name: "kills".to_string(),
        score: 1,
        display: None,
        number_format: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetPlayerTeam(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::ResetScore(bbb_protocol::packets::ResetScore {
        owner: "Alex".to_string(),
        objective_name: Some("kills".to_string()),
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );
    assert_eq!(counters.set_objective_packets, 1);
    assert_eq!(counters.set_display_objective_packets, 1);
    assert_eq!(counters.set_score_packets, 2);
    assert_eq!(counters.set_player_team_packets, 1);
    assert_eq!(counters.reset_score_packets, 1);

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
fn hud_session_events_update_world_and_counters() {
    let boss_id = Uuid::from_u128(1);
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
        id: boss_id,
        operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 0.25 },
    }))
    .unwrap();
    tx.try_send(NetEvent::TabList(bbb_protocol::packets::TabList {
        header: Some("Welcome".to_string()),
        footer: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::ChangeDifficulty(
        bbb_protocol::packets::ChangeDifficulty {
            difficulty: bbb_protocol::packets::Difficulty::Hard,
            locked: true,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(counters.boss_event_packets, 2);
    assert_eq!(counters.tab_list_packets, 1);
    assert_eq!(counters.change_difficulty_packets, 1);

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
    assert_eq!(world_counters.boss_event_packets, 2);
    assert_eq!(world_counters.boss_bars_tracked, 1);
    assert_eq!(world_counters.tab_list_packets, 1);
    assert_eq!(world_counters.change_difficulty_packets, 1);
}

#[test]
fn player_info_events_update_world_and_counters() {
    let profile_id = Uuid::from_u128(1);
    let removed_profile_id = Uuid::from_u128(2);
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::PlayerInfoUpdate(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::PlayerInfoRemove(
        bbb_protocol::packets::PlayerInfoRemove {
            profile_ids: vec![removed_profile_id],
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(counters.player_info_update_packets, 1);
    assert_eq!(counters.player_info_remove_packets, 1);

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
fn server_presentation_events_update_world_and_counters() {
    let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::ServerData(bbb_protocol::packets::ServerData {
        motd: "Native test server".to_string(),
        icon_bytes: Some(vec![1, 2, 3, 4]),
    }))
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
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: None },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(counters.server_data_packets, 1);
    assert_eq!(counters.resource_pack_push_packets, 1);
    assert_eq!(counters.resource_pack_pop_packets, 1);

    let server_data = world.server_data().unwrap();
    assert_eq!(server_data.motd, "Native test server");
    assert_eq!(server_data.icon_byte_len(), Some(4));
    assert!(world.resource_packs().is_empty());

    let world_counters = world.counters();
    assert_eq!(world_counters.server_data_packets, 1);
    assert_eq!(world_counters.resource_pack_push_packets, 1);
    assert_eq!(world_counters.resource_pack_pop_packets, 1);
    assert_eq!(world_counters.resource_packs_tracked, 0);
}

#[test]
fn entity_status_events_update_world_and_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    }))
    .unwrap();
    tx.try_send(NetEvent::DamageEvent(bbb_protocol::packets::DamageEvent {
        entity_id,
        source_type_id: 5,
        source_cause_id: -1,
        source_direct_id: 42,
        source_position: Some(bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }),
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateMobEffect(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id,
            effect_id: 99,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(counters.cooldown_packets, 1);
    assert_eq!(counters.damage_event_packets, 1);
    assert_eq!(counters.update_mob_effect_packets, 1);
    assert_eq!(counters.remove_mob_effect_packets, 1);

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
    assert_eq!(world_counters.damage_event_packets, 1);
    assert_eq!(world_counters.damage_events_applied, 1);
    assert_eq!(world_counters.update_mob_effect_packets, 1);
    assert_eq!(world_counters.remove_mob_effect_packets, 1);
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
        .try_send(NetEvent::MoveVehicle(bbb_protocol::packets::MoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &commands),
        1
    );

    assert_eq!(counters.move_vehicle_packets, 1);
    assert_eq!(counters.move_vehicle_commands_queued, 1);
    assert_eq!(world.counters().vehicle_moves_snapped, 1);
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
fn world_time_and_weather_update_snapshot_and_clear_color() {
    let mut counters = NetCounters::default();

    apply_world_time_update(
        &mut counters,
        bbb_protocol::packets::PlayTime {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        },
    );
    apply_game_event(
        &mut counters,
        bbb_protocol::packets::GameEvent {
            event_id: 7,
            param: 0.5,
        },
    );

    assert_eq!(
        counters.world_time,
        Some(bbb_control::WorldTime {
            game_time: 123,
            day_time: 6000,
            clock_updates: 1,
        })
    );
    assert!(counters.weather.raining);
    assert_eq!(counters.weather.rain_level, 0.5);
    assert_eq!(counters.world_time_packets, 1);
    assert_eq!(counters.game_event_packets, 1);

    let day = clear_color_for_day_time(6000, 0.0, 0.0);
    let night = clear_color_for_day_time(18000, 0.0, 0.0);
    let storm = clear_color_for_day_time(6000, 1.0, 1.0);
    assert!(day.b > night.b);
    assert!(storm.r < day.r);
    assert!(storm.g < day.g);
    assert!(storm.b < day.b);
}

fn protocol_play_login(player_id: i32) -> PlayLogin {
    PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}

fn protocol_add_entity(id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: 7,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}
