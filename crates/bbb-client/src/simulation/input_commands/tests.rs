use super::*;

mod chat_input;
mod wants_cursor;

fn queue() -> (
    crate::simulation::commands::CommandQueue,
    crate::simulation::commands::CommandInbox,
) {
    crate::simulation::commands::command_queue(std::sync::Arc::new(
        crate::simulation::commands::LoopWake::default(),
    ))
}

struct TestServerPackCacheDir(std::path::PathBuf);

impl TestServerPackCacheDir {
    fn new(label: &str) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seq = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!(
            "bbb-input-cmd-pack-cache-{label}-{}-{seq}",
            std::process::id()
        )))
    }

    fn path(&self) -> std::path::PathBuf {
        self.0.clone()
    }
}

impl Drop for TestServerPackCacheDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn profiling_toggle_requests_preserve_start_stop_order() {
    assert_eq!(
        debug_profiling_commands(vec![
            DebugProfilingToggleRequest::Start,
            DebugProfilingToggleRequest::Stop,
            DebugProfilingToggleRequest::Start,
        ]),
        vec![
            DebugProfilingCommand::Start,
            DebugProfilingCommand::Stop,
            DebugProfilingCommand::Start,
        ]
    );
}

#[tokio::test]
async fn recipe_book_language_transition_reloads_saves_and_broadcasts_in_order() {
    use tokio::runtime::Handle;
    use uuid::Uuid;

    let cache_dir = std::env::temp_dir().join(format!(
        "bbb-recipe-language-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut packs = ServerResourcePackRuntime::new(
        Handle::current(),
        cache_dir.clone(),
        "recipe-language-test",
        Uuid::nil(),
        None,
    )
    .unwrap();
    let (net_tx, mut net_rx) = mpsc::channel(4);
    let net_commands = Some(net_tx);
    let mut counters = crate::control::NetCounters::default();
    let mut information = ClientInformation::default();
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    apply_recipe_book_language_transition(1, false, &mut information, &mut packs, &mut platform);
    assert_eq!(information.language, "en_us");
    assert!(packs.take_reload().is_none());
    assert!(net_rx.try_recv().is_err());

    apply_recipe_book_language_transition(1, true, &mut information, &mut packs, &mut platform);
    // Repeated checks after the selected language changed are no-ops.
    apply_recipe_book_language_transition(1, true, &mut information, &mut packs, &mut platform);
    assert_eq!(information.language, "en_pt");
    assert!(packs.take_reload().is_some());
    assert!(packs.take_reload().is_none());
    // The packet cannot overtake the platform owner's save attempt.
    assert_eq!(counters.broadcast_client_information_commands_queued, 0);
    assert!(net_rx.try_recv().is_err());

    let effects = platform.take();
    let [PlatformEffect::SetClientLocale(locale), PlatformEffect::SaveClientOptions { writes, broadcast }] =
        effects.as_slice()
    else {
        panic!("expected locale and options save: {effects:?}");
    };
    assert_eq!(locale, "en_pt");
    assert_eq!(
        writes,
        &[crate::options::ClientOptionWrite {
            key: "lang",
            value: crate::options::ClientOptionValue::Text("en_pt"),
        }]
    );
    assert_eq!(broadcast, &information);
    apply_client_options_save_completed(broadcast.clone(), &mut counters, &net_commands);
    assert_eq!(counters.broadcast_client_information_commands_queued, 1);
    assert!(matches!(
        net_rx.try_recv(),
        Ok(NetCommand::BroadcastClientInformation(ClientInformation {
            language,
            ..
        })) if language == "en_pt"
    ));
    assert!(net_rx.try_recv().is_err());

    let _ = std::fs::remove_dir_all(cache_dir);
}

#[test]
fn text_input_is_reconciled_once_per_applied_step() {
    // `TextInputManager.tick` runs every tick, not only when a screen
    // opens: a text field can lose focus without any command of its own,
    // and an IME left allowed grabs every letter key after that.
    let source = include_str!("../input_commands.rs");
    let body = source
        .split_once("pub(crate) fn apply_commands(")
        .expect("command batch owner")
        .1;
    let sync = body
        .find("self.sync_text_input_enabled(context)")
        .expect("per-step text-input reconciliation");
    let preedit = body
        .find("self.sync_ime_preedit_focus(context)")
        .expect("per-step preedit reconciliation");
    // The preedit is routed to its target first, so the window is told to
    // stop accepting text only after the composition it owns is handed on.
    assert!(preedit < sync);
    // One owner: nothing else may hand the window an IME decision.  Count
    // in production only -- this test names the effect too.
    let production = format!(
        "{}\n{}\n{}",
        source.split_once("\nmod tests").expect("test boundary").0,
        include_str!("keyboard_and_ime.rs"),
        include_str!("pointer.rs"),
    );
    assert_eq!(production.matches("SetTextInputEnabled(").count(), 1);
}

#[test]
fn terrain_readiness_is_scoped_to_the_published_level_generation() {
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    runtime.terrain_readiness = Some((7, 1, true));

    assert!(runtime.player_section_ready(7, 1));
    assert!(!runtime.player_section_ready(7, 2));
    assert!(!runtime.player_section_ready(8, 1));

    runtime.terrain_readiness = Some((8, 1, false));
    assert!(!runtime.player_section_ready(8, 1));
}

#[test]
fn surface_resize_recomputes_gui_scale_and_projection() {
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1920, 1080),
        false,
        ClientInformation::default(),
    );
    runtime.set_gui_scale_options(0, false);
    assert_eq!(runtime.gui_scale, 4);
    assert_eq!(runtime.gui_scaled_width, 480);
    assert_eq!(runtime.gui_scaled_height, 270);
    assert!(!runtime.minimized);

    assert!(runtime.set_surface_size(PhysicalSize::new(1280, 720)));
    assert_eq!(runtime.gui_scale, 3);
    assert_eq!(runtime.gui_scaled_width, 427);
    assert_eq!(runtime.gui_scaled_height, 240);
    assert_eq!(runtime.surface_size, PhysicalSize::new(1280, 720));
    assert_eq!(runtime.gui_viewport(), (PhysicalSize::new(427, 240), 3));
    assert!(!runtime.set_surface_size(PhysicalSize::new(1280, 720)));
    // The layout rounds up; the projection retains the selected scale.
    for (unicode, expected) in [(false, (3, 427, 241)), (true, (4, 321, 181))] {
        assert_eq!(
            project_gui_scale(PhysicalSize::new(1281, 721), 3, unicode),
            expected
        );
    }
}

