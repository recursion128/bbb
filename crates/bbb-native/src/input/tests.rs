use super::*;
use crate::item_runtime::NativeItemRuntime;
use bbb_protocol::packets::{
    AddEntity, BlockPos as ProtocolBlockPos, ChatCommand, CommandArgumentParser, CommandNode,
    CommandNodeType, CommandSuggestion, CommandSuggestionRequest, CommandSuggestions, Commands,
    CommonPlayerSpawnInfo, ContainerClick, ContainerCloseRequest, ContainerInput,
    ContainerSetContent as ProtocolContainerSetContent, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, FilterMask, FilterMaskKind, GameEvent as ProtocolGameEvent,
    HashedComponentPatch, HashedItemStack, HashedStack,
    ItemStackSummary as ProtocolItemStackSummary, LastSeenMessagesUpdate, MessageSignature,
    OpenScreen as ProtocolOpenScreen, OpenSignEditor, PaddleBoat, PlayLogin, PlayerAbilities,
    PlayerAbilitiesCommand, PlayerAction, PlayerChat, PlayerCommand, PlayerHealth, RenameItem,
    SelectBundleItem, SetCursorItem as ProtocolSetCursorItem,
    SetEntityData as ProtocolSetEntityData, SetPassengers,
    SetPlayerInventory as ProtocolSetPlayerInventory, SignUpdate, SignedMessageBody,
    Vec3d as ProtocolVec3d,
};
use bbb_protocol::packets::{ChatTypeBound, ChatTypeHolder};
use bbb_world::{
    BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkState, LightData,
    LocalPlayerPoseState, SignBlockEntityTextState, WorldStore,
};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const VANILLA_26_1_ELYTRA_ITEM_ID: i32 = 14;
const VANILLA_26_1_OAK_CHEST_BOAT_ENTITY_TYPE_ID: i32 = 90;
const VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID: i32 = 89;
const VANILLA_26_1_PLAYER_ENTITY_TYPE_ID: i32 = 155;
const VANILLA_ENTITY_DATA_POSE_ID: u8 = 6;
const VANILLA_POSE_SLEEPING_ID: i32 = 2;
const VANILLA_PLAYER_CHEST_EQUIPMENT_SLOT: i32 = 38;

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
    world_with_local_vehicle(player_id, 10, VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID)
}

fn world_with_local_vehicle(player_id: i32, vehicle_id: i32, entity_type_id: i32) -> WorldStore {
    let mut world = world_with_local_player_id(player_id);
    world.apply_add_entity(AddEntity {
        id: vehicle_id,
        uuid: Uuid::from_u128(vehicle_id as u128),
        entity_type_id,
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
        vehicle_id,
        passenger_ids: vec![player_id],
    }));
    world
}

fn world_with_sign_text(pos: BlockPos, front: [&str; 4], back: [&str; 4]) -> WorldStore {
    let mut world = WorldStore::new();
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: vec![BlockEntityRecord {
            local_x: pos.x.rem_euclid(16) as u8,
            y: i16::try_from(pos.y).unwrap(),
            local_z: pos.z.rem_euclid(16) as u8,
            type_id: 7,
            nbt: None,
            sign_text: Some(SignBlockEntityTextState {
                front: front.map(str::to_string),
                back: back.map(str::to_string),
            }),
        }],
        light: LightData::default(),
    });
    world
}

fn player_chat_with_signature(global_index: i32, signature: MessageSignature) -> PlayerChat {
    PlayerChat {
        global_index,
        sender: Uuid::from_u128(0x1234),
        index: global_index,
        signature: Some(signature),
        body: SignedMessageBody {
            content: format!("message {global_index}"),
            timestamp_millis: i64::from(global_index),
            salt: i64::from(global_index) + 1,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            target_name: None,
        },
    }
}

fn signable_message_command_tree() -> Commands {
    Commands {
        root_index: 0,
        nodes: vec![
            CommandNode {
                node_type: CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some("say".to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            CommandNode {
                node_type: CommandNodeType::Argument,
                flags: 6,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: 20,
                    name: "minecraft:message".to_string(),
                    properties: Vec::new(),
                }),
                suggestions: None,
                executable: true,
                restricted: false,
            },
        ],
    }
}

fn set_local_player_on_ground(world: &mut WorldStore, on_ground: bool) {
    world.set_local_player_pose(LocalPlayerPoseState {
        on_ground,
        ..LocalPlayerPoseState::default()
    });
}

