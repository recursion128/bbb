use super::*;
use bbb_item_model::NativeItemRuntime;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREEPER_ID;
use bbb_protocol::packets::BlockEntityData;
use bbb_protocol::packets::{
    AddEntity, AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
    AdvancementSummary, BlockEntityTagQuery, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, ChatCommand, CommandArgumentParser, CommandNode,
    CommandNodeType, CommandSuggestion, CommandSuggestionRequest, CommandSuggestions, Commands,
    CommonPlayerSpawnInfo, ContainerClick, ContainerCloseRequest, ContainerInput,
    ContainerSetContent as ProtocolContainerSetContent, DataComponentPatchSummary, DeleteChat,
    DialogHolder, EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityTagQuery, EquipmentSlot, EquipmentSlotUpdate,
    FilterMask, FilterMaskKind, GameEvent as ProtocolGameEvent, HashedComponentPatch,
    HashedItemStack, HashedStack, ItemStackSummary as ProtocolItemStackSummary,
    LastSeenMessagesUpdate, MessageSignature, OpenBook, OpenScreen as ProtocolOpenScreen,
    OpenSignEditor, PackedMessageSignature, PaddleBoat, PlayLogin, PlayerAbilities,
    PlayerAbilitiesCommand, PlayerAction, PlayerChat, PlayerCommand, PlayerHealth, RenameItem,
    SeenAdvancements, SelectBundleItem, SetCursorItem as ProtocolSetCursorItem,
    SetEntityData as ProtocolSetEntityData, SetEquipment, SetPassengers,
    SetPlayerInventory as ProtocolSetPlayerInventory, ShowDialog as ProtocolShowDialog, SignUpdate,
    SignedMessageBody, TagQuery, UpdateAdvancements, Vec3d as ProtocolVec3d,
    WrittenBookContentSummary,
};
use bbb_protocol::packets::{ChatTypeBound, ChatTypeHolder};
use bbb_protocol::{
    MC_BUILD_TIME, MC_DATA_PACK_FORMAT, MC_DATA_VERSION, MC_DATA_VERSION_SERIES,
    MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION, PROTOCOL_VERSION,
};
use bbb_world::{
    BlockEntityRecord, BlockPos, BlockStateRegistry, ChatMessageKind, ChunkColumn, ChunkPos,
    ChunkSection, ChunkState, ItemEquipmentSlot, LightData, LocalPlayerPoseState, PaletteDomain,
    PaletteKind, PalettedContainerData, SignBlockEntityTextState, WorldStore,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const VANILLA_26_1_ELYTRA_ITEM_ID: i32 = 14;
const VANILLA_26_1_CAMEL_ENTITY_TYPE_ID: i32 = 19;
const VANILLA_26_1_CAMEL_HUSK_ENTITY_TYPE_ID: i32 = 20;
const VANILLA_26_1_HORSE_ENTITY_TYPE_ID: i32 = 66;
const VANILLA_26_1_OAK_CHEST_BOAT_ENTITY_TYPE_ID: i32 = 90;
const VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID: i32 = 89;
const VANILLA_26_1_PLAYER_ENTITY_TYPE_ID: i32 = 155;
const TEST_SADDLE_ITEM_ID: i32 = 8_903;
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

fn world_with_debug_player(reduced_debug_info: bool) -> WorldStore {
    world_with_debug_player_in_game_mode(reduced_debug_info, 0, -1)
}

fn world_with_debug_player_in_game_mode(
    reduced_debug_info: bool,
    game_type: i8,
    previous_game_type: i8,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type,
            previous_game_type,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
    world
}

fn grant_debug_recreate_nbt_permission(world: &mut WorldStore) {
    let player_id = world
        .local_player_id()
        .expect("debug recreate test world has a local player");
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: player_id,
        event_id: 26,
    }));
    assert!(world.local_player_has_gamemaster_permission());
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

#[derive(Default)]
struct MockDebugClipboard {
    text: Option<String>,
    accepts_text: bool,
}

impl MockDebugClipboard {
    fn accepting() -> Self {
        Self {
            text: None,
            accepts_text: true,
        }
    }
}

impl DebugClipboard for MockDebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        if self.accepts_text {
            self.text = Some(text.to_string());
            true
        } else {
            false
        }
    }
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
    if entity_type_id == VANILLA_26_1_HORSE_ENTITY_TYPE_ID {
        world.set_default_item_equipment_slots(BTreeMap::from([(
            TEST_SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: vehicle_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: ProtocolItemStackSummary {
                    item_id: Some(TEST_SADDLE_ITEM_ID),
                    count: 1,
                    component_patch: Default::default(),
                },
            }],
        }));
    }
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id,
        passenger_ids: vec![player_id],
    }));
    world
}

fn sign_text_side(lines: [&str; 4]) -> bbb_world::SignTextSideState {
    bbb_world::SignTextSideState {
        lines: lines.map(|line| {
            if line.is_empty() {
                Vec::new()
            } else {
                vec![bbb_protocol::StyledTextRun {
                    text: line.to_string(),
                    style: bbb_protocol::ComponentStyle::default(),
                }]
            }
        }),
        color: bbb_world::SignTextDyeColor::Black,
        has_glowing_text: false,
    }
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
            raw_nbt: vec![0],
            nbt: None,
            sign_text: Some(SignBlockEntityTextState {
                front: sign_text_side(front),
                back: sign_text_side(back),
                is_waxed: false,
            }),
            vault_shared_data: None,
            decorated_pot_sherds: None,
            banner_patterns: None,
            end_gateway: None,
            spawner: None,
        }],
        light: LightData::default(),
    });
    world
}

