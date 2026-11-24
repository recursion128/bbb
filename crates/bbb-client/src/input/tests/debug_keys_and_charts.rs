use super::*;

#[test]
fn prediction_sequence_starts_at_one_and_wraps_like_java_int() {
    let mut world = WorldStore::new();

    assert_eq!(world.next_local_prediction_sequence(), 1);
    assert_eq!(world.next_local_prediction_sequence(), 2);

    world.set_local_prediction_sequence(i32::MAX);
    assert_eq!(world.next_local_prediction_sequence(), i32::MIN);
}

#[test]
fn debug_advanced_item_tooltips_startup_state_is_runtime_initial_value() {
    let mut input = ClientInputState::new(true);
    input.set_debug_advanced_item_tooltips(true);

    assert!(input.debug_advanced_item_tooltips());

    let commands = None;
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );

    assert!(!input.debug_advanced_item_tooltips());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(feedback, vec!["[Debug]: Advanced tooltips: hidden"]);
    // `keyDebugShowAdvancedTooltips` calls `options.save()` right after the
    // toggle, so the new value has to reach the file owner.
    assert_eq!(
        input.take_debug_client_option_writes(),
        vec![crate::options::ClientOptionWrite {
            key: "advancedItemTooltips",
            value: crate::options::ClientOptionValue::Boolean(false),
        }]
    );
    assert!(input.take_debug_client_option_writes().is_empty());
}

#[test]
fn focus_pause_toggle_queues_the_option_line_vanilla_saves() {
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    assert!(input.debug_pause_on_lost_focus());

    let commands = None;
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyP),
        ElementState::Pressed,
    );

    assert!(!input.debug_pause_on_lost_focus());
    assert_eq!(
        input.take_debug_client_option_writes(),
        vec![crate::options::ClientOptionWrite {
            key: "pauseOnLostFocus",
            value: crate::options::ClientOptionValue::Boolean(false),
        }]
    );
}

#[test]
fn digit_key_selects_hotbar_slot_updates_world_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(world.counters().held_slot_packets, 0);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(4));
}

#[test]
fn spectator_digit_key_opens_menu_without_queuing_held_slot() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    assert!(world.set_local_selected_hotbar_slot(2));
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 2);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(input.spectator_menu_is_active());
    assert!(rx.try_recv().is_err());
}

#[test]
fn delivered_movement_key_is_not_refiltered_by_cached_window_focus() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    // AppKit filters ordinary keyboard events before winit. An event that
    // reaches this handler was explicitly delivered (for example by the
    // exact-PID acceptance replay), matching Vanilla's handle-only guard.
    assert!(input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert!(rx.try_recv().is_ok());
}

#[test]
fn delivered_hotbar_and_drop_keys_are_not_refiltered_by_cached_window_focus() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 4);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_ok());
}

#[test]
fn f3_release_toggles_debug_overlay_without_gameplay_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    assert!(!input.debug_overlay_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_digit_chart_keys_toggle_overlay_state_and_do_not_toggle_on_f3_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_profiler_chart_visible());
    assert!(input
        .take_debug_profiler_chart_navigation_requests()
        .is_empty());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );
    assert!(input.debug_profiler_chart_visible());
    assert!(input.debug_fps_charts_visible());
    assert!(!input.debug_network_charts_visible());
    assert!(!input.debug_lightmap_texture_visible());
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_fps_charts_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit3),
        ElementState::Pressed,
    );
    assert!(input.debug_network_charts_visible());
    assert!(!input.debug_fps_charts_visible());
    assert!(!input.debug_lightmap_texture_visible());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::Digit4),
        ElementState::Pressed,
    );
    assert!(input.debug_lightmap_texture_visible());
    assert!(!input.debug_fps_charts_visible());
    assert!(!input.debug_network_charts_visible());
}

#[test]
fn profiler_chart_digit_navigation_records_without_blocking_hotbar() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit1),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(input.debug_profiler_chart_visible());
    assert!(input
        .take_debug_profiler_chart_navigation_requests()
        .is_empty());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit2),
        ElementState::Pressed,
    );

    assert_eq!(
        input.take_debug_profiler_chart_navigation_requests(),
        vec![2]
    );
    assert_eq!(world.local_player().selected_hotbar_slot, 1);
    assert_eq!(counters.held_slot_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::SetHeldSlot(1));
}

