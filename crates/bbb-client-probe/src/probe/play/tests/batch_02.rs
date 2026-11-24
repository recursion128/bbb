use super::*;

#[tokio::test]
async fn probe_applies_client_ui_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LowDiskSpaceWarning)
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: bbb_protocol::NbtSummaryValue::Compound(Vec::new()),
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ClearDialog)
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(AddEntity {
            id: 42,
            uuid: Uuid::from_u128(42),
            entity_type_id: VANILLA_ENTITY_TYPE_HORSE_ID,
            position: ProtocolVec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::MountScreenOpen(MountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::OpenBook(OpenBook {
            hand: InteractionHand::OffHand,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::OpenSignEditor(OpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlaceGhostRecipe(PlaceGhostRecipe {
            container_id: 9,
            recipe_display: packets::RecipeDisplaySummary {
                display_type: packets::RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                crafting: None,
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PongResponse(PongResponse {
            time: 123456789,
        }))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let ui = report.world.client_ui();

    assert_eq!(ui.low_disk_space_warning_count, 1);
    assert_eq!(
        ui.last_mount_screen,
        Some(bbb_client_world::MountScreenState {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        })
    );
    assert_eq!(
        ui.last_open_book,
        Some(bbb_client_world::OpenBookState {
            hand: "off_hand".to_string()
        })
    );
    assert_eq!(ui.last_ghost_recipe, None);
    assert_eq!(
        ui.last_pong_response,
        Some(bbb_client_world::PongResponseState { time: 123456789 })
    );
    assert_eq!(report.world_counters.low_disk_space_warnings, 1);
    assert_eq!(report.world_counters.show_dialog_packets, 1);
    assert_eq!(report.world_counters.clear_dialog_packets, 1);
    assert_eq!(report.world_counters.mount_screen_open_packets, 1);
    assert_eq!(report.world_counters.open_book_packets, 1);
    assert_eq!(report.world_counters.open_sign_editor_packets, 1);
    assert_eq!(report.world_counters.open_sign_editor_updates_applied, 0);
    assert_eq!(report.world_counters.open_sign_editor_updates_ignored, 1);
    assert_eq!(report.world_counters.ghost_recipe_packets, 1);
    assert_eq!(report.world_counters.ghost_recipe_updates_applied, 0);
    assert_eq!(report.world_counters.ghost_recipe_updates_ignored, 1);
    assert_eq!(report.world_counters.pong_response_packets, 1);
}

#[tokio::test]
async fn probe_applies_recipe_book_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let settings = RecipeBookSettings {
        crafting: RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: RecipeBookTypeSettings {
            open: false,
            filtering: true,
        },
        blast_furnace: RecipeBookTypeSettings {
            open: true,
            filtering: true,
        },
        smoker: RecipeBookTypeSettings {
            open: false,
            filtering: false,
        },
    };

    probe
        .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
            replace: true,
            entries: vec![
                probe_recipe_entry(7, true, true),
                probe_recipe_entry(8, false, false),
            ],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
            replace: false,
            entries: vec![probe_recipe_entry(9, true, false)],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RecipeBookRemove(RecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 7 }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::RecipeBookSettings(settings))
        .await
        .unwrap();

    let report = probe.finish(5, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let recipe_book = report.world.recipe_book();

    assert!(!recipe_book.known.contains_key(&7));
    assert_eq!(
        recipe_book.known.get(&8).unwrap().category_id,
        probe_recipe_category_id(8)
    );
    assert_eq!(
        recipe_book.known.get(&9).unwrap().category_id,
        probe_recipe_category_id(9)
    );
    assert!(recipe_book.highlights.is_empty());
    assert_eq!(recipe_book.notification_ids, vec![9]);
    assert_eq!(recipe_book.settings, settings);

    assert_eq!(report.world_counters.recipe_book_add_packets, 2);
    assert_eq!(report.world_counters.recipe_book_replace_packets, 1);
    assert_eq!(report.world_counters.recipe_book_remove_packets, 1);
    assert_eq!(report.world_counters.recipe_book_settings_packets, 1);
    assert_eq!(report.world_counters.recipe_book_entries_received, 3);
    assert_eq!(
        report.world_counters.recipe_book_removed_entries_received,
        1
    );
    assert_eq!(report.world_counters.recipe_book_entries_tracked, 2);
    assert_eq!(report.world_counters.recipe_book_highlights_tracked, 0);
    assert_eq!(report.world_counters.recipe_book_notifications_received, 2);
}

#[tokio::test]
async fn probe_applies_advancement_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::UpdateAdvancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: vec![vec!["mine_stone".into(), "get_log".into()]],
                sends_telemetry_event: true,
            }],
            removed: Vec::new(),
            progress: vec![AdvancementProgressSummary {
                id: "minecraft:story/root".to_string(),
                criteria: vec![AdvancementCriterionProgressSummary {
                    name: "mine_stone".into(),
                    obtained_epoch_millis: Some(1_700_000_000_000),
                }],
            }],
            show_advancements: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SelectAdvancementsTab(
            SelectAdvancementsTab {
                tab: Some("minecraft:story/root".to_string()),
            },
        ))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert!(report
        .world
        .advancements()
        .advancements
        .contains_key("minecraft:story/root"));
    let progress = report
        .world
        .advancements()
        .progress
        .get("minecraft:story/root")
        .unwrap();
    assert_eq!(progress.criteria.len(), 2);

    assert_eq!(report.world_counters.update_advancements_packets, 1);
    assert_eq!(report.world_counters.update_advancements_reset_packets, 1);
    assert_eq!(report.world_counters.update_advancements_show_packets, 1);
    assert_eq!(report.world_counters.advancements_added_received, 1);
    assert_eq!(report.world_counters.advancements_removed_received, 0);
    assert_eq!(report.world_counters.advancements_adds_ignored, 0);
    assert_eq!(report.world_counters.advancement_progress_received, 1);
    assert_eq!(
        report.world_counters.advancement_progress_updates_ignored,
        0
    );
    assert_eq!(report.world_counters.advancements_tracked, 1);
    assert_eq!(report.world_counters.advancement_roots_tracked, 1);
    assert_eq!(report.world_counters.advancement_progress_tracked, 1);
    assert_eq!(
        report.world_counters.advancement_progress_criteria_tracked,
        2
    );
    assert_eq!(report.world_counters.select_advancements_tab_packets, 1);
}

#[tokio::test]
async fn probe_applies_update_recipes_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::UpdateRecipes(UpdateRecipes {
            property_sets: vec![
                RecipePropertySetSummary {
                    key: "minecraft:furnace_input".to_string(),
                    item_ids: vec![42, 43],
                },
                RecipePropertySetSummary {
                    key: "minecraft:smithing_base".to_string(),
                    item_ids: vec![99],
                },
            ],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11, 12],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                    raw_payload: vec![4, 77],
                    item_stack: None,
                    tag: None,
                },
            }],
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let recipes = report.world.recipes();

    assert_eq!(
        recipes.property_sets.get("minecraft:furnace_input"),
        Some(&vec![42, 43])
    );
    assert_eq!(
        recipes.property_sets.get("minecraft:smithing_base"),
        Some(&vec![99])
    );
    assert_eq!(recipes.stonecutter_recipes.len(), 1);
    assert_eq!(recipes.stonecutter_recipes[0].input.item_ids, vec![11, 12]);
    assert_eq!(
        recipes.stonecutter_recipes[0]
            .option_display
            .display_type_id,
        VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID
    );
    assert_eq!(
        recipes.stonecutter_recipes[0].option_display.raw_payload,
        vec![4, 77]
    );

    assert_eq!(report.world_counters.update_recipes_packets, 1);
    assert_eq!(report.world_counters.recipe_property_sets_tracked, 2);
    assert_eq!(report.world_counters.recipe_property_set_items_tracked, 3);
    assert_eq!(report.world_counters.stonecutter_recipes_tracked, 1);
}