fn insert_empty_chunk_for_block(world: &mut WorldStore, pos: BlockPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24).map(|_| empty_chunk_section()).collect(),
        block_entities: Vec::new(),
        light: LightData::default(),
    });
}

fn empty_chunk_section() -> ChunkSection {
    ChunkSection {
        non_empty_block_count: 0,
        fluid_count: 0,
        block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
        biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
    }
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &string_props(props))
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {props:?}"))
        .id
}

fn string_props<const N: usize>(props: [(&str, &str); N]) -> BTreeMap<String, String> {
    props
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
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
            path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
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

#[test]
fn f3_i_consumes_copy_data_modifier_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyI),
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
fn shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_with_permission_copies_local_block_entity_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id: 9,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]{Lock:\"secret\"}")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn shift_f3_i_without_permission_omits_local_block_entity_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id: 9,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_without_shift_records_server_recreate_query_request_shell() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert_eq!(
        input.pending_debug_recreate_server_query,
        Some(PendingDebugRecreateServerQuery {
            transaction_id: 0,
            target: PendingDebugRecreateServerQueryTarget::Block {
                pos: target_pos,
                description: "minecraft:oak_log[axis=x]".to_string(),
            },
        })
    );
    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_i_without_gamemaster_permission_copies_client_recreate_command_without_query() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_block_tag_response_copies_recreate_command() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_string("id", "minecraft:chest"),
            nbt_string("Lock", "secret"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]{Lock:\"secret\",id:\"minecraft:chest\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_entity_tag_response_copies_recreate_command_without_uuid_or_pos() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 0,
            entity_id: 50,
        }]
    );
    world.apply_tag_query(TagQuery {
        transaction_id: 99,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    assert!(!input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert!(input.pending_debug_recreate_server_query.is_some());

    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_long_array("UUID", &[1, 2]),
            nbt_list(
                "Pos",
                6,
                vec![
                    0.0f64.to_be_bytes().to_vec(),
                    0.0f64.to_be_bytes().to_vec(),
                    3.0f64.to_be_bytes().to_vec(),
                ],
            ),
            nbt_byte("Charged", 1),
            nbt_string("CustomName", "Boom"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00 {Charged: 1b, CustomName: \"Boom\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side entity data to clipboard"
    );
}

#[test]
fn f3_i_null_server_tag_response_copies_command_without_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    input.take_debug_recreate_server_query_requests();
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: false,
        raw_nbt: vec![0],
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:stone")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side block data to clipboard"
    );
}

#[test]
fn queues_debug_recreate_server_query_requests_as_tag_query_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();

    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        },
    );
    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 4,
            entity_id: 88,
        },
    );

    assert_eq!(counters.block_entity_tag_query_commands_queued, 1);
    assert_eq!(counters.entity_tag_query_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::BlockEntityTagQuery(BlockEntityTagQuery {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::EntityTagQuery(EntityTagQuery {
            transaction_id: 4,
            entity_id: 88,
        })
    );
}

fn nbt_compound(entries: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![10];
    for entry in entries {
        out.extend_from_slice(&entry);
    }
    out.push(0);
    out
}

fn nbt_byte(name: &str, value: i8) -> Vec<u8> {
    let mut out = vec![1];
    write_mutf8(&mut out, name);
    out.push(value as u8);
    out
}

fn nbt_string(name: &str, value: &str) -> Vec<u8> {
    let mut out = vec![8];
    write_mutf8(&mut out, name);
    write_mutf8(&mut out, value);
    out
}

fn nbt_list(name: &str, element_type: u8, values: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![9];
    write_mutf8(&mut out, name);
    out.push(element_type);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value);
    }
    out
}

fn nbt_long_array(name: &str, values: &[i64]) -> Vec<u8> {
    let mut out = vec![12];
    write_mutf8(&mut out, name);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    let mut bytes = Vec::new();
    for unit in value.encode_utf16() {
        if unit == 0 {
            bytes.extend_from_slice(&[0xc0, 0x80]);
        } else if unit <= 0x7f {
            bytes.push(unit as u8);
        } else if unit <= 0x7ff {
            bytes.push((0xc0 | ((unit >> 6) & 0x1f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        } else {
            bytes.push((0xe0 | ((unit >> 12) & 0x0f)) as u8);
            bytes.push((0x80 | ((unit >> 6) & 0x3f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        }
    }
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
}

#[test]
fn shift_f3_i_copies_entity_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_with_permission_copies_local_entity_transform_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        },
        x_rot: 10.0,
        y_rot: 45.0,
        y_head_rot: 45.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: ENTITY_SHARED_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(ENTITY_SHARED_FLAG_GLOWING),
            },
            ProtocolEntityDataValue {
                data_id: ENTITY_AIR_SUPPLY_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(123),
            },
            ProtocolEntityDataValue {
                data_id: ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: ENTITY_SILENT_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: ENTITY_NO_GRAVITY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: ENTITY_TICKS_FROZEN_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(42),
            },
        ],
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:creeper 0.00 0.00 3.00 \
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], Air: 123s, \
             CustomNameVisible: 1b, Silent: 1b, NoGravity: 1b, Glowing: 1b, TicksFrozen: 42}"
        )
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );
}