#[test]
fn f3_debug_status_keys_toggle_state_without_forcing_overlay_visible() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );
    assert!(input.debug_entity_hitboxes_visible());
    assert!(!input.debug_overlay_visible());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_entity_hitboxes_visible());
    assert!(!input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyG),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );
    assert!(input.debug_chunk_borders_visible());
    assert!(input.debug_advanced_item_tooltips());
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [KeyCode::KeyB, KeyCode::KeyG, KeyCode::KeyH] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert!(!input.debug_entity_hitboxes_visible());
    assert!(!input.debug_chunk_borders_visible());
    assert!(!input.debug_advanced_item_tooltips());
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Hitboxes: shown",
            "[Debug]: Chunk borders: shown",
            "[Debug]: Advanced tooltips: shown",
            "[Debug]: Hitboxes: hidden",
            "[Debug]: Chunk borders: hidden",
            "[Debug]: Advanced tooltips: hidden",
        ]
    );
    assert!(world
        .client_chat()
        .messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(world.counters().chat_messages_tracked, 6);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn shared_debug_hotkeys_toggle_dev_state_when_startup_flag_is_enabled() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyL,
        KeyCode::KeyO,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
    ] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    input.shift_left_down = true;
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyU),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionOctree),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionVisibility),
        DebugScreenEntryStatus::AlwaysOn
    );
    assert!(!input.debug_fog_enabled());
    assert!(!input.debug_smart_cull_enabled());
    assert!(input.debug_wireframe_enabled);
    assert_eq!(
        input.take_debug_frustum_requests(),
        vec![DebugFrustumRequest::Capture, DebugFrustumRequest::Kill]
    );
    assert!(input.take_debug_feature_count_requests().is_empty());
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());

    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: SectionPath: shown",
            "[Debug]: Fog: disabled",
            "[Debug]: SmartCull: disabled",
            "[Debug]: Frustum culling Octree: enabled",
            "[Debug]: Captured frustum",
            "[Debug]: SectionVisibility: enabled",
            "[Debug]: WireFrame: enabled",
            "[Debug]: Killed frustum",
        ]
    );
}

#[test]
fn shared_debug_hotkeys_without_player_only_handle_global_dev_toggles() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths),
        DebugScreenEntryStatus::Never
    );
    assert!(!input.debug_fog_enabled());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_feature_count_hotkeys_are_gated_ahead_of_profiling() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_feature_count_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyR),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(
        input.take_debug_feature_count_requests(),
        vec![
            DebugFeatureCountRequest::Log,
            DebugFeatureCountRequest::Clear
        ]
    );
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_hotkeys_take_priority_over_feature_count_and_regular_keymap() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    input.set_debug_hotkeys_enabled(true);
    input.set_debug_feature_count_enabled(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_smart_cull_enabled());
    assert!(input.take_debug_feature_count_requests().is_empty());
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_debug_status_key_uses_in_overlay_status_when_overlay_is_visible() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );

    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::InOverlay
    );
    assert!(input.debug_entity_hitboxes_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());
    assert!(input.debug_entity_hitboxes_visible());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(!input.debug_overlay_visible());
    assert!(!input.debug_entity_hitboxes_visible());
}

#[test]
fn f3_debug_status_toggle_persists_debug_profile_store() {
    let commands = None;
    let path = unique_input_temp_dir("debug-profile-store").join("debug-profile.json");
    let mut input = ClientInputState::new(true);
    input.set_debug_profile_store_path(path.clone());
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );

    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
        .expect("debug profile json parses");
    assert!(value.get("profile").is_none());
    assert_eq!(
        value["custom"]["minecraft:entity_hitboxes"].as_str(),
        Some("alwaysOn")
    );

    let loaded =
        DebugScreenEntryList::load_from_debug_profile_file(&path, DebugScreenProfile::Default)
            .unwrap();
    assert_eq!(
        loaded.status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::AlwaysOn
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(path.parent().unwrap());
}

#[test]
fn custom_debug_screen_entry_status_drives_entry_visibility() {
    let mut input = ClientInputState::new(true);

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryStatus::AlwaysOn,
    );
    assert!(input.debug_chunk_borders_visible());

    input.set_debug_screen_entry_status(
        DebugScreenEntryId::ChunkBorders,
        DebugScreenEntryStatus::Never,
    );
    assert!(!input.debug_chunk_borders_visible());
}