#[tokio::test]
async fn probe_applies_world_time_weather_and_ticking_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetTime(PlayTime {
            game_time: 123,
            clock_updates: vec![ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
            event_id: 7,
            param: 0.5,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
            event_id: 8,
            param: 0.75,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
            event_id: 3,
            param: 2.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
            event_id: 11,
            param: 1.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
            event_id: 12,
            param: 1.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TickingState(TickingState {
            tick_rate: 0.25,
            frozen: true,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TickingStep(TickingStep { tick_steps: 7 }))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let time = report.world.world_time().unwrap();

    assert_eq!(time.game_time, 123);
    assert_eq!(time.day_time, 6000);
    assert_eq!(
        time.clock_updates,
        vec![bbb_client_world::ClockUpdateState {
            clock_id: 0,
            total_ticks: 6000,
            partial_tick: 0.25,
            rate: 1.0,
        }]
    );
    assert_eq!(
        report.world.weather(),
        bbb_client_world::WorldWeatherState {
            raining: true,
            rain_level: 0.5,
            thunder_level: 0.75,
            last_game_event_id: Some(12),
            last_game_event_param: 1.0,
        }
    );
    assert_eq!(report.world.gameplay().game_type, 2);
    assert_eq!(report.world.gameplay().game_type_name, "adventure");
    assert_eq!(report.world.gameplay().previous_game_type, Some(0));
    assert!(!report.world.gameplay().show_death_screen);
    assert!(report.world.gameplay().do_limited_crafting);
    assert_eq!(
        report.world.ticking(),
        bbb_client_world::WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );
    assert_eq!(report.world_counters.world_time_packets, 1);
    assert_eq!(report.world_counters.game_event_packets, 5);
    assert_eq!(report.world_counters.ticking_state_packets, 1);
    assert_eq!(report.world_counters.ticking_step_packets, 1);
}

#[tokio::test]
async fn probe_applies_world_border_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::InitializeBorder(InitializeBorder {
            new_center_x: 1.0,
            new_center_z: 2.0,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 30,
            new_absolute_max_size: 500,
            warning_blocks: 6,
            warning_time: 7,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetBorderCenter(SetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetBorderSize(SetBorderSize {
            size: 250.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetBorderWarningDelay(
            SetBorderWarningDelay { warning_delay: 9 },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetBorderWarningDistance(
            SetBorderWarningDistance { warning_blocks: 8 },
        ))
        .await
        .unwrap();

    let report = probe.finish(7, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let border = report.world.world_border();

    assert_eq!(border.center_x, 3.0);
    assert_eq!(border.center_z, 4.0);
    assert_eq!(border.size, 250.0);
    assert_eq!(border.lerp_target, 250.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.absolute_max_size, 500);
    assert_eq!(border.warning_blocks, 8);
    assert_eq!(border.warning_time, 9);

    assert_eq!(report.world_counters.world_border_initializes_received, 1);
    assert_eq!(
        report.world_counters.world_border_center_updates_received,
        1
    );
    assert_eq!(
        report
            .world_counters
            .world_border_lerp_size_updates_received,
        1
    );
    assert_eq!(report.world_counters.world_border_size_updates_received, 1);
    assert_eq!(
        report
            .world_counters
            .world_border_warning_delay_updates_received,
        1
    );
    assert_eq!(
        report
            .world_counters
            .world_border_warning_distance_updates_received,
        1
    );
}

#[tokio::test]
async fn probe_open_book_records_the_packet_and_discards_the_screen_request() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetPlayerInventory(SetPlayerInventory {
            slot: 40,
            item: written_book_stack(vec!["Probe first", "Probe second"]),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::OpenBook(OpenBook {
            hand: InteractionHand::OffHand,
        }))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.last_open_book(),
        Some(&bbb_client_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert!(report.world.screen_requests().is_empty());
}

#[tokio::test]
async fn probe_applies_hud_text_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SystemChat(SystemChat::plain(
            "Server restart soon",
            false,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SystemChat(SystemChat::plain(
            "Now entering camp",
            true,
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetActionBarText(SetActionBarText::plain(
            "Action ready",
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetTitleText(SetTitleText::plain(
            "Quest complete",
        )))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetSubtitleText(SetSubtitleText::plain(
            "Return to camp",
        )))
        .await
        .unwrap();

    let system_chat = probe.world.system_chat().unwrap();
    assert_eq!(system_chat.content, "Now entering camp");
    assert!(system_chat.overlay);
    assert_eq!(probe.world.client_chat().messages.len(), 1);
    assert_eq!(
        probe.world.client_chat().messages[0].kind,
        bbb_client_world::ChatMessageKind::System
    );
    assert_eq!(
        probe.world.client_chat().messages[0].content,
        "Server restart soon"
    );
    let action_bar = probe.world.action_bar().unwrap();
    assert_eq!(action_bar.content.text(), "Action ready");
    assert_eq!(action_bar.display_ticks, 60);
    assert_eq!(
        probe
            .world
            .title()
            .title
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Quest complete")
    );
    assert_eq!(
        probe
            .world
            .title()
            .subtitle
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Return to camp")
    );
    assert_eq!(probe.world.title().fade_in, 5);
    assert_eq!(probe.world.title().stay, 40);
    assert_eq!(probe.world.title().fade_out, 15);
    assert_eq!(probe.world.title().title_time, 60);

    probe
        .handle_play_packet(PlayClientbound::ClearTitles(ClearTitles {
            reset_times: true,
        }))
        .await
        .unwrap();

    let report = probe.finish(7, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world.title().title, None);
    assert_eq!(report.world.title().subtitle, None);
    assert_eq!(report.world.title().fade_in, 10);
    assert_eq!(report.world.title().stay, 70);
    assert_eq!(report.world.title().fade_out, 20);
    assert_eq!(report.world.title().title_time, 0);
    assert_eq!(report.world_counters.system_chat_packets, 2);
    assert_eq!(report.world_counters.action_bar_packets, 1);
    assert_eq!(report.world_counters.titles_animation_packets, 1);
    assert_eq!(report.world_counters.title_text_packets, 1);
    assert_eq!(report.world_counters.subtitle_text_packets, 1);
    assert_eq!(report.world_counters.clear_titles_packets, 1);
}

#[tokio::test]
async fn probe_applies_hud_session_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let boss_id = Uuid::from_u128(1);

    probe
        .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
            id: boss_id,
            operation: BossEventOperation::Add {
                name: Box::new(bbb_protocol::RichTextComponent::plain("Ender Dragon")),
                progress: 0.75,
                color: BossBarColor::Purple,
                overlay: BossBarOverlay::Progress,
                flags: BossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
            id: boss_id,
            operation: BossEventOperation::UpdateProgress { progress: 0.25 },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
            id: Uuid::from_u128(99),
            operation: BossEventOperation::UpdateProgress { progress: 1.0 },
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::TabList(TabList {
            header: bbb_protocol::RichTextComponent::plain("Welcome"),
            footer: bbb_protocol::RichTextComponent::plain(""),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ChangeDifficulty(ChangeDifficulty {
            difficulty: Difficulty::Hard,
            locked: true,
        }))
        .await
        .unwrap();

    let report = probe.finish(5, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let boss = report.world.boss_bar(boss_id).unwrap();

    assert_eq!(boss.name.text(), "Ender Dragon");
    assert_eq!(boss.progress, 0.25);
    assert_eq!(boss.color, "purple");
    assert_eq!(boss.overlay, "progress");
    assert!(boss.darken_screen);
    assert!(boss.create_world_fog);
    assert_eq!(
        report
            .world
            .tab_list()
            .header
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Welcome")
    );
    assert_eq!(report.world.tab_list().footer, None);
    assert_eq!(report.world.difficulty().difficulty, "hard");
    assert!(report.world.difficulty().difficulty_locked);

    assert_eq!(report.world_counters.boss_event_packets, 3);
    assert_eq!(report.world_counters.boss_bars_tracked, 1);
    assert_eq!(report.world_counters.boss_events_ignored, 1);
    assert_eq!(report.world_counters.tab_list_packets, 1);
    assert_eq!(report.world_counters.change_difficulty_packets, 1);
}

#[tokio::test]
async fn probe_applies_scoreboard_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
            objective_name: "kills".into(),
            method: SetObjectiveMethod::Add.into(),
            parameters: Some(SetObjectiveParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Kills"),
                render_type: ObjectiveRenderType::Integer,
                number_format: Some(bbb_protocol::packets::NumberFormatSummary::Blank),
            }),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".into()),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetScore(SetScore {
            owner: "Steve".into(),
            objective_name: "kills".into(),
            score: 4,
            display: Some(bbb_protocol::RichTextComponent::plain("Four")),
            number_format: None,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetScore(SetScore {
            owner: "Alex".into(),
            objective_name: "kills".into(),
            score: 1,
            display: None,
            number_format: None,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
            name: "red".into(),
            method: PlayerTeamMethod::Add.into(),
            parameters: Some(PlayerTeamParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Red Team"),
                options: 0b11,
                nametag_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::Never,
                color: ChatFormatting::Red,
                player_prefix: bbb_protocol::RichTextComponent::plain("[R]"),
                player_suffix: bbb_protocol::RichTextComponent::plain("!"),
            }),
            players: vec!["Steve".into()],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
            owner: "Alex".into(),
            objective_name: Some("kills".into()),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
            objective_name: "missing".into(),
            method: SetObjectiveMethod::Change.into(),
            parameters: Some(SetObjectiveParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Missing"),
                render_type: ObjectiveRenderType::Integer,
                number_format: None,
            }),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
            slot: ScoreboardDisplaySlot::List,
            objective_name: Some("missing".into()),
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetScore(SetScore {
            owner: "Nobody".into(),
            objective_name: "missing".into(),
            score: 9,
            display: None,
            number_format: None,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
            name: "missing".into(),
            method: PlayerTeamMethod::Join.into(),
            parameters: None,
            players: vec!["Nobody".into()],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
            owner: "Nobody".into(),
            objective_name: Some("missing".into()),
        }))
        .await
        .unwrap();

    let report = probe.finish(11, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let counters = &report.world_counters;
    assert_eq!(counters.set_objective_packets, 2);
    assert_eq!(counters.set_objective_updates_applied, 1);
    assert_eq!(counters.set_objective_updates_ignored, 1);
    assert_eq!(counters.set_display_objective_packets, 2);
    assert_eq!(counters.set_display_objective_updates_applied, 1);
    assert_eq!(counters.set_display_objective_updates_ignored, 1);
    assert_eq!(counters.set_score_packets, 3);
    assert_eq!(counters.set_score_updates_applied, 2);
    assert_eq!(counters.set_score_updates_ignored, 1);
    assert_eq!(counters.set_player_team_packets, 2);
    assert_eq!(counters.set_player_team_updates_applied, 1);
    assert_eq!(counters.set_player_team_updates_ignored, 1);
    assert_eq!(counters.reset_score_packets, 2);
    assert_eq!(counters.reset_score_updates_applied, 1);
    assert_eq!(counters.reset_score_updates_ignored, 1);

    let scoreboard = report.world.scoreboard();
    let objective = scoreboard.objectives.get("kills").unwrap();
    assert_eq!(objective.display_name, "Kills");
    assert_eq!(objective.render_type, "integer");
    assert_eq!(
        objective.number_format,
        Some(bbb_protocol::packets::NumberFormatSummary::Blank)
    );
    assert_eq!(
        scoreboard.display_slots.get("sidebar").map(String::as_str),
        Some("kills")
    );

    let steve_score = &scoreboard.scores["Steve"]["kills"];
    assert_eq!(steve_score.value, 4);
    assert_eq!(steve_score.display.as_deref(), Some("Four"));
    assert!(!scoreboard.scores.contains_key("Alex"));

    let team = scoreboard.teams.get("red").unwrap();
    assert!(team.players.contains("Steve"));
    let parameters = team.parameters.as_ref().unwrap();
    assert_eq!(parameters.display_name, "Red Team");
    assert_eq!(parameters.color, "red");
}

#[tokio::test]
async fn probe_applies_block_changed_ack_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::BlockChangedAck(BlockChangedAck {
            sequence: 17,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.block_changed_ack_packets, 1);
    assert_eq!(
        report.world.last_block_changed_ack(),
        Some(&bbb_client_world::BlockChangedAckState { sequence: 17 })
    );
}

#[tokio::test]
async fn probe_applies_block_and_level_events_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::BlockEvent(BlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.block_events_received, 1);
    assert_eq!(report.world_counters.block_events_tracked, 1);
    assert_eq!(report.world_counters.level_events_received, 1);
    assert_eq!(report.world_counters.level_events_tracked, 1);

    let block_event = report.world.block_events().first().unwrap();
    assert_eq!(
        block_event.pos,
        BlockPos {
            x: 12,
            y: 65,
            z: -5,
        }
    );
    assert_eq!(block_event.b0, 2);
    assert_eq!(block_event.b1, 9);
    assert_eq!(block_event.block_id, 54);

    let level_event = report.world.level_events().first().unwrap();
    assert_eq!(level_event.event_type, 1001);
    assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
    assert_eq!(level_event.data, 42);
    assert!(level_event.global);
}

#[tokio::test]
async fn probe_records_portal_travel_level_event_as_local_sound() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1032,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 0,
            global: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.level_events_received, 1);
    let sound = report.world.last_local_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.portal.travel")
    );
    assert_eq!(sound.source, "ambient");
    assert_close(sound.volume, 0.25);
    assert_close(sound.pitch, 1.092_387_1);
    assert_eq!(sound.seed, 0);
}

#[tokio::test]
async fn probe_records_positioned_level_event_sound_in_world_audio_state() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1015,
            pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
            data: 0,
            global: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.level_events_received, 1);
    assert_eq!(report.world_counters.sound_packets, 0);
    let sound = report.world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.ghast.warn")
    );
    assert_eq!(sound.source, "hostile");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: -3.5,
            y: 70.5,
            z: 9.5,
        }
    );
    assert_close(sound.volume, 10.0);
    assert_close(sound.pitch, 0.979_905_37);
    assert_eq!(sound.seed, 4_437_113_781_045_784_766);
}

#[tokio::test]
async fn probe_records_growth_level_event_sound_without_particle_sink() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let event_pos = ProtocolBlockPos {
        x: 16,
        y: -63,
        z: -32,
    };
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);

    probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BlockUpdate(BlockUpdate {
            pos: event_pos,
            block_state_id: water_id,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1505,
            pos: event_pos,
            data: 3,
            global: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 1, z: -2 }, Vec::new(), None, None);
    assert_eq!(report.world_counters.level_events_received, 1);
    let sound = report.world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:item.bone_meal.use")
    );
}