#[test]
fn f3_i_is_consumed_but_does_not_copy_when_reduced_debug_info_blocks_inspect() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_block_state_description_matches_vanilla_recreate_property_format() {
    assert_eq!(
        debug_block_state_description("minecraft:stone", &BTreeMap::new()),
        "minecraft:stone"
    );
    assert_eq!(
        debug_block_state_description(
            "minecraft:mangrove_propagule",
            &string_props([
                ("stage", "1"),
                ("hanging", "true"),
                ("waterlogged", "false"),
                ("age", "2"),
            ])
        ),
        "minecraft:mangrove_propagule[age=2,hanging=true,stage=1,waterlogged=false]"
    );
}

#[test]
fn f3_l_toggles_profiling_feedback_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Start]
    );
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Stop]
    );
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Profiling started for 10 seconds. Use F3 + L to stop early"
    );
    assert_eq!(messages[1].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[1].content,
        "[Debug]: Profiling ended. Results folder debug/profiling"
    );
    assert_eq!(messages[1].styled_content.len(), 4);
    assert_eq!(
        messages[1].styled_content[2].text,
        "Profiling ended. Results folder "
    );
    assert_eq!(
        messages[1].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[1].styled_content[3].text,
        DEBUG_PROFILING_RESULTS_RELATIVE_DIR
    );
    assert_eq!(messages[1].styled_content[3].style.underlined, Some(true));
    assert_eq!(
        messages[1].styled_content[3].style.click_event,
        Some(ComponentClickEvent::OpenFile {
            path: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
        })
    );
    assert_eq!(world.counters().chat_messages_tracked, 2);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_f6_records_debug_options_screen_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F6),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_options_screen_requests(), 1);
    assert_eq!(input.take_debug_options_screen_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_escape_records_pause_without_menu_request_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        None,
        None
    ));
    assert_eq!(input.take_debug_pause_without_menu_requests(), 1);
    assert_eq!(input.take_debug_pause_without_menu_requests(), 0);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_copies_location_tp_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/execute in minecraft:overworld run tp @s 10.25 64.00 -5.75 90.50 -15.25")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(messages[0].content, "[Debug]: Copied location to clipboard");
    assert_eq!(world.counters().chat_messages_tracked, 1);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_reduced_debug_blocks_location_copy_but_still_consumes_crash_modifier() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.25,
            y: 64.0,
            z: -5.75,
        },
        y_rot: 90.5,
        x_rot: -15.25,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert_eq!(clipboard.text, None);
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_c_hold_reports_manual_crash_warning_countdown_without_toggling_overlay() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

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
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
    );

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(999),
    );
    assert!(world.client_chat().messages.is_empty());

    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(1),
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(2),
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].content,
        "[Debug]: F3 + C is held down. This will crash the game unless released."
    );
    assert_eq!(messages[1].content, "[Debug]: Crashing in 8...");

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Released,
    );
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_secs(3),
    );
    assert_eq!(world.client_chat().messages.len(), 2);

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
#[should_panic(expected = "Manually triggered debug crash")]
fn f3_c_hold_panics_after_vanilla_manual_crash_delay() {
    let (tx, _rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    let start = Instant::now();

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
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
    );

    advance_player_input(&mut input, &mut world, &mut counters, &commands, start);
    advance_player_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        start + std::time::Duration::from_millis(10_001),
    );
}

#[test]
fn f3_game_mode_keys_report_no_permission_without_gameplay_commands() {
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
        PhysicalKey::Code(KeyCode::KeyN),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
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
    let feedback: Vec<_> = world
        .client_chat()
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(
        feedback,
        vec![
            "[Debug]: Unable to switch game mode; no permission",
            "[Debug]: Unable to open game mode switcher; no permission",
        ]
    );
    assert!(world
        .client_chat()
        .messages
        .iter()
        .all(|message| message.kind == ChatMessageKind::ClientSystem));
    assert_eq!(counters.player_input_commands_queued, 0);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(counters.change_game_mode_commands_queued, 0);
    assert_eq!(input.debug_game_mode_switcher_selected(), None);
    assert!(rx.try_recv().is_err());
}

#[test]
fn f3_n_queues_spectator_game_mode_with_permission() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

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
        PhysicalKey::Code(KeyCode::KeyN),
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
    assert!(world.client_chat().messages.is_empty());
    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_n_queues_previous_or_creative_game_mode_when_already_spectator() {
    for (previous_game_type, expected_game_mode) in [
        (-1, bbb_protocol::packets::GameType::Creative),
        (0, bbb_protocol::packets::GameType::Survival),
        (2, bbb_protocol::packets::GameType::Adventure),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, 3, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);
        assert!(world.local_player_is_spectator());

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
            PhysicalKey::Code(KeyCode::KeyN),
            ElementState::Pressed,
        );

        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_queues_default_selection_on_f3_release() {
    for (game_type, previous_game_type, expected_game_mode) in [
        (0, -1, bbb_protocol::packets::GameType::Creative),
        (1, -1, bbb_protocol::packets::GameType::Survival),
        (3, 0, bbb_protocol::packets::GameType::Survival),
    ] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player_in_game_mode(false, game_type, previous_game_type);
        grant_debug_recreate_nbt_permission(&mut world);

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
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );

        assert_eq!(
            input.debug_game_mode_switcher_selected(),
            Some(expected_game_mode)
        );
        assert_eq!(counters.change_game_mode_commands_queued, 0);
        assert!(rx.try_recv().is_err());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
        );

        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(!input.debug_overlay_visible());
        assert_eq!(counters.change_game_mode_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
                game_mode: expected_game_mode,
            })
        );
    }
}

