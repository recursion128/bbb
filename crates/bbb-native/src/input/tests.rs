use super::*;
use bbb_protocol::packets::{
    AddEntity, BlockPos as ProtocolBlockPos, ChatCommand, CommandSuggestion,
    CommandSuggestionRequest, CommandSuggestions, CommonPlayerSpawnInfo, ContainerCloseRequest,
    LastSeenMessagesUpdate, OpenScreen as ProtocolOpenScreen, PaddleBoat, PlayLogin, PlayerAction,
    PlayerCommand, SetPassengers, Vec3d as ProtocolVec3d,
};
use bbb_world::{BlockPos, WorldStore};
use uuid::Uuid;

const VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID: i32 = 89;

fn handle_key_input_without_world(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    let mut world = WorldStore::new();
    handle_key_input(
        input,
        counters,
        &mut world,
        net_commands,
        physical_key,
        state,
    );
}

fn handle_text_input_without_world(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    let mut world = WorldStore::new();
    handle_text_input(input, counters, &mut world, net_commands, text);
}

fn world_with_local_player_id(player_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
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
    });
    world
}

fn world_with_local_boat(player_id: i32) -> WorldStore {
    let mut world = world_with_local_player_id(player_id);
    world.apply_add_entity(AddEntity {
        id: 10,
        uuid: Uuid::from_u128(10),
        entity_type_id: VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![player_id],
    }));
    world
}

#[test]
fn prediction_sequence_starts_at_one_and_wraps_positive() {
    let mut world = WorldStore::new();

    assert_eq!(world.next_local_prediction_sequence(), 1);
    assert_eq!(world.next_local_prediction_sequence(), 2);

    world.set_local_prediction_sequence(i32::MAX);
    assert_eq!(world.next_local_prediction_sequence(), 1);
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
fn unfocused_movement_key_does_not_queue_player_input() {
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

    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn unfocused_hotbar_or_drop_key_does_not_queue_command() {
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

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn slash_text_opens_command_entry_and_releases_pressed_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string(),
        })
    );
    assert_eq!(
        world.command_suggestions().last_request.as_ref(),
        Some(&bbb_world::CommandSuggestionRequestState {
            id: 0,
            command: "/".to_string(),
        })
    );
}

#[test]
fn command_entry_submits_slash_command_without_leading_slash() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/time set day",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/time set day".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(ChatCommand {
            command: "time set day".to_string(),
        })
    );
}

#[test]
fn chat_key_opens_chat_entry_and_submits_unsigned_message() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("")
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "hello   world",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "hello world");
            assert!(packet.timestamp_millis > 0);
            assert_ne!(packet.salt, 0);
            assert_eq!(packet.last_seen_messages, LastSeenMessagesUpdate::default());
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_starting_with_slash_requests_suggestions_and_submits_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 1);
    assert_eq!(counters.chat_message_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(ChatCommand {
            command: "seed".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_blocks_movement_keys_and_backspace_edits_text() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givw");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert!(!input.forward);
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/givw".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/giv".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 2,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_escape_cancels_without_queuing_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn slash_only_enter_requests_suggestions_but_does_not_submit_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_multi_character_commit_requests_suggestions_once() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/seed");

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/seed")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_applies_latest_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 3,
        suggestions: vec![CommandSuggestion {
            text: "give".to_string(),
            tooltip: Some("Run give".to_string()),
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(
        input
            .chat_entry
            .as_ref()
            .and_then(|entry| entry.last_suggestion_request_text.as_deref()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/giv".to_string(),
        })
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(ChatCommand {
            command: "give".to_string(),
        })
    );
}

#[test]
fn command_entry_tab_ignores_stale_suggestion_response() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/giv");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 3,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".to_string(),
            tooltip: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/giv".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 1,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_tab_ignores_invalid_suggestion_range() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/give");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 10,
        length: 1,
        suggestions: vec![CommandSuggestion {
            text: "gamemode".to_string(),
            tooltip: None,
        }],
    });
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/give")
    );
    assert_eq!(counters.command_suggestion_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/give".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn text_input_requires_focus_and_leading_slash() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(false);
    let mut counters = NetCounters::default();

    handle_text_input_without_world(&mut input, &mut counters, &commands, "/seed");
    assert!(!input.chat_entry_is_active());

    input.focused = true;
    handle_text_input_without_world(&mut input, &mut counters, &commands, "seed");

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.chat_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn drop_key_queues_drop_item_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn control_drop_key_queues_drop_all_items_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn swap_offhand_key_queues_swap_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::SwapItemWithOffhand,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn inventory_key_queues_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::OpenInventory,
            data: 0,
        })
    );
}

#[test]
fn escape_key_closes_open_container_and_queues_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 2,
        title: "Chest".to_string(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(world.counters().container_close_updates_received, 0);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 7 })
    );
}

#[test]
fn inventory_key_closes_open_container_before_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: 2,
        title: "Chest".to_string(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 8 })
    );
}

#[test]
fn escape_key_without_open_container_does_not_queue_command() {
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
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn movement_key_changes_queue_player_input_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
    );

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.player_input_commands_queued, 1);

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
}

#[test]
fn sprint_key_queues_player_input_and_sprint_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        })
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
}

#[test]
fn sprint_key_without_local_player_id_only_queues_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn focus_loss_clears_pressed_input_and_queues_release() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.jump = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
}

#[test]
fn focus_loss_stops_active_sprinting() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        })
    );
}

#[test]
fn focus_loss_stops_boat_paddles() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.left = true;
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_boat(77);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(player_input_from_state(&input), PlayerInput::default());
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.paddle_boat_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PaddleBoat(PaddleBoat {
            left: false,
            right: false,
        })
    );
}

#[test]
fn focus_loss_aborts_destroying_block() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_destroying_block(BlockPos { x: 4, y: 70, z: -6 });
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert_eq!(world.local_player().interaction.destroying_block, None);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::AbortDestroyBlock,
            pos: ProtocolBlockPos { x: 4, y: 70, z: -6 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn focus_loss_releases_using_item() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!world.local_player().interaction.using_item);
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::ReleaseUseItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}