#[test]
fn f3_debug_status_keys_follow_player_reduced_debug_gate() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyB),
        ElementState::Pressed,
    );
    assert!(!input.debug_entity_hitboxes_visible());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyH),
        ElementState::Pressed,
    );
    assert!(input.debug_advanced_item_tooltips());
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(input.debug_overlay_visible());

    let mut reduced_world = world_with_debug_player(true);
    let mut reduced_input = ClientInputState::new(true);
    handle_key_input(
        &mut reduced_input,
        &mut counters,
        &mut reduced_world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for code in [KeyCode::KeyB, KeyCode::KeyG] {
        handle_key_input(
            &mut reduced_input,
            &mut counters,
            &mut reduced_world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }
    assert!(!reduced_input.debug_entity_hitboxes_visible());
    assert!(!reduced_input.debug_chunk_borders_visible());
    handle_key_input(
        &mut reduced_input,
        &mut counters,
        &mut reduced_world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(reduced_input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_d_clears_chat_display_without_resetting_signature_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![9; 256],
        },
    ));
    world.apply_delete_chat(DeleteChat {
        message_signature: PackedMessageSignature {
            cache_id: Some(0),
            full_signature: None,
        },
    });
    assert_eq!(world.client_chat().messages.len(), 1);
    assert_eq!(world.client_chat().deleted_messages.len(), 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyD),
        ElementState::Pressed,
    );
    assert!(world.client_chat().messages.is_empty());
    assert!(world.client_chat().deleted_messages.is_empty());
    assert_eq!(world.client_chat().expected_player_chat_global_index, 1);
    assert!(world
        .client_chat()
        .signature_cache
        .iter()
        .any(Option::is_some));
    assert_eq!(world.client_chat().acknowledgement.pending_offset, 1);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(world.counters().chat_messages_tracked, 0);
    assert_eq!(world.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(world.counters().chat_signature_cache_entries, 1);
    assert_eq!(world.counters().reset_chat_packets, 0);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_v_dumps_version_debug_chat_without_toggling_overlay_on_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
    );

    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 10);
    assert!(messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(messages[0].content, "[Debug]: Client version info:");
    assert_eq!(messages[1].content, format!("id = {MC_VERSION}"));
    assert_eq!(messages[2].content, format!("name = {MC_VERSION}"));
    assert_eq!(messages[3].content, format!("data = {MC_DATA_VERSION}"));
    assert_eq!(
        messages[4].content,
        format!("series = {MC_DATA_VERSION_SERIES}")
    );
    assert_eq!(
        messages[5].content,
        format!("protocol = {PROTOCOL_VERSION} (0x{PROTOCOL_VERSION:x})")
    );
    assert_eq!(messages[6].content, format!("build_time = {MC_BUILD_TIME}"));
    assert_eq!(
        messages[7].content,
        format!(
            "pack_resource = {}",
            MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
        )
    );
    assert_eq!(
        messages[8].content,
        format!("pack_data = {}", MC_DATA_PACK_FORMAT.to_vanilla_string())
    );
    assert_eq!(
        messages[9].content,
        if MC_STABLE {
            "stable = yes"
        } else {
            "stable = no"
        }
    );
    assert_eq!(world.counters().chat_messages_tracked, 10);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_v_consumes_without_world_to_suppress_release_toggle() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_t_records_resource_pack_reload_request_and_feedback_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );

    assert_eq!(input.take_debug_resource_pack_reload_requests(), 1);
    assert_eq!(input.take_debug_resource_pack_reload_requests(), 0);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(messages[0].content, "[Debug]: Reloaded resource packs");
    assert_eq!(messages[0].styled_content.len(), 3);
    assert_eq!(messages[0].styled_content[0].text, "[Debug]:");
    assert_eq!(messages[0].styled_content[0].style.bold, Some(true));
    assert_eq!(
        messages[0].styled_content[0].style.color,
        Some(VANILLA_DEBUG_FEEDBACK_COLOR)
    );
    assert_eq!(messages[0].styled_content[1].text, " ");
    assert_eq!(
        messages[0].styled_content[1].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[2].text,
        "Reloaded resource packs"
    );
    assert_eq!(
        messages[0].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_t_consumes_without_world_and_records_reload_request() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_resource_pack_reload_requests(), 1);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_s_records_dynamic_texture_dump_request_and_feedback_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyS),
        ElementState::Pressed,
    );

    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 1);
    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 0);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Saved dynamic textures to screenshots/debug"
    );
    assert_eq!(messages[0].styled_content.len(), 4);
    assert_eq!(messages[0].styled_content[0].text, "[Debug]:");
    assert_eq!(messages[0].styled_content[0].style.bold, Some(true));
    assert_eq!(
        messages[0].styled_content[0].style.color,
        Some(VANILLA_DEBUG_FEEDBACK_COLOR)
    );
    assert_eq!(messages[0].styled_content[1].text, " ");
    assert_eq!(
        messages[0].styled_content[1].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[2].text,
        "Saved dynamic textures to "
    );
    assert_eq!(
        messages[0].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[0].styled_content[3].text,
        DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH
    );
    assert_eq!(messages[0].styled_content[3].style.underlined, Some(true));
    assert_eq!(
        messages[0].styled_content[3].style.click_event,
        Some(ComponentClickEvent::OpenFile {
            path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.into(),
        })
    );
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().player_chat_packets, 0);
    assert_eq!(world.counters().disguised_chat_packets, 0);
    assert_eq!(world.counters().system_chat_packets, 0);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );
    assert!(!input.debug_overlay_visible());
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_s_consumes_without_world_and_records_dump_request() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyS),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_dynamic_texture_dump_requests(), 1);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}