#[test]
fn f3_f4_game_mode_switcher_cycles_with_additional_f4_presses() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    for _ in 0..3 {
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::F4),
            ElementState::Pressed,
        );
    }
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Adventure,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_hover_uses_first_mouse_suppression() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

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
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative)
    );

    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "first mouse position only arms hover detection"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Creative),
        "same as first mouse position stays suppressed"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(207.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert_eq!(counters.change_game_mode_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChangeGameMode(bbb_protocol::packets::ChangeGameModeCommand {
            game_mode: bbb_protocol::packets::GameType::Spectator,
        })
    );
}

#[test]
fn f3_f4_game_mode_switcher_f4_cycle_resets_first_mouse_suppression() {
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let surface_size = PhysicalSize::new(320, 240);

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
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(176.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Adventure)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::F4),
        ElementState::Pressed,
    );
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator)
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(114.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Spectator),
        "cursor hover is suppressed again after F4 cycling"
    );
    assert!(input.handle_debug_game_mode_switcher_cursor_moved(
        Some(PhysicalPosition::new(145.0, 102.0)),
        surface_size
    ));
    assert_eq!(
        input.debug_game_mode_switcher_selected(),
        Some(bbb_protocol::packets::GameType::Survival)
    );
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
fn chat_entry_limits_message_text_to_vanilla_length() {
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
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"a".repeat(CHAT_ENTRY_MAX_LENGTH + 20),
    );

    let entry = input.chat_entry.as_ref().unwrap();
    assert_eq!(entry.text.chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.cursor, CHAT_ENTRY_MAX_LENGTH);

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
            assert_eq!(packet.message.chars().count(), CHAT_ENTRY_MAX_LENGTH);
            assert_eq!(packet.message, "a".repeat(CHAT_ENTRY_MAX_LENGTH));
        }
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_mid_cursor_insert_only_uses_remaining_vanilla_length() {
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
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"b".repeat(CHAT_ENTRY_MAX_LENGTH - 1),
    );
    let entry = input.chat_entry.as_mut().unwrap();
    entry.cursor = 10;
    entry.selection = 10;

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "XYZ");

    let entry = input.chat_entry.as_ref().unwrap();
    let expected = format!(
        "{}{}{}",
        "b".repeat(10),
        "X",
        "b".repeat(CHAT_ENTRY_MAX_LENGTH - 11)
    );
    assert_eq!(entry.text.chars().count(), CHAT_ENTRY_MAX_LENGTH);
    assert_eq!(entry.text, expected);
    assert_eq!(entry.cursor, 11);
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_filters_vanilla_disallowed_formatting_character() {
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
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &format!("he{}llo", '\u{a7}'),
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("hello")
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_filters_vanilla_disallowed_formatting_character() {
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
        &format!("/s{}eed", '\u{a7}'),
    );

    assert_eq!(
        input.chat_entry.as_ref().map(|entry| entry.text.as_str()),
        Some("/seed")
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
            id: 0,
            command: "/seed".to_string(),
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
            command: "seed".to_string(),
        })
    );
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
fn command_entry_cursor_editing_updates_slash_suggestions() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/givve");
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
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    let entry = input
        .chat_entry
        .as_ref()
        .expect("command entry should stay open");
    assert_eq!(entry.text, "/give");
    assert_eq!(entry.cursor, 4);
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    for (id, command) in [(0, "/givve"), (1, "/give")] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_control_word_keys_edit_text() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/say alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
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
        PhysicalKey::Code(KeyCode::ArrowRight),
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
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    let entry = input
        .chat_entry
        .as_ref()
        .expect("command entry should stay open");
    assert_eq!(entry.text, "/say alpha ");
    assert_eq!(entry.cursor, 11);
    assert_eq!(counters.command_suggestion_commands_queued, 3);
    for (id, command) in [
        (0, "/say alpha beta gamma"),
        (1, "/say alpha gamma"),
        (2, "/say alpha "),
    ] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn command_entry_control_a_selection_replaces_and_submits_command() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "/gamemode creative",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
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
    assert_eq!(counters.command_suggestion_commands_queued, 2);
    assert_eq!(counters.chat_command_commands_queued, 1);
    for (id, command) in [(0, "/gamemode creative"), (1, "/time set day")] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::CommandSuggestionRequest(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            })
        );
    }
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::ChatCommand(ChatCommand {
            command: "time set day".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_control_a_selection_replaces_message_before_submit() {
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
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "wrong message",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "fixed message",
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
    assert_eq!(counters.chat_message_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "fixed message"),
        command => panic!("expected chat message command, got {command:?}"),
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn chat_entry_cursor_editing_submits_corrected_message() {
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
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "t");
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "helo");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "l");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    assert!(!input.chat_entry_is_active());
    assert_eq!(counters.command_suggestion_commands_queued, 0);
    assert_eq!(counters.chat_message_commands_queued, 1);
    match rx.try_recv().unwrap() {
        NetCommand::ChatMessage(packet) => assert_eq!(packet.message, "hello"),
        command => panic!("expected chat message command, got {command:?}"),
    }
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
fn anvil_text_input_starts_from_decoded_custom_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.custom_name = Some("Custom Pick".to_string());
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Custom Pick!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Pick!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_written_book_title_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Book Title".to_string(),
        title_filter: None,
        author: "Author".to_string(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Ignored Item Name".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Book Title!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Book Title!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_text_input_starts_from_decoded_item_name_without_item_runtime() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(42, 1);
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "   ".to_string(),
        title_filter: None,
        author: "Author".to_string(),
        generation: 0,
        pages: Vec::new(),
        page_filters: Vec::new(),
        resolved: true,
    });
    stack.component_patch.item_name = Some("Component Item".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input(&mut input, &mut counters, &mut world, &commands, "!");

    assert_eq!(input.anvil_rename_text(), "Component Item!");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Component Item!".to_string(),
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_default_hover_name_is_sent_as_empty_rename() {
    let root = unique_input_temp_dir("anvil-rename-default-empty");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
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
    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
    );

    assert_eq!(input.anvil_rename_text(), "Test Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Test Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: String::new(),
        })
    );
    assert!(rx.try_recv().is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn anvil_custom_name_matching_hover_name_is_sent_verbatim() {
    let root = unique_input_temp_dir("anvil-rename-custom-name");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut stack = test_item_stack(0, 1);
    stack.component_patch.custom_name = Some("Custom Combo".to_string());
    let mut world = anvil_container_world(7, 12, Some(stack));

    handle_text_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        "!",
    );
    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        Some(&item_runtime),
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
    );

    assert_eq!(input.anvil_rename_text(), "Custom Combo");
    assert_eq!(counters.rename_item_commands_queued, 2);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo!".to_string(),
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Custom Combo".to_string(),
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
fn anvil_cursor_keys_edit_rename_text_inside_line() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

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

    assert_eq!(input.anvil_rename_text(), ">abd<");
    assert_eq!(counters.rename_item_commands_queued, 6);
    for name in ["abcd", "abXcd", "abXd", "abd", ">abd", ">abd<"] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_word_keys_edit_rename_text() {
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
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
        PhysicalKey::Code(KeyCode::ArrowRight),
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
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );

    assert_eq!(input.anvil_rename_text(), "alpha ");
    assert_eq!(counters.rename_item_commands_queued, 3);
    for name in ["alpha beta gamma", "alpha gamma", "alpha "] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_control_a_selection_replaces_and_deletes_rename_text() {
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = anvil_container_world(7, 12, Some(test_item_stack(42, 1)));

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "replace me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "Diamond Pick",
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "delete me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "");
    assert_eq!(counters.rename_item_commands_queued, 5);
    for name in ["replace me", "Diamond Pick", "", "delete me", ""] {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::RenameItem(RenameItem {
                name: name.to_string(),
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn anvil_delete_at_end_is_consumed_without_rename_command() {
    let (tx, mut rx) = mpsc::channel(2);
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
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );

    assert_eq!(input.anvil_rename_text(), "Axe");
    assert_eq!(counters.rename_item_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RenameItem(RenameItem {
            name: "Axe".to_string(),
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
fn book_screen_keys_turn_pages_close_and_block_gameplay_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    open_test_book_screen(&mut world, vec!["First", "Second"]);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::PageDown),
        ElementState::Pressed,
    );
    assert_eq!(world.current_book().unwrap().current_page, 1);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert_eq!(world.current_book(), None);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
}

#[test]
fn book_screen_mouse_clicks_turn_pages_and_close() {
    let mut world = WorldStore::new();
    open_test_book_screen(&mut world, vec!["First", "Second"]);
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - f64::from(BOOK_SCREEN_WIDTH)) * 0.5;
    let origin_y = (600.0 - f64::from(BOOK_SCREEN_HEIGHT)) * 0.5;

    assert!(handle_book_screen_mouse_input(
        &mut world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(BOOK_PAGE_FORWARD_BUTTON_X + 1),
            origin_y + f64::from(BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));
    assert_eq!(world.current_book().unwrap().current_page, 1);

    assert!(handle_book_screen_mouse_input(
        &mut world,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(BOOK_MENU_DONE_BUTTON_X + 10),
            origin_y + f64::from(BOOK_MENU_BUTTON_Y + 1),
        )),
        surface_size,
    ));
    assert_eq!(world.current_book(), None);
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
fn sign_editor_control_word_keys_edit_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        is_front_text: false,
    });

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "alpha beta gamma",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
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
        PhysicalKey::Code(KeyCode::ArrowRight),
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
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
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
            pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
            is_front_text: false,
            lines: [
                "alpha ".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sign_editor_control_a_selection_replaces_and_deletes_current_line() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 2, y: 68, z: -7 },
        is_front_text: true,
    });

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "replace me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "kept");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "remove me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
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
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );

    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "delete me",
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    );
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
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(counters.sign_update_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SignUpdate(SignUpdate {
            pos: ProtocolBlockPos { x: 2, y: 68, z: -7 },
            is_front_text: true,
            lines: [
                "kept".to_string(),
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
        title_styled: Vec::new(),
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
fn escape_key_closes_narrow_recipe_book_before_container() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();

    handle_key_input_with_item_runtime(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
        PhysicalSize::new(378, 720),
    );

    assert!(world.inventory().open_container.is_some());
    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.container_close_commands_queued, 0);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
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
        title_styled: Vec::new(),
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
fn inventory_key_is_consumed_by_focused_recipe_book_search() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 8,
        menu_type_id: 12,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyE),
        ElementState::Pressed,
    );

    assert_eq!(world.open_container_id(), Some(8));
    assert_eq!(counters.container_close_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_page_arrow_click_updates_local_page_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: (0..21)
            .map(|index| recipe_book_shapeless_entry(index, 2, 200 + index))
            .collect(),
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_FORWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_PAGE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(input.recipe_book_page_hud_state().crafting, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    input.shift_left_down = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: true,
        })
    );
}

