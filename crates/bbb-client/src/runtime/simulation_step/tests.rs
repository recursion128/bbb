use super::*;
use crate::simulation::{
    commands::SimulationCommand,
    effects::PlatformEffect,
    input_commands::{SimulationInputContext, SimulationInputRuntime},
};
use bbb_protocol::packets::{
    ClientInformation, CommonPlayerSpawnInfo, ContainerClose, GameEvent, PlayClientbound,
    PlayLogin, Respawn,
};
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

fn key(code: KeyCode) -> SimulationCommand {
    SimulationCommand::Key {
        physical_key: PhysicalKey::Code(code),
        state: ElementState::Pressed,
        clipboard_text: None,
    }
}

fn key_up(code: KeyCode) -> SimulationCommand {
    SimulationCommand::Key {
        physical_key: PhysicalKey::Code(code),
        state: ElementState::Released,
        clipboard_text: None,
    }
}

fn notice() -> bbb_protocol::packets::ShowDialog {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    bbb_protocol::packets::ShowDialog {
        dialog: bbb_protocol::packets::DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:notice".into()),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::String("Notice".into()),
                },
            ]),
        },
    }
}

fn packet_step(
    runtime: &mut SimulationInputRuntime,
    cx: &mut SimulationInputContext<'_, '_>,
    packet: PlayClientbound,
    commands: Vec<SimulationCommand>,
) {
    let screen_was_open =
        cx.input.current_screen().is_some() || cx.world.level_load().screen_is_up();
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(packet)).unwrap();
    assert_eq!(
        events::drain_net_events_with_input(
            cx.input,
            &mut rx,
            cx.world,
            cx.net_counters,
            cx.net_commands
        ),
        1
    );
    cx.input
        .apply_screen_requests(cx.world, cx.net_counters, cx.net_commands);
    runtime.reconcile_screen_cursor(
        screen_was_open,
        cx.input,
        cx.world,
        cx.net_counters,
        cx.platform,
    );
    runtime.apply_commands(commands, cx, false);
}

#[tokio::test]
async fn server_close_recaptures_before_same_step_chat_input() {
    for close in [
        PlayClientbound::ContainerClose(ContainerClose { container_id: 0 }),
        PlayClientbound::ClearDialog,
    ] {
        with_context(|runtime, cx| {
            runtime.set_cursor_captured(true);
            if matches!(close, PlayClientbound::ContainerClose(_)) {
                runtime.apply_commands(vec![key(KeyCode::KeyT), key_up(KeyCode::KeyT)], cx, false);
                assert!(cx.input.chat_entry_is_active());
            } else {
                packet_step(runtime, cx, PlayClientbound::ShowDialog(notice()), vec![]);
                assert!(cx.input.dialog_screen_is_open());
            }
            assert!(!runtime.cursor_captured());
            cx.platform.take();
            packet_step(
                runtime,
                cx,
                close,
                vec![key(KeyCode::KeyT), key_up(KeyCode::KeyT)],
            );
            assert!(cx.input.chat_entry_is_active());
            assert!(cx
                .platform
                .take()
                .iter()
                .any(|e| matches!(e, PlatformEffect::SetCursorCapture(true))));
        });
    }
}

#[tokio::test]
async fn server_close_preserves_parent_and_inactive_window_capture() {
    with_context(|runtime, cx| {
        runtime.reconcile_screen_cursor(false, cx.input, cx.world, cx.net_counters, cx.platform);
        assert!(!runtime.cursor_captured());
        runtime.set_cursor_captured(true);
        runtime.apply_commands(vec![key(KeyCode::KeyT)], cx, false);
        packet_step(runtime, cx, PlayClientbound::ShowDialog(notice()), vec![]);
        cx.platform.take();
        packet_step(runtime, cx, PlayClientbound::ClearDialog, vec![]);
        assert!(cx.input.chat_entry_is_active());
        assert!(!runtime.cursor_captured());
        assert!(!cx
            .platform
            .take()
            .iter()
            .any(|e| matches!(e, PlatformEffect::SetCursorCapture(true))));
        runtime.apply_commands(vec![SimulationCommand::FocusChanged(false)], cx, false);
        packet_step(
            runtime,
            cx,
            PlayClientbound::ContainerClose(ContainerClose { container_id: 0 }),
            vec![],
        );
        assert!(cx.input.current_screen().is_none());
        assert!(!runtime.cursor_captured());
        assert!(!cx
            .platform
            .take()
            .iter()
            .any(|e| matches!(e, PlatformEffect::SetCursorCapture(true))));
    });
}

#[tokio::test]
async fn server_close_waits_for_level_ready_before_recapturing() {
    with_context(|runtime, cx| {
        runtime.set_cursor_captured(true);
        cx.input
            .open_debug_pause_screen_with_menu(cx.world, cx.net_counters, cx.net_commands);
        runtime.apply_commands(vec![], cx, false);
        packet_step(
            runtime,
            cx,
            PlayClientbound::Respawn(Respawn {
                common_spawn_info: spawn("minecraft:the_nether"),
                data_to_keep: 0,
            }),
            vec![],
        );
        assert!(cx.world.level_load().screen_is_up());
        assert!(cx.input.current_screen().is_none());
        assert!(!runtime.cursor_captured());
        packet_step(runtime, cx, PlayClientbound::GameEvent(GameEvent {
            event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        }), vec![]);
        let mut loading = LevelLoadRuntime::default();
        loading.tick(cx.now, cx.world, cx.net_commands, false);
        assert!(cx.world.level_load().screen_is_up());
        assert!(!runtime.cursor_captured());
        let loading_screen_was_up = cx.world.level_load().screen_is_up();
        loading.tick(cx.now, cx.world, cx.net_commands, true);
        runtime.reconcile_screen_cursor(
            loading_screen_was_up,
            cx.input,
            cx.world,
            cx.net_counters,
            cx.platform,
        );
        assert!(!cx.world.level_load().screen_is_up());
        assert!(runtime.cursor_captured());
        runtime.apply_commands(vec![key(KeyCode::KeyT)], cx, false);
        assert!(cx.input.chat_entry_is_active());
    });
}