#[test]
fn zero_size_resize_marks_minimized_and_keeps_last_usable_surface() {
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    runtime.set_gui_scale_options(2, true);
    assert_eq!(runtime.gui_scale, 2);

    runtime.set_surface_size(PhysicalSize::new(0, 0));
    assert!(runtime.minimized);
    assert_eq!(runtime.surface_size, PhysicalSize::new(1280, 720));
    assert_eq!(runtime.gui_scale, 2);

    runtime.set_surface_size(PhysicalSize::new(1920, 1080));
    assert!(!runtime.minimized);
    assert_eq!(runtime.surface_size, PhysicalSize::new(1920, 1080));
    assert_eq!(runtime.gui_scale, 2);
}

#[test]
fn manual_gui_scale_cap_and_unicode_rounding_flow_into_projection() {
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1920, 1080),
        false,
        ClientInformation::default(),
    );
    runtime.set_gui_scale_options(3, true);
    assert_eq!(runtime.gui_scale, 4);
    assert_eq!(runtime.gui_scaled_width, 480);
    assert_eq!(runtime.gui_scaled_height, 270);
}

#[tokio::test]
async fn focus_regained_in_the_same_step_prevents_the_pause_screen() {
    use crate::control::NetCounters;
    use tokio::runtime::Handle;
    use uuid::Uuid;

    // The production `apply_commands` path, not a direct helper call: a
    // sequenced `FocusChanged(true)` must run before the post-step
    // `pauseIfInactive` evaluation observes the focus state.
    let commands = vec![SimulationCommand::FocusChanged(true)];

    let now = Instant::now();
    let mut input = crate::input::ClientInputState::new(false);
    // The window has been unfocused since `now`, so by `now + 600 ms` it
    // is past the strict >500 ms delay (`Minecraft.pauseIfInactive`).
    input.set_last_active_at_for_test(now);
    let mut world = WorldStore::new();
    let mut net_counters = NetCounters::default();
    let mut first_person_hand_presentation =
        crate::item_models::FirstPersonHandPresentation::default();
    let cache_dir = TestServerPackCacheDir::new("focus");
    let mut server_resource_packs = ServerResourcePackRuntime::new(
        Handle::current(),
        cache_dir.path(),
        "window-surface-focus-test",
        Uuid::nil(),
        None,
    )
    .unwrap();
    let mut item_runtime = None;
    let mut win_screen_resources =
        std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
            lines: Vec::new(),
            narration_components: Vec::new(),
            total_scroll_length: 0,
        });
    let mut level_random = bbb_client_world::LevelEventSoundRandomState::default();
    let mut audio_events: Option<&mut dyn AudioEventSink> = None;
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    let mut context = SimulationInputContext {
        input: &mut input,
        now: now + Duration::from_millis(600),
        first_person_hand_presentation: &mut first_person_hand_presentation,
        world: &mut world,
        net_counters: &mut net_counters,
        net_commands: &None,
        server_resource_packs: &mut server_resource_packs,
        item_runtime: &mut item_runtime,
        code_of_conduct: None,
        win_screen_resources: &mut win_screen_resources,
        entity_partial_tick: 0.0,
        level_random: &mut level_random,
        audio_events: &mut audio_events,
        platform: &mut platform,
    };
    runtime.apply_commands(commands, &mut context, false);

    // The focus-regained command applied first, so the post-step pause
    // evaluation observes the updated focus and must not open a pause
    // screen even though the window was unfocused for >500 ms.
    assert!(input.is_focused());
    assert!(!input.debug_pause_screen_is_open());
}