#[test]
fn narrow_recipe_book_recipe_button_click_closes_book_and_queues_settings() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    let surface_size = PhysicalSize::new(378, 720);
    let origin_x = (378.0 - 176.0) / 2.0;
    let origin_y = (720.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + 14.0 + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert!(!world.recipe_book().settings.crafting.open);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(counters.recipe_book_change_settings_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::RecipeBookChangeSettings(
            bbb_protocol::packets::RecipeBookChangeSettingsCommand {
                book_type: bbb_protocol::packets::RecipeBookType::Crafting,
                open: false,
                filtering: false,
            }
        )
    );
}

#[test]
fn recipe_book_uncraftable_recipe_button_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_shapeless_entry(42, 2, 200)],
    });
    world.apply_place_ghost_recipe(bbb_protocol::packets::PlaceGhostRecipe {
        container_id: 7,
        recipe_display: bbb_protocol::packets::RecipeDisplaySummary {
            display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
            raw_body: Vec::new(),
            crafting: None,
            furnace: None,
        },
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    assert!(world.last_ghost_recipe().is_some());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));
    assert!(world.last_ghost_recipe().is_none());
    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        recipe_button,
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 42,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_craftable_recipe_button_retry_still_queues() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(50, 1),
    });
    let mut entry = recipe_book_shapeless_entry(42, 2, 200);
    entry.contents.crafting_requirements = Some(vec![bbb_protocol::packets::IngredientSummary {
        tag: None,
        item_ids: vec![50],
    }]);
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![entry],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 2);
    for _ in 0..2 {
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
                container_id: 7,
                recipe_index: 42,
                use_max_items: false,
            })
        );
    }
    assert!(rx.try_recv().is_err());
}