fn set_player_abilities(world: &mut WorldStore, flying: bool, can_fly: bool) {
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying,
        can_fly,
        instabuild: can_fly,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

fn equip_player_slot(world: &mut WorldStore, slot: i32, item_id: i32, count: i32) {
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot,
        item: ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        },
    });
}

fn set_local_spectator(world: &mut WorldStore) {
    world.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(world.local_player_is_spectator());
}

fn world_with_sleeping_local_player(player_id: i32) -> WorldStore {
    let mut world = world_with_local_player_id(player_id);
    world.apply_add_entity(AddEntity {
        id: player_id,
        uuid: Uuid::from_u128(player_id as u128),
        entity_type_id: VANILLA_26_1_PLAYER_ENTITY_TYPE_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: player_id,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ENTITY_DATA_POSE_ID,
            serializer_id: 20,
            value: EntityDataValueKind::Pose(VANILLA_POSE_SLEEPING_ID),
        }],
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
fn spectator_digit_key_does_not_queue_held_slot() {
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
    assert!(rx.try_recv().is_err());
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
fn command_entry_with_signable_message_argument_submits_signed_command() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(signable_message_command_tree());
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say hello",
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
            command: "/say hello".to_string(),
        })
    );
    match rx.try_recv().unwrap() {
        NetCommand::ChatCommandSigned(packet) => {
            assert_eq!(packet.command, "say hello");
            assert!(packet.timestamp_millis > 0);
            assert_ne!(packet.salt, 0);
            assert!(packet.argument_signatures.entries.is_empty());
            assert_eq!(packet.last_seen_messages.offset, 1);
            assert_eq!(packet.last_seen_messages.acknowledged, 1 << 19);
            assert_ne!(
                packet.last_seen_messages.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
        }
        command => panic!("expected signed chat command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
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
fn chat_key_submits_message_with_pending_last_seen_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let _ = world.apply_player_chat(player_chat_with_signature(
        0,
        MessageSignature {
            bytes: vec![12; 256],
        },
    ));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "reply");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => {
            assert_eq!(packet.message, "reply");
            assert_eq!(packet.last_seen_messages.offset, 1);
            assert_eq!(packet.last_seen_messages.acknowledged, 1 << 19);
            assert_ne!(
                packet.last_seen_messages.checksum,
                LastSeenMessagesUpdate::default().checksum
            );
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
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
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn control_drop_key_queues_drop_all_items_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_drop_key_still_queues_drop_item_action() {
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
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
}

#[test]
fn drop_key_predicts_one_selected_item_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 2));
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropItem,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn control_drop_key_predicts_selected_stack_and_swings_when_non_empty() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    equip_player_slot(&mut world, 0, 42, 3);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(
        test_player_slot_item(&world, 0),
        ProtocolItemStackSummary::empty()
    );
    assert_eq!(counters.player_action_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAction(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            direction: ProtocolDirection::Down,
            sequence: 0,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
}

#[test]
fn spectator_drop_key_does_not_drop_or_swing() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    equip_player_slot(&mut world, 0, 42, 3);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(test_player_slot_item(&world, 0), test_item_stack(42, 3));
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.swing_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_hovered_drop_key_queues_throw_one_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [(
                36,
                HashedStack::Item(HashedItemStack {
                    item_id: 42,
                    count: 2,
                    components: HashedComponentPatch::default(),
                }),
            )]
            .into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_control_drop_key_queues_throw_stack_container_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ProtocolItemStackSummary {
            item_id: Some(42),
            count: 3,
            component_patch: Default::default(),
        },
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    assert!(input.control_down());
    assert!(!input.sprint);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 0,
            slot_num: 36,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [(36, HashedStack::Empty)].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn local_inventory_hovered_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(36);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_drop_key_without_hovered_slot_does_not_queue_player_drop() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_hovered_number_keys_queue_swap_container_clicks() {
    const HOTBAR_KEYS: [(KeyCode, i8); 9] = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (code, button_num) in HOTBAR_KEYS {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_hovered_slot = Some(9);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: i32::from(button_num),
            item: test_item_stack(42 + i32::from(button_num), 1),
        });
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 9,
            item: test_item_stack(99, 2),
        });
        assert!(world.open_local_inventory());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        match rx.try_recv().unwrap() {
            NetCommand::ContainerClick(click) => {
                assert_eq!(click.container_id, 0);
                assert_eq!(click.state_id, 0);
                assert_eq!(click.slot_num, 9);
                assert_eq!(click.button_num, button_num);
                assert_eq!(click.input, ContainerInput::Swap);
                assert_eq!(
                    click.changed_slots,
                    [
                        (9, test_hashed_item_stack(42 + i32::from(button_num), 1)),
                        (36 + i16::from(button_num), test_hashed_item_stack(99, 2)),
                    ]
                    .into()
                );
                assert_eq!(click.carried_item, HashedStack::Empty);
            }
            command => panic!("expected container click command, got {command:?}"),
        }
        assert_eq!(
            test_player_slot_item(&world, 9),
            test_item_stack(42 + i32::from(button_num), 1)
        );
        assert_eq!(
            test_player_slot_item(&world, i32::from(button_num)),
            test_item_stack(99, 2)
        );
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn local_inventory_hovered_offhand_swap_key_queues_swap_container_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(9);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 9,
        item: test_item_stack(42, 1),
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 40,
        item: test_item_stack(77, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, 0);
            assert_eq!(click.state_id, 0);
            assert_eq!(click.slot_num, 9);
            assert_eq!(click.button_num, 40);
            assert_eq!(click.input, ContainerInput::Swap);
            assert_eq!(
                click.changed_slots,
                [
                    (9, test_hashed_item_stack(77, 1)),
                    (45, test_hashed_item_stack(42, 1)),
                ]
                .into()
            );
            assert_eq!(click.carried_item, HashedStack::Empty);
        }
        command => panic!("expected container click command, got {command:?}"),
    }
    assert_eq!(test_player_slot_item(&world, 9), test_item_stack(77, 1));
    assert_eq!(test_player_slot_item(&world, 40), test_item_stack(42, 1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn local_inventory_swap_keys_without_hover_or_with_carried_item_are_no_ops() {
    for code in [KeyCode::Digit5, KeyCode::KeyF] {
        assert_local_inventory_swap_key_noop(code, None, ProtocolItemStackSummary::empty());
        assert_local_inventory_swap_key_noop(
            code,
            Some(36),
            ProtocolItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: Default::default(),
            },
        );
    }
}

fn assert_local_inventory_swap_key_noop(
    code: KeyCode,
    hovered_slot: Option<i16>,
    cursor_item: ProtocolItemStackSummary,
) {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = hovered_slot;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_set_cursor_item(ProtocolSetCursorItem { item: cursor_item });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(code),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_drop_key_queues_throw_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Throw,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(
        world
            .inventory()
            .open_container
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .find(|slot| slot.slot == 0)
            .unwrap()
            .item,
        test_item_stack(42, 3)
    );
}

#[test]
fn server_opened_container_hovered_control_drop_key_queues_throw_stack_click() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.control_left_down = true;
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 1,
            input: ContainerInput::Throw,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
}