#[tokio::test]
async fn mouse_button_receipts_distinguish_delivery_inventory_routing_and_consumption() {
    use crate::control::NetCounters;
    use tokio::runtime::Handle;
    use uuid::Uuid;

    let (commands, inbox) = queue();
    let now = Instant::now();
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());
    let mut net_counters = NetCounters::default();
    let (net_tx, mut net_rx) = mpsc::channel(1);
    let net_commands = Some(net_tx);
    let mut first_person_hand_presentation =
        crate::item_models::FirstPersonHandPresentation::default();
    let cache_dir = TestServerPackCacheDir::new("mouse-button");
    let mut server_resource_packs = ServerResourcePackRuntime::new(
        Handle::current(),
        cache_dir.path(),
        "mouse-button-receipt-test",
        Uuid::nil(),
        None,
    )
    .unwrap();
    let mut item_runtime = None;
    let mut win_screen_resources =
        std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
            lines: Vec::new(),
            narration_components: Vec::new(),
            total_scroll_length: 0,
        });
    let mut level_random = bbb_client_world::LevelEventSoundRandomState::default();
    let mut audio_events: Option<&mut dyn AudioEventSink> = None;
    let mut platform = crate::simulation::effects::PlatformEffects::default();
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    runtime.set_gui_scale_options(0, false);
    assert!(!world.recipe_book().settings.crafting.open);
    input.apply_screen_requests(&mut world, &mut net_counters, &net_commands);
    let mut context = SimulationInputContext {
        input: &mut input,
        now,
        first_person_hand_presentation: &mut first_person_hand_presentation,
        world: &mut world,
        net_counters: &mut net_counters,
        net_commands: &net_commands,
        server_resource_packs: &mut server_resource_packs,
        item_runtime: &mut item_runtime,
        code_of_conduct: None,
        win_screen_resources: &mut win_screen_resources,
        entity_partial_tick: 0.0,
        level_random: &mut level_random,
        audio_events: &mut audio_events,
        platform: &mut platform,
    };
    assert!(context.input.wants_cursor());

    runtime.apply_mouse_button(
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(239.18671875, 107.33333333333333)),
        pointer::HudMouseHitTargets::default(),
        &mut context,
    );

    assert_eq!(runtime.mouse_button_events, 1);
    assert_eq!(runtime.left_mouse_button_events, 1);
    assert_eq!(runtime.left_mouse_button_press_events, 1);
    assert_eq!(runtime.inventory_mouse_button_events, 1);
    assert_eq!(runtime.inventory_mouse_button_consumed_events, 1);
    assert_eq!(runtime.inventory_recipe_book_button_hit_events, 1);
    assert_eq!(runtime.keyboard_events, 0);
    assert!(context.world.recipe_book().settings.crafting.open);
    assert_eq!(
        context
            .net_counters
            .recipe_book_change_settings_commands_queued,
        1
    );
    assert!(matches!(
        net_rx.try_recv(),
        Ok(NetCommand::RecipeBookChangeSettings(_))
    ));
    commands.push(SimulationCommand::Key {
        physical_key: PhysicalKey::Code(KeyCode::KeyT),
        state: ElementState::Pressed,
        clipboard_text: None,
    });
    commands.push(SimulationCommand::Key {
        physical_key: PhysicalKey::Code(KeyCode::KeyT),
        state: ElementState::Released,
        clipboard_text: None,
    });
    commands.push(SimulationCommand::ImeCommit("t".to_string()));
    runtime.apply_commands(inbox.drain().0, &mut context, false);
    assert!(context.input.recipe_book_search_hud_state().focused);
    assert_eq!(context.input.recipe_book_search_hud_state().edit.text(), "");
    commands.push(SimulationCommand::ImeCommit("excitedz".to_string()));
    runtime.apply_commands(inbox.drain().0, &mut context, false);
    assert!(context.input.recipe_book_search_hud_state().focused);
    runtime.apply_input_control(
        InputControlRequest::Text {
            text: "e".to_string(),
        },
        &mut context,
        &mut None,
    );
    let search = context.input.recipe_book_search_hud_state();
    assert!(search.focused);
    assert_eq!(search.edit.text(), "excitedze");
    assert_eq!(context.input.take_recipe_book_language_requests(), 0);
    assert_eq!(runtime.recipe_book_language_requests, 1);
    assert_eq!(runtime.keyboard_events, 2);
    assert_eq!(runtime.keyboard_press_events, 1);
    assert_eq!(runtime.key_t_events, 2);
    assert_eq!(runtime.inventory_keyboard_events, 2);
    assert_eq!(runtime.inventory_key_t_press_events, 1);
    assert!(!runtime.recipe_book_language_target_available);
    assert_eq!(runtime.recipe_book_language_transitions, 0);
    let publication = runtime.publication();
    assert_eq!(publication.mouse_button_events, 1);
    assert_eq!(publication.left_mouse_button_events, 1);
    assert_eq!(publication.left_mouse_button_press_events, 1);
    assert_eq!(publication.inventory_mouse_button_events, 1);
    assert_eq!(publication.inventory_mouse_button_consumed_events, 1);
    assert_eq!(publication.inventory_recipe_book_button_hit_events, 1);
    assert_eq!(publication.keyboard_events, 2);
    assert_eq!(publication.keyboard_press_events, 1);
    assert_eq!(publication.key_t_events, 2);
    assert_eq!(publication.inventory_keyboard_events, 2);
    assert_eq!(publication.inventory_key_t_press_events, 1);
    assert_eq!(publication.recipe_book_language_requests, 1);
    assert!(!publication.recipe_book_language_target_available);
    assert_eq!(publication.recipe_book_language_transitions, 0);
}