#[test]
fn recipe_book_recipe_button_click_uses_search_filtered_collection() {
    let root = unique_input_temp_dir("recipe-book-search");
    write_input_tooltip_item_assets(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    input.recipe_book_search_text = "combo".to_string();
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry(42, 2, 200),
            recipe_book_shapeless_entry(43, 2, 0),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input_with_item_runtime(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(&item_runtime),
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recipe_book_recipe_button_click_uses_current_multi_recipe_cycle() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 30,
        clock_updates: Vec::new(),
    });
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_right_click_overlay_queues_clicked_recipe() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Right,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(
        input.recipe_book_overlay_hud_state(),
        Some(RecipeBookOverlayHudState {
            book_type: bbb_protocol::packets::RecipeBookType::Crafting,
            tab_index: 1,
            page_index: 0,
            button_index: 0,
            x: 11,
            y: 31,
        })
    );
    assert_eq!(counters.place_recipe_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0)),
        surface_size,
    ));

    assert_eq!(input.recipe_book_overlay_hud_state(), None);
    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
}

#[test]
fn recipe_book_overlay_uncraftable_recipe_retry_is_guarded() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_crafting_tab_index = 1;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_crafting_table_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_shapeless_entry_with_group(42, 2, Some(7), 200),
            recipe_book_shapeless_entry_with_group(43, 2, Some(7), 201),
        ],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;
    let recipe_button = Some(PhysicalPosition::new(
        origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
        origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
    ));
    let overlay_second_recipe = Some(PhysicalPosition::new(origin_x + 41.0, origin_y + 37.0));

    for _ in 0..2 {
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Right,
            ElementState::Pressed,
            recipe_button,
            surface_size,
        ));
        assert!(handle_inventory_mouse_input(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            overlay_second_recipe,
            surface_size,
        ));
    }

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 43,
            use_max_items: false,
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn furnace_recipe_book_recipe_button_click_queues_place_recipe_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.recipe_book_furnace_tab_index = 1;
    input.recipe_book_search_focused = true;
    let mut counters = NetCounters::default();
    let mut world = open_recipe_book_furnace_world();
    world.apply_recipe_book_add(bbb_protocol::packets::RecipeBookAdd {
        replace: true,
        entries: vec![recipe_book_furnace_entry(84, 4, 200)],
    });
    let surface_size = PhysicalSize::new(800, 600);
    let origin_x = (800.0 - 320.0) / 2.0;
    let origin_y = (600.0 - 166.0) / 2.0;

    assert!(handle_inventory_mouse_input(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            origin_x + f64::from(RECIPE_BOOK_RECIPE_BUTTON_X + 1),
            origin_y + f64::from(RECIPE_BOOK_RECIPE_BUTTON_Y + 1),
        )),
        surface_size,
    ));

    assert_eq!(counters.place_recipe_commands_queued, 1);
    assert!(!input.recipe_book_search_hud_state().focused);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlaceRecipe(bbb_protocol::packets::PlaceRecipeCommand {
            container_id: 7,
            recipe_index: 84,
            use_max_items: false,
        })
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
        title_styled: Vec::new(),
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
fn advancements_key_opens_local_screen_without_seen_command() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_advancement("minecraft:hidden/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), None);
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_selects_first_root_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_advancement("minecraft:z/root", None),
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:y/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:y/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

fn input_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
    AdvancementSummary {
        id: id.to_string(),
        parent: parent.map(str::to_string),
        display: None,
        requirements: Vec::new(),
        sends_telemetry_event: false,
    }
}

fn input_displayed_advancement(id: &str, parent: Option<&str>) -> AdvancementSummary {
    let mut advancement = input_advancement(id, parent);
    advancement.display = Some(AdvancementDisplaySummary {
        title: id.to_string(),
        description: String::new(),
        icon: AdvancementIconSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
        frame_type: AdvancementFrameType::Task,
        show_toast: false,
        hidden: false,
        background: None,
        x: 0.0,
        y: 0.0,
    });
    advancement
}