#[tokio::test]
async fn probe_records_cobweb_place_sound_without_particle_sink() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 3018,
            pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
            data: 0,
            global: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.level_events_received, 1);
    assert_eq!(report.world_counters.sound_packets, 0);
    let sound = report.world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.cobweb.place")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(
        sound.position,
        ProtocolVec3d {
            x: 2.5,
            y: 64.5,
            z: -4.5,
        }
    );
    assert_close(sound.volume, 1.0);
    assert!((0.8..=1.2).contains(&sound.pitch));
    assert!(sound.distance_delay);
}

#[tokio::test]
async fn probe_records_global_level_event_sound_when_camera_pose_is_known() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: -1.12,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });

    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1028,
            pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
            data: 0,
            global: true,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.level_events_received, 1);
    assert_eq!(report.world_counters.sound_packets, 0);
    let sound = report.world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.death")
    );
    assert_eq!(sound.source, "hostile");
    assert!((sound.position.x - 2.5).abs() < 1.0e-6);
    assert!((sound.position.y - 0.5).abs() < 1.0e-6);
    assert!((sound.position.z - 0.5).abs() < 1.0e-6);
    assert_close(sound.volume, 5.0);
    assert_close(sound.pitch, 1.0);
    assert_eq!(sound.seed, -4_962_768_465_676_381_896);
}