#[tokio::test]
async fn an_open_composition_hands_off_to_a_newly_focused_sign_editor() {
    use crate::control::NetCounters;
    use tokio::runtime::Handle;
    use uuid::Uuid;

    // Vanilla `KeyboardHandler` stores `lastPreeditEvent` even when no
    // element renders it; `Minecraft.onTextInputFocusChange` then resubmits
    // it to the element gaining focus.
    let commands = vec![SimulationCommand::ImePreedit {
        text: "compose".to_string(),
        cursor_range: Some((3, 3)),
    }];

    let now = Instant::now();
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    // `sign_editor_is_valid_at` needs a local player within the vanilla
    // block-interaction range of the sign.
    world.apply_login(&bbb_protocol::packets::PlayLogin {
        player_id: 7,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
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
    });
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: -4.5,
            y: 69.0,
            z: 12.5,
        },
        on_ground: true,
        ..bbb_client_world::LocalPlayerPoseState::default()
    });
    let mut net_counters = NetCounters::default();
    let mut first_person_hand_presentation =
        crate::item_models::FirstPersonHandPresentation::default();
    let cache_dir = TestServerPackCacheDir::new("ime-preedit");
    let mut server_resource_packs = ServerResourcePackRuntime::new(
        Handle::current(),
        cache_dir.path(),
        "ime-preedit-handoff-test",
        Uuid::nil(),
        None,
    )
    .unwrap();
    let mut item_runtime = None;
    let mut win_screen_resources =
        std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
            lines: Vec::new(),
            narration_components: Vec::new(),
            total_scroll_length: 0,
        });
    let mut level_random = bbb_client_world::LevelEventSoundRandomState::default();
    let mut audio_events: Option<&mut dyn AudioEventSink> = None;
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    {
        let mut context = SimulationInputContext {
            input: &mut input,
            now,
            first_person_hand_presentation: &mut first_person_hand_presentation,
            world: &mut world,
            net_counters: &mut net_counters,
            net_commands: &None,
            server_resource_packs: &mut server_resource_packs,
            item_runtime: &mut item_runtime,
            code_of_conduct: None,
            win_screen_resources: &mut win_screen_resources,
            entity_partial_tick: 0.0,
            level_random: &mut level_random,
            audio_events: &mut audio_events,
            platform: &mut platform,
        };
        runtime.apply_commands(commands, &mut context, false);
    }
    assert!(input.sign_editor_hud_state().is_none());

    // The sign editor opens with no further IME traffic; the next step's
    // focus sync must hand the stored composition to the gaining editor.
    world.insert_decoded_chunk(bbb_client_world::ChunkColumn {
        pos: bbb_client_world::ChunkPos { x: -1, z: 0 },
        state: bbb_client_world::ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::new(vec![bbb_client_world::BlockEntityRecord::empty(
            11, 70, 12, 7,
        )
        .with_raw_nbt(vec![0])
        .with_payload(bbb_client_world::BlockEntityPayload::SignText(Box::new(
            bbb_client_world::SignBlockEntityTextState {
                front: bbb_client_world::SignTextSideState::default(),
                back: bbb_client_world::SignTextSideState::default(),
                is_waxed: false,
            },
        )))]),
        light: bbb_client_world::LightData::default(),
    });
    world.apply_open_sign_editor(bbb_protocol::packets::OpenSignEditor {
        pos: bbb_protocol::packets::BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut net_counters, &None);
    {
        let mut context = SimulationInputContext {
            input: &mut input,
            now,
            first_person_hand_presentation: &mut first_person_hand_presentation,
            world: &mut world,
            net_counters: &mut net_counters,
            net_commands: &None,
            server_resource_packs: &mut server_resource_packs,
            item_runtime: &mut item_runtime,
            code_of_conduct: None,
            win_screen_resources: &mut win_screen_resources,
            entity_partial_tick: 0.0,
            level_random: &mut level_random,
            audio_events: &mut audio_events,
            platform: &mut platform,
        };
        runtime.apply_commands(Vec::new(), &mut context, false);
    }
    let editor = input.sign_editor_hud_state().expect("sign editor opened");
    assert!(
        editor.preedit.is_some(),
        "the stored composition must reach the newly focused sign editor"
    );
}