#[test]
fn advancements_key_is_consumed_while_container_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 9,
        menu_type_id: 19,
        title: "Merchant".to_string(),
        title_styled: Vec::new(),
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert!(world.inventory().open_container.is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_is_consumed_while_dialog_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert!(world.current_dialog().is_some());
    assert_eq!(counters.advancements_seen_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn escape_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_key_closes_advancements_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
    );

    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_done_button_click_closes_screen_and_queues_seen_packet() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_advancements_screen());

    let handled = handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(400.0, 580.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(!world.advancements_screen_is_open());
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_tab_click_selects_tab_and_queues_opened_tab() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![
            input_displayed_advancement("minecraft:y/root", None),
            input_displayed_advancement("minecraft:a/root", None),
        ],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:y/root".to_string())
    );

    let handled = handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(316.0, 212.0)),
        PhysicalSize::new(800, 600),
    );

    assert!(handled);
    assert!(world.advancements_screen_is_open());
    assert_eq!(world.selected_advancements_tab(), Some("minecraft:a/root"));
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:a/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_mouse_wheel_scrolls_selected_tab_locally() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    let handled = handle_advancements_screen_mouse_wheel(
        &mut input,
        &world,
        MouseScrollDelta::LineDelta(0.0, -1.0),
    );

    assert!(handled);
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((0.0, -16.0))
    );
}

#[test]
fn gameplay_mouse_wheel_is_consumed_while_advancements_screen_is_open() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.set_local_selected_hotbar_slot(0));
    assert!(world.open_advancements_screen());

    handle_mouse_wheel(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        MouseScrollDelta::LineDelta(0.0, 1.0),
    );

    assert_eq!(world.local_player().selected_hotbar_slot, 0);
    assert_eq!(counters.held_slot_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

#[test]
fn advancements_left_drag_scrolls_after_first_drag_event_until_release() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    assert!(world.open_advancements_screen());
    assert_eq!(
        world.ensure_advancements_screen_selected_tab(),
        Some("minecraft:story/root".to_string())
    );

    assert!(handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(100.0, 100.0)),
        Some(PhysicalPosition::new(110.0, 120.0)),
    ));
    assert_eq!(input.inventory_cursor_position(), Some((110, 120)));
    assert_eq!(input.inventory_hovered_slot(), None);
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        None
    );

    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(110.0, 120.0)),
        Some(PhysicalPosition::new(115.0, 116.0)),
    ));
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );

    assert!(handle_advancements_screen_mouse_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        None,
        PhysicalSize::new(800, 600),
    ));
    assert!(handle_advancements_screen_cursor_moved(
        &mut input,
        &world,
        Some(PhysicalPosition::new(115.0, 116.0)),
        Some(PhysicalPosition::new(120.0, 150.0)),
    ));
    assert_eq!(
        input.advancement_scroll_delta(Some("minecraft:story/root")),
        Some((5.0, -4.0))
    );
    assert_eq!(counters.advancements_seen_commands_queued, 0);
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
fn item_model_keybind_context_tracks_default_key_and_mouse_state() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Pressed,
    );

    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.forward"));
    assert!(context.keybind_down("key.hotbar.5"));
    assert!(!context.keybind_down("key.use"));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Digit5),
        ElementState::Released,
    );

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.forward"));
    assert!(!context.keybind_down("key.hotbar.5"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Pressed,
        1.0,
    );
    assert!(input.item_model_keybind_context().keybind_down("key.use"));
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.attack"));

    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Right,
        ElementState::Released,
        1.0,
    );
    assert!(!input.item_model_keybind_context().keybind_down("key.use"));
}

#[test]
fn item_model_keybind_context_tracks_non_debug_default_keymappings() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    for (code, keybind) in [
        (KeyCode::KeyP, "key.socialInteractions"),
        (KeyCode::F2, "key.screenshot"),
        (KeyCode::F5, "key.togglePerspective"),
        (KeyCode::F11, "key.fullscreen"),
        (KeyCode::KeyG, "key.quickActions"),
        (KeyCode::F1, "key.toggleGui"),
        (KeyCode::F4, "key.toggleSpectatorShaderEffects"),
        (KeyCode::KeyC, "key.saveToolbarActivator"),
        (KeyCode::KeyX, "key.loadToolbarActivator"),
    ] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
        assert!(input.item_model_keybind_context().keybind_down(keybind));
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Released,
        );
        assert!(!input.item_model_keybind_context().keybind_down(keybind));
    }

    input.set_key_down(KeyCode::KeyL, true);
    assert!(input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));
    input.set_key_down(KeyCode::KeyL, false);
    assert!(!input
        .item_model_keybind_context()
        .keybind_down("key.advancements"));

    let mut world = WorldStore::new();
    handle_mouse_input_at_partial_tick(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        MouseButton::Middle,
        ElementState::Pressed,
        1.0,
    );
    let context = input.item_model_keybind_context();
    assert!(context.keybind_down("key.pickItem"));
    assert!(context.keybind_down("key.spectatorHotbar"));
}

#[test]
fn item_model_keybind_context_accepts_vanilla_default_unbound_keymappings() {
    // Vanilla Options registers these with InputConstants.UNKNOWN, so
    // IsKeybindDown can decode the key names but KeyMapping.isDown() is false
    // under the default keymap.
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    for code in [KeyCode::F4, KeyCode::F5] {
        handle_key_input_without_world(
            &mut input,
            &mut counters,
            &None,
            PhysicalKey::Code(code),
            ElementState::Pressed,
        );
    }

    let context = input.item_model_keybind_context();
    assert!(!context.keybind_down("key.smoothCamera"));
    assert!(!context.keybind_down("key.spectatorOutlines"));
}