#[test]
fn server_opened_container_empty_slot_drop_key_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_hovered_number_and_offhand_keys_queue_swap_clicks() {
    for (code, button_num) in [(KeyCode::Digit5, 4), (KeyCode::KeyF, 40)] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        input.inventory_hovered_slot = Some(0);
        let mut counters = NetCounters::default();
        let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
        assert_eq!(world.local_player().selected_hotbar_slot, 0);

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );

        assert_eq!(counters.container_click_commands_queued, 1);
        assert_eq!(counters.held_slot_commands_queued, 0);
        assert_eq!(counters.player_action_commands_queued, 0);
        assert_eq!(world.local_player().selected_hotbar_slot, 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ContainerClick(ContainerClick {
                container_id: 7,
                state_id: 12,
                slot_num: 0,
                button_num,
                input: ContainerInput::Swap,
                changed_slots: [].into(),
                carried_item: HashedStack::Empty,
            })
        );
    }
}

#[test]
fn server_opened_bundle_slot_swap_key_clears_selection_before_click() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_bundle_stack(42, 1, 3))));
    assert!(world.apply_local_select_bundle_item(0, 1));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.select_bundle_item_commands_queued, 1);
    assert_eq!(counters.container_click_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SelectBundleItem(SelectBundleItem {
            slot_id: 0,
            selected_item_index: -1,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClick(ContainerClick {
            container_id: 7,
            state_id: 12,
            slot_num: 0,
            button_num: 4,
            input: ContainerInput::Swap,
            changed_slots: [].into(),
            carried_item: HashedStack::Empty,
        })
    );
    assert_eq!(open_container_slot_bundle_selection(&world, 0), Some(-1));
    assert!(rx.try_recv().is_err());
}

#[test]
fn server_opened_container_swap_key_with_carried_item_is_consumed_without_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.inventory_hovered_slot = Some(0);
    let mut counters = NetCounters::default();
    let mut world = generic_9x1_container_world(7, 12, Some((0, test_item_stack(42, 3))));
    world.apply_set_cursor_item(ProtocolSetCursorItem {
        item: test_item_stack(99, 1),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    assert_eq!(counters.container_click_commands_queued, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_queues_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Sharp Pick",
    );

    assert_eq!(input.anvil_rename_text(), "Sharp Pick");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Sharp Pick".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_default_hover_name_when_item_runtime_is_available() {
    let root = unique_input_temp_dir("anvil-rename-default");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(0, 1)));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_backspace_queues_updated_rename_item_command() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Axe");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Ax");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Ax".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_rename_field_consumes_inventory_key_when_editable() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "e");

    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "e".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_filters_vanilla_invalid_chars_and_max_length() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));
    let text = format!("{}{}\u{a7}\n", "a".repeat(49), "🙂");

    handle_text_input(&mut input, &mut counters, &mut world, &commands, &text);

    let expected = "a".repeat(49);
    assert_eq!(input.anvil_rename_text(), expected);
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem { name: expected })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_queues_empty_sign_update() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
    });

    assert!(input.sign_editor_is_active_or_pending(&world));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
            lines: Default::default(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn pending_sign_editor_escape_preserves_existing_sign_text() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let sign_pos = BlockPos { x: 3, y: 64, z: -9 };
    let mut world = world_with_sign_text(
        sign_pos,
        ["Front 1", "Front 2", "Front 3", "Front 4"],
        ["Back 1", "Back 2", "Back 3", "Back 4"],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: sign_pos.x,
            y: sign_pos.y,
            z: sign_pos.z,
        },
        is_front_text: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos {
                x: sign_pos.x,
                y: sign_pos.y,
                z: sign_pos.z,
            },
            is_front_text: false,
            lines: [
                "Back 1".to_string(),
                "Back 2".to_string(),
                "Back 3".to_string(),
                "Back 4".to_string(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_text_input_changes_lines_and_queues_sign_update() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
        is_front_text: true,
    });

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Front");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Back");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowUp),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.forward);
    assert!(!input.sign_editor_is_active_or_pending(&world));
    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 3, y: 64, z: -9 },
            is_front_text: true,
            lines: [
                "Fron".to_string(),
                "Back".to_string(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_cursor_keys_edit_inside_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 8, y: 65, z: -3 },
        is_front_text: true,
    });

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "abcd");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "X");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Home),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, ">");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::End),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "<");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 8, y: 65, z: -3 },
            is_front_text: true,
            lines: [
                ">abd<".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_text_input_filters_invalid_chars_and_max_length() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
        is_front_text: true,
    });
    let text = format!("{}{}\u{a7}\n", "a".repeat(383), "🙂");

    handle_text_input(&mut input, &mut counters, &mut world, &commands, &text);
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    let expected = "a".repeat(383);
    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 2 },
            is_front_text: true,
            lines: [expected, String::new(), String::new(), String::new(),],
        })
    );
    assert!(rx.try_recv().is_err());
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
fn spectator_swap_offhand_key_does_not_queue_action() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyF),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn death_respawn_key_queues_perform_respawn() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert_eq!(counters.perform_respawn_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::PerformRespawn);
}