#[test]
fn hud_air_bubble_pop_command_rejects_stale_level_generations() {
    let mut world = WorldStore::new();
    let position = bbb_protocol::packets::Vec3d {
        x: 1.25,
        y: 64.0,
        z: -3.5,
    };
    let mut audio_events = None;
    let level_generation = world.client_level_generation();

    apply_hud_air_bubble_pop_sound(
        &mut world,
        &mut audio_events,
        level_generation.saturating_add(1),
        position,
        1.0,
        1.3,
    );
    assert!(world.last_sound().is_none());

    apply_hud_air_bubble_pop_sound(
        &mut world,
        &mut audio_events,
        level_generation,
        position,
        1.0,
        1.3,
    );
    let sound = world.last_sound().expect("current-generation HUD sound");
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:ui.hud.bubble_pop")
    );
    assert_eq!(sound.position, position);
}

#[test]
fn network_terminal_shutdown_is_idempotent_and_requests_disconnect() {
    let mut runtime = SimulationInputRuntime::new(
        PhysicalSize::new(1280, 720),
        false,
        ClientInformation::default(),
    );
    let (tx, mut rx) = mpsc::channel(2);
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    runtime.request_network_terminal_shutdown(&Some(tx), &mut platform);
    runtime.request_network_terminal_shutdown(&None, &mut platform);

    assert!(!runtime.running());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::Disconnect)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_hit_target_revision_rejects_new_delivery() {
    let mut world = WorldStore::new();
    let mut input = crate::input::ClientInputState::new(true);
    let mut counters = crate::control::NetCounters::default();
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    let target = HudChatHitTarget {
        level_generation: world.client_level_generation(),
        delivery_revision: world.client_chat().delivery_revision,
        chat_screen_revision: input.chat_screen_revision(),
        resource_generation: 0,
        resources_loaded: false,
        kind: HudChatHitTargetKind::Queue,
    };
    assert!(chat_hit_target_is_current(&input, &world, None, target));
    let stale_resource_target = HudChatHitTarget {
        resources_loaded: true,
        ..target
    };
    assert!(!chat_hit_target_is_current(
        &input,
        &world,
        None,
        stale_resource_target
    ));

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    assert!(!chat_hit_target_is_current(&input, &world, None, target));

    world.push_server_system_chat_message("new delivery");
    assert!(!chat_hit_target_is_current(&input, &world, None, target));
}