fn assert_sprint_key_on_mount_only_queues_raw_player_input(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

fn assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id: i32) {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, entity_type_id);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

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
            forward: true,
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
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_with_forward_input_queues_player_input_and_sprint_commands() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
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
            forward: true,
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
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            ..PlayerInput::default()
        })
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
fn double_tap_forward_within_sprint_window_queues_start_sprinting() {
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
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
            forward: true,
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
    assert!(rx.try_recv().is_err());
}

#[test]
fn double_tap_forward_after_sprint_window_expires_does_not_start_sprinting() {
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
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Released,
    );

    let start = Instant::now();
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
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 3);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
            forward: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_without_forward_input_does_not_start_sprinting_until_forward_pressed() {
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
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 2);
    assert_eq!(counters.player_command_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
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
}

#[test]
fn sprint_key_with_low_food_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 6,
        saturation: 0.0,
    });

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_while_using_slow_item_only_queues_raw_player_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: test_item_stack(42, 1),
    });
    world.set_local_using_item(true);

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );

    assert_eq!(counters.player_input_commands_queued, 1);
    assert_eq!(counters.player_command_commands_queued, 0);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::PlayerInput(PlayerInput {
            forward: true,
            sprint: true,
            ..PlayerInput::default()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn sprint_key_on_horse_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_26_1_HORSE_ENTITY_TYPE_ID);
}

#[test]
fn sprint_key_on_boat_mount_only_queues_raw_player_input() {
    assert_sprint_key_on_mount_only_queues_raw_player_input(VANILLA_26_1_OAK_BOAT_ENTITY_TYPE_ID);
}

#[test]
fn sprint_key_on_camel_mount_queues_player_input_and_sprint_command_with_low_food() {
    for entity_type_id in [
        VANILLA_26_1_CAMEL_ENTITY_TYPE_ID,
        VANILLA_26_1_CAMEL_HUSK_ENTITY_TYPE_ID,
    ] {
        assert_sprint_key_on_camel_type_queues_start_sprinting(entity_type_id);
    }
}

#[test]
fn sprint_key_camel_mount_focus_loss_queues_stop_sprinting() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.forward = true;
    input.sprint = true;
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_26_1_CAMEL_ENTITY_TYPE_ID);

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
    assert!(rx.try_recv().is_err());
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
fn focus_loss_releases_charged_riding_jump() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    input.jump = true;
    input.riding_jump_charge_seconds = Some(0.2);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_vehicle(77, 10, VANILLA_26_1_HORSE_ENTITY_TYPE_ID);

    handle_focus_change(&mut input, &mut world, &mut counters, &commands, false);

    assert!(!input.focused);
    assert_eq!(input.riding_jump_charge_seconds, None);
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
            action: PlayerCommandAction::StartRidingJump,
            data: 40,
        })
    );
    assert!(rx.try_recv().is_err());
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

fn open_test_book_screen(world: &mut WorldStore, pages: Vec<&str>) {
    let mut stack = test_item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: stack,
    });
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    assert!(world.current_book().is_some());
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
        title_styled: Vec::new(),
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

fn open_recipe_book_crafting_table_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 12,
        title: "Crafting".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(); 46],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn open_recipe_book_furnace_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: 14,
        title: "Furnace".to_string(),
        title_styled: Vec::new(),
    });
    world.apply_container_set_content(ProtocolContainerSetContent {
        container_id: 7,
        state_id: 12,
        items: vec![ProtocolItemStackSummary::empty(); 39],
        carried_item: ProtocolItemStackSummary::empty(),
    });
    world.apply_recipe_book_settings(bbb_protocol::packets::RecipeBookSettings {
        crafting: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        furnace: bbb_protocol::packets::RecipeBookTypeSettings {
            open: true,
            filtering: false,
        },
        blast_furnace: bbb_protocol::packets::RecipeBookTypeSettings::default(),
        smoker: bbb_protocol::packets::RecipeBookTypeSettings::default(),
    });
    world
}

fn recipe_book_shapeless_entry(
    id: i32,
    category_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    recipe_book_shapeless_entry_with_group(id, category_id, None, result_item_id)
}

fn recipe_book_shapeless_entry_with_group(
    id: i32,
    category_id: i32,
    group: Option<i32>,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: 5,
                            raw_payload: Vec::new(),
                            item_stack: Some(test_item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: 0,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                ),
                furnace: None,
            },
            group,
            category_id,
            crafting_requirements: None,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn recipe_book_furnace_entry(
    id: i32,
    category_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::Furnace,
                raw_body: Vec::new(),
                crafting: None,
                furnace: Some(bbb_protocol::packets::FurnaceRecipeDisplaySummary {
                    ingredient: recipe_book_slot_display_item(1),
                    fuel: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: 1,
                        raw_payload: vec![1],
                        item_stack: None,
                        tag: None,
                    },
                    result: recipe_book_slot_display_stack(result_item_id, 1),
                    crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                        display_type_id: 0,
                        raw_payload: vec![0],
                        item_stack: None,
                        tag: None,
                    },
                    duration: 200,
                    experience_bits: 0.0_f32.to_bits(),
                }),
            },
            group: None,
            category_id,
            crafting_requirements: None,
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn recipe_book_slot_display_item(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: 4,
        raw_payload: Vec::new(),
        item_stack: Some(test_item_stack(item_id, 1)),
        tag: None,
    }
}

fn recipe_book_slot_display_stack(
    item_id: i32,
    count: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: 5,
        raw_payload: Vec::new(),
        item_stack: Some(test_item_stack(item_id, count)),
        tag: None,
    }
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
        title_styled: Vec::new(),
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