#[test]
fn death_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_player_health(PlayerHealth {
        health: 0.0,
        food: 0,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.perform_respawn_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn sleeping_escape_key_queues_stop_sleeping_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StopSleeping,
            data: 0,
        })
    );
}

#[test]
fn sleeping_state_consumes_gameplay_keys_without_queueing_actions() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sleeping_local_player(77);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyQ),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_key_opens_local_inventory_without_player_command() {
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

    assert!(world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), Some(0));
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_key_on_server_controlled_mount_queues_open_inventory_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_26_1_OAK_CHEST_BOAT_ENTITY_TYPE_ID);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
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
fn escape_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
    );
}

#[test]
fn inventory_key_closes_local_inventory_and_queues_container_zero() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_local_inventory());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert!(!world.local_inventory_is_open());
    assert_eq!(world.open_container_id(), None);
    assert_eq!(counters.container_close_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ContainerClose(ContainerCloseRequest { container_id: 0 })
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
fn gameplay_keys_are_consumed_while_unsupported_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: 19,
        title: "Merchant".to_string(),
    });

    for code in [KeyCode::Digit5, KeyCode::KeyQ] {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert_eq!(counters.player_action_commands_queued, 0);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(world.inventory().open_container.is_some());
    assert!(rx.try_recv().is_err());
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
fn release_active_input_keeps_shift_modifier_for_inventory_clicks() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    );
    assert!(input.shift_down());
    assert!(input.sneak);

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.shift_down());
    assert!(!input.sneak);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    );
    assert!(!input.shift_down());
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
fn jump_key_double_tap_toggles_creative_flight() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert!(world.local_player().abilities.unwrap().flying);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput::default())
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_double_tap_without_can_fly_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn spectator_jump_key_double_tap_does_not_toggle_flying_off() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_spectator(&mut world);
    set_player_abilities(&mut world, true, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_jump_trigger_expires_after_vanilla_window() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );
    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(250),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(350),
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 0);
    assert!(!world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(rx.try_recv().is_err());
}