#[test]
fn shift_chat_interaction_inserts_without_consuming_the_click() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = crate::control::NetCounters::default();
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    let mut platform = crate::simulation::effects::PlatformEffects::default();
    let interaction = crate::runtime::HudChatComponentInteraction {
        click_event: Some(ComponentClickEvent::SuggestCommand {
            command: "/clicked".to_string().into(),
        }),
        insertion: Some("inserted".to_string().into()),
    };
    assert!(!apply_chat_component_interaction(
        interaction.clone(),
        true,
        &mut input,
        &mut world,
        &mut counters,
        &None,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert_eq!(
        input
            .command_suggestion_hud_state(
                &world,
                &bbb_client_render::HudFontGlyphMap::default(),
                PhysicalSize::new(800, 600),
            )
            .map(|state| state.input_text),
        Some("inserted".to_string())
    );
    assert!(apply_chat_component_interaction(
        interaction,
        false,
        &mut input,
        &mut world,
        &mut counters,
        &None,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert_eq!(
        input
            .command_suggestion_hud_state(
                &world,
                &bbb_client_render::HudFontGlyphMap::default(),
                PhysicalSize::new(800, 600),
            )
            .map(|state| state.input_text),
        Some("/clicked".to_string())
    );
}

#[test]
fn chat_component_actions_preserve_safe_platform_and_network_boundaries() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = crate::control::NetCounters::default();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    assert!(run_component_click_event(
        ComponentClickEvent::RunCommand {
            command: "/seed".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(matches!(
        rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(input.command_confirmation().is_some());
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(
        matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(command)) if command.command == "seed")
    );
    assert!(run_component_click_event(
        ComponentClickEvent::Custom {
            id: "example:chat".to_string(),
            payload: None,
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::CustomClickAction(CustomClickAction { id, .. }))
            if id == "example:chat"
    ));
    assert!(run_component_click_event(
        ComponentClickEvent::CopyToClipboard {
            value: "copy me".to_string().into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    // `open_file` only ever comes from a client-made component, because the
    // serialized codec refuses it from a server.
    assert!(run_component_click_event(
        ComponentClickEvent::OpenFile {
            path: "/tmp/a".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(matches!(platform.take().as_slice(),
        [PlatformEffect::ClipboardWrite(text), PlatformEffect::OpenPath(path)]
        if text == &bbb_protocol::JavaString::from("copy me") && path == std::path::Path::new("/tmp/a")
    ));
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(counters.custom_click_action_commands_queued, 1);
}

#[test]
fn lectern_component_click_keeps_page_server_owned_and_closes_before_command() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 7,
        menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    let mut book = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(42),
        count: 1,
        component_patch: Default::default(),
    };
    book.component_patch.written_book = Some(bbb_protocol::packets::WrittenBookContentSummary {
        title: "Guide".into(),
        title_filter: None,
        author: "Alex".into(),
        generation: 0,
        pages: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        page_filters: vec![None, None, None],
        resolved: true,
    });
    assert!(
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![book],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        })
    );

    let mut counters = crate::control::NetCounters::default();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    assert!(apply_book_component_click_event(
        HudBookMode::Lectern,
        ComponentClickEvent::ChangePage { page: 3 },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert_eq!(world.lectern_book_view().unwrap().current_page, 0);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ContainerButtonClick(packet))
            if packet.container_id == 7 && packet.button_id == 102
    ));

    assert!(apply_book_component_click_event(
        HudBookMode::Lectern,
        ComponentClickEvent::RunCommand {
            command: "/seed".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(world.lectern_book_view().is_none());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ContainerClose(packet)) if packet.container_id == 7
    ));
    assert!(rx.try_recv().is_err());
    assert!(input.command_confirmation().is_some());
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ChatCommand(command)) if command.command == "seed" && command.signing.is_none()
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn handheld_book_run_command_closes_book_on_direct_or_confirmed_action() {
    use bbb_protocol::packets::{
        CommandArgumentParser, CommandNode, Commands, ContainerSetSlot, DataComponentPatchSummary,
        InteractionHand, ItemStackSummary, OpenBook, WrittenBookContentSummary,
        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
    };

    let written_book_id =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:written_book")
            .unwrap()
            .id;
    let open_handheld_book = |input: &mut crate::input::ClientInputState,
                              world: &mut WorldStore| {
        world.apply_open_book(OpenBook {
            hand: InteractionHand::MainHand,
        });
        input.apply_screen_requests(world, &mut crate::control::NetCounters::default(), &None);
        assert!(input.book_screen().is_some());
    };

    let mut world = WorldStore::new();
    assert!(world.apply_commands(Commands {
        root_index: 0,
        nodes: vec![
            CommandNode::root().with_children(vec![1, 2, 3]),
            CommandNode::literal("help").executes(),
            CommandNode::literal("seed").executes().restrict(),
            CommandNode::literal("say").with_children(vec![4]),
            CommandNode::argument(
                "message",
                CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".into(),
                    properties: vec![],
                }
            )
            .executes(),
        ],
    }));
    assert!(world.apply_container_set_slot(ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 36,
        item: ItemStackSummary {
            item_id: Some(written_book_id),
            count: 1,
            component_patch: DataComponentPatchSummary {
                written_book: Some(WrittenBookContentSummary {
                    title: "Guide".into(),
                    title_filter: None,
                    author: "Server".into(),
                    generation: 0,
                    pages: vec!["run".into()],
                    page_filters: vec![None],
                    resolved: true,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    }));

    let mut input = crate::input::ClientInputState::new(true);
    let mut counters = crate::control::NetCounters::default();
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut platform = crate::simulation::effects::PlatformEffects::default();

    open_handheld_book(&mut input, &mut world);
    assert!(apply_book_component_click_event(
        HudBookMode::View,
        ComponentClickEvent::RunCommand {
            command: "/help".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(input.book_screen().is_none());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ChatCommand(command)) if command.command == "help" && command.signing.is_none()
    ));

    open_handheld_book(&mut input, &mut world);
    assert!(apply_book_component_click_event(
        HudBookMode::View,
        ComponentClickEvent::RunCommand {
            command: "/seed".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(input.command_confirmation().is_some());
    assert!(input.command_confirmation().unwrap().has_parent);
    input.resolve_command_confirmation(
        false,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.book_screen().is_some());
    assert!(rx.try_recv().is_err());

    assert!(apply_book_component_click_event(
        HudBookMode::View,
        ComponentClickEvent::RunCommand {
            command: "/seed".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.book_screen().is_none());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ChatCommand(command)) if command.command == "seed" && command.signing.is_none()
    ));

    open_handheld_book(&mut input, &mut world);
    assert!(apply_book_component_click_event(
        HudBookMode::View,
        ComponentClickEvent::RunCommand {
            command: "/say hi".into(),
        },
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        &mut platform,
        &crate::input::FallbackTextMeasurer,
    ));
    assert!(input.command_confirmation().unwrap().open_chat_on_accept);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &crate::input::FallbackTextMeasurer,
    );
    assert!(input.book_screen().is_none());
    let chat_hud = input
        .command_suggestion_hud_state(
            &world,
            &crate::input::FallbackTextMeasurer,
            winit::dpi::PhysicalSize::new(320, 240),
        )
        .expect("chat screen open");
    assert_eq!(
        String::from_utf16_lossy(&chat_hud.input_view.expect("chat input view").value),
        "/say hi"
    );
}

#[test]
fn raw_clipboard_empty_payload_is_ignored_but_nul_payload_still_writes() {
    for (units, expected) in [(vec![], false), (vec![0], true)] {
        let mut input = crate::input::ClientInputState::new(true);
        let mut world = WorldStore::new();
        let mut counters = crate::control::NetCounters::default();
        let mut platform = crate::simulation::effects::PlatformEffects::default();
        assert!(run_component_click_event(
            ComponentClickEvent::CopyToClipboard {
                value: units.into()
            },
            &mut input,
            &mut world,
            &mut counters,
            &None,
            &mut platform,
            &crate::input::FallbackTextMeasurer,
        ));
        assert_eq!(!platform.take().is_empty(), expected);
    }
    let effects = RefCell::new(Vec::new());
    let mut clipboard = SimulationClipboard {
        read: None,
        effects: &effects,
    };
    let raw: bbb_protocol::JavaString = vec![0xd800, 0].into();
    assert!(clipboard.set_debug_clipboard_java_text(&raw));
    assert!(
        matches!(effects.borrow().as_slice(), [PlatformEffect::ClipboardWrite(text)] if text == &raw)
    );
}