#[tokio::test]
async fn probe_records_jukebox_level_events_in_world_audio_state() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
            event_type: 1010,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 12,
            global: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.level_events_received, 1);
    assert_eq!(report.world.playing_jukebox_songs().len(), 1);
    let song = report.world.playing_jukebox_songs()[0];
    assert_eq!(song.pos, BlockPos { x: 3, y: 4, z: 5 });
    assert_eq!(song.song_registry_id, 12);
    let event = report.world.last_jukebox_event().unwrap();
    assert_eq!(
        event.action,
        bbb_client_world::JukeboxLevelEventAction::Start
    );
    assert_eq!(event.song_registry_id, Some(12));
}

#[tokio::test]
async fn probe_applies_player_look_at_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::PlayerLookAt(PlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 12.0,
                y: 65.0,
                z: -7.0,
            },
            target: None,
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.player_look_at_packets, 1);
    assert_eq!(
        report.world.local_player().last_look_at,
        Some(bbb_client_world::LocalPlayerLookAtState {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 12.0,
                y: 65.0,
                z: -7.0,
            },
            target_entity_id: None,
            to_anchor: None,
        })
    );
}

#[tokio::test]
async fn probe_applies_player_combat_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::PlayerCombatEnter)
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlayerCombatEnd(PlayerCombatEnd {
            duration: 37,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlayerCombatKill(PlayerCombatKill {
            player_id: 123,
            message: bbb_protocol::RichTextComponent::plain("You died"),
        }))
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(report.world_counters.player_combat_enter_packets, 1);
    assert_eq!(report.world_counters.player_combat_end_packets, 1);
    assert_eq!(report.world_counters.player_combat_kill_packets, 1);
    assert_eq!(
        report.world.last_player_combat(),
        Some(&bbb_client_world::PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(123),
            message: Some(bbb_protocol::RichTextComponent::plain("You died")),
        })
    );
}