fn spawn(dimension: &str) -> CommonPlayerSpawnInfo {
    CommonPlayerSpawnInfo {
        dimension_type_id: 0,
        dimension: dimension.into(),
        seed: 0,
        game_type: 0,
        previous_game_type: -1,
        is_debug: false,
        is_flat: false,
        last_death_location: None,
        portal_cooldown: 0,
        sea_level: 63,
    }
}

fn with_context(
    run: impl FnOnce(&mut SimulationInputRuntime, &mut SimulationInputContext<'_, '_>),
) {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".into(), "minecraft:the_nether".into()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: spawn("minecraft:overworld"),
        enforces_secure_chat: false,
    });
    world.apply_game_event(GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    });
    assert!(world.complete_client_level_load(false));
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        ..Default::default()
    });
    let mut counters = NetCounters {
        connected: true,
        state: Some("Play".into()),
        ..Default::default()
    };
    input.apply_screen_requests(&mut world, &mut counters, &None);
    let (sender, _receiver) = mpsc::channel(128);
    let commands = Some(sender);
    let mut hands = FirstPersonHandPresentation::default();
    static NEXT_CACHE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    struct TempCacheDir(std::path::PathBuf);
    impl Drop for TempCacheDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let cache_dir = TempCacheDir(std::env::temp_dir().join(format!(
        "bbb-screen-cursor-pack-cache-{}-{}",
        std::process::id(),
        NEXT_CACHE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )));
    let mut packs = ServerResourcePackRuntime::new(
        tokio::runtime::Handle::current(),
        cache_dir.0.clone(),
        "screen-cursor-tests",
        uuid::Uuid::nil(),
        None,
    )
    .unwrap();
    let mut items = None;
    let mut win = Arc::new(crate::win_screen_resources::WinScreenResources {
        lines: vec![],
        narration_components: vec![],
        total_scroll_length: 0,
    });
    let mut random = LevelEventSoundRandomState::default();
    let mut audio: Option<&mut dyn AudioEventSink> = None;
    let mut platform = PlatformEffects::default();
    let mut cx = SimulationInputContext {
        input: &mut input,
        now: Instant::now(),
        first_person_hand_presentation: &mut hands,
        world: &mut world,
        net_counters: &mut counters,
        net_commands: &commands,
        server_resource_packs: &mut packs,
        item_runtime: &mut items,
        code_of_conduct: None,
        win_screen_resources: &mut win,
        entity_partial_tick: 0.0,
        level_random: &mut random,
        audio_events: &mut audio,
        platform: &mut platform,
    };
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(320, 240),
        false,
        ClientInformation::default(),
    );
    run(&mut runtime, &mut cx);
}

#[tokio::test]
async fn written_book_and_chat_escape_restore_capture_for_next_chat_key() {
    with_context(|runtime, cx| {
        use bbb_protocol::packets::{
            InteractionHand, ItemStackSummary, OpenBook, SetPlayerInventory,
            WrittenBookContentSummary,
        };
        let mut book = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        };
        book.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
            generation: 0,
            pages: vec!["Page".into()],
            page_filters: vec![None],
            resolved: true,
        });
        cx.world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: book,
        });
        runtime.set_cursor_captured(true);
        packet_step(
            runtime,
            cx,
            PlayClientbound::OpenBook(OpenBook {
                hand: InteractionHand::MainHand,
            }),
            vec![],
        );
        assert!(cx.input.book_screen().is_some());
        assert!(!runtime.cursor_captured());
        runtime.apply_commands(
            vec![
                key(KeyCode::Escape),
                key_up(KeyCode::Escape),
                key(KeyCode::KeyT),
                key_up(KeyCode::KeyT),
            ],
            cx,
            false,
        );
        assert!(cx.input.book_screen().is_none());
        assert!(cx.input.chat_entry_is_active());
        assert!(!runtime.cursor_captured());
        runtime.apply_commands(
            vec![key(KeyCode::Escape), key_up(KeyCode::Escape)],
            cx,
            false,
        );
        assert!(cx.input.current_screen().is_none());
        assert!(runtime.cursor_captured());
        runtime.apply_commands(vec![key(KeyCode::KeyT), key_up(KeyCode::KeyT)], cx, false);
        assert!(cx.input.chat_entry_is_active());
    });
}

#[tokio::test]
async fn level_loading_screen_blocks_gameplay_keys_and_local_disconnect_resets_on_reconnect() {
    with_context(|runtime, cx| {
        runtime.set_cursor_captured(true);
        packet_step(
            runtime,
            cx,
            PlayClientbound::Respawn(Respawn {
                common_spawn_info: spawn("minecraft:the_nether"),
                data_to_keep: 0,
            }),
            vec![],
        );
        assert!(cx.world.level_load().screen_is_up());
        runtime.apply_commands(vec![key(KeyCode::KeyT), key_up(KeyCode::KeyT)], cx, false);
        assert!(
            !cx.input.chat_entry_is_active(),
            "gameplay keys must not open screens behind level loading screen"
        );
        assert!(!runtime.cursor_captured());

        runtime.request_local_disconnect(cx.net_commands);
        assert!(runtime.net_disconnect_requested());
        assert!(runtime.take_pending_local_disconnect_teardown());
        runtime.reset_net_disconnect_requested();
        assert!(!runtime.net_disconnect_requested());
    });
}