#[test]
fn creative_flight_toggle_suppresses_same_jump_fall_flying_command() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_player_abilities(&mut world, false, true);
    set_local_player_on_ground(&mut world, true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Released,
    );

    set_local_player_on_ground(&mut world, false);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_CHEST_EQUIPMENT_SLOT,
        VANILLA_26_1_ELYTRA_ITEM_ID,
        1,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_abilities_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert!(world.local_player().abilities.unwrap().flying);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerAbilities(PlayerAbilitiesCommand { flying: true })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_queues_start_fall_flying_when_airborne_with_elytra() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_CHEST_EQUIPMENT_SLOT,
        VANILLA_26_1_ELYTRA_ITEM_ID,
        1,
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerCommand(PlayerCommand {
            entity_id: 77,
            action: PlayerCommandAction::StartFallFlying,
            data: 0,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_on_ground_with_elytra_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, true);
    equip_player_slot(
        &mut world,
        VANILLA_PLAYER_CHEST_EQUIPMENT_SLOT,
        VANILLA_26_1_ELYTRA_ITEM_ID,
        1,
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn jump_key_airborne_without_elytra_only_queues_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    set_local_player_on_ground(&mut world, false);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Space),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            jump: true,
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

fn test_item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn test_bundle_stack(
    item_id: i32,
    count: i32,
    bundle_contents_item_count: usize,
) -> ProtocolItemStackSummary {
    let mut stack = test_item_stack(item_id, count);
    stack.component_patch.bundle_contents_item_count = Some(bundle_contents_item_count);
    stack
}

fn test_hashed_item_stack(item_id: i32, count: i32) -> HashedStack {
    HashedStack::Item(HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch::default(),
    })
}

fn generic_9x1_container_world(
    container_id: i32,
    state_id: i32,
    item: Option<(usize, ProtocolItemStackSummary)>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: 0,
        title: "Chest".to_string(),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 45];
    if let Some((slot, stack)) = item {
        items[slot] = stack;
    }
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world
}

fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
    world
        .inventory()
        .open_container
        .as_ref()?
        .slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.local_selected_bundle_item_index)
}

fn anvil_container_world(
    container_id: i32,
    state_id: i32,
    input_item: Option<ProtocolItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: 8,
        title: "Anvil".to_string(),
    });
    let mut items = vec![ProtocolItemStackSummary::empty(); 39];
    if let Some(stack) = input_item {
        items[0] = stack;
    }
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world
}

fn write_input_tooltip_item_assets(root: &Path) {
    let assets = input_assets_dir(root);
    write_input_json(
        &assets.join("atlases").join("items.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }
            ]
        }"#,
    );
    write_input_json(
        &assets.join("atlases").join("blocks.json"),
        r#"{
            "sources": [
                {
                    "type": "minecraft:directory",
                    "prefix": "block/",
                    "source": "block"
                }
            ]
        }"#,
    );
    write_input_json(
        &assets.join("items").join("test_combo.json"),
        r#"{
            "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_input_json(
        &assets.join("models").join("item").join("test_combo.json"),
        r#"{
            "textures": {
                "layer0": "minecraft:item/test_combo"
            }
        }"#,
    );
    write_input_json(
        &assets.join("lang").join("en_us.json"),
        r#"{
            "item.minecraft.test_combo": "Test Combo"
        }"#,
    );
    write_input_png(
        &assets.join("textures").join("item").join("test_combo.png"),
        &[80, 120, 160, 255],
    );
    write_input_json(
        &root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("net")
            .join("minecraft")
            .join("world")
            .join("item")
            .join("Items.java"),
        r#"public class Items {
            public static final Item TEST_COMBO = registerItem("test_combo");
        }"#,
    );
}

fn input_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

fn write_input_json(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn write_input_png(path: &Path, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
}

fn unique_input_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-native-input-{label}-{nanos}"))
}

fn test_player_slot_item(world: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
    world
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}