#[tokio::test]
async fn probe_applies_player_health_position_and_rotation_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
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
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
            y_rot: 10.0,
            relative_y: true,
            x_rot: -5.0,
            relative_x: false,
        }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let local = report.world.local_player();
    assert_eq!(
        local.health,
        Some(bbb_client_world::LocalPlayerHealthState {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        })
    );
    assert_eq!(report.world_counters.player_health_packets, 1);
    assert_eq!(report.world_counters.player_position_packets, 1);
    assert_eq!(report.world_counters.player_rotation_packets, 1);

    let pose = report.world.local_player_pose().unwrap();
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
}

#[tokio::test]
async fn probe_player_position_echoes_unchanged_mounted_pose() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(10)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }))
        .await
        .unwrap();
    let initial_pose = LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 3.0,
            y: 66.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.0,
            z: -0.2,
        },
        no_jump_delay_ticks: 0,
        y_rot: 25.0,
        x_rot: -10.0,
        last_teleport_id: 2,
        ..LocalPlayerPoseState::default()
    };
    probe.world.set_local_player_pose(initial_pose);

    probe
        .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
            id: 17,
            position: ProtocolVec3d {
                x: 100.0,
                y: 80.0,
                z: 50.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 180.0,
            x_rot: 45.0,
            relatives_mask: 0,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("teleport ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 17);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("mounted player pose should be echoed")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), initial_pose.position.x);
    assert_eq!(decoder.read_f64().unwrap(), initial_pose.position.y);
    assert_eq!(decoder.read_f64().unwrap(), initial_pose.position.z);
    assert_eq!(decoder.read_f32().unwrap(), initial_pose.y_rot);
    assert_eq!(decoder.read_f32().unwrap(), initial_pose.x_rot);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());
    assert_eq!(probe.world.local_player_pose(), Some(initial_pose));
    assert_eq!(probe.player_position_state, initial_pose.position_state());
}

#[tokio::test]
async fn probe_player_position_currently_sends_loaded_after_first_sync_only() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
            id: 17,
            position: ProtocolVec3d {
                x: 1.25,
                y: 64.5,
                z: -8.75,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 90.0,
            x_rot: -15.0,
            relatives_mask: 0,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("teleport ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 17);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move player pos/rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 1.25);
    assert_eq!(decoder.read_f64().unwrap(), 64.5);
    assert_eq!(decoder.read_f64().unwrap(), -8.75);
    assert_eq!(decoder.read_f32().unwrap(), 90.0);
    assert_eq!(decoder.read_f32().unwrap(), -15.0);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("current player-loaded approximation should fire after first position sync")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_LOADED);
    assert!(payload.is_empty());
    assert!(probe.player_loaded_sent);

    probe
        .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
            id: 18,
            position: ProtocolVec3d {
                x: 2.0,
                y: 70.0,
                z: -9.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 100.0,
            x_rot: 5.0,
            relatives_mask: 0,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("second teleport ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 18);
    assert!(decoder.is_empty());

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("second move player pos/rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 2.0);
    assert_eq!(decoder.read_f64().unwrap(), 70.0);
    assert_eq!(decoder.read_f64().unwrap(), -9.0);
    assert_eq!(decoder.read_f32().unwrap(), 100.0);
    assert_eq!(decoder.read_f32().unwrap(), 5.0);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());

    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "current player-loaded approximation should only fire once"
    );

    assert_eq!(
        probe.player_position_state.position,
        ProtocolVec3d {
            x: 2.0,
            y: 70.0,
            z: -9.0,
        }
    );
    assert_eq!(probe.player_position_state.y_rot, 100.0);
    assert_eq!(probe.player_position_state.x_rot, 5.0);
}

#[tokio::test]
async fn probe_player_rotation_sends_vanilla_rot_ack() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe.world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 30.0,
        x_rot: 5.0,
        ..LocalPlayerPoseState::default()
    });
    probe
        .handle_play_packet(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
            y_rot: 15.0,
            relative_y: true,
            x_rot: -10.0,
            relative_x: false,
        }))
        .await
        .unwrap();

    let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
        .await
        .expect("move player rot ack should be sent")
        .unwrap();
    assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f32().unwrap(), 45.0);
    assert_eq!(decoder.read_f32().unwrap(), -10.0);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());
    assert_eq!(probe.player_position_state.y_rot, 45.0);
    assert_eq!(probe.player_position_state.x_rot, -10.0);
}

#[tokio::test]
async fn probe_applies_local_player_status_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::PlayerAbilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.08,
            walking_speed: 0.12,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
            progress: 0.5,
            level: 12,
            total: 345,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 5 }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 99 }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetDefaultSpawnPosition(
            SetDefaultSpawnPosition {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos {
                    x: -12,
                    y: 70,
                    z: 44,
                },
                yaw: 180.0,
                pitch: -15.0,
            },
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetSimulationDistance(
            SetSimulationDistance { distance: 12 },
        ))
        .await
        .unwrap();

    let report = probe.finish(6, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let local = report.world.local_player();

    assert_eq!(
        local.abilities,
        Some(bbb_client_world::LocalPlayerAbilitiesState {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.08,
            walking_speed: 0.12,
        })
    );
    assert_eq!(
        local.experience,
        Some(bbb_client_world::LocalPlayerExperienceState {
            progress: 0.5,
            level: 12,
            total: 345,
        })
    );
    assert_eq!(local.selected_hotbar_slot, 5);
    assert_eq!(
        local.default_spawn,
        Some(bbb_client_world::DefaultSpawnState {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos {
                x: -12,
                y: 70,
                z: 44,
            },
            yaw: 180.0,
            pitch: -15.0,
        })
    );
    assert_eq!(local.simulation_distance, Some(12));
    assert_eq!(report.world_counters.player_abilities_packets, 1);
    assert_eq!(report.world_counters.player_experience_packets, 1);
    assert_eq!(report.world_counters.held_slot_packets, 2);
    assert_eq!(report.world_counters.held_slot_updates_applied, 1);
    assert_eq!(report.world_counters.held_slot_updates_ignored, 1);
    assert_eq!(report.world_counters.default_spawn_position_packets, 1);
    assert_eq!(report.world_counters.simulation_distance_packets, 1);
}

#[tokio::test]
async fn probe_applies_player_info_packets_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    let profile_id = Uuid::from_u128(1);
    let removed_profile_id = Uuid::from_u128(2);

    probe
        .handle_play_packet(PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::InitializeChat,
                PlayerInfoAction::UpdateGameMode,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateLatency,
                PlayerInfoAction::UpdateDisplayName,
                PlayerInfoAction::UpdateListOrder,
                PlayerInfoAction::UpdateHat,
            ],
            entries: vec![
                PlayerInfoEntry {
                    profile_id,
                    profile: Some(GameProfile {
                        uuid: profile_id,
                        name: "Ada".into(),
                        properties: vec![GameProfileProperty {
                            name: "textures".into(),
                            value: "skin".to_string(),
                            signature: Some("signature".to_string()),
                        }],
                    }),
                    listed: true,
                    latency: 42,
                    game_mode: GameType::Creative,
                    display_name: Some(bbb_protocol::RichTextComponent::plain("Ada Lovelace")),
                    show_hat: true,
                    list_order: 3,
                    chat_session: Some(PlayerInfoChatSession {
                        session_id: Uuid::from_u128(3),
                        expires_at_epoch_millis: 99,
                        public_key: vec![1, 2],
                        key_signature: vec![3, 4],
                    }),
                },
                PlayerInfoEntry {
                    profile_id: removed_profile_id,
                    profile: Some(GameProfile {
                        uuid: removed_profile_id,
                        name: "Removed".into(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 7,
                    game_mode: GameType::Survival,
                    display_name: None,
                    show_hat: false,
                    list_order: 0,
                    chat_session: None,
                },
            ],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
            profile_ids: vec![removed_profile_id],
        }))
        .await
        .unwrap();

    let report = probe.finish(2, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let entry = report.world.player_info_entry(profile_id).unwrap();
    assert_eq!(entry.profile.uuid, profile_id);
    assert_eq!(entry.profile.name, "Ada");
    assert_eq!(entry.profile.properties.len(), 1);
    assert!(entry.listed);
    assert_eq!(entry.latency, 42);
    assert_eq!(entry.game_mode, "creative");
    assert_eq!(
        entry
            .display_name
            .as_ref()
            .map(bbb_protocol::RichTextComponent::text),
        Some("Ada Lovelace")
    );
    assert!(entry.show_hat);
    assert_eq!(entry.list_order, 3);
    assert!(!entry.chat_session_present);
    assert!(report.world.listed_players().contains(&profile_id));
    assert!(report.world.player_info_entry(removed_profile_id).is_none());
    assert!(!report.world.listed_players().contains(&removed_profile_id));

    assert_eq!(report.world_counters.player_info_update_packets, 1);
    assert_eq!(report.world_counters.player_info_remove_packets, 1);
    assert_eq!(report.world_counters.player_info_entries_tracked, 1);
    assert_eq!(report.world_counters.listed_players_tracked, 1);
}

#[tokio::test]
async fn probe_applies_set_camera_to_world() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 9 }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 123 }))
        .await
        .unwrap();

    let report = probe.finish(3, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert_eq!(
        report.world.local_player().camera,
        bbb_client_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(report.world_counters.set_camera_packets, 2);
    assert_eq!(report.world_counters.set_camera_updates_applied, 1);
    assert_eq!(report.world_counters.set_camera_updates_ignored, 1);
}

#[tokio::test]
async fn probe_respawn_updates_world_and_clears_level_bound_state_when_dimension_changes() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(9)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(55)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 55 }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
            health: 4.0,
            food: 7,
            saturation: 0.5,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
            progress: 0.25,
            level: 3,
            total: 40,
        }))
        .await
        .unwrap();
    probe.world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: 65.0,
            z: -4.0,
        },
        on_ground: true,
        horizontal_collision: true,
        fall_distance: 2.0,
        ..LocalPlayerPoseState::default()
    });
    probe
        .handle_play_packet(PlayClientbound::LevelChunkWithLight(
            synthetic_probe_level_chunk_packet(),
        ))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::BlockDestruction(
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
        .await
        .unwrap();

    let mut spawn_info = protocol_play_login(9).common_spawn_info;
    spawn_info.dimension_type_id = 1;
    spawn_info.dimension = "minecraft:the_nether".to_string();
    spawn_info.sea_level = 32;
    probe
        .handle_play_packet(PlayClientbound::Respawn(Respawn {
            common_spawn_info: spawn_info,
            data_to_keep: 0,
        }))
        .await
        .unwrap();

    let report = probe.finish(9, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    let level = report.world.level_info().unwrap();

    assert_eq!(level.dimension, "minecraft:the_nether");
    assert_eq!(level.dimension_type_id, 1);
    assert_eq!(
        level.dimension_type_name.as_deref(),
        Some("minecraft:the_nether")
    );
    assert_eq!(level.sea_level, 32);
    assert_eq!(report.world.local_player_id(), Some(9));
    // The respawned LocalPlayer is the only entity in the new level.
    assert_eq!(report.world.entity_count(), 1);
    assert!(report.world.probe_entities().is_empty());
    assert_eq!(report.world.chunk_count(), 0);
    assert_eq!(report.world.first_chunk(), None);
    assert_eq!(report.world.local_player().health, None);
    assert_eq!(report.world.local_player().experience, None);
    assert_eq!(
        report.world.local_player().camera,
        bbb_client_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(report.world.local_player_pose(), None);
    assert!(report.world.block_destructions().is_empty());

    assert_eq!(report.world_counters.play_logins_received, 1);
    assert_eq!(report.world_counters.respawns_received, 1);
    assert_eq!(report.world_counters.chunks_received, 1);
    assert_eq!(report.world_counters.block_destructions_received, 1);
    assert_eq!(report.world_counters.block_destructions_tracked, 0);
    assert_eq!(report.world_counters.entities_tracked, 0);
}

#[tokio::test]
async fn probe_respawn_keep_all_data_preserves_local_pose_entity_data_and_attribute_modifiers() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(9)))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
            health: 4.0,
            food: 7,
            saturation: 0.5,
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
            progress: 0.25,
            level: 3,
            total: 40,
        }))
        .await
        .unwrap();
    probe.world.set_local_player_pose(LocalPlayerPoseState {
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
        minor_horizontal_collision: false,
        fall_distance: 8.0,
        sneaking: true,
        swimming: true,
        no_jump_delay_ticks: 0,
        y_rot: 90.0,
        x_rot: 20.0,
        last_teleport_id: 77,
    });
    let old_pose = probe.world.local_player_pose().unwrap();
    probe
        .world
        .set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
    probe.world.set_local_using_item(true);
    probe
        .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
            id: 9,
            values: vec![entity_shift_key_down_data()],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::UpdateAttributes(UpdateAttributes {
            entity_id: 9,
            attributes: vec![AttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.1,
                modifiers: vec![AttributeModifier {
                    id: "minecraft:test_speed".to_string(),
                    amount: 0.25,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                }],
            }],
        }))
        .await
        .unwrap();
    probe
        .handle_play_packet(PlayClientbound::UpdateMobEffect(UpdateMobEffect {
            entity_id: 9,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            amplifier: 1,
            duration_ticks: 200,
            flags: MobEffectFlags::default(),
        }))
        .await
        .unwrap();
    assert!(probe
        .world
        .entity_effect(9, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
        .is_some());
    let old_data = probe.world.probe_entity(9).unwrap().data_values;

    probe
        .handle_play_packet(PlayClientbound::Respawn(Respawn {
            common_spawn_info: protocol_play_login(9).common_spawn_info,
            data_to_keep: 3,
        }))
        .await
        .unwrap();

    let report = probe.finish(8, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);

    assert!(report.world.local_player().health.is_none());
    assert!(report.world.local_player().experience.is_none());
    assert_eq!(
        report.world.local_player_pose(),
        Some(LocalPlayerPoseState {
            position: Default::default(),
            delta_movement: old_pose.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            minor_horizontal_collision: false,
            fall_distance: 0.0,
            sneaking: old_pose.sneaking,
            swimming: old_pose.swimming,
            no_jump_delay_ticks: 0,
            y_rot: old_pose.y_rot,
            x_rot: old_pose.x_rot,
            last_teleport_id: 0,
        })
    );
    assert_eq!(
        report.world.local_player().camera,
        bbb_client_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(
        report.world.local_player().interaction,
        bbb_client_world::LocalPlayerInteractionState::default()
    );

    // KEEP_ENTITY_DATA copies the old LocalPlayer's synced data, including
    // the pose and flags it set itself.
    let entity = report.world.probe_entity(9).unwrap();
    assert_eq!(entity.data_values, old_data);
    assert!(!old_data.is_empty());
    assert!(entity.mob_effects.is_empty());
    assert_eq!(entity.attributes.len(), 1);
    assert_eq!(entity.attributes[0].base, 0.1);
    assert_eq!(
        entity.attributes[0].modifiers,
        vec![AttributeModifier {
            id: "minecraft:test_speed".to_string(),
            amount: 0.25,
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        }]
    );

    assert_eq!(report.world_counters.play_logins_received, 1);
    assert_eq!(report.world_counters.respawns_received, 1);
    assert_eq!(report.world_counters.entity_data_updates_received, 1);
    assert_eq!(report.world_counters.entity_data_updates_applied, 1);
    assert_eq!(report.world_counters.entity_attribute_updates_received, 1);
    assert_eq!(report.world_counters.entity_attribute_updates_applied, 1);
    assert_eq!(report.world_counters.update_mob_effect_packets, 1);
    assert_eq!(report.world_counters.update_mob_effects_ignored, 0);
    assert_eq!(report.world_counters.active_mob_effects_tracked, 0);
}

#[tokio::test]
async fn probe_set_health_records_dead_health_without_auto_respawn() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);

    probe
        .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
            health: 0.0,
            food: 3,
            saturation: 0.5,
        }))
        .await
        .unwrap();

    assert!(
        timeout(Duration::from_millis(100), server.read_packet())
            .await
            .is_err(),
        "dead health alone must not send perform respawn"
    );

    assert_eq!(
        probe.world.local_player().health,
        Some(bbb_client_world::LocalPlayerHealthState {
            health: 0.0,
            food: 3,
            saturation: 0.5,
        })
    );
    assert!(probe.world.local_player_is_dead());
    assert_eq!(probe.world.counters().player_health_packets, 1);
}

#[tokio::test]
async fn probe_play_unknown_packets_update_report_diagnostics() {
    let (client, _server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.state = ConnectionState::Play;

    probe
        .handle_play_packet(PlayClientbound::Unknown {
            packet_id: 0x7e,
            len: 9,
        })
        .await
        .unwrap();

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    assert_eq!(report.unsupported_packets, 1);
    assert_eq!(
        report.last_unsupported_packet_state.as_deref(),
        Some("Play")
    );
    assert_eq!(report.last_unsupported_packet_id, Some(0x7e));
    assert_eq!(report.last_unsupported_packet_len, Some(9));
    assert_eq!(report.world_counters.play_logins_received, 0);
}

#[tokio::test]
async fn probe_bundle_delimiter_is_transport_noop() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.state = ConnectionState::Play;

    let first_chunk = probe
        .handle_play_packet(PlayClientbound::BundleDelimiter)
        .await
        .unwrap();

    assert_eq!(first_chunk, None);
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "bundle delimiter must not send serverbound packets"
    );

    let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
    assert_eq!(report.unsupported_packets, 0);
    assert_eq!(report.world_counters.play_logins_received, 0);
}

#[tokio::test]
async fn probe_play_disconnect_returns_disconnect_error() {
    let (client, mut server) = raw_connection_pair().await;
    let mut probe = ProbeContext::new(client);
    probe.state = ConnectionState::Play;

    let err = probe
        .handle_play_packet(PlayClientbound::Disconnect(Disconnect {
            reason: "Kicked".to_string(),
            styled_reason: Vec::new(),
            raw_reason: Vec::new(),
        }))
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "play disconnected: Kicked");
    assert!(
        timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err(),
        "disconnect handling must not send serverbound packets"
    );
}
